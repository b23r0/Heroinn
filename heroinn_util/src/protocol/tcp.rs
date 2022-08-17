use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener, SocketAddr};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::packet::Message;
use crate::protocol::TUNNEL_FLAG;
use crate::{ HeroinnProtocol};

use super::{Server, TunnelClient};
use super::Client;

const TCP_MAX_PACKET: u32 = 1024*9999;

pub struct TcpServer{
    local_addr : SocketAddr,
    closed : Arc<AtomicBool>,
    connections : Arc<Mutex<HashMap<SocketAddr , TcpStream>>>
}

pub struct TcpConnection{
    s : TcpStream,
    is_tunnel : bool
}

impl Clone for TcpConnection{
    fn clone(&self) -> Self {
        Self { s: self.s.try_clone().unwrap() , is_tunnel:  self.is_tunnel }
    }
}

impl Server<TcpStream> for TcpServer{
    fn new<
        CBCB: 'static + Fn(Message) + Send + Copy , 
        CB: 'static + Fn(HeroinnProtocol , Vec<u8>, SocketAddr, CBCB) + Send
    >(
        address : &str , 
        cb_data : CB,
        cbcb : CBCB,
    ) -> std::io::Result<Self> where Self: Sized{
        let mut local_addr : SocketAddr = address.parse().unwrap();
        let server = TcpListener::bind(local_addr)?;
        local_addr = server.local_addr().unwrap();

        server.set_nonblocking(true)?;

        let connections = Arc::new(Mutex::new(HashMap::new()));
        
        let closed = Arc::new(AtomicBool::new(false));

        let closed_1 = closed.clone();
        let connections_1 = connections.clone();

        let cb_data = Arc::new(Mutex::new(cb_data));
        std::thread::spawn(move || {

            for stream in server.incoming(){
                std::thread::sleep(std::time::Duration::from_millis(200));
                let cb_data = cb_data.clone();
                match stream {
                    Ok(s) => {

                        s.set_nonblocking(false).unwrap();

                        let peer_addr = s.peer_addr().unwrap();
                        let mut s_1 = s.try_clone().unwrap();
                        connections_1.lock().unwrap().insert(peer_addr, s);

                        let connections_2 = connections_1.clone();
                        
                        std::thread::spawn(move || {

                            loop{
                                let mut size_buf = [0u8 ; 4];
                                match s_1.read_exact(&mut size_buf){
                                    Ok(_) => {},
                                    Err(_) => break,
                                };

                                if size_buf == TUNNEL_FLAG {
                                    let mut port = [0u8;2];
                                    match s_1.read_exact(&mut port){
                                        Ok(_) => {},
                                        Err(_) => break,
                                    };

                                    let port = u16::from_be_bytes(port);

                                    let full_addr = format!("127.0.0.1:{}", port);
                                    let tunnel_client = match TcpStream::connect(&full_addr){
                                        Ok(p) => p,
                                        Err(e) => {
                                            log::error!("tunnel connect faild : {}" , e);
                                            break;
                                        },
                                    };

                                    let mut tunnel_client_1 = tunnel_client.try_clone().unwrap();
                                    let mut s_2 = s_1.try_clone().unwrap();
                                    std::thread::spawn(move || {
                                        let mut buf = [0u8;1024];
                                        loop{
                                            let size = match tunnel_client_1.read(&mut buf){
                                                Ok(p) => p,
                                                Err(e) => {
                                                    log::error!("tunnel1 recv faild : {}" , e);
                                                    break;
                                                },
                                            };

                                            match s_2.write_all(&buf[..size]){
                                                Ok(p) => p,
                                                Err(e) => {
                                                    log::error!("tunnel1 send faild : {}" , e);
                                                    break;
                                                },
                                            };
                                        }

                                        log::info!("tunnel1 finished!");

                                        match tunnel_client_1.shutdown(std::net::Shutdown::Both){
                                            Ok(_) => {},
                                            Err(_) => {},
                                        };

                                        match s_2.shutdown(std::net::Shutdown::Both){
                                            Ok(_) => {},
                                            Err(_) => {},
                                        };
                                    });

                                    let mut tunnel_client_2 = tunnel_client.try_clone().unwrap();
                                    let mut s_3 = s_1.try_clone().unwrap();
                                    std::thread::spawn(move || {
                                        let mut buf = [0u8;1024];
                                        loop{
                                            let size = match s_3.read(&mut buf){
                                                Ok(p) => p,
                                                Err(e) => {
                                                    log::error!("tunnel2 recv faild : {}" , e);
                                                    break;
                                                },
                                            };

                                            match tunnel_client_2.write_all(&buf[..size]){
                                                Ok(p) => p,
                                                Err(e) => {
                                                    log::error!("tunnel2 send faild : {}" , e);
                                                    break;
                                                },
                                            };
                                        }

                                        log::info!("tunnel2 finished!");

                                        match s_3.shutdown(std::net::Shutdown::Both){
                                            Ok(_) => {},
                                            Err(_) => {},
                                        };

                                        match tunnel_client_2.shutdown(std::net::Shutdown::Both){
                                            Ok(_) => {},
                                            Err(_) => {},
                                        };
                                    });

                                    break;
                                }

                                let size = u32::from_be_bytes(size_buf);
                                if size > TCP_MAX_PACKET{
                                    log::error!("packet length error!");
                                    break;
                                }
                        
                                let mut buf = vec![0u8 ; size as usize];
                        
                                match s_1.read_exact(&mut buf){
                                    Ok(_) => {},
                                    Err(_) => break,
                                };
    
                                cb_data.lock().unwrap()(HeroinnProtocol::TCP , buf, peer_addr, cbcb);
                            }

                            log::info!("connection closed or enter tunnel : {}" , peer_addr);
                            connections_2.lock().unwrap().remove(&peer_addr);
                        });

                    },
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            if closed_1.load(Ordering::Relaxed){
                                break;
                            }
                        }else {
                            continue;
                        }
                    },
                }
            }
            log::info!("server closed");
        });

        Ok(Self{
            local_addr,
            closed,
            connections
        })
    }

    fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok(self.local_addr)
    }

    fn sendto(&mut self , peer_addr : &SocketAddr , buf : &[u8]) -> std::io::Result<()> {
        match self.connections.lock().unwrap().get(peer_addr){
            Some(mut k) => {
                let size = buf.len() as u32;

                if size > TCP_MAX_PACKET{
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData , format!("packet size error : {}", size)));
                }
        
                let size = size.to_be_bytes();
                k.write_all(&size)?;
                k.write_all(buf)?;
                Ok(())
            },
            None => Err(std::io::Error::new(std::io::ErrorKind::NotFound, "not found client")),
        }
    }

    fn contains_addr(&mut self , peer_addr : &SocketAddr) -> bool{
        self.connections.lock().unwrap().contains_key(peer_addr)
    }
}

