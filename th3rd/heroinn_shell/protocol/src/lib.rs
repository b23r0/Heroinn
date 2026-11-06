//! In-band control frames exchanged between Heroinn and `heroinn_shell`.
//!
//! Terminal input and output remain opaque bytes. Two exact byte sequences are
//! reserved solely for closing a terminal and carrying a row/column resize.

/// Two-byte prefix that marks an in-band control frame.
pub const MAGIC: [u8; 2] = [0x37, 0x37];

/// Third byte of the close frame.
pub const CLOSE: u8 = 0xff;

/// Encoded length of a resize frame.
pub const RESIZE_LEN: usize = 6;

/// Runtime window dimensions carried from the shell window to the PTY.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowSize {
    pub rows: u16,
    pub cols: u16,
}

/// A classified shell stream fragment.
#[derive(Debug, PartialEq, Eq)]
pub enum Control<'a> {
    /// Bytes that belong to the terminal data stream.
    Data(&'a [u8]),
    /// A request to resize the remote PTY.
    Resize(WindowSize),
    /// A request to close this shell session.
    Close,
}

/// Classify one complete shell packet.
///
/// Only the exact close and resize lengths are reserved. Other packets,
/// including malformed packets beginning with [`MAGIC`], stay on the terminal
/// data path to preserve the historical wire behavior.
pub fn classify(data: &[u8]) -> Control<'_> {
    if data.len() == 3 && data[..2] == MAGIC && data[2] == CLOSE {
        return Control::Close;
    }

    if data.len() == RESIZE_LEN && data[..2] == MAGIC {
        return Control::Resize(WindowSize {
            rows: u16::from_be_bytes([data[2], data[3]]),
            cols: u16::from_be_bytes([data[4], data[5]]),
        });
    }

    Control::Data(data)
}

/// Decode one control frame at the start of a shell stream chunk.
///
/// Local socket reads may coalesce a resize/close frame with terminal input.
/// The returned byte count lets callers consume only the control frame and
/// forward the remaining bytes as terminal data.
pub fn control_prefix(data: &[u8]) -> Option<(Control<'_>, usize)> {
    if data.len() >= 3 && data[..2] == MAGIC && data[2] == CLOSE {
        return Some((Control::Close, 3));
    }

    if data.len() >= RESIZE_LEN && data[..2] == MAGIC {
        return Some((
            Control::Resize(WindowSize {
                rows: u16::from_be_bytes([data[2], data[3]]),
                cols: u16::from_be_bytes([data[4], data[5]]),
            }),
            RESIZE_LEN,
        ));
    }

    None
}

/// Encode a resize control frame.
pub fn encode_resize(size: WindowSize) -> [u8; RESIZE_LEN] {
    let rows = size.rows.to_be_bytes();
    let cols = size.cols.to_be_bytes();
    [MAGIC[0], MAGIC[1], rows[0], rows[1], cols[0], cols[1]]
}

/// Encode the close control frame.
pub fn encode_close() -> [u8; 3] {
    [MAGIC[0], MAGIC[1], CLOSE]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_frame_is_exact() {
        let frame = encode_close();
        assert_eq!(frame, [0x37, 0x37, 0xff]);
        assert_eq!(classify(&frame), Control::Close);
        assert_eq!(classify(&[0x37, 0x37, 0xfe]), Control::Data(&[0x37, 0x37, 0xfe]));
        assert_eq!(classify(&[0x37, 0x37, 0xff, 0x00]), Control::Data(&[0x37, 0x37, 0xff, 0x00]));
    }

    #[test]
    fn resize_frame_is_rows_then_columns_big_endian() {
        let size = WindowSize { rows: 0x1234, cols: 0xabcd };
        let frame = encode_resize(size);
        assert_eq!(frame, [0x37, 0x37, 0x12, 0x34, 0xab, 0xcd]);
        assert_eq!(classify(&frame), Control::Resize(size));
    }

    #[test]
    fn ordinary_and_malformed_packets_remain_terminal_data() {
        for data in [
            b"hello".as_slice(),
            &[0x37],
            &[0x37, 0x37],
            &[0x37, 0x37, 0x01],
            &[0x37, 0x37, 0x01, 0x02, 0x03],
        ] {
            assert_eq!(classify(data), Control::Data(data));
        }
    }

    #[test]
    fn control_prefix_is_separate_from_coalesced_terminal_data() {
        let size = WindowSize { rows: 0x0023, cols: 0x0064 };
        let mut packet = encode_resize(size).to_vec();
        packet.extend_from_slice(b"echo shell\r");

        let (control, consumed) = control_prefix(&packet).unwrap();
        assert_eq!(control, Control::Resize(size));
        assert_eq!(consumed, RESIZE_LEN);
        assert_eq!(&packet[consumed..], b"echo shell\r");
    }

    #[test]
    fn close_prefix_leaves_later_data_untouched() {
        let mut packet = encode_close().to_vec();
        packet.extend_from_slice(b"tail");

        let (control, consumed) = control_prefix(&packet).unwrap();
        assert_eq!(control, Control::Close);
        assert_eq!(consumed, 3);
        assert_eq!(&packet[consumed..], b"tail");
    }
}
