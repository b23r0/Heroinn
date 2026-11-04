//! Parsing and formatting for the client-wide transfer rate setting.

/// Default aggregate transfer rate used by clients and the server UI.
pub const DEFAULT_TRANSFER_RATE: u64 = 1024 * 1024;

/// Number of bytes in one kilobyte as used by the UI.
pub const KB: u64 = 1024;

/// Number of bytes in one megabyte as used by the UI.
pub const MB: u64 = 1024 * KB;

/// Unit selected in the transfer-rate dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferRateUnit {
    KB,
    MB,
}

impl TransferRateUnit {
    /// Units offered by the dialog, in display order.
    pub const ALL: [Self; 2] = [Self::KB, Self::MB];

    /// Number of bytes represented by one unit.
    pub fn bytes(self) -> u64 {
        match self {
            Self::KB => KB,
            Self::MB => MB,
        }
    }

    /// Short label shown in the unit selector.
    pub fn label(self) -> &'static str {
        match self {
            Self::KB => "KB",
            Self::MB => "MB",
        }
    }
}

/// Parse a dialog value into bytes per second.
///
/// The value may contain decimals (for example `0.5 MB`), must be finite and
/// greater than zero, and must fit in a `u64` after conversion.
pub fn parse_transfer_rate(
    value: &str,
    unit: TransferRateUnit,
) -> std::result::Result<u64, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("enter a transfer rate".to_string());
    }

    let number = value
        .parse::<f64>()
        .map_err(|_| "transfer rate must be a number".to_string())?;

    if !number.is_finite() || number <= 0.0 {
        return Err("transfer rate must be greater than zero".to_string());
    }

    let bytes = number * unit.bytes() as f64;
    if !bytes.is_finite() || bytes > u64::MAX as f64 {
        return Err("transfer rate is too large".to_string());
    }

    let bytes = bytes.round();
    if bytes < 1.0 {
        return Err("transfer rate is too small".to_string());
    }

    Ok(bytes as u64)
}

/// Split a byte-per-second rate into a compact dialog value and its unit.
pub fn format_transfer_rate_input(bytes_per_sec: u64) -> (String, TransferRateUnit) {
    if bytes_per_sec >= MB {
        (
            format_number(bytes_per_sec as f64 / MB as f64),
            TransferRateUnit::MB,
        )
    } else {
        (
            format_number(bytes_per_sec as f64 / KB as f64),
            TransferRateUnit::KB,
        )
    }
}

fn format_number(value: f64) -> String {
    if (value - value.round()).abs() < 0.000_001 {
        return format!("{:.0}", value);
    }

    format!("{:.2}", value)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rate_is_one_mebibyte_per_second() {
        assert_eq!(DEFAULT_TRANSFER_RATE, 1024 * 1024);
    }

    #[test]
    fn parses_kb_and_mb_values() {
        assert_eq!(
            parse_transfer_rate("512", TransferRateUnit::KB).unwrap(),
            512 * 1024
        );
        assert_eq!(
            parse_transfer_rate(" 1 ", TransferRateUnit::MB).unwrap(),
            1024 * 1024
        );
        assert_eq!(
            parse_transfer_rate("1.5", TransferRateUnit::MB).unwrap(),
            1536 * 1024
        );
        assert_eq!(
            parse_transfer_rate("0.5", TransferRateUnit::KB).unwrap(),
            512
        );
    }

    #[test]
    fn rejects_invalid_rates() {
        for value in ["", " ", "zero", "0", "-1", "NaN", "inf", "1e300", "0.0001"] {
            assert!(
                parse_transfer_rate(value, TransferRateUnit::KB).is_err(),
                "{value} should be rejected"
            );
        }
    }

    #[test]
    fn formats_rates_for_the_dialog() {
        assert_eq!(
            format_transfer_rate_input(DEFAULT_TRANSFER_RATE),
            ("1".to_string(), TransferRateUnit::MB)
        );
        assert_eq!(
            format_transfer_rate_input(1536 * 1024),
            ("1.5".to_string(), TransferRateUnit::MB)
        );
        assert_eq!(
            format_transfer_rate_input(512 * 1024),
            ("512".to_string(), TransferRateUnit::KB)
        );
    }

    #[test]
    fn formatted_rate_round_trips() {
        for bytes in [512, 1024, 64 * 1024, 1024 * 1024, 1536 * 1024] {
            let (value, unit) = format_transfer_rate_input(bytes);
            assert_eq!(parse_transfer_rate(&value, unit).unwrap(), bytes);
        }
    }
}
