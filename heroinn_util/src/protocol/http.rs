//! WebSocket transport using tungstenite's synchronous stream API.
//!
//! The proxy-aware `OutboundStream` is established before the WebSocket
//! handshake. Tungstenite therefore never performs its own DNS, proxy or TLS
//! work; it only speaks RFC 6455 over the already-connected byte stream.

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tungstenite::client::client_with_config;
use tungstenite::handshake::HandshakeError;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

use crate::HeroinnProtocol;
use crate::error::{Error, Result};
use crate::packet::Message as ProtocolMessage;
use crate::protocol::noise::{
    NOISE_HANDSHAKE_TIMEOUT, NoiseIntro, NoiseKind, NoiseSession, NoiseTransport, NoiseWire,
    STREAM_RECORD_MAX, STREAM_RECORD_PAYLOAD, handshake_initiator, handshake_responder,
};
use crate::protocol::outbound::{
    OutboundConnector, OutboundReader, OutboundStream, ProxyContext, RouteReport, SocketWake,
    TransportFactory, wait_tcp_activity, wait_tcp_listener,
};
use crate::protocol::proxy::{Destination, FrozenRoute, ProxyRoute};
use crate::protocol::tcp::TcpConnection;
use crate::protocol::{DisconnectHandler, MessageHandler, TUNNEL_FLAG};

const WS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const WS_HANDSHAKE_POLL: Duration = Duration::from_millis(250);
const WS_ACTIVE_READ_TIMEOUT: Duration = Duration::from_millis(1);
const WS_IDLE_READ_TIMEOUT: Duration = Duration::from_millis(25);
const WS_ACTIVE_WINDOW: Duration = Duration::from_millis(100);
const WS_WRITE_TIMEOUT: Duration = Duration::from_millis(1);
const WS_CLOSE_FLUSH_TIMEOUT: Duration = Duration::from_secs(1);
const WS_SEND_TIMEOUT: Duration = Duration::from_secs(5);
const WS_SEND_QUEUE: usize = 32;
// A full 10 MiB Noise message arrives as a series of 60 KiB records. Drain a
// bounded slice of the already-bounded send queue and issue one flush for the
// batch instead of paying a syscall and wakeup per record.
const WS_WRITE_BATCH: usize = 8;
const WS_RECV_QUEUE: usize = 8;
const TUNNEL_IDLE_POLL: Duration = Duration::from_millis(100);
const WS_MAX_MESSAGE_SIZE: usize = 100 * 1024 * 1024;
const WS_MAX_FRAME_SIZE: usize = 100 * 1024 * 1024;
const WS_MAX_WRITE_BUFFER: usize = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConnectionEnd {
    Closed,
    Tunnel,
}

enum WsEvent {
    Binary(Vec<u8>),
    Closed,
    Error(String),
}

enum WsCommand {
    Binary(Vec<u8>),
    Flush(mpsc::SyncSender<()>),
}

struct WsActorControl {
    stop: Arc<AtomicBool>,
    close_requested: Arc<AtomicBool>,
    close_ack: Arc<Mutex<Option<mpsc::SyncSender<()>>>>,
    send_space: Arc<(Mutex<()>, Condvar)>,
    wake: Arc<SocketWake>,
}

trait WsReadControl {
    fn set_ws_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()>;
    fn enable_ws_nonblocking_poll(&self) -> std::io::Result<bool>;
    fn wait_ws_activity(
        &self,
        wake: &SocketWake,
        readable: bool,
        writable: bool,
        timeout: Duration,
    ) -> std::io::Result<bool>;
}

impl WsReadControl for std::net::TcpStream {
    fn set_ws_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.set_read_timeout(timeout)
    }

    fn enable_ws_nonblocking_poll(&self) -> std::io::Result<bool> {
        self.set_nonblocking(true)?;
        Ok(true)
    }

    fn wait_ws_activity(
        &self,
        wake: &SocketWake,
        readable: bool,
        writable: bool,
        timeout: Duration,
    ) -> std::io::Result<bool> {
        wait_tcp_activity(self, wake, readable, writable, timeout)
    }
}

impl WsReadControl for OutboundStream {
    fn set_ws_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.set_read_timeout(timeout)
    }

    fn enable_ws_nonblocking_poll(&self) -> std::io::Result<bool> {
        self.enable_nonblocking_poll()
    }

    fn wait_ws_activity(
        &self,
        wake: &SocketWake,
        readable: bool,
        writable: bool,
        timeout: Duration,
    ) -> std::io::Result<bool> {
        self.wait_ws_activity(wake, readable, writable, timeout)
    }
}

/// A bounded, cancellable owner for one tungstenite socket.
///
/// Reads and writes are serialized by the actor so fragmented messages,
/// automatic Pong replies and Close handshakes remain ordered. User sends
/// enter a bounded queue and `recv` consumes a separate bounded queue, so a
/// slow reader applies backpressure without holding the socket mutex across a
/// blocking read.
struct WsSession {
    send_tx: mpsc::SyncSender<WsCommand>,
    recv_rx: Mutex<mpsc::Receiver<WsEvent>>,
    stop: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
    close_requested: Arc<AtomicBool>,
    close_ack: Arc<Mutex<Option<mpsc::SyncSender<()>>>>,
    close_lock: Mutex<()>,
    send_lock: Mutex<()>,
    send_space: Arc<(Mutex<()>, Condvar)>,
    wake: Arc<SocketWake>,
    close: Arc<dyn Fn(Shutdown) + Send + Sync>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl WsSession {
    fn start<S>(
        socket: WebSocket<S>,
        close: Arc<dyn Fn(Shutdown) + Send + Sync>,
    ) -> Result<Arc<Self>>
    where
        S: Read + Write + WsReadControl + Send + 'static,
    {
        let (send_tx, send_rx) = mpsc::sync_channel(WS_SEND_QUEUE);
        let (recv_tx, recv_rx) = mpsc::sync_channel(WS_RECV_QUEUE);
        let stop = Arc::new(AtomicBool::new(false));
        let actor_stop = stop.clone();
        let done = Arc::new(AtomicBool::new(false));
        let actor_done = done.clone();
        let close_requested = Arc::new(AtomicBool::new(false));
        let actor_close_requested = close_requested.clone();
        let close_ack = Arc::new(Mutex::new(None));
        let actor_close_ack = close_ack.clone();
        let send_space = Arc::new((Mutex::new(()), Condvar::new()));
        let actor_send_space = send_space.clone();
        let wake = Arc::new(SocketWake::new().map_err(Error::Io)?);
        let actor_wake = wake.clone();
        let join = thread::Builder::new()
            .name("ws-actor".to_string())
            .spawn(move || {
                ws_actor(
                    socket,
                    send_rx,
                    recv_tx,
                    WsActorControl {
                        stop: actor_stop,
                        close_requested: actor_close_requested,
                        close_ack: actor_close_ack,
                        send_space: actor_send_space,
                        wake: actor_wake,
                    },
                );
                actor_done.store(true, Ordering::Release);
            })
            .map_err(|error| Error::Other(format!("cannot start ws actor: {error}")))?;

        Ok(Arc::new(Self {
            send_tx,
            recv_rx: Mutex::new(recv_rx),
            stop,
            done,
            close_requested,
            close_ack,
            close_lock: Mutex::new(()),
            send_lock: Mutex::new(()),
            send_space,
            wake,
            close,
            join: Mutex::new(Some(join)),
        }))
    }

    fn send(&self, payload: &[u8]) -> Result<()> {
        self.send_with_deadline(payload, WS_SEND_TIMEOUT)
    }

    fn send_with_deadline(&self, payload: &[u8], timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        let mut command = WsCommand::Binary(payload.to_vec());
        loop {
            {
                let _send_guard = self
                    .send_lock
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if self.stop.load(Ordering::Acquire) || self.close_requested.load(Ordering::Acquire)
                {
                    return Err(Error::Other("ws closed".to_string()));
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(Error::Timeout);
                }
                let (space, wake) = &*self.send_space;
                let space = space
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                match self.send_tx.try_send(command) {
                    Ok(()) => {
                        self.wake.notify();
                        return Ok(());
                    }
                    Err(mpsc::TrySendError::Full(returned)) => command = returned,
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        return Err(Error::Other("ws writer stopped".to_string()));
                    }
                }
                let _ = wake.wait_timeout(space, remaining);
            }
        }
    }

