//! Reverse SOCKS5 protocol shared by the server, client and GUI sidecar.
//!
//! The public wire additions in this module are additive. SOCKS5 listeners
//! speak RFC 1928 directly to local applications; Heroinn control messages
//! only carry stream ids, sanitized targets and display state.

use std::fmt;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::protocol::frame::read_frame;

pub const SOCKS5_VERSION: u8 = 0x05;
pub const SOCKS5_AUTH_NONE: u8 = 0x00;
pub const SOCKS5_AUTH_NO_ACCEPTABLE: u8 = 0xff;
pub const SOCKS5_CMD_CONNECT: u8 = 0x01;
pub const SOCKS5_CMD_BIND: u8 = 0x02;
pub const SOCKS5_CMD_UDP_ASSOCIATE: u8 = 0x03;
pub const SOCKS5_REP_SUCCEEDED: u8 = 0x00;
pub const SOCKS5_REP_GENERAL_FAILURE: u8 = 0x01;
pub const SOCKS5_REP_HOST_UNREACHABLE: u8 = 0x04;
pub const SOCKS5_REP_CONNECTION_REFUSED: u8 = 0x05;
pub const SOCKS5_REP_COMMAND_NOT_SUPPORTED: u8 = 0x07;
pub const SOCKS5_REP_ADDRESS_TYPE_NOT_SUPPORTED: u8 = 0x08;

pub const SOCKS5_MAX_CONCURRENCY: usize = 64;
pub const SOCKS5_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
pub const SOCKS5_TARGET_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub const SOCKS5_TUNNEL_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub const SOCKS5_MAX_TUNNEL_CHUNK: usize = 64 * 1024;
pub const SOCKS5_TUNNEL_MAGIC: [u8; 8] = *b"HRNSOCKS";
/// Maximum application bytes read for one SOCKS5 data frame.
///
/// Reads use the full frame limit even though a 64 KiB frame is carried by
/// two Noise records (60 KiB plus the remainder). Splitting the read into
/// two sub-record frames would double the number of aggregate
/// rate-limiter charges per 64 KiB, which measurably halves throughput on
/// Windows where short waits are timer-bound.
pub const SOCKS5_TUNNEL_PUMP_CHUNK: usize = SOCKS5_MAX_TUNNEL_CHUNK;
const _: () = assert!(SOCKS5_TUNNEL_PUMP_CHUNK <= SOCKS5_MAX_TUNNEL_CHUNK);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Socks5Address {
    Domain(String),
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
}

