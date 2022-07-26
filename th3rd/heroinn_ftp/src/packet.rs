enum FTPId{
    GetDirectory,
    Unknow
}

impl FTPId{
    fn to_u8(&self) -> u8{
        match self{
            FTPId::GetDirectory => 0x01,
            FTPId::Unknow => 0xff,
        }
    }

    fn from(id : u8) -> Self{
        match id{
            0x01 => FTPId::GetDirectory,
            _ => FTPId::Unknow
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct FTPPacket{
    id : u8,
    data : String
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct FileInfo{
    name : String,
    size : u64,
    typ : String,
    last_modified : String,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct DirectoryInfo{
    path : String,     
    detail : Vec<FileInfo>
}