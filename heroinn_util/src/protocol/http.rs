//! WebSocket transport (the `websocket` crate), used both for plain connections
//! and for reverse tunnels.

use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use websocket::OwnedMessage;
use websocket::stream::sync::ReadWritePair;
use websocket::sync::{Reader, Writer};

use crate::HeroinnProtocol;
use crate::error::{Error, Result};
use crate::packet::Message;
use crate::protocol::outbound::{
    OutboundConnector, OutboundReader, OutboundWriter, ProxyContext, TransportFactory,
};
use crate::protocol::proxy::{Destination, ProxyRoute};
use crate::protocol::{MessageHandler, TUNNEL_FLAG};

use super::tcp::TcpConnection;

/// Accepting side of a WebSocket listener.
pub struct WSServer {
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    connections: Arc<Mutex<HashMap<SocketAddr, Writer<TcpStream>>>>,
}

/// One outbound WebSocket connection.
pub struct WSConnection {
    reader: Option<Arc<Mutex<Reader<OutboundReader>>>>,
    writer: Option<Arc<Mutex<Writer<OutboundWriter>>>>,
    close_reader: OutboundReader,
    local_addr: SocketAddr,
    proxy_context: Option<ProxyContext>,
    closed: Arc<AtomicBool>,
}

impl WSServer {
    /// Bind `address` and pass every decoded message to `on_message`.
    pub fn new(address: &str, on_message: MessageHandler) -> Result<Self> {
        let mut server = websocket::sync::Server::bind(address)?;
        server.set_nonblocking(true).unwrap();

        let connections: Arc<Mutex<HashMap<SocketAddr, Writer<TcpStream>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let closed = Arc::new(AtomicBool::new(false));

        let local_addr = server.local_addr().unwrap();

        let connections_1 = connections.clone();
        let closed_1 = closed.clone();
        std::thread::Builder::new()
            .name(format!("ws listener : {}", local_addr))
            .spawn(move || {
                loop {
                    let client = match server.accept() {
                        Ok(p) => match p.accept() {
                            Ok(c) => c,
                            Err(e) => {
                                log::warn!("ws handshake failed : {:?}", e);
                                continue;
                            }
                        },
                        Err(_) => {
                            if closed_1.load(Ordering::Relaxed) {
                                break;
                            }

                            std::thread::sleep(std::time::Duration::from_millis(200));
                            continue;
                        }
                    };

                    let connections_2 = connections_1.clone();
                    let on_message = on_message.clone();
                    std::thread::Builder::new()
                        .name(format!("ws connection : {}", local_addr))
                        .spawn(move || {
                            handle_connection(client, &on_message, &connections_2);
                            log::info!("ws client worker finished");
                        })
                        .unwrap();
                }

                let mut conns = connections_1.lock().unwrap();
                for i in conns.values_mut() {
                    let _ = i.shutdown_all();
                }
                conns.clear();

                log::info!("ws main worker finished");
            })
            .unwrap();

        Ok(Self {
            local_addr,
            closed,
            connections,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn sendto(&mut self, peer_addr: &SocketAddr, buf: &[u8]) -> Result<()> {
        let mut connections = self.connections.lock().unwrap();
        let writer = connections
            .get_mut(peer_addr)
            .ok_or_else(|| Error::NotFound(format!("ws client {}", peer_addr)))?;

        writer
            .send_message(&OwnedMessage::Binary(buf.to_vec()))
            .map_err(|e| Error::Other(format!("ws send msg error : {}", e)))
    }

    pub fn contains_addr(&mut self, peer_addr: &SocketAddr) -> bool {
        self.connections.lock().unwrap().contains_key(peer_addr)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}

/// Serve one accepted WebSocket connection until it closes or tunnels.
fn handle_connection(
    client: websocket::sync::Client<TcpStream>,
    on_message: &MessageHandler,
    connections: &Arc<Mutex<HashMap<SocketAddr, Writer<TcpStream>>>>,
) {
    client.set_nonblocking(false).unwrap();
    let remote_addr = client.peer_addr().unwrap();

    log::info!("ws accept from : {}", remote_addr);

    let Ok((mut receiver, sender)) = client.split() else {
        log::warn!("failed to split ws connection {}", remote_addr);
        return;
    };

    connections.lock().unwrap().insert(remote_addr, sender);

    for message in receiver.incoming_messages() {
        let message = match message {
            Ok(p) => p,
            Err(e) => {
                log::info!("ws connection incomming msg error : {}", e);
                break;
            }
        };

        match message {
            OwnedMessage::Close(_) => {
                log::info!("ws connection closed : {}", remote_addr);
                break;
            }
            OwnedMessage::Binary(buf) => {
                if buf.len() == 6 && buf[..4] == TUNNEL_FLAG {
                    let sender = match connections.lock().unwrap().remove(&remote_addr) {
                        Some(s) => s,
                        None => break,
                    };

                    let port = u16::from_be_bytes([buf[4], buf[5]]);
                    serve_tunnel(receiver, sender, remote_addr, port);
                    break;
                }

                match Message::new(remote_addr, HeroinnProtocol::HTTP, &buf) {
                    Ok(msg) => on_message(msg),
                    Err(e) => log::warn!("failed to decode ws message from {}: {}", remote_addr, e),
                }
            }
            _ => {}
        }
    }

    connections.lock().unwrap().remove(&remote_addr);
}

/// Pump bytes both ways between the WebSocket peer and a local TCP connection.
fn serve_tunnel(
    mut receiver: Reader<TcpStream>,
    sender: Writer<TcpStream>,
    remote_addr: SocketAddr,
    port: u16,
) {
    let full_addr = format!("127.0.0.1:{}", port);
    let tunnel_client = match TcpConnection::connect_direct(&full_addr) {
        Ok(p) => p,
        Err(e) => {
            log::error!("tunnel connect failed : {}", e);
            return;
        }
    };

    let mut local_to_ws = tunnel_client.clone();
    let mut sender = sender;
    std::thread::Builder::new()
        .name(format!("ws tunnel worker1 : {}", full_addr))
        .spawn(move || {
            loop {
                let buf = match local_to_ws.recv() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("tunnel read failed : {}", e);
                        break;
                    }
                };

                if buf.is_empty() {
                    break;
                }

                if let Err(e) = sender.send_message(&OwnedMessage::Binary(buf)) {
                    log::error!("ws sender error : {}", e);
                    break;
                }
            }
            log::debug!("ws tunnel1 finished!");
        })
        .unwrap();

    let mut ws_to_local = tunnel_client;
    std::thread::Builder::new()
        .name(format!("ws tunnel worker2 : {}", full_addr))
        .spawn(move || {
            loop {
                let buf = match receiver.recv_message() {
                    Ok(OwnedMessage::Binary(p)) => p,
                    Ok(OwnedMessage::Close(_)) => break,
                    Ok(_) => continue,
                    Err(e) => {
                        log::error!("ws receiver error : {}", e);
                        break;
                    }
                };

                if let Err(e) = ws_to_local.send(&buf) {
                    log::error!("ws sender error : {}", e);
                    break;
                }
            }

            log::debug!("ws tunnel2 finished!");
        })
        .unwrap();

    log::info!("ws tunnel established for {}", remote_addr);
}

impl WSConnection {
    /// Dial `address`.
    pub fn connect(address: &str) -> Result<Self> {
        Self::connect_direct(address)
    }

    /// Dial `address` directly, bypassing any proxy discovery.
    pub fn connect_direct(address: &str) -> Result<Self> {
        Self::connect_with_connector(&OutboundConnector::direct(), address)
    }

    /// Resolve this destination's proxy route and dial through it.
    pub fn connect_with_proxy(address: &str) -> Result<Self> {
        Self::connect_with_factory(&TransportFactory::system(), address)
    }

    /// Dial `address` using an already-resolved proxy route.
    pub fn connect_with_route(address: &str, route: &ProxyRoute) -> Result<Self> {
        let connector = OutboundConnector::with_static_route(route.clone());
        Self::connect_with_connector(&connector, address)
    }

    /// Dial `address` with a shared factory.
    pub fn connect_with_factory(factory: &TransportFactory, address: &str) -> Result<Self> {
        Self::connect_with_connector(factory.connector(), address)
    }

    fn connect_with_connector(connector: &OutboundConnector, address: &str) -> Result<Self> {
        let destination = Destination::parse(address)?;
        let stream = connector.open(&destination)?;
        let local_addr = stream.local_addr()?;
        let proxy_context = stream.proxy_context().cloned();
        let close_reader = stream.close_reader();
        let (reader, writer) = stream
            .split()
            .map_err(|e| Error::Other(format!("ws stream split error : {}", e)))?;

        let s = websocket::ClientBuilder::new(&format!("ws://{}", address))
            .map_err(|e| Error::Other(format!("ws url error : {}", e)))?
            .connect_on(ReadWritePair(reader, writer))
            .map_err(|e| Error::Other(format!("ws handshake error : {}", e)))?;
        let (reader, writer) = s
            .split()
            .map_err(|e| Error::Other(format!("ws split error : {}", e)))?;
        Ok(Self::from_parts(
            reader,
            writer,
            close_reader,
            local_addr,
            proxy_context,
        ))
    }

    fn from_parts(
        reader: Reader<OutboundReader>,
        writer: Writer<OutboundWriter>,
        close_reader: OutboundReader,
        local_addr: SocketAddr,
        proxy_context: Option<ProxyContext>,
    ) -> Self {
        Self {
            reader: Some(Arc::new(Mutex::new(reader))),
            writer: Some(Arc::new(Mutex::new(writer))),
            close_reader,
            closed: Arc::new(AtomicBool::new(false)),
            local_addr,
            proxy_context,
        }
    }

    /// Dial `remote_addr` and ask it to tunnel to `server_local_port`.
    pub fn tunnel(remote_addr: &str, server_local_port: u16) -> Result<Self> {
        Self::tunnel_direct(remote_addr, server_local_port)
    }

    /// Dial `remote_addr` directly and ask it to tunnel to `server_local_port`.
    pub fn tunnel_direct(remote_addr: &str, server_local_port: u16) -> Result<Self> {
        log::info!("start tunnel [{}] [{}]", remote_addr, server_local_port);

        let mut ret = Self::connect_direct(remote_addr)?;

        let mut buf = TUNNEL_FLAG.to_vec();
        buf.append(&mut server_local_port.to_be_bytes().to_vec());
        ret.send(&buf)?;

        Ok(ret)
    }

    /// Resolve the tunnel destination's proxy route and dial through it.
    pub fn tunnel_with_proxy(remote_addr: &str, server_local_port: u16) -> Result<Self> {
        Self::tunnel_with_factory(&TransportFactory::system(), remote_addr, server_local_port)
    }

    /// Dial a tunnel through an already-resolved proxy route.
    pub fn tunnel_with_route(
        remote_addr: &str,
        server_local_port: u16,
        route: &ProxyRoute,
    ) -> Result<Self> {
        let connector = OutboundConnector::with_static_route(route.clone());
        Self::tunnel_with_connector(&connector, remote_addr, server_local_port)
    }

    /// Dial a reverse tunnel with a shared factory.
    pub fn tunnel_with_factory(
        factory: &TransportFactory,
        remote_addr: &str,
        server_local_port: u16,
    ) -> Result<Self> {
        Self::tunnel_with_connector(factory.connector(), remote_addr, server_local_port)
    }

    fn tunnel_with_connector(
        connector: &OutboundConnector,
        remote_addr: &str,
        server_local_port: u16,
    ) -> Result<Self> {
        log::info!("start tunnel [{}] [{}]", remote_addr, server_local_port);

        let mut ret = Self::connect_with_connector(connector, remote_addr)?;

        let mut buf = TUNNEL_FLAG.to_vec();
        buf.append(&mut server_local_port.to_be_bytes().to_vec());
        ret.send(&buf)?;

        Ok(ret)
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        let msg = {
            let reader = self.reader()?;
            reader.lock().unwrap().recv_message()
        };

        match msg {
            Ok(OwnedMessage::Binary(buf)) => Ok(buf),
            Ok(OwnedMessage::Close(_)) => {
                self.close();
                Err(Error::Other("ws closed".to_string()))
            }
            Ok(_) => Ok(vec![]),
            Err(e) => Err(Error::Other(format!("ws receive error : {}", e))),
        }
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<()> {
        let writer = self.writer()?;

        writer
            .lock()
            .unwrap()
            .send_message(&OwnedMessage::Binary(buf.to_vec()))
            .map_err(|e| Error::Other(format!("ws send msg error : {}", e)))
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn proxy_context(&self) -> Option<&ProxyContext> {
        self.proxy_context.as_ref()
    }

    pub fn remote_closed(&self) -> Result<bool> {
        self.close_reader.remote_closed().map_err(Error::Io)
    }

    pub fn wait_for_remote_close(&self, timeout: std::time::Duration) -> Result<()> {
        self.close_reader
            .wait_for_remote_close(timeout)
            .map_err(Error::Io)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        self.reader = None;
        self.writer = None;
    }

    fn reader(&self) -> Result<&Arc<Mutex<Reader<OutboundReader>>>> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Other("socket closed".to_string()));
        }

        self.reader
            .as_ref()
            .ok_or_else(|| Error::Other("socket closed".to_string()))
    }

    fn writer(&self) -> Result<&Arc<Mutex<Writer<OutboundWriter>>>> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Other("socket closed".to_string()));
        }

        self.writer
            .as_ref()
            .ok_or_else(|| Error::Other("socket closed".to_string()))
    }
}

