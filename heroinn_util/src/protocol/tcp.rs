//! Plain TCP transport: a length-prefixed stream plus the reverse-tunnel handshake.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use socket2::{SockRef, TcpKeepalive};

use crate::error::{Error, Result};
use crate::packet::Message;
use crate::protocol::frame::{MAX_PACKET, read_frame, write_frame};
use crate::protocol::noise::{
    NOISE_HANDSHAKE_TIMEOUT, NoiseIntro, NoiseKind, NoiseSession, NoiseTransport, NoiseWire,
    STREAM_RECORD_MAX, STREAM_RECORD_PAYLOAD, handshake_initiator, handshake_responder,
};
use crate::protocol::outbound::{
    OutboundStream, ProxyContext, RouteReport, TransportFactory, wait_tcp_listener,
};
use crate::protocol::proxy::{Destination, FrozenRoute};
use crate::protocol::{DisconnectHandler, MessageHandler, TUNNEL_FLAG};
use crate::{HeroinnProtocol, cur_timestamp_secs};

const TUNNEL_IDLE_POLL: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConnectionEnd {
    Closed,
    Tunnel,
}

/// Accepting side of a TCP listener.
pub struct TcpServer {
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    connections: Arc<Mutex<HashMap<SocketAddr, TcpConnection>>>,
}

/// One outbound TCP connection.
pub struct TcpConnection {
    s: Option<OutboundStream>,
    proxy_context: Option<ProxyContext>,
    route_report: RouteReport,
    frozen_route: Option<FrozenRoute>,
    closed: Arc<AtomicBool>,
    noise: Option<Arc<NoiseSession>>,
}

impl Clone for TcpConnection {
    fn clone(&self) -> Self {
        Self {
            s: self.s.as_ref().and_then(|stream| stream.try_clone().ok()),
            proxy_context: self.proxy_context.clone(),
            route_report: self.route_report.clone(),
            frozen_route: self.frozen_route.clone(),
            closed: self.closed.clone(),
            noise: self.noise.clone(),
        }
    }
}

const TCP_NOISE_WRITE_BATCH_BYTES: usize = 1024 * 1024;

struct TcpNoiseWriter {
    stream: OutboundStream,
    pending: Vec<u8>,
}

impl TcpNoiseWriter {
    fn new(stream: OutboundStream) -> Self {
        Self {
            stream,
            pending: Vec::with_capacity(TCP_NOISE_WRITE_BATCH_BYTES.min(STREAM_RECORD_MAX * 2)),
        }
    }

    fn queue_frame(&mut self, payload: &[u8]) -> Result<()> {
        if payload.len() > MAX_PACKET {
            return Err(Error::PacketTooLarge(payload.len()));
        }
        if !self.pending.is_empty()
            && self.pending.len() + 4 + payload.len() > TCP_NOISE_WRITE_BATCH_BYTES
        {
            self.flush()?;
        }
        self.pending
            .extend_from_slice(&(payload.len() as u32).to_be_bytes());
        self.pending.extend_from_slice(payload);
        if self.pending.len() >= TCP_NOISE_WRITE_BATCH_BYTES {
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        if !self.pending.is_empty() {
            if let Err(error) = self.stream.write_all(&self.pending) {
                self.pending.clear();
                return Err(Error::Io(error));
            }
            self.pending.clear();
        }
        self.stream.flush().map_err(Error::Io)
    }

    fn flush_timeout(&mut self, timeout: Duration) -> Result<()> {
        self.stream.set_write_timeout(Some(timeout))?;
        let result = self.flush();
        let restore = self.stream.set_write_timeout(None);
        match result {
            Ok(()) => {
                restore?;
                Ok(())
            }
            Err(Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                let _ = restore;
                Err(Error::Timeout)
            }
            Err(error) => {
                let _ = restore;
                Err(error)
            }
        }
    }
}

struct TcpNoiseWire {
    reader: Mutex<OutboundStream>,
    writer: Mutex<TcpNoiseWriter>,
}

impl TcpNoiseWire {
    fn new(stream: &OutboundStream) -> Result<Self> {
        Ok(Self {
            reader: Mutex::new(stream.try_clone()?),
            writer: Mutex::new(TcpNoiseWriter::new(stream.try_clone()?)),
        })
    }
}

impl NoiseWire for TcpNoiseWire {
    fn send_plain(&self, payload: &[u8]) -> Result<()> {
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        writer.queue_frame(payload)
    }

    fn send_plain_timeout(&self, payload: &[u8], timeout: Duration) -> Result<()> {
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        writer.queue_frame(payload)?;
        writer.flush_timeout(timeout)
    }

    fn recv_plain(&self) -> Result<Vec<u8>> {
        let mut reader = self
            .reader
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        read_frame(&mut *reader)
    }

    fn recv_plain_timeout(&self, timeout: Duration) -> Result<Vec<u8>> {
        let mut reader = self
            .reader
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reader.set_read_timeout(Some(timeout))?;
        let result = read_frame(&mut *reader);
        let restore = reader.set_read_timeout(None);
        match result {
            Ok(payload) => {
                restore?;
                Ok(payload)
            }
            Err(Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                let _ = restore;
                Err(Error::Timeout)
            }
            Err(error) => {
                let _ = restore;
                Err(error)
            }
        }
    }

    fn flush_plain(&self) -> Result<()> {
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        writer.flush()
    }
}

impl TcpServer {
    /// Bind `address` and pass every decoded message to `on_message`.
    ///
    /// Each accepted connection gets a worker thread that first peeks at the
    /// 4-byte prefix: [`TUNNEL_FLAG`] turns the connection into a byte pump
    /// toward `127.0.0.1:<port>`, anything else is a length-prefixed frame.
    pub fn new(address: &str, on_message: MessageHandler) -> Result<Self> {
        Self::new_with_disconnect(address, on_message, Arc::new(|_, _| {}))
    }

