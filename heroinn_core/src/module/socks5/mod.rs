//! Server-side ownership of one reverse SOCKS5 GUI sidecar.

mod sidecar;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use heroinn_util::session::{Session, SessionPacket};
use heroinn_util::socks5::{
    Socks5ClientEvent, Socks5ServerCommand, Socks5SidecarCommand, Socks5SidecarEvent,
    Socks5SidecarReady,
};
use heroinn_util::{Error, Result};
use uuid::Uuid;

use sidecar::{SidecarInstance, SidecarProcess};
pub use sidecar::{SidecarProcessError, locate_sidecar, sidecar_available, sidecar_binary_name};

pub type ServerCommandSender = Arc<dyn Fn(&str, Socks5ServerCommand) + Send + Sync>;

/// One live reverse-SOCKS5 GUI owned by a connected client.
pub struct Socks5Server {
    id: String,
    clientid: String,
    closed: Arc<AtomicBool>,
    control: Arc<Mutex<std::net::TcpStream>>,
    process: SidecarProcess,
    ready: Socks5SidecarReady,
}

impl Socks5Server {
    pub fn new(
        clientid: &str,
        peer_addr: &str,
        proxy_chain: &str,
        transport: &str,
        send_command: ServerCommandSender,
    ) -> Result<Self> {
        let driver_path = locate_sidecar()
            .map_err(|error| Error::Other(format!("SOCKS5 sidecar unavailable: {error}")))?;
        let session_id = Uuid::new_v4().to_string();
        let instance =
            SidecarInstance::new(&driver_path, &session_id, peer_addr, proxy_chain, transport)
                .map_err(|error| Error::Other(format!("start SOCKS5 sidecar failed: {error}")))?;

        let ready = instance.ready().clone();
        log::info!(
            "SOCKS5 sidecar control ready session={} listener={} tunnel_port={}",
            session_id,
            if ready.socks_address.is_empty() {
                "stopped"
            } else {
                ready.socks_address.as_str()
            },
            ready.tunnel_port
        );

        let control = instance.control();
        let closed = instance.closed_flag();
        let process = instance.into_process();

        let server = Self {
            id: session_id.clone(),
            clientid: clientid.to_string(),
            closed: closed.clone(),
            control,
            process,
            ready,
        };
        server.spawn_control_reader(send_command);
        Ok(server)
    }

    pub fn ready(&self) -> &Socks5SidecarReady {
        &self.ready
    }

    fn spawn_control_reader(&self, send_command: ServerCommandSender) {
        let control = self.control.clone();
        let closed = self.closed.clone();
        let clientid = self.clientid.clone();
        let session_id = self.id.clone();
        let tunnel_port = self.ready.tunnel_port;
        std::thread::Builder::new()
            .name(format!("socks5-sidecar-control-{session_id}"))
            .spawn(move || {
                let mut socket = {
                    let stream = match control.lock() {
                        Ok(stream) => stream,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    match stream.try_clone() {
                        Ok(stream) => stream,
                        Err(error) => {
                            log::error!("SOCKS5 sidecar control clone failed: {error}");
                            closed.store(true, Ordering::SeqCst);
                            return;
                        }
                    }
                };
                loop {
                    if closed.load(Ordering::SeqCst) {
                        break;
                    }
                    let payload = match heroinn_util::protocol::frame::read_frame(&mut socket) {
                        Ok(payload) => payload,
                        Err(error) => {
                            log::info!("SOCKS5 sidecar control closed: {error}");
                            break;
                        }
                    };
                    let event = match serde_json::from_slice::<Socks5SidecarEvent>(&payload) {
                        Ok(event) => event,
                        Err(error) => {
                            log::warn!("malformed SOCKS5 sidecar event: {error}");
                            continue;
                        }
                    };
                    match event {
                        Socks5SidecarEvent::Open { stream_id, target } => {
                            send_command(
                                &clientid,
                                Socks5ServerCommand::Open(heroinn_util::socks5::Socks5ServerOpen {
                                    session_id: session_id.clone(),
                                    stream_id,
                                    target,
                                    tunnel_port,
                                }),
                            );
                        }
                        Socks5SidecarEvent::Stopped { reason } => {
                            log::info!("SOCKS5 sidecar stopped: {}", sanitize_reason(&reason));
                            break;
                        }
                        Socks5SidecarEvent::Ready(_) => {}
                    }
                }
                closed.store(true, Ordering::SeqCst);
                send_command(
                    &clientid,
                    Socks5ServerCommand::Shutdown {
                        session_id: session_id.clone(),
                    },
                );
            })
            .ok();
    }
}

impl Session for Socks5Server {
    fn id(&self) -> &str {
        &self.id
    }

    fn clientid(&self) -> &str {
        &self.clientid
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        let event: Socks5ClientEvent = serde_json::from_slice(data)?;
        let command = match event {
            Socks5ClientEvent::OpenResult {
                stream_id,
                accepted,
                category,
                detail,
                ..
            } => Some(Socks5SidecarCommand::OpenResult {
                stream_id,
                accepted,
                category,
                detail,
            }),
            Socks5ClientEvent::Closed {
                stream_id,
                bytes_to_target,
                bytes_from_target,
                ..
            } => Some(Socks5SidecarCommand::Closed {
                stream_id,
                bytes_to_target,
                bytes_from_target,
            }),
            Socks5ClientEvent::ShutdownAck { .. } => None,
        };
        if let Some(command) = command {
            self.write_command(command)?;
        }
        Ok(())
    }

    fn alive(&self) -> bool {
        !self.closed.load(Ordering::SeqCst) && self.process.alive()
    }

    fn close(&mut self) {
        if self.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        let _ = self.write_command(Socks5SidecarCommand::Shutdown);
        self.process.close();
    }
}

impl Socks5Server {
    fn write_command(&self, command: Socks5SidecarCommand) -> Result<()> {
        let payload = serde_json::to_vec(&command)?;
        let mut socket = match self.control.lock() {
            Ok(socket) => socket,
            Err(poisoned) => poisoned.into_inner(),
        };
        heroinn_util::protocol::frame::write_frame(&mut *socket, &payload)
    }
}

impl Drop for Socks5Server {
    fn drop(&mut self) {
        self.close();
    }
}

fn sanitize_reason(reason: &str) -> String {
    heroinn_util::socks5::sanitize_detail(reason)
}

pub fn session_packet_id(data: &[u8]) -> Result<String> {
    Ok(serde_json::from_slice::<SessionPacket>(data)?.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_packet_id_requires_a_valid_packet() {
        let packet = SessionPacket {
            id: "session-id".to_string(),
            data: vec![1, 2, 3],
        };
        assert_eq!(
            session_packet_id(&serde_json::to_vec(&packet).unwrap()).unwrap(),
            "session-id"
        );
        assert!(session_packet_id(b"not-json").is_err());
        assert!(session_packet_id(br#"{"data":[]}"#).is_err());
    }

    #[test]
    fn sidecar_reason_redacts_userinfo_and_control_characters() {
        let reason = "failed\nhttps://user:secret@example.com:8443/path\r\u{1b}[31m";
        let sanitized = sanitize_reason(reason);
        assert!(!sanitized.contains("secret"));
        assert!(!sanitized.contains("user"));
        assert!(!sanitized.contains('\n'));
        assert!(!sanitized.contains('\r'));
        assert!(!sanitized.contains('\u{1b}'));
        assert!(sanitized.len() <= 512);
    }
}
