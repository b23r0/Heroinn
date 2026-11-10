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
pub mod outbound;
pub mod proxy;
pub mod tcp;
pub mod udp;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::HeroinnProtocol;
use crate::error::Result;
use crate::packet::Message;

use self::http::{WSConnection, WSServer};
pub use self::outbound::{
    OutboundConnector, OutboundReader, OutboundStream, OutboundWriter, ProxyChainContext,
    ProxyContext, RouteReport, TransportFactory,
};
pub use self::proxy::FrozenRoute;
use self::tcp::{TcpConnection, TcpServer};
use self::udp::{UDPConnection, UDPServer};

/// Sentinel written before a 2-byte port when a peer asks for a reverse tunnel.
static TUNNEL_FLAG: [u8; 4] = [0x38, 0x38, 0x38, 0x38];

/// Callback invoked for every decoded inbound message.
pub type MessageHandler = Arc<dyn Fn(Message) + Send + Sync>;
pub type DisconnectHandler = Arc<dyn Fn(SocketAddr, HeroinnProtocol) + Send + Sync>;

/// Outbound connection to a peer.
pub enum Transport {
    Tcp(Box<TcpConnection>),
    Http(Box<WSConnection>),
    Udp(UDPConnection),
}

impl Transport {
    /// Dial a peer over `protocol`.
    pub fn connect(protocol: &HeroinnProtocol, address: &str) -> Result<Self> {
        let factory = TransportFactory::system();
        Self::connect_with_factory(&factory, protocol, address)
    }

    /// Dial a peer explicitly without proxy discovery.
    pub fn connect_direct(protocol: &HeroinnProtocol, address: &str) -> Result<Self> {
        Ok(match protocol {
            HeroinnProtocol::TCP => Self::Tcp(Box::new(TcpConnection::connect_direct(address)?)),
            HeroinnProtocol::HTTP => Self::Http(Box::new(WSConnection::connect_direct(address)?)),
            HeroinnProtocol::UDP => Self::Udp(UDPConnection::connect(address)?),
        })
    }

    /// Dial a peer using one shared, injectable outbound factory.
    pub fn connect_with_factory(
        factory: &TransportFactory,
        protocol: &HeroinnProtocol,
        address: &str,
    ) -> Result<Self> {
        Ok(match protocol {
            HeroinnProtocol::TCP => Self::Tcp(Box::new(TcpConnection::connect_with_factory(
                factory, address,
            )?)),
            HeroinnProtocol::HTTP => Self::Http(Box::new(WSConnection::connect_with_factory(
                factory, address,
            )?)),
            HeroinnProtocol::UDP => Self::Udp(UDPConnection::connect(address)?),
        })
    }

    /// Dial `remote_addr` and ask it to tunnel to `server_local_port`.
    pub fn tunnel(
        protocol: &HeroinnProtocol,
        remote_addr: &str,
        server_local_port: u16,
    ) -> Result<Self> {
        let factory = TransportFactory::system();
        Self::tunnel_with_factory(&factory, protocol, remote_addr, server_local_port)
    }

    /// Dial a reverse tunnel explicitly without proxy discovery.
    pub fn tunnel_direct(
        protocol: &HeroinnProtocol,
        remote_addr: &str,
        server_local_port: u16,
    ) -> Result<Self> {
        Ok(match protocol {
            HeroinnProtocol::TCP => Self::Tcp(Box::new(TcpConnection::tunnel_direct(
                remote_addr,
                server_local_port,
            )?)),
            HeroinnProtocol::HTTP => Self::Http(Box::new(WSConnection::tunnel_direct(
                remote_addr,
                server_local_port,
            )?)),
            HeroinnProtocol::UDP => {
                Self::Udp(UDPConnection::tunnel(remote_addr, server_local_port)?)
            }
        })
    }