    pub fn new_with_disconnect(
        address: &str,
        on_message: MessageHandler,
        on_disconnect: DisconnectHandler,
    ) -> Result<Self> {
        let listener = TcpListener::bind(address)?;
        let local_addr = listener.local_addr()?;

        listener.set_nonblocking(true)?;

        let connections = Arc::new(Mutex::new(HashMap::new()));
        let closed = Arc::new(AtomicBool::new(false));

        let closed_1 = closed.clone();
        let connections_1 = connections.clone();

        std::thread::Builder::new()
            .name(format!("tcp listener : {}", local_addr))
            .spawn(move || {
                while !closed_1.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            if closed_1.load(Ordering::Relaxed) {
                                let _ = stream.shutdown(std::net::Shutdown::Both);
                                break;
                            }
                            accept_connection(stream, &on_message, &on_disconnect, &connections_1);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if let Err(error) =
                                wait_tcp_listener(&listener, Duration::from_millis(200))
                            {
                                log::warn!("tcp listener readiness failed: {error}");
                                break;
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                        Err(error) => {
                            log::warn!("tcp accept failed: {error}");
                            break;
                        }
                    }
                }

                let mut conns = connections_1
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                for i in conns.values_mut() {
                    let _ = i.shutdown(std::net::Shutdown::Both);
                }
                conns.clear();
                log::info!("server closed");
            })
            .map_err(|error| Error::Other(format!("cannot start TCP listener worker: {error}")))?;

        Ok(Self {
            local_addr,
            closed,
            connections,
        })
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn sendto(&mut self, peer_addr: &SocketAddr, buf: &[u8]) -> Result<()> {
        let mut conn = self
            .connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(peer_addr)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("tcp client {}", peer_addr)))?;

        conn.send(buf)
    }

    pub fn contains_addr(&mut self, peer_addr: &SocketAddr) -> bool {
        self.connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(peer_addr)
    }
}

/// Register an accepted socket and spawn its reader thread.
fn accept_connection(
    s: TcpStream,
    on_message: &MessageHandler,
    on_disconnect: &DisconnectHandler,
    connections: &Arc<Mutex<HashMap<SocketAddr, TcpConnection>>>,
) {
    if let Err(error) = s.set_nonblocking(false) {
        log::warn!("failed to set TCP connection blocking mode: {}", error);
        return;
    }

    if let Err(e) = SockRef::from(&s)
        .set_tcp_keepalive(&TcpKeepalive::new().with_time(Duration::from_millis(200)))
    {
        log::warn!("failed to enable tcp keepalive : {}", e);
    }

    let peer_addr = match s.peer_addr() {
        Ok(peer_addr) => peer_addr,
        Err(error) => {
            log::warn!("failed to read accepted TCP peer address: {}", error);
            return;
        }
    };
    let connections = connections.clone();
    let on_message = on_message.clone();
    let on_disconnect = on_disconnect.clone();

    if let Err(error) = std::thread::Builder::new()
        .name(format!("tcp connection : {}", peer_addr))
        .spawn(move || {
            let (mut connection, tunnel_port) =
                match accept_remote_connection(s, peer_addr, &connections) {
                    Ok(connection) => connection,
                    Err(error) => {
                        log::warn!(
                            "tcp encrypted handshake failed for {}: {}",
                            peer_addr,
                            error
                        );
                        return;
                    }
                };
            let end = match tunnel_port {
                Some(port) => {
                    serve_tunnel(&mut connection, peer_addr, port);
                    ConnectionEnd::Tunnel
                }
                None => handle_connection(&mut connection, peer_addr, &on_message),
            };
            connections
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&peer_addr);
            connection.close();
            match end {
                ConnectionEnd::Closed => {
                    log::info!("connection closed : {}", peer_addr);
                    on_disconnect(peer_addr, HeroinnProtocol::TCP);
                }
                ConnectionEnd::Tunnel => {
                    log::info!("connection entered tunnel : {}", peer_addr);
                }
            }
        })
    {
        log::warn!("failed to start TCP connection worker: {}", error);
    }
}

/// Read encrypted application frames until the peer disconnects.
fn handle_connection(
    connection: &mut TcpConnection,
    peer_addr: SocketAddr,
    on_message: &MessageHandler,
) -> ConnectionEnd {
    loop {
        let buf = match connection.recv() {
            Ok(buf) => buf,
            Err(error) => {
                log::debug!("tcp connection receive finished for {peer_addr}: {error}");
                return ConnectionEnd::Closed;
            }
        };

        match Message::new(peer_addr, HeroinnProtocol::TCP, &buf) {
            Ok(msg) => on_message(msg),
            Err(e) => log::warn!("failed to decode tcp message from {}: {}", peer_addr, e),
        }
    }
}

