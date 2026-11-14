//! Reliable-UDP transport built on `rust-raknet`.

pub mod wrapper;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use self::wrapper::{RUdpClient, RUdpServer};

use super::noise::{
    NOISE_HANDSHAKE_TIMEOUT, NoiseIntro, NoiseKind, NoiseSession, NoiseTransport, NoiseWire,
    UDP_RECORD_MAX, UDP_RECORD_PAYLOAD, handshake_initiator, handshake_responder,
};
use super::tcp::TcpConnection;
use crate::HeroinnProtocol;
use crate::error::{Error, Result};
use crate::packet::Message;
use crate::protocol::{DisconnectHandler, MessageHandler, TUNNEL_FLAG};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConnectionEnd {
    Closed,
    Tunnel,
}

/// Byte prefixed to every UDP payload to keep tunnel and data frames apart.
const UDP_PREFIX: u8 = 0xfe;

/// Accepting side of a reliable-UDP listener.
pub struct UDPServer {
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    connections: Arc<Mutex<HashMap<SocketAddr, UdpPeer>>>,
}

/// One outbound reliable-UDP connection.
pub struct UDPConnection {
    s: Option<RUdpClient>,
    noise: Option<Arc<NoiseSession>>,
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
}

#[derive(Clone)]
struct UdpPeer {
    client: RUdpClient,
    noise: Arc<NoiseSession>,
}

struct UdpNoiseWire {
    client: RUdpClient,
}

impl NoiseWire for UdpNoiseWire {
    fn send_plain(&self, payload: &[u8]) -> Result<()> {
        self.client.send(prefixed(payload))
    }

    fn recv_plain(&self) -> Result<Vec<u8>> {
        recv_datagram(&self.client)
    }

    fn recv_plain_timeout(&self, timeout: Duration) -> Result<Vec<u8>> {
        recv_datagram_timeout(&self.client, timeout)
    }

    fn flush_plain(&self) -> Result<()> {
        Ok(())
    }
}

impl UdpPeer {
    fn send(&self, payload: &[u8]) -> Result<()> {
        self.noise.send(payload)
    }

    fn recv(&self) -> Result<Vec<u8>> {
        self.noise.recv()
    }

    fn finish(&self) -> Result<()> {
        self.noise.finish()
    }

    fn close(&self) {
        self.noise.close();
        let _ = self.client.close();
    }
}

impl UDPServer {
    /// Bind `address` and pass every decoded message to `on_message`.
    pub fn new(address: &str, on_message: MessageHandler) -> Result<Self> {
        Self::new_with_disconnect(address, on_message, Arc::new(|_, _| {}))
    }

    pub fn new_with_disconnect(
        address: &str,
        on_message: MessageHandler,
        on_disconnect: DisconnectHandler,
    ) -> Result<Self> {
        let mut server = RUdpServer::new(address)?;

        let connections: Arc<Mutex<HashMap<SocketAddr, UdpPeer>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let closed = Arc::new(AtomicBool::new(false));

        let local_addr = server.local_addr().unwrap();

        let connections_1 = connections.clone();
        let closed_1 = closed.clone();
        std::thread::Builder::new()
            .name(format!("udp listener : {}", local_addr))
            .spawn(move || {
                loop {
                    let client = match server.accept(500) {
                        Ok(p) => p,
                        Err(e) => {
                            if closed_1.load(Ordering::Relaxed) {
                                break;
                            }

                            if matches!(e, Error::Timeout) {
                                std::thread::sleep(std::time::Duration::from_millis(200));
                                continue;
                            } else {
                                closed_1.store(true, Ordering::Relaxed);
                                break;
                            }
                        }
                    };

                    let connections_2 = connections_1.clone();
                    let on_message = on_message.clone();
                    let on_disconnect = on_disconnect.clone();
                    let remote_addr = client.peer_addr().unwrap();

                    std::thread::Builder::new()
                        .name(format!("udp connection : {}", local_addr))
                        .spawn(move || {
                            let (peer, tunnel_port) =
                                match accept_remote_connection(client, remote_addr) {
                                    Ok(connection) => connection,
                                    Err(error) => {
                                        log::warn!(
                                            "udp encrypted handshake failed for {}: {}",
                                            remote_addr,
                                            error
                                        );
                                        return;
                                    }
                                };
                            connections_2
                                .lock()
                                .unwrap()
                                .insert(remote_addr, peer.clone());
                            let end = match tunnel_port {
                                Some(port) => {
                                    serve_tunnel(peer.clone(), remote_addr, port);
                                    ConnectionEnd::Tunnel
                                }
                                None => handle_connection(&peer, remote_addr, &on_message),
                            };
                            connections_2.lock().unwrap().remove(&remote_addr);
                            peer.close();
                            match end {
                                ConnectionEnd::Closed => {
                                    log::info!("udp client worker finished");
                                    on_disconnect(remote_addr, HeroinnProtocol::UDP);
                                }
                                ConnectionEnd::Tunnel => {
                                    log::info!("udp client worker entered tunnel");
                                }
                            }
                        })
                        .unwrap();
                }

                let peers = connections_1
                    .lock()
                    .unwrap()
                    .drain()
                    .map(|(_, peer)| peer)
                    .collect::<Vec<_>>();
                for peer in peers {
                    peer.close();
                }

                let _ = server.close();
                log::info!("udp main worker finished");
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
        let peer = self
            .connections
            .lock()
            .unwrap()
            .get(peer_addr)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("udp client {}", peer_addr)))?;

        peer.send(buf)
            .map_err(|e| Error::Other(format!("udp send msg error : {}", e)))
    }