    fn flush(&self) -> Result<()> {
        let deadline = Instant::now() + WS_SEND_TIMEOUT;
        let (ack_tx, ack_rx) = mpsc::sync_channel(1);
        let mut command = WsCommand::Flush(ack_tx);
        loop {
            {
                let _send_guard = self
                    .send_lock
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if self.stop.load(Ordering::Acquire) || self.close_requested.load(Ordering::Acquire)
                {
                    return Err(Error::Other("ws closed".to_string()));
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(Error::Timeout);
                }
                let (space, wake) = &*self.send_space;
                let space = space
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                match self.send_tx.try_send(command) {
                    Ok(()) => {
                        self.wake.notify();
                        break;
                    }
                    Err(mpsc::TrySendError::Full(returned)) => command = returned,
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        return Err(Error::Other("ws writer stopped".to_string()));
                    }
                }
                let _ = wake.wait_timeout(space, remaining);
            }
        }
        match ack_rx.recv_timeout(WS_SEND_TIMEOUT) {
            Ok(()) => Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(Error::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(Error::Other("ws writer stopped".to_string()))
            }
        }
    }

    fn recv(&self) -> Result<Vec<u8>> {
        let receiver = self
            .recv_rx
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match receiver.recv() {
            Ok(WsEvent::Binary(payload)) => {
                self.wake.notify();
                Ok(payload)
            }
            Ok(WsEvent::Closed) => Err(Error::Other("ws closed".to_string())),
            Ok(WsEvent::Error(detail)) => Err(Error::Other(detail)),
            Err(_) => Err(Error::Other("ws reader stopped".to_string())),
        }
    }

    fn recv_timeout(&self, timeout: Duration) -> Result<Vec<u8>> {
        let receiver = self
            .recv_rx
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match receiver.recv_timeout(timeout) {
            Ok(WsEvent::Binary(payload)) => {
                self.wake.notify();
                Ok(payload)
            }
            Ok(WsEvent::Closed) => Err(Error::Other("ws closed".to_string())),
            Ok(WsEvent::Error(detail)) => Err(Error::Other(detail)),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(Error::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(Error::Other("ws reader stopped".to_string()))
            }
        }
    }

    fn shutdown(&self) {
        self.stop.store(true, Ordering::Release);
        self.wake.notify();
        (self.close)(Shutdown::Both);
    }

    fn join(&self) {
        let handle = self
            .join
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }

    #[cfg(test)]
    fn actor_finished(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    fn close_and_join(&self) {
        let _close_guard = self
            .close_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if self.done.load(Ordering::Acquire) {
            self.shutdown();
            self.join();
            return;
        }
        let (ack_tx, ack_rx) = mpsc::sync_channel(1);
        {
            let _send_guard = self
                .send_lock
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *self
                .close_ack
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(ack_tx);
            self.close_requested.store(true, Ordering::Release);
            self.wake.notify();
            self.send_space.1.notify_all();
        }
        {
            let _ = ack_rx.recv_timeout(WS_CLOSE_FLUSH_TIMEOUT);
        }
        self.shutdown();
        self.join();
    }
}

impl Drop for WsSession {
    fn drop(&mut self) {
        self.close_and_join();
    }
}

struct WsNoiseWire {
    session: Arc<WsSession>,
}

impl NoiseWire for WsNoiseWire {
    fn send_plain(&self, payload: &[u8]) -> Result<()> {
        self.session.send(payload)
    }

    fn flush_after_send(&self) -> bool {
        // The WS actor owns a bounded writer queue and flushes every drained
        // batch. Waiting for an explicit ack after every Noise message turns
        // one bulk transfer into hundreds of poll/futex round trips.
        false
    }

    fn send_plain_timeout(&self, payload: &[u8], timeout: Duration) -> Result<()> {
        self.session.send_with_deadline(payload, timeout)
    }

    fn recv_plain(&self) -> Result<Vec<u8>> {
        self.session.recv()
    }

    fn recv_plain_timeout(&self, timeout: Duration) -> Result<Vec<u8>> {
        self.session.recv_timeout(timeout)
    }

    fn flush_plain(&self) -> Result<()> {
        self.session.flush()
    }
}

fn ws_config() -> WebSocketConfig {
    WebSocketConfig::default()
        .read_buffer_size(16 * 1024)
        .write_buffer_size(0)
        .max_write_buffer_size(WS_MAX_WRITE_BUFFER)
        .max_message_size(Some(WS_MAX_MESSAGE_SIZE))
        .max_frame_size(Some(WS_MAX_FRAME_SIZE))
}

fn is_transient_io(error: &tungstenite::Error) -> bool {
    matches!(
        error,
        tungstenite::Error::Io(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock
                    | std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::Interrupted
            ) || is_windows_io_pending(error)
    )
}

fn is_windows_io_pending(error: &std::io::Error) -> bool {
    // std's Windows socket timeout path can surface ERROR_IO_PENDING instead
    // of WouldBlock/TimedOut when a short read timeout expires. The frame
    // parser preserves its partial buffer, so retrying is safe.
    cfg!(windows) && error.raw_os_error() == Some(997)
}

fn is_closed_error(error: &tungstenite::Error) -> bool {
    matches!(
        error,
        tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed
    )
}

fn ws_actor<S>(
    mut socket: WebSocket<S>,
    send_rx: mpsc::Receiver<WsCommand>,
    recv_tx: mpsc::SyncSender<WsEvent>,
    control: WsActorControl,
) where
    S: Read + Write + WsReadControl,
{
    let WsActorControl {
        stop,
        close_requested,
        close_ack,
        send_space,
        wake,
    } = control;
    let mut pending_in: Option<Vec<u8>> = None;
    let mut pending_out = VecDeque::new();
    let mut pending_flush: Option<mpsc::SyncSender<()>> = None;
    let mut flush_pending = false;
    let mut closed_sent = false;
    let mut finished = false;
    let mut last_activity = Instant::now();
    let mut read_timeout = Duration::ZERO;
    let native_polling = match socket.get_ref().enable_ws_nonblocking_poll() {
        Ok(enabled) => enabled,
        Err(error) => {
            log::debug!("ws nonblocking polling setup failed: {error}");
            false
        }
    };

    while !stop.load(Ordering::Acquire) && !finished {
        wake.drain();
        let mut progressed = false;
        // Automatic control replies (notably Pong) must reach the peer before
        // another read can block on an idle connection.
        if flush_pending {
            match socket.flush() {
                Ok(()) => {
                    last_activity = Instant::now();
                    progressed = true;
                    flush_pending = false;
                    if let Some(ack) = pending_flush.take() {
                        let _ = ack.try_send(());
                    }
                }
                Err(error) if is_transient_io(&error) => {}
                Err(error) if is_closed_error(&error) => finished = true,
                Err(error) => {
                    log::warn!("ws flush error: {error}");
                    send_ws_event(&recv_tx, WsEvent::Error(format!("ws flush error: {error}")));
                    finished = true;
                }
            }
            if finished {
                break;
            }
        }

        if pending_flush.is_some() && pending_out.is_empty() && !flush_pending {
            let result = socket.flush();
            match result {
                Ok(()) => {
                    last_activity = Instant::now();
                    progressed = true;
                    if let Some(ack) = pending_flush.take() {
                        let _ = ack.try_send(());
                    }
                }
                Err(error) if is_transient_io(&error) => {
                    flush_pending = true;
                }
                Err(error) if is_closed_error(&error) => {
                    finished = true;
                }
                Err(error) => {
                    log::warn!("ws flush error: {error}");
                    send_ws_event(&recv_tx, WsEvent::Error(format!("ws flush error: {error}")));
                    finished = true;
                }
            }
            if finished {
                break;
            }
        }

        if close_requested.load(Ordering::Acquire) {
            let deadline = Instant::now() + WS_CLOSE_FLUSH_TIMEOUT;
            while Instant::now() < deadline {
                if pending_flush.is_none() {
                    drain_ws_commands(&send_rx, &mut pending_out, &mut pending_flush, &send_space);
                }
                if pending_out.is_empty() && pending_flush.is_none() {
                    break;
                }
                while let Some(payload) = pending_out.pop_front() {
                    match socket.write(Message::Binary(payload.into())) {
                        Ok(()) => {}
                        Err(error) if is_transient_io(&error) => {}
                        Err(_) => break,
                    }
                }
                if let Err(error) = socket.flush()
                    && !is_transient_io(&error)
                {
                    break;
                }
                if let Some(ack) = pending_flush.take() {
                    let _ = ack.try_send(());
                }
            }
            if let Some(ack) = pending_flush.take() {
                let _ = ack.try_send(());
            }
            match socket.close(None) {
                Ok(()) => {}
                Err(error) if is_transient_io(&error) || is_closed_error(&error) => {}
                Err(error) => log::debug!("ws close frame failed: {error}"),
            }
            flush_ws_until(&mut socket, &stop, Instant::now() + WS_CLOSE_FLUSH_TIMEOUT);
            break;
        }

        // Keep memory bounded by WS_WRITE_BATCH records while reducing a long
        // sequence of Noise records to one flush.
        if pending_flush.is_none() && !flush_pending && pending_out.is_empty() {
            drain_ws_commands(&send_rx, &mut pending_out, &mut pending_flush, &send_space);
        }

        if !flush_pending && !pending_out.is_empty() {
            while let Some(payload) = pending_out.pop_front() {
                match socket.write(Message::Binary(payload.into())) {
                    Ok(()) => progressed = true,
                    Err(error) if is_transient_io(&error) => {
                        // Tungstenite keeps the failed frame in its own
                        // bounded out-buffer. Re-submitting the payload would
                        // duplicate it; the remaining batch stays queued.
                        flush_pending = true;
                        progressed = true;
                        break;
                    }
                    Err(error) => {
                        log::warn!("ws send error: {error}");
                        send_ws_event(&recv_tx, WsEvent::Error(format!("ws send error: {error}")));
                        finished = true;
                        break;
                    }
                }
            }
            if finished {
                break;
            }
            if !flush_pending {
                match socket.flush() {
                    Ok(()) => {
                        last_activity = Instant::now();
                        if pending_out.is_empty()
                            && let Some(ack) = pending_flush.take()
                        {
                            let _ = ack.try_send(());
                        }
                    }
                    Err(error) if is_transient_io(&error) => flush_pending = true,
                    Err(error) if is_closed_error(&error) => finished = true,
                    Err(error) => {
                        log::warn!("ws flush error: {error}");
                        send_ws_event(&recv_tx, WsEvent::Error(format!("ws flush error: {error}")));
                        finished = true;
                    }
                }
            }
        }

        // Full-duplex peers can stop reading until their own send buffer has
        // drained. If this actor refuses to read while its write side is
        // blocked, both endpoints can deadlock with full TCP buffers. Keep a
        // bounded read opportunity even while writes or a flush remain pending.
        if let Some(payload) = pending_in.take() {
            match recv_tx.try_send(WsEvent::Binary(payload)) {
                Ok(()) => progressed = true,
                Err(mpsc::TrySendError::Full(WsEvent::Binary(payload))) => {
                    pending_in = Some(payload);
                }
                Err(mpsc::TrySendError::Full(_)) => {}
                Err(mpsc::TrySendError::Disconnected(_)) => break,
            }
        }

        if pending_in.is_none() {
            if !native_polling {
                let desired_timeout = ws_read_timeout(last_activity.elapsed());
                if desired_timeout != read_timeout {
                    if let Err(error) = socket.get_ref().set_ws_read_timeout(Some(desired_timeout))
                    {
                        log::debug!("ws read timeout update failed: {error}");
                    }
                    read_timeout = desired_timeout;
                }
            }
            match socket.read() {
                Ok(Message::Binary(payload)) => {
                    last_activity = Instant::now();
                    progressed = true;
                    pending_in = Some(payload.to_vec());
                }
                Ok(Message::Close(_)) => {
                    send_ws_event(&recv_tx, WsEvent::Closed);
                    closed_sent = true;
                    // Tungstenite has already queued the RFC 6455 Close
                    // reply when it returned this message.
                    flush_ws_until(&mut socket, &stop, Instant::now() + WS_CLOSE_FLUSH_TIMEOUT);
                    finished = true;
                }
                Ok(Message::Ping(_)) => flush_pending = true,
                Ok(Message::Pong(_)) | Ok(Message::Text(_)) => {
                    last_activity = Instant::now();
                }
                Ok(Message::Frame(_)) => {
                    last_activity = Instant::now();
                }
                Err(error) if is_transient_io(&error) => {}
                Err(error) if is_closed_error(&error) => {
                    log::info!("ws actor observed a closed transport while receiving");
                    send_ws_event(&recv_tx, WsEvent::Closed);
                    closed_sent = true;
                    finished = true;
                }
                Err(error) => {
                    log::warn!("ws receive error: {error}");
                    send_ws_event(
                        &recv_tx,
                        WsEvent::Error(format!("ws receive error: {error}")),
                    );
                    finished = true;
                }
            }
        }

        if let Some(payload) = pending_in.take() {
            match recv_tx.try_send(WsEvent::Binary(payload)) {
                Ok(()) => progressed = true,
                Err(mpsc::TrySendError::Full(WsEvent::Binary(payload))) => {
                    pending_in = Some(payload)
                }
                Err(mpsc::TrySendError::Full(_)) => {}
                Err(mpsc::TrySendError::Disconnected(_)) => finished = true,
            }
        }

        if native_polling && !progressed {
            if let Err(error) = socket.get_ref().wait_ws_activity(
                &wake,
                pending_in.is_none(),
                flush_pending || !pending_out.is_empty(),
                WS_IDLE_READ_TIMEOUT,
            ) {
                send_ws_event(&recv_tx, WsEvent::Error(format!("ws wait error: {error}")));
                break;
            }
        } else if !native_polling && (pending_in.is_some() || !pending_out.is_empty()) {
            thread::sleep(Duration::from_millis(1));
        }
    }

    log::info!(
        "ws actor stopped closed_sent={closed_sent} stop={}",
        stop.load(Ordering::Acquire)
    );
    if !closed_sent {
        send_ws_event(&recv_tx, WsEvent::Closed);
    }
    if let Some(ack) = close_ack
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
    {
        let _ = ack.try_send(());
    }
    if let Some(ack) = pending_flush.take() {
        let _ = ack.try_send(());
    }
    stop.store(true, Ordering::Release);
}

