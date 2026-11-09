//! Server-side interactive shell session.
//!
//! Each session owns a `heroinn_shell` worker process; bytes flow between the
//! worker's local socket and the remote client through a [`SessionBase`] channel.

mod shell_port;

use std::ffi::OsStr;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::Sender};

use heroinn_shell_protocol::{Control, control_prefix, encode_close};
use heroinn_util::Result;
use heroinn_util::session::{ServerSession, Session, SessionBase, SessionPacket};

use self::shell_port::{TermInstance, resolve_shell_path};

fn read_worker_packet(reader: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
    let mut buffer = [0u8; 1024];
    match reader.read(&mut buffer) {
        Ok(0) => Ok(None),
        Ok(size) => Ok(Some(buffer[..size].to_vec())),
        Err(error) => Err(error),
    }
}

/// A live shell backed by a worker process.
pub struct ShellServer {
    id: String,
    clientid: String,
    closed: Arc<AtomicBool>,
    close_started: bool,
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

        let driver_path = resolve_shell_path(OsStr::new(driver))
            .map_err(|e| heroinn_util::Error::Other(e.to_string()))?;
        let term = TermInstance::new(&driver_path, peer_addr).map_err(|e| {
            log::error!("failed to launch heroinn_shell: {e}");
            heroinn_util::Error::Other(e.to_string())
        })?;

        let term_1 = term.try_clone().map_err(|e| {
            heroinn_util::Error::Other(format!("clone shell wait socket failed: {e}"))
        })?;
        let closed_2 = closed.clone();
        std::thread::spawn(move || {
            if let Err(error) = term_1.wait_for_exit() {
                log::warn!("wait for shell process failed: {error}");
            }
            closed_2.store(true, Ordering::Relaxed);
        });

        let id = uuid::Uuid::new_v4().to_string();
        let id_1 = id.clone();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.to_string();
        let mut term_2 = term.try_clone().map_err(|e| {
            heroinn_util::Error::Other(format!("clone shell read socket failed: {e}"))
        })?;
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                if closed_1.load(Ordering::Relaxed) {
                    break;
                }

                let data = match read_worker_packet(&mut term_2) {
                    Ok(None) => {
                        log::info!("shell worker socket reached EOF");
                        break;
                    }
                    Ok(Some(data)) => data,
                    Err(e) => {
                        log::error!("term instance read error : {}", e);
                        break;
                    }
                };

                let packet = SessionPacket {
                    id: id_1.clone(),
                    data,
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
            if let Err(error) = term_2.close() {
                log::debug!("close shell worker after reader exit failed: {error}");
            }
            closed_1.store(true, Ordering::Relaxed);
        });

        Ok(Self {
            id,
            closed,
            close_started: false,
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
        let mut remaining = data;
        while let Some((control, consumed)) = control_prefix(remaining) {
            remaining = &remaining[consumed..];
            match control {
                Control::Close => {
                    log::info!("client closed");
                    self.close();
                    return Ok(());
                }
                Control::Resize(_) => {
                    // Resize packets travel from this window to the remote PTY
                    // and are never valid shell input.
                }
                Control::Data(_) => {}
            }
        }

        if !remaining.is_empty() {
            self.term.write(remaining)?;
        }

        Ok(())
    }

    fn alive(&self) -> bool {
        !self.closed.load(Ordering::Relaxed)
    }

    fn close(&mut self) {
        if self.close_started {
            return;
        }
        self.close_started = true;

        log::info!("shell closed");

        let packet = SessionPacket {
            id: self.id.clone(),
            data: encode_close().to_vec(),
        };

        let _ = self.sender.send(SessionBase {
            id: self.id.clone(),
            clientid: self.clientid.clone(),
            packet,
        });

        if let Err(error) = self.term.close() {
            log::warn!("close shell worker failed: {error}");
        }
        self.closed.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::read_worker_packet;

    #[test]
    fn worker_eof_produces_no_empty_packet() {
        let mut input = Cursor::new(b"terminal output".to_vec());
        assert_eq!(
            read_worker_packet(&mut input).unwrap(),
            Some(b"terminal output".to_vec())
        );
        assert_eq!(read_worker_packet(&mut input).unwrap(), None);
    }
}