/// Accept the Noise preface and handshake, then register a main connection.
fn accept_remote_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    connections: &Arc<Mutex<HashMap<SocketAddr, TcpConnection>>>,
) -> Result<(TcpConnection, Option<u16>)> {
    let stream = OutboundStream::from_tcp(stream)?;
    let wire = Arc::new(TcpNoiseWire::new(&stream)?);

    let (intro, kind, tunnel_port) = {
        let mut reader = wire
            .reader
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reader.set_read_timeout(Some(NOISE_HANDSHAKE_TIMEOUT))?;
        let mut prefix = [0u8; 4];
        let result = (|| {
            reader.read_exact(&mut prefix)?;
            // The tunnel client writes the four-byte flag directly instead of
            // using the normal frame-length prefix.
            if prefix == TUNNEL_FLAG {
                let mut port = [0u8; 2];
                reader.read_exact(&mut port)?;
                let port = u16::from_be_bytes(port);
                let intro = read_frame(&mut *reader)?;
                return Ok((intro, NoiseKind::Tunnel { port }, Some(port)));
            }
            let size = u32::from_be_bytes(prefix) as usize;
            if size > MAX_PACKET {
                return Err(Error::PacketTooLarge(size));
            }
            let mut intro = vec![0u8; size];
            reader.read_exact(&mut intro)?;
            Ok((intro, NoiseKind::Main, None))
        })();
        let restore = reader.set_read_timeout(None);
        let result = result?;
        restore?;
        result
    };

    let session = handshake_responder(
        wire,
        &intro,
        NoiseTransport::Tcp,
        kind,
        STREAM_RECORD_PAYLOAD,
        STREAM_RECORD_MAX,
    )?;
    let connection = TcpConnection {
        s: Some(stream),
        proxy_context: None,
        route_report: RouteReport::default(),
        frozen_route: None,
        closed: Arc::new(AtomicBool::new(false)),
        noise: Some(Arc::new(session)),
    };
    if tunnel_port.is_none() {
        connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(peer_addr, connection.clone());
    }
    Ok((connection, tunnel_port))
}

