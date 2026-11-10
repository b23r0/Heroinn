//! Wire packets: the envelope ([`Message`]) plus the payload types sent over it.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::error::{Error, Result};
use crate::{HeroinnProtocol, session::SessionPacket};

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
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct ProxyDisplay {
    pub protocol: String,
    pub address: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct HostCapabilities {
    #[serde(default)]
    pub socks5_reverse: bool,
}

/// Host metadata reported by a client when it first connects.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct HostInfo {
    pub ip: String,
    pub host_name: String,
    pub os: String,
    /// Canonical operating-system id reported by `std::env::consts::OS`.
    ///
    /// This is optional so packets produced by older clients remain valid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_id: Option<String>,
    pub whoami: String,
    pub remark: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_proxy: Option<ProxyDisplay>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socks5_proxy: Option<ProxyDisplay>,
    /// Optional additive capabilities reported by current clients.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<HostCapabilities>,
}

/// Periodic traffic counters reported by a client.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Heartbeat {
    pub time: u64,
    pub in_rate: u64,
    pub out_rate: u64,
}

/// Aggregate client transfer-rate limit, in bytes per second.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TransferRate {
    pub bytes_per_sec: u64,
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

    pub fn parse_transfer_rate(&self) -> Result<TransferRate> {
        Ok(serde_json::from_str(&self.data)?)
    }

    pub fn parse_socks5_server_command(&self) -> Result<crate::socks5::Socks5ServerCommand> {
        Ok(serde_json::from_str(&self.data)?)
    }

    pub fn parse_socks5_client_event(&self) -> Result<crate::socks5::Socks5ClientEvent> {
        Ok(serde_json::from_str(&self.data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HeroinnProtocol, HeroinnServerCommandID};

    #[test]
    fn transfer_rate_command_round_trips() {
        let peer = "127.0.0.1:9001".parse().unwrap();
        let setting = TransferRate {
            bytes_per_sec: 512 * 1024,
        };

        let buf = Message::build(
            HeroinnServerCommandID::SetTransferRate.to_u8(),
            "client-1",
            setting.clone(),
        )
        .unwrap();

        assert_eq!(buf[0], 0x03);

        let msg = Message::new(peer, HeroinnProtocol::TCP, &buf).unwrap();
        assert_eq!(msg.clientid(), "client-1");
        assert_eq!(msg.parse_transfer_rate().unwrap(), setting);
    }

    #[test]
    fn hostinfo_old_packets_default_new_proxy_fields() {
        let info: HostInfo = serde_json::from_str(
            r#"{"ip":"127.0.0.1","host_name":"host","os":"linux","whoami":"user","remark":"old"}"#,
        )
        .unwrap();
        assert_eq!(info.system_proxy, None);
        assert_eq!(info.socks5_proxy, None);
    }

    #[test]
    fn hostinfo_proxy_fields_round_trip_without_credentials() {
        let info = HostInfo {
            ip: "127.0.0.1".to_string(),
            host_name: "host".to_string(),
            os: "linux".to_string(),
            platform_id: Some("linux".to_string()),
            whoami: "user".to_string(),
            remark: "new".to_string(),
            system_proxy: Some(ProxyDisplay {
                protocol: "HTTPS".to_string(),
                address: "proxy.example:8443".to_string(),
                source: Some("system".to_string()),
            }),
            socks5_proxy: Some(ProxyDisplay {
                protocol: "SOCKS5".to_string(),
                address: "[::1]:1080".to_string(),
                source: Some("specified".to_string()),
            }),
            capabilities: Some(HostCapabilities {
                socks5_reverse: true,
            }),
        };
        let payload = serde_json::to_vec(&info).unwrap();
        let parsed: HostInfo = serde_json::from_slice(&payload).unwrap();
        assert_eq!(parsed, info);
        let text = String::from_utf8(payload.clone()).unwrap();
        assert!(!text.contains("password"));
        assert!(!text.contains("username"));

        #[derive(Deserialize)]
        struct LegacyHostInfo {
            ip: String,
            host_name: String,
            os: String,
            whoami: String,
            remark: String,
        }
        let legacy: LegacyHostInfo = serde_json::from_slice(&payload).unwrap();
        assert_eq!(legacy.ip, "127.0.0.1");
        assert_eq!(legacy.host_name, "host");
        assert_eq!(legacy.os, "linux");
        assert_eq!(legacy.whoami, "user");
        assert_eq!(legacy.remark, "new");
    }

    #[test]
    fn hostinfo_old_packets_default_platform_id() {
        let info: HostInfo = serde_json::from_str(
            r#"{"ip":"127.0.0.1","host_name":"host","os":"Linux","whoami":"user","remark":"old"}"#,
        )
        .unwrap();
        assert_eq!(info.platform_id, None);
    }

    #[test]
    fn hostinfo_platform_id_is_optional_on_the_wire() {
        let info = HostInfo {
            ip: "127.0.0.1".to_string(),
            host_name: "host".to_string(),
            os: "Microsoft Windows 11".to_string(),
            platform_id: None,
            whoami: "user".to_string(),
            remark: "legacy-platform".to_string(),
            system_proxy: None,
            socks5_proxy: None,
            capabilities: None,
        };
        let payload = serde_json::to_string(&info).unwrap();
        assert!(!payload.contains("platform_id"));
    }

    #[test]
    fn hostinfo_old_packets_default_capabilities_and_old_servers_ignore_them() {
        let info = HostInfo {
            ip: "127.0.0.1".to_string(),
            host_name: "host".to_string(),
            os: "linux".to_string(),
            platform_id: Some("linux".to_string()),
            whoami: "user".to_string(),
            remark: "new".to_string(),
            system_proxy: None,
            socks5_proxy: None,
            capabilities: Some(HostCapabilities {
                socks5_reverse: true,
            }),
        };
        let payload = serde_json::to_vec(&info).unwrap();
        let old: HostInfo = serde_json::from_str(
            r#"{"ip":"127.0.0.1","host_name":"old","os":"linux","whoami":"u","remark":"old"}"#,
        )
        .unwrap();
        assert_eq!(old.capabilities, None);

        #[derive(Deserialize)]
        struct LegacyHostInfo {
            ip: String,
            host_name: String,
            os: String,
            whoami: String,
            remark: String,
        }
        let legacy: LegacyHostInfo = serde_json::from_slice(&payload).unwrap();
        let _ = (&legacy.ip, &legacy.os, &legacy.whoami, &legacy.remark);
        assert_eq!(legacy.host_name, "host");
        assert!(
            String::from_utf8(payload)
                .unwrap()
                .contains("socks5_reverse")
        );
    }
}
