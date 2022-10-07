use lazy_static::*;
use rust_raknet::*;
use std::io::*;
use std::{net::SocketAddr, sync::Arc};

lazy_static! {
    static ref RT: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
}

pub struct RUdpServer {
    server: RaknetListener,
}

pub struct RUdpClient {
    client: Arc<RaknetSocket>,
}

impl RUdpClient {
    pub fn new(address: String) -> Result<RUdpClient> {
        let address: SocketAddr = match address.parse() {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{:?}", e),
                ));
            }
        };
        let client = match match RT.block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(3),
                RaknetSocket::connect(&address),
            )
            .await
        }) {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{:?}", e),
                ));
            }
        } {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{:?}", e),
                ));
            }
        };

        Ok(RUdpClient {
            client: Arc::new(client),
        })
    }

    pub fn send(&self, buf: Vec<u8>) -> Result<()> {
        match RT.block_on(self.client.send(&buf, Reliability::ReliableOrdered)) {
            Ok(_) => {}
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{:?}", e),
                ));
            }
        };
        match RT.block_on(self.client.flush()) {
            Ok(_) => {}
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    format!("{:?}", e),
                ));
            }
        };
        Ok(())
    }

    pub fn recv(&self) -> Result<Vec<u8>> {
        let buf = match RT.block_on(self.client.recv()) {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{:?}", e),
                ));
            }
        };

        Ok(buf)
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        Ok(self.client.peer_addr().unwrap())
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.client.local_addr().unwrap())
    }

    pub fn close(&self) -> Result<()> {
        RT.block_on(self.client.close()).unwrap();
        Ok(())
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
    pub fn new(address: &String) -> Result<RUdpServer> {
        let address: SocketAddr = match address.parse() {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{:?}", e),
                ));
            }
        };
        let mut server = match RT.block_on(RaknetListener::bind(&address)) {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{:?}", e),
                ))
            }
        };

        RT.block_on(server.listen());

        Ok(RUdpServer { server })
    }

    pub fn accept(&mut self, timeout_mills: u64) -> Result<RUdpClient> {
        let client = match match RT.block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_millis(timeout_mills),
                self.server.accept(),
            )
            .await
        }) {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("{:?}", e),
                ));
            }
        } {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{:?}", e),
                ));
            }
        };
        Ok(RUdpClient {
            client: Arc::new(client),
        })
    }

    pub fn local_addr(&mut self) -> Result<SocketAddr> {
        Ok(self.server.local_addr().unwrap())
    }

    pub fn close(&mut self) -> Result<()> {
        RT.block_on(self.server.close()).unwrap();
        Ok(())
    }

    pub fn setmotd(&mut self, motd: String) {
        self.server.set_full_motd(motd).unwrap();
    }
}
