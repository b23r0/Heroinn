//! Length-prefixed framing shared by the plain-TCP transport and the local
//! worker sockets (`heroinn_shell`, `heroinn_ftp`).
//!
//! Every frame is a 4-byte big-endian length followed by exactly that many
//! payload bytes. The tunnel handshake is deliberately *not* framed this way
//! (see [`crate::protocol::TUNNEL_FLAG`]) and is handled separately.

use crate::error::{Error, Result};
use std::io::{Read, Write};

/// Largest single frame accepted on any Heroinn transport.
pub const MAX_PACKET: usize = 10 * 1024 * 1024;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, ErrorKind};

    #[test]
    fn empty_and_unaligned_frames_round_trip() {
        for payload in [
            Vec::new(),
            vec![0],
            b"unaligned payload".to_vec(),
            vec![0xa5; 4096],
        ] {
            let mut encoded = Vec::new();
            write_frame(&mut encoded, &payload).unwrap();
            assert_eq!(
                u32::from_be_bytes(encoded[..4].try_into().unwrap()) as usize,
                payload.len()
            );
            let mut cursor = Cursor::new(encoded);
            assert_eq!(read_frame(&mut cursor).unwrap(), payload);
        }
    }

    #[test]
    fn truncated_prefix_and_payload_return_unexpected_eof() {
        for bytes in [&b"\x00\x00"[..], &b"\x00\x00\x00\x04abc"[..]] {
            let error = read_frame(&mut Cursor::new(bytes)).unwrap_err();
            assert!(matches!(
                error,
                Error::Io(ref io) if io.kind() == ErrorKind::UnexpectedEof
            ));
        }
    }

    #[test]
    fn oversized_frames_are_rejected_without_writing_a_partial_prefix() {
        let mut writer = Vec::new();
        let error = write_frame(&mut writer, &vec![0; MAX_PACKET + 1]).unwrap_err();
        assert!(matches!(error, Error::PacketTooLarge(size) if size == MAX_PACKET + 1));
        assert!(writer.is_empty());

        let mut encoded = Vec::new();
        encoded.extend_from_slice(&((MAX_PACKET + 1) as u32).to_be_bytes());
        let error = read_frame(&mut Cursor::new(encoded)).unwrap_err();
        assert!(matches!(error, Error::PacketTooLarge(size) if size == MAX_PACKET + 1));
    }

    #[test]
    fn exact_ten_mebibyte_frame_round_trips_and_one_byte_over_is_rejected() {
        let payload = vec![0x5a; 10 * 1024 * 1024];
        let mut encoded = Vec::new();
        write_frame(&mut encoded, &payload).unwrap();
        assert_eq!(
            u32::from_be_bytes(encoded[..4].try_into().unwrap()) as usize,
            payload.len()
        );
        let mut cursor = Cursor::new(encoded);
        assert_eq!(read_frame(&mut cursor).unwrap(), payload);

        let mut writer = Vec::new();
        let error = write_frame(&mut writer, &vec![0; 10 * 1024 * 1024 + 1]).unwrap_err();
        assert!(matches!(error, Error::PacketTooLarge(size) if size == 10 * 1024 * 1024 + 1));
        assert!(writer.is_empty());
    }
}
