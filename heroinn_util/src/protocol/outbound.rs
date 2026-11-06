//! Unified outbound connection layer.
//!
//! A `TransportFactory` owns one resolver/connector pair and is injected into
//! every feature module. Modules only describe a destination; they never read
//! proxy variables or implement CONNECT/SOCKS handshakes themselves.

use std::collections::VecDeque;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use native_tls::{TlsConnector, TlsStream};

use crate::error::Result;

use super::proxy::{
    Destination, DirectProxyResolver, ProxyResolver, ProxyRoute, ProxySource, ProxyTransport,
    SystemProxyResolver,
};

/// Maximum bytes queued for the TLS writer before `write` applies backpressure.
pub const MAX_OUTGOING_BUFFER: usize = 256 * 1024;
/// Maximum bytes buffered from the TLS reader before the driver stops reading.
pub const MAX_INCOMING_BUFFER: usize = 1024 * 1024;
const DRIVER_POLL_INTERVAL: Duration = Duration::from_millis(5);
const DIRECT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Credential-free identity of the proxy selected for a live connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxyContext {
    pub source: ProxySource,
    pub endpoint: String,
}

/// The half-duplex read side of an outbound connection.
#[derive(Clone)]
pub struct OutboundReader {
    inner: Arc<Inner>,
}

/// The half-duplex write side of an outbound connection.
#[derive(Clone)]
pub struct OutboundWriter {
    inner: Arc<Inner>,
}

/// A full-duplex, cloneable outbound byte stream.
///
/// Plain TCP handles share the same socket without cloning its descriptor;
/// a TLS session is owned by a single driver thread and consumers exchange
/// bounded byte queues with that driver. No lock is held while TLS performs
/// blocking work, so a blocked read cannot prevent a write.
pub struct OutboundStream {
    inner: Arc<Inner>,
    reader: Option<OutboundReader>,
    writer: Option<OutboundWriter>,
    proxy: Option<ProxyContext>,
}

impl fmt::Debug for OutboundStream {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OutboundStream")
            .field(
                "kind",
                &match &*self.inner {
                    Inner::Plain(_) => "plain",
                    Inner::Tls(_) => "tls",
                },
            )
            .finish()
    }
}

impl Clone for OutboundStream {
    fn clone(&self) -> Self {
        Self::from_inner(self.inner.clone(), self.proxy.clone())
    }
}

impl OutboundStream {
    pub(crate) fn from_tcp(stream: TcpStream) -> io::Result<Self> {
        stream.set_nodelay(true)?;
        Ok(Self::from_inner(
            Arc::new(Inner::Plain(PlainIo {
                stream,
                write_lock: Mutex::new(()),
            })),
            None,
        ))
    }

    pub(crate) fn from_tls(stream: TlsStream<TcpStream>) -> io::Result<Self> {
        let local_addr = stream.get_ref().local_addr()?;
        let peer_addr = stream.get_ref().peer_addr()?;
        stream.get_ref().set_nodelay(true)?;
        stream.get_ref().set_nonblocking(true)?;

        let shared = Arc::new(TlsShared::new());
        let alive = Arc::new(AtomicBool::new(true));
        let driver_shared = shared.clone();
        let driver_alive = alive.clone();
        std::thread::Builder::new()
            .name("heroinn-tls-outbound".to_string())
            .spawn(move || tls_driver(stream, driver_shared, driver_alive))
            .map_err(|error| io::Error::other(format!("cannot start TLS I/O driver: {error}")))?;

        Ok(Self::from_inner(
            Arc::new(Inner::Tls(TlsIo {
                shared,
                alive,
                local_addr,
                peer_addr,
                write_lock: Mutex::new(()),
            })),
            None,
        ))
    }

    pub(crate) fn from_proxy_transport(
        transport: ProxyTransport,
        proxy: Option<ProxyContext>,
    ) -> Result<Self> {
        let mut stream = match transport {
            ProxyTransport::Plain(stream) => {
                // The proxy handshake uses short read/write timeouts. A
                // connected byte stream must keep the same blocking semantics
                // as an ordinary TCP connection after CONNECT succeeds.
                stream.set_read_timeout(None)?;
                stream.set_write_timeout(None)?;
                Self::from_tcp(stream)?
            }
            ProxyTransport::Tls(stream) => Self::from_tls(*stream)?,
        };
        stream.proxy = proxy;
        Ok(stream)
    }

    pub(crate) fn connect_direct(destination: &Destination) -> io::Result<Self> {
        let addrs = (destination.host(), destination.port()).to_socket_addrs()?;
        let mut last_error = None;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, DIRECT_CONNECT_TIMEOUT) {
                Ok(stream) => return Self::from_tcp(stream),
                Err(error) => last_error = Some(error),
            }
        }
        Err(last_error.unwrap_or_else(|| io::Error::other("destination resolved to no addresses")))
    }

    fn from_inner(inner: Arc<Inner>, proxy: Option<ProxyContext>) -> Self {
        Self {
            reader: Some(OutboundReader {
                inner: inner.clone(),
            }),
            writer: Some(OutboundWriter {
                inner: inner.clone(),
            }),
            inner,
            proxy,
        }
    }

    /// Split the stream into independently movable read and write halves.
    pub fn split(mut self) -> io::Result<(OutboundReader, OutboundWriter)> {
        let reader = self
            .reader
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "reader already taken"))?;
        let writer = self
            .writer
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "writer already taken"))?;
        Ok((reader, writer))
    }

    #[cfg(test)]
    fn tls_shared_for_test(&self) -> Option<std::sync::Weak<TlsShared>> {
        match &*self.inner {
            Inner::Plain(_) => None,
            Inner::Tls(tls) => Some(Arc::downgrade(&tls.shared)),
        }
    }

    /// Clone a handle to the same connection. This never duplicates a TLS
    /// session; both handles use the single bounded TLS driver.
    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(self.clone())
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        match &*self.inner {
            Inner::Plain(plain) => plain.stream.local_addr(),
            Inner::Tls(tls) => Ok(tls.local_addr),
        }
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        match &*self.inner {
            Inner::Plain(plain) => plain.stream.peer_addr(),
            Inner::Tls(tls) => Ok(tls.peer_addr),
        }
    }

    pub fn proxy_context(&self) -> Option<&ProxyContext> {
        self.proxy.as_ref()
    }

    pub(crate) fn close_reader(&self) -> OutboundReader {
        OutboundReader {
            inner: self.inner.clone(),
        }
    }

    /// Observe a remote close without blocking.
    ///
    /// TLS exposes an EOF independently of the reader queue so a streaming
    /// producer can detect that the peer closed before it finished sending.
    /// Plain streams cannot observe EOF without consuming data, so they keep
    /// their historical behaviour.
    pub fn remote_closed(&self) -> io::Result<bool> {
        match &*self.inner {
            Inner::Plain(_) => Ok(false),
            Inner::Tls(tls) => tls.remote_closed(),
        }
    }

    /// Wait until the remote side closes the connection, surfacing a truncated
    /// TLS stream instead of treating the local write buffer as delivery.
    pub fn wait_for_remote_close(&self, timeout: Duration) -> io::Result<()> {
        self.reader
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "reader closed"))?
            .wait_for_remote_close(timeout)
    }

    pub fn set_nodelay(&self, enabled: bool) -> io::Result<()> {
        match &*self.inner {
            Inner::Plain(plain) => plain.stream.set_nodelay(enabled),
            Inner::Tls(tls) => tls.shared.set_nodelay(enabled),
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        match &*self.inner {
            Inner::Plain(plain) => plain.stream.shutdown(how),
            Inner::Tls(tls) => tls.shutdown(how),
        }
    }

    fn reader_mut(&mut self) -> io::Result<&mut OutboundReader> {
        self.reader
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "reader closed"))
    }

    fn writer_mut(&mut self) -> io::Result<&mut OutboundWriter> {
        self.writer
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "writer closed"))
    }
}

