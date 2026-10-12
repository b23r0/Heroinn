//! Server-side interactive shell session.
//!
//! Each session owns a `heroinn_shell` worker process; bytes flow between the
//! worker's local socket and the remote client through a [`SessionBase`] channel.

mod shell_port;

use std::env::current_dir;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::Sender};

use heroinn_util::Result;
use heroinn_util::session::{ServerSession, Session, SessionBase, SessionPacket};

use self::shell_port::TermInstance;

/// Magic prefix marking an in-band control packet instead of shell output.
static MAGIC_FLAG: [u8; 2] = [0x37, 0x37];

/// A live shell backed by a worker process.
pub struct ShellServer {
    id: String,
    clientid: String,
    closed: Arc<AtomicBool>,
    term: TermInstance,
    sender: Sender<SessionBase>,
}

impl ServerSession for ShellServer {
    fn new(sender: Sender<SessionBase>, clientid: &str, peer_addr: &str) -> Result<Self> {
        let closed = Arc::new(AtomicBool::new(false));

        let driver = if cfg!(target_os = "windows") {
            "heroinn_shell.exe"
        } else {
            "heroinn_shell"
        };

        let driver_path = current_dir()
            .unwrap()
            .join(driver)
            .to_str()
            .unwrap()
            .to_string();

        let term = TermInstance::new(&driver_path, peer_addr)
            .map_err(|e| heroinn_util::Error::Other(e.to_string()))?;

        let mut term_1 = term.clone();
        let closed_2 = closed.clone();
        std::thread::spawn(move || {
            term_1.wait_for_exit().unwrap();
            closed_2.store(true, Ordering::Relaxed);
        });

        let id = uuid::Uuid::new_v4().to_string();
        let id_1 = id.clone();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.to_string();
        let mut term_2 = term.clone();
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                if closed_1.load(Ordering::Relaxed) {
                    break;
                }

                let mut buf = [0u8; 1024];
                let size = match term_2.read(&mut buf) {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("term instance read error : {}", e);
                        break;
                    }
                };

                let packet = SessionPacket {
                    id: id_1.clone(),
                    data: buf[..size].to_vec(),
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
            log::info!("shell worker closed");
            closed_1.store(true, Ordering::Relaxed);
        });

        Ok(Self {
            id,
            closed,
            clientid: clientid.to_string(),
            term,
            sender,
        })
    }
}

impl Session for ShellServer {
    fn id(&self) -> &str {
        &self.id
    }

    fn clientid(&self) -> &str {
        &self.clientid
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        if data.len() == 3 && data[..2] == MAGIC_FLAG && data[2] == 0xff {
            log::info!("client closed");
            self.close();
            return Ok(());
        }

        Ok(self.term.write(data)?)
    }

    fn alive(&self) -> bool {
        !self.closed.load(Ordering::Relaxed)
    }

    fn close(&mut self) {
        log::info!("shell closed");

        let packet = SessionPacket {
            id: self.id.clone(),
            data: vec![MAGIC_FLAG[0], MAGIC_FLAG[1], 0xff],
        };

        let _ = self.sender.send(SessionBase {
            id: self.id.clone(),
            clientid: self.clientid.clone(),
            packet,
        });

        self.term.close().unwrap();
        self.closed.store(true, Ordering::Relaxed);
    }
}
