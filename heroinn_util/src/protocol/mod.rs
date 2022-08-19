
pub mod tcp;
use std::{io::*, net::SocketAddr};

use crate::{HeroinnProtocol, packet::Message};

use self::tcp::TcpConnection;

static TUNNEL_FLAG : [u8;4] = [0x38, 0x38 , 0x38, 0x38];

pub trait Client{
    fn connect(address : &str) -> Result<Self> where Self: Sized;
    fn tunnel(remote_addr : &str , server_local_port : u16) -> Result<Self> where Self: Sized;
    fn recv(&mut self) -> Result<Vec<u8>>;
    fn send(&mut self,buf : &mut [u8]) -> Result<()>;
    fn local_addr(&self) -> Result<SocketAddr>;
    fn close(&mut self);
}

pub trait Server {
    fn new<
        CBCB: 'static + Fn(Message) + Send + Copy , 
        CB: 'static + Fn(HeroinnProtocol , Vec<u8>, SocketAddr, CBCB) + Send
    >(
        address : &str , 
        cb_data : CB,
        cbcb : CBCB,
    ) -> std::io::Result<Self> where Self: Sized;

    fn local_addr(&self) -> Result<SocketAddr>;
    fn sendto(&mut self , peer_addr : &SocketAddr , buf : &[u8]) -> Result<()>;
    fn contains_addr(&mut self , peer_addr : &SocketAddr) -> bool;
    fn close(&mut self);
}

pub fn create_tunnel(addr : &str , protocol : &HeroinnProtocol , server_local_port : u16) -> Result<Box<dyn Client>>{
    Ok(match protocol{
        HeroinnProtocol::TCP => {
            Box::new(TcpConnection::tunnel(addr, server_local_port)?)
        },
    })
}