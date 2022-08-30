use heroinn_util::{ConnectionInfo, HeroinnProtocol};

use crate::G_DNA;

pub fn master_configure() -> ConnectionInfo {
    let size = u64::from_be_bytes(G_DNA.size);

    // if not write the line , flag will be compiler optimized.
    log::trace!("flag : {:?}", G_DNA.flag);

    if size == 0 {
        log::debug!("use default config");
        return ConnectionInfo {
            protocol: HeroinnProtocol::UDP.to_u8(),
            address: String::from("127.0.0.1:8000"),
            remark: String::from("Default"),
        };
    }

    if size > 1024 {
        log::error!("parse master connection info data too long");
        std::process::exit(0);
    }

    

    match ConnectionInfo::parse(&G_DNA.data[..size as usize].to_vec()) {
        Ok(p) => p,
        Err(_) => {
            log::error!("parse master connection info faild");
            std::process::exit(0);
        }
    }
}