impl Clone for WSConnection {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
            writer: self.writer.clone(),
            close_reader: self.close_reader.clone(),
            closed: self.closed.clone(),
            local_addr: self.local_addr,
            proxy_context: self.proxy_context.clone(),
        }
    }
}

impl Drop for WSServer {
    fn drop(&mut self) {
        self.close();
    }
}

impl Drop for WSConnection {
    fn drop(&mut self) {
        self.reader = None;
        self.writer = None;
    }
}

#[test]
fn test_ws_tunnel() {
    let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let server2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let remote_local_port = server2.local_addr().unwrap().port();

    let remote = &format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let mut client1 = WSConnection::tunnel(remote, remote_local_port).unwrap();

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

#[cfg(test)]
mod proxy_transport_tests {
    use super::*;
    use crate::protocol::proxy;
    use crate::protocol::proxy::{ProxyEnvironment, ProxyKind, ProxySource};
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpListener, TcpStream};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    fn route_with_http(target: &str, proxy_address: std::net::SocketAddr) -> ProxyRoute {
        proxy::resolve_target_with(
            target,
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{}", proxy_address)),
                https_proxy: None,
                all_proxy: None,
                no_proxy: None,
            },
            None,
        )
        .unwrap()
    }

    fn route_with_socks(target: &str, proxy_address: std::net::SocketAddr) -> ProxyRoute {
        proxy::resolve_target_with(
            target,
            &ProxyEnvironment {
                http_proxy: None,
                https_proxy: None,
                all_proxy: Some(format!("socks5://{}", proxy_address)),
                no_proxy: None,
            },
            None,
        )
        .unwrap()
    }

    fn relay(mut left: TcpStream, mut right: TcpStream) {
        let mut left_reader = left.try_clone().unwrap();
        let mut right_writer = right.try_clone().unwrap();
        thread::spawn(move || {
            let _ = std::io::copy(&mut left_reader, &mut right_writer);
            let _ = right_writer.shutdown(Shutdown::Both);
        });

        let _ = std::io::copy(&mut right, &mut left);
        let _ = left.shutdown(Shutdown::Both);
    }

    fn spawn_http_proxy() -> (std::net::SocketAddr, Arc<AtomicUsize>) {
        spawn_http_proxy_on(0)
    }

    fn spawn_http_proxy_on(port: u16) -> (std::net::SocketAddr, Arc<AtomicUsize>) {
        let listener = TcpListener::bind(("127.0.0.1", port)).unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let accepted_thread = accepted.clone();

        thread::spawn(move || {
            let (mut client, _) = listener.accept().unwrap();
            accepted_thread.fetch_add(1, Ordering::SeqCst);

            let mut request_bytes = Vec::new();
            let mut byte = [0u8; 1];
            while request_bytes.len() < 16 * 1024 {
                client.read_exact(&mut byte).unwrap();
                request_bytes.push(byte[0]);
                if request_bytes.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&request_bytes);

            let authority = request
                .lines()
                .next()
                .and_then(|line| line.strip_prefix("CONNECT "))
                .and_then(|line| line.split_whitespace().next())
                .unwrap()
                .to_string();
            let upstream = TcpStream::connect(&authority).unwrap();
            client
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .unwrap();
            relay(client, upstream);
        });

        (address, accepted)
    }

    fn spawn_socks_proxy() -> (std::net::SocketAddr, Arc<AtomicUsize>) {
        spawn_socks_proxy_on(0)
    }

    fn spawn_socks_proxy_on(port: u16) -> (std::net::SocketAddr, Arc<AtomicUsize>) {
        let listener = TcpListener::bind(("127.0.0.1", port)).unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let accepted_thread = accepted.clone();

        thread::spawn(move || {
            let (mut client, _) = listener.accept().unwrap();
            accepted_thread.fetch_add(1, Ordering::SeqCst);

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
            relay(client, upstream);
        });

        (address, accepted)
    }

    fn spawn_http_status_proxy(status: &'static str) -> (std::net::SocketAddr, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let accepted_thread = accepted.clone();

        thread::spawn(move || {
            let (mut client, _) = listener.accept().unwrap();
            accepted_thread.fetch_add(1, Ordering::SeqCst);
            let mut request_bytes = Vec::new();
            let mut byte = [0u8; 1];
            while request_bytes.len() < 16 * 1024 {
                client.read_exact(&mut byte).unwrap();
                request_bytes.push(byte[0]);
                if request_bytes.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let response = format!("HTTP/1.1 {status}\r\n\r\n");
            client.write_all(response.as_bytes()).unwrap();
        });

        (address, accepted)
    }

    #[test]
    fn main_websocket_connects_through_http_proxy() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let (proxy, accepted) = spawn_http_proxy();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let route = route_with_http(&remote, proxy);
        assert_eq!(route.source(), Some(ProxySource::Environment));

        let mut client = WSConnection::connect_with_route(&remote, &route).unwrap();
        client.send(&[0, 1, 2, 3]).unwrap();
        thread::sleep(Duration::from_millis(100));
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn main_websocket_connects_through_socks5_proxy() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let (proxy, accepted) = spawn_socks_proxy();
        let remote = format!("localhost:{}", server.local_addr().unwrap().port());
        let route = route_with_socks(&remote, proxy);

        let mut client = WSConnection::connect_with_route(&remote, &route).unwrap();
        client.send(&[0, 1, 2, 3]).unwrap();
        thread::sleep(Duration::from_millis(100));
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn http_proxied_reverse_tunnel_carries_frames() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let tunnel_port = tunnel_listener.local_addr().unwrap().port();
        let (proxy, accepted) = spawn_http_proxy();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let route = route_with_http(&remote, proxy);

        let mut client = WSConnection::tunnel_with_route(&remote, tunnel_port, &route).unwrap();
        let (mut server_side, _) = TcpConnection::tunnel_server(tunnel_listener, 10).unwrap();

        client.send(&[4, 5, 6, 7]).unwrap();
        assert_eq!(server_side.recv().unwrap(), vec![4, 5, 6, 7]);
        server_side.send(&[8, 9, 10, 11]).unwrap();
        assert_eq!(client.recv().unwrap(), vec![8, 9, 10, 11]);
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn socks_proxied_reverse_tunnel_carries_frames() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let tunnel_port = tunnel_listener.local_addr().unwrap().port();
        let (proxy, accepted) = spawn_socks_proxy();
        let remote = format!("localhost:{}", server.local_addr().unwrap().port());
        let route = route_with_socks(&remote, proxy);

        let mut client = WSConnection::tunnel_with_route(&remote, tunnel_port, &route).unwrap();
        let (mut server_side, _) = TcpConnection::tunnel_server(tunnel_listener, 10).unwrap();

        client.send(&[12, 13, 14, 15]).unwrap();
        assert_eq!(server_side.recv().unwrap(), vec![12, 13, 14, 15]);
        server_side.send(&[16, 17, 18, 19]).unwrap();
        assert_eq!(client.recv().unwrap(), vec![16, 17, 18, 19]);
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn https_proxied_reverse_tunnel_delivers_final_frame_before_close() {
        let payload = (0..262_144usize)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let tunnel_port = tunnel_listener.local_addr().unwrap().port();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let accepted = Arc::new(AtomicUsize::new(0));
        let factory =
            super::super::outbound::tests::factory_with_https_proxy(&remote, accepted.clone());

        let mut client = WSConnection::tunnel_with_factory(&factory, &remote, tunnel_port)
            .expect("TLS proxy HTTP tunnel");
        let (mut server_side, _) =
            TcpConnection::tunnel_server(tunnel_listener, 10).expect("tunnel accept");

        let sent_payload = payload.clone();
        let server_thread = std::thread::spawn(move || {
            for chunk in sent_payload.chunks(20 * 1024) {
                server_side.send(chunk).expect("send tunnel frame");
            }
            drop(server_side);
        });

        let mut received = Vec::with_capacity(payload.len());
        while received.len() < payload.len() {
            let frame = client.recv().expect("recv tunnel frame");
            assert!(!frame.is_empty(), "empty tunnel frame before payload end");
            received.extend_from_slice(&frame);
        }
        server_thread.join().unwrap();
        assert_eq!(received, payload);
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    fn assert_https_reverse_tunnel_flushes_final_frame(payload_len: usize) {
        let payload = (0..payload_len)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let tunnel_port = tunnel_listener.local_addr().unwrap().port();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let accepted = Arc::new(AtomicUsize::new(0));
        let factory =
            super::super::outbound::tests::factory_with_https_proxy(&remote, accepted.clone());

        let mut client = WSConnection::tunnel_with_factory(&factory, &remote, tunnel_port)
            .expect("TLS proxy HTTP tunnel");
        let (mut server_side, _) = tunnel_listener.accept().expect("tunnel accept");
        server_side
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();

        let sent_payload = payload.clone();
        let client_thread = std::thread::spawn(move || {
            for chunk in sent_payload.chunks(20 * 1024) {
                client.send(chunk).expect("send tunnel frame");
            }
            // This mirrors the FTP worker: the final send returns, then the
            // session immediately drops the tunnel.
            drop(client);
        });

        let mut received = Vec::with_capacity(payload.len());
        while received.len() < payload.len() {
            let mut size = [0u8; 4];
            server_side.read_exact(&mut size).unwrap_or_else(|error| {
                panic!(
                    "short tunnel at {}/{} bytes while reading frame size: {error}",
                    received.len(),
                    payload.len()
                )
            });
            let size = u32::from_be_bytes(size) as usize;
            let mut frame = vec![0u8; size];
            server_side.read_exact(&mut frame).unwrap_or_else(|error| {
                panic!(
                    "short tunnel at {}/{} bytes while reading {size}-byte frame: {error}",
                    received.len(),
                    payload.len()
                )
            });
            received.extend_from_slice(&frame);
        }
        client_thread.join().unwrap();
        assert_eq!(received, payload);
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn https_proxied_reverse_tunnel_flushes_final_frame_before_drop() {
        for payload_len in [262_144usize, 262_437] {
            assert_https_reverse_tunnel_flushes_final_frame(payload_len);
        }
    }

    #[test]
    fn failed_http_candidate_falls_back_to_socks_from_same_environment() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let (http_proxy, http_accepted) = spawn_http_status_proxy("502 Bad Gateway");
        let (socks_proxy, socks_accepted) = spawn_socks_proxy();
        let remote = format!("localhost:{}", server.local_addr().unwrap().port());
        let route = proxy::resolve_target_with(
            &remote,
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{http_proxy}")),
                https_proxy: None,
                all_proxy: Some(format!("socks5://{socks_proxy}")),
                no_proxy: None,
            },
            None,
        )
        .unwrap();

        let mut client = WSConnection::connect_with_route(&remote, &route).unwrap();
        client.send(&[20, 21, 22, 23]).unwrap();
        assert_eq!(http_accepted.load(Ordering::SeqCst), 1);
        assert_eq!(socks_accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[test]
    fn no_proxy_bypasses_the_proxy_fixture() {
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        proxy_listener.set_nonblocking(true).unwrap();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let route = proxy::resolve_target_with(
            &remote,
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{}", proxy_listener.local_addr().unwrap())),
                https_proxy: None,
                all_proxy: None,
                no_proxy: Some("127.0.0.1".to_string()),
            },
            None,
        )
        .unwrap();
        assert!(route.is_direct());

        let mut client = WSConnection::connect_with_route(&remote, &route).unwrap();
        client.send(&[24, 25, 26, 27]).unwrap();
        assert!(proxy_listener.accept().is_err());
        server.close();
    }

    #[test]
    fn dead_proxy_fails_closed_without_a_direct_connection() {
        let dead_proxy = TcpListener::bind("127.0.0.1:0").unwrap();
        let dead_proxy_address = dead_proxy.local_addr().unwrap();
        drop(dead_proxy);

        let direct_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        direct_listener.set_nonblocking(true).unwrap();
        let remote = format!("127.0.0.1:{}", direct_listener.local_addr().unwrap().port());
        let route = proxy::resolve_target_with(
            &remote,
            &ProxyEnvironment {
                http_proxy: Some(format!("http://{dead_proxy_address}")),
                https_proxy: None,
                all_proxy: None,
                no_proxy: None,
            },
            None,
        )
        .unwrap();

        assert!(WSConnection::connect_with_route(&remote, &route).is_err());
        assert!(
            direct_listener.accept().is_err(),
            "selected proxy failure must never fall back to direct"
        );
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    #[ignore = "requires a configured desktop HTTP proxy and proxy environment variables removed"]
    fn live_desktop_http_proxy_connects_to_websocket() {
        let port = std::env::var("HEROINN_LIVE_PROXY_PORT")
            .unwrap()
            .parse::<u16>()
            .unwrap();
        let (proxy, accepted) = spawn_http_proxy_on(port);
        assert_eq!(proxy.port(), port);

        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
        let route = proxy::resolve_target(&remote).unwrap();
        assert_eq!(route.source(), Some(ProxySource::System));
        assert_eq!(route.candidates()[0].kind(), ProxyKind::Http);
        let mut client = WSConnection::connect_with_route(&remote, &route).unwrap();
        client.send(&[30, 31, 32, 33]).unwrap();
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    #[ignore = "requires a configured desktop SOCKS5 proxy and proxy environment variables removed"]
    fn live_desktop_socks_proxy_connects_to_websocket() {
        let port = std::env::var("HEROINN_LIVE_PROXY_PORT")
            .unwrap()
            .parse::<u16>()
            .unwrap();
        let (proxy, accepted) = spawn_socks_proxy_on(port);
        assert_eq!(proxy.port(), port);

        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let remote = format!("localhost:{}", server.local_addr().unwrap().port());
        let route = proxy::resolve_target(&remote).unwrap();
        assert_eq!(route.source(), Some(ProxySource::System));
        assert_eq!(route.candidates()[0].kind(), ProxyKind::Socks5);
        let mut client = WSConnection::connect_with_route(&remote, &route).unwrap();
        client.send(&[34, 35, 36, 37]).unwrap();
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        server.close();
    }
}