fn drain_ws_commands(
    send_rx: &mpsc::Receiver<WsCommand>,
    pending_out: &mut VecDeque<Vec<u8>>,
    pending_flush: &mut Option<mpsc::SyncSender<()>>,
    send_space: &(Mutex<()>, Condvar),
) {
    let _space = send_space
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    queue_ws_commands(send_rx, pending_out, pending_flush);
    send_space.1.notify_all();
}

/// Move at most one bounded write batch into the actor-local queue.
///
/// A flush command acts as a barrier: commands received after it are left in
/// the channel until the preceding batch has been flushed and acknowledged.
fn queue_ws_commands(
    send_rx: &mpsc::Receiver<WsCommand>,
    pending_out: &mut VecDeque<Vec<u8>>,
    pending_flush: &mut Option<mpsc::SyncSender<()>>,
) {
    while pending_out.len() < WS_WRITE_BATCH && pending_flush.is_none() {
        match send_rx.try_recv() {
            Ok(WsCommand::Binary(payload)) => pending_out.push_back(payload),
            Ok(WsCommand::Flush(ack)) => *pending_flush = Some(ack),
            Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
        }
    }
}

fn ws_read_timeout(idle_for: Duration) -> Duration {
    if idle_for < WS_ACTIVE_WINDOW {
        WS_ACTIVE_READ_TIMEOUT
    } else {
        WS_IDLE_READ_TIMEOUT
    }
}

fn send_ws_event(recv_tx: &mpsc::SyncSender<WsEvent>, event: WsEvent) {
    let _ = recv_tx.try_send(event);
}

fn flush_ws_until<S>(socket: &mut WebSocket<S>, stop: &AtomicBool, deadline: Instant)
where
    S: Read + Write,
{
    while Instant::now() < deadline && !stop.load(Ordering::Acquire) {
        match socket.flush() {
            Ok(()) => return,
            Err(error) if is_transient_io(&error) => {
                thread::sleep(Duration::from_millis(1));
            }
            Err(_) => return,
        }
    }
}

fn accept_ws_polling(
    stream: std::net::TcpStream,
    listener_closed: &AtomicBool,
) -> Result<WebSocket<std::net::TcpStream>> {
    stream.set_nonblocking(true).map_err(Error::Io)?;
    stream
        .set_read_timeout(Some(WS_HANDSHAKE_POLL))
        .map_err(Error::Io)?;
    stream
        .set_write_timeout(Some(WS_HANDSHAKE_POLL))
        .map_err(Error::Io)?;
    let deadline = Instant::now() + WS_HANDSHAKE_TIMEOUT;
    let mut handshake = match tungstenite::accept_with_config(stream, Some(ws_config())) {
        Ok(websocket) => {
            websocket.get_ref().set_nonblocking(false)?;
            return Ok(websocket);
        }
        Err(HandshakeError::Interrupted(handshake)) => handshake,
        Err(HandshakeError::Failure(error)) => {
            return Err(Error::Other(format!("ws handshake error: {error}")));
        }
    };

    loop {
        if listener_closed.load(Ordering::Acquire) {
            return Err(Error::Other(
                "ws listener closed during handshake".to_string(),
            ));
        }
        if Instant::now() >= deadline {
            return Err(Error::Timeout);
        }
        thread::sleep(Duration::from_millis(5));
        handshake = match handshake.handshake() {
            Ok(websocket) => {
                websocket.get_ref().set_nonblocking(false)?;
                return Ok(websocket);
            }
            Err(HandshakeError::Interrupted(handshake)) => handshake,
            Err(HandshakeError::Failure(error)) => {
                return Err(Error::Other(format!("ws handshake error: {error}")));
            }
        };
    }
}

/// Accepting side of a WebSocket listener.
pub struct WSServer {
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    connections: Arc<Mutex<HashMap<SocketAddr, WSConnection>>>,
    workers: Arc<Mutex<Vec<JoinHandle<()>>>>,
    #[cfg(test)]
    active_handshakes: Arc<std::sync::atomic::AtomicUsize>,
    listener: Mutex<Option<JoinHandle<()>>>,
}

impl WSServer {
    pub fn new(address: &str, on_message: MessageHandler) -> Result<Self> {
        Self::new_with_disconnect(address, on_message, Arc::new(|_, _| {}))
    }