    pub fn contains_addr(&mut self, peer_addr: &SocketAddr) -> bool {
        self.connections.lock().unwrap().contains_key(peer_addr)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}

fn recv_datagram_timeout(client: &RUdpClient, timeout: Duration) -> Result<Vec<u8>> {
    let datagram = client.recv_timeout(timeout)?;
    decode_datagram(datagram)
}

fn recv_datagram(client: &RUdpClient) -> Result<Vec<u8>> {
    let datagram = client.recv()?;
    decode_datagram(datagram)
}

fn decode_datagram(datagram: Vec<u8>) -> Result<Vec<u8>> {
    match datagram.split_first() {
        Some((&UDP_PREFIX, payload)) => Ok(payload.to_vec()),
        _ => Err(Error::Noise("invalid UDP noise prefix")),
    }
}

fn accept_remote_connection(
    client: RUdpClient,
    remote_addr: SocketAddr,
) -> Result<(UdpPeer, Option<u16>)> {
    let first = recv_datagram_timeout(&client, NOISE_HANDSHAKE_TIMEOUT)?;
    let (intro, kind, tunnel_port) = if first.len() == 6 && first[..4] == TUNNEL_FLAG {
        let port = u16::from_be_bytes([first[4], first[5]]);
        let intro = recv_datagram_timeout(&client, NOISE_HANDSHAKE_TIMEOUT)?;
        (intro, NoiseKind::Tunnel { port }, Some(port))
    } else {
        (first, NoiseKind::Main, None)
    };

    let wire: Arc<dyn NoiseWire> = Arc::new(UdpNoiseWire {
        client: client.clone(),
    });
    let noise = handshake_responder(
        wire,
        &intro,
        NoiseTransport::Udp,
        kind,
        UDP_RECORD_PAYLOAD,
        UDP_RECORD_MAX,
    )
    .map_err(|error| {
        log::debug!("udp noise responder failed for {remote_addr}: {error}");
        error
    })?;
    Ok((
        UdpPeer {
            client,
            noise: Arc::new(noise),
        },
        tunnel_port,
    ))
}

/// Read encrypted messages until the peer disconnects.
fn handle_connection(
    peer: &UdpPeer,
    remote_addr: SocketAddr,
    on_message: &MessageHandler,
) -> ConnectionEnd {
    loop {
        let buf = match peer.recv() {
            Ok(p) => p,
            Err(e) => {
                log::error!("udp client recv error : {}", e);
                return ConnectionEnd::Closed;
            }
        };

        match Message::new(remote_addr, HeroinnProtocol::UDP, &buf) {
            Ok(msg) => on_message(msg),
            Err(e) => log::warn!("failed to decode udp message from {}: {}", remote_addr, e),
        }
    }
}

/// Pump framed bytes both ways between the encrypted UDP peer and a local
/// loopback sidecar connection.
fn serve_tunnel(peer: UdpPeer, remote_addr: SocketAddr, port: u16) {
    let full_addr = format!("127.0.0.1:{}", port);

    let mut tunnel_client = match TcpConnection::connect_local_plain(&full_addr) {
        Ok(p) => p,
        Err(e) => {
            log::error!("tunnel connect failed : {}", e);
            peer.close();
            return;
        }
    };

    let mut local_reader = tunnel_client.clone();
    let mut local_writer = tunnel_client.clone();
    let peer_reader = peer.clone();
    let peer_writer = peer.clone();
    std::thread::scope(|scope| {
        let first = std::thread::Builder::new()
            .name(format!("udp tunnel worker1 : {}", full_addr))
            .spawn_scoped(scope, move || {
                loop {
                    let payload = match local_reader.recv() {
                        Ok(payload) => payload,
                        Err(error) => {
                            log::debug!("udp tunnel local read finished: {error}");
                            break;
                        }
                    };
                    if let Err(error) = peer_writer.send(&payload) {
                        let _ = local_reader.shutdown(std::net::Shutdown::Both);
                        log::debug!("udp tunnel local-to-peer finished: {error}");
                        break;
                    }
                }
                let _ = peer_writer.finish();
                let _ = local_reader.shutdown(std::net::Shutdown::Read);
            });
        let second = std::thread::Builder::new()
            .name(format!("udp tunnel worker2 : {}", full_addr))
            .spawn_scoped(scope, move || {
                loop {
                    let payload = match peer_reader.recv() {
                        Ok(payload) => payload,
                        Err(error) => {
                            log::debug!("udp tunnel peer read finished: {error}");
                            break;
                        }
                    };
                    if let Err(error) = local_writer.send(&payload) {
                        log::debug!("udp tunnel peer-to-local finished: {error}");
                        break;
                    }
                }
                let how = if peer_reader.noise.remote_finished() {
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
                log::warn!("udp tunnel worker start failed: {error}");
                let _ = tunnel_client.shutdown(std::net::Shutdown::Both);
                peer.close();
                if let Ok(handle) = second {
                    let _ = handle.join();
                }
            }
        }
    });
    tunnel_client.close();
    log::debug!("udp tunnel finished for {remote_addr}");
}

/// Prefix `buf` with [`UDP_PREFIX`].
fn prefixed(buf: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(buf.len() + 1);
    data.push(UDP_PREFIX);
    data.extend_from_slice(buf);
    data
}

impl UDPConnection {
    /// Dial `address`.
    pub fn connect(address: &str) -> Result<Self> {
        let (s, local_addr, noise) = Self::connect_raw(address, NoiseKind::Main)?;

        Ok(Self {
            s: Some(s),
            noise: Some(noise),
            closed: Arc::new(AtomicBool::new(false)),
            local_addr,
        })
    }

    /// Dial `remote_addr` and ask it to tunnel to `server_local_port`.
    pub fn tunnel(remote_addr: &str, server_local_port: u16) -> Result<Self> {
        log::info!("start tunnel [{}] [{}]", remote_addr, server_local_port);

        let (client, local_addr) = Self::dial(remote_addr)?;
        let mut buf = TUNNEL_FLAG.to_vec();
        buf.append(&mut server_local_port.to_be_bytes().to_vec());
        client.send(prefixed(&buf))?;
        let noise = Self::start_noise(
            &client,
            NoiseKind::Tunnel {
                port: server_local_port,
            },
        )?;

        Ok(Self {
            s: Some(client),
            noise: Some(noise),
            closed: Arc::new(AtomicBool::new(false)),
            local_addr,
        })
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        self.noise()?
            .recv()
            .map_err(|e| Error::Other(format!("udp receive error : {}", e)))
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<()> {
        self.noise()?
            .send(buf)
            .map_err(|e| Error::Other(format!("udp send error : {}", e)))
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        if let Some(noise) = &self.noise {
            noise.close();
        }
        if let Some(client) = self.s.take() {
            let _ = client.close();
        }
    }

    fn noise(&self) -> Result<&Arc<NoiseSession>> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Other("socket closed".to_string()));
        }

        self.noise
            .as_ref()
            .ok_or_else(|| Error::Other("socket closed".to_string()))
    }

