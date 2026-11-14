//! Unified outbound connection layer.
//!
//! A `TransportFactory` owns one resolver/connector pair and is injected into
//! every feature module. Modules only describe a destination; they never read
//! proxy variables or implement CONNECT/SOCKS handshakes themselves.

use std::collections::VecDeque;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use native_tls::{TlsConnector, TlsStream};

use crate::error::Result;
use crate::{Error, Socks5ProxyConfig};

use super::proxy::{
    Destination, DirectProxyResolver, FrozenRoute, ProxyEndpoint, ProxyError, ProxyResolver,
    ProxyRoute, ProxySource, ProxyTransport, SystemProxyResolver, socks5_connect,
};

/// Maximum bytes queued for the TLS writer before `write` applies backpressure.
pub const MAX_OUTGOING_BUFFER: usize = 256 * 1024;
/// Maximum bytes buffered from the TLS reader before the driver stops reading.
pub const MAX_INCOMING_BUFFER: usize = 1024 * 1024;
const DRIVER_POLL_INTERVAL: Duration = Duration::from_millis(5);
const DIRECT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SocketWait {
    Read,
    Write,
}

pub(crate) struct SocketWake {
    socket: UdpSocket,
    pending: AtomicBool,
}

impl SocketWake {
    pub(crate) fn new() -> io::Result<Self> {
        let socket = UdpSocket::bind(("127.0.0.1", 0))?;
        socket.connect(socket.local_addr()?)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            pending: AtomicBool::new(false),
        })
    }

    pub(crate) fn notify(&self) {
        if !self.pending.swap(true, Ordering::AcqRel) {
            let _ = self.socket.send(&[1]);
        }
    }

    pub(crate) fn drain(&self) {
        self.pending.store(false, Ordering::Release);
        let mut bytes = [0; 64];
        while self.socket.recv(&mut bytes).is_ok() {}
    }
}

pub(crate) fn wait_tcp_activity(
    stream: &TcpStream,
    wake: &SocketWake,
    readable: bool,
    writable: bool,
    timeout: Duration,
) -> io::Result<bool> {
    let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as i32;

    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        let mut descriptors = [
            libc::pollfd {
                fd: if readable || writable {
                    stream.as_raw_fd()
                } else {
                    -1
                },
                events: if readable { libc::POLLIN } else { 0 }
                    | if writable { libc::POLLOUT } else { 0 },
                revents: 0,
            },
            libc::pollfd {
                fd: wake.socket.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        loop {
            let result = unsafe { libc::poll(descriptors.as_mut_ptr(), 2, timeout_ms) };
            if result >= 0 {
                return Ok(result > 0);
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawSocket;
        use windows_sys::Win32::Networking::WinSock::{POLLIN, POLLOUT, WSAPOLLFD, WSAPoll};
        let data = WSAPOLLFD {
            fd: stream.as_raw_socket() as usize,
            events: if readable { POLLIN } else { 0 } | if writable { POLLOUT } else { 0 },
            revents: 0,
        };
        let signal = WSAPOLLFD {
            fd: wake.socket.as_raw_socket() as usize,
            events: POLLIN,
            revents: 0,
        };
        let mut descriptors = [signal, data];
        let count = if readable || writable { 2 } else { 1 };
        loop {
            let result = unsafe { WSAPoll(descriptors.as_mut_ptr(), count, timeout_ms) };
            if result >= 0 {
                return Ok(result > 0);
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (stream, wake, readable, writable, timeout_ms);
        Ok(false)
    }
}

pub(crate) fn wait_tcp_listener(listener: &TcpListener, timeout: Duration) -> io::Result<bool> {
    let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as i32;
    #[cfg(unix)]
    let result = {
        use std::os::fd::AsRawFd;
        let mut descriptor = libc::pollfd {
            fd: listener.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        unsafe { libc::poll(&mut descriptor, 1, timeout_ms) }
    };
    #[cfg(windows)]
    let result = {
        use std::os::windows::io::AsRawSocket;
        use windows_sys::Win32::Networking::WinSock::{POLLIN, WSAPOLLFD, WSAPoll};
        let mut descriptor = WSAPOLLFD {
            fd: listener.as_raw_socket() as usize,
            events: POLLIN,
            revents: 0,
        };
        unsafe { WSAPoll(&mut descriptor, 1, timeout_ms) }
    };
    #[cfg(not(any(unix, windows)))]
    let result = {
        let _ = (listener, timeout_ms);
        0
    };
    if result >= 0 {
        return Ok(result > 0);
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::Interrupted {
        Ok(false)
    } else {
        Err(error)
    }
}

/// Wait for socket readiness without sleeping through the platform timer
/// granularity. Windows rounds a 1 ms `sleep` to roughly 16 ms, which would
/// throttle every WebSocket poll iteration.
#[cfg(test)]
pub(crate) fn wait_tcp_socket(
    stream: &TcpStream,
    wait: SocketWait,
    timeout: Duration,
) -> io::Result<bool> {
    let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as i32;

    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;

        let events = match wait {
            SocketWait::Read => libc::POLLIN,
            SocketWait::Write => libc::POLLOUT,
        };
        let mut descriptor = libc::pollfd {
            fd: stream.as_raw_fd(),
            events,
            revents: 0,
        };
        loop {
            let result = unsafe { libc::poll(&mut descriptor, 1, timeout_ms) };
            if result < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
            return Ok(result > 0);
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawSocket;
        use windows_sys::Win32::Networking::WinSock::{POLLIN, POLLOUT, WSAPOLLFD, WSAPoll};

        let events = match wait {
            SocketWait::Read => POLLIN,
            SocketWait::Write => POLLOUT,
        };
        let mut descriptor = WSAPOLLFD {
            fd: stream.as_raw_socket() as usize,
            events,
            revents: 0,
        };
        loop {
            let result = unsafe { WSAPoll(&mut descriptor, 1, timeout_ms) };
            if result < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
            return Ok(result > 0);
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (stream, wait, timeout_ms);
        Ok(false)
    }
}

/// Credential-free identity of the proxy chain selected for a live connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxyChainContext {
    pub source: ProxySource,
    pub endpoint: String,
    pub system_proxy: Option<String>,
    pub system_bypassed: bool,
    pub socks5_proxy: Option<String>,
}

/// Backward-compatible name used by existing diagnostics.
pub type ProxyContext = ProxyChainContext;

/// Route state attached to every successfully opened stream.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RouteReport {
    pub system_proxy: Option<String>,
    pub system_source: Option<ProxySource>,
    pub system_bypassed: bool,
    pub socks5_proxy: Option<String>,
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
    route: RouteReport,
    frozen: Option<FrozenRoute>,
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
        Self::from_inner(
            self.inner.clone(),
            self.proxy.clone(),
            self.route.clone(),
            self.frozen.clone(),
        )
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
            RouteReport::default(),
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
            RouteReport::default(),
            None,
        ))
    }

    pub(crate) fn from_proxy_transport(
        transport: ProxyTransport,
        proxy: Option<ProxyContext>,
        route: RouteReport,
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
        stream.route = route;
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

    fn from_inner(
        inner: Arc<Inner>,
        proxy: Option<ProxyContext>,
        route: RouteReport,
        frozen: Option<FrozenRoute>,
    ) -> Self {
        Self {
            reader: Some(OutboundReader {
                inner: inner.clone(),
            }),
            writer: Some(OutboundWriter {
                inner: inner.clone(),
            }),
            inner,
            proxy,
            route,
            frozen,
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

    pub fn route_report(&self) -> &RouteReport {
        &self.route
    }

    /// Exact credential-bearing route used by this connection, suitable for
    /// pinning later reverse tunnels to the same proxy chain.
    pub fn frozen_route(&self) -> Option<&FrozenRoute> {
        self.frozen.as_ref()
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

    /// Apply a short read timeout to the blocking transport. WebSocket uses
    /// this to poll its bounded send queue without holding a lock across a
    /// blocking read. Other callers keep the default blocking behaviour.
    pub(crate) fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        match &*self.inner {
            Inner::Plain(plain) => plain.stream.set_read_timeout(timeout),
            Inner::Tls(tls) => {
                tls.shared
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .read_timeout = timeout;
                tls.shared.readable.notify_all();
                Ok(())
            }
        }
    }

    /// Apply a bounded write timeout to the blocking transport. WebSocket
    /// actors use this so a stalled peer cannot prevent deterministic
    /// cancellation. Other callers keep the default blocking behaviour.
    pub(crate) fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        match &*self.inner {
            Inner::Plain(plain) => plain.stream.set_write_timeout(timeout),
            Inner::Tls(tls) => {
                tls.shared
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .write_timeout = timeout;
                tls.shared.writable.notify_all();
                Ok(())
            }
        }
    }

    /// Switch the plain socket between blocking and polling handshake modes.
    /// The TLS driver is already nonblocking; its consumers use condvar timeouts.
    pub(crate) fn set_nonblocking(&self, enabled: bool) -> io::Result<()> {
        match &*self.inner {
            Inner::Plain(plain) => plain.stream.set_nonblocking(enabled),
            Inner::Tls(_) => Ok(()),
        }
    }

    /// Native polling is safe for plain sockets. TLS consumers remain on the
    /// driver's condvar-backed read timeout because switching the TLS stream
    /// itself would bypass the driver state machine.
    pub(crate) fn enable_nonblocking_poll(&self) -> io::Result<bool> {
        match &*self.inner {
            Inner::Plain(plain) => {
                plain.stream.set_nonblocking(true)?;
                Ok(true)
            }
            Inner::Tls(_) => Ok(false),
        }
    }

    pub(crate) fn wait_ws_activity(
        &self,
        wake: &SocketWake,
        readable: bool,
        writable: bool,
        timeout: Duration,
    ) -> io::Result<bool> {
        match &*self.inner {
            Inner::Plain(plain) => {
                wait_tcp_activity(&plain.stream, wake, readable, writable, timeout)
            }
            Inner::Tls(_) => Ok(false),
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
            state = if let Some(timeout) = state.read_timeout {
                let (next, wait) = self
                    .shared
                    .readable
                    .wait_timeout(state, timeout)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if wait.timed_out() && next.incoming.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "TLS read timed out",
                    ));
                }
                next
            } else {
                self.shared
                    .readable
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
            };
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
                    if let Some(timeout) = state.write_timeout {
                        let (next, wait) = self
                            .shared
                            .writable
                            .wait_timeout(state, timeout)
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        if wait.timed_out() && next.outgoing_bytes != 0 {
                            // The bytes were accepted into the bounded queue.
                            // `flush` remains responsible for confirming that
                            // the TLS driver actually wrote them.
                            return Ok(count);
                        }
                        state = next;
                    } else {
                        state = self
                            .shared
                            .writable
                            .wait(state)
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                    }
                }
            }

            if let Some(timeout) = state.write_timeout {
                let (next, wait) = self
                    .shared
                    .writable
                    .wait_timeout(state, timeout)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if wait.timed_out() {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "TLS write timed out waiting for buffer space",
                    ));
                }
                state = next;
            } else {
                state = self
                    .shared
                    .writable
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
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
            if let Some(timeout) = state.write_timeout {
                let (next, wait) = self
                    .shared
                    .writable
                    .wait_timeout(state, timeout)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if wait.timed_out() && next.outgoing_bytes != 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "TLS flush timed out",
                    ));
                }
                state = next;
            } else {
                state = self
                    .shared
                    .writable
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
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
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
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
    specified_socks5: Option<ProxyEndpoint>,
}

