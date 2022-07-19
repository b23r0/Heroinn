pub mod protocol;
pub mod packet;

#[derive(Debug,PartialEq)]
pub enum HeroinnClientMsgID{
    HostInfo,
    Heartbeat,
    Unknow
}

impl HeroinnClientMsgID{
    pub fn to_u8(&self) -> u8{
        match self{
            HeroinnClientMsgID::HostInfo => 0x00,
            HeroinnClientMsgID::Heartbeat => 0x01,
            HeroinnClientMsgID::Unknow => 0xff 
        }
    }

    pub fn from(v : u8) -> Self{
        match v{
            0x00 => HeroinnClientMsgID::HostInfo,
            0x01 => HeroinnClientMsgID::Heartbeat,
            _ => HeroinnClientMsgID::Unknow
        }
    }
}

#[derive(Debug,PartialEq)]
pub enum HeroinnServerCommandID{
    Shell,
    File,
    Unknow
}

impl HeroinnServerCommandID{
    pub fn to_u8(&self) -> u8{
        match self{
            HeroinnServerCommandID::Shell => 0x00,
            HeroinnServerCommandID::File => 0x01,
            HeroinnServerCommandID::Unknow => 0xff 
        }
    }

    pub fn from(v : u8) -> Self{
        match v{
            0x00 => HeroinnServerCommandID::Shell,
            0x01 => HeroinnServerCommandID::File,
            _ => HeroinnServerCommandID::Unknow
        }
    }
}

#[derive(Debug,Clone,PartialEq)]
pub enum HeroinnProtocol{
    TCP
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