    /// Dial a reverse tunnel using one shared, injectable outbound factory.
    pub fn tunnel_with_factory(
        factory: &TransportFactory,
        protocol: &HeroinnProtocol,
        remote_addr: &str,
        server_local_port: u16,
    ) -> Result<Self> {
        Ok(match protocol {
            HeroinnProtocol::TCP => Self::Tcp(Box::new(TcpConnection::tunnel_with_factory(
                factory,
                remote_addr,
                server_local_port,
            )?)),
            HeroinnProtocol::HTTP => Self::Http(Box::new(WSConnection::tunnel_with_factory(
                factory,
                remote_addr,
                server_local_port,
            )?)),
            HeroinnProtocol::UDP => {
                Self::Udp(UDPConnection::tunnel(remote_addr, server_local_port)?)
            }
        })
    }

    pub fn tunnel_with_frozen_route(
        factory: &TransportFactory,
        protocol: &HeroinnProtocol,
        remote_addr: &str,
        server_local_port: u16,
        frozen: &self::proxy::FrozenRoute,
    ) -> Result<Self> {
        Ok(match protocol {
            HeroinnProtocol::TCP => Self::Tcp(Box::new(TcpConnection::tunnel_with_frozen_route(
                factory,
                remote_addr,
                server_local_port,
                frozen,
            )?)),
            HeroinnProtocol::HTTP => Self::Http(Box::new(WSConnection::tunnel_with_frozen_route(
                factory,
                remote_addr,
                server_local_port,
                frozen,
            )?)),
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

    /// Credential-free identity of the proxy used by this live connection.
    pub fn proxy_context(&self) -> Option<&ProxyContext> {
        match self {
            Self::Tcp(connection) => connection.proxy_context(),
            Self::Http(connection) => connection.proxy_context(),
            Self::Udp(_) => None,
        }
    }

    /// Credential-free route details for the successful connection.
    pub fn route_report(&self) -> Option<&RouteReport> {
        match self {
            Self::Tcp(connection) => Some(connection.route_report()),
            Self::Http(connection) => Some(connection.route_report()),
            Self::Udp(_) => None,
        }
    }

    pub fn frozen_route(&self) -> Option<&self::proxy::FrozenRoute> {
        match self {
            Self::Tcp(connection) => connection.frozen_route(),
            Self::Http(connection) => connection.frozen_route(),
            Self::Udp(_) => None,
        }
    }

    pub fn shutdown(&self, how: std::net::Shutdown) -> Result<()> {
        match self {
            Self::Tcp(connection) => connection.shutdown(how),
            Self::Http(connection) => connection.shutdown(how),
            Self::Udp(connection) => {
                let mut connection = connection.clone();
                connection.close();
                Ok(())
            }
        }
    }

    /// Observe a premature remote close without blocking.
    pub fn remote_closed(&self) -> Result<bool> {
        match self {
            Self::Tcp(connection) => connection.remote_closed(),
            Self::Http(connection) => connection.remote_closed(),
            Self::Udp(_) => Ok(false),
        }
    }

    /// Confirm a finished send-direction transfer was followed by a clean
    /// remote close. UDP and plain streams retain their existing best-effort
    /// completion semantics.
    pub fn wait_for_remote_close(&self, timeout: Duration) -> Result<()> {
        match self {
            Self::Tcp(connection) => connection.wait_for_remote_close(timeout),
            Self::Http(connection) => connection.wait_for_remote_close(timeout),
            Self::Udp(_) => Ok(()),
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
        Self::bind_with_disconnect(protocol, port, on_message, Arc::new(|_, _| {}))
    }

    pub fn bind_with_disconnect(
        protocol: &HeroinnProtocol,
        port: u16,
        on_message: MessageHandler,
        on_disconnect: DisconnectHandler,
    ) -> Result<Self> {
        let address = format!("0.0.0.0:{}", port);

        Ok(match protocol {
            HeroinnProtocol::TCP => Self::Tcp(TcpServer::new_with_disconnect(
                &address,
                on_message,
                on_disconnect,
            )?),
            HeroinnProtocol::HTTP => Self::Http(WSServer::new_with_disconnect(
                &address,
                on_message,
                on_disconnect,
            )?),
            HeroinnProtocol::UDP => Self::Udp(UDPServer::new_with_disconnect(
                &address,
                on_message,
                on_disconnect,
            )?),
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
