use super::{Client, Server};
use crate::{
    protocol::{tcp::TcpConnection, TUNNEL_FLAG},
    HeroinnProtocol,
};
use std::{
    collections::HashMap,
    net::{SocketAddr, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use websocket::{
    sync::{Reader, Writer},
    OwnedMessage,
};

pub struct WSServer {
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    connections: Arc<Mutex<HashMap<SocketAddr, Writer<TcpStream>>>>,
}

pub struct WSConnection {
    reader: Option<Arc<Mutex<Reader<TcpStream>>>>,
    writer: Option<Arc<Mutex<Writer<TcpStream>>>>,
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
}

impl Server for WSServer {
    fn new<
        CBCB: 'static + Fn(crate::packet::Message) + Send + Copy,
        CB: 'static + Fn(crate::HeroinnProtocol, Vec<u8>, SocketAddr, CBCB) + Send,
    >(
        address: &str,
        cb_data: CB,
        cbcb: CBCB,
    ) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let mut server = websocket::sync::Server::bind(address)?;
        server.set_nonblocking(true).unwrap();

        let connections: Arc<Mutex<HashMap<SocketAddr, Writer<TcpStream>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let closed = Arc::new(AtomicBool::new(false));

        let local_addr = server.local_addr().unwrap();

        let connections_1 = connections.clone();
        let closed_1 = closed.clone();
        let cb_data = Arc::new(Mutex::new(cb_data));
        std::thread::Builder::new()
            .name(format!("ws main worker : {}", local_addr.clone()))
            .spawn(move || {
                loop {
                    let client = match server.accept() {
                        Ok(p) => p.accept().unwrap(),
                        Err(_) => {
                            if closed_1.load(std::sync::atomic::Ordering::Relaxed) {
                                break;
                            }

                            std::thread::sleep(std::time::Duration::from_millis(200));
                            continue;
                        }
                    };

                    let connections_2 = connections_1.clone();
                    let cb_data = cb_data.clone();
                    std::thread::Builder::new()
                        .name(format!("ws client worker : {}", local_addr.clone()))
                        .spawn(move || {
                            client.set_nonblocking(false).unwrap();
                            let remote_addr = client.peer_addr().unwrap();

                            log::info!("ws accept from : {}", remote_addr);

                            let (mut receiver, sender) = client.split().unwrap();

                            {
                                let mut conns = connections_2.lock().unwrap();
                                conns.insert(remote_addr, sender);
                            }

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
                                            let mut sender = connections_2
                                                .lock()
                                                .unwrap()
                                                .remove(&remote_addr)
                                                .unwrap();

                                            let port = [buf[4], buf[5]];
                                            let port = u16::from_be_bytes(port);

                                            let full_addr = format!("127.0.0.1:{}", port);
                                            let tunnel_client =
                                                match TcpConnection::connect(&full_addr) {
                                                    Ok(p) => p,
                                                    Err(e) => {
                                                        log::error!(
                                                            "tunnel connect faild : {}",
                                                            e
                                                        );
                                                        break;
                                                    }
                                                };

                                            let mut tunnel_client_1 = tunnel_client.clone();
                                            std::thread::Builder::new()
                                                .name(format!(
                                                    "ws tunnel worker1 : {}",
                                                    tunnel_client.local_addr().unwrap()
                                                ))
                                                .spawn(move || {
                                                    loop {
                                                        let buf = match tunnel_client_1.recv() {
                                                            Ok(p) => p,
                                                            Err(e) => {
                                                                log::error!(
                                                                    "tunnel read faild : {}",
                                                                    e
                                                                );
                                                                break;
                                                            }
                                                        };

                                                        if buf.is_empty() {
                                                            break;
                                                        }

                                                        if let Err(e) = sender.send_message(
                                                            &OwnedMessage::Binary(buf),
                                                        ) {
                                                            log::error!(
                                                                "ws sender error : {}",
                                                                e
                                                            );
                                                            break;
                                                        }
                                                    }
                                                    log::debug!("ws tunnel1 finished!");
                                                })
                                                .unwrap();

                                            let mut tunnel_client_2 = tunnel_client.clone();
                                            std::thread::Builder::new()
                                                .name(format!(
                                                    "ws tunnel worker2 : {}",
                                                    tunnel_client.local_addr().unwrap()
                                                ))
                                                .spawn(move || {
                                                    loop {
                                                        let mut buf = match receiver
                                                            .recv_message()
                                                        {
                                                            Ok(p) => match p {
                                                                OwnedMessage::Binary(p) => p,
                                                                OwnedMessage::Close(_) => {
                                                                    break;
                                                                }
                                                                _ => continue,
                                                            },
                                                            Err(e) => {
                                                                log::error!(
                                                                    "ws receiver error : {}",
                                                                    e
                                                                );
                                                                break;
                                                            }
                                                        };

                                                        if let Err(e) =
                                                            tunnel_client_2.send(&mut buf)
                                                        {
                                                            log::error!(
                                                                "ws sender error : {}",
                                                                e
                                                            );
                                                            break;
                                                        }
                                                    }

                                                    log::debug!("ws tunnel2 finished!");
                                                })
                                                .unwrap();

                                            break;
                                        }

                                        cb_data.lock().unwrap()(
                                            HeroinnProtocol::HTTP,
                                            buf,
                                            remote_addr,
                                            cbcb,
                                        );
                                    }
                                    _ => {}
                                }
                            }
                            connections_2.lock().unwrap().remove(&remote_addr);
                            log::info!("ws client worker finished");
                        })
                        .unwrap();
                }

                let mut conns = connections_1.lock().unwrap();
                for i in conns.values_mut() {
                    match i.shutdown_all() {
                        Ok(_) => {}
                        Err(_) => {}
                    };
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

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok(self.local_addr)
    }

    fn sendto(&mut self, peer_addr: &SocketAddr, buf: &[u8]) -> std::io::Result<()> {
        match self.connections.lock().unwrap().get_mut(peer_addr) {
            Some(k) => {
                let msg = OwnedMessage::Binary(buf.to_vec());
                match k.send_message(&msg) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Interrupted,
                            format!("ws send msg error : {}", e),
                        ));
                    }
                };
                Ok(())
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found client",
            )),
        }
    }

    fn contains_addr(&mut self, peer_addr: &SocketAddr) -> bool {
        self.connections.lock().unwrap().contains_key(peer_addr)
    }

    fn close(&mut self) {
        self.closed
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Drop for WSServer {
    fn drop(&mut self) {
        self.close();
    }
}

impl Client for WSConnection {
    fn connect(address: &str) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let s = match websocket::ClientBuilder::new(&format!("ws://{}", address))
            .unwrap()
            .connect_insecure()
        {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    format!("ws connect error : {}", e),
                ));
            }
        };

        let local_addr = s.local_addr().unwrap();

        let (reader, writer) = s.split().unwrap();

        Ok(Self {
            reader: Some(Arc::new(Mutex::new(reader))),
            writer: Some(Arc::new(Mutex::new(writer))),
            closed: Arc::new(AtomicBool::new(false)),
            local_addr,
        })
    }

    fn tunnel(remote_addr: &str, server_local_port: u16) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        log::info!("start tunnel [{}] [{}]", remote_addr, server_local_port);
        let s = match websocket::ClientBuilder::new(&format!("ws://{}", remote_addr))
            .unwrap()
            .connect_insecure()
        {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    format!("ws connect error : {}", e),
                ));
            }
        };

        let local_addr = s.local_addr().unwrap();

        let mut buf = TUNNEL_FLAG.to_vec();
        buf.append(&mut server_local_port.to_be_bytes().to_vec());

        let (reader, writer) = s.split().unwrap();

        let mut ret = Self {
            reader: Some(Arc::new(Mutex::new(reader))),
            writer: Some(Arc::new(Mutex::new(writer))),
            closed: Arc::new(AtomicBool::new(false)),
            local_addr,
        };

        ret.send(&mut buf)?;

        Ok(ret)
    }

    fn recv(&mut self) -> std::io::Result<Vec<u8>> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "socket closed",
            ));
        }

        let s = match self.reader.as_mut() {
            Some(p) => p,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "socket closed",
                ));
            }
        };

        let mut s_lock = s.lock().unwrap();

        match s_lock.recv_message() {
            Ok(msg) => match msg {
                OwnedMessage::Binary(buf) => Ok(buf),
                OwnedMessage::Close(_) => {
                    drop(s_lock);
                    self.close();
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "ws closed".to_string(),
                    ))
                }
                _ => Ok(vec![]),
            },
            Err(e) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    format!("ws receive error : {}", e),
                ))
            }
        }
    }

    fn send(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "socket closed",
            ));
        }

        let s = match self.writer.as_mut() {
            Some(p) => p,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "socket closed",
                ));
            }
        };

        let msg = OwnedMessage::Binary(buf.to_vec());
        if let Err(e) = s.lock().unwrap().send_message(&msg) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                format!("ws send msg error : {}", e),
            ));
        };
        Ok(())
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok(self.local_addr)
    }

    fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        self.reader = None;
        self.writer = None;
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

impl Drop for WSConnection {
    fn drop(&mut self) {
        self.reader = None;
        self.writer = None;
    }
}

#[test]
fn test_ws_tunnel() {
    let server = WSServer::new("127.0.0.1:0", |_, _, _, _| {}, |_| {}).unwrap();
    let server2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let remote_local_port = server2.local_addr().unwrap().port();

    let remote = &format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let mut client1 = WSConnection::tunnel(remote, remote_local_port).unwrap();

    let (mut client2, _) = super::tcp::TcpConnection::tunnel_server(server2, 10).unwrap();

    for _ in 0..3 {
        client1.send(&mut [0, 1, 2, 3]).unwrap();
        let buf = client2.recv().unwrap();
        assert!(buf == [0, 1, 2, 3]);

        client2.send(&mut [4, 5, 6, 7]).unwrap();
        let buf = client1.recv().unwrap();
        assert!(buf == [4, 5, 6, 7]);
    }
}
