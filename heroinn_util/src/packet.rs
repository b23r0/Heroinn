//! Wire packets: the envelope ([`Message`]) plus the payload types sent over it.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::error::{Error, Result};
use crate::{session::SessionPacket, HeroinnProtocol};

/// Envelope shared by every message: a client id and an opaque JSON payload.
#[derive(Serialize, Deserialize, Debug)]
struct BasePacket {
    clientid: String,
    data: String,
}

/// Request asking the peer to open a reverse tunnel to `port` on its side.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TunnelRequest {
    pub port: u16,
}

impl TunnelRequest {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}

/// Host metadata reported by a client when it first connects.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HostInfo {
    pub ip: String,
    pub host_name: String,
    pub os: String,
    pub whoami: String,
    pub remark: String,
}

/// Periodic traffic counters reported by a client.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Heartbeat {
    pub time: u64,
    pub in_rate: u64,
    pub out_rate: u64,
}

/// A decoded envelope, tagged with the transport it arrived on.
pub struct Message {
    id: u8,
    peer_addr: SocketAddr,
    proto: HeroinnProtocol,
    clientid: String,
    data: String,
    data_length: usize,
}

impl Message {
    /// Decode a raw frame: one id byte followed by a JSON [`BasePacket`].
    pub fn new(peer_addr: SocketAddr, proto: HeroinnProtocol, buf: &[u8]) -> Result<Self> {
        let id = *buf.first().ok_or(Error::Malformed("empty frame"))?;

        let base_str = std::str::from_utf8(&buf[1..])
            .map_err(|_| Error::Malformed("base packet is not utf-8"))?;

        let base: BasePacket = serde_json::from_str(base_str)?;

        Ok(Self {
            id,
            peer_addr,
            proto,
            clientid: base.clientid,
            data: base.data,
            data_length: buf.len(),
        })
    }

    /// Encode a frame carrying `data` under message id `id`.
    pub fn build<T: Serialize>(id: u8, clientid: &str, data: T) -> Result<Vec<u8>> {
        let mut ret = vec![id];

        let base = BasePacket {
            clientid: clientid.to_string(),
            data: serde_json::to_string(&data)?,
        };

        ret.extend_from_slice(serde_json::to_string(&base)?.as_bytes());

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

    pub fn clientid(&self) -> &str {
        &self.clientid
    }

    /// Total size of the frame this message was decoded from.
    pub fn length(&self) -> usize {
        self.data_length
    }

    pub fn parse_hostinfo(&self) -> Result<HostInfo> {
        Ok(serde_json::from_str(&self.data)?)
    }

    pub fn parse_heartbeat(&self) -> Result<Heartbeat> {
        Ok(serde_json::from_str(&self.data)?)
    }

    pub fn parse_sessionpacket(&self) -> Result<SessionPacket> {
        Ok(serde_json::from_str(&self.data)?)
    }
}
