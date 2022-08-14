use sysinfo::{SystemExt, DiskExt};
use std::{io::*, path::Path};
use chrono::DateTime;
use chrono::offset::Utc;
use path_absolutize::*;
use super::FileInfo;

pub fn transfer_size(size : f64) -> String{
    if size < 1024.0 {
        format!("{:.2} Byte" , size )
    } else if size < (1024.0 * 1024.0) {
        format!("{:.2} KB" , size / 1024.0 )
    } else if size < (1024.0 * 1024.0 * 1024.0) {
        format!("{:.2} MB" , size / (1024.0 * 1024.0) )
    } else {
        format!("{:.2} GB" , size / (1024.0 * 1024.0 * 1024.0) )
    }
}


pub fn get_disk_info(_ : Vec<String>) -> Result<Vec<String>>{
    
    let mut ret = vec![];

    let sys = sysinfo::System::new_all();
    for d in sys.disks(){
        let name = d.mount_point().to_str().unwrap().to_string();
        let typ = format!("{:?}", d.type_());
        let size = d.total_space();

        let info = FileInfo{
            name,
            size,
            typ,
            last_modified: String::new()
        };

        ret.push(info.serialize()?);
    }
    Ok(ret)
}

pub fn get_folder_info(param : Vec<String>) -> Result<Vec<String>>{
    
    let mut ret = vec![];

    let cur_path = param[0].clone();
    let dirs = std::fs::read_dir(cur_path)?;
    for d in dirs{
        let d = d?;
        let t = d.file_type()?;
        let mt = d.metadata()?.modified()?;
        let mt: DateTime<Utc> = mt.into();

        let info = FileInfo{
            name : d.file_name().to_str().unwrap().to_string(),
            size : d.metadata()?.len(),
            typ : if t.is_dir() {
				String::from("FOLDER")
			} else if t.is_file() {
				String::from("FILE")
			} else if t.is_symlink() {
				String::from("SYMLINK")
			} else {
				String::from("OTHER")
			},
            last_modified: mt.format("%Y-%m-%d %H:%M:%S").to_string()
        };

        ret.push(info.serialize()?);
    }
    Ok(ret)
}

pub fn join_path(param : Vec<String>) -> Result<Vec<String>>{
    Ok(vec![Path::new(&param[0]).join(&param[1]).absolutize().unwrap().to_str().unwrap().to_string()])
}