    fn dial(address: &str) -> Result<(RUdpClient, SocketAddr)> {
        let client = RUdpClient::new(address.to_string())?;
        let local_addr = client.local_addr()?;
        Ok((client, local_addr))
    }

    fn connect_raw(
        address: &str,
        kind: NoiseKind,
    ) -> Result<(RUdpClient, SocketAddr, Arc<NoiseSession>)> {
        let (client, local_addr) = Self::dial(address)?;
        let noise = Self::start_noise(&client, kind)?;
        Ok((client, local_addr, noise))
    }

    fn start_noise(client: &RUdpClient, kind: NoiseKind) -> Result<Arc<NoiseSession>> {
        let wire: Arc<dyn NoiseWire> = Arc::new(UdpNoiseWire {
            client: client.clone(),
        });
        let intro = NoiseIntro::new(NoiseTransport::Udp, kind);
        Ok(Arc::new(handshake_initiator(
            wire,
            intro,
            UDP_RECORD_PAYLOAD,
            UDP_RECORD_MAX,
        )?))
    }
}

impl Clone for UDPConnection {
    fn clone(&self) -> Self {
        Self {
            s: self.s.clone(),
            noise: self.noise.clone(),
            closed: self.closed.clone(),
            local_addr: self.local_addr,
        }
    }
}