impl Socks5Address {
    pub fn parse_host(host: &str) -> Result<Self> {
        let host = host.trim();
        if host.is_empty() {
            return Err(Error::Malformed("empty SOCKS5 target host"));
        }
        if host.len() > u8::MAX as usize {
            return Err(Error::Malformed("SOCKS5 target host is too long"));
        }
        if host
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        {
            return Err(Error::Malformed(
                "SOCKS5 target host contains whitespace or control bytes",
            ));
        }

        if let Some(inner) = host
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            let address = inner
                .parse::<Ipv6Addr>()
                .map_err(|_| Error::Malformed("invalid bracketed SOCKS5 IPv6 address"))?;
            return Ok(Self::Ipv6(address));
        }
        if let Ok(address) = host.parse::<Ipv4Addr>() {
            return Ok(Self::Ipv4(address));
        }
        if let Ok(address) = host.parse::<Ipv6Addr>() {
            return Ok(Self::Ipv6(address));
        }
        Ok(Self::Domain(host.to_string()))
    }

    pub fn host(&self) -> String {
        match self {
            Self::Domain(host) => host.clone(),
            Self::Ipv4(address) => address.to_string(),
            Self::Ipv6(address) => address.to_string(),
        }
    }

    pub fn authority(&self, port: u16) -> String {
        match self {
            Self::Domain(host) => format!("{host}:{port}"),
            Self::Ipv4(address) => format!("{address}:{port}"),
            Self::Ipv6(address) => format!("[{address}]:{port}"),
        }
    }

    pub fn resolve(&self, port: u16) -> Result<Vec<SocketAddr>> {
        let addresses = match self {
            Self::Domain(host) => (host.as_str(), port).to_socket_addrs()?.collect(),
            Self::Ipv4(address) => vec![SocketAddr::new(IpAddr::V4(*address), port)],
            Self::Ipv6(address) => vec![SocketAddr::new(IpAddr::V6(*address), port)],
        };
        if addresses.is_empty() {
            return Err(Error::NotFound(
                "SOCKS5 target resolved to no addresses".to_string(),
            ));
        }
        Ok(addresses)
    }

    fn wire_type(&self) -> u8 {
        match self {
            Self::Ipv4(_) => 0x01,
            Self::Domain(_) => 0x03,
            Self::Ipv6(_) => 0x04,
        }
    }

    fn write_wire(&self, writer: &mut impl Write) -> std::io::Result<()> {
        writer.write_all(&[self.wire_type()])?;
        match self {
            Self::Ipv4(address) => writer.write_all(&address.octets()),
            Self::Ipv6(address) => writer.write_all(&address.octets()),
            Self::Domain(host) => {
                writer.write_all(&[host.len() as u8])?;
                writer.write_all(host.as_bytes())
            }
        }
    }

    fn read_wire(reader: &mut impl Read, address_type: u8) -> Result<Self> {
        match address_type {
            0x01 => {
                let mut bytes = [0u8; 4];
                reader.read_exact(&mut bytes)?;
                Ok(Self::Ipv4(Ipv4Addr::from(bytes)))
            }
            0x03 => {
                let mut length = [0u8; 1];
                reader.read_exact(&mut length)?;
                let length = length[0] as usize;
                if length == 0 {
                    return Err(Error::Malformed("empty SOCKS5 domain"));
                }
                let mut bytes = vec![0u8; length];
                reader.read_exact(&mut bytes)?;
                let host = std::str::from_utf8(&bytes)
                    .map_err(|_| Error::Malformed("SOCKS5 domain is not UTF-8"))?;
                Self::parse_host(host)
            }
            0x04 => {
                let mut bytes = [0u8; 16];
                reader.read_exact(&mut bytes)?;
                Ok(Self::Ipv6(Ipv6Addr::from(bytes)))
            }
            _ => Err(Error::Malformed("unsupported SOCKS5 address type")),
        }
    }
}

impl fmt::Display for Socks5Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.host())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Socks5RequestCommand {
    Connect,
    Bind,
    UdpAssociate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Socks5Request {
    pub command: Socks5RequestCommand,
    pub target: Socks5Address,
    pub port: u16,
}

impl Socks5Request {
    pub fn read_from(reader: &mut impl Read) -> Result<Self> {
        let mut header = [0u8; 4];
        reader.read_exact(&mut header)?;
        if header[0] != SOCKS5_VERSION {
            return Err(Error::Malformed("unsupported SOCKS5 request version"));
        }
        if header[2] != 0 {
            return Err(Error::Malformed("invalid SOCKS5 reserved byte"));
        }
        let command = match header[1] {
            SOCKS5_CMD_CONNECT => Socks5RequestCommand::Connect,
            SOCKS5_CMD_BIND => Socks5RequestCommand::Bind,
            SOCKS5_CMD_UDP_ASSOCIATE => Socks5RequestCommand::UdpAssociate,
            _ => return Err(Error::Malformed("unsupported SOCKS5 command")),
        };
        let target = Socks5Address::read_wire(reader, header[3])?;
        let mut port = [0u8; 2];
        reader.read_exact(&mut port)?;
        Ok(Self {
            command,
            target,
            port: u16::from_be_bytes(port),
        })
    }

