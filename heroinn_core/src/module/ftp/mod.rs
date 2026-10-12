//! Server-side file-transfer session.
//!
//! Each session owns a `heroinn_ftp` worker process and relays its framed
//! messages to the remote client through a [`SessionBase`] channel.

mod ftp_port;

use std::env::current_dir;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::Sender};

use heroinn_util::Result;
use heroinn_util::ftp::{FTPId, FTPPacket};
use heroinn_util::session::{ServerSession, Session, SessionBase, SessionPacket};

use self::ftp_port::FtpInstance;

/// A live file-transfer channel backed by a worker process.
pub struct FtpServer {
    id: String,
    clientid: String,
    closed: Arc<AtomicBool>,
    sender: Sender<SessionBase>,
    instance: FtpInstance,
}

impl ServerSession for FtpServer {
    fn new(sender: Sender<SessionBase>, clientid: &str, peer_addr: &str) -> Result<Self> {
        let closed = Arc::new(AtomicBool::new(false));

        let driver = if cfg!(target_os = "windows") {
            "heroinn_ftp.exe"
        } else {
            "heroinn_ftp"
        };

        let driver_path = current_dir()
            .unwrap()
            .join(driver)
            .to_str()
            .unwrap()
            .to_string();

        let ftp = FtpInstance::new(&driver_path, peer_addr)?;

        let mut ftp_1 = ftp.clone();
        let closed_2 = closed.clone();
        std::thread::spawn(move || {
            ftp_1.wait_for_exit().unwrap();
            closed_2.store(true, Ordering::Relaxed);
        });

        let id = uuid::Uuid::new_v4().to_string();
        let id_1 = id.clone();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.to_string();
        let mut ftp_2 = ftp.clone();
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                if closed_1.load(Ordering::Relaxed) {
                    break;
                }

                let buf = match ftp_2.read() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("ftp instance read error : {}", e);
                        break;
                    }
                };

                log::debug!("recv msg from ftp instance");

                let packet = SessionPacket {
                    id: id_1.clone(),
                    data: buf,
                };

                if let Err(e) = sender_1.send(SessionBase {
                    id: id_1.clone(),
                    clientid: clientid_1.clone(),
                    packet,
                }) {
                    log::info!("sender closed : {}", e);
                    break;
                }
            }
            log::info!("ftp worker closed");
            closed_1.store(true, Ordering::Relaxed);
        });

        Ok(Self {
            id,
            clientid: clientid.to_string(),
            closed,
            sender,
            instance: ftp,
        })
    }
}

impl Session for FtpServer {
    fn id(&self) -> &str {
        &self.id
    }

    fn clientid(&self) -> &str {
        &self.clientid
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        log::debug!("write msg from ftp client");
        self.instance.write(data)
    }

    fn alive(&self) -> bool {
        !self.closed.load(Ordering::Relaxed)
    }

    fn close(&mut self) {
        let packet = SessionPacket {
            id: self.id.clone(),
            data: FTPPacket {
                id: FTPId::Close.to_u8(),
                data: vec![],
            }
            .serialize()
            .unwrap(),
        };

        if let Err(e) = self.sender.send(SessionBase {
            id: self.id.clone(),
            clientid: self.clientid.clone(),
            packet,
        }) {
            log::info!("sender close msg error: {}", e);
        }

        self.instance.close().unwrap();
        log::info!("ftp session closed");
        self.closed.store(true, Ordering::Relaxed);
    }
}
