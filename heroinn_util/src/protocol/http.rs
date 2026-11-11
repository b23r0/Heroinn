//! WebSocket transport using tungstenite's synchronous stream API.
//!
//! The proxy-aware `OutboundStream` is established before the WebSocket
//! handshake. Tungstenite therefore never performs its own DNS, proxy or TLS
//! work; it only speaks RFC 6455 over the already-connected byte stream.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tungstenite::client::client_with_config;
use tungstenite::handshake::HandshakeError;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

use crate::HeroinnProtocol;
use crate::error::{Error, Result};
use crate::packet::Message as ProtocolMessage;
use crate::protocol::outbound::{
    OutboundConnector, OutboundReader, OutboundStream, ProxyContext, RouteReport, TransportFactory,
};
use crate::protocol::proxy::{Destination, FrozenRoute, ProxyRoute};
use crate::protocol::{DisconnectHandler, MessageHandler, TUNNEL_FLAG};

const WS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const WS_HANDSHAKE_POLL: Duration = Duration::from_millis(250);
const WS_IO_TIMEOUT: Duration = Duration::from_millis(25);
const WS_CLOSE_FLUSH_TIMEOUT: Duration = Duration::from_secs(1);
const WS_SEND_TIMEOUT: Duration = Duration::from_secs(5);
const WS_SEND_QUEUE: usize = 32;
const WS_RECV_QUEUE: usize = 8;
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
    close: Arc<dyn Fn(Shutdown) + Send + Sync>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl WsSession {
    fn start<S>(
        socket: WebSocket<S>,
        close: Arc<dyn Fn(Shutdown) + Send + Sync>,
    ) -> Result<Arc<Self>>
    where
        S: Read + Write + Send + 'static,
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
        let join = thread::Builder::new()
            .name("ws-actor".to_string())
            .spawn(move || {
                ws_actor(
                    socket,
                    send_rx,
                    recv_tx,
                    actor_stop,
                    actor_close_requested,
                    actor_close_ack,
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
            close,
            join: Mutex::new(Some(join)),
        }))
    }

    fn send(&self, payload: &[u8]) -> Result<()> {
        let deadline = Instant::now() + WS_SEND_TIMEOUT;
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
                match self.send_tx.try_send(command) {
                    Ok(()) => return Ok(()),
                    Err(mpsc::TrySendError::Full(returned)) => command = returned,
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        return Err(Error::Other("ws writer stopped".to_string()));
                    }
                }
            }
            if Instant::now() >= deadline {
                return Err(Error::Timeout);
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn recv(&self) -> Result<Vec<u8>> {
        let receiver = self
            .recv_rx
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match receiver.recv() {
            Ok(WsEvent::Binary(payload)) => Ok(payload),
            Ok(WsEvent::Closed) => Err(Error::Other("ws closed".to_string())),
            Ok(WsEvent::Error(detail)) => Err(Error::Other(detail)),
            Err(_) => Err(Error::Other("ws reader stopped".to_string())),
        }
    }

    fn shutdown(&self) {
        self.stop.store(true, Ordering::Release);
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
            )
    )
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
    stop: Arc<AtomicBool>,
    close_requested: Arc<AtomicBool>,
    close_ack: Arc<Mutex<Option<mpsc::SyncSender<()>>>>,
) where
    S: Read + Write,
{
    let mut pending_in: Option<Vec<u8>> = None;
    let mut pending_out: Option<Vec<u8>> = None;
    let mut flush_pending = false;
    let mut closed_sent = false;
    let mut finished = false;

    while !stop.load(Ordering::Acquire) && !finished {
        // Automatic control replies (notably Pong) must reach the peer before
        // another read can block on an idle connection.
        if flush_pending {
            match socket.flush() {
                Ok(()) => flush_pending = false,
                Err(error) if is_transient_io(&error) => {}
                Err(error) if is_closed_error(&error) => finished = true,
                Err(error) => {
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
                if pending_out.is_none() {
                    match send_rx.try_recv() {
                        Ok(WsCommand::Binary(payload)) => pending_out = Some(payload),
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => break,
                    }
                }
                if let Some(payload) = pending_out.take() {
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
            }
            match socket.close(None) {
                Ok(()) => {}
                Err(error) if is_transient_io(&error) || is_closed_error(&error) => {}
                Err(error) => log::debug!("ws close frame failed: {error}"),
            }
            flush_ws_until(&mut socket, &stop, Instant::now() + WS_CLOSE_FLUSH_TIMEOUT);
            break;
        }

        if let Some(payload) = pending_out.take() {
            match socket.write(Message::Binary(payload.into())) {
                Ok(()) => match socket.flush() {
                    Ok(()) => flush_pending = false,
                    Err(error) if is_transient_io(&error) => flush_pending = true,
                    Err(error) if is_closed_error(&error) => finished = true,
                    Err(error) => {
                        send_ws_event(&recv_tx, WsEvent::Error(format!("ws flush error: {error}")));
                        finished = true;
                    }
                },
                Err(error) if is_transient_io(&error) => {
                    // Tungstenite keeps the failed frame in its own bounded
                    // out-buffer. Re-submitting the payload would duplicate it.
                    flush_pending = true;
                }
                Err(error) => {
                    send_ws_event(&recv_tx, WsEvent::Error(format!("ws send error: {error}")));
                    finished = true;
                    continue;
                }
            }
        }

        // Pull one queued application message before reading. The actor only
        // owns one pending outbound frame, so memory remains bounded by the
        // channel capacity and tungstenite's configured write buffer.
        if pending_out.is_none() && !flush_pending {
            match send_rx.try_recv() {
                Ok(WsCommand::Binary(payload)) => pending_out = Some(payload),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => finished = true,
            }
        }
        if finished {
            break;
        }

        if pending_out.is_some() || flush_pending {
            continue;
        }

        if pending_in.is_none() {
            match socket.read() {
                Ok(Message::Binary(payload)) => {
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
                Ok(Message::Pong(_)) | Ok(Message::Text(_)) => {}
                Ok(Message::Frame(_)) => {}
                Err(error) if is_transient_io(&error) => {}
                Err(error) if is_closed_error(&error) => {
                    send_ws_event(&recv_tx, WsEvent::Closed);
                    closed_sent = true;
                    finished = true;
                }
                Err(error) => {
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
                Ok(()) => {}
                Err(mpsc::TrySendError::Full(WsEvent::Binary(payload))) => {
                    pending_in = Some(payload)
                }
                Err(mpsc::TrySendError::Full(_)) => {}
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    finished = true;
                }
            }
        }

        if pending_in.is_some() || pending_out.is_some() {
            // The consumer is backpressuring us. Keep draining sends, but do
            // not accumulate another incoming message.
            thread::sleep(Duration::from_millis(1));
        }
    }

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
    stop.store(true, Ordering::Release);
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
    connections: Arc<Mutex<HashMap<SocketAddr, Arc<WsSession>>>>,
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
        let connections: Arc<Mutex<HashMap<SocketAddr, Arc<WsSession>>>> =
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
                            let _ = websocket.get_ref().set_read_timeout(Some(WS_IO_TIMEOUT));
                            let _ = websocket.get_ref().set_write_timeout(Some(WS_IO_TIMEOUT));
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
                            listener_connections
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner())
                                .insert(remote_addr, session.clone());
                            let connections = listener_connections.clone();
                            let on_message = on_message.clone();
                            let on_disconnect = on_disconnect.clone();
                            match thread::Builder::new()
                                .name(format!("ws connection : {remote_addr}"))
                                .spawn(move || {
                                    match handle_connection(session, remote_addr, &on_message) {
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
                            thread::sleep(Duration::from_millis(200));
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
                for session in sessions {
                    session.shutdown();
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
        let session = self
            .connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(peer_addr)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("ws client {peer_addr}")))?;
        session.send(buf)
    }

    pub fn contains_addr(&mut self, peer_addr: &SocketAddr) -> bool {
        self.connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(peer_addr)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Release);
        let sessions = self
            .connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .drain()
            .map(|(_, session)| session)
            .collect::<Vec<_>>();
        for session in sessions {
            session.shutdown();
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

fn handle_connection(
    session: Arc<WsSession>,
    remote_addr: SocketAddr,
    on_message: &MessageHandler,
) -> ConnectionEnd {
    loop {
        let buf = match session.recv() {
            Ok(buf) => buf,
            Err(error) => {
                log::info!("ws connection receive finished: {error}");
                return ConnectionEnd::Closed;
            }
        };

        if buf.len() == 6 && buf[..4] == TUNNEL_FLAG {
            let port = u16::from_be_bytes([buf[4], buf[5]]);
            serve_tunnel(session, remote_addr, port);
            return ConnectionEnd::Tunnel;
        }

        match ProtocolMessage::new(remote_addr, HeroinnProtocol::HTTP, &buf) {
            Ok(message) => on_message(message),
            Err(error) => log::warn!("failed to decode ws message from {remote_addr}: {error}"),
        }
    }
}

struct PollingFrameReader {
    stream: std::net::TcpStream,
    header: [u8; 4],
    header_filled: usize,
    payload: Option<Vec<u8>>,
    payload_filled: usize,
}

impl PollingFrameReader {
    fn new(stream: std::net::TcpStream) -> Self {
        Self {
            stream,
            header: [0; 4],
            header_filled: 0,
            payload: None,
            payload_filled: 0,
        }
    }

    fn next_frame(&mut self) -> std::io::Result<Option<Vec<u8>>> {
        loop {
            if self.header_filled < self.header.len() {
                match self.stream.read(&mut self.header[self.header_filled..]) {
                    Ok(0) => {
                        return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
                    }
                    Ok(count) => {
                        self.header_filled += count;
                        continue;
                    }
                    Err(error) if is_io_would_block(&error) => return Ok(None),
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error),
                }
            }

            if self.payload.is_none() {
                let size = u32::from_be_bytes(self.header) as usize;
                if size > crate::protocol::frame::MAX_PACKET {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("tunnel frame too large: {size}"),
                    ));
                }
                self.payload = Some(vec![0; size]);
            }

            let expected = self.payload.as_ref().map_or(0, Vec::len);
            if self.payload_filled < expected {
                let payload = self.payload.as_mut().expect("payload exists");
                match self.stream.read(&mut payload[self.payload_filled..]) {
                    Ok(0) => {
                        return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
                    }
                    Ok(count) => {
                        self.payload_filled += count;
                        continue;
                    }
                    Err(error) if is_io_would_block(&error) => return Ok(None),
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error),
                }
            }

            self.header_filled = 0;
            self.payload_filled = 0;
            return Ok(self.payload.take());
        }
    }
}

fn is_io_would_block(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

fn write_all_polling(
    stream: &mut std::net::TcpStream,
    mut bytes: &[u8],
    cancel: &AtomicBool,
) -> std::io::Result<()> {
    while !bytes.is_empty() {
        if cancel.load(Ordering::Acquire) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "tunnel cancelled",
            ));
        }
        match stream.write(bytes) {
            Ok(0) => return Err(std::io::Error::from(std::io::ErrorKind::WriteZero)),
            Ok(count) => bytes = &bytes[count..],
            Err(error) if is_io_would_block(&error) => {}
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn serve_tunnel(session: Arc<WsSession>, remote_addr: SocketAddr, port: u16) {
    let target: SocketAddr = match format!("127.0.0.1:{port}").parse() {
        Ok(target) => target,
        Err(error) => {
            log::error!("invalid tunnel target: {error}");
            session.shutdown();
            return;
        }
    };
    let local = match std::net::TcpStream::connect_timeout(&target, WS_HANDSHAKE_TIMEOUT) {
        Ok(stream) => stream,
        Err(error) => {
            log::error!("tunnel connect failed: {error}");
            session.shutdown();
            return;
        }
    };
    let _ = local.set_nodelay(true);
    let _ = local.set_read_timeout(Some(WS_IO_TIMEOUT));
    let _ = local.set_write_timeout(Some(WS_IO_TIMEOUT));
    let local_write = match local.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            log::error!("tunnel clone failed: {error}");
            session.shutdown();
            return;
        }
    };
    let cancel = Arc::new(AtomicBool::new(false));

    let local_to_ws = {
        let session = session.clone();
        let cancel = cancel.clone();
        thread::Builder::new()
            .name(format!("ws tunnel local->peer : {remote_addr}"))
            .spawn(move || {
                let mut reader = PollingFrameReader::new(local);
                while !cancel.load(Ordering::Acquire) {
                    match reader.next_frame() {
                        Ok(Some(payload)) => {
                            if session.send(&payload).is_err() {
                                break;
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            log::debug!("tunnel local read failed: {error}");
                            break;
                        }
                    }
                }
                // A local EOF means the sidecar has finished producing
                // upstream frames, but `send` only queues them. Close through
                // the actor so it drains those queued Binary messages and
                // flushes the final frame before shutting the socket down.
                // Shutting down here first would race the writer and truncate
                // the peer's final payload.
                session.close_and_join();
                cancel.store(true, Ordering::Release);
                let _ = reader.stream.shutdown(Shutdown::Both);
            })
    };
    let ws_to_local = {
        let session = session.clone();
        let cancel = cancel.clone();
        thread::Builder::new()
            .name(format!("ws tunnel peer->local : {remote_addr}"))
            .spawn(move || {
                let mut local = local_write;
                while let Ok(payload) = session.recv() {
                    if write_all_polling(&mut local, &(payload.len() as u32).to_be_bytes(), &cancel)
                        .is_err()
                    {
                        break;
                    }
                    if write_all_polling(&mut local, &payload, &cancel).is_err() {
                        break;
                    }
                }
                cancel.store(true, Ordering::Release);
                session.shutdown();
                let _ = local.shutdown(Shutdown::Both);
            })
    };

    match (local_to_ws, ws_to_local) {
        (Ok(first), Ok(second)) => {
            let _ = first.join();
            let _ = second.join();
        }
        (Err(error), second) | (second, Err(error)) => {
            log::warn!("tunnel worker start failed: {error}");
            cancel.store(true, Ordering::Release);
            session.shutdown();
            if let Ok(handle) = second {
                let _ = handle.join();
            }
        }
    }
    log::info!("ws tunnel finished for {remote_addr}");
}

/// One outbound WebSocket connection.
pub struct WSConnection {
    session: Option<Arc<WsSession>>,
    close_reader: OutboundReader,
    local_addr: SocketAddr,
    proxy_context: Option<ProxyContext>,
    route_report: RouteReport,
    frozen_route: Option<FrozenRoute>,
    closed: Arc<AtomicBool>,
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
        Self::connect_with_connector(factory.connector(), address)
    }

    fn connect_with_connector(connector: &OutboundConnector, address: &str) -> Result<Self> {
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
            .set_read_timeout(Some(WS_IO_TIMEOUT))
            .map_err(|error| Error::Other(format!("ws read timeout setup failed: {error}")))?;
        websocket
            .get_ref()
            .set_write_timeout(Some(WS_IO_TIMEOUT))
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
            close_reader,
            local_addr,
            proxy_context,
            route_report,
            frozen_route,
            closed: Arc::new(AtomicBool::new(false)),
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
        connection.send(&handshake)?;
        Ok(connection)
    }

    fn tunnel_with_connector(
        connector: &OutboundConnector,
        remote_addr: &str,
        server_local_port: u16,
    ) -> Result<Self> {
        let mut connection = Self::connect_with_connector(connector, remote_addr)?;
        let mut handshake = TUNNEL_FLAG.to_vec();
        handshake.extend_from_slice(&server_local_port.to_be_bytes());
        connection.send(&handshake)?;
        Ok(connection)
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        let session = self.session()?;
        let result = session.recv();
        if result.is_err() {
            self.closed.store(true, Ordering::Release);
        }
        result
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<()> {
        let session = self.session()?;
        let result = session.send(buf);
        if result.is_err() {
            self.closed.store(true, Ordering::Release);
        }
        result
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
        self.close_reader.shutdown(how).map_err(Error::Io)
    }

    pub fn remote_closed(&self) -> Result<bool> {
        self.close_reader.remote_closed().map_err(Error::Io)
    }

    pub fn wait_for_remote_close(&self, timeout: Duration) -> Result<()> {
        self.close_reader
            .wait_for_remote_close(timeout)
            .map_err(Error::Io)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Release);
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

    fn write_masked_frame(stream: &mut TcpStream, opcode: u8, final_frame: bool, payload: &[u8]) {
        let first = if final_frame { 0x80 | opcode } else { opcode };
        let len = payload.len();
        let mut frame = vec![first];
        if len <= 125 {
            frame.push(0x80 | len as u8);
        } else if len <= u16::MAX as usize {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(len as u64).to_be_bytes());
        }
        let mask = [0x12, 0x34, 0x56, 0x78];
        frame.extend_from_slice(&mask);
        for (index, byte) in payload.iter().enumerate() {
            frame.push(byte ^ mask[index % 4]);
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

    fn ws_client(address: &str) -> tungstenite::WebSocket<std::net::TcpStream> {
        let stream = std::net::TcpStream::connect(address).unwrap();
        let request = ws_request(address).unwrap();
        client_with_config(request, stream, Some(ws_config()))
            .unwrap()
            .0
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
        let mut client = ws_client(&remote);
        client.send(Message::Ping(vec![7, 8].into())).unwrap();
        assert_eq!(client.read().unwrap(), Message::Pong(vec![7, 8].into()));
        client.send(Message::Text("not binary".into())).unwrap();
        assert!(
            receiver.recv_timeout(Duration::from_millis(250)).is_err(),
            "Text must not be treated as an application Binary message"
        );
        client
            .send(Message::Binary(
                ProtocolMessage::build(1, "c", "binary").unwrap().into(),
            ))
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
        let mut client = TcpStream::connect(&address).unwrap();
        raw_handshake(&mut client, &address);
        let payload = ProtocolMessage::build(0x31, "raw", "fragmented").unwrap();
        let split = payload.len() / 2;
        write_masked_frame(&mut client, 0x2, false, &payload[..split]);
        write_masked_frame(&mut client, 0x0, true, &payload[split..]);
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(2)).unwrap().id(),
            0x31
        );
        write_masked_frame(&mut client, 0x9, true, b"ping");
        let (opcode, pong) = read_frame(&mut client, false);
        assert_eq!(opcode, 0xA);
        assert_eq!(pong, b"ping");
        server.close();
    }

    #[test]
    fn rfc6455_client_handshake_and_server_frame_interoperate() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let request = String::from_utf8_lossy(&read_until_headers_end(&mut socket)).to_string();
            let key = request
                .lines()
                .find_map(|line| line.strip_prefix("Sec-WebSocket-Key: "))
                .unwrap()
                .trim()
                .to_string();
            let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
            write!(
                socket,
                "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
                 Connection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
            )
            .unwrap();
            let (opcode, payload) = read_frame(&mut socket, true);
            assert_eq!(opcode, 0x2);
            assert_eq!(payload, ProtocolMessage::build(0x41, "raw", "in").unwrap());
            let reply = ProtocolMessage::build(0x42, "raw", "out").unwrap();
            let mut frame = vec![0x82];
            assert!(reply.len() <= 125);
            frame.push(reply.len() as u8);
            frame.extend_from_slice(&reply);
            socket.write_all(&frame).unwrap();
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
    fn empty_and_above_default_16_mib_payloads_round_trip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(30)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(30)))
                .unwrap();
            let mut websocket = tungstenite::accept_with_config(stream, Some(ws_config())).unwrap();
            assert_eq!(
                websocket.read().unwrap(),
                Message::Binary(Vec::new().into())
            );
            websocket.send(Message::Binary(Vec::new().into())).unwrap();

            let large = websocket.read().unwrap();
            let Message::Binary(large) = large else {
                panic!("expected Binary");
            };
            assert_eq!(large.len(), 16 * 1024 * 1024 + 1);
            assert!(large.iter().all(|byte| *byte == 0xA5));
            websocket.send(Message::Binary(large)).unwrap();
        });

        let mut client = WSConnection::connect_direct(&address).unwrap();
        client.send(&[]).unwrap();
        assert!(client.recv().unwrap().is_empty());
        let large = vec![0xA5; 16 * 1024 * 1024 + 1];
        client.send(&large).unwrap();
        assert_eq!(client.recv().unwrap(), large);
        client.close();
        server.join().unwrap();
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
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let (seen_tx, seen_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut websocket = tungstenite::accept(stream).unwrap();
            assert_eq!(
                websocket.read().unwrap(),
                Message::Binary(b"final-message".to_vec().into())
            );
            assert!(matches!(websocket.read().unwrap(), Message::Close(_)));
            seen_tx.send(()).unwrap();
        });

        let mut client = WSConnection::connect_direct(&address).unwrap();
        let session = client.session.as_ref().unwrap().clone();
        client.send(b"final-message").unwrap();
        let started = Instant::now();
        client.close();
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(session.actor_finished());
        seen_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        server.join().unwrap();
        client.close();
    }

    #[test]
    fn abnormal_eof_and_repeated_close_are_bounded() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let peer = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let websocket = tungstenite::accept(stream).unwrap();
            drop(websocket);
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
        let (release_tx, release_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut websocket = tungstenite::accept(stream).unwrap();
            let _ = release_rx.recv_timeout(Duration::from_secs(5));
            let _ = websocket.read();
        });

        let mut client = WSConnection::connect_direct(&address).unwrap();
        let progress = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sender_progress = progress.clone();
        let mut sender = client.clone();
        let sender = thread::spawn(move || {
            let payload = vec![0x5A; 512 * 1024];
            for _ in 0..100 {
                if sender.send(&payload).is_err() {
                    break;
                }
                sender_progress.fetch_add(1, Ordering::Release);
            }
        });

        let wait_deadline = Instant::now() + Duration::from_secs(5);
        while progress.load(Ordering::Acquire) < WS_SEND_QUEUE && Instant::now() < wait_deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            progress.load(Ordering::Acquire) >= WS_SEND_QUEUE,
            "send queue did not apply backpressure"
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