    pub fn new_with_disconnect(
        address: &str,
        on_message: MessageHandler,
        on_disconnect: DisconnectHandler,
    ) -> Result<Self> {
        let server = TcpListener::bind(address)?;
        server.set_nonblocking(true)?;
        let local_addr = server.local_addr()?;
        let connections: Arc<Mutex<HashMap<SocketAddr, WSConnection>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let workers: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::new(Mutex::new(Vec::new()));
        #[cfg(test)]
        let active_handshakes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let closed = Arc::new(AtomicBool::new(false));
        let listener_connections = connections.clone();
        let listener_workers = workers.clone();
        #[cfg(test)]
        let listener_active_handshakes = active_handshakes.clone();
        let listener_closed = closed.clone();

        let listener = thread::Builder::new()
            .name(format!("ws listener : {local_addr}"))
            .spawn(move || {
                loop {
                    match server.accept() {
                        Ok((stream, _)) => {
                            if listener_closed.load(Ordering::Acquire) {
                                let _ = stream.shutdown(Shutdown::Both);
                                break;
                            }
                            let _ = stream.set_nonblocking(false);
                            let _ = stream.set_nodelay(true);
                            let remote_addr = match stream.peer_addr() {
                                Ok(address) => address,
                                Err(error) => {
                                    log::warn!("ws peer address failed: {error}");
                                    continue;
                                }
                            };
                            #[cfg(test)]
                            listener_active_handshakes.fetch_add(1, Ordering::AcqRel);
                            let websocket = match accept_ws_polling(stream, &listener_closed) {
                                Ok(websocket) => websocket,
                                Err(error) => {
                                    #[cfg(test)]
                                    listener_active_handshakes.fetch_sub(1, Ordering::AcqRel);
                                    log::warn!("ws handshake failed: {error}");
                                    continue;
                                }
                            };
                            #[cfg(test)]
                            listener_active_handshakes.fetch_sub(1, Ordering::AcqRel);
                            let _ = websocket
                                .get_ref()
                                .set_read_timeout(Some(WS_ACTIVE_READ_TIMEOUT));
                            let _ = websocket
                                .get_ref()
                                .set_write_timeout(Some(WS_WRITE_TIMEOUT));
                            if listener_closed.load(Ordering::Acquire) {
                                let _ = websocket.get_ref().shutdown(Shutdown::Both);
                                break;
                            }
                            let close_stream = match websocket.get_ref().try_clone() {
                                Ok(stream) => stream,
                                Err(error) => {
                                    log::warn!("ws close handle clone failed: {error}");
                                    continue;
                                }
                            };
                            let close = Arc::new(move |how| {
                                let _ = close_stream.shutdown(how);
                            });
                            let session = match WsSession::start(websocket, close) {
                                Ok(session) => session,
                                Err(error) => {
                                    log::warn!("ws actor start failed: {error}");
                                    continue;
                                }
                            };
                            let mut connection = WSConnection {
                                session: Some(session),
                                close_reader: None,
                                local_addr: remote_addr,
                                proxy_context: None,
                                route_report: RouteReport::default(),
                                frozen_route: None,
                                closed: Arc::new(AtomicBool::new(false)),
                                noise: None,
                            };
                            let connections = listener_connections.clone();
                            let on_message = on_message.clone();
                            let on_disconnect = on_disconnect.clone();
                            match thread::Builder::new()
                                .name(format!("ws connection : {remote_addr}"))
                                .spawn(move || {
                                    let tunnel_port = match accept_remote_connection(
                                        &mut connection,
                                        remote_addr,
                                        &connections,
                                    ) {
                                        Ok(tunnel_port) => tunnel_port,
                                        Err(error) => {
                                            log::warn!(
                                                "ws encrypted handshake failed for {}: {}",
                                                remote_addr,
                                                error
                                            );
                                            connection.close();
                                            return;
                                        }
                                    };
                                    let end = match tunnel_port {
                                        Some(port) => {
                                            serve_tunnel(&mut connection, remote_addr, port);
                                            ConnectionEnd::Tunnel
                                        }
                                        None => handle_connection(
                                            &mut connection,
                                            remote_addr,
                                            &on_message,
                                        ),
                                    };
                                    match end {
                                        ConnectionEnd::Closed => {
                                            log::info!("ws client worker finished");
                                            on_disconnect(remote_addr, HeroinnProtocol::HTTP);
                                        }
                                        ConnectionEnd::Tunnel => {
                                            log::info!("ws client worker entered tunnel");
                                        }
                                    }
                                    connections
                                        .lock()
                                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                                        .remove(&remote_addr);
                                    connection.close();
                                }) {
                                Ok(worker) => listener_workers
                                    .lock()
                                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                                    .push(worker),
                                Err(error) => {
                                    log::warn!("ws connection worker start failed: {error}");
                                    listener_connections
                                        .lock()
                                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                                        .remove(&remote_addr);
                                }
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if listener_closed.load(Ordering::Acquire) {
                                break;
                            }
                            if let Err(error) =
                                wait_tcp_listener(&server, Duration::from_millis(200))
                            {
                                log::warn!("ws listener readiness failed: {error}");
                                break;
                            }
                        }
                        Err(error) => log::warn!("ws accept failed: {error}"),
                    }
                }

                let sessions = listener_connections
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .drain()
                    .map(|(_, session)| session)
                    .collect::<Vec<_>>();
                for mut connection in sessions {
                    connection.close();
                }
                log::info!("ws main worker finished");
            })
            .map_err(|error| Error::Other(format!("cannot start WS listener: {error}")))?;

        Ok(Self {
            local_addr,
            closed,
            connections,
            workers,
            #[cfg(test)]
            active_handshakes,
            listener: Mutex::new(Some(listener)),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn sendto(&mut self, peer_addr: &SocketAddr, buf: &[u8]) -> Result<()> {
        let mut connection = self
            .connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(peer_addr)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("ws client {peer_addr}")))?;
        connection.send(buf)
    }

    pub fn contains_addr(&mut self, peer_addr: &SocketAddr) -> bool {
        self.connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(peer_addr)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Release);
        let connections = self
            .connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .drain()
            .map(|(_, session)| session)
            .collect::<Vec<_>>();
        for mut connection in connections {
            connection.close();
        }
        if let Some(handle) = self
            .listener
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            let _ = handle.join();
        }
        let workers = std::mem::take(
            &mut *self
                .workers
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        for worker in workers {
            let _ = worker.join();
        }
    }
}

fn accept_remote_connection(
    connection: &mut WSConnection,
    remote_addr: SocketAddr,
    connections: &Arc<Mutex<HashMap<SocketAddr, WSConnection>>>,
) -> Result<Option<u16>> {
    let first = connection.recv_raw_timeout(NOISE_HANDSHAKE_TIMEOUT)?;
    let (intro, kind, tunnel_port) = if first.len() == 6 && first[..4] == TUNNEL_FLAG {
        let port = u16::from_be_bytes([first[4], first[5]]);
        let intro = connection.recv_raw_timeout(NOISE_HANDSHAKE_TIMEOUT)?;
        (intro, NoiseKind::Tunnel { port }, Some(port))
    } else {
        (first, NoiseKind::Main, None)
    };

    let session = connection.session()?.clone();
    let wire: Arc<dyn NoiseWire> = Arc::new(WsNoiseWire { session });
    let noise = handshake_responder(
        wire,
        &intro,
        NoiseTransport::Http,
        kind,
        STREAM_RECORD_PAYLOAD,
        STREAM_RECORD_MAX,
    )?;
    connection.noise = Some(Arc::new(noise));
    connections
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(remote_addr, connection.clone());
    Ok(tunnel_port)
}

fn handle_connection(
    connection: &mut WSConnection,
    remote_addr: SocketAddr,
    on_message: &MessageHandler,
) -> ConnectionEnd {
    loop {
        let buf = match connection.recv() {
            Ok(buf) => buf,
            Err(error) => {
                log::info!("ws connection receive finished: {error}");
                return ConnectionEnd::Closed;
            }
        };

        match ProtocolMessage::new(remote_addr, HeroinnProtocol::HTTP, &buf) {
            Ok(message) => on_message(message),
            Err(error) => log::warn!("failed to decode ws message from {remote_addr}: {error}"),
        }
    }
}

fn serve_tunnel(connection: &mut WSConnection, remote_addr: SocketAddr, port: u16) {
    let full_addr = format!("127.0.0.1:{port}");
    let mut local = match TcpConnection::connect_local_plain(&full_addr) {
        Ok(connection) => connection,
        Err(error) => {
            log::error!("tunnel connect failed: {error}");
            connection.close();
            return;
        }
    };
    log::debug!("ws tunnel started peer={remote_addr} local={full_addr}");

    let mut local_reader = local.clone();
    let mut local_writer = local.clone();
    let mut remote_reader = connection.clone();
    let mut remote_writer = connection.clone();
    let cancel_local = Arc::new(AtomicBool::new(false));
    let first_cancel = cancel_local.clone();
    let second_cancel = cancel_local;
    thread::scope(|scope| {
        let first = thread::Builder::new()
            .name(format!("ws tunnel local->peer : {remote_addr}"))
            .spawn_scoped(scope, move || {
                let mut partial = Vec::new();
                while !first_cancel.load(Ordering::Acquire) {
                    match local_reader.recv_poll(&mut partial, TUNNEL_IDLE_POLL) {
                        Ok(Some(payload)) => {
                            if let Err(error) = remote_writer.send(&payload) {
                                log::debug!("ws tunnel local->peer send ended: {error}");
                                let _ = local_reader.shutdown(Shutdown::Both);
                                break;
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            log::debug!("ws tunnel local->peer read ended: {error}");
                            break;
                        }
                    }
                }
                log::debug!(
                    "ws tunnel local->peer stopped cancelled={}",
                    first_cancel.load(Ordering::Acquire)
                );
                let _ = remote_writer.finish();
                let _ = local_reader.shutdown(Shutdown::Read);
            });
        let second = thread::Builder::new()
            .name(format!("ws tunnel peer->local : {remote_addr}"))
            .spawn_scoped(scope, move || {
                loop {
                    match remote_reader.recv() {
                        Ok(payload) => {
                            if let Err(error) = local_writer.send(&payload) {
                                log::debug!("ws tunnel peer->local send ended: {error}");
                                break;
                            }
                        }
                        Err(error) => {
                            log::debug!("ws tunnel peer->local receive ended: {error}");
                            break;
                        }
                    }
                }
                let remote_finished = remote_reader.remote_closed().unwrap_or(false);
                log::debug!("ws tunnel peer->local stopped remote_finished={remote_finished}");
                if !remote_finished {
                    // An abnormal peer loss must release the opposite pump.
                    // A clean FIN, however, only closes peer->local: the
                    // local side may still have a final response in flight.
                    second_cancel.store(true, Ordering::Release);
                }
                let how = if remote_finished {
                    Shutdown::Write
                } else {
                    Shutdown::Both
                };
                let _ = local_writer.shutdown(how);
            });
        match (first, second) {
            (Ok(first), Ok(second)) => {
                let _ = first.join();
                let _ = second.join();
            }
            (Err(error), second) | (second, Err(error)) => {
                log::warn!("ws tunnel worker start failed: {error}");
                let _ = connection.shutdown(Shutdown::Both);
                let _ = local.shutdown(Shutdown::Both);
                if let Ok(handle) = second {
                    let _ = handle.join();
                }
            }
        }
    });
    connection.close();
    local.close();
    log::info!("ws tunnel finished for {remote_addr}");
}

/// One outbound WebSocket connection.
pub struct WSConnection {
    session: Option<Arc<WsSession>>,
    close_reader: Option<OutboundReader>,
    local_addr: SocketAddr,
    proxy_context: Option<ProxyContext>,
    route_report: RouteReport,
    frozen_route: Option<FrozenRoute>,
    closed: Arc<AtomicBool>,
    noise: Option<Arc<NoiseSession>>,
}

impl WSConnection {
    pub fn connect(address: &str) -> Result<Self> {
        Self::connect_direct(address)
    }

    pub fn connect_direct(address: &str) -> Result<Self> {
        Self::connect_with_connector(&OutboundConnector::direct(), address)
    }

    pub fn connect_with_proxy(address: &str) -> Result<Self> {
        Self::connect_with_factory(&TransportFactory::system(), address)
    }

    pub fn connect_with_route(address: &str, route: &ProxyRoute) -> Result<Self> {
        let connector = OutboundConnector::with_static_route(route.clone());
        Self::connect_with_connector(&connector, address)
    }

    pub fn connect_with_factory(factory: &TransportFactory, address: &str) -> Result<Self> {
        let mut connection = Self::connect_raw_with_connector(factory.connector(), address)?;
        connection.start_noise(NoiseKind::Main)?;
        Ok(connection)
    }

    fn connect_with_connector(connector: &OutboundConnector, address: &str) -> Result<Self> {
        let mut connection = Self::connect_raw_with_connector(connector, address)?;
        connection.start_noise(NoiseKind::Main)?;
        Ok(connection)
    }

    fn connect_raw_with_connector(connector: &OutboundConnector, address: &str) -> Result<Self> {
        let destination = Destination::parse(address)?;
        let stream = connector.open(&destination)?;
        let local_addr = stream.local_addr()?;
        let proxy_context = stream.proxy_context().cloned();
        let route_report = stream.route_report().clone();
        let frozen_route = stream.frozen_route().cloned();
        let close_reader = stream.close_reader();
        let websocket = connect_on_with_timeout(address, stream, &close_reader)?;
        Self::from_websocket(
            websocket,
            close_reader,
            local_addr,
            proxy_context,
            route_report,
            frozen_route,
        )
    }

    fn from_websocket(
        websocket: WebSocket<OutboundStream>,
        close_reader: OutboundReader,
        local_addr: SocketAddr,
        proxy_context: Option<ProxyContext>,
        route_report: RouteReport,
        frozen_route: Option<FrozenRoute>,
    ) -> Result<Self> {
        websocket
            .get_ref()
            .set_read_timeout(Some(WS_ACTIVE_READ_TIMEOUT))
            .map_err(|error| Error::Other(format!("ws read timeout setup failed: {error}")))?;
        websocket
            .get_ref()
            .set_write_timeout(Some(WS_WRITE_TIMEOUT))
            .map_err(|error| Error::Other(format!("ws write timeout setup failed: {error}")))?;
        let close = {
            let close_reader = close_reader.clone();
            Arc::new(move |how| {
                let _ = close_reader.shutdown(how);
            })
        };
        let session = WsSession::start(websocket, close)?;
        Ok(Self {
            session: Some(session),
            close_reader: Some(close_reader),
            local_addr,
            proxy_context,
            route_report,
            frozen_route,
            closed: Arc::new(AtomicBool::new(false)),
            noise: None,
        })
    }

    pub fn tunnel(remote_addr: &str, server_local_port: u16) -> Result<Self> {
        Self::tunnel_direct(remote_addr, server_local_port)
    }

    pub fn tunnel_direct(remote_addr: &str, server_local_port: u16) -> Result<Self> {
        Self::tunnel_with_connector(&OutboundConnector::direct(), remote_addr, server_local_port)
    }

    pub fn tunnel_with_proxy(remote_addr: &str, server_local_port: u16) -> Result<Self> {
        Self::tunnel_with_factory(&TransportFactory::system(), remote_addr, server_local_port)
    }

    pub fn tunnel_with_route(
        remote_addr: &str,
        server_local_port: u16,
        route: &ProxyRoute,
    ) -> Result<Self> {
        let connector = OutboundConnector::with_static_route(route.clone());
        Self::tunnel_with_connector(&connector, remote_addr, server_local_port)
    }

    pub fn tunnel_with_factory(
        factory: &TransportFactory,
        remote_addr: &str,
        server_local_port: u16,
    ) -> Result<Self> {
        Self::tunnel_with_connector(factory.connector(), remote_addr, server_local_port)
    }

    pub fn tunnel_with_frozen_route(
        factory: &TransportFactory,
        remote_addr: &str,
        server_local_port: u16,
        frozen: &FrozenRoute,
    ) -> Result<Self> {
        let destination = Destination::parse(remote_addr)?;
        let stream = factory.connector().open_frozen(&destination, frozen)?;
        let local_addr = stream.local_addr()?;
        let proxy_context = stream.proxy_context().cloned();
        let route_report = stream.route_report().clone();
        let frozen_route = stream.frozen_route().cloned();
        let close_reader = stream.close_reader();
        let websocket = connect_on_with_timeout(remote_addr, stream, &close_reader)?;
        let mut connection = Self::from_websocket(
            websocket,
            close_reader,
            local_addr,
            proxy_context,
            route_report,
            frozen_route,
        )?;
        let mut handshake = TUNNEL_FLAG.to_vec();
        handshake.extend_from_slice(&server_local_port.to_be_bytes());
        connection.send_raw(&handshake)?;
        connection.start_noise(NoiseKind::Tunnel {
            port: server_local_port,
        })?;
        Ok(connection)
    }

    fn tunnel_with_connector(
        connector: &OutboundConnector,
        remote_addr: &str,
        server_local_port: u16,
    ) -> Result<Self> {
        let mut connection = Self::connect_raw_with_connector(connector, remote_addr)?;
        let mut handshake = TUNNEL_FLAG.to_vec();
        handshake.extend_from_slice(&server_local_port.to_be_bytes());
        connection.send_raw(&handshake)?;
        connection.start_noise(NoiseKind::Tunnel {
            port: server_local_port,
        })?;
        Ok(connection)
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        if let Some(noise) = &self.noise {
            let result = noise.recv();
            if result.is_err() {
                self.closed.store(true, Ordering::Release);
            }
            return result;
        }
        let session = self.session()?;
        let result = session.recv();
        if result.is_err() {
            self.closed.store(true, Ordering::Release);
        }
        result
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<()> {
        if let Some(noise) = &self.noise {
            let result = noise.send(buf);
            if result.is_err() {
                self.closed.store(true, Ordering::Release);
            }
            return result;
        }
        let session = self.session()?;
        let result = session.send(buf);
        if result.is_err() {
            self.closed.store(true, Ordering::Release);
        }
        result
    }

    pub fn finish(&self) -> Result<()> {
        if let Some(noise) = &self.noise {
            noise.finish()
        } else {
            self.session()?.flush()
        }
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn proxy_context(&self) -> Option<&ProxyContext> {
        self.proxy_context.as_ref()
    }

    pub fn route_report(&self) -> &RouteReport {
        &self.route_report
    }

    pub fn frozen_route(&self) -> Option<&FrozenRoute> {
        self.frozen_route.as_ref()
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        if matches!(how, Shutdown::Write | Shutdown::Both)
            && let Some(noise) = &self.noise
        {
            let _ = noise.finish();
        }
        if let Some(close_reader) = &self.close_reader {
            close_reader.shutdown(how).map_err(Error::Io)
        } else if let Some(session) = &self.session {
            session.shutdown();
            Ok(())
        } else {
            Err(Error::Other("socket closed".to_string()))
        }
    }

    pub fn remote_closed(&self) -> Result<bool> {
        if self
            .noise
            .as_ref()
            .is_some_and(|session| session.remote_finished())
        {
            return Ok(true);
        }
        match &self.close_reader {
            Some(close_reader) => close_reader.remote_closed().map_err(Error::Io),
            None => Ok(false),
        }
    }

    pub fn wait_for_remote_close(&self, timeout: Duration) -> Result<()> {
        match &self.close_reader {
            Some(close_reader) => close_reader
                .wait_for_remote_close(timeout)
                .map_err(Error::Io),
            None => Ok(()),
        }
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Release);
        if let Some(noise) = &self.noise {
            noise.close();
        }
        if let Some(session) = self.session.take() {
            session.close_and_join();
        }
    }

    fn session(&self) -> Result<&Arc<WsSession>> {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::Other("socket closed".to_string()));
        }
        self.session
            .as_ref()
            .ok_or_else(|| Error::Other("socket closed".to_string()))
    }

    fn send_raw(&self, payload: &[u8]) -> Result<()> {
        self.session()?.send(payload)
    }

    fn recv_raw_timeout(&self, timeout: Duration) -> Result<Vec<u8>> {
        self.session()?.recv_timeout(timeout)
    }

    fn start_noise(&mut self, kind: NoiseKind) -> Result<()> {
        let session = self.session()?.clone();
        let wire: Arc<dyn NoiseWire> = Arc::new(WsNoiseWire { session });
        let intro = NoiseIntro::new(NoiseTransport::Http, kind);
        let noise = handshake_initiator(wire, intro, STREAM_RECORD_PAYLOAD, STREAM_RECORD_MAX)?;
        self.noise = Some(Arc::new(noise));
        Ok(())
    }
}

impl Clone for WSConnection {
    fn clone(&self) -> Self {
        Self {
            session: self.session.clone(),
            close_reader: self.close_reader.clone(),
            local_addr: self.local_addr,
            proxy_context: self.proxy_context.clone(),
            route_report: self.route_report.clone(),
            frozen_route: self.frozen_route.clone(),
            closed: self.closed.clone(),
            noise: self.noise.clone(),
        }
    }
}

impl Drop for WSServer {
    fn drop(&mut self) {
        self.close();
    }
}

fn ws_request(address: &str) -> Result<tungstenite::http::Request<()>> {
    let uri: tungstenite::http::Uri = format!("ws://{address}/")
        .parse()
        .map_err(|error| Error::Other(format!("ws uri error: {error}")))?;
    let authority = uri
        .authority()
        .ok_or_else(|| Error::Other("ws uri has no authority".to_string()))?
        .as_str()
        .to_string();
    tungstenite::http::Request::builder()
        .method("GET")
        .uri(uri)
        .header("Host", authority)
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tungstenite::handshake::client::generate_key(),
        )
        .body(())
        .map_err(|error| Error::Other(format!("ws request error: {error}")))
}

