//! File-system operations exposed to remote clients over RPC.

use std::io::Read;
use std::path::Path;

use chrono::DateTime;
use chrono::offset::Utc;
use md5::{Digest, Md5};
use path_absolutize::*;
use sysinfo::Disks;

use super::FileInfo;
use crate::error::Result;

/// Render a byte count as a human-readable size.
pub fn transfer_size(size: f64) -> String {
    if size < 1024.0 {
        format!("{:.2} Byte", size)
    } else if size < (1024.0 * 1024.0) {
        format!("{:.2} KB", size / 1024.0)
    } else if size < (1024.0 * 1024.0 * 1024.0) {
        format!("{:.2} MB", size / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", size / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Render a bytes-per-second figure as a human-readable speed.
pub fn transfer_speed(size: f64) -> String {
    if size < 1024.0 {
        format!("{:.2} Byte/s", size)
    } else if size < (1024.0 * 1024.0) {
        format!("{:.2} KB/s", size / 1024.0)
    } else if size < (1024.0 * 1024.0 * 1024.0) {
        format!("{:.2} MB/s", size / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB/s", size / (1024.0 * 1024.0 * 1024.0))
    }
}

/// List the mounted drives as serialized [`FileInfo`]s.
pub fn get_disk_info(_: Vec<String>) -> Result<Vec<String>> {
    let mut ret = vec![];

    let disks = Disks::new_with_refreshed_list();
    for d in disks.list() {
        let name = d.mount_point().to_str().unwrap().to_string();
        let mut typ = format!("{:?}", d.kind());

        if typ.contains("Unknown") {
            typ = "Unknown Drive".to_string();
        }

        let info = FileInfo {
            name,
            size: d.total_space(),
            typ,
            last_modified: String::new(),
        };

        ret.push(info.serialize()?);
    }
    Ok(ret)
}

/// List the entries of the directory named by `param[0]`.
pub fn get_folder_info(param: Vec<String>) -> Result<Vec<String>> {
    let mut ret = vec![];

    let cur_path = param[0].clone();
    let dirs = std::fs::read_dir(cur_path)?;

    for d in dirs {
        let d = d?;
        let t = d.file_type()?;
        let mt: DateTime<Utc> = d.metadata()?.modified()?.into();

        let info = FileInfo {
            name: d.file_name().to_str().unwrap().to_string(),
            size: d.metadata()?.len(),
            typ: if t.is_dir() {
                String::from("FOLDER")
            } else if t.is_file() {
                String::from("FILE")
            } else if t.is_symlink() {
                String::from("SYMLINK")
            } else {
                String::from("Unknown")
            },
            last_modified: mt.format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        ret.push(info.serialize()?);
    }

    ret.sort_by(|a, b| {
        let a = FileInfo::parse(a).unwrap();
        let b = FileInfo::parse(b).unwrap();
        if a.typ == "FOLDER" {
            std::cmp::Ordering::Less
        } else if b.typ == "FOLDER" {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Less
        }
    });
    Ok(ret)
}

/// Join `param[1]` onto `param[0]` and absolutize the result.
pub fn join_path(param: Vec<String>) -> Result<Vec<String>> {
    Ok(vec![
        Path::new(&param[0])
            .join(&param[1])
            .absolutize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
    ])
}

/// Delete the file named by `param[0]`.
pub fn remove_file(param: Vec<String>) -> Result<Vec<String>> {
    std::fs::remove_file(param[0].clone())?;
    Ok(vec![])
}

/// Hash the first `param[1]` bytes (or the whole file) and return hex + size.
pub fn md5_file(param: Vec<String>) -> Result<Vec<String>> {
    let path = param[0].clone();

    let mut f = std::fs::File::open(path)?;

    let end_pos: u64 = if param.len() == 2 {
        param[1].clone().parse::<u64>().unwrap()
    } else {
        f.metadata()?.len()
    };

    let mut md5 = Md5::new();
    let mut buffer = vec![0u8; 1024 * 1024 * 20].into_boxed_slice();

    let mut sum: u64 = 0;
    loop {
        if (end_pos - sum) <= 1024 * 1024 * 20 {
            let mut last_buf = vec![0u8; (end_pos - sum) as usize].into_boxed_slice();
            f.read_exact(&mut last_buf)?;

            md5.update(&last_buf);
            break;
        }

        let n = f.read(&mut buffer)?;
        sum += n as u64;
        md5.update(&buffer[..n]);

        if n == 0 {
            break;
        }
    }

    let md5_str = md5
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    Ok(vec![md5_str, f.metadata()?.len().to_string()])
}

/// Report the size of the file named by `param[0]`.
pub fn file_size(param: Vec<String>) -> Result<Vec<String>> {
    Ok(vec![
        std::fs::File::open(&param[0])?
            .metadata()?
            .len()
            .to_string(),
    ])
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "heroinn-ftp-method-{}-{}",
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

    #[test]
    fn byte_and_rate_formatting_switch_at_each_unit_boundary() {
        assert_eq!(transfer_size(0.0), "0.00 Byte");
        assert_eq!(transfer_size(1023.0), "1023.00 Byte");
        assert_eq!(transfer_size(1024.0), "1.00 KB");
        assert_eq!(transfer_size(1024.0 * 1024.0), "1.00 MB");
        assert_eq!(transfer_size(1024.0 * 1024.0 * 1024.0), "1.00 GB");

        assert_eq!(transfer_speed(0.0), "0.00 Byte/s");
        assert_eq!(transfer_speed(1023.0), "1023.00 Byte/s");
        assert_eq!(transfer_speed(1024.0), "1.00 KB/s");
        assert_eq!(transfer_speed(1024.0 * 1024.0), "1.00 MB/s");
        assert_eq!(transfer_speed(1024.0 * 1024.0 * 1024.0), "1.00 GB/s");
    }

    #[test]
    fn folder_listing_sorts_folders_before_files_and_reports_metadata() {
        let dir = TestDir::new();
        fs::write(dir.0.join("z-file.txt"), b"abc").unwrap();
        fs::create_dir(dir.0.join("a-folder")).unwrap();
        fs::write(dir.0.join("a-file.txt"), b"12").unwrap();

        let encoded = get_folder_info(vec![dir.0.to_string_lossy().into_owned()]).unwrap();
        let entries = encoded
            .iter()
            .map(|entry| FileInfo::parse(entry).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(entries.len(), 3);
        assert!(entries[0].typ == "FOLDER");
        assert!(entries[1..].iter().all(|entry| entry.typ == "FILE"));
        assert!(
            entries
                .iter()
                .any(|entry| entry.name == "a-file.txt" && entry.size == 2)
        );
        assert!(entries.iter().all(|entry| entry.last_modified.len() == 19));
        assert!(
            get_folder_info(vec![dir.0.join("missing").to_string_lossy().into_owned()]).is_err()
        );
    }

    #[test]
    fn join_remove_size_and_md5_cover_whole_and_partial_files() {
        let dir = TestDir::new();
        let nested = join_path(vec![
            dir.0.to_string_lossy().into_owned(),
            "nested".to_string(),
            "file.bin".to_string(),
        ])
        .unwrap()
        .remove(0);
        assert!(Path::new(&nested).is_absolute());

        let file = dir.0.join("file.bin");
        fs::write(&file, b"abcdef").unwrap();
        assert_eq!(
            file_size(vec![file.to_string_lossy().into_owned()]).unwrap(),
            ["6"]
        );
        let whole = md5_file(vec![file.to_string_lossy().into_owned()]).unwrap();
        let partial = md5_file(vec![file.to_string_lossy().into_owned(), "3".to_string()]).unwrap();
        assert_eq!(whole[1], "6");
        assert_eq!(partial[1], "6");
        assert_ne!(whole[0], partial[0]);

        remove_file(vec![file.to_string_lossy().into_owned()]).unwrap();
        assert!(!file.exists());
        assert!(file_size(vec![file.to_string_lossy().into_owned()]).is_err());
        assert!(md5_file(vec![file.to_string_lossy().into_owned()]).is_err());
    }

    #[test]
    fn disk_listing_returns_serializable_entries() {
        let disks = get_disk_info(Vec::new()).unwrap();
        for disk in disks {
            let parsed = FileInfo::parse(&disk).unwrap();
            assert!(!parsed.name.is_empty());
            assert!(!parsed.typ.is_empty());
        }
    }
}
