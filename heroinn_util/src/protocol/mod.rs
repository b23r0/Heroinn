
pub mod tcp;
use std::{io::*, net::SocketAddr};

use crate::{HeroinnProtocol, packet::Message};

static TUNNEL_FLAG : [u8;4] = [0x38, 0x38 , 0x38, 0x38];

pub trait Client<T>{
    fn connect(address : &str) -> Result<Self> where Self: Sized;
    fn from(s : T) -> Result<Self> where Self: Sized;
    fn recv(&mut self) -> Result<Vec<u8>>;
    fn send(&mut self,buf : &mut [u8]) -> Result<()>;
    fn local_addr(&self) -> Result<SocketAddr>;
    fn close(&mut self);
}

pub trait Server<T> {
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

pub trait TunnelClient {
    fn tunnel(remote_addr : & SocketAddr , server_local_port : u16) -> Result<Self> where Self: Sized;
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()>;
    fn write_all(&mut self, buf: &[u8]) -> Result<()>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}