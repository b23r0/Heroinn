use std::io::*;
use std::net::SocketAddr;

pub mod module;

use heroinn_util::protocol::http::WSServer;
use heroinn_util::protocol::udp::UDPServer;
use heroinn_util::{packet::Message, protocol::tcp::*, protocol::Server, *};

pub struct HeroinnServer {
    tcp_server: Option<TcpServer>,
    ws_server: Option<WSServer>,
    udp_server: Option<UDPServer>,
    protocol: HeroinnProtocol,
}

impl HeroinnServer {
    fn cb_connection<CB: 'static + Fn(Message) + Send + Copy>(
        proto: HeroinnProtocol,
        data: Vec<u8>,
        peer_addr: SocketAddr,
        cb: CB,
    ) {
        let msg = Message::new(peer_addr, proto, &data).unwrap();
        cb(msg);
    }

    pub fn new<CB: 'static + Fn(Message) + Send + Copy>(
        protocol: HeroinnProtocol,
        port: u16,
        cb_msg: CB,
    ) -> std::io::Result<Self> {
        match protocol {
            HeroinnProtocol::TCP => {
                match TcpServer::new(
                    format!("0.0.0.0:{}", port).as_str(),
                    HeroinnServer::cb_connection,
                    cb_msg,
                ) {
                    Ok(tcp_server) => Ok(Self {
                        tcp_server: Some(tcp_server),
                        ws_server: None,
                        udp_server: None,
                        protocol,
                    }),
                    Err(e) => Err(e),
                }
            }
            HeroinnProtocol::HTTP => {
                match WSServer::new(
                    format!("0.0.0.0:{}", port).as_str(),
                    HeroinnServer::cb_connection,
                    cb_msg,
                ) {
                    Ok(ws_server) => Ok(Self {
                        tcp_server: None,
                        ws_server: Some(ws_server),
                        udp_server: None,
                        protocol,
                    }),
                    Err(e) => Err(e),
                }
            }
            HeroinnProtocol::UDP => {
                match UDPServer::new(
                    format!("0.0.0.0:{}", port).as_str(),
                    HeroinnServer::cb_connection,
                    cb_msg,
                ) {
                    Ok(udp_server) => Ok(Self {
                        tcp_server: None,
                        ws_server: None,
                        udp_server: Some(udp_server),
                        protocol,
                    }),
                    Err(e) => Err(e),
                }
            }
            HeroinnProtocol::Unknow => panic!("unknow protocol"),
        }
    }

    pub fn sendto(&mut self, peer_addr: &SocketAddr, buf: &[u8]) -> Result<()> {
        match self.protocol {
            HeroinnProtocol::TCP => self.tcp_server.as_mut().unwrap().sendto(peer_addr, buf),
            HeroinnProtocol::HTTP => self.ws_server.as_mut().unwrap().sendto(peer_addr, buf),
            HeroinnProtocol::UDP => self.udp_server.as_mut().unwrap().sendto(peer_addr, buf),
            HeroinnProtocol::Unknow => panic!("unknow protocol"),
        }
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        match self.protocol {
            HeroinnProtocol::TCP => self.tcp_server.as_ref().unwrap().local_addr(),
            HeroinnProtocol::HTTP => self.ws_server.as_ref().unwrap().local_addr(),
            HeroinnProtocol::UDP => self.udp_server.as_ref().unwrap().local_addr(),
            HeroinnProtocol::Unknow => panic!("unknow protocol"),
        }
    }

    pub fn proto(&self) -> HeroinnProtocol {
        self.protocol.clone()
    }

    pub fn contains_addr(&mut self, peer_addr: &SocketAddr) -> bool {
        match self.protocol {
            HeroinnProtocol::TCP => self.tcp_server.as_mut().unwrap().contains_addr(peer_addr),
            HeroinnProtocol::HTTP => self.ws_server.as_mut().unwrap().contains_addr(peer_addr),
            HeroinnProtocol::UDP => self.udp_server.as_mut().unwrap().contains_addr(peer_addr),
            HeroinnProtocol::Unknow => panic!("unknow protocol"),
        }
    }

    pub fn close(&mut self) {
        match self.protocol {
            HeroinnProtocol::TCP => self.tcp_server.as_mut().unwrap().close(),
            HeroinnProtocol::HTTP => self.ws_server.as_mut().unwrap().close(),
            HeroinnProtocol::UDP => self.udp_server.as_mut().unwrap().close(),
            HeroinnProtocol::Unknow => panic!("unknow protocol"),
        }
    }
}
