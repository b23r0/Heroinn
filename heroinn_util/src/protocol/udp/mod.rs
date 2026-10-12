//! Reliable-UDP transport built on `rust-raknet`.

pub mod wrapper;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use self::wrapper::{RUdpClient, RUdpServer};

use super::tcp::TcpConnection;
use crate::error::{Error, Result};
use crate::packet::Message;
use crate::protocol::{MessageHandler, TUNNEL_FLAG};
use crate::HeroinnProtocol;

/// Byte prefixed to every UDP payload to keep tunnel and data frames apart.
const UDP_PREFIX: u8 = 0xfe;

/// Accepting side of a reliable-UDP listener.
pub struct UDPServer {
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    connections: Arc<Mutex<HashMap<SocketAddr, Arc<RUdpClient>>>>,
}

/// One outbound reliable-UDP connection.
pub struct UDPConnection {
    s: Option<RUdpClient>,
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
}

impl UDPServer {
    /// Bind `address` and pass every decoded message to `on_message`.
    pub fn new(address: &str, on_message: MessageHandler) -> Result<Self> {
        let mut server = RUdpServer::new(address)?;

        let connections: Arc<Mutex<HashMap<SocketAddr, Arc<RUdpClient>>>> =
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

                    let client = Arc::new(client);
                    let connections_2 = connections_1.clone();
                    let on_message = on_message.clone();
                    let remote_addr = client.peer_addr().unwrap();

                    connections_1
                        .lock()
                        .unwrap()
                        .insert(remote_addr, client.clone());

                    std::thread::Builder::new()
                        .name(format!("udp connection : {}", local_addr))
                        .spawn(move || {
                            handle_connection(client, remote_addr, &on_message);
                            connections_2.lock().unwrap().remove(&remote_addr);
                            log::info!("udp client worker finished");
                        })
                        .unwrap();
                }

                let mut conns = connections_1.lock().unwrap();
                for i in conns.values_mut() {
                    let _ = i.close();
                }
                conns.clear();

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
        let client = self
            .connections
            .lock()
            .unwrap()
            .get(peer_addr)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("udp client {}", peer_addr)))?;

        client
            .send(prefixed(buf))
            .map_err(|e| Error::Other(format!("udp send msg error : {}", e)))
    }

    pub fn contains_addr(&mut self, peer_addr: &SocketAddr) -> bool {
        self.connections.lock().unwrap().contains_key(peer_addr)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}

/// Read datagrams until the peer disconnects or requests a tunnel.
fn handle_connection(client: Arc<RUdpClient>, remote_addr: SocketAddr, on_message: &MessageHandler) {
    loop {
        let buf = match client.recv() {
            Ok(p) => p,
            Err(e) => {
                log::error!("udp client recv error : {}", e);
                return;
            }
        };

        if buf.len() == TUNNEL_FLAG.len() + 3 && buf[1..5] == TUNNEL_FLAG {
            let port = u16::from_be_bytes([buf[5], buf[6]]);
            serve_tunnel(client, port);
            return;
        }

        match Message::new(remote_addr, HeroinnProtocol::UDP, &buf[1..]) {
            Ok(msg) => on_message(msg),
            Err(e) => log::warn!("failed to decode udp message from {}: {}", remote_addr, e),
        }
    }
}

/// Pump bytes both ways between the UDP peer and a local TCP connection.
fn serve_tunnel(client: Arc<RUdpClient>, port: u16) {
    let full_addr = format!("127.0.0.1:{}", port);

    let tunnel_client = match TcpConnection::connect(&full_addr) {
        Ok(p) => p,
        Err(e) => {
            log::error!("tunnel connect failed : {}", e);
            return;
        }
    };

    let mut local_to_udp = tunnel_client.clone();
    let client_1 = client.clone();
    std::thread::Builder::new()
        .name(format!("udp tunnel worker1 : {}", full_addr))
        .spawn(move || {
            loop {
                let buf = match local_to_udp.recv() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("tunnel read failed : {}", e);
                        break;
                    }
                };

                if buf.is_empty() {
                    break;
                }

                if let Err(e) = client_1.send(prefixed(&buf)) {
                    log::error!("udp sender error : {}", e);
                    break;
                }
            }
            log::debug!("udp tunnel1 finished!");
        })
        .unwrap();

    let mut udp_to_local = tunnel_client;
    std::thread::Builder::new()
        .name(format!("udp tunnel worker2 : {}", full_addr))
        .spawn(move || {
            loop {
                let buf = match client.recv() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("udp receiver error : {}", e);
                        break;
                    }
                };

                if let Err(e) = udp_to_local.send(&buf[1..]) {
                    log::error!("udp sender error : {}", e);
                    break;
                }
            }

            log::debug!("udp tunnel2 finished!");
        })
        .unwrap();
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
        let s = RUdpClient::new(address.to_string())?;
        let local_addr = s.local_addr().unwrap();

        Ok(Self {
            s: Some(s),
            closed: Arc::new(AtomicBool::new(false)),
            local_addr,
        })
    }

    /// Dial `remote_addr` and ask it to tunnel to `server_local_port`.
    pub fn tunnel(remote_addr: &str, server_local_port: u16) -> Result<Self> {
        log::info!("start tunnel [{}] [{}]", remote_addr, server_local_port);

        let mut ret = Self::connect(remote_addr)?;

        let mut buf = TUNNEL_FLAG.to_vec();
        buf.append(&mut server_local_port.to_be_bytes().to_vec());
        ret.send(&buf)?;

        Ok(ret)
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        let s = self.client()?;

        let msg = s
            .recv()
            .map_err(|e| Error::Other(format!("udp receive error : {}", e)))?;

        Ok(msg[1..].to_vec())
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<()> {
        let s = self.client()?;
        s.send(prefixed(buf))?;
        Ok(())
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        self.s = None;
    }

    fn client(&self) -> Result<&RUdpClient> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Other("socket closed".to_string()));
        }

        self.s
            .as_ref()
            .ok_or_else(|| Error::Other("socket closed".to_string()))
    }
}

impl Clone for UDPConnection {
    fn clone(&self) -> Self {
        Self {
            s: self.s.clone(),
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