impl Drop for UDPServer {
    fn drop(&mut self) {
        self.close();
    }
}

impl Drop for UDPConnection {
    fn drop(&mut self) {
        self.s = None;
    }
}

#[test]
fn test_udp_tunnel() {
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let server2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let remote_local_port = server2.local_addr().unwrap().port();

    let remote = &format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let mut client1 = UDPConnection::tunnel(remote, remote_local_port).unwrap();

    let (mut client2, _) = TcpConnection::tunnel_server(server2, 10).unwrap();

    for _ in 0..3 {
        client1.send(&[0, 1, 2, 3]).unwrap();
        let buf = client2.recv().unwrap();
        assert_eq!(buf, [0, 1, 2, 3]);

        client2.send(&[4, 5, 6, 7]).unwrap();
        let buf = client1.recv().unwrap();
        assert_eq!(buf, [4, 5, 6, 7]);
    }

    server.close();
}

#[test]
fn plaintext_legacy_client_is_rejected_without_delivery() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut server = UDPServer::new(
        "127.0.0.1:0",
        Arc::new(move |message| {
            let _ = sender.send(message.id());
        }),
    )
    .unwrap();
    let remote = server.local_addr().unwrap().to_string();
    let client = RUdpClient::new(remote).unwrap();
    let plaintext = Message::build(0x33, "legacy", "plaintext").unwrap();
    client.send(prefixed(&plaintext)).unwrap();
    assert!(
        receiver
            .recv_timeout(std::time::Duration::from_secs(2))
            .is_err(),
        "plaintext must never reach the reliable-UDP application handler"
    );
    let _ = client.close();
    server.close();
}

#[test]
fn role_mismatch_is_rejected_before_udp_application_delivery() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut server = UDPServer::new(
        "127.0.0.1:0",
        Arc::new(move |message| {
            let _ = sender.send(message.id());
        }),
    )
    .unwrap();
    let remote = server.local_addr().unwrap().to_string();
    let client = RUdpClient::new(remote).unwrap();
    let peer_addr = client.peer_addr().unwrap();
    let mut intro = NoiseIntro::new(NoiseTransport::Udp, NoiseKind::Main).encode();
    intro[12..14].copy_from_slice(&2u16.to_be_bytes());
    client.send(prefixed(&intro)).unwrap();

    assert!(
        receiver
            .recv_timeout(std::time::Duration::from_millis(500))
            .is_err(),
        "role mismatch reached the reliable-UDP application handler"
    );
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(
        !server.contains_addr(&peer_addr),
        "role mismatch established a reliable-UDP peer"
    );
    let _ = client.close();
    server.close();
}

#[test]
fn udp_server_reports_transport_disconnect_immediately() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut server = UDPServer::new_with_disconnect(
        "127.0.0.1:0",
        Arc::new(|_| {}),
        Arc::new(move |peer, protocol| {
            let _ = sender.send((peer, protocol));
        }),
    )
    .unwrap();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let client = UDPConnection::connect(&remote).unwrap();
    drop(client);
    let (_peer, protocol) = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("disconnect callback");
    assert_eq!(protocol, HeroinnProtocol::UDP);
    server.close();
}

