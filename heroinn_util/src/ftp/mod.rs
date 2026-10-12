//! File-transfer protocol types.

pub mod method;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Kind of an [`FTPPacket`], matching the byte written on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FTPId {
    RPC,
    Get,
    Put,
    Close,
}

impl From<FTPId> for u8 {
    fn from(id: FTPId) -> Self {
        match id {
            FTPId::RPC => 0x01,
            FTPId::Get => 0x02,
            FTPId::Put => 0x03,
            FTPId::Close => 0x04,
        }
    }
}

impl TryFrom<u8> for FTPId {
    type Error = Error;

    fn try_from(id: u8) -> Result<Self> {
        match id {
            0x01 => Ok(FTPId::RPC),
            0x02 => Ok(FTPId::Get),
            0x03 => Ok(FTPId::Put),
            0x04 => Ok(FTPId::Close),
            other => Err(Error::UnknownFtpId(other)),
        }
    }
}

impl FTPId {
    /// Byte value written on the wire.
    pub fn to_u8(&self) -> u8 {
        (*self).into()
    }
}

/// Envelope for a single file-transfer message.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FTPPacket {
    pub id: u8,
    pub data: Vec<u8>,
}

/// One directory entry.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub typ: String,
    pub last_modified: String,
}

/// Header preceding a file download.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FTPGetHeader {
    pub path: String,
    pub start_pos: u64,
}

impl FTPGetHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}

/// Header preceding a file upload.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FTPPutHeader {
    pub path: String,
    pub total_size: u64,
    pub start_pos: u64,
}

impl FTPPutHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}

impl FTPPacket {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    pub fn id(&self) -> Result<FTPId> {
        FTPId::try_from(self.id)
    }
}

impl FileInfo {
    pub fn serialize(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn parse(data: &str) -> Result<Self> {
        Ok(serde_json::from_str(data)?)
    }
}

/// A directory listing.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DirectoryInfo {
    pub path: String,
    pub detail: Vec<FileInfo>,
}
