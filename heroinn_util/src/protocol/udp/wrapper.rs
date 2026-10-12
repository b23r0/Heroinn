//! Thin blocking facade over the async `rust-raknet` API.
//!
//! A single shared Tokio runtime drives every RakNet socket so callers can keep
//! the rest of the transport layer synchronous.

use std::net::SocketAddr;
use std::sync::{Arc, LazyLock};

use rust_raknet::*;

use crate::error::{Error, Result};

/// Shared multi-threaded Tokio runtime driving the reliable-UDP (RakNet) transport.
static RT: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

/// Accepting side of a RakNet socket.
pub struct RUdpServer {
    server: RaknetListener,
}

/// Connected side of a RakNet socket.
pub struct RUdpClient {
    client: Arc<RaknetSocket>,
}

impl RUdpClient {
    /// Connect to `address`, giving up after three seconds.
    pub fn new(address: String) -> Result<Self> {
        let address: SocketAddr = address
            .parse()
            .map_err(|e| Error::Other(format!("address format error : {:?}", e)))?;

        let client = RT
            .block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    RaknetSocket::connect(&address),
                )
                .await
            })
            .map_err(|e| Error::Other(format!("{:?}", e)))?
            .map_err(|e| Error::Other(format!("{:?}", e)))?;

        Ok(Self {
            client: Arc::new(client),
        })
    }

    pub fn send(&self, buf: Vec<u8>) -> Result<()> {
        RT.block_on(self.client.send(&buf, Reliability::ReliableOrdered))
            .map_err(|e| Error::Other(format!("{:?}", e)))?;

        RT.block_on(self.client.flush())
            .map_err(|e| Error::Other(format!("{:?}", e)))?;

        Ok(())
    }

    pub fn recv(&self) -> Result<Vec<u8>> {
        RT.block_on(self.client.recv())
            .map_err(|e| Error::Other(format!("{:?}", e)))
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        Ok(self.client.peer_addr().unwrap())
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.client.local_addr().unwrap())
    }

    pub fn close(&self) -> Result<()> {
        RT.block_on(self.client.close())
            .map_err(|e| Error::Other(format!("{:?}", e)))
    }
}

impl Clone for RUdpClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

impl RUdpServer {
    /// Bind a RakNet listener to `address` and start listening.
    pub fn new(address: &str) -> Result<Self> {
        let address: SocketAddr = address
            .parse()
            .map_err(|e| Error::Other(format!("address format error : {:?}", e)))?;

        let mut server = RT
            .block_on(RaknetListener::bind(&address))
            .map_err(|e| Error::Other(format!("{:?}", e)))?;

        RT.block_on(server.listen());

        Ok(Self { server })
    }

    /// Accept one peer, giving up after `timeout_mills`.
    pub fn accept(&mut self, timeout_mills: u64) -> Result<RUdpClient> {
        let client = RT
            .block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_millis(timeout_mills),
                    self.server.accept(),
                )
                .await
            })
            .map_err(|_| Error::Timeout)?
            .map_err(|e| Error::Other(format!("{:?}", e)))?;

        Ok(RUdpClient {
            client: Arc::new(client),
        })
    }

    pub fn local_addr(&mut self) -> Result<SocketAddr> {
        Ok(self.server.local_addr().unwrap())
    }

    pub fn close(&mut self) -> Result<()> {
        RT.block_on(self.server.close())
            .map_err(|e| Error::Other(format!("{:?}", e)))
    }

    pub fn set_motd(&mut self, motd: String) {
        RT.block_on(self.server.set_full_motd(motd)).unwrap();
    }
}
