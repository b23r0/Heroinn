pub mod http;
pub mod tcp;
pub mod udp;
use std::{
    io::*,
    net::SocketAddr,
    ops::{Deref, DerefMut},
};

use crate::{packet::Message, HeroinnProtocol};

use self::{http::WSConnection, tcp::TcpConnection, udp::UDPConnection};

static TUNNEL_FLAG: [u8; 4] = [0x38, 0x38, 0x38, 0x38];

pub trait Client {
    fn connect(address: &str) -> Result<Self>
    where
        Self: Sized;
    fn tunnel(remote_addr: &str, server_local_port: u16) -> Result<Self>
    where
        Self: Sized;
    fn recv(&mut self) -> Result<Vec<u8>>;
    fn send(&mut self, buf: &mut [u8]) -> Result<()>;
    fn local_addr(&self) -> Result<SocketAddr>;
    fn close(&mut self);
}

pub trait Server {
    fn new<
        CBCB: 'static + Fn(Message) + Send + Copy,
        CB: 'static + Fn(HeroinnProtocol, Vec<u8>, SocketAddr, CBCB) + Send,
    >(
        address: &str,
        cb_data: CB,
        cbcb: CBCB,
    ) -> std::io::Result<Self>
    where
        Self: Sized;

    fn local_addr(&self) -> Result<SocketAddr>;
    fn sendto(&mut self, peer_addr: &SocketAddr, buf: &[u8]) -> Result<()>;
    fn contains_addr(&mut self, peer_addr: &SocketAddr) -> bool;
    fn close(&mut self);
}

pub struct ClientWrapper {
    typ: HeroinnProtocol,
    tcp_client: Option<TcpConnection>,
    http_client: Option<WSConnection>,
    udp_client: Option<UDPConnection>,
}

impl Deref for ClientWrapper {
    type Target = dyn Client;

    fn deref(&self) -> &Self::Target {
        match self.typ {
            HeroinnProtocol::TCP => self.tcp_client.as_ref().unwrap(),
            HeroinnProtocol::HTTP => self.http_client.as_ref().unwrap(),
            HeroinnProtocol::UDP => self.udp_client.as_ref().unwrap(),
            HeroinnProtocol::Unknow => panic!("unknow protocol"),
        }
    }
}

impl DerefMut for ClientWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self.typ {
            HeroinnProtocol::TCP => self.tcp_client.as_mut().unwrap(),
            HeroinnProtocol::HTTP => self.http_client.as_mut().unwrap(),
            HeroinnProtocol::UDP => self.udp_client.as_mut().unwrap(),
            HeroinnProtocol::Unknow => panic!("unknow protocol"),
        }
    }
}

impl Clone for ClientWrapper {
    fn clone(&self) -> Self {
        Self {
            typ: self.typ.clone(),
            tcp_client: self.tcp_client.clone(),
            http_client: self.http_client.clone(),
            udp_client: self.udp_client.clone(),
        }
    }
}

impl ClientWrapper {
    pub fn connect(typ: &HeroinnProtocol, address: &str) -> Result<Self> {
        match typ {
            HeroinnProtocol::TCP => {
                let client = TcpConnection::connect(address)?;
                Ok(Self {
                    typ: typ.clone(),
                    tcp_client: Some(client),
                    http_client: None,
                    udp_client: None,
                })
            }
            HeroinnProtocol::HTTP => {
                let client = WSConnection::connect(address)?;
                Ok(Self {
                    typ: typ.clone(),
                    tcp_client: None,
                    http_client: Some(client),
                    udp_client: None,
                })
            }
            HeroinnProtocol::UDP => {
                let client = UDPConnection::connect(address)?;
                Ok(Self {
                    typ: typ.clone(),
                    tcp_client: None,
                    http_client: None,
                    udp_client: Some(client),
                })
            }
            HeroinnProtocol::Unknow => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invaild protocol type",
                ))
            }
        }
    }
}

pub fn create_tunnel(
    addr: &str,
    protocol: &HeroinnProtocol,
    server_local_port: u16,
) -> Result<Box<dyn Client>> {
    Ok(match protocol {
        HeroinnProtocol::TCP => Box::new(TcpConnection::tunnel(addr, server_local_port)?),
        HeroinnProtocol::HTTP => Box::new(WSConnection::tunnel(addr, server_local_port)?),
        HeroinnProtocol::UDP => Box::new(UDPConnection::tunnel(addr, server_local_port)?),
        HeroinnProtocol::Unknow => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            ));
        }
    })
}
