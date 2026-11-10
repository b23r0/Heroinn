use heroinn_util::{
    Error, Result, cur_timestamp_secs,
    ftp::{
        FTPGetHeader, FTPId, FTPPacket, FTPPutHeader, FileInfo,
        method::{get_disk_info, get_folder_info, md5_file, remove_file},
    },
    packet::TunnelRequest,
    protocol::tcp::TcpConnection,
    rpc::RpcMessage,
};
use std::io::{Read, Seek, SeekFrom, Write};
use std::{net::TcpListener, sync::mpsc::Sender};

use crate::{G_RPCCLIENT, G_TRANSFER, TransferInfo};

fn send_ftp_packet(sender: &Sender<FTPPacket>, packet: FTPPacket) -> Result<()> {
    match sender.send(packet) {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::Other("sender ftp packet failed".to_string())),
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
                return Err(Error::Other(p.msg));
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
    full_path: &str,
) -> Result<Vec<FileInfo>> {
    let msg = RpcMessage::build_call("get_folder_info", vec![full_path.to_string()]);
    let mut remote_folder_info = vec![];
    send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
    match G_RPCCLIENT.wait_msg(&msg.id, 10) {
        Ok(p) => {
            if p.retcode != 0 {
                return Err(Error::Other(p.msg));
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
    cur_path: &str,
    filename: &str,
) -> Result<String> {
    let msg = RpcMessage::build_call(
        "join_path",
        vec![cur_path.to_string(), filename.to_string()],
    );
    send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
    match G_RPCCLIENT.wait_msg(&msg.id, 10) {
        Ok(p) => {
            if p.retcode != 0 {
                return Err(Error::Other(p.msg));
            }

            Ok(p.data[0].clone())
        }
        Err(e) => Err(e),
    }
}

pub fn get_local_folder_info(full_path: &str) -> Result<Vec<FileInfo>> {
    let mut local_folder_info = vec![];
    match get_folder_info(vec![full_path.to_string()]) {
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

pub fn delete_local_file(full_path: &str) -> Result<()> {
    remove_file(vec![full_path.to_string()])?;
    Ok(())
}

pub fn delete_remote_file(sender: &Sender<FTPPacket>, full_path: &str) -> Result<()> {
    let msg = RpcMessage::build_call("remove_file", vec![full_path.to_string()]);
    send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
    match G_RPCCLIENT.wait_msg(&msg.id, 10) {
        Ok(p) => {
            if p.retcode != 0 {
                return Err(Error::Other(p.msg));
            }

            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub fn download_file(
    sender: &Sender<FTPPacket>,
    local_path: &str,
    remote_path: &str,
) -> Result<()> {
    let local_md5 = match md5_file(vec![local_path.to_string()]) {
        Ok(p) => p[0].clone(),
        Err(_) => String::new(),
    };

    let mut header = FTPGetHeader {
        path: remote_path.to_string(),
        start_pos: 0,
    };

    let (mut f, total_size) = if !local_md5.is_empty() {
        let mut f = std::fs::File::options().write(true).open(local_path)?;

        let msg = RpcMessage::build_call(
            "md5_file",
            vec![remote_path.to_string(), f.metadata()?.len().to_string()],
        );
        send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
        let (remote_md5, file_size) = match G_RPCCLIENT.wait_msg(&msg.id, 10) {
            Ok(p) => {
                if p.retcode != 0 {
                    return Err(Error::Other(p.msg));
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
                    log::error!("seek local file failed : {}", e);
                    return Err(Error::Io(e));
                }
            };
        }

        (f, file_size)
    } else {
        let f = std::fs::File::create(local_path)?;

        let msg = RpcMessage::build_call("file_size", vec![remote_path.to_string()]);
        send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
        let file_size = match G_RPCCLIENT.wait_msg(&msg.id, 10) {
            Ok(p) => {
                if p.retcode != 0 {
                    return Err(Error::Other(p.msg));
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
    let local_path = local_path.to_string();
    let remote_path = remote_path.to_string();
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
                    log::error!("send open tunnel msg failed : {}", e);
                    return;
                }
            }

            let (mut s, _) = match TcpConnection::tunnel_server(server, 10) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("create tunnel server failed : {}", e);
                    return;
                }
            };

            match s.send(&header.serialize().unwrap()) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("send get header failed : {}", e);
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
                        log::error!("recv data failed from ftp slave : {}", e);
                        break;
                    }
                };

                if data.is_empty() {
                    break;
                }

                match f.write_all(&data) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("write download file failed : {}", e);
                        break;
                    }
                };
                log::debug!("recv transfer data [{}]", data.len());
                let pos = match f.stream_position() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("get localfile size failed : {}", e);
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

pub fn upload_file(sender: &Sender<FTPPacket>, local_path: &str, remote_path: &str) -> Result<()> {
    let mut header = FTPPutHeader {
        path: remote_path.to_string(),
        total_size: 0,
        start_pos: 0,
    };

    let msg = RpcMessage::build_call("md5_file", vec![remote_path.to_string()]);
    send_ftp_packet(sender, build_ftp_rpc_packet(&msg)?)?;
    if let Ok(p) = G_RPCCLIENT.wait_msg(&msg.id, 10)
        && p.retcode == 0
    {
        let (remote_file_md5, remote_file_size) =
            (p.data[0].clone(), p.data[1].parse::<u64>().unwrap());

        let local_md5 = md5_file(vec![local_path.to_string()])?[0].clone();

        if remote_file_md5 == local_md5 {
            log::info!("resume broken transfer");
            header.start_pos = remote_file_size;
        }
    };

    let mut f = std::fs::File::options().read(true).open(local_path)?;

    header.total_size = f.metadata()?.len();

    let sender = sender.clone();
    let local_path = local_path.to_string();
    let remote_path = remote_path.to_string();
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
                    log::error!("send open tunnel msg failed : {}", e);
                    return;
                }
            }

            let (mut s, _) = match TcpConnection::tunnel_server(server, 10) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("create tunnel server failed : {}", e);
                    return;
                }
            };

            match s.send(&header.serialize().unwrap()) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("send get header failed : {}", e);
                    return;
                }
            };

            log::debug!("start get transfer [{}]", header.path);

            if header.start_pos != 0 {
                match f.seek(SeekFrom::Start(header.start_pos)) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("seek start pos failed : {}", e);
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
                        log::error!("read file failed : {}", e);
                        break;
                    }
                };

                if size == 0 {
                    break;
                }

                log::debug!("send file data [{}]", size);
                match s.send(&buf[..size]) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("get worker send to server failed : {}", e);
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
                        log::error!("get localfile size failed : {}", e);
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc::{Receiver, channel};
    use std::sync::{Mutex, MutexGuard};

    use super::*;

    static RPC_TEST_LOCK: Mutex<()> = Mutex::new(());
    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn unique_test_id() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .wrapping_add(u128::from(NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)))
    }

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "heroinn-ftp-controller-{}-{}",
                std::process::id(),
                unique_test_id()
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

    fn serial() -> MutexGuard<'static, ()> {
        RPC_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn respond_once(
        receiver: Receiver<FTPPacket>,
        expected_name: &'static str,
        retcode: i32,
        data: Vec<String>,
        message: &'static str,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let packet = receiver
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            assert_eq!(packet.id().unwrap(), FTPId::RPC);
            let request = RpcMessage::parse(&packet.data).unwrap();
            assert_eq!(request.name, expected_name);
            let mut response = RpcMessage {
                id: request.id,
                name: request.name,
                retcode,
                time: request.time,
                msg: message.to_string(),
                data,
            };
            response.time = cur_timestamp_secs();
            G_RPCCLIENT.write(&response);
        })
    }

    fn file_info(name: &str, typ: &str) -> FileInfo {
        FileInfo {
            name: name.to_string(),
            size: 3,
            typ: typ.to_string(),
            last_modified: "2026-01-01 00:00:00".to_string(),
        }
    }

    #[test]
    fn ftp_rpc_packet_uses_the_rpc_id_and_serialized_message() {
        let request = RpcMessage::build_call("name", vec!["value".to_string()]);
        let packet = build_ftp_rpc_packet(&request).unwrap();
        assert_eq!(packet.id, FTPId::RPC.to_u8());
        let decoded = RpcMessage::parse(&packet.data).unwrap();
        assert_eq!(decoded.id, request.id);
        assert_eq!(decoded.name, "name");
        assert_eq!(decoded.data, ["value"]);
    }

    #[test]
    fn send_packet_reports_a_closed_session_channel() {
        let (sender, receiver) = channel();
        drop(receiver);
        let error = send_ftp_packet(
            &sender,
            FTPPacket {
                id: FTPId::Close.to_u8(),
                data: vec![],
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("sender ftp packet failed"));
    }

    #[test]
    fn remote_discovery_and_folder_rpc_decode_entries_and_propagate_errors() {
        let _guard = serial();
        let (sender, receiver) = channel();
        let response = vec![
            file_info("disk", "SSD").serialize().unwrap(),
            file_info("folder", "FOLDER").serialize().unwrap(),
        ];
        let worker = respond_once(receiver, "get_disk_info", 0, response, "");
        let entries = get_remote_disk_info(&sender).unwrap();
        worker.join().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "disk");

        let (sender, receiver) = channel();
        let response = vec![file_info("remote", "FILE").serialize().unwrap()];
        let worker = respond_once(receiver, "get_folder_info", 0, response, "");
        let entries = get_remote_folder_info(&sender, "/tmp").unwrap();
        worker.join().unwrap();
        assert_eq!(entries[0].name, "remote");

        let (sender, receiver) = channel();
        let worker = respond_once(receiver, "get_folder_info", 3, vec![], "denied");
        let error = get_remote_folder_info(&sender, "/denied").unwrap_err();
        worker.join().unwrap();
        assert!(error.to_string().contains("denied"));
    }

    #[test]
    fn remote_join_and_delete_propagate_success_and_failure() {
        let _guard = serial();
        let (sender, receiver) = channel();
        let worker = respond_once(
            receiver,
            "join_path",
            0,
            vec!["/remote/file".to_string()],
            "",
        );
        assert_eq!(
            get_remote_join_path(&sender, "/remote", "file").unwrap(),
            "/remote/file"
        );
        worker.join().unwrap();

        let (sender, receiver) = channel();
        let worker = respond_once(receiver, "remove_file", 2, vec![], "remove denied");
        let error = delete_remote_file(&sender, "/remote/file").unwrap_err();
        worker.join().unwrap();
        assert!(error.to_string().contains("remove denied"));

        let (sender, receiver) = channel();
        let worker = respond_once(receiver, "remove_file", 0, vec![], "");
        delete_remote_file(&sender, "/remote/file").unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn local_discovery_join_listing_and_delete_use_the_local_filesystem() {
        let dir = TestDir::new();
        let folder = dir.0.join("folder");
        fs::create_dir(&folder).unwrap();
        let file = folder.join("file.txt");
        fs::write(&file, b"abc").unwrap();

        let disks = get_local_disk_info().unwrap();
        assert!(!disks.is_empty());
        let entries = get_local_folder_info(&folder.to_string_lossy()).unwrap();
        assert!(entries.iter().any(|entry| entry.name == "file.txt"));
        let joined = get_remote_join_path_for_test(&dir.0, "folder", "file.txt");
        assert!(joined.ends_with("folder/file.txt") || joined.ends_with("folder\\file.txt"));

        delete_local_file(&file.to_string_lossy()).unwrap();
        assert!(!file.exists());
        assert!(get_local_folder_info(&folder.to_string_lossy()).is_ok());
    }

    fn get_remote_join_path_for_test(root: &std::path::Path, first: &str, second: &str) -> String {
        heroinn_util::ftp::method::join_path(vec![
            root.join(first).to_string_lossy().into_owned(),
            second.to_string(),
        ])
        .unwrap()
        .remove(0)
    }

    #[test]
    fn local_errors_are_returned_instead_of_panicking() {
        let missing =
            std::env::temp_dir().join(format!("heroinn-ftp-missing-{}", unique_test_id()));
        assert!(get_local_folder_info(&missing.to_string_lossy()).is_err());
        assert!(delete_local_file(&missing.to_string_lossy()).is_err());
    }

    fn wait_until(timeout: std::time::Duration, mut predicate: impl FnMut() -> bool) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if predicate() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        predicate()
    }

    fn rpc_response(request: &RpcMessage, retcode: i32, data: Vec<String>, message: &str) {
        let response = RpcMessage {
            id: request.id.clone(),
            name: request.name.clone(),
            retcode,
            time: cur_timestamp_secs(),
            msg: message.to_string(),
            data,
        };
        G_RPCCLIENT.write(&response);
    }

    fn spawn_download_responder(
        receiver: Receiver<FTPPacket>,
        expected_rpc: &'static str,
        expected_data: Vec<String>,
        remote_path: &'static str,
        payload: &'static [u8],
        expected_start: u64,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let packet = receiver
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            assert_eq!(packet.id().unwrap(), FTPId::RPC);
            let request = RpcMessage::parse(&packet.data).unwrap();
            assert_eq!(request.name, expected_rpc);
            rpc_response(&request, 0, expected_data, "");

            let packet = receiver
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            assert_eq!(packet.id().unwrap(), FTPId::Get);
            let request = TunnelRequest::parse(&packet.data).unwrap();
            let mut tunnel =
                TcpConnection::connect(&format!("127.0.0.1:{}", request.port)).unwrap();
            let header = FTPGetHeader::parse(&tunnel.recv().unwrap()).unwrap();
            assert_eq!(header.path, remote_path);
            assert_eq!(header.start_pos, expected_start);
            tunnel.send(&payload[expected_start as usize..]).unwrap();
            tunnel.close();
        })
    }

    #[test]
    fn download_file_creates_the_target_and_writes_all_frames() {
        let _guard = serial();
        let dir = TestDir::new();
        let local = dir.0.join("download.bin");
        let local_string = local.to_string_lossy().into_owned();
        let payload = b"download payload";
        let (sender, receiver) = channel();
        let responder = spawn_download_responder(
            receiver,
            "file_size",
            vec![payload.len().to_string()],
            "/remote/download.bin",
            payload,
            0,
        );

        download_file(&sender, &local_string, "/remote/download.bin").unwrap();
        assert!(wait_until(std::time::Duration::from_secs(5), || {
            fs::read(&local).ok().as_deref() == Some(payload)
        }));
        responder.join().unwrap();
        assert!(wait_until(std::time::Duration::from_secs(5), || {
            !G_TRANSFER.read().unwrap().contains_key(&local_string)
        }));
    }

    #[test]
    fn download_file_resumes_a_matching_partial_file() {
        let _guard = serial();
        let dir = TestDir::new();
        let local = dir.0.join("resume.bin");
        let local_string = local.to_string_lossy().into_owned();
        let payload = b"resumed payload";
        fs::write(&local, &payload[..4]).unwrap();
        let local_md5 = md5_file(vec![local_string.clone()]).unwrap()[0].clone();
        let (sender, receiver) = channel();
        let responder = spawn_download_responder(
            receiver,
            "md5_file",
            vec![local_md5, payload.len().to_string()],
            "/remote/resume.bin",
            payload,
            4,
        );

        download_file(&sender, &local_string, "/remote/resume.bin").unwrap();
        assert!(wait_until(std::time::Duration::from_secs(5), || {
            fs::read(&local).ok().as_deref() == Some(payload)
        }));
        responder.join().unwrap();
    }

    fn spawn_upload_responder(
        receiver: Receiver<FTPPacket>,
        rpc_response_data: Vec<String>,
        rpc_retcode: i32,
        remote_path: &'static str,
        expected_start: u64,
        payload: &'static [u8],
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let packet = receiver
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            assert_eq!(packet.id().unwrap(), FTPId::RPC);
            let request = RpcMessage::parse(&packet.data).unwrap();
            assert_eq!(request.name, "md5_file");
            rpc_response(&request, rpc_retcode, rpc_response_data, "");

            let packet = receiver
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            assert_eq!(packet.id().unwrap(), FTPId::Put);
            let request = TunnelRequest::parse(&packet.data).unwrap();
            let mut tunnel =
                TcpConnection::connect(&format!("127.0.0.1:{}", request.port)).unwrap();
            let header = FTPPutHeader::parse(&tunnel.recv().unwrap()).unwrap();
            assert_eq!(header.path, remote_path);
            assert_eq!(header.total_size, payload.len() as u64);
            assert_eq!(header.start_pos, expected_start);

            let expected = &payload[expected_start as usize..];
            let mut received = Vec::new();
            while received.len() < expected.len() {
                let chunk = tunnel.recv().unwrap();
                assert!(!chunk.is_empty());
                received.extend_from_slice(&chunk);
            }
            assert_eq!(received.as_slice(), expected);
            tunnel.close();
        })
    }

    #[test]
    fn upload_file_streams_the_complete_file() {
        let _guard = serial();
        let dir = TestDir::new();
        let local = dir.0.join("upload.bin");
        let local_string = local.to_string_lossy().into_owned();
        let payload = b"upload payload";
        fs::write(&local, payload).unwrap();
        let (sender, receiver) = channel();
        let responder =
            spawn_upload_responder(receiver, vec![], 1, "/remote/upload.bin", 0, payload);

        upload_file(&sender, &local_string, "/remote/upload.bin").unwrap();
        responder.join().unwrap();
        assert!(wait_until(std::time::Duration::from_secs(5), || {
            !G_TRANSFER.read().unwrap().contains_key(&local_string)
        }));
    }

    #[test]
    fn upload_file_resumes_from_the_remote_byte_offset() {
        let _guard = serial();
        let dir = TestDir::new();
        let local = dir.0.join("upload-resume.bin");
        let local_string = local.to_string_lossy().into_owned();
        let payload = b"resumed upload payload";
        fs::write(&local, payload).unwrap();
        let local_md5 = md5_file(vec![local_string.clone()]).unwrap()[0].clone();
        let (sender, receiver) = channel();
        let responder = spawn_upload_responder(
            receiver,
            vec![local_md5, "5".to_string()],
            0,
            "/remote/upload-resume.bin",
            5,
            payload,
        );

        upload_file(&sender, &local_string, "/remote/upload-resume.bin").unwrap();
        responder.join().unwrap();
        assert!(wait_until(std::time::Duration::from_secs(5), || {
            !G_TRANSFER.read().unwrap().contains_key(&local_string)
        }));
    }
}
