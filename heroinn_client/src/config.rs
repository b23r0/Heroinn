use heroinn_util::{ConnectionInfo, HeroinnProtocol, SlaveDNA};

use crate::G_CONNECTION_INFO;

fn embedded_dna() -> SlaveDNA {
    // The generator edits these bytes after linking. A volatile runtime load
    // prevents the compiler from replacing them with the initial zero values.
    //
    // SAFETY: G_CONNECTION_INFO is a live immutable static for the whole
    // program. read_volatile requires a valid, properly aligned pointer.
    unsafe { std::ptr::read_volatile(std::ptr::addr_of!(G_CONNECTION_INFO)) }
}

pub fn master_configure() -> ConnectionInfo {
    // Keep the load opaque across optimization and LTO passes as well.
    let dna = std::hint::black_box(embedded_dna());
    match configure_from_dna(dna) {
        Ok(config) => config,
        Err(error) => {
            log::error!("{error}");
            std::process::exit(0);
        }
    }
}

fn configure_from_dna(dna: SlaveDNA) -> Result<ConnectionInfo, String> {
    let size = u64::from_be_bytes(dna.size);

    if size == 0 {
        log::debug!("use default config");
        return Ok(ConnectionInfo {
            protocol: HeroinnProtocol::UDP.to_u8(),
            address: String::from("127.0.0.1:8000"),
            remark: String::from("Default"),
            socks5_proxy: None,
        });
    }

    let payload = dna
        .payload()
        .map_err(|error| format!("parse master connection info failed : {error}"))?;

    ConnectionInfo::parse(payload).map_err(|_| "parse master connection info failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_stamp_starts_with_the_connection_flag() {
        assert_eq!(embedded_dna().flag, heroinn_util::dna::CONNECTION_INFO_FLAG);
    }

    #[test]
    fn zero_sized_stamp_uses_the_documented_default_configuration() {
        let dna = SlaveDNA {
            flag: heroinn_util::dna::CONNECTION_INFO_FLAG,
            size: [0; 8],
            data: [0; heroinn_util::CONNECTION_INFO_DATA_LEN],
        };
        let config = configure_from_dna(dna).unwrap();
        assert_eq!(config.protocol, HeroinnProtocol::UDP.to_u8());
        assert_eq!(config.address, "127.0.0.1:8000");
        assert_eq!(config.remark, "Default");
        assert!(config.socks5_proxy.is_none());
    }

    #[test]
    fn valid_stamp_round_trips_the_embedded_connection_info() {
        let expected = ConnectionInfo {
            protocol: HeroinnProtocol::TCP.to_u8(),
            address: "127.0.0.1:9000".to_string(),
            remark: "round-trip".to_string(),
            socks5_proxy: None,
        };
        let dna = SlaveDNA::new(&expected.serialize().unwrap()).unwrap();
        let actual = configure_from_dna(dna).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn oversized_stamp_is_rejected_with_a_diagnostic() {
        let mut dna = SlaveDNA {
            flag: heroinn_util::dna::CONNECTION_INFO_FLAG,
            size: ((heroinn_util::CONNECTION_INFO_DATA_LEN + 1) as u64).to_be_bytes(),
            data: [0; heroinn_util::CONNECTION_INFO_DATA_LEN],
        };
        dna.data[0] = b'{';
        let error = configure_from_dna(dna).unwrap_err();
        assert!(error.contains("parse master connection info failed"));
    }

    #[test]
    fn malformed_json_is_rejected_without_panicking() {
        let dna = SlaveDNA::new(b"{not-json").unwrap();
        let error = configure_from_dna(dna).unwrap_err();
        assert_eq!(error, "parse master connection info failed");
    }

    #[test]
    fn semantically_invalid_connection_info_is_rejected() {
        let payload = br#"{"protocol":2,"address":"127.0.0.1:9000","remark":"","socks5_proxy":{"host":"127.0.0.1","port":1080}}"#;
        let dna = SlaveDNA::new(payload).unwrap();
        let error = configure_from_dna(dna).unwrap_err();
        assert_eq!(error, "parse master connection info failed");
    }
}
