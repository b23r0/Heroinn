use std::{io::*, net::SocketAddr};
use serde::{Serialize, Deserialize};

use crate::HeroinnProtocol;

#[derive(Serialize, Deserialize, Debug)]
struct BasePacket{
    clientid : String,
    data : String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HostInfo{
    ip : String,
    host_name : String,
    os : String ,
    remark : String
}

pub struct Message{
    id : u8,
    peer_addr : SocketAddr,
    proto : HeroinnProtocol,
    clientid : String,
    data : String
}

impl Message{
    pub fn new(peer_addr : SocketAddr , proto : HeroinnProtocol , buf : & [u8]) -> Result<Self>{
        let id = buf[0];
        
        let base_str = match String::from_utf8(buf[1..].to_vec()){
            Ok(p) => p,
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "parse base packet string error"))
        };

        let base : BasePacket = serde_json::from_str(&base_str)?;

        Ok(Self{
            id,
            peer_addr,
            proto,
            clientid: base.clientid,
            data: base.data,
        })
    }

    pub fn id(&self) -> u8{
        self.id
    }

    pub fn clientid(&self) -> String{
        self.clientid.clone()
    }

    pub fn to_host_info(&self) -> Result<HostInfo>{
        let packet : HostInfo = serde_json::from_str(&self.data)?;
        Ok(packet)
    }
}