impl Clone for OutboundConnector {
    fn clone(&self) -> Self {
        Self {
            resolver: self.resolver.clone(),
            tls: self.tls.clone(),
            specified_socks5: self.specified_socks5.clone(),
        }
    }
}

impl OutboundConnector {
    /// Production connector: environment/system discovery plus system roots.
    pub fn system() -> Self {
        Self::with_resolver(Arc::new(SystemProxyResolver))
    }

    /// Production connector with an optional client-specified SOCKS5 hop.
    pub fn system_with_specified_socks5(config: Option<&Socks5ProxyConfig>) -> Result<Self> {
        Self::system().with_optional_specified_socks5(config)
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
        Self {
            resolver,
            tls,
            specified_socks5: None,
        }
    }

    /// Inject a test connector that adds a private test CA. Production code
    /// must use the default system trust connector.
    pub fn with_tls_connector(self, connector: TlsConnector) -> Self {
        Self {
            resolver: self.resolver,
            tls: TlsSetup::Ready(Arc::new(connector)),
            specified_socks5: self.specified_socks5,
        }
    }

    pub fn with_specified_socks5(mut self, config: &Socks5ProxyConfig) -> Result<Self> {
        self.specified_socks5 = Some(ProxyEndpoint::from_socks5_config(config)?);
        Ok(self)
    }

    pub fn with_optional_specified_socks5(
        mut self,
        config: Option<&Socks5ProxyConfig>,
    ) -> Result<Self> {
        self.specified_socks5 = config.map(ProxyEndpoint::from_socks5_config).transpose()?;
        Ok(self)
    }

    pub fn with_static_route(route: ProxyRoute) -> Self {
        Self::with_resolver(Arc::new(super::proxy::StaticProxyResolver::new(route)))
    }

    pub fn open(&self, destination: &Destination) -> Result<OutboundStream> {
        let route = self.resolver.resolve(destination)?;
        self.open_with_route(destination, &route)
    }

    /// Open a connection using an already-frozen route. Unlike `open`, this
    /// performs no proxy discovery and cannot try a different candidate.
    pub fn open_frozen(
        &self,
        destination: &Destination,
        frozen: &FrozenRoute,
    ) -> Result<OutboundStream> {
        if let Some(mut transport) = frozen.connect(destination, self.tls.as_result())? {
            transport.clear_handshake_timeouts()?;
            let system_proxy = frozen.system_endpoint_label();
            let socks5_proxy = frozen.specified_socks5_label();
            let source = if system_proxy.is_none() {
                ProxySource::Specified
            } else {
                frozen.source().unwrap_or(ProxySource::Specified)
            };
            let endpoint = socks5_proxy
                .clone()
                .or_else(|| system_proxy.clone())
                .unwrap_or_else(|| "proxied".to_string());
            let mut stream = OutboundStream::from_proxy_transport(
                transport,
                Some(ProxyContext {
                    source,
                    endpoint,
                    system_proxy: system_proxy.clone(),
                    system_bypassed: frozen.was_bypassed(),
                    socks5_proxy: socks5_proxy.clone(),
                }),
                RouteReport {
                    system_proxy,
                    system_source: frozen.source(),
                    system_bypassed: frozen.was_bypassed(),
                    socks5_proxy,
                },
            )?;
            stream.frozen = Some(frozen.clone());
            return Ok(stream);
        }

        if !frozen.is_direct() {
            return Err(Error::Other(
                "frozen proxy route did not produce a proxy transport".to_string(),
            ));
        }
        let mut stream = OutboundStream::connect_direct(destination)?;
        stream.route = RouteReport {
            system_proxy: None,
            system_source: frozen.source(),
            system_bypassed: frozen.was_bypassed(),
            socks5_proxy: None,
        };
        stream.frozen = Some(frozen.clone());
        Ok(stream)
    }

    pub fn open_with_route(
        &self,
        destination: &Destination,
        route: &ProxyRoute,
    ) -> Result<OutboundStream> {
        if let Some(specified) = &self.specified_socks5 {
            return self.open_with_specified_socks5(destination, route, specified);
        }

        if route.is_direct() {
            let mut stream = OutboundStream::connect_direct(destination)?;
            stream.route = RouteReport {
                system_proxy: None,
                system_source: route.source(),
                system_bypassed: route.was_bypassed(),
                socks5_proxy: None,
            };
            stream.frozen = Some(FrozenRoute::from_parts(
                route.source(),
                None,
                route.was_bypassed(),
                None,
            ));
            return Ok(stream);
        }
        let connected = super::proxy::connect_via_route_with_tls_metadata(
            destination,
            route,
            self.tls.as_result(),
        )?;
        let frozen = FrozenRoute::from_parts(
            route.source(),
            Some(
                route
                    .candidates()
                    .iter()
                    .find(|candidate| candidate.safe_label() == connected.endpoint)
                    .cloned()
                    .ok_or_else(|| {
                        Error::Other("selected proxy candidate was not in the route".to_string())
                    })?,
            ),
            false,
            None,
        );
        let transport = connected.transport;
        let mut stream = OutboundStream::from_proxy_transport(
            transport,
            Some(ProxyContext {
                source: connected.source,
                endpoint: connected.endpoint.clone(),
                system_proxy: Some(connected.endpoint.clone()),
                system_bypassed: false,
                socks5_proxy: None,
            }),
            RouteReport {
                system_proxy: Some(connected.endpoint),
                system_source: route.source(),
                system_bypassed: false,
                socks5_proxy: None,
            },
        )?;
        stream.frozen = Some(frozen);
        Ok(stream)
    }

