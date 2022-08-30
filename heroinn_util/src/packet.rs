use serde::{Deserialize, Serialize};
use std::{io::*, net::SocketAddr};

use crate::{session::SessionPacket, HeroinnProtocol};

#[derive(Serialize, Deserialize, Debug)]
struct BasePacket {
    clientid: String,
    data: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TunnelRequest {
    pub port: u16,
}

impl TunnelRequest {
    pub fn parse(data: &Vec<u8>) -> Result<Self> {
        let ret: TunnelRequest = serde_json::from_slice(data)?;
        Ok(ret)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match serde_json::to_vec(self) {
            Ok(p) => Ok(p),
            Err(_) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "serilize TunnelRequest packet faild",
                ))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HostInfo {
    pub ip: String,
    pub host_name: String,
    pub os: String,
    pub whoami: String,
    pub remark: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Heartbeat {
    pub time: u64,
    pub in_rate: u64,
    pub out_rate: u64,
}

pub struct Message {
    id: u8,
    peer_addr: SocketAddr,
    proto: HeroinnProtocol,
    clientid: String,
    data: String,
    data_length: usize,
}

impl Message {
    pub fn new(peer_addr: SocketAddr, proto: HeroinnProtocol, buf: &[u8]) -> Result<Self> {
        let id = buf[0];

        let base_str = match String::from_utf8(buf[1..].to_vec()) {
            Ok(p) => p,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "parse base packet string error",
                ))
            }
        };

        let base: BasePacket = serde_json::from_str(&base_str)?;

        Ok(Self {
            id,
            peer_addr,
            proto,
            clientid: base.clientid,
            data: base.data,
            data_length: buf.len(),
        })
    }

    pub fn build<T: Serialize>(id: u8, clientid: &String, data: T) -> Result<Vec<u8>> {
        let mut ret = vec![];
        ret.push(id);

        let data = serde_json::to_string(&data)?;

        let base = BasePacket {
            clientid: clientid.clone(),
            data,
        };

        let data = serde_json::to_string(&base)?;
        ret.append(&mut data.as_bytes().to_vec());

        Ok(ret)
    }

    pub fn id(&self) -> u8 {
        self.id
    }

    pub fn proto(&self) -> HeroinnProtocol {
        self.proto.clone()
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    pub fn clientid(&self) -> String {
        self.clientid.clone()
    }

    pub fn parser_hostinfo(&self) -> Result<HostInfo> {
        let packet: HostInfo = serde_json::from_str(&self.data)?;
        Ok(packet)
    }

    pub fn parser_heartbeat(&self) -> Result<Heartbeat> {
        let packet: Heartbeat = serde_json::from_str(&self.data)?;
        Ok(packet)
    }

    pub fn parser_sessionpacket(&self) -> Result<SessionPacket> {
        let packet: SessionPacket = serde_json::from_str(&self.data)?;
        Ok(packet)
    }

    pub fn length(&self) -> usize {
        self.data_length
    }
}
