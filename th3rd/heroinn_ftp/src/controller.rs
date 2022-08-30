use heroinn_util::{
    cur_timestamp_secs,
    ftp::{
        method::{get_disk_info, get_folder_info, md5_file, remove_file},
        FTPGetHeader, FTPId, FTPPacket, FTPPutHeader, FileInfo,
    },
    packet::TunnelRequest,
    protocol::{tcp::TcpConnection, Client},
    rpc::RpcMessage,
};
use std::{io::*, net::TcpListener, sync::mpsc::Sender};

use crate::{TransferInfo, G_RPCCLIENT, G_TRANSFER};

fn send_ftp_packet(sender: &Sender<FTPPacket>, packet: FTPPacket) -> Result<()> {
    match sender.send(packet) {
        Ok(_) => Ok(()),
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "sender ftp packet faild",
        )),
    }
}

fn build_ftp_rpc_packet(rpc_data: &RpcMessage) -> Result<FTPPacket> {
    Ok(FTPPacket {
        id: FTPId::RPC.to_u8(),
        data: rpc_data.serialize()?,
    })
}

pub fn get_remote_disk_info(sender: &Sender<FTPPacket>) -> Result<Vec<FileInfo>> {
    let msg = RpcMessage::build_call("get_disk_info", vec![]);
    let mut remote_disk_info = vec![];
    send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
    match G_RPCCLIENT.wait_msg(&msg.id, 10) {
        Ok(p) => {
            if p.retcode != 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, p.msg));
            }

            for i in &p.data {
                let item = FileInfo::parse(i).unwrap();
                remote_disk_info.push(item);
            }

            Ok(remote_disk_info)
        }
        Err(e) => Err(e),
    }
}

pub fn get_local_disk_info() -> Result<Vec<FileInfo>> {
    let mut local_disk_info = vec![];
    match get_disk_info(vec![]) {
        Ok(p) => {
            for i in &p {
                let item = FileInfo::parse(i).unwrap();
                local_disk_info.push(item);
            }

            Ok(local_disk_info)
        }
        Err(e) => Err(e),
    }
}

pub fn get_remote_folder_info(
    sender: &Sender<FTPPacket>,
    full_path: &String,
) -> Result<Vec<FileInfo>> {
    let msg = RpcMessage::build_call("get_folder_info", vec![full_path.clone()]);
    let mut remote_folder_info = vec![];
    send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
    match G_RPCCLIENT.wait_msg(&msg.id, 10) {
        Ok(p) => {
            if p.retcode != 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, p.msg));
            }

            for i in &p.data {
                let item = FileInfo::parse(i).unwrap();
                remote_folder_info.push(item);
            }

            Ok(remote_folder_info)
        }
        Err(e) => Err(e),
    }
}

pub fn get_remote_join_path(
    sender: &Sender<FTPPacket>,
    cur_path: &String,
    filename: &String,
) -> Result<String> {
    let msg = RpcMessage::build_call("join_path", vec![cur_path.clone(), filename.clone()]);
    send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
    match G_RPCCLIENT.wait_msg(&msg.id, 10) {
        Ok(p) => {
            if p.retcode != 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, p.msg));
            }

            Ok(p.data[0].clone())
        }
        Err(e) => Err(e),
    }
}

pub fn get_local_folder_info(full_path: &String) -> Result<Vec<FileInfo>> {
    let mut local_folder_info = vec![];
    match get_folder_info(vec![full_path.clone()]) {
        Ok(p) => {
            for i in &p {
                let item = FileInfo::parse(i).unwrap();
                local_folder_info.push(item);
            }

            Ok(local_folder_info)
        }
        Err(e) => Err(e),
    }
}

pub fn delete_local_file(full_path: &String) -> Result<()> {
    remove_file(vec![full_path.clone()])?;
    Ok(())
}