impl Read for OutboundStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader_mut()?.read(buf)
    }
}

impl Write for OutboundStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer_mut()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer_mut()?.flush()
    }
}

impl Read for OutboundReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &*self.inner {
            Inner::Plain(plain) => {
                let mut stream = &plain.stream;
                stream.read(buf)
            }
            Inner::Tls(tls) => tls.read(buf),
        }
    }
}

impl OutboundReader {
    /// Observe a remote close without blocking.
    pub fn remote_closed(&self) -> io::Result<bool> {
        match &*self.inner {
            Inner::Plain(_) => Ok(false),
            Inner::Tls(tls) => tls.remote_closed(),
        }
    }

    /// Wait until the remote side closes this connection.
    ///
    /// A TLS connection is confirmed only by the TLS driver observing either a
    /// clean end-of-stream or a transport error. Plain streams retain their
    /// historical best-effort semantics and return immediately.
    pub fn wait_for_remote_close(&self, timeout: Duration) -> io::Result<()> {
        match &*self.inner {
            Inner::Plain(_) => Ok(()),
            Inner::Tls(tls) => tls.wait_for_remote_close(timeout),
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        match &*self.inner {
            Inner::Plain(plain) => plain.stream.shutdown(how),
            Inner::Tls(tls) => tls.shutdown(how),
        }
    }
}

impl Write for OutboundWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &*self.inner {
            Inner::Plain(plain) => {
                let _guard = plain
                    .write_lock
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let mut stream = &plain.stream;
                stream.write(buf)
            }
            Inner::Tls(tls) => tls.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &*self.inner {
            Inner::Plain(plain) => {
                let _guard = plain
                    .write_lock
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                (&plain.stream).flush()
            }
            Inner::Tls(tls) => tls.flush(),
        }
    }
}

impl OutboundWriter {
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        match &*self.inner {
            Inner::Plain(plain) => plain.stream.shutdown(how),
            Inner::Tls(tls) => tls.shutdown(how),
        }
    }
}

enum Inner {
    Plain(PlainIo),
    Tls(TlsIo),
}

struct PlainIo {
    stream: TcpStream,
    write_lock: Mutex<()>,
}

struct TlsIo {
    shared: Arc<TlsShared>,
    alive: Arc<AtomicBool>,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    write_lock: Mutex<()>,
}

impl TlsIo {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            if !state.incoming.is_empty() {
                let count = buf.len().min(state.incoming.len());
                for slot in &mut buf[..count] {
                    *slot = state.incoming.pop_front().expect("incoming is non-empty");
                }
                self.shared.writable.notify_all();
                return Ok(count);
            }
            if let Some(error) = &state.error {
                return Err(error.to_io());
            }
            if state.eof || state.read_closed || state.closed || !self.alive.load(Ordering::Acquire)
            {
                return Ok(0);
            }
            state = self
                .shared
                .readable
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        // Queue insertion alone is not a completed write: callers may drop the
        // transport immediately after their final successful send. Wait for the
        // driver to pass this chunk to the TLS session so the final record can
        // never be discarded with the queue.
        let _write_guard = self
            .write_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            if let Some(error) = &state.error {
                return Err(error.to_io());
            }
            if state.write_closed || state.closed || !self.alive.load(Ordering::Acquire) {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "TLS writer is closed",
                ));
            }

            let available = MAX_OUTGOING_BUFFER.saturating_sub(state.outgoing_bytes);
            if available > 0 {
                let count = buf.len().min(available);
                state.outgoing.push_back(buf[..count].to_vec());
                state.outgoing_bytes += count;
                self.shared.worker.notify_all();
                loop {
                    if let Some(error) = &state.error {
                        return Err(error.to_io());
                    }
                    if state.closed || !self.alive.load(Ordering::Acquire) {
                        return Err(io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            "TLS connection closed before queued data was written",
                        ));
                    }
                    if state.outgoing_bytes == 0 {
                        return Ok(count);
                    }
                    state = self
                        .shared
                        .writable
                        .wait(state)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
            }

            state = self
                .shared
                .writable
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    fn flush(&self) -> io::Result<()> {
        let _write_guard = self
            .write_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        while state.outgoing_bytes != 0 {
            if let Some(error) = &state.error {
                return Err(error.to_io());
            }
            if state.closed || !self.alive.load(Ordering::Acquire) {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "TLS connection closed before queued data was flushed",
                ));
            }
            state = self
                .shared
                .writable
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        if let Some(error) = &state.error {
            return Err(error.to_io());
        }
        Ok(())
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match how {
            Shutdown::Read => state.read_closed = true,
            Shutdown::Write => {
                state.write_closed = true;
                while state.outgoing_bytes != 0 && state.error.is_none() && !state.closed {
                    self.shared.worker.notify_all();
                    state = self
                        .shared
                        .writable
                        .wait(state)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                if let Some(error) = &state.error {
                    return Err(error.to_io());
                }
                if state.outgoing_bytes != 0 || state.closed || !self.alive.load(Ordering::Acquire)
                {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "TLS connection closed before write shutdown completed",
                    ));
                }
            }
            Shutdown::Both => {
                state.closed = true;
                state.outgoing.clear();
                state.outgoing_bytes = 0;
            }
        }
        self.shared.readable.notify_all();
        self.shared.writable.notify_all();
        self.shared.worker.notify_all();
        Ok(())
    }

    fn wait_for_remote_close(&self, timeout: Duration) -> io::Result<()> {
        let deadline = Instant::now() + timeout;
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        loop {
            if let Some(error) = &state.error {
                return Err(error.to_io());
            }
            if state.eof {
                return Ok(());
            }
            if state.read_closed || state.closed || !self.alive.load(Ordering::Acquire) {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "TLS connection closed before the remote end-of-stream was confirmed",
                ));
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "timed out waiting for the remote TLS connection to close",
                ));
            }
            let (next, wait) = self
                .shared
                .readable
                .wait_timeout(state, deadline.saturating_duration_since(now))
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state = next;
            if wait.timed_out() && !state.eof && state.error.is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "timed out waiting for the remote TLS connection to close",
                ));
            }
        }
    }

    fn remote_closed(&self) -> io::Result<bool> {
        let state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(error) = &state.error {
            return Err(error.to_io());
        }
        if state.eof {
            return Ok(true);
        }
        if state.read_closed || state.closed || !self.alive.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "TLS connection closed before the remote end-of-stream was confirmed",
            ));
        }
        Ok(false)
    }
}

struct TlsShared {
    state: Mutex<TlsState>,
    readable: Condvar,
    writable: Condvar,
    worker: Condvar,
    socket: Mutex<Option<TcpStream>>,
    #[cfg(test)]
    driver_writes_paused: AtomicBool,
}

impl TlsShared {
    fn new() -> Self {
        Self {
            state: Mutex::new(TlsState::default()),
            readable: Condvar::new(),
            writable: Condvar::new(),
            worker: Condvar::new(),
            socket: Mutex::new(None),
            #[cfg(test)]
            driver_writes_paused: AtomicBool::new(false),
        }
    }

    fn set_nodelay(&self, enabled: bool) -> io::Result<()> {
        let socket = self
            .socket
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match socket.as_ref() {
            Some(socket) => socket.set_nodelay(enabled),
            None => Ok(()),
        }
    }

    #[cfg(test)]
    fn pause_driver_writes(&self, paused: bool) {
        self.driver_writes_paused.store(paused, Ordering::Release);
        self.worker.notify_all();
    }
}

#[derive(Default)]
struct TlsState {
    incoming: VecDeque<u8>,
    outgoing: VecDeque<Vec<u8>>,
    outgoing_bytes: usize,
    read_closed: bool,
    write_closed: bool,
    closed: bool,
    eof: bool,
    error: Option<IoFailure>,
}