    fn open_with_specified_socks5(
        &self,
        destination: &Destination,
        system_route: &ProxyRoute,
        specified: &ProxyEndpoint,
    ) -> Result<OutboundStream> {
        let specified_endpoint = specified.safe_label();

        if system_route.is_direct() {
            let mut connected = specified
                .connect(destination, self.tls.as_result())
                .map_err(|error| {
                    proxy_at_endpoint(ProxySource::Specified, &specified_endpoint, error)
                })?;
            connected.clear_handshake_timeouts()?;
            let frozen = FrozenRoute::from_parts(
                system_route.source(),
                None,
                system_route.was_bypassed(),
                Some(specified.clone()),
            );
            return OutboundStream::from_proxy_transport(
                connected,
                Some(ProxyContext {
                    source: ProxySource::Specified,
                    endpoint: specified_endpoint.clone(),
                    system_proxy: None,
                    system_bypassed: system_route.was_bypassed(),
                    socks5_proxy: Some(specified_endpoint.clone()),
                }),
                RouteReport {
                    system_proxy: None,
                    system_source: system_route.source(),
                    system_bypassed: system_route.was_bypassed(),
                    socks5_proxy: Some(specified_endpoint),
                },
            )
            .map(|mut stream| {
                stream.frozen = Some(frozen);
                stream
            });
        }

        let outer_destination = Destination::parse(&specified.authority())?;
        let mut connected = super::proxy::connect_via_route_with_tls_metadata(
            &outer_destination,
            system_route,
            self.tls.as_result(),
        )?;

        match &mut connected.transport {
            ProxyTransport::Plain(stream) => {
                socks5_connect(stream, specified, destination).map_err(|error| {
                    proxy_at_endpoint(ProxySource::Specified, &specified_endpoint, error)
                })?;
            }
            ProxyTransport::Tls(stream) => {
                socks5_connect(stream.as_mut(), specified, destination).map_err(|error| {
                    proxy_at_endpoint(ProxySource::Specified, &specified_endpoint, error)
                })?;
            }
        }
        connected.transport.clear_handshake_timeouts()?;

        let outer_endpoint = connected.endpoint.clone();
        let system_endpoint = system_route
            .candidates()
            .iter()
            .find(|candidate| candidate.safe_label() == outer_endpoint)
            .cloned()
            .ok_or_else(|| {
                Error::Other("selected outer proxy candidate was not in the route".to_string())
            })?;
        let frozen = FrozenRoute::from_parts(
            system_route.source(),
            Some(system_endpoint),
            false,
            Some(specified.clone()),
        );
        OutboundStream::from_proxy_transport(
            connected.transport,
            Some(ProxyContext {
                source: connected.source,
                endpoint: outer_endpoint.clone(),
                system_proxy: Some(outer_endpoint.clone()),
                system_bypassed: false,
                socks5_proxy: Some(specified_endpoint.clone()),
            }),
            RouteReport {
                system_proxy: Some(outer_endpoint),
                system_source: system_route.source(),
                system_bypassed: false,
                socks5_proxy: Some(specified_endpoint),
            },
        )
        .map(|mut stream| {
            stream.frozen = Some(frozen);
            stream
        })
    }
}

