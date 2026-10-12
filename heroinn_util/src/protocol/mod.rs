//! Transport layer.
//!
//! Two closed enums cover everything the rest of the workspace needs:
//!
//! * [`Transport`] — an outbound connection used by clients and tunnels.
//! * [`Listener`] — an inbound server used by the GUI to accept clients.
//!
//! Both wrap the three concrete implementations (plain TCP, WebSocket and
//! reliable UDP) and dispatch to them, so callers never deal with a trait
//! object or an `Option`-per-protocol struct.

pub mod frame;
pub mod http;
pub mod tcp;
pub mod udp;

use std::net::SocketAddr;
use std::sync::Arc;

use crate::error::Result;
use crate::packet::Message;
use crate::HeroinnProtocol;

use self::http::{WSConnection, WSServer};
use self::tcp::{TcpConnection, TcpServer};
use self::udp::{UDPConnection, UDPServer};

/// Sentinel written before a 2-byte port when a peer asks for a reverse tunnel.
static TUNNEL_FLAG: [u8; 4] = [0x38, 0x38, 0x38, 0x38];

/// Callback invoked for every decoded inbound message.
pub type MessageHandler = Arc<dyn Fn(Message) + Send + Sync>;

/// Outbound connection to a peer.
pub enum Transport {
    Tcp(TcpConnection),
    Http(WSConnection),
    Udp(UDPConnection),
}

impl Transport {
    /// Dial a peer over `protocol`.
    pub fn connect(protocol: &HeroinnProtocol, address: &str) -> Result<Self> {
        Ok(match protocol {
            HeroinnProtocol::TCP => Self::Tcp(TcpConnection::connect(address)?),
            HeroinnProtocol::HTTP => Self::Http(WSConnection::connect(address)?),
            HeroinnProtocol::UDP => Self::Udp(UDPConnection::connect(address)?),
        })
    }

    /// Dial `remote_addr` and ask it to tunnel to `server_local_port`.
    pub fn tunnel(
        protocol: &HeroinnProtocol,
        remote_addr: &str,
        server_local_port: u16,
    ) -> Result<Self> {
        Ok(match protocol {
            HeroinnProtocol::TCP => {
                Self::Tcp(TcpConnection::tunnel(remote_addr, server_local_port)?)
            }
            HeroinnProtocol::HTTP => {
                Self::Http(WSConnection::tunnel(remote_addr, server_local_port)?)
            }
            HeroinnProtocol::UDP => {
                Self::Udp(UDPConnection::tunnel(remote_addr, server_local_port)?)
            }
        })
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        match self {
            Self::Tcp(c) => c.recv(),
            Self::Http(c) => c.recv(),
            Self::Udp(c) => c.recv(),
        }
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<()> {
        match self {
            Self::Tcp(c) => c.send(buf),
            Self::Http(c) => c.send(buf),
            Self::Udp(c) => c.send(buf),
        }
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        match self {
            Self::Tcp(c) => c.local_addr(),
            Self::Http(c) => c.local_addr(),
            Self::Udp(c) => c.local_addr(),
        }
    }

    pub fn close(&mut self) {
        match self {
            Self::Tcp(c) => c.close(),
            Self::Http(c) => c.close(),
            Self::Udp(c) => c.close(),
        }
    }
}

impl Clone for Transport {
    fn clone(&self) -> Self {
        match self {
            Self::Tcp(c) => Self::Tcp(c.clone()),
            Self::Http(c) => Self::Http(c.clone()),
            Self::Udp(c) => Self::Udp(c.clone()),
        }
    }
}

/// Inbound server accepting peers on one protocol.
pub enum Listener {
    Tcp(TcpServer),
    Http(WSServer),
    Udp(UDPServer),
}

impl Listener {
    /// Bind `0.0.0.0:port` and deliver every decoded message to `on_message`.
    pub fn bind(protocol: &HeroinnProtocol, port: u16, on_message: MessageHandler) -> Result<Self> {
        let address = format!("0.0.0.0:{}", port);

        Ok(match protocol {
            HeroinnProtocol::TCP => Self::Tcp(TcpServer::new(&address, on_message)?),
            HeroinnProtocol::HTTP => Self::Http(WSServer::new(&address, on_message)?),
            HeroinnProtocol::UDP => Self::Udp(UDPServer::new(&address, on_message)?),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        match self {
            Self::Tcp(s) => s.local_addr(),
            Self::Http(s) => s.local_addr(),
            Self::Udp(s) => s.local_addr(),
        }
    }

    pub fn sendto(&mut self, peer_addr: &SocketAddr, buf: &[u8]) -> Result<()> {
        match self {
            Self::Tcp(s) => s.sendto(peer_addr, buf),
            Self::Http(s) => s.sendto(peer_addr, buf),
            Self::Udp(s) => s.sendto(peer_addr, buf),
        }
    }

    pub fn contains_addr(&mut self, peer_addr: &SocketAddr) -> bool {
        match self {
            Self::Tcp(s) => s.contains_addr(peer_addr),
            Self::Http(s) => s.contains_addr(peer_addr),
            Self::Udp(s) => s.contains_addr(peer_addr),
        }
    }

    pub fn close(&mut self) {
        match self {
            Self::Tcp(s) => s.close(),
            Self::Http(s) => s.close(),
            Self::Udp(s) => s.close(),
        }
    }

    pub fn proto(&self) -> HeroinnProtocol {
        match self {
            Self::Tcp(_) => HeroinnProtocol::TCP,
            Self::Http(_) => HeroinnProtocol::HTTP,
            Self::Udp(_) => HeroinnProtocol::UDP,
        }
    }
}
