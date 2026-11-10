//! Server-side file-transfer session.
//!
//! Each session owns a `heroinn_ftp` worker process and relays its framed
//! messages to the remote client through a [`SessionBase`] channel.

mod ftp_port;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::Sender};

use heroinn_util::ftp::{FTPId, FTPPacket};
use heroinn_util::session::{ServerSession, Session, SessionBase, SessionPacket};
use heroinn_util::{Error, Result};

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
        let driver = if cfg!(target_os = "windows") {
            "heroinn_ftp.exe"
        } else {
            "heroinn_ftp"
        };

        let driver_path = ftp_port::locate_sidecar(driver).map_err(Error::Other)?;

        let ftp = FtpInstance::new(&driver_path, peer_addr)?;
        Self::from_instance(sender, clientid, ftp)
    }
}

impl FtpServer {
    fn from_instance(
        sender: Sender<SessionBase>,
        clientid: &str,
        ftp: FtpInstance,
    ) -> Result<Self> {
        let closed = Arc::new(AtomicBool::new(false));
        let mut ftp_1 = ftp.try_clone()?;
        let closed_2 = closed.clone();
        std::thread::spawn(move || {
            if let Err(error) = ftp_1.wait_for_exit() {
                log::warn!("ftp worker exit wait failed: {error}");
            }
            closed_2.store(true, Ordering::Relaxed);
        });

        let id = uuid::Uuid::new_v4().to_string();
        let id_1 = id.clone();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.to_string();
        let mut ftp_2 = ftp.try_clone()?;
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

        if let Err(error) = self.instance.close() {
            log::warn!("close ftp worker failed: {error}");
        }
        log::info!("ftp session closed");
        self.closed.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::process::{Child, Command, Stdio};
    use std::sync::mpsc::{Receiver, channel};
    use std::time::{Duration, Instant};

    use heroinn_util::ftp::FTPId;
    use heroinn_util::protocol::frame::{read_frame, write_frame};

    fn idle_child() -> Child {
        let mut command = if cfg!(windows) {
            Command::new("cmd")
        } else {
            Command::new("sh")
        };
        if cfg!(windows) {
            command.args(["/C", "ping -n 30 127.0.0.1 >NUL"]);
        } else {
            command.args(["-c", "sleep 30"]);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }

    fn connected_server() -> (FtpServer, TcpStream, Receiver<SessionBase>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let peer = std::thread::spawn(move || TcpStream::connect(address).unwrap());
        let (socket, _) = listener.accept().unwrap();
        let peer = peer.join().unwrap();
        let instance = FtpInstance::from_test_parts(socket, idle_child());
        let (sender, receiver) = channel();
        let server = FtpServer::from_instance(sender, "client-a", instance).unwrap();
        (server, peer, receiver)
    }

    fn wait_until(mut predicate: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if predicate() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn relay_forwards_worker_frames_and_close_packet_once() {
        let (mut server, mut peer, receiver) = connected_server();
        assert!(server.alive());
        assert_eq!(server.id().len(), 36);
        assert_eq!(server.clientid(), "client-a");

        server.write(b"client-to-worker").unwrap();
        assert_eq!(read_frame(&mut peer).unwrap(), b"client-to-worker");

        write_frame(&mut peer, b"worker-to-client").unwrap();
        let base = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(base.id, server.id());
        assert_eq!(base.clientid, "client-a");
        assert_eq!(base.packet.id, server.id());
        assert_eq!(base.packet.data, b"worker-to-client");

        drop(peer);
        assert!(wait_until(|| !server.alive()));

        server.close();
        assert!(!server.alive());
        let close = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        let packet = FTPPacket::parse(&close.packet.data).unwrap();
        assert_eq!(packet.id().unwrap(), FTPId::Close);
        assert!(packet.data.is_empty());
    }

    #[test]
    fn worker_eof_marks_the_session_dead_without_an_empty_packet() {
        let (server, peer, receiver) = connected_server();
        drop(peer);

        assert!(wait_until(|| !server.alive()));
        assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
    }
}
