use super::FileInfo;
use chrono::offset::Utc;
use chrono::DateTime;
use path_absolutize::*;
use std::{io::*, path::Path};
use sysinfo::{DiskExt, SystemExt};

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

pub fn get_disk_info(_: Vec<String>) -> Result<Vec<String>> {
    let mut ret = vec![];

    let sys = sysinfo::System::new_all();
    for d in sys.disks() {
        let name = d.mount_point().to_str().unwrap().to_string();
        let mut typ = format!("{:?}", d.type_());

        if typ.contains("Unknown") {
            typ = "Unknown Drive".to_string();
        }

        let size = d.total_space();

        let info = FileInfo {
            name,
            size,
            typ,
            last_modified: String::new(),
        };

        ret.push(info.serialize()?);
    }
    Ok(ret)
}

pub fn get_folder_info(param: Vec<String>) -> Result<Vec<String>> {
    let mut ret = vec![];

    let cur_path = param[0].clone();
    let dirs = std::fs::read_dir(cur_path)?;

    for d in dirs {
        let d = d?;
        let t = d.file_type()?;
        let mt = d.metadata()?.modified()?;
        let mt: DateTime<Utc> = mt.into();

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
        if a.typ == "FOLDER" && b.typ == "FOLDER" {
            std::cmp::Ordering::Less
        } else if a.typ == "FOLDER" {
            std::cmp::Ordering::Less
        } else if b.typ == "FOLDER" {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Less
        }
    });
    Ok(ret)
}

pub fn join_path(param: Vec<String>) -> Result<Vec<String>> {
    Ok(vec![Path::new(&param[0])
        .join(&param[1])
        .absolutize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()])
}

pub fn remove_file(param: Vec<String>) -> Result<Vec<String>> {
    let filename = param[0].clone();
    std::fs::remove_file(filename)?;
    Ok(vec![])
}

pub fn md5_file(param: Vec<String>) -> Result<Vec<String>> {
    let path = param[0].clone();

    let mut f = std::fs::File::open(path)?;

    let end_pos: u64 = if param.len() == 2 {
        param[1].clone().parse::<u64>().unwrap()
    } else {
        f.metadata()?.len()
    };

    let mut md5_str = String::new();

    let mut md5 = md5::Md5::default();

    let mut buffer = vec![0u8; 1024 * 1024 * 20].into_boxed_slice();

    let mut sum: u64 = 0;
    loop {
        if (end_pos - sum) <= 1024 * 1024 * 20 {
            let mut last_buf = vec![0u8; (end_pos - sum) as usize].into_boxed_slice();
            f.read_exact(&mut last_buf)?;

            md5::Digest::update(&mut md5, &last_buf);

            break;
        }

        let n = f.read(&mut buffer)?;
        sum += n as u64;
        md5::Digest::update(&mut md5, &buffer[..n]);

        if n == 0 {
            break;
        }
    }

    for b in md5::Digest::finalize(md5) {
        let a = format!("{:02x}", b);
        md5_str += &a;
    }

    Ok(vec![md5_str, f.metadata()?.len().to_string()])
}

pub fn file_size(param: Vec<String>) -> Result<Vec<String>> {
    Ok(vec![std::fs::File::open(&param[0])?
        .metadata()?
        .len()
        .to_string()])
}