/// Pump application frames both ways between the encrypted tunnel and a fresh
/// plaintext loopback sidecar connection.
fn serve_tunnel(connection: &mut TcpConnection, peer_addr: SocketAddr, port: u16) {
    let full_addr = format!("127.0.0.1:{}", port);
    let local = match TcpConnection::connect_local_plain(&full_addr) {
        Ok(connection) => connection,
        Err(error) => {
            log::error!("tunnel connect failed : {}", error);
            connection.close();
            return;
        }
    };
    log::debug!("tunnel started peer={peer_addr} local={full_addr}");

    let mut local_reader = local.clone();
    let mut local_writer = local.clone();
    let mut remote_reader = connection.clone();
    let mut remote_writer = connection.clone();
    let cancel_local = Arc::new(AtomicBool::new(false));
    let first_cancel = cancel_local.clone();
    let second_cancel = cancel_local;
    let first_name = format!("tunnel worker1 : {} , {}", peer_addr, full_addr);
    let second_name = format!("tunnel worker2 : {} , {}", peer_addr, full_addr);
    std::thread::scope(|scope| {
        let first = std::thread::Builder::new()
            .name(first_name)
            .spawn_scoped(scope, move || {
                let mut partial = Vec::new();
                while !first_cancel.load(Ordering::Acquire) {
                    match local_reader.recv_poll(&mut partial, TUNNEL_IDLE_POLL) {
                        Ok(Some(payload)) => {
                            if let Err(error) = remote_writer.send(&payload) {
                                let _ = remote_writer.shutdown(std::net::Shutdown::Both);
                                let _ = local_reader.shutdown(std::net::Shutdown::Both);
                                log::debug!("tunnel local-to-remote ended: {error}");
                                break;
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            log::debug!("tunnel local read ended: {error}");
                            break;
                        }
                    }
                }
                log::debug!(
                    "tunnel local-to-remote stopped cancelled={}",
                    first_cancel.load(Ordering::Acquire)
                );
                let _ = remote_writer.finish();
                let _ = local_reader.shutdown(std::net::Shutdown::Read);
            });
        let second = std::thread::Builder::new()
            .name(second_name)
            .spawn_scoped(scope, move || {
                loop {
                    match remote_reader.recv() {
                        Ok(payload) => {
                            if let Err(error) = local_writer.send(&payload) {
                                let _ = remote_reader.shutdown(std::net::Shutdown::Both);
                                let _ = local_writer.shutdown(std::net::Shutdown::Both);
                                log::debug!("tunnel remote-to-local ended: {error}");
                                break;
                            }
                        }
                        Err(error) => {
                            log::debug!("tunnel remote receive ended: {error}");
                            break;
                        }
                    }
                }
                let remote_finished = remote_reader.remote_closed().unwrap_or(false);
                log::debug!("tunnel remote-to-local stopped remote_finished={remote_finished}");
                if !remote_finished {
                    // Preserve TCP half-close semantics: a clean remote FIN
                    // ends only peer->local. Cancelling local->peer here can
                    // discard the local side's final response.
                    second_cancel.store(true, Ordering::Release);
                }
                let how = if remote_finished {
                    std::net::Shutdown::Write
                } else {
                    std::net::Shutdown::Both
                };
                let _ = local_writer.shutdown(how);
            });
        match (first, second) {
            (Ok(first), Ok(second)) => {
                let _ = first.join();
                let _ = second.join();
            }
            (Err(error), second) | (second, Err(error)) => {
                log::warn!("tunnel worker start failed: {error}");
                let _ = connection.shutdown(std::net::Shutdown::Both);
                let _ = local.shutdown(std::net::Shutdown::Both);
                if let Ok(handle) = second {
                    let _ = handle.join();
                }
            }
        }
    });
    log::debug!("tunnel finished for {peer_addr}");
}

impl TcpConnection {
    /// Dial `address` directly, bypassing proxy discovery.
    pub fn connect_direct(address: &str) -> Result<Self> {
        let address: SocketAddr = address
            .parse()
            .map_err(|e| Error::Other(format!("address format error : {}", e)))?;

        let stream = OutboundStream::from_tcp(TcpStream::connect(address)?)?;
        let noise = Self::start_noise(&stream, NoiseKind::Main)?;
        Ok(Self {
            s: Some(stream),
            proxy_context: None,
            route_report: RouteReport::default(),
            frozen_route: None,
            closed: Arc::new(AtomicBool::new(false)),
            noise: Some(noise),
        })
    }

    /// Backward-compatible direct dialing entry point.
    pub fn connect(address: &str) -> Result<Self> {
        Self::connect_direct(address)
    }

    /// Dial `address` using the shared outbound factory.
    pub fn connect_with_factory(factory: &TransportFactory, address: &str) -> Result<Self> {
        let destination = Destination::parse(address)?;
        let stream = factory.open(&destination)?;
        let proxy_context = stream.proxy_context().cloned();
        let route_report = stream.route_report().clone();
        let frozen_route = stream.frozen_route().cloned();
        let noise = Self::start_noise(&stream, NoiseKind::Main)?;
        Ok(Self {
            s: Some(stream),
            proxy_context,
            route_report,
            frozen_route,
            closed: Arc::new(AtomicBool::new(false)),
            noise: Some(noise),
        })
    }

    /// Dial a reverse tunnel directly, bypassing proxy discovery.
    pub fn tunnel_direct(remote_addr: &str, server_local_port: u16) -> Result<Self> {
        let remote_addr: SocketAddr = remote_addr
            .parse()
            .map_err(|e| Error::Other(format!("address format error : {}", e)))?;

        log::info!("start tunnel [{}] [{}]", remote_addr, server_local_port);
        let mut s = OutboundStream::from_tcp(TcpStream::connect(remote_addr)?)?;

        s.write_all(&TUNNEL_FLAG)?;
        s.write_all(&server_local_port.to_be_bytes())?;
        let noise = Self::start_noise(
            &s,
            NoiseKind::Tunnel {
                port: server_local_port,
            },
        )?;

        Ok(Self {
            s: Some(s),
            proxy_context: None,
            route_report: RouteReport::default(),
            frozen_route: None,
            closed: Arc::new(AtomicBool::new(false)),
            noise: Some(noise),
        })
    }

    /// Backward-compatible direct tunnel entry point.
    pub fn tunnel(remote_addr: &str, server_local_port: u16) -> Result<Self> {
        Self::tunnel_direct(remote_addr, server_local_port)
    }

    /// Dial a tunnel using the shared outbound factory.
    pub fn tunnel_with_factory(
        factory: &TransportFactory,
        remote_addr: &str,
        server_local_port: u16,
    ) -> Result<Self> {
        let destination = Destination::parse(remote_addr)?;
        log::info!("start tunnel [{}] [{}]", remote_addr, server_local_port);
        let mut s = factory.open(&destination)?;
        let proxy_context = s.proxy_context().cloned();
        let route_report = s.route_report().clone();
        let frozen_route = s.frozen_route().cloned();
        s.write_all(&TUNNEL_FLAG)?;
        s.write_all(&server_local_port.to_be_bytes())?;
        let noise = Self::start_noise(
            &s,
            NoiseKind::Tunnel {
                port: server_local_port,
            },
        )?;
        Ok(Self {
            s: Some(s),
            proxy_context,
            route_report,
            frozen_route,
            closed: Arc::new(AtomicBool::new(false)),
            noise: Some(noise),
        })
    }

    /// Dial a reverse tunnel using the exact route snapshot from the main
    /// connection. The handshake target is still the same Heroinn server;
    /// only the server-local tunnel port changes.
    pub fn tunnel_with_frozen_route(
        factory: &TransportFactory,
        remote_addr: &str,
        server_local_port: u16,
        frozen: &FrozenRoute,
    ) -> Result<Self> {
        let destination = Destination::parse(remote_addr)?;
        log::info!(
            "start frozen-route tunnel [{}] [{}]",
            remote_addr,
            server_local_port
        );
        let mut s = factory.open_frozen(&destination, frozen)?;
        let proxy_context = s.proxy_context().cloned();
        let route_report = s.route_report().clone();
        let frozen_route = s.frozen_route().cloned();
        s.write_all(&TUNNEL_FLAG)?;
        s.write_all(&server_local_port.to_be_bytes())?;
        let noise = Self::start_noise(
            &s,
            NoiseKind::Tunnel {
                port: server_local_port,
            },
        )?;
        Ok(Self {
            s: Some(s),
            proxy_context,
            route_report,
            frozen_route,
            closed: Arc::new(AtomicBool::new(false)),
            noise: Some(noise),
        })
    }

    /// Accept one tunnelled connection, waiting up to `timeout_secs`.
    pub fn tunnel_server(server: TcpListener, timeout_secs: u64) -> Result<(Self, SocketAddr)> {
        server.set_nonblocking(true)?;

        let start = cur_timestamp_secs();

        loop {
            let (s, addr) = match server.accept() {
                Ok(p) => p,
                Err(e) => {
                    if cur_timestamp_secs() - start > timeout_secs {
                        return Err(e.into());
                    }

                    std::thread::sleep(Duration::from_millis(200));
                    continue;
                }
            };

            s.set_nonblocking(false)?;
            s.set_nodelay(true)?;

            return Ok((
                Self {
                    s: Some(OutboundStream::from_tcp(s)?),
                    proxy_context: None,
                    route_report: RouteReport::default(),
                    frozen_route: None,
                    closed: Arc::new(AtomicBool::new(false)),
                    noise: None,
                },
                addr,
            ));
        }
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Other("socket closed".to_string()));
        }
        if let Some(noise) = &self.noise {
            return noise.recv();
        }
        let s = self.stream_mut()?;
        read_frame(s)
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<()> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Other("socket closed".to_string()));
        }
        if let Some(noise) = &self.noise {
            return noise.send(buf);
        }
        let s = self.stream_mut()?;
        write_frame(s, buf)
    }

    /// Send the authenticated Noise FIN. Plaintext loopback connections only
    /// flush their final frame.
    pub fn finish(&self) -> Result<()> {
        if let Some(noise) = &self.noise {
            noise.finish()
        } else {
            let mut stream = self.stream()?.try_clone()?;
            stream.flush().map_err(Error::Io)
        }
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        if let Some(noise) = &self.noise {
            noise.close();
        }
        if let Some(stream) = self.s.take() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }

    /// Read one plaintext frame while retaining partial frame state between
    /// bounded read timeouts. This is used only by the trusted loopback
    /// sidecar pump; an encrypted connection must use `NoiseSession::recv`.
    pub fn recv_poll(
        &mut self,
        partial: &mut Vec<u8>,
        timeout: Duration,
    ) -> Result<Option<Vec<u8>>> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Other("socket closed".to_string()));
        }
        if self.noise.is_some() {
            return Err(Error::Other(
                "recv_poll is only valid for a plaintext loopback sidecar".to_string(),
            ));
        }

        let stream = self.stream_mut()?;
        stream.set_read_timeout(Some(timeout))?;
        let mut chunk = [0u8; 16 * 1024];
        loop {
            if partial.len() >= 4 {
                let size =
                    u32::from_be_bytes(partial[..4].try_into().expect("four-byte prefix")) as usize;
                if size > MAX_PACKET {
                    partial.clear();
                    return Err(Error::PacketTooLarge(size));
                }
                if partial.len() >= 4 + size {
                    let payload = partial[4..4 + size].to_vec();
                    partial.drain(..4 + size);
                    return Ok(Some(payload));
                }
            }

            match stream.read(&mut chunk) {
                Ok(0) => {
                    return Err(Error::Io(std::io::Error::from(
                        std::io::ErrorKind::UnexpectedEof,
                    )));
                }
                Ok(read) => {
                    partial.extend_from_slice(&chunk[..read]);
                    if partial.len() > 4 + MAX_PACKET + chunk.len() {
                        partial.clear();
                        return Err(Error::PacketTooLarge(MAX_PACKET + 1));
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    return Ok(None);
                }
                Err(error) => return Err(Error::Io(error)),
            }
        }
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.stream()?.local_addr()?)
    }

    fn peer_addr(&self) -> Result<SocketAddr> {
        Ok(self.stream()?.peer_addr()?)
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

    pub fn shutdown(&self, how: std::net::Shutdown) -> Result<()> {
        if matches!(how, std::net::Shutdown::Write | std::net::Shutdown::Both)
            && let Some(noise) = &self.noise
        {
            let _ = noise.finish();
        }
        self.stream()?.shutdown(how).map_err(Error::Io)
    }

    pub fn remote_closed(&self) -> Result<bool> {
        if self
            .noise
            .as_ref()
            .is_some_and(|session| session.remote_finished())
        {
            return Ok(true);
        }
        self.stream()?.remote_closed().map_err(Error::Io)
    }

    pub fn wait_for_remote_close(&self, timeout: Duration) -> Result<()> {
        self.stream()?
            .wait_for_remote_close(timeout)
            .map_err(Error::Io)
    }

    /// Borrow the live stream, failing if the connection is already closed.
    fn stream(&self) -> Result<&OutboundStream> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Other("socket closed".to_string()));
        }

        self.s
            .as_ref()
            .ok_or_else(|| Error::Other("socket closed".to_string()))
    }

    /// Mutably borrow the live stream, failing if the connection is already closed.
    fn stream_mut(&mut self) -> Result<&mut OutboundStream> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Other("socket closed".to_string()));
        }

        self.s
            .as_mut()
            .ok_or_else(|| Error::Other("socket closed".to_string()))
    }

    fn start_noise(stream: &OutboundStream, kind: NoiseKind) -> Result<Arc<NoiseSession>> {
        let wire: Arc<dyn NoiseWire> = Arc::new(TcpNoiseWire::new(stream)?);
        let intro = NoiseIntro::new(NoiseTransport::Tcp, kind);
        Ok(Arc::new(handshake_initiator(
            wire,
            intro,
            STREAM_RECORD_PAYLOAD,
            STREAM_RECORD_MAX,
        )?))
    }

    /// Connect to a server-local sidecar over the trusted loopback domain.
    pub fn connect_local_plain(address: &str) -> Result<Self> {
        let address: SocketAddr = address
            .parse()
            .map_err(|error| Error::Other(format!("address format error : {error}")))?;
        Ok(Self {
            s: Some(OutboundStream::from_tcp(TcpStream::connect(address)?)?),
            proxy_context: None,
            route_report: RouteReport::default(),
            frozen_route: None,
            closed: Arc::new(AtomicBool::new(false)),
            noise: None,
        })
    }
}

