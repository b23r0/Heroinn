use heroinn_util::{
    ftp::{method::*, FTPGetHeader, FTPId, FTPPacket, FTPPutHeader},
    packet::TunnelRequest,
    protocol::create_tunnel,
    rpc::{RpcMessage, RpcServer},
    session::{Session, SessionBase, SessionPacket},
    HeroinnProtocol,
};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    sync::{atomic::AtomicBool, mpsc::Sender, Arc},
};

use crate::config::master_configure;

pub struct FtpClient {
    id: String,
    clientid: String,
    closed: Arc<AtomicBool>,
    rpc_server: RpcServer,
    sender: Sender<SessionBase>,
}

impl Session for FtpClient {
    fn new_client(
        sender: std::sync::mpsc::Sender<heroinn_util::session::SessionBase>,
        clientid: &String,
        id: &String,
    ) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let mut rpc_server = RpcServer::new();
        rpc_server.register(&"get_disk_info".to_string(), get_disk_info);
        rpc_server.register(&"get_folder_info".to_string(), get_folder_info);
        rpc_server.register(&"join_path".to_string(), join_path);
        rpc_server.register(&"remove_file".to_string(), remove_file);
        rpc_server.register(&"file_size".to_string(), file_size);
        rpc_server.register(&"md5_file".to_string(), md5_file);
        Ok(Self {
            id: id.clone(),
            clientid: clientid.clone(),
            closed: Arc::new(AtomicBool::new(false)),
            rpc_server,
            sender,
        })
    }

    fn new(
        _: std::sync::mpsc::Sender<heroinn_util::session::SessionBase>,
        _: &String,
        _: &String,
    ) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not server",
        ))
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn write(&mut self, data: &Vec<u8>) -> std::io::Result<()> {
        let packet = FTPPacket::parse(data)?;

        match packet.id() {
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
                    self.closed
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                };
            }
            FTPId::Get => {
                let packet = TunnelRequest::parse(&packet.data)?;

                let closed = self.closed.clone();

                std::thread::spawn(move || {
                    let config = master_configure();

                    let mut client = match create_tunnel(
                        &config.address,
                        &HeroinnProtocol::from(config.protocol),
                        packet.port,
                    ) {
                        Ok(p) => p,
                        Err(e) => {
                            log::error!("create tunnel faild : {}", e);
                            return;
                        }
                    };
                    log::debug!("create tunnel success");
                    let header = match client.recv() {
                        Ok(p) => p,
                        Err(e) => {
                            log::error!("recv get header tunnel faild : {}", e);
                            return;
                        }
                    };

                    let header = FTPGetHeader::parse(&header).unwrap();

                    let mut f = match std::fs::File::open(&header.path) {
                        Ok(p) => p,
                        Err(e) => {
                            log::error!("open file faild : {}", e);
                            return;
                        }
                    };

                    if header.start_pos != 0 {
                        match f.seek(SeekFrom::Start(header.start_pos)) {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("seek file faild : {}", e);
                                return;
                            }
                        };
                    }

                    log::debug!("start get transfer [{}]", header.path);
                    loop {
                        if closed.load(std::sync::atomic::Ordering::Relaxed) {
                            log::error!("session closed");
                            break;
                        }

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
                        match client.send(&mut buf[..size]) {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("get worker send to server faild : {}", e);
                                break;
                            }
                        };
                    }

                    log::info!("get file worker finished");
                });
            }
            FTPId::Put => {
                let packet = TunnelRequest::parse(&packet.data)?;
                let closed = self.closed.clone();

                std::thread::spawn(move || {
                    let config = master_configure();

                    let mut client = match create_tunnel(
                        &config.address,
                        &HeroinnProtocol::from(config.protocol),
                        packet.port,
                    ) {
                        Ok(p) => p,
                        Err(e) => {
                            log::error!("create tunnel faild : {}", e);
                            return;
                        }
                    };
                    log::debug!("create tunnel success");

                    let header = match client.recv() {
                        Ok(p) => p,
                        Err(e) => {
                            log::error!("recv get header faild : {}", e);
                            return;
                        }
                    };

                    let header = match FTPPutHeader::parse(&header) {
                        Ok(p) => p,
                        Err(e) => {
                            log::error!("parse get header faild : {}", e);
                            return;
                        }
                    };

                    let mut f = if header.start_pos == 0 {
                        match std::fs::File::create(&header.path) {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("create remote file faild [{}] : {}", header.path, e);
                                return;
                            }
                        }
                    } else {
                        let mut f = match std::fs::File::options().write(true).open(&header.path) {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("open remote file faild [{}] : {}", header.path, e);
                                return;
                            }
                        };

                        match f.seek(SeekFrom::Start(header.start_pos)) {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("seek remote file faild [{}] : {}", header.path, e);
                                return;
                            }
                        };
                        f
                    };

                    log::debug!("start put transfer [{}]", header.path);
                    loop {
                        if closed.load(std::sync::atomic::Ordering::Relaxed) {
                            log::error!("session closed");
                            break;
                        }

                        let data = match client.recv() {
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

                        if pos >= header.total_size {
                            break;
                        }
                    }
                });
            }
            FTPId::Close => {
                self.close();
            }
            FTPId::Unknown => {}
        }

        Ok(())
    }

    fn alive(&self) -> bool {
        !self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn close(&mut self) {
        log::info!("ftp session closed");
        self.closed
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn clientid(&self) -> String {
        self.clientid.clone()
    }
}
