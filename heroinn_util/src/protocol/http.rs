//! WebSocket transport (the `websocket` crate), used both for plain connections
//! and for reverse tunnels.

use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use websocket::sync::{Reader, Writer};
use websocket::OwnedMessage;

use crate::error::{Error, Result};
use crate::packet::Message;
use crate::protocol::{MessageHandler, TUNNEL_FLAG};
use crate::HeroinnProtocol;

use super::tcp::TcpConnection;

/// Accepting side of a WebSocket listener.
pub struct WSServer {
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    connections: Arc<Mutex<HashMap<SocketAddr, Writer<TcpStream>>>>,
}

/// One outbound WebSocket connection.
pub struct WSConnection {
    reader: Option<Arc<Mutex<Reader<TcpStream>>>>,
    writer: Option<Arc<Mutex<Writer<TcpStream>>>>,
    local_addr: SocketAddr,
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
    let tunnel_client = match TcpConnection::connect(&full_addr) {
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
        let s = websocket::ClientBuilder::new(&format!("ws://{}", address))
            .map_err(|e| Error::Other(format!("ws url error : {}", e)))?
            .connect_insecure()
            .map_err(|e| Error::Other(format!("ws connect error : {}", e)))?;

        let local_addr = s.local_addr().unwrap();
        let (reader, writer) = s
            .split()
            .map_err(|e| Error::Other(format!("ws split error : {}", e)))?;

        Ok(Self {
            reader: Some(Arc::new(Mutex::new(reader))),
            writer: Some(Arc::new(Mutex::new(writer))),
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

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        self.reader = None;
        self.writer = None;
    }

    fn reader(&self) -> Result<&Arc<Mutex<Reader<TcpStream>>>> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Other("socket closed".to_string()));
        }

        self.reader
            .as_ref()
            .ok_or_else(|| Error::Other("socket closed".to_string()))
    }

    fn writer(&self) -> Result<&Arc<Mutex<Writer<TcpStream>>>> {
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
            closed: self.closed.clone(),
            local_addr: self.local_addr,
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
