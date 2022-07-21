use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener, SocketAddr};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::packet::Message;
use crate::{ HeroinnProtocol};

use super::Server;
use super::Client;

const TCP_MAX_PACKET: u32 = 1024*10;

pub struct TcpServer{
    local_addr : SocketAddr,
    closed : Arc<AtomicBool>,
    connections : Arc<Mutex<HashMap<SocketAddr , TcpStream>>>
}

pub struct TcpConnection{
    s : TcpStream
}

impl Clone for TcpConnection{
    fn clone(&self) -> Self {
        Self { s: self.s.try_clone().unwrap() }
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
        let local_addr : SocketAddr = address.parse().unwrap();
        let server = TcpListener::bind(local_addr)?;
        server.set_nonblocking(true)?;

        let connections = Arc::new(Mutex::new(HashMap::new()));
        
        let closed = Arc::new(AtomicBool::new(false));

        let closed_1 = closed.clone();
        let connections_1 = connections.clone();

        let cb_data = Arc::new(Mutex::new(cb_data));
        std::thread::spawn(move || {

            for stream in server.incoming(){
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

                            log::info!("connection closed : {}" , peer_addr);
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
            connections,
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
        let address : std::net::SocketAddr = address.parse().unwrap();
        let s = TcpStream::connect(address)?;
        Ok(Self{s})
    }

    fn from(s : TcpStream) -> std::io::Result<Self> where Self: Sized {
        Ok(Self{s})
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