#[test]
fn udp_connection_survives_more_than_the_handshake_timeout_when_idle() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut server = UDPServer::new(
        "127.0.0.1:0",
        Arc::new(move |message| {
            if message.id() == 0x77 {
                let _ = sender.send(message.clientid().to_string());
            }
        }),
    )
    .unwrap();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let mut client = UDPConnection::connect(&remote).unwrap();

    // The handshake budget must not leak into the established data path. An
    // idle reliable-UDP main connection has no application-level timeout, so a
    // record that arrives after the old 10s handshake deadline still has to be
    // delivered on this same session.
    std::thread::sleep(std::time::Duration::from_millis(10_200));
    let frame = Message::build(0x77, "idle-survivor", "payload").unwrap();
    client.send(&frame).unwrap();

    let clientid = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("an idle UDP session was torn down by the handshake timeout");
    assert_eq!(clientid, "idle-survivor");
    server.close();
}

#[test]
fn udp_tunnel_does_not_report_a_main_transport_disconnect() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut server = UDPServer::new_with_disconnect(
        "127.0.0.1:0",
        Arc::new(|_| {}),
        Arc::new(move |peer, protocol| {
            let _ = sender.send((peer, protocol));
        }),
    )
    .unwrap();
    let tunnel_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let tunnel_port = tunnel_listener.local_addr().unwrap().port();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());

    let mut tunnel = UDPConnection::tunnel(&remote, tunnel_port).unwrap();
    let (mut local, _) = TcpConnection::tunnel_server(tunnel_listener, 5).unwrap();
    tunnel.send(b"still-connected").unwrap();
    assert_eq!(local.recv().unwrap(), b"still-connected");

    assert!(
        receiver
            .recv_timeout(std::time::Duration::from_millis(500))
            .is_err(),
        "starting a reliable-UDP tunnel must not be reported as main-transport disconnect"
    );
    server.close();
}

#[test]
fn udp_transport_shutdown_unblocks_a_pending_receiver() {
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let tunnel_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let tunnel_port = tunnel_listener.local_addr().unwrap().port();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());

    let tunnel = UDPConnection::tunnel(&remote, tunnel_port).unwrap();
    let (local, _) = TcpConnection::tunnel_server(tunnel_listener, 5).unwrap();
    let transport = crate::protocol::Transport::Udp(tunnel);
    let receiver = transport.clone();
    let (ended_sender, ended_receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut receiver = receiver;
        let _ = ended_sender.send(receiver.recv().is_err());
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    transport
        .shutdown(std::net::Shutdown::Both)
        .expect("shutdown UDP transport");
    assert!(
        ended_receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("pending UDP receiver did not unblock"),
        "a closed UDP transport must report an error to its receiver"
    );
    worker.join().expect("receiver worker");
    let _ = local.shutdown(std::net::Shutdown::Both);
    server.close();
}

#[test]
fn udp_tunnel_closes_both_directions_after_target_eof() {
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let tunnel_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let tunnel_port = tunnel_listener.local_addr().unwrap().port();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());

    let mut tunnel = UDPConnection::tunnel(&remote, tunnel_port).unwrap();
    let (mut local, _) = TcpConnection::tunnel_server(tunnel_listener, 5).unwrap();
    local.send(b"final-data").unwrap();
    assert_eq!(tunnel.recv().unwrap(), b"final-data");
    local.shutdown(std::net::Shutdown::Both).unwrap();

    let (closed_sender, closed_receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = closed_sender.send(tunnel.recv().is_err());
    });
    assert!(
        closed_receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("target EOF did not close the reverse UDP tunnel"),
        "target EOF must make the client tunnel receiver terminate"
    );
    worker.join().expect("tunnel receiver worker");
    server.close();
}

#[test]
fn udp_tunnel_delivers_full_payload_before_teardown() {
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let tunnel_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let tunnel_port = tunnel_listener.local_addr().unwrap().port();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let mut tunnel = UDPConnection::tunnel(&remote, tunnel_port).unwrap();
    let (mut local, _) = TcpConnection::tunnel_server(tunnel_listener, 20).unwrap();
    let payload = (0..65536)
        .map(|index| (index % 251) as u8)
        .collect::<Vec<_>>();

    local.send(&payload).unwrap();
    local.shutdown(std::net::Shutdown::Both).unwrap();

    let (sender, receiver) = std::sync::mpsc::channel();
    let receiver_thread = std::thread::spawn(move || {
        let mut bytes = Vec::with_capacity(65536);
        loop {
            match tunnel.recv() {
                Ok(data) => {
                    bytes.extend_from_slice(&data);
                    if bytes.len() >= 65536 {
                        break;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(error.to_string()));
                    return;
                }
            }
        }
        let _ = sender.send(Ok(bytes));
    });

    let result = receiver
        .recv_timeout(std::time::Duration::from_secs(20))
        .expect("reverse UDP tunnel did not finish");
    assert!(result.is_ok(), "reverse UDP tunnel failed: {result:?}");
    assert_eq!(result.unwrap(), payload);
    receiver_thread.join().expect("receiver thread");
    server.close();
}

