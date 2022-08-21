pub mod protocol;
pub mod packet;
pub mod session;
pub mod rpc;
pub mod ftp;
pub mod msgbox;

pub const HEART_BEAT_TIME : u64 = 5;

#[derive(Debug,PartialEq)]
pub struct ConnectionInfo{
    pub flag : u64,
    pub protocol : u8,
    pub address_size : u16,
    pub address : [u8;255],
    pub remark_size : u16,
    pub remark : [u8;255],
}

#[derive(Debug,PartialEq)]
pub enum HeroinnClientMsgID{
    HostInfo,
    Heartbeat,
    SessionPacket,
    Unknow
}

impl HeroinnClientMsgID{
    pub fn to_u8(&self) -> u8{
        match self{
            HeroinnClientMsgID::HostInfo => 0x00,
            HeroinnClientMsgID::Heartbeat => 0x01,
            HeroinnClientMsgID::SessionPacket => 0x02, 
            HeroinnClientMsgID::Unknow => 0xff,
            
        }
    }

    pub fn from(v : u8) -> Self{
        match v{
            0x00 => HeroinnClientMsgID::HostInfo,
            0x01 => HeroinnClientMsgID::Heartbeat,
            0x02 => HeroinnClientMsgID::SessionPacket,
            _ => HeroinnClientMsgID::Unknow
        }
    }
}

#[derive(Debug,PartialEq)]
pub enum HeroinnServerCommandID{
    Shell,
    File,
    SessionPacket,
    Unknow
}

impl HeroinnServerCommandID{
    pub fn to_u8(&self) -> u8{
        match self{
            HeroinnServerCommandID::Shell => 0x00,
            HeroinnServerCommandID::File => 0x01,
            HeroinnServerCommandID::SessionPacket => 0x02, 
            HeroinnServerCommandID::Unknow => 0xff,
        }
    }

    pub fn from(v : u8) -> Self{
        match v{
            0x00 => HeroinnServerCommandID::Shell,
            0x01 => HeroinnServerCommandID::File,
            0x02 => HeroinnServerCommandID::SessionPacket,
            _ => HeroinnServerCommandID::Unknow
        }
    }
}

#[derive(Debug,Clone,PartialEq)]
pub enum HeroinnProtocol{
    TCP,
    Unknow
}

impl HeroinnProtocol{
    pub fn to_u8(&self) -> u8{
        match self{
            HeroinnProtocol::TCP => 0x00,
            HeroinnProtocol::Unknow => 0xff,
        }
    }

    pub fn from(v : u8) -> Self{
        match v{
            0x00 => HeroinnProtocol::TCP,
            _ => HeroinnProtocol::Unknow,
        }
    }
}

pub fn cur_timestamp_millis() -> u128{
    std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis()
    .try_into()
    .unwrap_or(0)
}

pub fn cur_timestamp_secs() -> u64{
    std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs()
    .try_into()
    .unwrap_or(0)
}