use heroinn_util::{ConnectionInfo, HeroinnProtocol};

use crate::G_CONNECTION_INFO;

pub fn master_configure() -> ConnectionInfo{

    let size = u64::from_be_bytes(G_CONNECTION_INFO.size);

    //log::debug!("master configure : {:?} [{}]" , G_CONNECTION_INFO , size);

    if size == 0{
        return ConnectionInfo{
            protocol : HeroinnProtocol::HTTP.to_u8(),
            address : String::from("127.0.0.1:8000"),
            remark : String::from("Default"),
        };
    }

    if size > 1024{
        log::error!("parse master connection info data too long");
        std::process::exit(0);
    }

    let config = match ConnectionInfo::parse(&G_CONNECTION_INFO.data[..size as usize].to_vec()){
        Ok(p) => p,
        Err(_) => {
            log::error!("parse master connection info faild");
            std::process::exit(0);
        },
    };

    config
}