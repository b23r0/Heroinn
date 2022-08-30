use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use net2::TcpStreamExt;

use crate::packet::Message;
use crate::protocol::TUNNEL_FLAG;
use crate::{cur_timestamp_secs, HeroinnProtocol};

use super::Client;
use super::Server;

const TCP_MAX_PACKET: u32 = 1024 * 9999;

pub struct TcpServer {
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    connections: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
}

pub struct TcpConnection {
    s: Option<TcpStream>,
    closed: Arc<AtomicBool>,
}

impl Clone for TcpConnection {
    fn clone(&self) -> Self {
        Self {
            s: Some(self.s.as_ref().unwrap().try_clone().unwrap()),
            closed: self.closed.clone(),
        }
    }
}

impl Server for TcpServer {
    fn new<
        CBCB: 'static + Fn(Message) + Send + Copy,
        CB: 'static + Fn(HeroinnProtocol, Vec<u8>, SocketAddr, CBCB) + Send,
    >(
        address: &str,
        cb_data: CB,
        cbcb: CBCB,
    ) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let mut local_addr: SocketAddr = address.parse().unwrap();
        let server = TcpListener::bind(local_addr)?;
        local_addr = server.local_addr().unwrap();

        server.set_nonblocking(true)?;

        let connections = Arc::new(Mutex::new(HashMap::new()));

        let closed = Arc::new(AtomicBool::new(false));

        let closed_1 = closed.clone();
        let connections_1 = connections.clone();

        let cb_data = Arc::new(Mutex::new(cb_data));
        std::thread::Builder::new()
            .name(format!(
                "tcp main worker : {}",
                server.local_addr().unwrap()
            ))
            .spawn(move || {
                for stream in server.incoming() {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    let cb_data = cb_data.clone();
                    match stream {
                        Ok(s) => {
                            s.set_nonblocking(false).unwrap();
                            s.set_keepalive_ms(Some(200)).unwrap();

                            let peer_addr = s.peer_addr().unwrap();
                            let mut s_1 = s.try_clone().unwrap();
                            connections_1.lock().unwrap().insert(peer_addr, s);

                            let connections_2 = connections_1.clone();

                            std::thread::Builder::new()
                                .name(format!("tcp client worker : {}", s_1.peer_addr().unwrap()))
                                .spawn(move || {
                                    loop {
                                        let mut size_buf = [0u8; 4];
                                        match s_1.read_exact(&mut size_buf) {
                                            Ok(_) => {}
                                            Err(_) => break,
                                        };

                                        if size_buf == TUNNEL_FLAG {
                                            let mut port = [0u8; 2];
                                            match s_1.read_exact(&mut port) {
                                                Ok(_) => {}
                                                Err(_) => break,
                                            };

                                            let port = u16::from_be_bytes(port);

                                            let full_addr = format!("127.0.0.1:{}", port);
                                            let tunnel_client = match TcpStream::connect(&full_addr)
                                            {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    log::error!("tunnel connect faild : {}", e);
                                                    break;
                                                }
                                            };

                                            let mut tunnel_client_1 =
                                                tunnel_client.try_clone().unwrap();
                                            let mut s_2 = s_1.try_clone().unwrap();
                                            std::thread::Builder::new()
                                                .name(format!(
                                                    "tunnel worker1 : {} , {}",
                                                    s_1.peer_addr().unwrap(),
                                                    tunnel_client.peer_addr().unwrap()
                                                ))
                                                .spawn(move || {
                                                    if let Err(e) = std::io::copy(
                                                        &mut tunnel_client_1,
                                                        &mut s_2,
                                                    ) {
                                                        log::error!(
                                                            "tunnel1 io copy faild : {}",
                                                            e
                                                        );
                                                    };
                                                    log::debug!("tunnel1 finished!");
                                                })
                                                .unwrap();

                                            let mut tunnel_client_2 =
                                                tunnel_client.try_clone().unwrap();
                                            let mut s_3 = s_1.try_clone().unwrap();
                                            std::thread::Builder::new()
                                                .name(format!(
                                                    "tunnel worker2 : {} , {}",
                                                    s_1.peer_addr().unwrap(),
                                                    tunnel_client.peer_addr().unwrap()
                                                ))
                                                .spawn(move || {
                                                    if let Err(e) = std::io::copy(
                                                        &mut s_3,
                                                        &mut tunnel_client_2,
                                                    ) {
                                                        log::error!(
                                                            "tunnel2 io copy faild : {}",
                                                            e
                                                        );
                                                    };
                                                    log::debug!("tunnel2 finished!");
                                                })
                                                .unwrap();

                                            break;
                                        } else {
                                            let size = u32::from_be_bytes(size_buf);
                                            if size > TCP_MAX_PACKET {
                                                log::error!("packet length error!");
                                                break;
                                            }

                                            let mut buf = vec![0u8; size as usize];

                                            match s_1.read_exact(&mut buf) {
                                                Ok(_) => {}
                                                Err(_) => break,
                                            };

                                            cb_data.lock().unwrap()(
                                                HeroinnProtocol::TCP,
                                                buf,
                                                peer_addr,
                                                cbcb,
                                            );
                                        }
                                    }

                                    log::info!("connection closed or enter tunnel : {}", peer_addr);
                                    connections_2.lock().unwrap().remove(&peer_addr);
                                })
                                .unwrap();
                        }
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::WouldBlock {
                                if closed_1.load(Ordering::Relaxed) {
                                    break;
                                }
                            } else {
                                continue;
                            }
                        }
                    }
                }