impl Drop for TcpConnection {
    fn drop(&mut self) {
        if let Some(s) = self.s.as_ref() {
            match s.peer_addr() {
                Ok(peer) => log::info!("tcp client [{}] dropped", peer),
                Err(error) => {
                    log::debug!(
                        "tcp client with unavailable peer address dropped: {}",
                        error
                    )
                }
            }
            self.s = None;
        } else {
            log::info!("tcp client dropped");
        }
    }
}

impl Drop for TcpServer {
    fn drop(&mut self) {
        self.close();
        let connections = self
            .connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for connection in connections.values() {
            match connection.peer_addr() {
                Ok(peer) => log::info!("tcp [{}] dropped", peer),
                Err(error) => {
                    log::debug!(
                        "tcp server connection with unavailable peer address: {}",
                        error
                    )
                }
            }
        }
    }
}

#[test]
fn test_tcp_tunnel() {
    let mut server = TcpServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let server2 = TcpListener::bind("127.0.0.1:0").unwrap();
    let remote_local_port = server2.local_addr().unwrap().port();

    let remote = &format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let mut client1 = TcpConnection::tunnel(remote, remote_local_port).unwrap();

    let (mut client2, _) = TcpConnection::tunnel_server(server2, 10).unwrap();

    for _ in 0..3 {
        client1.send(&[0, 1, 2, 3]).unwrap();
        let buf = client2.recv().unwrap();
        assert!(buf == [0, 1, 2, 3]);

        client2.send(&[4, 5, 6, 7]).unwrap();
        let buf = client1.recv().unwrap();
        assert!(buf == [4, 5, 6, 7]);
    }

    server.close();
}