#[cfg(test)]
struct ExchangeGate {
    active: Mutex<usize>,
    finished: std::sync::Condvar,
    limit: usize,
}

#[cfg(test)]
impl ExchangeGate {
    fn new(limit: usize) -> Self {
        Self {
            active: Mutex::new(0),
            finished: std::sync::Condvar::new(),
            limit,
        }
    }

    fn enter(&self) -> ExchangePermit<'_> {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        while *active >= self.limit {
            active = self
                .finished
                .wait(active)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        *active += 1;
        ExchangePermit { gate: self }
    }
}

#[cfg(test)]
struct ExchangePermit<'a> {
    gate: &'a ExchangeGate,
}

#[cfg(test)]
impl Drop for ExchangePermit<'_> {
    fn drop(&mut self) {
        let mut active = self
            .gate
            .active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *active -= 1;
        self.gate.finished.notify_one();
    }
}

#[cfg(test)]
#[derive(Clone, Copy)]
struct LossSchedule {
    loss_percent: usize,
    first_drop: usize,
}

#[cfg(test)]
impl LossSchedule {
    fn new(loss_percent: usize) -> Self {
        let first_drop = match loss_percent {
            0 => 0,
            _ => 1,
        };
        Self {
            loss_percent,
            first_drop,
        }
    }

    fn should_drop(&self, sequence: usize) -> bool {
        match self.loss_percent {
            0 => false,
            1 => {
                sequence == self.first_drop
                    || (sequence > self.first_drop
                        && (sequence - self.first_drop).is_multiple_of(100))
            }
            _ => {
                sequence == self.first_drop
                    || (sequence > self.first_drop
                        && (sequence - self.first_drop).is_multiple_of(20))
            }
        }
    }
}