fn connect_on_with_timeout(
    address: &str,
    stream: OutboundStream,
    close_reader: &OutboundReader,
) -> Result<WebSocket<OutboundStream>> {
    connect_on_with_timeout_with_limit(address, stream, close_reader, WS_HANDSHAKE_TIMEOUT)
}

fn connect_on_with_timeout_with_limit(
    address: &str,
    stream: OutboundStream,
    close_reader: &OutboundReader,
    timeout: Duration,
) -> Result<WebSocket<OutboundStream>> {
    let request = ws_request(address)?;
    let poll = WS_HANDSHAKE_POLL.min(timeout.max(Duration::from_millis(1)));
    stream
        .set_nonblocking(true)
        .map_err(|error| Error::Other(format!("ws handshake mode setup failed: {error}")))?;
    stream
        .set_read_timeout(Some(poll))
        .map_err(|error| Error::Other(format!("ws handshake timeout setup failed: {error}")))?;
    stream
        .set_write_timeout(Some(poll))
        .map_err(|error| Error::Other(format!("ws handshake timeout setup failed: {error}")))?;
    let deadline = Instant::now() + timeout;
    let mut handshake = match client_with_config(request, stream, Some(ws_config())) {
        Ok((websocket, _response)) => {
            websocket
                .get_ref()
                .set_nonblocking(false)
                .map_err(Error::Io)?;
            return Ok(websocket);
        }
        Err(HandshakeError::Interrupted(handshake)) => handshake,
        Err(HandshakeError::Failure(error)) => {
            return Err(Error::Other(format!("ws handshake error: {error}")));
        }
    };

    loop {
        if Instant::now() >= deadline {
            let _ = close_reader.shutdown(Shutdown::Both);
            return Err(Error::Timeout);
        }
        thread::sleep(Duration::from_millis(5));
        handshake = match handshake.handshake() {
            Ok((websocket, _response)) => {
                websocket
                    .get_ref()
                    .set_nonblocking(false)
                    .map_err(Error::Io)?;
                return Ok(websocket);
            }
            Err(HandshakeError::Interrupted(handshake)) => handshake,
            Err(HandshakeError::Failure(error)) => {
                return Err(Error::Other(format!("ws handshake error: {error}")));
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::Message as ProtocolMessage;
    use crate::protocol::tcp::TcpConnection;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    fn read_until_headers_end(stream: &mut TcpStream) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut byte = [0u8; 1];
        while !bytes.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            bytes.push(byte[0]);
        }
        bytes
    }

    fn write_frame(
        stream: &mut TcpStream,
        opcode: u8,
        final_frame: bool,
        masked: bool,
        payload: &[u8],
    ) {
        let first = if final_frame { 0x80 | opcode } else { opcode };
        let len = payload.len();
        let mut frame = vec![first];
        if len <= 125 {
            frame.push((if masked { 0x80 } else { 0 }) | len as u8);
        } else if len <= u16::MAX as usize {
            frame.push((if masked { 0x80 } else { 0 }) | 126);
            frame.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            frame.push((if masked { 0x80 } else { 0 }) | 127);
            frame.extend_from_slice(&(len as u64).to_be_bytes());
        }
        if masked {
            let mask = [0x12, 0x34, 0x56, 0x78];
            frame.extend_from_slice(&mask);
            for (index, byte) in payload.iter().enumerate() {
                frame.push(byte ^ mask[index % 4]);
            }
        } else {
            frame.extend_from_slice(payload);
        }
        stream.write_all(&frame).unwrap();
    }

    fn read_frame(stream: &mut TcpStream, masked: bool) -> (u8, Vec<u8>) {
        let mut header = [0u8; 2];
        stream.read_exact(&mut header).unwrap();
        let opcode = header[0] & 0x0f;
        let mut len = (header[1] & 0x7f) as usize;
        if len == 126 {
            let mut value = [0u8; 2];
            stream.read_exact(&mut value).unwrap();
            len = u16::from_be_bytes(value) as usize;
        } else if len == 127 {
            let mut value = [0u8; 8];
            stream.read_exact(&mut value).unwrap();
            len = u64::from_be_bytes(value) as usize;
        }
        let mask = if masked {
            let mut mask = [0u8; 4];
            stream.read_exact(&mut mask).unwrap();
            Some(mask)
        } else {
            None
        };
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload).unwrap();
        if let Some(mask) = mask {
            for (index, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[index % 4];
            }
        }
        (opcode, payload)
    }

    fn raw_handshake(stream: &mut TcpStream, address: &str) {
        let request = format!(
            "GET / HTTP/1.1\r\nHost: {address}\r\nConnection: Upgrade\r\n\
             Upgrade: websocket\r\nSec-WebSocket-Version: 13\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).unwrap();
        let response = String::from_utf8_lossy(&read_until_headers_end(stream)).to_string();
        assert!(response.starts_with("HTTP/1.1 101"), "{response}");
    }

    struct RawFramedWire {
        stream: Mutex<TcpStream>,
        outbound_masked: bool,
        fragment_next: AtomicBool,
    }

    impl RawFramedWire {
        fn new(stream: TcpStream, outbound_masked: bool) -> Arc<Self> {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
            Arc::new(Self {
                stream: Mutex::new(stream),
                outbound_masked,
                fragment_next: AtomicBool::new(false),
            })
        }

        fn send_control(&self, opcode: u8, payload: &[u8]) {
            let mut stream = self
                .stream
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            write_frame(&mut stream, opcode, true, self.outbound_masked, payload);
            stream.flush().unwrap();
        }

        fn read_control(&self, expected: u8) -> Vec<u8> {
            let mut stream = self
                .stream
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            loop {
                let (opcode, payload) = read_frame(&mut stream, !self.outbound_masked);
                if opcode == expected {
                    return payload;
                }
                assert_ne!(opcode, 0x8, "unexpected WebSocket close");
            }
        }

        fn fragment_next_message(&self) {
            self.fragment_next.store(true, Ordering::Release);
        }
    }

    impl NoiseWire for RawFramedWire {
        fn send_plain(&self, payload: &[u8]) -> Result<()> {
            let mut stream = self
                .stream
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if self.fragment_next.swap(false, Ordering::AcqRel) && payload.len() > 1 {
                let split = payload.len() / 2;
                write_frame(
                    &mut stream,
                    0x2,
                    false,
                    self.outbound_masked,
                    &payload[..split],
                );
                write_frame(
                    &mut stream,
                    0x0,
                    true,
                    self.outbound_masked,
                    &payload[split..],
                );
            } else {
                write_frame(&mut stream, 0x2, true, self.outbound_masked, payload);
            }
            stream.flush().map_err(Error::Io)
        }

        fn recv_plain(&self) -> Result<Vec<u8>> {
            self.recv_plain_timeout(Duration::from_secs(10))
        }

        fn recv_plain_timeout(&self, timeout: Duration) -> Result<Vec<u8>> {
            let mut stream = self
                .stream
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            stream.set_read_timeout(Some(timeout)).map_err(Error::Io)?;
            loop {
                let (opcode, payload) = read_frame(&mut stream, !self.outbound_masked);
                match opcode {
                    0x2 => return Ok(payload),
                    0x1 | 0x0 => {}
                    0x8 => {
                        return Err(Error::Io(std::io::Error::from(
                            std::io::ErrorKind::UnexpectedEof,
                        )));
                    }
                    0x9 => {
                        write_frame(&mut stream, 0xA, true, self.outbound_masked, &payload);
                        stream.flush().map_err(Error::Io)?;
                    }
                    0xA => {}
                    _ => return Err(Error::Noise("unsupported raw websocket frame")),
                }
            }
        }

        fn flush_plain(&self) -> Result<()> {
            self.stream
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .flush()
                .map_err(Error::Io)
        }
    }

    fn raw_noise_client(address: &str) -> (Arc<RawFramedWire>, NoiseSession) {
        let mut stream = TcpStream::connect(address).unwrap();
        raw_handshake(&mut stream, address);
        let wire = RawFramedWire::new(stream, true);
        let noise = handshake_initiator(
            wire.clone(),
            NoiseIntro::new(NoiseTransport::Http, NoiseKind::Main),
            STREAM_RECORD_PAYLOAD,
            STREAM_RECORD_MAX,
        )
        .unwrap();
        (wire, noise)
    }

    fn accept_raw_noise_server(
        listener: TcpListener,
    ) -> (Arc<RawFramedWire>, NoiseSession, SocketAddr) {
        let (mut stream, address) = listener.accept().unwrap();
        let request = String::from_utf8_lossy(&read_until_headers_end(&mut stream)).to_string();
        let key = request
            .lines()
            .find_map(|line| line.strip_prefix("Sec-WebSocket-Key: "))
            .unwrap()
            .trim()
            .to_string();
        let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
        write!(
            stream,
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        )
        .unwrap();
        stream.flush().unwrap();
        let wire = RawFramedWire::new(stream, false);
        let (_, intro) = {
            let mut stream = wire
                .stream
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            read_frame(&mut stream, true)
        };
        let noise = handshake_responder(
            wire.clone(),
            &intro,
            NoiseTransport::Http,
            NoiseKind::Main,
            STREAM_RECORD_PAYLOAD,
            STREAM_RECORD_MAX,
        )
        .unwrap();
        (wire, noise, address)
    }

    #[test]
    fn binary_round_trip_and_server_send_preserve_application_payload() {
        let (sender, receiver) = mpsc::channel();
        let mut server = WSServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = sender.send(message);
            }),
        )
        .unwrap();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let mut client = WSConnection::connect_direct(&remote).unwrap();
        let payload = ProtocolMessage::build(0x21, "tester", "hello").unwrap();
        client.send(&payload).unwrap();
        let message = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(message.id(), 0x21);
        assert_eq!(message.clientid(), "tester");
        assert_eq!(message.proto(), HeroinnProtocol::HTTP);
        server
            .sendto(
                &message.peer_addr(),
                &ProtocolMessage::build(0x22, "server", "ok").unwrap(),
            )
            .unwrap();
        let reply = client.recv().unwrap();
        assert_eq!(reply[0], 0x22);
        server.close();
    }

    #[test]
    fn plaintext_legacy_client_is_rejected_without_delivery() {
        let (sender, receiver) = mpsc::channel();
        let mut server = WSServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = sender.send(message.id());
            }),
        )
        .unwrap();
        let address = server.local_addr().unwrap().to_string();
        let mut stream = TcpStream::connect(&address).unwrap();
        raw_handshake(&mut stream, &address);
        let wire = RawFramedWire::new(stream, true);
        let plaintext = ProtocolMessage::build(0x32, "legacy", "plaintext").unwrap();
        wire.send_control(0x2, &plaintext);
        assert!(
            receiver.recv_timeout(Duration::from_secs(2)).is_err(),
            "plaintext must never reach the WebSocket application handler"
        );
        server.close();
    }

    #[test]
    fn role_mismatch_is_rejected_before_websocket_application_delivery() {
        let (sender, receiver) = mpsc::channel();
        let mut server = WSServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = sender.send(message.id());
            }),
        )
        .unwrap();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let mut client = WSConnection::connect_direct(&remote).unwrap();
        let mut intro = NoiseIntro::new(NoiseTransport::Http, NoiseKind::Main).encode();
        intro[12..14].copy_from_slice(&2u16.to_be_bytes());
        client.send_raw(&intro).unwrap();

        assert!(
            client.recv_raw_timeout(Duration::from_secs(3)).is_err(),
            "role mismatch did not close the WebSocket before application data"
        );
        assert!(
            receiver.recv_timeout(Duration::from_millis(500)).is_err(),
            "role mismatch reached the WebSocket application handler"
        );
        client.close();
        server.close();
    }

    #[test]
    fn tunnel_preamble_and_frames_survive_both_directions() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let mut client = WSConnection::tunnel(&remote, port).unwrap();
        let (mut local, _) = TcpConnection::tunnel_server(listener, 5).unwrap();

        client.send(b"server-bound").unwrap();
        assert_eq!(local.recv().unwrap(), b"server-bound");
        local.send(b"client-bound").unwrap();
        assert_eq!(client.recv().unwrap(), b"client-bound");
        server.close();
    }

    #[test]
    fn tunnel_flushes_the_final_payload_when_the_local_side_closes() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let mut client = WSConnection::tunnel(&remote, port).unwrap();
        let (mut local, _) = TcpConnection::tunnel_server(listener, 5).unwrap();

        let payload = (0..64 * 1024)
            .map(|index| (index * 31 + 7) as u8)
            .collect::<Vec<_>>();
        local.send(&payload).unwrap();
        local.close();

        let (sender, receiver) = mpsc::channel();
        let reader = thread::spawn(move || {
            let mut received = Vec::new();
            while let Ok(payload) = client.recv() {
                received.extend_from_slice(&payload);
            }
            let _ = sender.send(received);
        });
        let received = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("tunnel payload was not flushed before close");
        assert_eq!(received, payload);
        reader.join().unwrap();
        server.close();
    }

    #[test]
    fn ping_is_answered_and_text_is_not_delivered_as_binary() {
        let (sender, receiver) = mpsc::channel();
        let mut server = WSServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = sender.send(message);
            }),
        )
        .unwrap();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let (wire, client) = raw_noise_client(&remote);
        wire.send_control(0x9, &[7, 8]);
        assert_eq!(wire.read_control(0xA), [7, 8]);
        wire.send_control(0x1, b"not binary");
        assert!(
            receiver.recv_timeout(Duration::from_millis(250)).is_err(),
            "Text must not be treated as an application Binary message"
        );
        client
            .send(&ProtocolMessage::build(1, "c", "binary").unwrap())
            .unwrap();
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(2)).unwrap().id(),
            1
        );
        server.close();
    }

    #[test]
    fn raw_rfc6455_client_fragments_are_aggregated_and_ping_gets_pong() {
        let (sender, receiver) = mpsc::channel();
        let mut server = WSServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = sender.send(message);
            }),
        )
        .unwrap();
        let address = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let (wire, client) = raw_noise_client(&address);
        let payload = ProtocolMessage::build(0x31, "raw", "fragmented").unwrap();
        wire.fragment_next_message();
        client.send(&payload).unwrap();
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(2)).unwrap().id(),
            0x31
        );
        wire.send_control(0x9, b"ping");
        assert_eq!(wire.read_control(0xA), b"ping");
        server.close();
    }

    #[test]
    fn rfc6455_client_handshake_and_server_frame_interoperate() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (_wire, noise, _) = accept_raw_noise_server(listener);
            assert_eq!(
                noise.recv().unwrap(),
                ProtocolMessage::build(0x41, "raw", "in").unwrap()
            );
            noise
                .send(&ProtocolMessage::build(0x42, "raw", "out").unwrap())
                .unwrap();
            noise.finish().unwrap();
        });

        let mut client = WSConnection::connect_direct(&address).unwrap();
        client
            .send(&ProtocolMessage::build(0x41, "raw", "in").unwrap())
            .unwrap();
        assert_eq!(
            client.recv().unwrap(),
            ProtocolMessage::build(0x42, "raw", "out").unwrap()
        );
        server.join().unwrap();
    }

    #[test]
    fn close_is_idempotent_and_send_after_close_is_rejected() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let mut client = WSConnection::connect_direct(&remote).unwrap();
        client.close();
        client.close();
        assert!(client.send(b"after-close").is_err());
        server.close();
    }

    #[test]
    fn server_close_interrupts_an_incomplete_handshake() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let address = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let _pending = TcpStream::connect(&address).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while server.active_handshakes.load(Ordering::Acquire) == 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(server.active_handshakes.load(Ordering::Acquire) > 0);
        let started = Instant::now();
        server.close();
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn configured_limits_preserve_large_messages_and_bound_buffers() {
        let config = ws_config();
        assert_eq!(config.max_message_size, Some(WS_MAX_MESSAGE_SIZE));
        assert_eq!(config.max_frame_size, Some(WS_MAX_FRAME_SIZE));
        assert_eq!(config.max_write_buffer_size, WS_MAX_WRITE_BUFFER);
        const {
            assert!(WS_MAX_MESSAGE_SIZE >= crate::protocol::frame::MAX_PACKET);
            assert!(WS_MAX_FRAME_SIZE >= crate::protocol::frame::MAX_PACKET);
            assert!(WS_MAX_WRITE_BUFFER > WS_MAX_MESSAGE_SIZE);
        }
    }

    #[test]
    fn queued_server_push_wakes_an_actor_waiting_without_peer_data() {
        struct WaitingStream {
            stream: TcpStream,
            entered: mpsc::SyncSender<()>,
        }
        impl Read for WaitingStream {
            fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
                self.stream.read(bytes)
            }
        }
        impl Write for WaitingStream {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.stream.write(bytes)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.stream.flush()
            }
        }
        impl WsReadControl for WaitingStream {
            fn set_ws_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
                self.stream.set_read_timeout(timeout)
            }
            fn enable_ws_nonblocking_poll(&self) -> std::io::Result<bool> {
                self.stream.set_nonblocking(true)?;
                Ok(true)
            }
            fn wait_ws_activity(
                &self,
                wake: &SocketWake,
                readable: bool,
                writable: bool,
                timeout: Duration,
            ) -> std::io::Result<bool> {
                let _ = self.entered.try_send(());
                wait_tcp_activity(&self.stream, wake, readable, writable, timeout)
            }
        }
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let (entered, waiting) = mpsc::sync_channel(1);
        let bootstrap = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accepted");
            let closer = stream.try_clone().expect("shutdown handle");
            let socket = tungstenite::accept(WaitingStream { stream, entered }).expect("upgrade");
            WsSession::start(
                socket,
                Arc::new(move |how| {
                    let _ = closer.shutdown(how);
                }),
            )
            .expect("session")
        });
        let stream = TcpStream::connect(address).expect("client");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read deadline");
        let (mut client, _) = client_with_config(
            ws_request(&address.to_string()).expect("request"),
            stream,
            Some(ws_config()),
        )
        .expect("client upgrade");
        let session = bootstrap.join().expect("bootstrap");
        waiting
            .recv_timeout(Duration::from_secs(2))
            .expect("actor entered readiness wait");
        session
            .send(b"server push without client data")
            .expect("queued send");
        assert_eq!(
            client.read().expect("server push"),
            Message::Binary(b"server push without client data".to_vec().into()),
        );
        let started = Instant::now();
        session.close_and_join();
        assert!(session.actor_finished());
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn tls_ws_polling_uses_a_short_timeout_and_idle_backs_off() {
        assert_eq!(ws_read_timeout(Duration::ZERO), WS_ACTIVE_READ_TIMEOUT);
        assert_eq!(
            ws_read_timeout(WS_ACTIVE_WINDOW - Duration::from_millis(1)),
            WS_ACTIVE_READ_TIMEOUT
        );
        assert_eq!(ws_read_timeout(WS_ACTIVE_WINDOW), WS_IDLE_READ_TIMEOUT);
        assert_eq!(
            ws_read_timeout(WS_ACTIVE_WINDOW + Duration::from_secs(1)),
            WS_IDLE_READ_TIMEOUT
        );
        assert!(WS_ACTIVE_READ_TIMEOUT < WS_IDLE_READ_TIMEOUT);
        assert!(WS_IDLE_READ_TIMEOUT <= Duration::from_millis(25));
    }

    #[test]
    fn ws_write_batches_are_bounded_and_flush_is_a_barrier() {
        let (send, recv) = mpsc::sync_channel(32);
        for value in 0..3 {
            send.try_send(WsCommand::Binary(vec![value])).unwrap();
        }
        let (ack_tx, _ack_rx) = mpsc::sync_channel(1);
        send.try_send(WsCommand::Flush(ack_tx)).unwrap();
        send.try_send(WsCommand::Binary(vec![3])).unwrap();

        let mut pending = VecDeque::new();
        let mut flush = None;
        queue_ws_commands(&recv, &mut pending, &mut flush);
        assert_eq!(pending.len(), 3);
        assert!(flush.is_some());
        assert_eq!(
            recv.try_recv().map(|command| match command {
                WsCommand::Binary(payload) => payload,
                WsCommand::Flush(_) => panic!("flush must remain behind the barrier"),
            }),
            Ok(vec![3]),
            "a command after Flush must not cross the barrier"
        );

        let mut capped = VecDeque::new();
        let mut no_flush = None;
        for value in 0..WS_WRITE_BATCH + 3 {
            send.try_send(WsCommand::Binary(vec![value as u8])).unwrap();
        }
        queue_ws_commands(&recv, &mut capped, &mut no_flush);
        assert_eq!(capped.len(), WS_WRITE_BATCH);
        assert!(no_flush.is_none());
    }

    #[test]
    fn plain_transports_use_native_nonblocking_ws_polling() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let peer = thread::spawn(move || listener.accept().unwrap().0);
        let mut client = TcpStream::connect(address).unwrap();
        let server = peer.join().unwrap();

        assert!(client.enable_ws_nonblocking_poll().unwrap());
        let mut byte = [0u8; 1];
        assert_eq!(
            client.read(&mut byte).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );

        let mut wrapped = OutboundStream::from_tcp(server).unwrap();
        assert!(wrapped.enable_ws_nonblocking_poll().unwrap());
        assert_eq!(
            wrapped.read(&mut byte).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_overlapped_io_pending_is_a_transient_ws_io_error() {
        let pending = tungstenite::Error::Io(std::io::Error::from_raw_os_error(997));
        assert!(is_transient_io(&pending));

        let reset = tungstenite::Error::Io(std::io::Error::from_raw_os_error(10054));
        assert!(!is_transient_io(&reset));
    }

    #[test]
    fn handshake_timeout_closes_the_transport() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let peer = thread::spawn(move || {
            let (_socket, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_secs(2));
        });
        let stream =
            OutboundStream::from_tcp(std::net::TcpStream::connect(&address).unwrap()).unwrap();
        let close_reader = stream.close_reader();
        let started = Instant::now();
        let result = connect_on_with_timeout_with_limit(
            &address,
            stream,
            &close_reader,
            Duration::from_millis(100),
        );
        assert!(matches!(result, Err(Error::Timeout)));
        assert!(started.elapsed() < Duration::from_secs(5));
        peer.join().unwrap();
    }

    #[test]
    fn large_application_payloads_are_fragmented_and_reassembled() {
        let (sender, receiver) = mpsc::channel();
        let mut server = WSServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = sender.send(message.length());
            }),
        )
        .unwrap();
        let address = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let mut client = WSConnection::connect_direct(&address).unwrap();
        let large = ProtocolMessage::build(
            0x61,
            "large",
            "x".repeat(crate::protocol::frame::MAX_PACKET - 256),
        )
        .unwrap();
        client.send(&large).unwrap();
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(30)).unwrap(),
            large.len()
        );
        client.close();
        server.close();
    }

    #[test]
    fn oversized_frame_is_rejected_without_delivering_a_partial_message() {
        let (sender, receiver) = mpsc::channel();
        let mut server = WSServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = sender.send(message);
            }),
        )
        .unwrap();
        let address = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let mut client = TcpStream::connect(&address).unwrap();
        raw_handshake(&mut client, &address);
        client
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();

        let mut header = vec![0x82, 0xFF];
        header.extend_from_slice(&(WS_MAX_FRAME_SIZE as u64 + 1).to_be_bytes());
        header.extend_from_slice(&[1, 2, 3, 4]);
        client.write_all(&header).unwrap();
        let mut byte = [0u8; 1];
        match client.read(&mut byte) {
            Ok(0) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::UnexpectedEof
                ) => {}
            other => panic!("oversized frame did not close the transport: {other:?}"),
        }
        assert!(receiver.recv_timeout(Duration::from_millis(200)).is_err());
        server.close();
    }

    #[test]
    fn close_flushes_the_final_message_before_stopping_the_actor() {
        let (sender, receiver) = mpsc::channel();
        let mut server = WSServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = sender.send(message.id());
            }),
        )
        .unwrap();
        let address = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let mut client = WSConnection::connect_direct(&address).unwrap();
        let session = client.session.as_ref().unwrap().clone();
        client
            .send(&ProtocolMessage::build(0x71, "close", "final-message").unwrap())
            .unwrap();
        let started = Instant::now();
        client.close();
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(session.actor_finished());
        assert_eq!(receiver.recv_timeout(Duration::from_secs(3)).unwrap(), 0x71);
        server.close();
        client.close();
    }

    #[test]
    fn abnormal_eof_and_repeated_close_are_bounded() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let peer = thread::spawn(move || {
            let (wire, noise, _) = accept_raw_noise_server(listener);
            drop(noise);
            drop(wire);
        });

        let mut client = WSConnection::connect_direct(&address).unwrap();
        let started = Instant::now();
        assert!(client.recv().is_err());
        assert!(started.elapsed() < Duration::from_secs(3));
        client.close();
        client.close();
        assert!(client.send(b"after-eof").is_err());
        peer.join().unwrap();
    }

    #[test]
    fn concurrent_senders_and_receiver_preserve_each_message_once() {
        let (sender, receiver) = mpsc::channel();
        let mut server = WSServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = sender.send(message);
            }),
        )
        .unwrap();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let mut client = WSConnection::connect_direct(&remote).unwrap();

        let mut senders = Vec::new();
        for sender_index in 0..4 {
            let mut connection = client.clone();
            senders.push(thread::spawn(move || {
                for item in 0..8 {
                    let id = 0x50 + sender_index * 8 + item;
                    connection
                        .send(
                            &ProtocolMessage::build(id, &format!("sender-{sender_index}"), "x")
                                .unwrap(),
                        )
                        .unwrap();
                }
            }));
        }

        let mut ids = std::collections::BTreeSet::new();
        let mut sender_positions = [None; 4];
        for _ in 0..32 {
            let message = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
            assert!(ids.insert(message.id()));
            let offset = message.id() - 0x50;
            let sender_index = usize::from(offset / 8);
            let item = offset % 8;
            if let Some(previous) = sender_positions[sender_index] {
                assert_eq!(item, previous + 1, "sender messages were reordered");
            }
            sender_positions[sender_index] = Some(item);
            server
                .sendto(
                    &message.peer_addr(),
                    &ProtocolMessage::build(message.id(), "server", "ack").unwrap(),
                )
                .unwrap();
            assert_eq!(client.recv().unwrap()[0], message.id());
        }
        assert_eq!(ids.len(), 32);
        for sender in senders {
            sender.join().unwrap();
        }
        server.close();
    }

    #[test]
    fn slow_reader_applies_backpressure_and_close_cancels_the_sender() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let (_wire, _noise, _) = accept_raw_noise_server(listener);
            ready_tx.send(()).unwrap();
            let _ = release_rx.recv_timeout(Duration::from_secs(5));
        });

        let mut client = WSConnection::connect_direct(&address).unwrap();
        ready_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let progress = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sender_finished = Arc::new(AtomicBool::new(false));
        let sender_progress = progress.clone();
        let sender_done = sender_finished.clone();
        let mut sender = client.clone();
        let sender = thread::spawn(move || {
            let payload = vec![0x5A; 64 * 1024];
            for _ in 0..100 {
                if sender.send(&payload).is_err() {
                    break;
                }
                sender_progress.fetch_add(1, Ordering::Release);
            }
            sender_done.store(true, Ordering::Release);
        });

        let wait_deadline = Instant::now() + Duration::from_secs(5);
        while progress.load(Ordering::Acquire) < WS_SEND_QUEUE
            && !sender_finished.load(Ordering::Acquire)
            && Instant::now() < wait_deadline
        {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            progress.load(Ordering::Acquire) < 100,
            "the sender completed without applying backpressure"
        );

        let started = Instant::now();
        client.close();
        assert!(started.elapsed() < Duration::from_secs(4));
        sender.join().unwrap();
        assert!(client.send(b"after-close").is_err());
        let _ = release_tx.send(());
        server.join().unwrap();
    }

    #[test]
    fn write_buffer_full_is_a_bounded_error() {
        struct Blocked;

        impl Read for Blocked {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(std::io::ErrorKind::WouldBlock))
            }
        }

        impl Write for Blocked {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(std::io::ErrorKind::WouldBlock))
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::from(std::io::ErrorKind::WouldBlock))
            }
        }

        let config = WebSocketConfig::default()
            .write_buffer_size(0)
            .max_write_buffer_size(1024)
            .max_message_size(Some(WS_MAX_MESSAGE_SIZE))
            .max_frame_size(Some(WS_MAX_FRAME_SIZE));
        let mut websocket =
            WebSocket::from_raw_socket(Blocked, tungstenite::protocol::Role::Client, Some(config));
        assert!(matches!(
            websocket.write(Message::Binary(vec![0u8; 2048].into())),
            Err(tungstenite::Error::WriteBufferFull(_))
        ));
    }
}