#[test]
fn queued_connections_complete_handshakes_without_per_accept_delay() {
    let mut server = TcpServer::new("127.0.0.1:0", Arc::new(|_| {})).expect("server");
    let address = server.local_addr().expect("address").to_string();
    let start = Arc::new(std::sync::Barrier::new(5));
    let (completed, results) = std::sync::mpsc::channel();
    let count = std::thread::scope(|scope| {
        for _ in 0..4 {
            let address = address.clone();
            let start = start.clone();
            let completed = completed.clone();
            scope.spawn(move || {
                start.wait();
                let mut connection = TcpConnection::connect_direct(&address).expect("handshake");
                completed.send(()).expect("completion");
                connection.close();
            });
        }
        start.wait();
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        let mut count = 0;
        while count < 4
            && results
                .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                .is_ok()
        {
            count += 1;
        }
        count
    });
    server.close();
    assert_eq!(count, 4, "queued accepts must not each incur a 200ms delay");
}

#[test]
fn dropping_connection_after_remote_close_does_not_panic() {
    enable_test_logging();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (stream, _) = listener.accept().unwrap();
    let connection = TcpConnection {
        s: Some(OutboundStream::from_tcp(stream).unwrap()),
        proxy_context: None,
        route_report: RouteReport::default(),
        frozen_route: None,
        closed: Arc::new(AtomicBool::new(false)),
        noise: None,
    };
    socket2::SockRef::from(&client)
        .set_linger(Some(Duration::ZERO))
        .unwrap();
    drop(client);
    std::thread::sleep(Duration::from_millis(50));
    let _ = connection.shutdown(std::net::Shutdown::Both);
    std::thread::sleep(Duration::from_millis(50));
    #[cfg(unix)]
    assert!(
        connection.stream().unwrap().peer_addr().is_err(),
        "test precondition: peer address should be unavailable after the peer reset"
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(connection)));
    assert!(
        result.is_ok(),
        "dropping a closed connection must not panic"
    );
}

#[test]
fn dropping_tcp_server_after_remote_close_does_not_panic() {
    enable_test_logging();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (stream, _) = listener.accept().unwrap();
    socket2::SockRef::from(&client)
        .set_linger(Some(Duration::ZERO))
        .unwrap();
    drop(client);
    std::thread::sleep(Duration::from_millis(50));
    #[cfg(unix)]
    assert!(
        stream.peer_addr().is_err(),
        "test precondition: server peer address should be unavailable after the client reset"
    );

    let connection = TcpConnection {
        s: Some(OutboundStream::from_tcp(stream).unwrap()),
        proxy_context: None,
        route_report: RouteReport::default(),
        frozen_route: None,
        closed: Arc::new(AtomicBool::new(false)),
        noise: None,
    };
    let mut connections = HashMap::new();
    connections.insert(address, connection);
    let server = TcpServer {
        local_addr: address,
        closed: Arc::new(AtomicBool::new(false)),
        connections: Arc::new(Mutex::new(connections)),
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(server)));
    assert!(
        result.is_ok(),
        "dropping a server with a closed peer must not panic"
    );
}

#[cfg(test)]
fn enable_test_logging() {
    struct TestLogger;

    impl log::Log for TestLogger {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.level() <= log::Level::Info
        }

        fn log(&self, _record: &log::Record<'_>) {}

        fn flush(&self) {}
    }

    static LOGGER: TestLogger = TestLogger;
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Info);
}

#[test]
fn tcp_server_reports_transport_disconnect_immediately() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut server = TcpServer::new_with_disconnect(
        "127.0.0.1:0",
        Arc::new(|_| {}),
        Arc::new(move |peer, protocol| {
            let _ = sender.send((peer, protocol));
        }),
    )
    .unwrap();
    let address = server.local_addr().unwrap();
    let client = TcpConnection::connect(&address.to_string()).unwrap();
    drop(client);
    let (peer, protocol) = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("disconnect callback");
    assert_eq!(peer.ip(), address.ip());
    assert_eq!(protocol, HeroinnProtocol::TCP);
    server.close();
}

#[test]
fn plaintext_legacy_client_is_rejected_without_delivery() {
    let (message_sender, message_receiver) = std::sync::mpsc::channel();
    let mut server = TcpServer::new(
        "127.0.0.1:0",
        Arc::new(move |message| {
            let _ = message_sender.send(message.id());
        }),
    )
    .unwrap();
    let address = server.local_addr().unwrap();
    let mut client = TcpStream::connect(address).unwrap();
    let plaintext = Message::build(0x31, "legacy", "plaintext").unwrap();
    write_frame(&mut client, &plaintext).unwrap();
    assert!(
        message_receiver
            .recv_timeout(Duration::from_secs(2))
            .is_err(),
        "plaintext must never reach the application handler"
    );
    server.close();
}

