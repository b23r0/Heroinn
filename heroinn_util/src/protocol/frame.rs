//! Length-prefixed framing shared by the plain-TCP transport and the local
//! worker sockets (`heroinn_shell`, `heroinn_ftp`).
//!
//! Every frame is a 4-byte big-endian length followed by exactly that many
//! payload bytes. The tunnel handshake is deliberately *not* framed this way
//! (see [`crate::protocol::TUNNEL_FLAG`]) and is handled separately.

use crate::error::{Error, Result};
use std::io::{Read, Write};

/// Largest single frame accepted on any Heroinn transport.
pub const MAX_PACKET: usize = 1024 * 9999;

/// Read one `u32` big-endian length-prefixed frame.
pub fn read_frame<R: Read>(reader: &mut R) -> Result<Vec<u8>> {
    let mut size_buf = [0u8; 4];
    reader.read_exact(&mut size_buf)?;

    let size = u32::from_be_bytes(size_buf) as usize;
    if size > MAX_PACKET {
        return Err(Error::PacketTooLarge(size));
    }

    let mut buf = vec![0u8; size];
    reader.read_exact(&mut buf)?;

    Ok(buf)
}

/// Write one `u32` big-endian length-prefixed frame.
pub fn write_frame<W: Write>(writer: &mut W, buf: &[u8]) -> Result<()> {
    if buf.len() > MAX_PACKET {
        return Err(Error::PacketTooLarge(buf.len()));
    }

    writer.write_all(&(buf.len() as u32).to_be_bytes())?;
    writer.write_all(buf)?;

    Ok(())
}
