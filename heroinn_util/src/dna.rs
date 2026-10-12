//! Rewriting the connection stamp inside a generated client binary.
//!
//! The generator locates [`CONNECTION_INFO_FLAG`] in the executable image and
//! overwrites the [`SlaveDNA`] region that follows it.

use crate::{ConnectionInfo, Error, Result, SlaveDNA};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

/// Magic marker that precedes a connection stamp in a built client binary.
pub const CONNECTION_INFO_FLAG: [u8; 8] = [0xff, 0xfe, 0xf1, 0xa1, 0xff, 0xfe, 0xf1, 0xa1];

/// Copy `path` to `new_path`, replacing the embedded connection stamp with `new_info`.
pub fn replace_connection_info_to_new_file(
    path: &str,
    new_path: &str,
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
            cursor.write_all(&SlaveDNA::new(&payload)?.serialize())?;

            found = true;
            break;
        }

        cursor.seek(SeekFrom::Current(-7))?;
        size -= 1;
    }

    if !found {
        return Err(Error::NotFound(
            "connection info flag not present in binary".to_string(),
        ));
    }

    let mut new_f = std::fs::File::create(new_path)?;
    new_f.write_all(&buf)?;

    Ok(())
}