#[test]
fn recv_poll_preserves_split_prefix_and_payload_across_read_timeouts() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (first_sent, first_received) = std::sync::mpsc::channel();
    let (second_sent, second_received) = std::sync::mpsc::channel();
    let (continue_first_sender, continue_first_receiver) = std::sync::mpsc::channel();
    let (continue_second_sender, continue_second_receiver) = std::sync::mpsc::channel();
    let payload = b"split-frame-payload".to_vec();
    let expected = payload.clone();

    let writer = std::thread::spawn(move || {
        let (mut peer, _) = listener.accept().unwrap();
        let encoded_len = (payload.len() as u32).to_be_bytes();
        peer.write_all(&encoded_len[..2]).unwrap();
        first_sent.send(()).unwrap();
        continue_first_receiver.recv().unwrap();
        peer.write_all(&encoded_len[2..]).unwrap();
        peer.write_all(&payload[..5]).unwrap();
        second_sent.send(()).unwrap();
        continue_second_receiver.recv().unwrap();
        peer.write_all(&payload[5..]).unwrap();
    });

    let stream = TcpStream::connect(address).unwrap();
    let mut reader = TcpConnection {
        s: Some(OutboundStream::from_tcp(stream).unwrap()),
        proxy_context: None,
        route_report: RouteReport::default(),
        frozen_route: None,
        closed: Arc::new(AtomicBool::new(false)),
        noise: None,
    };
    first_received
        .recv_timeout(Duration::from_secs(2))
        .expect("writer sent the partial frame");
    let mut partial = Vec::new();
    assert!(
        reader
            .recv_poll(&mut partial, Duration::from_millis(20))
            .unwrap()
            .is_none(),
        "a timed-out partial frame must not be reported as complete"
    );
    assert!(
        partial.len() < 4 + expected.len(),
        "the prefix and partial payload must remain buffered"
    );

    continue_first_sender.send(()).unwrap();
    second_received
        .recv_timeout(Duration::from_secs(2))
        .expect("writer sent the remaining prefix and partial payload");
    assert!(
        reader
            .recv_poll(&mut partial, Duration::from_millis(20))
            .unwrap()
            .is_none(),
        "a second timed-out partial payload must remain buffered"
    );
    assert!(
        partial.len() >= 4 + 5 && partial.len() < 4 + expected.len(),
        "the complete prefix and partial payload must remain buffered"
    );

    continue_second_sender.send(()).unwrap();
    let received = loop {
        if let Some(frame) = reader
            .recv_poll(&mut partial, Duration::from_millis(100))
            .unwrap()
        {
            break frame;
        }
    };
    assert_eq!(received, expected);
    assert!(partial.is_empty());
    writer.join().unwrap();
}

#[test]
fn recv_poll_rejects_oversized_prefix_and_eof_without_stale_state() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (ready_sender, ready_receiver) = std::sync::mpsc::channel();
    let (continue_sender, continue_receiver) = std::sync::mpsc::channel();
    let writer = std::thread::spawn(move || {
        let (mut peer, _) = listener.accept().unwrap();
        ready_sender.send(()).unwrap();
        continue_receiver.recv().unwrap();
        peer.write_all(&((MAX_PACKET + 1) as u32).to_be_bytes())
            .unwrap();
    });

    let stream = TcpStream::connect(address).unwrap();
    let mut reader = TcpConnection {
        s: Some(OutboundStream::from_tcp(stream).unwrap()),
        proxy_context: None,
        route_report: RouteReport::default(),
        frozen_route: None,
        closed: Arc::new(AtomicBool::new(false)),
        noise: None,
    };
    ready_receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("writer connected");
    continue_sender.send(()).unwrap();
    let mut partial = Vec::new();
    let error = loop {
        match reader.recv_poll(&mut partial, Duration::from_millis(100)) {
            Ok(Some(_)) => panic!("oversized prefix was accepted"),
            Ok(None) => continue,
            Err(error) => break error,
        }
    };
    assert!(matches!(
        error,
        Error::PacketTooLarge(size) if size == MAX_PACKET + 1
    ));
    assert!(
        partial.is_empty(),
        "oversized prefix must clear retained frame state"
    );
    writer.join().unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (ready_sender, ready_receiver) = std::sync::mpsc::channel();
    let (eof_sender, eof_receiver) = std::sync::mpsc::channel();
    let writer = std::thread::spawn(move || {
        let (mut peer, _) = listener.accept().unwrap();
        peer.write_all(&[0, 0]).unwrap();
        ready_sender.send(()).unwrap();
        eof_receiver.recv().unwrap();
    });
    let stream = TcpStream::connect(address).unwrap();
    let mut reader = TcpConnection {
        s: Some(OutboundStream::from_tcp(stream).unwrap()),
        proxy_context: None,
        route_report: RouteReport::default(),
        frozen_route: None,
        closed: Arc::new(AtomicBool::new(false)),
        noise: None,
    };
    ready_receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("writer sent a partial prefix");
    let mut partial = Vec::new();
    assert!(
        reader
            .recv_poll(&mut partial, Duration::from_millis(20))
            .unwrap()
            .is_none(),
        "a truncated prefix must wait for more bytes"
    );
    assert_eq!(partial.len(), 2);
    eof_sender.send(()).unwrap();
    let error = loop {
        match reader.recv_poll(&mut partial, Duration::from_millis(100)) {
            Ok(Some(_)) => panic!("truncated prefix was accepted"),
            Ok(None) => continue,
            Err(error) => break error,
        }
    };
    assert!(matches!(
        error,
        Error::Io(error) if error.kind() == std::io::ErrorKind::UnexpectedEof
    ));
    assert!(
        !partial.is_empty(),
        "EOF must not fabricate a complete frame"
    );
    writer.join().unwrap();
}

#[test]
fn role_mismatch_is_rejected_before_tcp_application_delivery() {
    let (message_sender, message_receiver) = std::sync::mpsc::channel();
    let mut server = TcpServer::new(
        "127.0.0.1:0",
        Arc::new(move |message| {
            let _ = message_sender.send(message.id());
        }),
    )
    .unwrap();
    let address = server.local_addr().unwrap();
    let mut client = TcpStream::connect(address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut intro = NoiseIntro::new(NoiseTransport::Tcp, NoiseKind::Main).encode();
    intro[12..14].copy_from_slice(&2u16.to_be_bytes());
    write_frame(&mut client, &intro).unwrap();

    let mut byte = [0u8; 1];
    let closed = match client.read(&mut byte) {
        Ok(0) | Err(_) => true,
        Ok(_) => false,
    };
    assert!(
        closed,
        "role mismatch was not closed before application data"
    );
    assert!(
        message_receiver
            .recv_timeout(Duration::from_millis(500))
            .is_err(),
        "role mismatch reached the TCP application handler"
    );
    server.close();
}

#[test]
fn tcp_tunnel_does_not_report_a_main_transport_disconnect() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut server = TcpServer::new_with_disconnect(
        "127.0.0.1:0",
        Arc::new(|_| {}),
        Arc::new(move |peer, protocol| {
            let _ = sender.send((peer, protocol));
        }),
    )
    .unwrap();
    let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let tunnel_port = tunnel_listener.local_addr().unwrap().port();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());

    let mut tunnel = TcpConnection::tunnel(&remote, tunnel_port).unwrap();
    let (mut local, _) = TcpConnection::tunnel_server(tunnel_listener, 5).unwrap();
    tunnel.send(b"still-connected").unwrap();
    assert_eq!(local.recv().unwrap(), b"still-connected");

    assert!(
        receiver.recv_timeout(Duration::from_millis(500)).is_err(),
        "starting a data tunnel must not be reported as main-transport disconnect"
    );
    server.close();
}

