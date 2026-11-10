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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ftp_ids_round_trip_and_unknown_ids_fail() {
        for (id, byte) in [
            (FTPId::RPC, 0x01),
            (FTPId::Get, 0x02),
            (FTPId::Put, 0x03),
            (FTPId::Close, 0x04),
        ] {
            assert_eq!(id.to_u8(), byte);
            assert_eq!(FTPId::try_from(byte).unwrap(), id);
        }
        assert!(matches!(
            FTPId::try_from(0xff),
            Err(Error::UnknownFtpId(0xff))
        ));
    }

    #[test]
    fn packets_headers_and_file_info_round_trip() {
        let packet = FTPPacket {
            id: FTPId::Put.to_u8(),
            data: b"packet".to_vec(),
        };
        let decoded = FTPPacket::parse(&packet.serialize().unwrap()).unwrap();
        assert_eq!(decoded.id, 0x03);
        assert_eq!(decoded.data, b"packet");
        assert_eq!(decoded.id().unwrap(), FTPId::Put);

        for packet in [
            FTPPacket {
                id: 1,
                data: Vec::new(),
            },
            FTPPacket {
                id: 2,
                data: vec![0xff],
            },
        ] {
            let decoded = FTPPacket::parse(&packet.serialize().unwrap()).unwrap();
            assert_eq!(decoded.id, packet.id);
            assert_eq!(decoded.data, packet.data);
        }

        let get = FTPGetHeader {
            path: "/tmp/file".to_string(),
            start_pos: 42,
        };
        assert_eq!(
            FTPGetHeader::parse(&get.serialize().unwrap())
                .unwrap()
                .start_pos,
            42
        );
        let put = FTPPutHeader {
            path: "C:\\file".to_string(),
            total_size: 100,
            start_pos: 10,
        };
        let parsed = FTPPutHeader::parse(&put.serialize().unwrap()).unwrap();
        assert_eq!(parsed.path, "C:\\file");
        assert_eq!(parsed.total_size, 100);
        assert_eq!(parsed.start_pos, 10);

        let info = FileInfo {
            name: "name".to_string(),
            size: 7,
            typ: "FILE".to_string(),
            last_modified: "2026-01-01 00:00:00".to_string(),
        };
        let encoded = info.serialize().unwrap();
        let parsed = FileInfo::parse(&encoded).unwrap();
        assert_eq!(parsed.name, info.name);
        assert_eq!(parsed.size, info.size);
        assert_eq!(parsed.typ, info.typ);
        assert_eq!(parsed.last_modified, info.last_modified);
    }

    #[test]
    fn malformed_packets_return_errors() {
        assert!(FTPPacket::parse(b"not-json").is_err());
        assert!(FTPGetHeader::parse(b"{}").is_err());
        assert!(FTPPutHeader::parse(b"{}").is_err());
        assert!(FileInfo::parse("{").is_err());
    }
}
