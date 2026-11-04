use heroinn_util::{ConnectionInfo, HeroinnProtocol};

use crate::G_DNA;

pub fn master_configure() -> ConnectionInfo {
    // Force a runtime read so optimization cannot replace the mutable-on-disk
    // stamp with its original zero-filled constant value.
    let dna = std::hint::black_box(G_DNA);
    let size = u64::from_be_bytes(dna.size);

    // if not write the line , flag will be compiler optimized.
    log::trace!("flag : {:?}", dna.flag);

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
