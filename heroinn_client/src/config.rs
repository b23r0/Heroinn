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
    let size = u64::from_be_bytes(dna.size);

    if size == 0 {
        log::debug!("use default config");
        return ConnectionInfo {
            protocol: HeroinnProtocol::UDP.to_u8(),
            address: String::from("127.0.0.1:8000"),
            remark: String::from("Default"),
        };
    }

    let payload = match dna.payload() {
        Ok(payload) => payload,
        Err(e) => {
            log::error!("parse master connection info failed : {}", e);
            std::process::exit(0);
        }
    };

    match ConnectionInfo::parse(payload) {
        Ok(p) => p,
        Err(_) => {
            log::error!("parse master connection info failed");
            std::process::exit(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_stamp_starts_with_the_connection_flag() {
        assert_eq!(embedded_dna().flag, heroinn_util::dna::CONNECTION_INFO_FLAG);
    }
}