pub fn delete_remote_file(sender: &Sender<FTPPacket>, full_path: &String) -> Result<()> {
    let msg = RpcMessage::build_call("remove_file", vec![full_path.clone()]);
    send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
    match G_RPCCLIENT.wait_msg(&msg.id, 10) {
        Ok(p) => {
            if p.retcode != 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, p.msg));
            }

            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub fn download_file(
    sender: &Sender<FTPPacket>,
    local_path: &String,
    remote_path: &String,
) -> Result<()> {
    let local_md5 = match md5_file(vec![local_path.to_string()]) {
        Ok(p) => p[0].clone(),
        Err(_) => String::new(),
    };

    let mut header = FTPGetHeader {
        path: remote_path.clone(),
        start_pos: 0,
    };

    let (mut f, total_size) = if !local_md5.is_empty() {
        let mut f = std::fs::File::options().write(true).open(local_path)?;

        let msg = RpcMessage::build_call(
            "md5_file",
            vec![remote_path.clone(), f.metadata()?.len().to_string()],
        );
        send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
        let (remote_md5, file_size) = match G_RPCCLIENT.wait_msg(&msg.id, 10) {
            Ok(p) => {
                if p.retcode != 0 {
                    return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, p.msg));
                }

                (p.data[0].clone(), p.data[1].parse::<u64>().unwrap())
            }
            Err(e) => {
                return Err(e);
            }
        };

        if remote_md5 == local_md5 {
            log::info!("resume broken transfer");
            header.start_pos = f.metadata()?.len();

            match f.seek(SeekFrom::Start(header.start_pos)) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("seek local file faild : {}", e);
                    return Err(e);
                }
            };
        }

        (f, file_size)
    } else {
        let f = std::fs::File::create(local_path)?;

        let msg = RpcMessage::build_call("file_size", vec![remote_path.clone()]);
        send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
        let file_size = match G_RPCCLIENT.wait_msg(&msg.id, 10) {
            Ok(p) => {
                if p.retcode != 0 {
                    return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, p.msg));
                }

                p.data[0].parse::<u64>().unwrap()
            }
            Err(e) => {
                return Err(e);
            }
        };

        (f, file_size)
    };

    let sender = sender.clone();
    let local_path = local_path.clone();
    let remote_path = remote_path.clone();
    std::thread::Builder::new()
        .name("download file worker".to_string())
        .spawn(move || {
            let server = TcpListener::bind("127.0.0.1:0").unwrap();

            let req = TunnelRequest {
                port: server.local_addr().unwrap().port(),
            };

            match sender.send(FTPPacket {
                id: FTPId::Get.to_u8(),
                data: req.serialize().unwrap(),
            }) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("send open tunnel msg faild : {}", e);
                    return;
                }
            }

            let (mut s, _) = match TcpConnection::tunnel_server(server, 10) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("create tunnel server faild : {}", e);
                    return;
                }
            };

            match s.send(&mut header.serialize().unwrap()) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("send get header faild : {}", e);
                    return;
                }
            };

            // init transfer log
            {
                let mut transfer = G_TRANSFER.write().unwrap();
                transfer.insert(
                    local_path.clone(),
                    TransferInfo {
                        typ: "Download".to_string(),
                        local_path: local_path.clone(),
                        remote_path: remote_path.clone(),
                        size: total_size as f64,
                        remaind_size: (total_size - header.start_pos) as f64,
                        speed: 0.0,
                        remaind_time: 999999.0,
                    },
                );
            }

            let mut tick_time = cur_timestamp_secs();

            log::debug!("start get transfer [{}]", header.path);
            loop {
                let data = match s.recv() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("recv data faild from ftp slave : {}", e);
                        break;
                    }
                };

                if data.is_empty() {
                    break;
                }

                match f.write_all(&data) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("write download file faild : {}", e);
                        break;
                    }
                };
                log::debug!("recv transfer data [{}]", data.len());
                let pos = match f.stream_position() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("get localfile size faild : {}", e);
                        break;
                    }
                };

                if pos >= total_size {
                    break;
                }

                // check cancel
                {
                    let transfer = G_TRANSFER.read().unwrap();
                    if !transfer.contains_key(&local_path) {
                        log::debug!("user cancel transfer task");
                        break;
                    }
                }

                // update status
                if cur_timestamp_secs() - tick_time >= 1 {
                    let mut transfer = G_TRANSFER.write().unwrap();

                    if transfer.contains_key(&local_path) {
                        let item = transfer.get_mut(&local_path).unwrap();
                        item.speed = item.remaind_size - (total_size - pos) as f64;
                        item.remaind_size = (total_size - pos) as f64;
                        item.remaind_time = item.remaind_size / item.speed;

                        tick_time = cur_timestamp_secs();
                    }
                }
            }

            let mut transfer = G_TRANSFER.write().unwrap();

            if transfer.contains_key(&local_path) {
                transfer.remove(&local_path);
            }

            log::info!("get file worker finished");
        })
        .unwrap();

    Ok(())
}

