use crate::{ConnectionInfo, SlaveDNA};
use std::io::*;

pub const CONNECTION_INFO_FLAG: [u8; 8] = [0xff, 0xfe, 0xf1, 0xa1, 0xff, 0xfe, 0xf1, 0xa1];

pub fn replace_connection_info_to_new_file(
    path: &String,
    new_path: &String,
    new_info: ConnectionInfo,
) -> Result<()> {
    let mut f = std::fs::File::open(path)?;

    let mut buf = vec![];
    let mut size = f.read_to_end(&mut buf)?;

    let mut cursor = Cursor::new(&mut buf);

    let mut found = false;

    while size >= 8 {
        let mut flag = [0u8; 8];

        cursor.read_exact(&mut flag)?;

        if flag == CONNECTION_INFO_FLAG {
            cursor.seek(SeekFrom::Current(-8))?;

            let payload = new_info.serialize()?;

            cursor.write_all(&SlaveDNA::new(&payload).serilize())?;

            found = true;

            break;
        }

        cursor.seek(SeekFrom::Current(-7))?;

        size -= 1;
    }

    if !found {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Not found flag",
        ));
    }

    let mut new_f = std::fs::File::create(new_path)?;

    new_f.write_all(&mut buf)?;

    Ok(())
}