#[cfg(test)]
mod proxy_transport_tests {
    use super::*;
    use crate::protocol::outbound::{OutboundConnector, TransportFactory};
    use crate::protocol::proxy::{ProxyEnvironment, ProxyKind};
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::thread;

    fn route_with_http(
        target: &str,
        proxy_address: SocketAddr,
    ) -> crate::protocol::proxy::ProxyRoute {
        let environment = ProxyEnvironment {
            http_proxy: Some(format!("http://{proxy_address}")),
            https_proxy: None,
            all_proxy: None,
            no_proxy: None,
        };
        crate::protocol::proxy::resolve_target_with(target, &environment, None).unwrap()
    }

    fn spawn_http_proxy() -> (SocketAddr, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let accepted_worker = accepted.clone();

        thread::spawn(move || {
            let (mut client, _) = listener.accept().unwrap();
            accepted_worker.fetch_add(1, Ordering::SeqCst);
            let mut request = Vec::new();
            let mut byte = [0u8; 1];
            while request.len() < 16 * 1024 {
                use std::io::Read;
                client.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
                if request.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&request);
            let authority = request
                .lines()
                .next()
                .and_then(|line| line.strip_prefix("CONNECT "))
                .and_then(|line| line.split_whitespace().next())
                .unwrap()
                .to_string();
            let upstream = TcpStream::connect(authority).unwrap();
            client
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .unwrap();
            let mut upstream_reader = upstream.try_clone().unwrap();
            let mut upstream_writer = upstream;
            let mut client_writer = client.try_clone().unwrap();
            thread::spawn(move || {
                let _ = std::io::copy(&mut upstream_reader, &mut client_writer);
                let _ = client_writer.shutdown(std::net::Shutdown::Both);
            });
            let mut client_reader = client;
            let _ = std::io::copy(&mut client_reader, &mut upstream_writer);
            let _ = upstream_writer.shutdown(std::net::Shutdown::Both);
        });

        (address, accepted)
    }

    #[test]
    fn tcp_direct_main_connection_delivers_frames() {
        let (message_sender, message_receiver) = mpsc::channel();
        let mut server = TcpServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = message_sender.send(message.length());
            }),
        )
        .unwrap();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let mut client = TcpConnection::connect(&remote).unwrap();
        let frame = crate::packet::Message::build(0, "test", serde_json::json!({})).unwrap();
        client.send(&frame).unwrap();
        assert!(
            message_receiver
                .recv_timeout(Duration::from_secs(3))
                .unwrap()
                > 0
        );
        server.close();
    }

    #[test]
    fn tcp_main_connection_uses_http_proxy() {
        let (message_sender, message_receiver) = mpsc::channel();
        let callback_sender = message_sender;
        let mut server = TcpServer::new(
            "127.0.0.1:0",
            Arc::new(move |message| {
                let _ = callback_sender.send(message.length());
            }),
        )
        .unwrap();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let (proxy, accepted) = spawn_http_proxy();
        let route = route_with_http(&remote, proxy);
        let connector = Arc::new(OutboundConnector::with_static_route(route));
        let factory = TransportFactory::with_connector(connector);

        let mut client = TcpConnection::connect_with_factory(&factory, &remote).unwrap();
        let frame = crate::packet::Message::build(0, "test", serde_json::json!({})).unwrap();
        client.send(&frame).unwrap();
        assert!(
            message_receiver
                .recv_timeout(Duration::from_secs(3))
                .unwrap()
                > 0
        );
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn tcp_reverse_tunnel_uses_http_proxy() {
        let target_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let local_port = target_listener.local_addr().unwrap().port();
        let mut remote_server = TcpServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let remote = format!("127.0.0.1:{}", remote_server.local_addr().unwrap().port());
        let (proxy, accepted) = spawn_http_proxy();
        let route = route_with_http(&remote, proxy);
        let connector = Arc::new(OutboundConnector::with_static_route(route));
        let factory = TransportFactory::with_connector(connector);

        let mut client = TcpConnection::tunnel_with_factory(&factory, &remote, local_port).unwrap();
        let (mut server_side, _) = TcpConnection::tunnel_server(target_listener, 10).unwrap();

        client.send(&[5, 6, 7, 8]).unwrap();
        assert_eq!(server_side.recv().unwrap(), vec![5, 6, 7, 8]);
        server_side.send(&[9, 10, 11, 12]).unwrap();
        assert_eq!(client.recv().unwrap(), vec![9, 10, 11, 12]);
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        remote_server.close();
    }

    #[test]
    fn tcp_fixture_confirms_http_proxy_route_is_selected() {
        let (proxy, _accepted) = spawn_http_proxy();
        let route = route_with_http("server.example:443", proxy);
        assert_eq!(
            route.source(),
            Some(crate::protocol::proxy::ProxySource::Environment)
        );
        assert_eq!(route.candidates()[0].kind(), ProxyKind::Http);
    }
}