fn proxy_at_endpoint(source: ProxySource, endpoint: &str, error: crate::Error) -> crate::Error {
    let proxy_error = match error {
        crate::Error::Proxy(error) => error,
        error => ProxyError::Handshake(error.to_string()),
    };
    ProxyError::ProxyAtEndpoint {
        source_kind: Some(source),
        endpoint: endpoint.to_string(),
        source: Box::new(proxy_error),
    }
    .into()
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

    pub fn system_with_specified_socks5(config: Option<&Socks5ProxyConfig>) -> Result<Self> {
        Ok(Self::with_connector(Arc::new(
            OutboundConnector::system_with_specified_socks5(config)?,
        )))
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

    pub fn open_frozen(
        &self,
        destination: &Destination,
        frozen: &FrozenRoute,
    ) -> Result<OutboundStream> {
        self.connector.open_frozen(destination, frozen)
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

    type SocksAuthObservation = (Vec<u8>, Vec<u8>, String);

    #[test]
    fn listener_readiness_wakes_for_a_connection_before_the_idle_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        listener.set_nonblocking(true).expect("nonblocking");
        let address = listener.local_addr().expect("address");
        let (entered, ready) = mpsc::sync_channel(1);
        let (finished, result) = mpsc::sync_channel(1);
        let waiter = thread::spawn(move || {
            entered.send(()).expect("signal waiter");
            let readable = wait_tcp_listener(&listener, Duration::from_secs(2))
                .expect("wait for incoming connection");
            finished.send(readable).expect("signal readiness");
            let (stream, _) = listener.accept().expect("accept ready connection");
            stream.peer_addr().expect("connected peer")
        });
        ready
            .recv_timeout(Duration::from_secs(2))
            .expect("waiter entered");
        let client = TcpStream::connect(address).expect("client");
        assert!(
            result
                .recv_timeout(Duration::from_millis(500))
                .expect("readiness wake"),
            "an incoming connection must wake the listener without waiting for its idle deadline"
        );
        assert_eq!(
            waiter.join().expect("waiter joined"),
            client.local_addr().expect("client address")
        );
    }

    #[test]
    fn listener_readiness_times_out_without_consuming_future_connections() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        listener.set_nonblocking(true).expect("nonblocking");
        let started = Instant::now();
        assert!(!wait_tcp_listener(&listener, Duration::from_millis(25)).expect("idle timeout"));
        assert!(started.elapsed() >= Duration::from_millis(10));
        assert_eq!(
            listener.accept().expect_err("still idle").kind(),
            io::ErrorKind::WouldBlock
        );
        let client = TcpStream::connect(listener.local_addr().expect("address")).expect("client");
        assert!(wait_tcp_listener(&listener, Duration::from_secs(2)).expect("connection ready"));
        let (stream, _) = listener.accept().expect("accept");
        assert_eq!(
            stream.peer_addr().expect("peer"),
            client.local_addr().expect("client address")
        );
        assert!(!wait_tcp_listener(&listener, Duration::ZERO).expect("queue drained"));
    }

    #[test]
    fn socket_activity_is_woken_by_a_queued_command_without_peer_data() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let client = TcpStream::connect(listener.local_addr().expect("address")).expect("client");
        let (_server, _) = listener.accept().expect("server");
        let wake = Arc::new(SocketWake::new().expect("wake"));
        let waiter_wake = wake.clone();
        let (entered, ready) = mpsc::sync_channel(1);
        let waiter = thread::spawn(move || {
            entered.send(()).expect("signal waiter");
            wait_tcp_activity(&client, &waiter_wake, true, false, Duration::from_secs(2))
                .expect("wait for socket or command")
        });
        ready
            .recv_timeout(Duration::from_secs(2))
            .expect("waiter entered");
        for _ in 0..256 {
            wake.notify();
        }
        assert!(waiter.join().expect("waiter joined"));
        wake.drain();
        assert!(!wake.pending.load(Ordering::Acquire));
        wake.notify();
        let mut byte = [0; 1];
        assert_eq!(wake.socket.recv(&mut byte).expect("subsequent wake"), 1);
        assert_eq!(byte, [1]);
    }

    #[test]
    fn paused_socket_reads_wait_for_capacity_instead_of_readable_data() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let client = TcpStream::connect(listener.local_addr().expect("address")).expect("client");
        let (mut server, _) = listener.accept().expect("server");
        server
            .write_all(b"unconsumed frame")
            .expect("pending peer data");
        assert!(
            wait_tcp_socket(&client, SocketWait::Read, Duration::from_secs(2)).expect("readable")
        );
        let wake = SocketWake::new().expect("wake");
        assert!(!wait_tcp_activity(&client, &wake, false, false, Duration::ZERO).expect("paused"));
        wake.notify();
        assert!(
            wait_tcp_activity(&client, &wake, false, false, Duration::from_secs(2))
                .expect("capacity wake")
        );
        wake.drain();
        assert!(!wait_tcp_activity(&client, &wake, false, false, Duration::ZERO).expect("drained"));
    }

    #[test]
    fn socket_wait_wakes_immediately_when_data_arrives() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("listener address");
        let mut client = TcpStream::connect(address).expect("client");
        let (mut server, _) = listener.accept().expect("accepted server");
        let writer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            server.write_all(b"x").expect("write readiness byte");
            server.flush().expect("flush readiness byte");
        });

        let started = Instant::now();
        assert!(
            wait_tcp_socket(&client, SocketWait::Read, Duration::from_secs(2))
                .expect("socket readiness wait")
        );
        let elapsed = started.elapsed();
        writer.join().expect("writer thread");
        let mut byte = [0u8; 1];
        client.read_exact(&mut byte).expect("read readiness byte");
        assert_eq!(&byte, b"x");
        assert!(
            elapsed < Duration::from_millis(500),
            "readiness wake was delayed by {elapsed:?}"
        );
    }

    #[test]
    fn socket_wait_times_out_without_data() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("listener address");
        let client = TcpStream::connect(address).expect("client");
        let (_server, _) = listener.accept().expect("accepted server");

        let started = Instant::now();
        assert!(
            !wait_tcp_socket(&client, SocketWait::Read, Duration::from_millis(25))
                .expect("socket timeout wait")
        );
        let elapsed = started.elapsed();
        assert!(
            elapsed >= Duration::from_millis(10),
            "wait returned too early: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "wait exceeded bound: {elapsed:?}"
        );
    }

    #[test]
    fn socket_wait_reports_writability_for_a_connected_socket() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("listener address");
        let client = TcpStream::connect(address).expect("client");
        let (_server, _) = listener.accept().expect("accepted server");

        assert!(
            wait_tcp_socket(&client, SocketWait::Write, Duration::from_millis(500))
                .expect("socket writability wait")
        );
    }

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

    fn spawn_tls_proxy_n(
        identity: Identity,
        accepted: Arc<AtomicUsize>,
        expected_connections: usize,
    ) -> (SocketAddr, Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("TLS proxy listener");
        let address = listener.local_addr().expect("TLS proxy address");
        let (request_sender, request_receiver) = mpsc::channel();
        let acceptor = TlsAcceptor::new(identity).expect("TLS proxy acceptor");

        thread::spawn(move || {
            for _ in 0..expected_connections {
                let Ok((stream, _)) = listener.accept() else {
                    break;
                };
                accepted.fetch_add(1, Ordering::SeqCst);
                let Ok(mut tls) = acceptor.accept(stream) else {
                    continue;
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
                    continue;
                }
                thread::spawn(move || {
                    let _ = relay_tls_proxy(tls, authority);
                });
            }
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

    fn spawn_socks_proxy_n(
        accepted: Arc<AtomicUsize>,
        expected_connections: usize,
    ) -> (SocketAddr, Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("SOCKS proxy listener");
        let address = listener.local_addr().unwrap();
        let (authority_sender, authority_receiver) = mpsc::channel();

        thread::spawn(move || {
            for _ in 0..expected_connections {
                let Ok((mut client, _)) = listener.accept() else {
                    break;
                };
                accepted.fetch_add(1, Ordering::SeqCst);
                let authority_sender = authority_sender.clone();
                thread::spawn(move || {
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
                    let authority = format!("{host}:{}", u16::from_be_bytes(port));
                    let _ = authority_sender.send(authority.clone());
                    let upstream =
                        TcpStream::connect(&authority).expect("SOCKS proxy upstream connection");
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
            }
        });

        (address, authority_receiver)
    }

    fn spawn_http_proxy(accepted: Arc<AtomicUsize>) -> (SocketAddr, Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP proxy listener");
        let address = listener.local_addr().unwrap();
        let (request_sender, request_receiver) = mpsc::channel();
        thread::spawn(move || {
            let (mut client, _) = listener.accept().expect("HTTP proxy accept");
            accepted.fetch_add(1, Ordering::SeqCst);
            let request = read_headers(&mut client);
            let authority = request
                .lines()
                .next()
                .and_then(|line| line.strip_prefix("CONNECT "))
                .and_then(|line| line.split_whitespace().next())
                .unwrap_or_default()
                .to_string();
            let _ = request_sender.send(request);
            let upstream = TcpStream::connect(&authority).expect("HTTP proxy upstream");
            client
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .unwrap();

            let mut client_reader = client.try_clone().unwrap();
            let mut upstream_writer = upstream.try_clone().unwrap();
            thread::spawn(move || {
                let _ = io::copy(&mut client_reader, &mut upstream_writer);
                let _ = upstream_writer.shutdown(Shutdown::Write);
            });
            let mut upstream_reader = upstream;
            let _ = io::copy(&mut upstream_reader, &mut client);
            let _ = client.shutdown(Shutdown::Write);
        });
        (address, request_receiver)
    }

    fn spawn_http_proxy_n(
        accepted: Arc<AtomicUsize>,
        expected_connections: usize,
    ) -> (SocketAddr, Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP proxy listener");
        let address = listener.local_addr().unwrap();
        let (request_sender, request_receiver) = mpsc::channel();
        thread::spawn(move || {
            for _ in 0..expected_connections {
                let Ok((mut client, _)) = listener.accept() else {
                    break;
                };
                accepted.fetch_add(1, Ordering::SeqCst);
                let request_sender = request_sender.clone();
                thread::spawn(move || {
                    let request = read_headers(&mut client);
                    let authority = request
                        .lines()
                        .next()
                        .and_then(|line| line.strip_prefix("CONNECT "))
                        .and_then(|line| line.split_whitespace().next())
                        .unwrap_or_default()
                        .to_string();
                    let _ = request_sender.send(request);
                    let upstream = TcpStream::connect(&authority).expect("HTTP proxy upstream");
                    client
                        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                        .unwrap();

                    let mut client_reader = client.try_clone().unwrap();
                    let mut upstream_writer = upstream.try_clone().unwrap();
                    thread::spawn(move || {
                        let _ = io::copy(&mut client_reader, &mut upstream_writer);
                        let _ = upstream_writer.shutdown(Shutdown::Write);
                    });
                    let mut upstream_reader = upstream;
                    let _ = io::copy(&mut upstream_reader, &mut client);
                    let _ = client.shutdown(Shutdown::Write);
                });
            }
        });
        (address, request_receiver)
    }

    fn spawn_socks_proxy_requiring_auth(
        accepted: Arc<AtomicUsize>,
        approve: bool,
    ) -> (SocketAddr, Receiver<SocksAuthObservation>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("authenticated SOCKS listener");
        let address = listener.local_addr().unwrap();
        let (observation_sender, observation_receiver) = mpsc::channel();
        thread::spawn(move || {
            let (mut client, _) = listener.accept().expect("authenticated SOCKS accept");
            accepted.fetch_add(1, Ordering::SeqCst);

            let mut greeting = [0u8; 2];
            client.read_exact(&mut greeting).unwrap();
            let mut methods = vec![0u8; greeting[1] as usize];
            client.read_exact(&mut methods).unwrap();
            assert!(methods.contains(&0x02));
            client.write_all(&[5, 2]).unwrap();

            let mut header = [0u8; 2];
            client.read_exact(&mut header).unwrap();
            assert_eq!(header[0], 1);
            let mut username = vec![0u8; header[1] as usize];
            client.read_exact(&mut username).unwrap();
            let mut password_len = [0u8; 1];
            client.read_exact(&mut password_len).unwrap();
            let mut password = vec![0u8; password_len[0] as usize];
            client.read_exact(&mut password).unwrap();
            client.write_all(&[1, if approve { 0 } else { 1 }]).unwrap();
            if !approve {
                return;
            }

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
                atyp => panic!("unexpected authenticated SOCKS ATYP {atyp}"),
            };
            let mut port = [0u8; 2];
            client.read_exact(&mut port).unwrap();
            let target = format!("{host}:{}", u16::from_be_bytes(port));
            observation_sender
                .send((username, password, target.clone()))
                .unwrap();

            let upstream = TcpStream::connect(&target).expect("authenticated SOCKS upstream");
            client
                .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 80])
                .unwrap();
            let mut upstream_reader = upstream.try_clone().unwrap();
            let mut client_writer = client.try_clone().unwrap();
            thread::spawn(move || {
                let _ = io::copy(&mut upstream_reader, &mut client_writer);
                let _ = client_writer.shutdown(Shutdown::Both);
            });
            let mut upstream_writer = upstream;
            let _ = io::copy(&mut client, &mut upstream_writer);
            let _ = upstream_writer.shutdown(Shutdown::Both);
        });
        (address, observation_receiver)
    }

    fn spawn_socks_proxy_rejecting(reply: u8) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").expect("rejecting SOCKS listener");
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut client, _) = listener.accept().expect("rejecting SOCKS accept");
            let mut greeting = [0u8; 2];
            client.read_exact(&mut greeting).unwrap();
            let mut methods = vec![0u8; greeting[1] as usize];
            client.read_exact(&mut methods).unwrap();
            client.write_all(&[5, 0]).unwrap();

            let mut request = [0u8; 4];
            client.read_exact(&mut request).unwrap();
            let address_len = match request[3] {
                0x01 => 4,
                0x04 => 16,
                0x03 => {
                    let mut length = [0u8; 1];
                    client.read_exact(&mut length).unwrap();
                    length[0] as usize
                }
                atyp => panic!("unexpected rejected SOCKS ATYP {atyp}"),
            };
            let mut ignored = vec![0u8; address_len + 2];
            client.read_exact(&mut ignored).unwrap();
            client
                .write_all(&[5, reply, 0, 1, 127, 0, 0, 1, 0, 80])
                .unwrap();
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

    fn factory_with_two_layer_chain(
        target: &str,
    ) -> (TransportFactory, SocketAddr, Receiver<String>) {
        let inner = spawn_socks_proxy(Arc::new(AtomicUsize::new(0)));
        let config = Socks5ProxyConfig::new("127.0.0.1", inner.port()).unwrap();
        let outer_accepted = Arc::new(AtomicUsize::new(0));
        let (outer, request) = spawn_http_proxy(outer_accepted);
        let route = proxy::resolve_target_with(
            target,
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{outer}")),
                ..ProxyEnvironment::default()
            },
            None,
        )
        .unwrap();
        let connector = OutboundConnector::with_static_route(route)
            .with_specified_socks5(&config)
            .unwrap();
        (
            TransportFactory::with_connector(Arc::new(connector)),
            inner,
            request,
        )
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
    fn websocket_and_http_tunnel_use_the_same_two_layer_factory() {
        let mut ws_server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let ws_remote = format!("127.0.0.1:{}", ws_server.local_addr().unwrap().port());
        let (factory, inner, request) = factory_with_two_layer_chain(&ws_remote);
        let mut client = WSConnection::connect_with_factory(&factory, &ws_remote).unwrap();
        client.send(&[1, 2, 3, 4]).unwrap();
        assert!(
            request
                .recv_timeout(Duration::from_secs(3))
                .unwrap()
                .starts_with(&format!("CONNECT {inner} HTTP/1.1\r\n"))
        );
        assert_eq!(
            client.proxy_context().unwrap().socks5_proxy,
            Some(format!("socks5://{inner}"))
        );

        let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let tunnel_port = tunnel_listener.local_addr().unwrap().port();
        let (tunnel_factory, _tunnel_inner, _tunnel_request) =
            factory_with_two_layer_chain(&ws_remote);
        let mut tunnel =
            WSConnection::tunnel_with_factory(&tunnel_factory, &ws_remote, tunnel_port).unwrap();
        let (mut server_side, _) = TcpConnection::tunnel_server(tunnel_listener, 10).unwrap();
        tunnel.send(&[5, 6, 7, 8]).unwrap();
        assert_eq!(server_side.recv().unwrap(), vec![5, 6, 7, 8]);
        server_side.send(&[9, 10, 11, 12]).unwrap();
        assert_eq!(tunnel.recv().unwrap(), vec![9, 10, 11, 12]);
        ws_server.close();
    }

    #[test]
    fn tcp_main_and_reverse_tunnel_use_the_same_two_layer_factory() {
        let (message_sender, message_receiver) = mpsc::channel();
        let mut server = TcpServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = message_sender.send(message.length());
            }),
        )
        .unwrap();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let (factory, inner, request) = factory_with_two_layer_chain(&remote);
        let mut client = TcpConnection::connect_with_factory(&factory, &remote).unwrap();
        let frame = crate::packet::Message::build(0, "two-layer", serde_json::json!({})).unwrap();
        client.send(&frame).unwrap();
        assert!(
            message_receiver
                .recv_timeout(Duration::from_secs(3))
                .unwrap()
                > 0
        );
        assert!(
            request
                .recv_timeout(Duration::from_secs(3))
                .unwrap()
                .starts_with(&format!("CONNECT {inner} HTTP/1.1\r\n"))
        );
        assert_eq!(
            client.proxy_context().unwrap().socks5_proxy,
            Some(format!("socks5://{inner}"))
        );

        let target_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = target_listener.local_addr().unwrap().port();
        let (tunnel_factory, _tunnel_inner, _tunnel_request) =
            factory_with_two_layer_chain(&remote);
        let mut tunnel =
            TcpConnection::tunnel_with_factory(&tunnel_factory, &remote, port).unwrap();
        let (mut server_side, _) = TcpConnection::tunnel_server(target_listener, 10).unwrap();
        tunnel.send(&[21, 22, 23, 24]).unwrap();
        assert_eq!(server_side.recv().unwrap(), vec![21, 22, 23, 24]);
        server_side.send(&[25, 26, 27, 28]).unwrap();
        assert_eq!(tunnel.recv().unwrap(), vec![25, 26, 27, 28]);
        server.close();
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
    fn specified_socks5_connects_directly_when_no_system_route_exists() {
        let echo = spawn_echo_server();
        let accepted = Arc::new(AtomicUsize::new(0));
        let socks = spawn_socks_proxy(accepted.clone());
        let config = Socks5ProxyConfig::new("127.0.0.1", socks.port()).unwrap();
        let route =
            proxy::resolve_target_with(&echo.to_string(), &ProxyEnvironment::default(), None)
                .unwrap();
        assert!(route.is_direct());
        let connector = OutboundConnector::with_static_route(route)
            .with_specified_socks5(&config)
            .unwrap();
        let mut stream = connector
            .open(&Destination::parse(&echo.to_string()).unwrap())
            .expect("specified SOCKS5 connection");

        stream.write_all(b"direct-specified").unwrap();
        let mut echoed = [0u8; 16];
        stream.read_exact(&mut echoed).unwrap();
        assert_eq!(&echoed, b"direct-specified");
        assert_eq!(accepted.load(Ordering::SeqCst), 1);

        let context = stream.proxy_context().unwrap();
        assert_eq!(context.source, ProxySource::Specified);
        assert_eq!(context.socks5_proxy, Some(format!("socks5://{socks}")));
        assert_eq!(stream.route_report().system_proxy, None);
        assert_eq!(
            stream.route_report().socks5_proxy,
            Some(format!("socks5://{socks}"))
        );
    }

    #[test]
    fn specified_socks5_uses_rfc1929_and_ipv4_ipv6_and_domain_targets() {
        for target in ["127.0.0.1", "::1", "localhost"] {
            let listener = TcpListener::bind(if target == "::1" {
                "[::1]:0"
            } else {
                "127.0.0.1:0"
            });
            let Ok(listener) = listener else {
                continue;
            };
            let address = listener.local_addr().unwrap();
            let accept = thread::spawn(move || listener.accept().map(|(stream, _)| stream));

            let accepted = Arc::new(AtomicUsize::new(0));
            let (socks, observation) = spawn_socks_proxy_requiring_auth(accepted.clone(), true);
            let config = Socks5ProxyConfig::with_auth(
                "127.0.0.1",
                socks.port(),
                "synthetic-user",
                "synthetic-password",
            )
            .unwrap();
            let route = ProxyRoute::direct();
            let connector = OutboundConnector::with_static_route(route)
                .with_specified_socks5(&config)
                .unwrap();
            let destination = Destination::parse(&format!(
                "{}:{}",
                if target.contains(':') {
                    format!("[{target}]")
                } else {
                    target.to_string()
                },
                address.port()
            ))
            .unwrap();
            let stream = connector.open(&destination).unwrap();
            let credentials = observation
                .recv_timeout(Duration::from_secs(3))
                .expect("inner SOCKS observation");
            assert_eq!(credentials.0, b"synthetic-user");
            assert_eq!(credentials.1, b"synthetic-password");
            assert!(credentials.2.ends_with(&address.port().to_string()));
            assert_eq!(accepted.load(Ordering::SeqCst), 1);
            drop(stream);
            drop(accept.join().unwrap().unwrap());
        }
    }

    #[test]
    fn specified_socks5_forms_fixed_two_layer_chains_for_http_https_and_socks5() {
        let echo = spawn_echo_server();
        let inner = spawn_socks_proxy(Arc::new(AtomicUsize::new(0)));
        let config = Socks5ProxyConfig::new("127.0.0.1", inner.port()).unwrap();
        let outer_http_accepted = Arc::new(AtomicUsize::new(0));
        let (outer_http, http_request) = spawn_http_proxy(outer_http_accepted.clone());
        let http_route = proxy::resolve_target_with(
            &echo.to_string(),
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{outer_http}")),
                ..ProxyEnvironment::default()
            },
            None,
        )
        .unwrap();
        let http_connector = OutboundConnector::with_static_route(http_route)
            .with_specified_socks5(&config)
            .unwrap();
        let mut stream = http_connector
            .open(&Destination::parse(&echo.to_string()).unwrap())
            .expect("HTTP outer SOCKS5 chain");
        stream.write_all(b"http-outer").unwrap();
        let mut echoed = [0u8; 10];
        stream.read_exact(&mut echoed).unwrap();
        assert_eq!(&echoed, b"http-outer");
        let request = http_request
            .recv_timeout(Duration::from_secs(3))
            .expect("HTTP outer CONNECT");
        assert!(request.starts_with(&format!("CONNECT {inner} HTTP/1.1\r\n")));
        assert_eq!(outer_http_accepted.load(Ordering::SeqCst), 1);

        let echo_https = spawn_echo_server();
        let inner_https = spawn_socks_proxy(Arc::new(AtomicUsize::new(0)));
        let config_https = Socks5ProxyConfig::new("127.0.0.1", inner_https.port()).unwrap();
        let outer_https_accepted = Arc::new(AtomicUsize::new(0));
        let proxy_identity = identity(
            include_bytes!("../../tests/fixtures/tls/localhost.pem"),
            include_bytes!("../../tests/fixtures/tls/localhost.key.pem"),
        );
        let (outer_https, https_request) = spawn_tls_proxy(proxy_identity, outer_https_accepted);
        let https_route = route_to_https_proxy(outer_https.port(), &echo_https.to_string());
        let https_connector = OutboundConnector::with_static_route(https_route)
            .with_tls_connector(test_tls_connector())
            .with_specified_socks5(&config_https)
            .unwrap();
        let mut stream = https_connector
            .open(&Destination::parse(&echo_https.to_string()).unwrap())
            .expect("HTTPS outer SOCKS5 chain");
        stream.write_all(b"https-outer").unwrap();
        let mut https_echoed = [0u8; 11];
        stream.read_exact(&mut https_echoed).unwrap();
        assert_eq!(&https_echoed, b"https-outer");
        let request = https_request
            .recv_timeout(Duration::from_secs(3))
            .expect("HTTPS outer CONNECT");
        assert!(request.starts_with(&format!("CONNECT {inner_https} HTTP/1.1\r\n")));

        let echo_socks = spawn_echo_server();
        let inner_socks = spawn_socks_proxy(Arc::new(AtomicUsize::new(0)));
        let config_socks = Socks5ProxyConfig::new("127.0.0.1", inner_socks.port()).unwrap();
        let outer_socks_accepted = Arc::new(AtomicUsize::new(0));
        let outer_socks = spawn_socks_proxy(outer_socks_accepted.clone());
        let socks_route = proxy::resolve_target_with(
            &echo_socks.to_string(),
            &ProxyEnvironment {
                all_proxy: Some(format!("socks5://{outer_socks}")),
                ..ProxyEnvironment::default()
            },
            None,
        )
        .unwrap();
        let socks_connector = OutboundConnector::with_static_route(socks_route)
            .with_specified_socks5(&config_socks)
            .unwrap();
        let mut stream = socks_connector
            .open(&Destination::parse(&echo_socks.to_string()).unwrap())
            .expect("SOCKS5 outer SOCKS5 chain");
        stream.write_all(b"socks-outer").unwrap();
        let mut socks_echoed = [0u8; 11];
        stream.read_exact(&mut socks_echoed).unwrap();
        assert_eq!(&socks_echoed, b"socks-outer");

        let context = stream.proxy_context().unwrap();
        assert_eq!(
            context.system_proxy,
            Some(format!("socks5://{outer_socks}"))
        );
        assert_eq!(
            context.socks5_proxy,
            Some(format!("socks5://{inner_socks}"))
        );
    }

    #[test]
    fn no_proxy_bypasses_only_the_outer_route_and_keeps_the_specified_socks5() {
        let echo = spawn_echo_server();
        let outer_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        outer_listener.set_nonblocking(true).unwrap();
        let inner_accepted = Arc::new(AtomicUsize::new(0));
        let inner = spawn_socks_proxy(inner_accepted.clone());
        let config = Socks5ProxyConfig::new("127.0.0.1", inner.port()).unwrap();
        let route = proxy::resolve_target_with(
            &echo.to_string(),
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{}", outer_listener.local_addr().unwrap())),
                no_proxy: Some("127.0.0.1".to_string()),
                ..ProxyEnvironment::default()
            },
            None,
        )
        .unwrap();
        assert!(route.is_direct());
        assert!(route.was_bypassed());
        let connector = OutboundConnector::with_static_route(route)
            .with_specified_socks5(&config)
            .unwrap();
        let mut stream = connector
            .open(&Destination::parse(&echo.to_string()).unwrap())
            .unwrap();
        stream.write_all(b"bypass-outer").unwrap();
        assert_eq!(inner_accepted.load(Ordering::SeqCst), 1);
        assert!(outer_listener.accept().is_err());
        assert_eq!(
            stream.proxy_context().unwrap().source,
            ProxySource::Specified
        );
        assert!(stream.route_report().system_bypassed);
    }

    fn exercise_frozen_route_tunnel(
        protocol: crate::HeroinnProtocol,
        factory: &TransportFactory,
    ) -> String {
        let mut tcp_server = None;
        let mut ws_server = None;
        let (remote, main) = match protocol {
            crate::HeroinnProtocol::TCP => {
                let server = TcpServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
                let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
                let main =
                    crate::protocol::Transport::connect_with_factory(factory, &protocol, &remote)
                        .expect("TCP main connection through route");
                tcp_server = Some(server);
                (remote, main)
            }
            crate::HeroinnProtocol::HTTP => {
                let server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
                let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
                let main =
                    crate::protocol::Transport::connect_with_factory(factory, &protocol, &remote)
                        .expect("HTTP main connection through route");
                ws_server = Some(server);
                (remote, main)
            }
            crate::HeroinnProtocol::UDP => panic!("UDP has no frozen proxy route"),
        };
        let frozen = main
            .frozen_route()
            .cloned()
            .expect("main connection must freeze its route");
        let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let tunnel_port = tunnel_listener.local_addr().unwrap().port();
        let mut tunnel = crate::protocol::Transport::tunnel_with_frozen_route(
            factory,
            &protocol,
            &remote,
            tunnel_port,
            &frozen,
        )
        .expect("frozen-route reverse tunnel");
        let (mut server_side, _) =
            TcpConnection::tunnel_server(tunnel_listener, 10).expect("tunnel accept");

        tunnel.send(b"frozen-route-to-server").unwrap();
        assert_eq!(server_side.recv().unwrap(), b"frozen-route-to-server");
        server_side.send(b"frozen-route-to-client").unwrap();
        assert_eq!(tunnel.recv().unwrap(), b"frozen-route-to-client");

        if let Some(mut server) = tcp_server {
            server.close();
        }
        if let Some(mut server) = ws_server {
            server.close();
        }
        remote
    }

    #[test]
    fn tcp_and_http_frozen_tunnels_reuse_http_connect_route() {
        for protocol in [crate::HeroinnProtocol::TCP, crate::HeroinnProtocol::HTTP] {
            let accepted = Arc::new(AtomicUsize::new(0));
            let (proxy, requests) = spawn_http_proxy_n(accepted.clone(), 2);
            let route = proxy::resolve_target_with(
                "127.0.0.1:1",
                &ProxyEnvironment {
                    http_proxy: Some(format!("http://{proxy}")),
                    ..ProxyEnvironment::default()
                },
                None,
            )
            .unwrap();
            let factory = TransportFactory::with_connector(Arc::new(
                OutboundConnector::with_static_route(route),
            ));
            let remote = exercise_frozen_route_tunnel(protocol, &factory);
            assert_eq!(accepted.load(Ordering::SeqCst), 2);
            for _ in 0..2 {
                let request = requests.recv_timeout(Duration::from_secs(3)).unwrap();
                assert!(request.starts_with(&format!("CONNECT {remote} HTTP/1.1\r\n")));
            }
        }
    }

    #[test]
    fn tcp_and_http_frozen_tunnels_reuse_direct_route() {
        for protocol in [crate::HeroinnProtocol::TCP, crate::HeroinnProtocol::HTTP] {
            let factory = TransportFactory::direct();
            exercise_frozen_route_tunnel(protocol, &factory);
        }
    }

    #[test]
    fn tcp_and_http_frozen_tunnels_reuse_https_tls_proxy_route() {
        for protocol in [crate::HeroinnProtocol::TCP, crate::HeroinnProtocol::HTTP] {
            let accepted = Arc::new(AtomicUsize::new(0));
            let identity = identity(
                include_bytes!("../../tests/fixtures/tls/localhost.pem"),
                include_bytes!("../../tests/fixtures/tls/localhost.key.pem"),
            );
            let (proxy, requests) = spawn_tls_proxy_n(identity, accepted.clone(), 2);
            let route = route_to_https_proxy(proxy.port(), "127.0.0.1:1");
            let connector = OutboundConnector::with_static_route(route)
                .with_tls_connector(test_tls_connector());
            let factory = TransportFactory::with_connector(Arc::new(connector));
            let remote = exercise_frozen_route_tunnel(protocol, &factory);
            assert_eq!(accepted.load(Ordering::SeqCst), 2);
            for _ in 0..2 {
                let request = requests.recv_timeout(Duration::from_secs(3)).unwrap();
                assert!(request.starts_with(&format!("CONNECT {remote} HTTP/1.1\r\n")));
            }
        }
    }

    #[test]
    fn tcp_and_http_frozen_tunnels_reuse_socks5_route() {
        for protocol in [crate::HeroinnProtocol::TCP, crate::HeroinnProtocol::HTTP] {
            let accepted = Arc::new(AtomicUsize::new(0));
            let (proxy, authorities) = spawn_socks_proxy_n(accepted.clone(), 2);
            let route = route_to_socks_proxy(proxy.port(), "127.0.0.1:1");
            let factory = TransportFactory::with_connector(Arc::new(
                OutboundConnector::with_static_route(route),
            ));
            let remote = exercise_frozen_route_tunnel(protocol, &factory);
            assert_eq!(accepted.load(Ordering::SeqCst), 2);
            for _ in 0..2 {
                assert_eq!(
                    authorities.recv_timeout(Duration::from_secs(3)).unwrap(),
                    remote
                );
            }
        }
    }

    #[test]
    fn tcp_and_http_frozen_tunnels_reuse_two_layer_system_and_specified_route() {
        for protocol in [crate::HeroinnProtocol::TCP, crate::HeroinnProtocol::HTTP] {
            let outer_accepted = Arc::new(AtomicUsize::new(0));
            let (outer, outer_requests) = spawn_http_proxy_n(outer_accepted.clone(), 2);
            let inner_accepted = Arc::new(AtomicUsize::new(0));
            let (inner, inner_authorities) = spawn_socks_proxy_n(inner_accepted.clone(), 2);
            let config = Socks5ProxyConfig::new("127.0.0.1", inner.port()).unwrap();
            let route = proxy::resolve_target_with(
                "127.0.0.1:1",
                &ProxyEnvironment {
                    http_proxy: Some(format!("http://{outer}")),
                    ..ProxyEnvironment::default()
                },
                None,
            )
            .unwrap();
            let connector = OutboundConnector::with_static_route(route)
                .with_specified_socks5(&config)
                .unwrap();
            let factory = TransportFactory::with_connector(Arc::new(connector));
            let remote = exercise_frozen_route_tunnel(protocol, &factory);
            assert_eq!(outer_accepted.load(Ordering::SeqCst), 2);
            assert_eq!(inner_accepted.load(Ordering::SeqCst), 2);
            for _ in 0..2 {
                let request = outer_requests.recv_timeout(Duration::from_secs(3)).unwrap();
                assert!(request.starts_with(&format!("CONNECT {inner} HTTP/1.1\r\n")));
                assert_eq!(
                    inner_authorities
                        .recv_timeout(Duration::from_secs(3))
                        .unwrap(),
                    remote
                );
            }
        }
    }

    #[test]
    fn tcp_and_http_frozen_tunnels_reuse_no_proxy_bypass_route() {
        for protocol in [crate::HeroinnProtocol::TCP, crate::HeroinnProtocol::HTTP] {
            let outer = TcpListener::bind("127.0.0.1:0").unwrap();
            outer.set_nonblocking(true).unwrap();
            let inner_accepted = Arc::new(AtomicUsize::new(0));
            let (inner, inner_authorities) = spawn_socks_proxy_n(inner_accepted.clone(), 2);
            let config = Socks5ProxyConfig::new("127.0.0.1", inner.port()).unwrap();
            let route = proxy::resolve_target_with(
                "127.0.0.1:1",
                &ProxyEnvironment {
                    http_proxy: Some(format!("http://{}", outer.local_addr().unwrap())),
                    no_proxy: Some("127.0.0.1".to_string()),
                    ..ProxyEnvironment::default()
                },
                None,
            )
            .unwrap();
            assert!(route.was_bypassed());
            let connector = OutboundConnector::with_static_route(route)
                .with_specified_socks5(&config)
                .unwrap();
            let factory = TransportFactory::with_connector(Arc::new(connector));
            let remote = exercise_frozen_route_tunnel(protocol, &factory);
            assert_eq!(inner_accepted.load(Ordering::SeqCst), 2);
            for _ in 0..2 {
                assert_eq!(
                    inner_authorities
                        .recv_timeout(Duration::from_secs(3))
                        .unwrap(),
                    remote
                );
            }
            assert!(
                outer.accept().is_err(),
                "bypassed outer proxy received a frozen reverse tunnel"
            );
        }
    }

    #[test]
    fn tcp_and_http_frozen_tunnels_reuse_direct_no_proxy_bypass_route() {
        for protocol in [crate::HeroinnProtocol::TCP, crate::HeroinnProtocol::HTTP] {
            let outer = TcpListener::bind("127.0.0.1:0").unwrap();
            outer.set_nonblocking(true).unwrap();
            let route = proxy::resolve_target_with(
                "127.0.0.1:1",
                &ProxyEnvironment {
                    http_proxy: Some(format!("http://{}", outer.local_addr().unwrap())),
                    no_proxy: Some("127.0.0.1".to_string()),
                    ..ProxyEnvironment::default()
                },
                None,
            )
            .unwrap();
            assert!(route.was_bypassed());
            let factory = TransportFactory::with_connector(Arc::new(
                OutboundConnector::with_static_route(route),
            ));
            exercise_frozen_route_tunnel(protocol, &factory);
            assert!(
                outer.accept().is_err(),
                "bypassed outer proxy received a direct frozen reverse tunnel"
            );
        }
    }

    #[test]
    fn frozen_route_never_rediscovers_or_falls_back_to_direct() {
        let target_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let target = target_listener.local_addr().unwrap();
        let proxy_accepted = Arc::new(AtomicUsize::new(0));
        let (proxy, _request) = spawn_http_proxy(proxy_accepted.clone());
        let route = proxy::resolve_target_with(
            &target.to_string(),
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{proxy}")),
                ..ProxyEnvironment::default()
            },
            None,
        )
        .unwrap();
        let connector = OutboundConnector::with_static_route(route);
        let main = connector
            .open(&Destination::parse(&target.to_string()).unwrap())
            .unwrap();
        let (first_target, _) = target_listener.accept().unwrap();
        drop(first_target);
        let frozen = main.frozen_route().cloned().expect("frozen route");
        drop(main);
        assert_eq!(proxy_accepted.load(Ordering::SeqCst), 1);

        let error = connector
            .open_frozen(&Destination::parse(&target.to_string()).unwrap(), &frozen)
            .unwrap_err();
        assert!(matches!(error, crate::Error::Proxy(_)));

        target_listener.set_nonblocking(true).unwrap();
        assert!(target_listener.accept().is_err());
        assert_eq!(proxy_accepted.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn outer_failure_is_fail_closed_and_never_dials_the_specified_proxy_or_server() {
        let echo = spawn_echo_server();
        let dead = TcpListener::bind("127.0.0.1:0").unwrap();
        let dead_address = dead.local_addr().unwrap();
        drop(dead);
        let inner_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        inner_listener.set_nonblocking(true).unwrap();
        let config =
            Socks5ProxyConfig::new("127.0.0.1", inner_listener.local_addr().unwrap().port())
                .unwrap();
        let route = proxy::resolve_target_with(
            &echo.to_string(),
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{dead_address}")),
                ..ProxyEnvironment::default()
            },
            None,
        )
        .unwrap();
        let connector = OutboundConnector::with_static_route(route)
            .with_specified_socks5(&config)
            .unwrap();
        assert!(
            connector
                .open(&Destination::parse(&echo.to_string()).unwrap())
                .is_err()
        );
        assert!(inner_listener.accept().is_err());
    }

    #[test]
    fn inner_authentication_failure_does_not_retry_the_outer_route() {
        let echo = spawn_echo_server();
        let outer_accepted = Arc::new(AtomicUsize::new(0));
        let (outer_http, _request) = spawn_http_proxy(outer_accepted.clone());
        let second_outer = TcpListener::bind("127.0.0.1:0").unwrap();
        second_outer.set_nonblocking(true).unwrap();
        let inner_accepted = Arc::new(AtomicUsize::new(0));
        let (inner, _observation) = spawn_socks_proxy_requiring_auth(inner_accepted.clone(), false);
        let config = Socks5ProxyConfig::with_auth(
            "127.0.0.1",
            inner.port(),
            "synthetic-user",
            "synthetic-password",
        )
        .unwrap();
        let route = proxy::resolve_target_with(
            &echo.to_string(),
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{outer_http}")),
                https_proxy: Some(format!("https://{}", second_outer.local_addr().unwrap())),
                ..ProxyEnvironment::default()
            },
            None,
        )
        .unwrap();
        let connector = OutboundConnector::with_static_route(route)
            .with_tls_connector(test_tls_connector())
            .with_specified_socks5(&config)
            .unwrap();
        let error = connector
            .open(&Destination::parse(&echo.to_string()).unwrap())
            .unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected specified SOCKS5 error");
        };
        let failure = error.failure();
        assert_eq!(failure.kind, ProxyFailureKind::SocksAuthentication);
        assert_eq!(failure.source, Some(ProxySource::Specified));
        assert_eq!(failure.endpoint, Some(format!("socks5://{inner}")));
        assert!(!format!("{error:?}").contains("synthetic-user"));
        assert!(!failure.detail.contains("synthetic-password"));
        assert_eq!(outer_accepted.load(Ordering::SeqCst), 1);
        assert_eq!(inner_accepted.load(Ordering::SeqCst), 1);
        assert!(second_outer.accept().is_err());
    }

    #[test]
    fn specified_target_rejection_is_fail_closed_and_credential_free() {
        let proxy = spawn_socks_proxy_rejecting(0x05);
        let config = Socks5ProxyConfig::with_auth(
            "127.0.0.1",
            proxy.port(),
            "synthetic-user",
            "synthetic-password",
        )
        .unwrap();
        let connector = OutboundConnector::with_static_route(ProxyRoute::direct())
            .with_specified_socks5(&config)
            .unwrap();
        let destination = Destination::parse("server.example:443").unwrap();
        let error = connector.open(&destination).unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected specified SOCKS5 error");
        };
        let failure = error.failure();
        assert_eq!(failure.kind, ProxyFailureKind::SocksTargetRejected);
        assert_eq!(failure.source, Some(ProxySource::Specified));
        assert_eq!(failure.endpoint, Some(format!("socks5://{proxy}")));
        assert!(!format!("{error:?}").contains("synthetic-user"));
        assert!(!failure.detail.contains("synthetic-password"));
    }

    #[test]
    fn outer_candidates_may_fall_back_before_the_inner_socks5_succeeds() {
        let echo = spawn_echo_server();
        let dead_http = TcpListener::bind("127.0.0.1:0").unwrap();
        let dead_http_address = dead_http.local_addr().unwrap();
        drop(dead_http);
        let inner = spawn_socks_proxy(Arc::new(AtomicUsize::new(0)));
        let config = Socks5ProxyConfig::new("127.0.0.1", inner.port()).unwrap();
        let outer_accepted = Arc::new(AtomicUsize::new(0));
        let proxy_identity = identity(
            include_bytes!("../../tests/fixtures/tls/localhost.pem"),
            include_bytes!("../../tests/fixtures/tls/localhost.key.pem"),
        );
        let (outer_https, request) = spawn_tls_proxy(proxy_identity, outer_accepted);
        let route = proxy::resolve_target_with(
            &echo.to_string(),
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{dead_http_address}")),
                https_proxy: Some(format!("https://localhost:{}", outer_https.port())),
                ..ProxyEnvironment::default()
            },
            None,
        )
        .unwrap();
        let connector = OutboundConnector::with_static_route(route)
            .with_tls_connector(test_tls_connector())
            .with_specified_socks5(&config)
            .unwrap();
        let mut stream = connector
            .open(&Destination::parse(&echo.to_string()).unwrap())
            .expect("fallback outer then inner SOCKS5");
        stream.write_all(b"outer-fallback").unwrap();
        let mut echoed = [0u8; 14];
        stream.read_exact(&mut echoed).unwrap();
        assert_eq!(&echoed, b"outer-fallback");
        let request = request.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(request.starts_with(&format!("CONNECT {inner} HTTP/1.1\r\n")));
        assert_eq!(
            stream.proxy_context().unwrap().system_proxy,
            Some(format!("https://localhost:{}", outer_https.port()))
        );
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
        // Windows timer granularity can report a deadline a few milliseconds
        // early. Keep the upper bound strict so an immediate/direct failure
        // cannot satisfy this timeout regression.
        assert!(started.elapsed() + Duration::from_millis(250) >= proxy::PROXY_HANDSHAKE_TIMEOUT);
        assert!(started.elapsed() < proxy::PROXY_HANDSHAKE_TIMEOUT + Duration::from_secs(5));
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
            specified_socks5: None,
        };

        let error = connector.open(&destination).unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected a proxy TLS error");
        };
        assert_eq!(error.failure().kind, ProxyFailureKind::TlsHandshake);
        assert!(target_listener.accept().is_err());
    }
}