    pub fn write_to(&self, writer: &mut impl Write) -> Result<()> {
        let command = match self.command {
            Socks5RequestCommand::Connect => SOCKS5_CMD_CONNECT,
            Socks5RequestCommand::Bind => SOCKS5_CMD_BIND,
            Socks5RequestCommand::UdpAssociate => SOCKS5_CMD_UDP_ASSOCIATE,
        };
        writer.write_all(&[SOCKS5_VERSION, command, 0])?;
        self.target.write_wire(writer)?;
        writer.write_all(&self.port.to_be_bytes())?;
        Ok(())
    }
}

/// Perform the SOCKS5 greeting and require the local NOAUTH method.
pub fn negotiate_noauth(reader: &mut impl Read, writer: &mut impl Write) -> Result<()> {
    let mut header = [0u8; 2];
    reader.read_exact(&mut header)?;
    if header[0] != SOCKS5_VERSION {
        return Err(Error::Malformed("unsupported SOCKS5 greeting version"));
    }
    let method_count = header[1] as usize;
    if method_count == 0 {
        return Err(Error::Malformed("empty SOCKS5 authentication methods"));
    }
    let mut methods = vec![0u8; method_count];
    reader.read_exact(&mut methods)?;
    if methods.contains(&SOCKS5_AUTH_NONE) {
        writer.write_all(&[SOCKS5_VERSION, SOCKS5_AUTH_NONE])?;
        writer.flush()?;
        Ok(())
    } else {
        writer.write_all(&[SOCKS5_VERSION, SOCKS5_AUTH_NO_ACCEPTABLE])?;
        writer.flush()?;
        Err(Error::Other(
            "SOCKS5 client did not offer NOAUTH".to_string(),
        ))
    }
}

pub fn write_socks5_reply(writer: &mut impl Write, reply: u8) -> Result<()> {
    writer.write_all(&[SOCKS5_VERSION, reply, 0, 0x01, 0, 0, 0, 0, 0, 0])?;
    writer.flush()?;
    Ok(())
}

pub fn read_socks5_reply(reader: &mut impl Read) -> Result<u8> {
    let mut header = [0u8; 4];
    reader.read_exact(&mut header)?;
    if header[0] != SOCKS5_VERSION {
        return Err(Error::Malformed("unsupported SOCKS5 reply version"));
    }
    match header[3] {
        0x01 => {
            let mut rest = [0u8; 6];
            reader.read_exact(&mut rest)?;
        }
        0x03 => {
            let mut length = [0u8; 1];
            reader.read_exact(&mut length)?;
            let mut rest = vec![0u8; length[0] as usize + 2];
            reader.read_exact(&mut rest)?;
        }
        0x04 => {
            let mut rest = [0u8; 18];
            reader.read_exact(&mut rest)?;
        }
        _ => return Err(Error::Malformed("invalid SOCKS5 reply address type")),
    }
    Ok(header[1])
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Socks5Target {
    pub host: String,
    pub port: u16,
}

impl Socks5Target {
    pub fn new(host: &str, port: u16) -> Result<Self> {
        if port == 0 {
            return Err(Error::Malformed("SOCKS5 target port must be non-zero"));
        }
        let address = Socks5Address::parse_host(host)?;
        Ok(Self {
            host: address.host(),
            port,
        })
    }

    pub fn address(&self) -> Result<Socks5Address> {
        Socks5Address::parse_host(&self.host)
    }

    pub fn authority(&self) -> String {
        self.address()
            .map(|address| address.authority(self.port))
            .unwrap_or_else(|_| format!("{}:{}", self.host, self.port))
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Socks5FailureCategory {
    TargetDns,
    TargetConnect,
    TunnelConnect,
    Timeout,
    Protocol,
    Closed,
    Rejected,
    Internal,
}

impl Socks5FailureCategory {
    pub fn socks_reply(self) -> u8 {
        match self {
            Self::TargetDns | Self::TargetConnect => SOCKS5_REP_HOST_UNREACHABLE,
            Self::TunnelConnect => SOCKS5_REP_GENERAL_FAILURE,
            Self::Timeout => SOCKS5_REP_HOST_UNREACHABLE,
            Self::Protocol => SOCKS5_REP_GENERAL_FAILURE,
            Self::Closed => SOCKS5_REP_GENERAL_FAILURE,
            Self::Rejected => SOCKS5_REP_CONNECTION_REFUSED,
            Self::Internal => SOCKS5_REP_GENERAL_FAILURE,
        }
    }
}

impl Socks5ClientEvent {
    pub fn session_id(&self) -> &str {
        match self {
            Self::OpenResult { session_id, .. }
            | Self::Closed { session_id, .. }
            | Self::ShutdownAck { session_id } => session_id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Socks5ServerOpen {
    pub session_id: String,
    pub stream_id: u64,
    pub target: Socks5Target,
    pub tunnel_port: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Socks5ServerCommand {
    Start { session_id: String },
    Open(Socks5ServerOpen),
    Shutdown { session_id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Socks5ClientEvent {
    OpenResult {
        session_id: String,
        stream_id: u64,
        accepted: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        category: Option<Socks5FailureCategory>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    Closed {
        session_id: String,
        stream_id: u64,
        bytes_to_target: u64,
        bytes_from_target: u64,
    },
    ShutdownAck {
        session_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Socks5SidecarReady {
    pub session_id: String,
    /// Empty until the user explicitly starts the SOCKS5 listener in the sidecar GUI.
    pub socks_address: String,
    pub tunnel_port: u16,
    pub proxy_chain: String,
    pub transport: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Socks5SidecarEvent {
    Ready(Socks5SidecarReady),
    Open {
        stream_id: u64,
        target: Socks5Target,
    },
    Stopped {
        reason: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Socks5SidecarCommand {
    OpenResult {
        stream_id: u64,
        accepted: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        category: Option<Socks5FailureCategory>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    Closed {
        stream_id: u64,
        bytes_to_target: u64,
        bytes_from_target: u64,
    },
    Shutdown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Socks5TunnelFrame {
    Hello { stream_id: u64 },
    Data(Vec<u8>),
    Fin,
    Reset,
}

impl Socks5TunnelFrame {
    pub fn read_from(reader: &mut impl Read) -> Result<Self> {
        let payload = read_frame(reader)?;
        Self::from_payload(&payload)
    }

    pub fn from_payload(payload: &[u8]) -> Result<Self> {
        let Some(kind) = payload.first().copied() else {
            return Err(Error::Malformed("empty SOCKS5 tunnel frame"));
        };
        match kind {
            0x00 => {
                if payload.len() != 9 {
                    return Err(Error::Malformed("invalid SOCKS5 tunnel hello"));
                }
                let mut id = [0u8; 8];
                id.copy_from_slice(&payload[1..9]);
                Ok(Self::Hello {
                    stream_id: u64::from_be_bytes(id),
                })
            }
            0x01 => {
                if payload.len() > SOCKS5_MAX_TUNNEL_CHUNK + 1 {
                    return Err(Error::PacketTooLarge(payload.len()));
                }
                Ok(Self::Data(payload[1..].to_vec()))
            }
            0x02 if payload.len() == 1 => Ok(Self::Fin),
            0x03 if payload.len() == 1 => Ok(Self::Reset),
            _ => Err(Error::Malformed("unknown SOCKS5 tunnel frame")),
        }
    }

    pub fn write_to(&self, writer: &mut impl Write) -> Result<()> {
        crate::protocol::frame::write_frame(writer, &self.to_payload()?)
    }

    pub fn to_payload(&self) -> Result<Vec<u8>> {
        let payload = match self {
            Self::Hello { stream_id } => {
                let mut payload = Vec::with_capacity(9);
                payload.push(0x00);
                payload.extend_from_slice(&stream_id.to_be_bytes());
                payload
            }
            Self::Data(data) => {
                if data.len() > SOCKS5_MAX_TUNNEL_CHUNK {
                    return Err(Error::PacketTooLarge(data.len()));
                }
                let mut payload = Vec::with_capacity(data.len() + 1);
                payload.push(0x01);
                payload.extend_from_slice(data);
                payload
            }
            Self::Fin => vec![0x02],
            Self::Reset => vec![0x03],
        };
        Ok(payload)
    }
}

pub fn sanitize_detail(detail: &str) -> String {
    let detail = redact_userinfo(detail);
    let mut output = String::new();
    for character in detail.chars() {
        if output.len() >= 256 {
            break;
        }
        if character.is_control() {
            output.push(' ');
        } else {
            output.push(character);
        }
    }
    output.trim().to_string()
}

fn redact_userinfo(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(relative_scheme) = input[cursor..].find("://") {
        let authority_start = cursor + relative_scheme + 3;
        output.push_str(&input[cursor..authority_start]);
        let tail = &input[authority_start..];
        let authority_end = tail
            .find(|character: char| {
                character.is_whitespace()
                    || matches!(
                        character,
                        '/' | '?' | '#' | ',' | ';' | '"' | '\'' | '<' | '>'
                    )
            })
            .unwrap_or(tail.len());
        if let Some(relative_at) = tail[..authority_end].rfind('@') {
            output.push_str("<redacted>@");
            cursor = authority_start + relative_at + 1;
        } else {
            cursor = authority_start;
        }
    }
    output.push_str(&input[cursor..]);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn request_bytes(command: u8, address: &[u8]) -> Vec<u8> {
        let mut bytes = vec![5, command, 0];
        bytes.extend_from_slice(address);
        bytes.extend_from_slice(&8080u16.to_be_bytes());
        bytes
    }

    #[test]
    fn parses_domain_ipv4_and_ipv6_connect_requests() {
        let domain = request_bytes(
            1,
            &[
                3, 11, b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c', b'o', b'm',
            ],
        );
        assert_eq!(
            Socks5Request::read_from(&mut Cursor::new(domain)).unwrap(),
            Socks5Request {
                command: Socks5RequestCommand::Connect,
                target: Socks5Address::Domain("example.com".to_string()),
                port: 8080
            }
        );

        let ipv4 = request_bytes(1, &[1, 127, 0, 0, 1]);
        assert_eq!(
            Socks5Request::read_from(&mut Cursor::new(ipv4))
                .unwrap()
                .target,
            Socks5Address::Ipv4(Ipv4Addr::LOCALHOST)
        );

        let ipv6 = request_bytes(1, &[4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(
            Socks5Request::read_from(&mut Cursor::new(ipv6))
                .unwrap()
                .target,
            Socks5Address::Ipv6(Ipv6Addr::LOCALHOST)
        );
    }

    #[test]
    fn rejects_bind_udp_and_invalid_headers() {
        assert_eq!(
            Socks5Request::read_from(&mut Cursor::new(request_bytes(2, &[1, 127, 0, 0, 1])))
                .unwrap()
                .command,
            Socks5RequestCommand::Bind
        );
        assert_eq!(
            Socks5Request::read_from(&mut Cursor::new(request_bytes(3, &[1, 127, 0, 0, 1])))
                .unwrap()
                .command,
            Socks5RequestCommand::UdpAssociate
        );

        let mut bad_version = request_bytes(1, &[1, 127, 0, 0, 1]);
        bad_version[0] = 4;
        assert!(Socks5Request::read_from(&mut Cursor::new(bad_version)).is_err());

        let mut bad_reserved = request_bytes(1, &[1, 127, 0, 0, 1]);
        bad_reserved[2] = 1;
        assert!(Socks5Request::read_from(&mut Cursor::new(bad_reserved)).is_err());

        let bad_atyp = request_bytes(1, &[9, 0, 0, 0, 0]);
        assert!(Socks5Request::read_from(&mut Cursor::new(bad_atyp)).is_err());
    }

    #[test]
    fn rejects_truncated_and_invalid_domains() {
        assert!(Socks5Request::read_from(&mut Cursor::new([5, 1, 0])).is_err());
        assert!(Socks5Request::read_from(&mut Cursor::new(request_bytes(1, &[3, 0]))).is_err());
        assert!(
            Socks5Request::read_from(&mut Cursor::new(request_bytes(
                1,
                &[3, 3, b'a', b' ', b'b']
            )))
            .is_err()
        );
    }

    #[test]
    fn negotiates_only_noauth_and_emits_protocol_reply() {
        let mut input = Cursor::new(vec![5, 2, 0, 2]);
        let mut output = Vec::new();
        negotiate_noauth(&mut input, &mut output).unwrap();
        assert_eq!(output, vec![5, 0]);

        let mut input = Cursor::new(vec![5, 2, 1, 2]);
        let mut output = Vec::new();
        assert!(negotiate_noauth(&mut input, &mut output).is_err());
        assert_eq!(output, vec![5, 0xff]);

        let mut reply = Vec::new();
        write_socks5_reply(&mut reply, SOCKS5_REP_COMMAND_NOT_SUPPORTED).unwrap();
        assert_eq!(read_socks5_reply(&mut Cursor::new(reply)).unwrap(), 0x07);
    }

    #[test]
    fn control_messages_round_trip_without_credentials() {
        let open = Socks5ServerOpen {
            session_id: "session".to_string(),
            stream_id: 7,
            target: Socks5Target::new("example.com", 443).unwrap(),
            tunnel_port: 21000,
        };
        let message = Socks5ServerCommand::Open(open);
        let encoded = serde_json::to_vec(&message).unwrap();
        assert_eq!(
            serde_json::from_slice::<Socks5ServerCommand>(&encoded).unwrap(),
            message
        );
        assert!(!String::from_utf8(encoded).unwrap().contains("password"));

        let old: Socks5ServerCommand =
            serde_json::from_str(r#"{"type":"start","session_id":"old"}"#).unwrap();
        assert_eq!(
            old,
            Socks5ServerCommand::Start {
                session_id: "old".to_string()
            }
        );
    }

    #[test]
    fn tunnel_frames_round_trip_and_reject_oversized_data() {
        let frames = [
            Socks5TunnelFrame::Hello { stream_id: 9 },
            Socks5TunnelFrame::Data(vec![1, 2, 3]),
            Socks5TunnelFrame::Fin,
            Socks5TunnelFrame::Reset,
        ];
        for frame in frames {
            let mut encoded = Vec::new();
            frame.write_to(&mut encoded).unwrap();
            assert_eq!(
                Socks5TunnelFrame::read_from(&mut Cursor::new(encoded)).unwrap(),
                frame
            );
        }

        let oversized = Socks5TunnelFrame::Data(vec![0u8; SOCKS5_MAX_TUNNEL_CHUNK + 1]);
        assert!(oversized.write_to(&mut Vec::new()).is_err());
    }

    #[test]
    fn target_validation_and_authority_handle_ipv6() {
        assert_eq!(
            Socks5Target::new("[::1]", 1080).unwrap().authority(),
            "[::1]:1080"
        );
        assert!(Socks5Target::new("", 1080).is_err());
        assert!(Socks5Target::new("bad host", 1080).is_err());
        assert!(Socks5Target::new("example.com", 0).is_err());
    }

    #[test]
    fn diagnostics_redact_url_userinfo_and_control_characters() {
        let detail =
            sanitize_detail("connect http://alice:top-secret@proxy.example:8080 failed\nretry");
        assert!(!detail.contains("alice"));
        assert!(!detail.contains("top-secret"));
        assert!(detail.contains("http://<redacted>@proxy.example:8080"));
        assert!(!detail.contains('\n'));
    }

    #[test]
    fn client_events_expose_their_session_id() {
        let event = Socks5ClientEvent::Closed {
            session_id: "session".to_string(),
            stream_id: 9,
            bytes_to_target: 1,
            bytes_from_target: 2,
        };
        assert_eq!(event.session_id(), "session");
    }
}