impl Client<TcpStream> for TcpConnection{
    fn connect(address : &str) -> std::io::Result<Self> where Self: Sized {
        let address : std::net::SocketAddr = match address.parse(){
            Ok(p) => p,
            Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData , format!("address format error : {}", e))),
        };
        let s = TcpStream::connect(address)?;
        Ok(Self{s , is_tunnel : false})
    }

    fn from(s : TcpStream) -> std::io::Result<Self> where Self: Sized {
        Ok(Self{s , is_tunnel : false})
    }

    fn recv(&mut self) -> std::io::Result<Vec<u8>> {

        let mut size_buf = [0u8 ; 4];
        
        self.s.read_exact(&mut size_buf)?;

        let size = u32::from_be_bytes(size_buf);

        if size > TCP_MAX_PACKET{
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData , format!("packet size error : {}", size)));
        }

        let mut buf = vec![0u8 ; size as usize];

        self.s.read_exact(&mut buf)?;

        Ok(buf)
    }

    fn send(&mut self,buf : &mut [u8]) -> std::io::Result<()> {
        let size = buf.len() as u32;

        if size > TCP_MAX_PACKET{
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData , format!("packet size error : {}", size)));
        }

        let size = size.to_be_bytes();

        self.s.write_all(&size)?;

        self.s.write_all(buf)?;
        
        Ok(())
    }

    fn close(&mut self) {
        match self.s.shutdown(std::net::Shutdown::Both){
            Ok(_) => {},
            Err(_) => {},
        };
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.s.local_addr()
    }
}

impl TunnelClient for TcpConnection{
    fn tunnel(remote_addr : &str , server_local_port : u16) -> std::io::Result<Self> where Self: Sized {
        let remote_addr : SocketAddr = remote_addr.parse().unwrap();
        
        log::info!("start tunnel [{}] [{}]", remote_addr , server_local_port);
        let mut s = TcpStream::connect(remote_addr)?;

        let buf = TUNNEL_FLAG.to_vec();
        
        s.write_all(&buf)?;
        s.write_all(&server_local_port.to_be_bytes().to_vec())?;

        Ok(Self{
            s,
            is_tunnel: true,
        })
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.s.read_exact(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.s.write_all(buf)
    }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.s.read(buf)
    }
}

impl Drop for TcpServer{
    fn drop(&mut self) {
        for i in self.connections.lock().unwrap().values(){
            log::info!("tcpserver dropped");
            match i.shutdown(std::net::Shutdown::Both){
                Ok(_) => {},
                Err(_) => {},
            }
        }
    }
}

#[test]
fn test_tcp_tunnel(){

    let mut server = TcpServer::new(&"127.0.0.1:0", |_ , _ ,_ ,_| {} , |_| {}).unwrap();
    let server2 = TcpListener::bind(&"127.0.0.1:0").unwrap();

    let remote = &format!("127.0.0.1:{}" , server.local_addr().unwrap().port());
    let remote_local_port = server2.local_addr().unwrap().port();
    let mut client1 = TcpConnection::tunnel(remote, remote_local_port).unwrap();

    let (mut client2 , _) = server2.accept().unwrap();

    for _ in 0..3{
        let mut buf = [0u8;4];
        client1.write_all(&[0,1,2,3]).unwrap();
        client2.read_exact(&mut buf).unwrap();
        assert!(buf == [0,1,2,3]);

        let mut buf = [0u8;4];
        client2.write_all(&[5,6,7,8]).unwrap();
        client1.read_exact(&mut buf).unwrap();
        assert!(buf == [5,6,7,8]);
    }

    client1.close();
    server.close();
}