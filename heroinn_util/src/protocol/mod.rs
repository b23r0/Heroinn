
pub mod tcp;
use std::{io::*, net::SocketAddr};

use crate::{HeroinnProtocol, packet::Message};

pub trait Client<T>{
    fn connect(address : &str) -> Result<Self> where Self: Sized;
    fn from(s : T) -> Result<Self> where Self: Sized;
    fn recv(&mut self) -> Result<Vec<u8>>;
    fn send(&mut self,buf : &mut [u8]) -> Result<()>;
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

    fn sendto(&mut self , peer_addr : SocketAddr , buf : &[u8]) -> Result<()>;

    fn close(&mut self);
}