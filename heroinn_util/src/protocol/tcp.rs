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
use crate::protocol::outbound::{OutboundStream, ProxyContext, RouteReport, TransportFactory};
use crate::protocol::proxy::{Destination, FrozenRoute};
use crate::protocol::{DisconnectHandler, MessageHandler, TUNNEL_FLAG};
use crate::{HeroinnProtocol, cur_timestamp_secs};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConnectionEnd {
    Closed,
    Tunnel,
}

/// Accepting side of a TCP listener.
pub struct TcpServer {
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    connections: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
}

/// One outbound TCP connection.
pub struct TcpConnection {
    s: Option<OutboundStream>,
    proxy_context: Option<ProxyContext>,
    route_report: RouteReport,
    frozen_route: Option<FrozenRoute>,
    closed: Arc<AtomicBool>,
}

impl Clone for TcpConnection {
    fn clone(&self) -> Self {
        Self {
            s: self.s.as_ref().and_then(|stream| stream.try_clone().ok()),
            proxy_context: self.proxy_context.clone(),
            route_report: self.route_report.clone(),
            frozen_route: self.frozen_route.clone(),
            closed: self.closed.clone(),
        }
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
                for stream in listener.incoming() {
                    std::thread::sleep(Duration::from_millis(200));

                    match stream {
                        Ok(s) => accept_connection(s, &on_message, &on_disconnect, &connections_1),
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::WouldBlock {
                                if closed_1.load(Ordering::Relaxed) {
                                    break;
                                }
                            } else {
                                log::warn!("tcp accept failed : {}", e);
                            }
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
            .ok_or_else(|| Error::NotFound(format!("tcp client {}", peer_addr)))?
            .try_clone()?;

        write_frame(&mut conn, buf)
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
    connections: &Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
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
    let mut reader = match s.try_clone() {
        Ok(reader) => reader,
        Err(error) => {
            log::warn!("failed to clone accepted TCP connection: {}", error);
            return;
        }
    };
    connections
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(peer_addr, s);

    let connections = connections.clone();
    let on_message = on_message.clone();
    let on_disconnect = on_disconnect.clone();

    if let Err(error) = std::thread::Builder::new()
        .name(format!("tcp connection : {}", peer_addr))
        .spawn(move || {
            let end = handle_connection(&mut reader, peer_addr, &on_message);
            connections
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&peer_addr);
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

/// Read frames until the peer disconnects or requests a tunnel.
fn handle_connection(
    reader: &mut TcpStream,
    peer_addr: SocketAddr,
    on_message: &MessageHandler,
) -> ConnectionEnd {
    loop {
        let mut size_buf = [0u8; 4];
        if reader.read_exact(&mut size_buf).is_err() {
            return ConnectionEnd::Closed;
        }

        if size_buf == TUNNEL_FLAG {
            serve_tunnel(reader, peer_addr);
            return ConnectionEnd::Tunnel;
        }

        let size = u32::from_be_bytes(size_buf) as usize;

        if size > MAX_PACKET {
            log::error!("packet length error!");
            return ConnectionEnd::Closed;
        }

        let mut buf = vec![0u8; size];
        if reader.read_exact(&mut buf).is_err() {
            return ConnectionEnd::Closed;
        }

        match Message::new(peer_addr, HeroinnProtocol::TCP, &buf) {
            Ok(msg) => on_message(msg),
            Err(e) => log::warn!("failed to decode tcp message from {}: {}", peer_addr, e),
        }
    }
}

/// Pump bytes both ways between the peer and a fresh local connection.
fn serve_tunnel(reader: &mut TcpStream, peer_addr: SocketAddr) {
    let mut port = [0u8; 2];
    if reader.read_exact(&mut port).is_err() {
        return;
    }

    let full_addr = format!("127.0.0.1:{}", u16::from_be_bytes(port));

    let tunnel_client = match TcpStream::connect(&full_addr) {
        Ok(p) => p,
        Err(e) => {
            log::error!("tunnel connect failed : {}", e);
            return;
        }
    };

    let mut to_peer = tunnel_client.try_clone().unwrap();
    let mut from_peer = reader.try_clone().unwrap();
    std::thread::Builder::new()
        .name(format!("tunnel worker1 : {} , {}", peer_addr, full_addr))
        .spawn(move || {
            if let Err(e) = std::io::copy(&mut to_peer, &mut from_peer) {
                log::error!("tunnel1 io copy failed : {}", e);
            }
            log::debug!("tunnel1 finished!");
        })
        .unwrap();

    let mut to_local = tunnel_client;
    let mut from_local = reader.try_clone().unwrap();
    std::thread::Builder::new()
        .name(format!("tunnel worker2 : {} , {}", peer_addr, full_addr))
        .spawn(move || {
            if let Err(e) = std::io::copy(&mut from_local, &mut to_local) {
                log::error!("tunnel2 io copy failed : {}", e);
            }
            log::debug!("tunnel2 finished!");
        })
        .unwrap();
}

impl TcpConnection {
    /// Dial `address` directly, bypassing proxy discovery.
    pub fn connect_direct(address: &str) -> Result<Self> {
        let address: SocketAddr = address
            .parse()
            .map_err(|e| Error::Other(format!("address format error : {}", e)))?;

        Ok(Self {
            s: Some(OutboundStream::from_tcp(TcpStream::connect(address)?)?),
            proxy_context: None,
            route_report: RouteReport::default(),
            frozen_route: None,
            closed: Arc::new(AtomicBool::new(false)),
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
        Ok(Self {
            s: Some(stream),
            proxy_context,
            route_report,
            frozen_route,
            closed: Arc::new(AtomicBool::new(false)),
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

        Ok(Self {
            s: Some(s),
            proxy_context: None,
            route_report: RouteReport::default(),
            frozen_route: None,
            closed: Arc::new(AtomicBool::new(false)),
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
        Ok(Self {
            s: Some(s),
            proxy_context,
            route_report,
            frozen_route,
            closed: Arc::new(AtomicBool::new(false)),
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
        Ok(Self {
            s: Some(s),
            proxy_context,
            route_report,
            frozen_route,
            closed: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Accept one tunnelled connection, waiting up to `timeout_secs`.
    pub fn tunnel_server(server: TcpListener, timeout_secs: u64) -> Result<(Self, SocketAddr)> {
        server.set_nonblocking(true).unwrap();

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

            s.set_nonblocking(false).unwrap();

            return Ok((
                Self {
                    s: Some(OutboundStream::from_tcp(s)?),
                    proxy_context: None,
                    route_report: RouteReport::default(),
                    frozen_route: None,
                    closed: Arc::new(AtomicBool::new(false)),
                },
                addr,
            ));
        }
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        let s = self.stream_mut()?;
        read_frame(s)
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<()> {
        let s = self.stream_mut()?;
        write_frame(s, buf)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        self.s = None;
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.stream()?.local_addr()?)
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
        self.stream()?.shutdown(how).map_err(Error::Io)
    }

    pub fn remote_closed(&self) -> Result<bool> {
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

    let mut connections = HashMap::new();
    connections.insert(address, stream);
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
    let client = TcpStream::connect(address).unwrap();
    drop(client);
    let (peer, protocol) = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("disconnect callback");
    assert_eq!(peer.ip(), address.ip());
    assert_eq!(protocol, HeroinnProtocol::TCP);
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
