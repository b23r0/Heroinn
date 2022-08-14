use std::io::*;
pub mod method;

use serde::{Serialize, Deserialize};

pub enum FTPId{
    GetDirectory,
    Unknow
}

impl FTPId{
    fn _to_u8(&self) -> u8{
        match self{
            FTPId::GetDirectory => 0x01,
            FTPId::Unknow => 0xff,
        }
    }

    fn _from(id : u8) -> Self{
        match id{
            0x01 => FTPId::GetDirectory,
            _ => FTPId::Unknow
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FTPPacket{
    id : u8,
    data : String
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FileInfo{
    pub name : String,
    pub size : u64,
    pub typ : String,
    pub last_modified : String,
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