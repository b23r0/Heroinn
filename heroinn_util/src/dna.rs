//! Rewriting the connection stamp inside a generated client binary.
//!
//! The generator locates [`CONNECTION_INFO_FLAG`] in the executable image and
//! overwrites the [`SlaveDNA`] region that follows it.

use crate::{CONNECTION_INFO_DATA_LEN, ConnectionInfo, Error, Result, SLAVE_DNA_LEN, SlaveDNA};
use std::fs;
use std::path::{Path, PathBuf};

/// Magic marker that precedes a connection stamp in a built client binary.
pub const CONNECTION_INFO_FLAG: [u8; 8] = [0xff, 0xfe, 0xf1, 0xa1, 0xff, 0xfe, 0xf1, 0xa1];

/// Copy the client template to `new_path`, replacing its connection stamp.
///
/// Exactly one structurally valid stamp must be present. The destination file
/// inherits the template's permissions, including the executable bit on Unix.
pub fn replace_connection_info_to_new_file(
    path: impl AsRef<Path>,
    new_path: impl AsRef<Path>,
    new_info: ConnectionInfo,
) -> Result<()> {
    let source = path.as_ref();
    let destination = new_path.as_ref();

    if source == destination {
        return Err(Error::Other(
            "the generated client must not overwrite its template".to_string(),
        ));
    }

    let metadata = fs::metadata(source)?;
    let mut binary = fs::read(source)?;
    let stamp = SlaveDNA::new(&new_info.serialize()?)?.serialize();
    let offsets = connection_stamp_offsets(&binary);

    let offset = match offsets.as_slice() {
        [] => {
            return Err(Error::NotFound(
                "connection info flag not present in binary".to_string(),
            ));
        }
        [offset] => *offset,
        _ => {
            return Err(Error::Other(format!(
                "multiple connection info stamps found at byte offsets {}; refusing to guess",
                offsets
                    .iter()
                    .map(|offset| offset.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    };

    let end = offset
        .checked_add(stamp.len())
        .ok_or_else(|| Error::Other("connection info offset overflow".to_string()))?;
    binary[offset..end].copy_from_slice(&stamp);

    fs::write(destination, &binary)?;
    fs::set_permissions(destination, metadata.permissions())?;

    Ok(())
}

/// Find a client template in the given directories.
///
/// Directory order takes precedence over file-name order: all names are tested
/// in the first directory before moving to the next one.
pub fn find_client_template(
    template_names: &[&str],
    search_directories: &[PathBuf],
) -> Result<PathBuf> {
    let mut checked = Vec::new();

    for directory in search_directories {
        for name in template_names {
            let candidate = directory.join(name);
            checked.push(candidate.clone());
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    let checked = checked
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(Error::NotFound(format!(
        "client template not found; checked: {checked}"
    )))
}

fn connection_stamp_offsets(binary: &[u8]) -> Vec<usize> {
    let mut offsets = Vec::new();
    let Some(last_start) = binary.len().checked_sub(CONNECTION_INFO_FLAG.len()) else {
        return offsets;
    };

    for offset in 0..=last_start {
        if binary[offset..offset + CONNECTION_INFO_FLAG.len()] != CONNECTION_INFO_FLAG {
            continue;
        }

        let Some(end) = offset.checked_add(SLAVE_DNA_LEN) else {
            continue;
        };
        let Some(stamp) = binary.get(offset..end) else {
            continue;
        };

        let payload_len = u64::from_be_bytes(stamp[8..16].try_into().unwrap());
        if payload_len > CONNECTION_INFO_DATA_LEN as u64 {
            continue;
        }

        let payload_end = 16 + payload_len as usize;
        if stamp[payload_end..].iter().any(|byte| *byte != 0) {
            continue;
        }

        offsets.push(offset);
    }

    offsets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HeroinnProtocol;
    use std::path::PathBuf;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "heroinn-dna-test-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn template(prefix: &[u8], suffix: &[u8]) -> Vec<u8> {
        let mut binary = prefix.to_vec();
        binary.extend(SlaveDNA::new(&[]).unwrap().serialize());
        binary.extend_from_slice(suffix);
        binary
    }

    #[test]
    fn replaces_stamp_and_preserves_other_bytes() {
        let dir = TestDir::new();
        let source = dir.0.join("template");
        let output = dir.0.join("client");
        let prefix = b"prefix";
        let suffix = b"suffix";
        fs::write(&source, template(prefix, suffix)).unwrap();

        let info =
            ConnectionInfo::new(HeroinnProtocol::HTTP, "example.com", "9001", "debug client")
                .unwrap();
        replace_connection_info_to_new_file(&source, &output, info.clone()).unwrap();

        let binary = fs::read(&output).unwrap();
        assert_eq!(&binary[..prefix.len()], prefix);
        assert_eq!(&binary[binary.len() - suffix.len()..], suffix);

        let stamp = SlaveDNA::parse(&binary[prefix.len()..prefix.len() + SLAVE_DNA_LEN]).unwrap();
        let parsed = ConnectionInfo::parse(stamp.payload().unwrap()).unwrap();
        assert_eq!(parsed, info);
    }

    #[cfg(unix)]
    #[test]
    fn preserves_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TestDir::new();
        let source = dir.0.join("template");
        let output = dir.0.join("client");
        fs::write(&source, template(b"", b"")).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o751)).unwrap();

        let info =
            ConnectionInfo::new(HeroinnProtocol::TCP, "127.0.0.1", "9001", "permissions").unwrap();
        replace_connection_info_to_new_file(&source, &output, info).unwrap();

        let mode = fs::metadata(&output).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o751);
    }

    #[test]
    fn rejects_template_without_stamp() {
        let dir = TestDir::new();
        let source = dir.0.join("template");
        let output = dir.0.join("client");
        fs::write(&source, b"not a heroinn client").unwrap();

        let info =
            ConnectionInfo::new(HeroinnProtocol::UDP, "127.0.0.1", "8000", "missing").unwrap();
        let err = replace_connection_info_to_new_file(&source, &output, info).unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
    }

    #[test]
    fn rejects_ambiguous_templates() {
        let dir = TestDir::new();
        let source = dir.0.join("template");
        let output = dir.0.join("client");
        let stamp = SlaveDNA::new(&[]).unwrap().serialize();
        let mut binary = stamp.clone();
        binary.extend(stamp);
        fs::write(&source, binary).unwrap();

        let info =
            ConnectionInfo::new(HeroinnProtocol::UDP, "127.0.0.1", "8000", "ambiguous").unwrap();
        let err = replace_connection_info_to_new_file(&source, &output, info).unwrap_err();
        assert!(matches!(err, Error::Other(message) if message.contains("multiple")));
    }

    #[test]
    fn finds_template_in_first_matching_directory() {
        let first = TestDir::new();
        let second = TestDir::new();
        fs::write(first.0.join("heroinn_client"), b"first").unwrap();
        fs::write(second.0.join("heroinn_client"), b"second").unwrap();

        let found = find_client_template(
            &["missing", "heroinn_client"],
            &[first.0.clone(), second.0.clone()],
        )
        .unwrap();

        assert_eq!(found, first.0.join("heroinn_client"));
    }
}
