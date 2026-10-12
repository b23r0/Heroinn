//! File-system operations exposed to remote clients over RPC.

use std::io::Read;
use std::path::Path;

use chrono::offset::Utc;
use chrono::DateTime;
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
    Ok(vec![Path::new(&param[0])
        .join(&param[1])
        .absolutize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()])
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
    Ok(vec![std::fs::File::open(&param[0])?
        .metadata()?
        .len()
        .to_string()])
}