#[cfg(test)]
struct LossyUdpRelay {
    front_addr: SocketAddr,
    stop: Arc<AtomicBool>,
    loss_enabled: Arc<AtomicBool>,
    counters: Arc<RelayCounters>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

#[cfg(test)]
struct RelayCounters {
    sequence: std::sync::atomic::AtomicUsize,
    dropped: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl LossyUdpRelay {
    fn start(server: SocketAddr, schedule: LossSchedule) -> std::io::Result<Self> {
        let front = std::net::UdpSocket::bind("127.0.0.1:0")?;
        let back = std::net::UdpSocket::bind("127.0.0.1:0")?;
        front.set_read_timeout(Some(Duration::from_millis(20)))?;
        back.set_read_timeout(Some(Duration::from_millis(20)))?;
        let front_addr = front.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let loss_enabled = Arc::new(AtomicBool::new(false));
        let last_client = Arc::new(Mutex::new(None::<SocketAddr>));
        let counters = Arc::new(RelayCounters {
            sequence: std::sync::atomic::AtomicUsize::new(0),
            dropped: std::sync::atomic::AtomicUsize::new(0),
        });
        let front = Arc::new(front);
        let back = Arc::new(back);

        let forward_stop = stop.clone();
        let forward_loss_enabled = loss_enabled.clone();
        let forward_last = last_client.clone();
        let forward_counters = counters.clone();
        let forward_front = front.clone();
        let forward_back = back.clone();
        let forward = std::thread::spawn(move || {
            let mut buffer = [0u8; 65_535];
            while !forward_stop.load(Ordering::Relaxed) {
                let (length, peer) = match forward_front.recv_from(&mut buffer) {
                    Ok(value) => value,
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) =>
                    {
                        continue;
                    }
                    Err(_) => break,
                };
                *forward_last
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(peer);
                if forward_loss_enabled.load(Ordering::Relaxed) {
                    let sequence = forward_counters.sequence.fetch_add(1, Ordering::Relaxed) + 1;
                    if schedule.should_drop(sequence) {
                        forward_counters.dropped.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                }
                let _ = forward_back.send_to(&buffer[..length], server);
            }
        });

        let reverse_stop = stop.clone();
        let reverse_loss_enabled = loss_enabled.clone();
        let reverse_last = last_client;
        let reverse_counters = counters.clone();
        let reverse_front = front;
        let reverse_back = back;
        let reverse = std::thread::spawn(move || {
            let mut buffer = [0u8; 65_535];
            while !reverse_stop.load(Ordering::Relaxed) {
                let (length, _) = match reverse_back.recv_from(&mut buffer) {
                    Ok(value) => value,
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) =>
                    {
                        continue;
                    }
                    Err(_) => break,
                };
                let peer = *reverse_last
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let Some(peer) = peer else {
                    continue;
                };
                if reverse_loss_enabled.load(Ordering::Relaxed) {
                    let sequence = reverse_counters.sequence.fetch_add(1, Ordering::Relaxed) + 1;
                    if schedule.should_drop(sequence) {
                        reverse_counters.dropped.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                }
                let _ = reverse_front.send_to(&buffer[..length], peer);
            }
        });

        Ok(Self {
            front_addr,
            stop,
            loss_enabled,
            counters,
            workers: vec![forward, reverse],
        })
    }

    fn enable_loss(&self) {
        self.loss_enabled.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
impl Drop for LossyUdpRelay {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
fn run_independent_udp_tunnel_peers(count: usize) {
    run_independent_udp_tunnel_peers_with_loss(count, 0);
}

#[cfg(test)]
fn run_independent_udp_tunnel_peers_with_loss(count: usize, loss_percent: usize) {
    assert!(matches!(loss_percent, 0 | 1 | 5));
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let server_addr = server.local_addr().unwrap();
    let schedule = LossSchedule::new(loss_percent);
    let mut relays = Vec::with_capacity(count);
    for _ in 0..count {
        relays.push(LossyUdpRelay::start(server_addr, schedule).unwrap());
    }
    // Keep every tunnel alive until all streams have exchanged data. The
    // lifecycle tests cover teardown independently; this test isolates the
    // maximum-concurrency data path from RakNet shutdown races.
    let exchange_gate = Arc::new(ExchangeGate::new(count.max(1)));
    let setup_gate = Arc::new(ExchangeGate::new(count.clamp(1, 8)));
    let start = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
    let release = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
    let progress = Arc::new(Mutex::new(std::collections::BTreeMap::new()));
    let (setup_sender, setup_receiver) = std::sync::mpsc::channel();
    let (result_sender, result_receiver) = std::sync::mpsc::channel();
    let mut workers = Vec::with_capacity(count);

    for (index, relay) in relays.iter().enumerate() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let local_port = listener.local_addr().unwrap().port();
        let remote = format!("127.0.0.1:{}", relay.front_addr.port());
        let exchange_gate = exchange_gate.clone();
        let setup_gate = setup_gate.clone();
        let start = start.clone();
        let release = release.clone();
        let progress = progress.clone();
        let setup_sender = setup_sender.clone();
        let result_sender = result_sender.clone();
        workers.push(std::thread::spawn(move || {
            let setup = (|| {
                let _setup_permit = setup_gate.enter();
                let tunnel = UDPConnection::tunnel(&remote, local_port)
                    .map_err(|error| format!("peer {index} tunnel: {error}"))?;
                let (local, _) = TcpConnection::tunnel_server(listener, 20)
                    .map_err(|error| format!("peer {index} accept: {error}"))?;
                Ok::<_, String>((tunnel, local))
            })();
            set_test_progress(&progress, index, "setup-complete");
            let _ = setup_sender.send(setup.as_ref().map(|_| ()).map_err(|error| error.clone()));
            wait_for_flag(&start);

            let mut setup = setup;
            let result = match setup.as_mut() {
                Err(error) => Err(error.clone()),
                Ok((tunnel, local)) => (|| {
                    let _exchange_permit = exchange_gate.enter();
                    set_test_progress(&progress, index, "exchange-start");
                    let to_target = format!("peer-{index:04}-to-target").into_bytes();
                    set_test_progress(&progress, index, "tunnel-send-start");
                    tunnel
                        .send(&to_target)
                        .map_err(|error| format!("peer {index} send: {error}"))?;
                    set_test_progress(&progress, index, "tunnel-send-done");
                    let received = local
                        .recv()
                        .map_err(|error| format!("peer {index} local recv: {error}"))?;
                    set_test_progress(&progress, index, "local-recv-done");
                    if received != to_target {
                        return Err(format!("peer {index} received another stream's payload"));
                    }

                    let to_client = format!("peer-{index:04}-to-client").into_bytes();
                    set_test_progress(&progress, index, "local-send-start");
                    local
                        .send(&to_client)
                        .map_err(|error| format!("peer {index} local send: {error}"))?;
                    set_test_progress(&progress, index, "local-send-done");
                    let received = tunnel
                        .recv()
                        .map_err(|error| format!("peer {index} tunnel recv: {error}"))?;
                    set_test_progress(&progress, index, "tunnel-recv-done");
                    if received != to_client {
                        return Err(format!("peer {index} received another stream's reply"));
                    }
                    Ok(())
                })(),
            };
            let _ = result_sender.send(result);
            set_test_progress(&progress, index, "done");
            wait_for_flag(&release);
            drop(setup);
        }));
    }

    drop(setup_sender);
    let mut failures = Vec::new();
    for _ in 0..count {
        match setup_receiver
            .recv_timeout(std::time::Duration::from_secs(60))
            .expect("parallel UDP peer setup timed out")
        {
            Ok(()) => {}
            Err(error) => failures.push(error),
        }
    }
    if loss_percent > 0 {
        for relay in &relays {
            relay.enable_loss();
        }
    }
    signal_flag(&start);

    for _ in 0..count {
        let result = match result_receiver.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(result) => result,
            Err(_) => {
                let progress = progress
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .clone();
                panic!("parallel UDP peer exchange timed out: {progress:#?}");
            }
        };
        match result {
            Ok(()) => {}
            Err(error) => failures.push(error),
        }
    }
    signal_flag(&release);

    drop(result_sender);
    for worker in workers {
        if worker.join().is_err() {
            failures.push("UDP peer worker panicked".to_string());
        }
    }
    server.close();
    let dropped: usize = relays
        .iter()
        .map(|relay| relay.counters.dropped.load(Ordering::Relaxed))
        .sum();
    if loss_percent > 0 {
        assert!(
            dropped > 0,
            "loss relay dropped no packets at {loss_percent}%"
        );
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[cfg(test)]
fn wait_for_flag(flag: &(Mutex<bool>, std::sync::Condvar)) {
    let mut ready = flag
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    while !*ready {
        ready = flag
            .1
            .wait(ready)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    }
}

#[cfg(test)]
fn signal_flag(flag: &(Mutex<bool>, std::sync::Condvar)) {
    *flag
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
    flag.1.notify_all();
}

#[cfg(test)]
fn set_test_progress(
    progress: &Mutex<std::collections::BTreeMap<usize, &'static str>>,
    index: usize,
    stage: &'static str,
) {
    progress
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(index, stage);
}

#[test]
fn two_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(2);
}

#[test]
fn four_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(4);
}

#[test]
fn closing_one_udp_tunnel_does_not_disrupt_other_streams() {
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let mut peers = Vec::new();

    for _ in 0..4 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let local_port = listener.local_addr().unwrap().port();
        let tunnel = UDPConnection::tunnel(&remote, local_port).unwrap();
        let (local, _) = TcpConnection::tunnel_server(listener, 20).unwrap();
        peers.push((tunnel, local));
    }

    let (closing, _) = peers.remove(0);
    drop(closing);
    std::thread::sleep(std::time::Duration::from_millis(300));

    for (index, (tunnel, local)) in peers.iter_mut().enumerate() {
        let downlink = format!("survivor-{index}-downlink").into_bytes();
        tunnel.send(&downlink).unwrap();
        assert_eq!(local.recv().unwrap(), downlink);

        let uplink = format!("survivor-{index}-uplink").into_bytes();
        local.send(&uplink).unwrap();
        assert_eq!(tunnel.recv().unwrap(), uplink);
    }

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let local_port = listener.local_addr().unwrap().port();
    let mut replacement = UDPConnection::tunnel(&remote, local_port).unwrap();
    let (mut replacement_local, _) = TcpConnection::tunnel_server(listener, 20).unwrap();
    replacement.send(b"replacement").unwrap();
    assert_eq!(replacement_local.recv().unwrap(), b"replacement");

    server.close();
}

#[test]
fn eight_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(8);
}

#[test]
fn sixteen_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(16);
}

#[test]
fn thirty_two_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(32);
}

#[test]
fn sixty_four_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(64);
}

#[test]
fn udp_tunnel_peers_2_8_64_survive_one_and_five_percent_loss() {
    for loss_percent in [1, 5] {
        for count in [2, 8, 64] {
            run_independent_udp_tunnel_peers_with_loss(count, loss_percent);
        }
    }
}