pub fn upload_file(
    sender: &Sender<FTPPacket>,
    local_path: &String,
    remote_path: &String,
) -> Result<()> {
    let mut header = FTPPutHeader {
        path: remote_path.clone(),
        total_size: 0,
        start_pos: 0,
    };

    let msg = RpcMessage::build_call("md5_file", vec![remote_path.clone()]);
    send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
    match G_RPCCLIENT.wait_msg(&msg.id, 10) {
        Ok(p) => {
            if p.retcode == 0 {
                let (remote_file_md5, remote_file_size) =
                    (p.data[0].clone(), p.data[1].parse::<u64>().unwrap());

                let local_md5 = md5_file(vec![local_path.clone()])?[0].clone();

                if remote_file_md5 == local_md5 {
                    log::info!("resume broken transfer");
                    header.start_pos = remote_file_size;
                }
            }
        }
        Err(_) => {}
    };

    let mut f = std::fs::File::options().read(true).open(local_path)?;

    header.total_size = f.metadata()?.len();

    let sender = sender.clone();
    let local_path = local_path.clone();
    let remote_path = remote_path.clone();
    std::thread::Builder::new()
        .name("upload file worker".to_string())
        .spawn(move || {
            let server = TcpListener::bind("127.0.0.1:0").unwrap();

            let req = TunnelRequest {
                port: server.local_addr().unwrap().port(),
            };

            match sender.send(FTPPacket {
                id: FTPId::Put.to_u8(),
                data: req.serialize().unwrap(),
            }) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("send open tunnel msg faild : {}", e);
                    return;
                }
            }

            let (mut s, _) = match TcpConnection::tunnel_server(server, 10) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("create tunnel server faild : {}", e);
                    return;
                }
            };

            match s.send(&mut header.serialize().unwrap()) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("send get header faild : {}", e);
                    return;
                }
            };

            log::debug!("start get transfer [{}]", header.path);

            if header.start_pos != 0 {
                match f.seek(SeekFrom::Start(header.start_pos)) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("seek start pos faild : {}", e);
                        return;
                    }
                };
            }

            let total_size = f.metadata().unwrap().len();
            // init transfer log
            {
                let mut transfer = G_TRANSFER.write().unwrap();
                transfer.insert(
                    local_path.clone(),
                    TransferInfo {
                        typ: "Upload".to_string(),
                        local_path: local_path.clone(),
                        remote_path: remote_path.clone(),
                        size: total_size as f64,
                        remaind_size: (total_size - header.start_pos) as f64,
                        speed: 0.0,
                        remaind_time: 999999.0,
                    },
                );
            }

            let mut tick_time = cur_timestamp_secs();

            loop {
                let mut buf = [0u8; 1024 * 20];
                let size = match f.read(&mut buf) {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("read file faild : {}", e);
                        break;
                    }
                };

                if size == 0 {
                    break;
                }

                log::debug!("send file data [{}]", size);
                match s.send(&mut buf[..size]) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("get worker send to server faild : {}", e);
                        break;
                    }
                };

                // check cancel
                {
                    let transfer = G_TRANSFER.read().unwrap();
                    if !transfer.contains_key(&local_path) {
                        log::debug!("user cancel transfer task");
                        break;
                    }
                }

                log::debug!("send transfer data [{}]", size);
                let pos = match f.stream_position() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("get localfile size faild : {}", e);
                        break;
                    }
                };

                // update status
                if cur_timestamp_secs() - tick_time >= 1 {
                    let mut transfer = G_TRANSFER.write().unwrap();

                    if transfer.contains_key(&local_path) {
                        let item = transfer.get_mut(&local_path).unwrap();
                        item.speed = item.remaind_size - (total_size - pos) as f64;
                        item.remaind_size = (total_size - pos) as f64;
                        item.remaind_time = item.remaind_size / item.speed;

                        tick_time = cur_timestamp_secs();
                    }
                }
            }

            let mut transfer = G_TRANSFER.write().unwrap();

            if transfer.contains_key(&local_path) {
                transfer.remove(&local_path);
            }

            log::info!("put file worker finished");
        })
        .unwrap();

    Ok(())
}
