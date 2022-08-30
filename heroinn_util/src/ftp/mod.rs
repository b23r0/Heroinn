use std::io::*;
pub mod method;

use serde::{Deserialize, Serialize};

pub enum FTPId {
    RPC,
    Get,
    Put,
    Close,
    Unknown,
}

impl FTPId {
    pub fn to_u8(&self) -> u8 {
        match self {
            FTPId::RPC => 0x01,
            FTPId::Get => 0x02,
            FTPId::Put => 0x03,
            FTPId::Close => 0x04,
            FTPId::Unknown => 0xff,
        }
    }

    pub fn from(id: u8) -> Self {
        match id {
            0x01 => FTPId::RPC,
            0x02 => FTPId::Get,
            0x03 => FTPId::Put,
            0x04 => FTPId::Close,
            _ => FTPId::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FTPPacket {
    pub id: u8,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub typ: String,
    pub last_modified: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FTPGetHeader {
    pub path: String,
    pub start_pos: u64,
}

impl FTPGetHeader {
    pub fn parse(data: &Vec<u8>) -> Result<Self> {
        let ret: FTPGetHeader = serde_json::from_slice(data)?;
        Ok(ret)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match serde_json::to_vec(self) {
            Ok(p) => Ok(p),
            Err(_) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "serilize FTPGetHeader packet faild",
                ))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FTPPutHeader {
    pub path: String,
    pub total_size: u64,
    pub start_pos: u64,
}

impl FTPPutHeader {
    pub fn parse(data: &Vec<u8>) -> Result<Self> {
        let ret: FTPPutHeader = serde_json::from_slice(data)?;
        Ok(ret)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match serde_json::to_vec(self) {
            Ok(p) => Ok(p),
            Err(_) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "serilize FTPPutHeader packet faild",
                ))
            }
        }
    }
}

impl FTPPacket {
    pub fn parse(data: &Vec<u8>) -> Result<Self> {
        let ret: FTPPacket = serde_json::from_slice(data)?;
        Ok(ret)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match serde_json::to_vec(self) {
            Ok(p) => Ok(p),
            Err(_) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "serilize FTPPacket packet faild",
                ))
            }
        }
    }

    pub fn id(&self) -> FTPId {
        FTPId::from(self.id)
    }
}

impl FileInfo {
    pub fn serialize(&self) -> Result<String> {
        match serde_json::to_string(&self) {
            Ok(p) => Ok(p),
            Err(e) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("serilize disk info faild : {}", e),
                ))
            }
        }
    }

    pub fn parse(data: &String) -> Result<Self> {
        let ret: FileInfo = serde_json::from_str(data)?;
        Ok(ret)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DirectoryInfo {
    pub path: String,
    pub detail: Vec<FileInfo>,
}
