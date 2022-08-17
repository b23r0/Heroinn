use std::io::*;
pub mod method;

use serde::{Serialize, Deserialize};

pub enum FTPId{
    RPC,
    Close,
    Unknown
}

impl FTPId{
    pub fn to_u8(&self) -> u8{
        match self{
            FTPId::RPC => 0x01,
            FTPId::Close => 0x02,
            FTPId::Unknown => 0xff,
            
        }
    }

    pub fn from(id : u8) -> Self{
        match id{
            0x01 => FTPId::RPC,
            0x02 => FTPId::Close,
            _ => FTPId::Unknown
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FTPPacket{
    pub id : u8,
    pub data : Vec<u8>
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FileInfo{
    pub name : String,
    pub size : u64,
    pub typ : String,
    pub last_modified : String,
}

impl FTPPacket{
    pub fn parse(data : &Vec<u8>) -> Result<Self>{
        let ret : FTPPacket = serde_json::from_slice(data)?;
        Ok(ret)
    }

    pub fn serialize(&self) -> Result<Vec<u8>>{
        match serde_json::to_vec(self){
            Ok(p) => Ok(p),
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "serilize ftp packet faild"))
        }
    }

    pub fn id(&self) -> FTPId{
        FTPId::from(self.id)
    }
}

impl FileInfo{
    pub fn serialize(&self) -> Result<String>{
        match serde_json::to_string(&self){
            Ok(p) => Ok(p),
            Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("serilize disk info faild : {}" , e))),
        }
    }

    pub fn parse(data : &String) -> Result<Self>{
        let ret : FileInfo = serde_json::from_str(data)?;
        Ok(ret)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DirectoryInfo{
    pub path : String,     
    pub detail : Vec<FileInfo>
}