                let mut conns = connections_1.lock().unwrap();
                for i in conns.values_mut() {
                    match i.shutdown(std::net::Shutdown::Both) {
                        Ok(_) => {}
                        Err(_) => {}
                    };
                }
                conns.clear();
                log::info!("server closed");
            })
            .unwrap();

        Ok(Self {
            local_addr,
            closed,
            connections,
        })
    }

    fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok(self.local_addr)
    }

    fn sendto(&mut self, peer_addr: &SocketAddr, buf: &[u8]) -> std::io::Result<()> {
        match self.connections.lock().unwrap().get(peer_addr) {
            Some(mut k) => {
                let size = buf.len() as u32;

                if size > TCP_MAX_PACKET {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("packet size error : {}", size),
                    ));
                }

                let size = size.to_be_bytes();
                k.write_all(&size)?;
                k.write_all(buf)?;
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
}

impl Client for TcpConnection {
    fn connect(address: &str) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let address: std::net::SocketAddr = match address.parse() {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("address format error : {}", e),
                ))
            }
        };
        let s = TcpStream::connect(address)?;
        Ok(Self {
            s: Some(s),
            closed: Arc::new(AtomicBool::new(false)),
        })
    }

    fn tunnel(remote_addr: &str, server_local_port: u16) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let remote_addr: SocketAddr = remote_addr.parse().unwrap();

        log::info!("start tunnel [{}] [{}]", remote_addr, server_local_port);
        let mut s = TcpStream::connect(remote_addr)?;

        let buf = TUNNEL_FLAG.to_vec();

        s.write_all(&buf)?;
        s.write_all(server_local_port.to_be_bytes().as_ref())?;

        Ok(Self {
            s: Some(s),
            closed: Arc::new(AtomicBool::new(false)),
        })
    }

    fn recv(&mut self) -> std::io::Result<Vec<u8>> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "socket closed",
            ));
        }

        let mut size_buf = [0u8; 4];

        let s = match self.s.as_mut() {
            Some(p) => p,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "socket closed",
                ));
            }
        };

        s.read_exact(&mut size_buf)?;

        let size = u32::from_be_bytes(size_buf);

        if size > TCP_MAX_PACKET {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("packet size error : {}", size),
            ));
        }

        let mut buf = vec![0u8; size as usize];

        s.read_exact(&mut buf)?;

        Ok(buf)
    }

    fn send(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "socket closed",
            ));
        }

        let size = buf.len() as u32;

        if size > TCP_MAX_PACKET {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("packet size error : {}", size),
            ));
        }

        let s = match self.s.as_mut() {
            Some(p) => p,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "socket closed",
                ));
            }
        };

        let size = size.to_be_bytes();

        s.write_all(&size)?;

        s.write_all(buf)?;

        Ok(())
    }

    fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        self.s = None;
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        let s = match self.s.as_ref() {
            Some(p) => p,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "socket closed",
                ));
            }
        };
        s.local_addr()
    }
}

impl TcpConnection {
    pub fn tunnel_server(
        server: TcpListener,
        timeout_secs: u64,
    ) -> std::io::Result<(Self, SocketAddr)> {
        server.set_nonblocking(true).unwrap();

        let t = cur_timestamp_secs();

        loop {
            let (s, addr) = match server.accept() {
                Ok(p) => p,
                Err(e) => {
                    if cur_timestamp_secs() - t > timeout_secs {
                        return Err(e);
                    }

                    std::thread::sleep(std::time::Duration::from_millis(200));
                    continue;
                }
            };

            s.set_nonblocking(false).unwrap();

            return Ok((
                Self {
                    s: Some(s),
                    closed: Arc::new(AtomicBool::new(false)),
                },
                addr,
            ));
        }
    }
}

impl Drop for TcpConnection {
    fn drop(&mut self) {
        if let Some(s) = self.s.as_ref() {
            log::info!("tcp client [{}] dropped", s.peer_addr().unwrap());
            self.s = None;
        } else {
            log::info!("tcp client dropped");
        }
    }
}

impl Drop for TcpServer {
    fn drop(&mut self) {
        self.close();
        for i in self.connections.lock().unwrap().values() {
            log::info!("tcp [{}] dropped", i.peer_addr().unwrap());
        }
    }
}

#[test]
fn test_tcp_tunnel() {
    let server = TcpServer::new("127.0.0.1:0", |_, _, _, _| {}, |_| {}).unwrap();
    let server2 = TcpListener::bind("127.0.0.1:0").unwrap();
    let remote_local_port = server2.local_addr().unwrap().port();

    let remote = &format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let mut client1 = TcpConnection::tunnel(remote, remote_local_port).unwrap();

    let (mut client2, _) = TcpConnection::tunnel_server(server2, 10).unwrap();

    for _ in 0..3 {
        client1.send(&mut [0, 1, 2, 3]).unwrap();
        let buf = client2.recv().unwrap();
        assert!(buf == [0, 1, 2, 3]);

        client2.send(&mut [4, 5, 6, 7]).unwrap();
        let buf = client1.recv().unwrap();
        assert!(buf == [4, 5, 6, 7]);
    }
}