#[derive(Clone, Debug)]
struct IoFailure {
    kind: io::ErrorKind,
    message: String,
}

impl IoFailure {
    fn from_io(error: io::Error) -> Self {
        Self {
            kind: error.kind(),
            message: error.to_string(),
        }
    }

    fn to_io(&self) -> io::Error {
        io::Error::new(self.kind, self.message.clone())
    }
}

fn tls_driver(mut tls: TlsStream<TcpStream>, shared: Arc<TlsShared>, alive: Arc<AtomicBool>) {
    let socket = tls.get_ref().try_clone().ok();
    if let Some(socket) = socket {
        *shared
            .socket
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(socket);
    }

    let weak = Arc::downgrade(&shared);
    // The driver must not keep the connection's Arc alive through the
    // function argument, otherwise dropping every OutboundStream handle would
    // leave a blocked driver and its socket alive forever.
    drop(shared);
    let mut pending: Option<Vec<u8>> = None;
    let mut pending_offset = 0usize;
    let mut write_shutdown_sent = false;

    loop {
        let Some(shared) = weak.upgrade() else {
            alive.store(false, Ordering::Release);
            return;
        };
        if !alive.load(Ordering::Acquire) {
            let _ = tls.get_ref().shutdown(Shutdown::Both);
            return;
        }

        let mut progressed = false;
        let mut closed = false;
        let mut do_write = false;
        let mut do_read = false;
        let mut send_write_shutdown = false;

        {
            let state = shared
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.closed {
                closed = true;
            } else {
                if !state.outgoing.is_empty() || pending.is_some() {
                    do_write = true;
                }
                if !state.read_closed && !state.eof && state.incoming.len() < MAX_INCOMING_BUFFER {
                    do_read = true;
                }
                if state.write_closed
                    && state.outgoing.is_empty()
                    && pending.is_none()
                    && !write_shutdown_sent
                {
                    send_write_shutdown = true;
                }
            }
        }

        #[cfg(test)]
        if do_write && shared.driver_writes_paused.load(Ordering::Acquire) {
            do_write = false;
        }

        if closed {
            let _ = tls.get_ref().shutdown(Shutdown::Both);
            alive.store(false, Ordering::Release);
            shared.readable.notify_all();
            shared.writable.notify_all();
            return;
        }

        if do_write {
            if pending.is_none() {
                let mut state = shared
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                pending = state.outgoing.pop_front();
                pending_offset = 0;
            }
            if let Some(data) = pending.as_ref() {
                match tls.write(&data[pending_offset..]) {
                    Ok(0) => {
                        fail_tls_io(
                            &shared,
                            io::Error::new(
                                io::ErrorKind::WriteZero,
                                "proxy TLS writer returned zero",
                            ),
                        );
                        alive.store(false, Ordering::Release);
                        let _ = tls.get_ref().shutdown(Shutdown::Both);
                        return;
                    }
                    Ok(count) => {
                        pending_offset += count;
                        progressed = true;
                        if pending_offset == data.len() {
                            let finished = pending.take().expect("pending exists");
                            drop(finished);
                            let mut state = shared
                                .state
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            state.outgoing_bytes =
                                state.outgoing_bytes.saturating_sub(pending_offset);
                            shared.writable.notify_all();
                            pending_offset = 0;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => {
                        fail_tls_io(&shared, error);
                        alive.store(false, Ordering::Release);
                        let _ = tls.get_ref().shutdown(Shutdown::Both);
                        return;
                    }
                }
            }
        }

        if do_read {
            let mut buffer = [0u8; 16 * 1024];
            loop {
                let available = {
                    let state = shared
                        .state
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    MAX_INCOMING_BUFFER.saturating_sub(state.incoming.len())
                };
                if available == 0 {
                    break;
                }
                let read_len = available.min(buffer.len());
                match tls.read(&mut buffer[..read_len]) {
                    Ok(0) => {
                        let mut state = shared
                            .state
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        state.eof = true;
                        shared.readable.notify_all();
                        break;
                    }
                    Ok(count) => {
                        progressed = true;
                        let mut state = shared
                            .state
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        state.incoming.extend(&buffer[..count]);
                        shared.readable.notify_all();
                        if state.incoming.len() >= MAX_INCOMING_BUFFER {
                            break;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => {
                        fail_tls_io(&shared, error);
                        alive.store(false, Ordering::Release);
                        let _ = tls.get_ref().shutdown(Shutdown::Both);
                        return;
                    }
                }
            }
        }

        if send_write_shutdown {
            let _ = tls.get_ref().shutdown(Shutdown::Write);
            write_shutdown_sent = true;
            progressed = true;
        }

        if !progressed {
            let should_notify = {
                let state = shared
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                !state.outgoing.is_empty()
                    || state.read_closed
                    || state.eof
                    || state.closed
                    || state.write_closed
            };
            if should_notify {
                shared.worker.notify_all();
            }
            drop(shared);
            std::thread::sleep(DRIVER_POLL_INTERVAL);
        }
    }
}

fn fail_tls_io(shared: &TlsShared, error: io::Error) {
    let mut state = shared
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.error = Some(IoFailure::from_io(error));
    state.read_closed = true;
    state.write_closed = true;
    shared.readable.notify_all();
    shared.writable.notify_all();
}

/// Resolver + connector used by every outbound transport.
pub struct OutboundConnector {
    resolver: Arc<dyn ProxyResolver>,
    tls: TlsSetup,
}

impl Clone for OutboundConnector {
    fn clone(&self) -> Self {
        Self {
            resolver: self.resolver.clone(),
            tls: self.tls.clone(),
        }
    }
}

impl OutboundConnector {
    /// Production connector: environment/system discovery plus system roots.
    pub fn system() -> Self {
        Self::with_resolver(Arc::new(SystemProxyResolver))
    }

    /// Explicit direct connector for local IPC and compatibility paths.
    pub fn direct() -> Self {
        Self::with_resolver(Arc::new(DirectProxyResolver))
    }

    pub fn with_resolver(resolver: Arc<dyn ProxyResolver>) -> Self {
        let tls = match TlsConnector::new() {
            Ok(connector) => TlsSetup::Ready(Arc::new(connector)),
            Err(error) => {
                let detail = format!("system TLS trust is unavailable: {error}");
                log::error!("{detail}");
                TlsSetup::Unavailable(Arc::from(detail))
            }
        };
        Self { resolver, tls }
    }

    /// Inject a test connector that adds a private test CA. Production code
    /// must use the default system trust connector.
    pub fn with_tls_connector(self, connector: TlsConnector) -> Self {
        Self {
            resolver: self.resolver,
            tls: TlsSetup::Ready(Arc::new(connector)),
        }
    }

    pub fn with_static_route(route: ProxyRoute) -> Self {
        Self::with_resolver(Arc::new(super::proxy::StaticProxyResolver::new(route)))
    }

    pub fn open(&self, destination: &Destination) -> Result<OutboundStream> {
        let route = self.resolver.resolve(destination)?;
        self.open_with_route(destination, &route)
    }

    pub fn open_with_route(
        &self,
        destination: &Destination,
        route: &ProxyRoute,
    ) -> Result<OutboundStream> {
        if route.is_direct() {
            return Ok(OutboundStream::connect_direct(destination)?);
        }
        let connected = super::proxy::connect_via_route_with_tls_metadata(
            destination,
            route,
            self.tls.as_result(),
        )?;
        OutboundStream::from_proxy_transport(
            connected.transport,
            Some(ProxyContext {
                source: connected.source,
                endpoint: connected.endpoint,
            }),
        )
    }
}

#[derive(Clone)]
enum TlsSetup {
    Ready(Arc<TlsConnector>),
    Unavailable(Arc<str>),
}

impl TlsSetup {
    fn as_result(&self) -> std::result::Result<&TlsConnector, &str> {
        match self {
            Self::Ready(connector) => Ok(connector),
            Self::Unavailable(detail) => Err(detail),
        }
    }
}

/// Cloneable handle passed to all feature modules.
#[derive(Clone)]
pub struct TransportFactory {
    connector: Arc<OutboundConnector>,
}

impl TransportFactory {
    pub fn system() -> Self {
        Self::with_connector(Arc::new(OutboundConnector::system()))
    }

    pub fn direct() -> Self {
        Self::with_connector(Arc::new(OutboundConnector::direct()))
    }

    pub fn with_connector(connector: Arc<OutboundConnector>) -> Self {
        Self { connector }
    }

    pub fn connector(&self) -> &Arc<OutboundConnector> {
        &self.connector
    }

    pub fn open(&self, destination: &Destination) -> Result<OutboundStream> {
        self.connector.open(destination)
    }

    /// Dial a protocol connection through this factory.
    pub fn connect(
        &self,
        protocol: &crate::HeroinnProtocol,
        address: &str,
    ) -> Result<super::Transport> {
        super::Transport::connect_with_factory(self, protocol, address)
    }

    /// Open a reverse tunnel through this factory.
    pub fn tunnel(
        &self,
        protocol: &crate::HeroinnProtocol,
        remote_addr: &str,
        server_local_port: u16,
    ) -> Result<super::Transport> {
        super::Transport::tunnel_with_factory(self, protocol, remote_addr, server_local_port)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::protocol::http::{WSConnection, WSServer};
    use crate::protocol::proxy::{
        self, ProxyEnvironment, ProxyError, ProxyFailureKind, ProxyResolver, ProxySource,
    };
    use crate::protocol::tcp::{TcpConnection, TcpServer};
    use native_tls::{Certificate, Identity, TlsAcceptor};
    use socket2::SockRef;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::AtomicUsize;
    use std::sync::mpsc::{self, Receiver};
    use std::thread;
    use std::time::{Duration, Instant};

    fn test_tls_connector() -> TlsConnector {
        let ca = Certificate::from_pem(include_bytes!("../../tests/fixtures/tls/ca.pem"))
            .expect("test CA");
        TlsConnector::builder()
            .add_root_certificate(ca)
            .build()
            .expect("test TLS connector")
    }

    fn identity(cert: &[u8], key: &[u8]) -> Identity {
        Identity::from_pkcs8(cert, key).expect("test identity")
    }

    fn route_to_https_proxy(port: u16, target: &str) -> ProxyRoute {
        let environment = ProxyEnvironment {
            http_proxy: None,
            https_proxy: Some(format!("https://localhost:{port}")),
            all_proxy: None,
            no_proxy: None,
        };
        proxy::resolve_target_with(target, &environment, None).expect("proxy route")
    }

    fn route_to_socks_proxy(port: u16, target: &str) -> ProxyRoute {
        let environment = ProxyEnvironment {
            http_proxy: None,
            https_proxy: None,
            all_proxy: Some(format!("socks5://localhost:{port}")),
            no_proxy: None,
        };
        proxy::resolve_target_with(target, &environment, None).expect("proxy route")
    }

    fn read_headers<S: Read>(stream: &mut S) -> String {
        let mut bytes = Vec::new();
        let mut byte = [0u8; 1];
        while bytes.len() < 16 * 1024 {
            match stream.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    bytes.push(byte[0]);
                    if bytes.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn relay_tls_proxy(tls: TlsStream<TcpStream>, authority: String) -> io::Result<()> {
        let upstream = TcpStream::connect(authority)?;
        let outbound = OutboundStream::from_tls(tls)?;
        let (mut tls_reader, mut tls_writer) = outbound.split()?;
        let mut upstream_reader = upstream.try_clone()?;
        let mut upstream_writer = upstream;

        let tls_to_upstream = thread::spawn(move || {
            let _ = io::copy(&mut tls_reader, &mut upstream_writer);
            let _ = upstream_writer.shutdown(Shutdown::Write);
        });
        let _ = io::copy(&mut upstream_reader, &mut tls_writer);
        let _ = tls_writer.flush();
        let _ = tls_writer.shutdown(Shutdown::Write);
        let _ = tls_to_upstream.join();
        Ok(())
    }

    fn spawn_tls_proxy(
        identity: Identity,
        accepted: Arc<AtomicUsize>,
    ) -> (SocketAddr, Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("TLS proxy listener");
        let address = listener.local_addr().expect("TLS proxy address");
        let (request_sender, request_receiver) = mpsc::channel();
        let acceptor = TlsAcceptor::new(identity).expect("TLS proxy acceptor");

        thread::spawn(move || {
            let (stream, _) = listener.accept().expect("TLS proxy accept");
            accepted.fetch_add(1, Ordering::SeqCst);
            let Ok(mut tls) = acceptor.accept(stream) else {
                return;
            };
            let request = read_headers(&mut tls);
            let authority = request
                .lines()
                .next()
                .and_then(|line| line.strip_prefix("CONNECT "))
                .and_then(|line| line.split_whitespace().next())
                .unwrap_or_default()
                .to_string();
            let _ = request_sender.send(request);
            if tls
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .is_err()
            {
                return;
            }
            let _ = relay_tls_proxy(tls, authority);
        });

        (address, request_receiver)
    }

    fn spawn_tls_proxy_closing_after_connect() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").expect("TLS proxy listener");
        let address = listener.local_addr().expect("TLS proxy address");
        let acceptor = TlsAcceptor::new(identity(
            include_bytes!("../../tests/fixtures/tls/localhost.pem"),
            include_bytes!("../../tests/fixtures/tls/localhost.key.pem"),
        ))
        .expect("TLS proxy acceptor");

        thread::spawn(move || {
            let (stream, _) = listener.accept().expect("TLS proxy accept");
            let Ok(mut tls) = acceptor.accept(stream) else {
                return;
            };
            let request = read_headers(&mut tls);
            assert!(request.starts_with("CONNECT "));
            let _ = tls.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n");
            let _ = tls.flush();
            thread::sleep(Duration::from_millis(50));
            let _ = tls.get_ref().shutdown(Shutdown::Both);
        });

        address
    }

    #[derive(Clone, Copy, Debug)]
    enum TlsCloseMode {
        Clean,
        Reset,
        FinWithoutCloseNotify,
    }

    fn spawn_tls_proxy_reading_then_closing(read_bytes: usize, mode: TlsCloseMode) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").expect("TLS proxy listener");
        let address = listener.local_addr().expect("TLS proxy address");
        let acceptor = TlsAcceptor::new(identity(
            include_bytes!("../../tests/fixtures/tls/localhost.pem"),
            include_bytes!("../../tests/fixtures/tls/localhost.key.pem"),
        ))
        .expect("TLS proxy acceptor");

        thread::spawn(move || {
            let (stream, _) = listener.accept().expect("TLS proxy accept");
            let Ok(mut tls) = acceptor.accept(stream) else {
                return;
            };
            let request = read_headers(&mut tls);
            assert!(request.starts_with("CONNECT "));
            tls.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .expect("CONNECT response");
            tls.flush().expect("CONNECT response flush");

            let mut remaining = read_bytes;
            let mut buffer = [0u8; 16 * 1024];
            while remaining > 0 {
                let read_len = remaining.min(buffer.len());
                let count = tls.read(&mut buffer[..read_len]);
                match count {
                    Ok(0) => break,
                    Ok(count) => remaining -= count,
                    Err(_) => break,
                }
            }

            match mode {
                TlsCloseMode::Clean => {
                    let _ = tls.shutdown();
                    let _ = tls.get_ref().shutdown(Shutdown::Both);
                }
                TlsCloseMode::Reset => {
                    let _ = SockRef::from(tls.get_ref()).set_linger(Some(Duration::ZERO));
                    drop(tls);
                }
                TlsCloseMode::FinWithoutCloseNotify => {
                    let _ = tls.get_ref().shutdown(Shutdown::Both);
                }
            }
        });

        address
    }

    fn spawn_socks_proxy(accepted: Arc<AtomicUsize>) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").expect("SOCKS proxy listener");
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut client, _) = listener.accept().expect("SOCKS proxy accept");
            accepted.fetch_add(1, Ordering::SeqCst);

            let mut greeting = [0u8; 2];
            client.read_exact(&mut greeting).unwrap();
            let mut methods = vec![0u8; greeting[1] as usize];
            client.read_exact(&mut methods).unwrap();
            assert!(methods.contains(&0));
            client.write_all(&[5, 0]).unwrap();

            let mut request = [0u8; 4];
            client.read_exact(&mut request).unwrap();
            assert_eq!(&request[..3], &[5, 1, 0]);
            let host = match request[3] {
                0x01 => {
                    let mut octets = [0u8; 4];
                    client.read_exact(&mut octets).unwrap();
                    std::net::Ipv4Addr::from(octets).to_string()
                }
                0x03 => {
                    let mut length = [0u8; 1];
                    client.read_exact(&mut length).unwrap();
                    let mut domain = vec![0u8; length[0] as usize];
                    client.read_exact(&mut domain).unwrap();
                    String::from_utf8(domain).unwrap()
                }
                0x04 => {
                    let mut octets = [0u8; 16];
                    client.read_exact(&mut octets).unwrap();
                    std::net::Ipv6Addr::from(octets).to_string()
                }
                other => panic!("unexpected ATYP {other}"),
            };
            let mut port = [0u8; 2];
            client.read_exact(&mut port).unwrap();
            let upstream = TcpStream::connect((host.as_str(), u16::from_be_bytes(port))).unwrap();
            client
                .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 80])
                .unwrap();

            let mut upstream_reader = upstream.try_clone().unwrap();
            let mut upstream_writer = upstream;
            let mut client_writer = client.try_clone().unwrap();
            thread::spawn(move || {
                let _ = io::copy(&mut upstream_reader, &mut client_writer);
                let _ = client_writer.shutdown(Shutdown::Both);
            });
            let mut client_reader = client;
            let _ = io::copy(&mut client_reader, &mut upstream_writer);
            let _ = upstream_writer.shutdown(Shutdown::Both);
        });
        address
    }

    fn spawn_echo_server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").expect("echo listener");
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("echo accept");
            let mut buffer = [0u8; 16 * 1024];
            loop {
                let count = match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => count,
                };
                if stream.write_all(&buffer[..count]).is_err() {
                    break;
                }
            }
        });
        address
    }

    fn spawn_flood_server(bytes: usize) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").expect("flood listener");
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("flood accept");
            let payload = vec![0x5a; bytes];
            let _ = stream.write_all(&payload);
            thread::sleep(Duration::from_secs(2));
        });
        address
    }

    fn spawn_payload_then_half_close(payload: Vec<u8>, chunk_size: usize) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").expect("payload listener");
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("payload accept");
            for chunk in payload.chunks(chunk_size) {
                stream.write_all(chunk).expect("payload write");
                stream.flush().expect("payload flush");
            }
            stream
                .shutdown(Shutdown::Write)
                .expect("payload half-close");
            thread::sleep(Duration::from_secs(1));
        });
        address
    }

    fn spawn_payload_sink(
        payload: Vec<u8>,
    ) -> (SocketAddr, Receiver<std::result::Result<(), String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("payload sink listener");
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = (|| -> std::result::Result<(), String> {
                let (mut stream, _) = listener.accept().map_err(|error| error.to_string())?;
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .map_err(|error| error.to_string())?;
                let mut received = vec![0u8; payload.len()];
                stream
                    .read_exact(&mut received)
                    .map_err(|error| error.to_string())?;
                if received != payload {
                    return Err("payload sink received different bytes".to_string());
                }
                let mut extra = [0u8; 1];
                match stream.read(&mut extra) {
                    Ok(0) => Ok(()),
                    Ok(count) => Err(format!(
                        "payload sink expected half-close EOF, read {count} extra bytes"
                    )),
                    Err(error) => Err(format!("payload sink EOF read failed: {error}")),
                }
            })();
            let _ = sender.send(result);
        });
        (address, receiver)
    }

    fn spawn_disconnecting_server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").expect("disconnect listener");
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (stream, _) = listener.accept().expect("disconnect accept");
            let _ = stream.shutdown(Shutdown::Both);
        });
        address
    }

    pub(crate) fn factory_with_https_proxy(
        target: &str,
        accepted: Arc<AtomicUsize>,
    ) -> TransportFactory {
        let proxy_identity = identity(
            include_bytes!("../../tests/fixtures/tls/localhost.pem"),
            include_bytes!("../../tests/fixtures/tls/localhost.key.pem"),
        );
        let (proxy, _request) = spawn_tls_proxy(proxy_identity, accepted);
        let route = route_to_https_proxy(proxy.port(), target);
        let connector = Arc::new(
            OutboundConnector::with_static_route(route).with_tls_connector(test_tls_connector()),
        );
        TransportFactory::with_connector(connector)
    }

    fn factory_for_https_proxy(proxy: SocketAddr, target: &str) -> TransportFactory {
        let route = route_to_https_proxy(proxy.port(), target);
        let connector = Arc::new(
            OutboundConnector::with_static_route(route).with_tls_connector(test_tls_connector()),
        );
        TransportFactory::with_connector(connector)
    }

    fn factory_with_socks_proxy(target: &str, accepted: Arc<AtomicUsize>) -> TransportFactory {
        let proxy = spawn_socks_proxy(accepted);
        let route = route_to_socks_proxy(proxy.port(), target);
        let connector = Arc::new(OutboundConnector::with_static_route(route));
        TransportFactory::with_connector(connector)
    }

    #[test]
    fn https_proxy_uses_tls_before_connect_and_carries_websocket() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).expect("WS server");
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let accepted = Arc::new(AtomicUsize::new(0));
        let proxy_identity = identity(
            include_bytes!("../../tests/fixtures/tls/localhost.pem"),
            include_bytes!("../../tests/fixtures/tls/localhost.key.pem"),
        );
        let (proxy, request) = spawn_tls_proxy(proxy_identity, accepted.clone());
        let route = route_to_https_proxy(proxy.port(), &remote);
        let connector = Arc::new(
            OutboundConnector::with_static_route(route).with_tls_connector(test_tls_connector()),
        );
        let factory = TransportFactory::with_connector(connector);

        let mut client = WSConnection::connect_with_factory(&factory, &remote)
            .expect("TLS proxy WebSocket connect");
        client.send(&[1, 2, 3, 4]).expect("send through TLS proxy");
        let request = request
            .recv_timeout(Duration::from_secs(3))
            .expect("CONNECT request was not observed");
        assert!(request.starts_with(&format!("CONNECT {remote} HTTP/1.1\r\n")));
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn https_proxy_carries_http_reverse_tunnel() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).expect("WS server");
        let tunnel_listener = TcpListener::bind("127.0.0.1:0").expect("tunnel listener");
        let tunnel_port = tunnel_listener.local_addr().unwrap().port();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let accepted = Arc::new(AtomicUsize::new(0));
        let factory = factory_with_https_proxy(&remote, accepted.clone());

        let mut client = WSConnection::tunnel_with_factory(&factory, &remote, tunnel_port)
            .expect("TLS proxy HTTP tunnel");
        let (mut server_side, _) =
            TcpConnection::tunnel_server(tunnel_listener, 10).expect("tunnel accept");

        client.send(&[41, 42, 43, 44]).unwrap();
        assert_eq!(server_side.recv().unwrap(), vec![41, 42, 43, 44]);
        server_side.send(&[45, 46, 47, 48]).unwrap();
        assert_eq!(client.recv().unwrap(), vec![45, 46, 47, 48]);
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn tcp_main_connection_uses_https_proxy() {
        let (message_sender, message_receiver) = mpsc::channel();
        let mut server = TcpServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = message_sender.send(message.length());
            }),
        )
        .expect("TCP server");
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let accepted = Arc::new(AtomicUsize::new(0));
        let factory = factory_with_https_proxy(&remote, accepted.clone());

        let mut client = TcpConnection::connect_with_factory(&factory, &remote)
            .expect("TLS proxy TCP connection");
        let frame = crate::packet::Message::build(0, "test", serde_json::json!({})).unwrap();
        client.send(&frame).unwrap();
        assert!(
            message_receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("TCP frame through TLS proxy")
                > 0
        );
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn tcp_reverse_tunnel_uses_socks5_proxy() {
        let target_listener = TcpListener::bind("127.0.0.1:0").expect("target listener");
        let local_port = target_listener.local_addr().unwrap().port();
        let mut remote_server = TcpServer::new("127.0.0.1:0", Arc::new(|_| {})).expect("server");
        let remote = format!("localhost:{}", remote_server.local_addr().unwrap().port());
        let accepted = Arc::new(AtomicUsize::new(0));
        let factory = factory_with_socks_proxy(&remote, accepted.clone());

        let mut client = TcpConnection::tunnel_with_factory(&factory, &remote, local_port)
            .expect("SOCKS5 TCP tunnel");
        let (mut server_side, _) =
            TcpConnection::tunnel_server(target_listener, 10).expect("tunnel accept");

        client.send(&[51, 52, 53, 54]).unwrap();
        assert_eq!(server_side.recv().unwrap(), vec![51, 52, 53, 54]);
        server_side.send(&[55, 56, 57, 58]).unwrap();
        assert_eq!(client.recv().unwrap(), vec![55, 56, 57, 58]);
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        remote_server.close();
    }

    #[test]
    fn tcp_main_connection_uses_socks5_proxy() {
        let (message_sender, message_receiver) = mpsc::channel();
        let mut server = TcpServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = message_sender.send(message.length());
            }),
        )
        .expect("TCP server");
        let remote = format!("localhost:{}", server.local_addr().unwrap().port());
        let accepted = Arc::new(AtomicUsize::new(0));
        let factory = factory_with_socks_proxy(&remote, accepted.clone());

        let mut client =
            TcpConnection::connect_with_factory(&factory, &remote).expect("SOCKS5 TCP connection");
        let frame = crate::packet::Message::build(0, "test", serde_json::json!({})).unwrap();
        client.send(&frame).unwrap();
        assert!(
            message_receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("TCP frame through SOCKS5 proxy")
                > 0
        );
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn https_proxy_basic_auth_is_sent_only_to_the_proxy() {
        let echo = spawn_echo_server();
        let accepted = Arc::new(AtomicUsize::new(0));
        let proxy_identity = identity(
            include_bytes!("../../tests/fixtures/tls/localhost.pem"),
            include_bytes!("../../tests/fixtures/tls/localhost.key.pem"),
        );
        let (proxy, request) = spawn_tls_proxy(proxy_identity, accepted);
        let target = echo.to_string();
        let route = proxy::resolve_target_with(
            &target,
            &ProxyEnvironment {
                http_proxy: None,
                https_proxy: Some(format!(
                    "https://alice:top-secret@localhost:{}",
                    proxy.port()
                )),
                all_proxy: None,
                no_proxy: None,
            },
            None,
        )
        .unwrap();
        let connector = Arc::new(
            OutboundConnector::with_static_route(route).with_tls_connector(test_tls_connector()),
        );
        let factory = TransportFactory::with_connector(connector);
        let mut stream = factory
            .open(&Destination::parse(&target).unwrap())
            .expect("authenticated TLS proxy");
        stream.write_all(b"ping").unwrap();

        let request = request
            .recv_timeout(Duration::from_secs(3))
            .expect("CONNECT request");
        assert!(request.contains("Proxy-Authorization: Basic YWxpY2U6dG9wLXNlY3JldA==\r\n"));
        assert!(!request.contains("top-secret@"));
    }

    #[test]
    fn tls_proxy_rejects_untrusted_certificate_and_never_connects_directly() {
        let target_listener = TcpListener::bind("127.0.0.1:0").expect("direct target");
        target_listener.set_nonblocking(true).unwrap();
        let remote = format!("127.0.0.1:{}", target_listener.local_addr().unwrap().port());
        let accepted = Arc::new(AtomicUsize::new(0));
        let proxy_identity = identity(
            include_bytes!("../../tests/fixtures/tls/localhost.pem"),
            include_bytes!("../../tests/fixtures/tls/localhost.key.pem"),
        );
        let (proxy, _request) = spawn_tls_proxy(proxy_identity, accepted);
        let route = route_to_https_proxy(proxy.port(), &remote);
        let connector = OutboundConnector::with_static_route(route);
        let destination = Destination::parse(&remote).unwrap();

        let error = connector.open(&destination).unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected a proxy TLS error");
        };
        assert_eq!(error.failure().kind, ProxyFailureKind::TlsHandshake);
        assert!(target_listener.accept().is_err());
    }

    #[test]
    fn tls_proxy_hostname_mismatch_fails_closed() {
        let target_listener = TcpListener::bind("127.0.0.1:0").expect("direct target");
        target_listener.set_nonblocking(true).unwrap();
        let remote = format!("127.0.0.1:{}", target_listener.local_addr().unwrap().port());
        let accepted = Arc::new(AtomicUsize::new(0));
        let proxy_identity = identity(
            include_bytes!("../../tests/fixtures/tls/wronghost.pem"),
            include_bytes!("../../tests/fixtures/tls/wronghost.key.pem"),
        );
        let (proxy, _request) = spawn_tls_proxy(proxy_identity, accepted);
        let route = route_to_https_proxy(proxy.port(), &remote);
        let connector = Arc::new(
            OutboundConnector::with_static_route(route).with_tls_connector(test_tls_connector()),
        );
        let factory = TransportFactory::with_connector(connector);

        let error = factory
            .open(&Destination::parse(&remote).unwrap())
            .unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected a proxy TLS error");
        };
        assert_eq!(error.failure().kind, ProxyFailureKind::TlsHandshake);
        assert!(target_listener.accept().is_err());
    }

    #[test]
    fn expired_tls_proxy_certificate_fails_closed() {
        let target_listener = TcpListener::bind("127.0.0.1:0").expect("direct target");
        target_listener.set_nonblocking(true).unwrap();
        let remote = format!("127.0.0.1:{}", target_listener.local_addr().unwrap().port());
        let accepted = Arc::new(AtomicUsize::new(0));
        let proxy_identity = identity(
            include_bytes!("../../tests/fixtures/tls/expired.pem"),
            include_bytes!("../../tests/fixtures/tls/expired.key.pem"),
        );
        let (proxy, _request) = spawn_tls_proxy(proxy_identity, accepted);
        let route = route_to_https_proxy(proxy.port(), &remote);
        let connector = Arc::new(
            OutboundConnector::with_static_route(route).with_tls_connector(test_tls_connector()),
        );

        let error = connector
            .open(&Destination::parse(&remote).unwrap())
            .unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected a proxy TLS error");
        };
        assert_eq!(error.failure().kind, ProxyFailureKind::TlsHandshake);
        assert!(target_listener.accept().is_err());
    }

    #[test]
    fn direct_connector_does_not_resolve_environment_proxy() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("direct listener");
        let address = listener.local_addr().unwrap();
        let accept = thread::spawn(move || listener.accept().map(|(stream, _)| stream));
        let destination = Destination::parse(&address.to_string()).unwrap();
        let mut stream = OutboundConnector::direct().open(&destination).unwrap();
        stream.write_all(b"direct").unwrap();
        let mut accepted = accept.join().unwrap().unwrap();
        let mut bytes = [0u8; 6];
        accepted.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"direct");
    }

    #[test]
    fn future_module_only_needs_the_injected_factory() {
        struct FutureFileTransfer {
            factory: Arc<TransportFactory>,
        }

        impl FutureFileTransfer {
            fn connect(
                &self,
                protocol: &crate::HeroinnProtocol,
                remote: &str,
            ) -> crate::Result<crate::protocol::Transport> {
                crate::protocol::Transport::connect_with_factory(&self.factory, protocol, remote)
            }
        }

        let (message_sender, message_receiver) = mpsc::channel();
        let mut server = TcpServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = message_sender.send(message.length());
            }),
        )
        .expect("TCP server");
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let accepted = Arc::new(AtomicUsize::new(0));
        let feature = FutureFileTransfer {
            factory: Arc::new(factory_with_https_proxy(&remote, accepted.clone())),
        };

        let mut transport = feature
            .connect(&crate::HeroinnProtocol::TCP, &remote)
            .expect("future module connection");
        let frame = crate::packet::Message::build(0, "future", serde_json::json!({})).unwrap();
        transport.send(&frame).unwrap();
        assert!(
            message_receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("future module frame")
                > 0
        );
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn resolver_is_invoked_for_every_new_connection() {
        struct CountingResolver {
            calls: Arc<AtomicUsize>,
        }

        impl ProxyResolver for CountingResolver {
            fn resolve(&self, _destination: &Destination) -> crate::Result<ProxyRoute> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(ProxyRoute::direct())
            }
        }

        let listener = TcpListener::bind("127.0.0.1:0").expect("direct listener");
        let address = listener.local_addr().unwrap();
        let _accept = thread::spawn(move || {
            for _ in 0..2 {
                let _ = listener.accept().unwrap();
            }
        });
        let calls = Arc::new(AtomicUsize::new(0));
        let connector = OutboundConnector::with_resolver(Arc::new(CountingResolver {
            calls: calls.clone(),
        }));
        let destination = Destination::parse(&address.to_string()).unwrap();

        drop(connector.open(&destination).unwrap());
        drop(connector.open(&destination).unwrap());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn dropping_every_tls_handle_stops_the_driver() {
        let echo = spawn_echo_server();
        let remote = echo.to_string();
        let accepted = Arc::new(AtomicUsize::new(0));
        let factory = factory_with_https_proxy(&remote, accepted);
        let stream = factory
            .open(&Destination::parse(&remote).unwrap())
            .expect("TLS proxy connection");
        let shared = stream
            .tls_shared_for_test()
            .expect("TLS stream has a shared driver");

        drop(stream);
        let deadline = Instant::now() + Duration::from_secs(2);
        while shared.upgrade().is_some() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            shared.upgrade().is_none(),
            "TLS driver retained the stream after every handle was dropped"
        );
    }

    #[test]
    fn proxy_handshake_timeout_fails_closed_without_a_direct_connection() {
        let target_listener = TcpListener::bind("127.0.0.1:0").expect("direct target");
        target_listener.set_nonblocking(true).unwrap();
        let target = format!("127.0.0.1:{}", target_listener.local_addr().unwrap().port());
        let proxy_listener = TcpListener::bind("127.0.0.1:0").expect("stalled proxy");
        let proxy_address = proxy_listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = proxy_listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            thread::sleep(proxy::PROXY_HANDSHAKE_TIMEOUT + Duration::from_secs(1));
        });
        let route = proxy::resolve_target_with(
            &target,
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{proxy_address}")),
                https_proxy: None,
                all_proxy: None,
                no_proxy: None,
            },
            None,
        )
        .unwrap();
        let connector = OutboundConnector::with_static_route(route);
        let started = Instant::now();

        let error = connector
            .open(&Destination::parse(&target).unwrap())
            .unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected proxy timeout");
        };
        let failure = error.failure();
        assert_eq!(failure.kind, ProxyFailureKind::Timeout);
        assert_eq!(failure.endpoint, Some(format!("http://{proxy_address}")));
        assert!(started.elapsed() >= proxy::PROXY_HANDSHAKE_TIMEOUT);
        assert!(target_listener.accept().is_err());
    }

    #[test]
    fn tls_outbound_stream_is_full_duplex_under_pressure() {
        let echo = spawn_echo_server();
        let remote = echo.to_string();
        let accepted = Arc::new(AtomicUsize::new(0));
        let proxy_identity = identity(
            include_bytes!("../../tests/fixtures/tls/localhost.pem"),
            include_bytes!("../../tests/fixtures/tls/localhost.key.pem"),
        );
        let (proxy, _request) = spawn_tls_proxy(proxy_identity, accepted);
        let route = route_to_https_proxy(proxy.port(), &remote);
        let connector = Arc::new(
            OutboundConnector::with_static_route(route).with_tls_connector(test_tls_connector()),
        );
        let factory = TransportFactory::with_connector(connector);
        let stream = factory.open(&Destination::parse(&remote).unwrap()).unwrap();
        let (mut reader, mut writer) = stream.split().unwrap();

        let payload = (0..512 * 1024)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let expected = payload.clone();
        let writer_thread = thread::spawn(move || {
            writer.write_all(&payload).unwrap();
            writer.flush().unwrap();
            writer.shutdown(Shutdown::Write).unwrap();
        });

        let mut received = vec![0u8; expected.len()];
        reader.read_exact(&mut received).unwrap();
        writer_thread.join().unwrap();
        assert_eq!(received, expected);
    }

    #[test]
    fn tls_proxy_delivers_aligned_and_unaligned_tail_before_half_close() {
        for size in [262_144usize, 262_437] {
            let payload = (0..size)
                .map(|index| (index % 251) as u8)
                .collect::<Vec<_>>();
            let remote = spawn_payload_then_half_close(payload.clone(), 16 * 1024).to_string();
            let accepted = Arc::new(AtomicUsize::new(0));
            let factory = factory_with_https_proxy(&remote, accepted);
            let stream = factory
                .open(&Destination::parse(&remote).unwrap())
                .expect("TLS proxy connection");
            let (mut reader, writer) = stream.split().unwrap();
            drop(writer);

            let mut received = vec![0u8; payload.len()];
            reader
                .read_exact(&mut received)
                .unwrap_or_else(|error| panic!("short TLS payload at {size} bytes: {error}"));
            let mut extra = [0u8; 1];
            let eof = reader.read(&mut extra).expect("read after TLS payload");
            assert_eq!(eof, 0, "TLS stream did not reach EOF at {size} bytes");
            assert_eq!(received, payload, "TLS stream changed bytes at {size}");
        }
    }

    #[test]
    fn tls_writer_waits_for_final_chunk_before_half_close() {
        for size in [262_144usize, 262_437] {
            let payload = (0..size)
                .map(|index| (index % 251) as u8)
                .collect::<Vec<_>>();
            let (remote, sink_result) = spawn_payload_sink(payload.clone());
            let accepted = Arc::new(AtomicUsize::new(0));
            let factory = factory_with_https_proxy(&remote.to_string(), accepted);
            let stream = factory
                .open(&Destination::parse(&remote.to_string()).unwrap())
                .expect("TLS proxy connection");
            let shared = stream
                .tls_shared_for_test()
                .expect("TLS stream has a shared driver")
                .upgrade()
                .expect("TLS driver is alive");
            shared.pause_driver_writes(true);

            let (reader, mut writer) = stream.split().unwrap();
            let (wrote_sender, wrote_receiver) = mpsc::channel();
            let (continue_sender, continue_receiver) = mpsc::channel();
            let (finished_sender, finished_receiver) = mpsc::channel();
            let worker = thread::spawn(move || {
                let wrote = writer.write_all(&payload);
                let wrote_ok = wrote.is_ok();
                let _ = wrote_sender.send(wrote);
                if !wrote_ok || continue_receiver.recv().is_err() {
                    return;
                }
                let finished = writer
                    .flush()
                    .and_then(|_| writer.shutdown(Shutdown::Write));
                let _ = finished_sender.send(finished);
            });

            assert!(
                wrote_receiver
                    .recv_timeout(Duration::from_millis(200))
                    .is_err(),
                "TLS writer returned while the {size}-byte final chunk was still queued"
            );
            shared.pause_driver_writes(false);
            wrote_receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("TLS writer neither completed nor reported an error")
                .expect("TLS writer rejected the payload");
            continue_sender
                .send(())
                .expect("TLS writer thread exited before flush");
            finished_receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("TLS writer did not finish its half-close")
                .expect("TLS writer half-close failed");
            worker.join().unwrap();
            sink_result
                .recv_timeout(Duration::from_secs(5))
                .expect("payload sink did not finish")
                .expect("payload sink did not observe the complete payload and EOF");

            drop(reader);
            drop(shared);
        }
    }

    #[test]
    fn tls_writer_progresses_while_inbound_backpressure_is_saturated() {
        let remote = spawn_flood_server(2 * 1024 * 1024).to_string();
        let accepted = Arc::new(AtomicUsize::new(0));
        let factory = factory_with_https_proxy(&remote, accepted);
        let stream = factory.open(&Destination::parse(&remote).unwrap()).unwrap();
        let (_reader, mut writer) = stream.split().unwrap();

        thread::sleep(Duration::from_millis(200));
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let result = writer
                .write_all(&vec![0x31; 64 * 1024])
                .and_then(|_| writer.flush());
            let _ = sender.send(result);
        });
        receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("a saturated TLS read queue must not block writes")
            .expect("TLS write while read queue is saturated");
        worker.join().unwrap();
    }

    #[test]
    fn tls_reader_surfaces_peer_disconnect() {
        let remote = spawn_disconnecting_server().to_string();
        let accepted = Arc::new(AtomicUsize::new(0));
        let factory = factory_with_https_proxy(&remote, accepted);
        let mut stream = factory.open(&Destination::parse(&remote).unwrap()).unwrap();
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut byte = [0u8; 1];
            let result = stream.read(&mut byte);
            let _ = sender.send(result);
        });
        let result = receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("peer disconnect must wake a blocked reader");
        assert!(matches!(result, Ok(0) | Err(_)));
        worker.join().unwrap();
    }

    #[test]
    fn tls_data_plane_write_failure_is_returned_with_proxy_context() {
        let proxy = spawn_tls_proxy_closing_after_connect();
        let remote = "127.0.0.1:1";
        let factory = factory_for_https_proxy(proxy, remote);
        let stream = factory
            .open(&Destination::parse(remote).unwrap())
            .expect("TLS proxy tunnel");
        let context = stream.proxy_context().expect("selected proxy context");
        assert_eq!(context.source, ProxySource::Environment);
        assert_eq!(
            context.endpoint,
            format!("https://localhost:{}", proxy.port())
        );

        let (_reader, mut writer) = stream.split().unwrap();
        let buffer = [0x6a; 16 * 1024];
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match writer.write(&buffer) {
                Err(error) => {
                    assert_ne!(error.kind(), io::ErrorKind::Interrupted);
                    return;
                }
                Ok(0) => panic!("TLS writer returned zero after the proxy disconnected"),
                Ok(_) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(10));
                }
                Ok(_) => panic!("TLS writer did not surface the proxy disconnect"),
            }
        }
    }

    #[test]
    fn tls_remote_close_distinguishes_clean_eof_from_truncation() {
        for size in [262_144usize, 262_437] {
            for (mode, expected_error) in [
                (TlsCloseMode::Clean, false),
                (TlsCloseMode::Reset, true),
                (TlsCloseMode::FinWithoutCloseNotify, true),
            ] {
                let read_bytes = if expected_error { 78_014 } else { size };
                let proxy = spawn_tls_proxy_reading_then_closing(read_bytes, mode);
                let remote = "127.0.0.1:1";
                let factory = factory_for_https_proxy(proxy, remote);
                let mut stream = factory
                    .open(&Destination::parse(remote).unwrap())
                    .expect("TLS proxy tunnel");
                let payload = vec![0x5a; size];
                let mut sent = 0usize;
                let mut premature_close = None;

                for chunk in payload.chunks(20 * 1024) {
                    if let Err(error) = stream.write_all(chunk) {
                        premature_close = Some(error);
                        break;
                    }
                    sent += chunk.len();

                    if sent < size {
                        let deadline = Instant::now() + Duration::from_millis(500);
                        loop {
                            match stream.remote_closed() {
                                Ok(true) => {
                                    premature_close = Some(io::Error::new(
                                        io::ErrorKind::BrokenPipe,
                                        format!("{mode:?} remote close after {sent}/{size} bytes"),
                                    ));
                                    break;
                                }
                                Err(error) => {
                                    premature_close = Some(error);
                                    break;
                                }
                                Ok(false) if Instant::now() < deadline => {
                                    thread::sleep(Duration::from_millis(5));
                                }
                                Ok(false) => break,
                            }
                        }
                    }

                    if premature_close.is_some() {
                        break;
                    }
                }

                if expected_error {
                    assert!(
                        premature_close.is_some(),
                        "{mode:?} truncation at {read_bytes}/{size} bytes was reported as success"
                    );
                } else {
                    assert!(
                        premature_close.is_none(),
                        "clean tunnel was reported as prematurely closed: {:?}",
                        premature_close
                    );
                    assert_eq!(sent, size, "clean tunnel write was incomplete");
                    stream
                        .wait_for_remote_close(Duration::from_secs(3))
                        .expect("clean TLS close_notify");
                }
            }
        }
    }

    #[test]
    fn proxy_tls_error_is_redacted() {
        let error = crate::Error::Proxy(ProxyError::ProxyAtEndpoint {
            source_kind: None,
            endpoint: "https://proxy.example:443".to_string(),
            source: Box::new(ProxyError::TlsHandshake(
                "certificate verify failed".to_string(),
            )),
        });
        let crate::Error::Proxy(error) = error else {
            unreachable!()
        };
        let failure = error.failure();
        assert_eq!(failure.kind, ProxyFailureKind::TlsHandshake);
        assert_eq!(
            failure.endpoint.as_deref(),
            Some("https://proxy.example:443")
        );
        assert!(!failure.detail.contains("password"));
    }

    #[test]
    fn unavailable_system_tls_fails_an_https_candidate_without_direct_fallback() {
        let proxy_listener = TcpListener::bind("127.0.0.1:0").expect("proxy listener");
        let target_listener = TcpListener::bind("127.0.0.1:0").expect("direct target");
        target_listener.set_nonblocking(true).unwrap();
        let remote = target_listener.local_addr().unwrap().to_string();
        let destination = Destination::parse(&remote).unwrap();
        let route = route_to_https_proxy(proxy_listener.local_addr().unwrap().port(), &remote);
        let connector = OutboundConnector {
            resolver: Arc::new(proxy::StaticProxyResolver::new(route)),
            tls: TlsSetup::Unavailable(Arc::from("system trust store unavailable")),
        };

        let error = connector.open(&destination).unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected a proxy TLS error");
        };
        assert_eq!(error.failure().kind, ProxyFailureKind::TlsHandshake);
        assert!(target_listener.accept().is_err());
    }
}
