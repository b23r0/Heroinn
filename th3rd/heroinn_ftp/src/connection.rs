use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{RecvTimeoutError, Sender, channel},
};
use std::thread::JoinHandle;
use std::time::Duration;

use heroinn_util::{
    ftp::{FTPId, FTPPacket},
    protocol::tcp::TcpConnection,
    rpc::RpcMessage,
};

pub struct FtpWorkers {
    connection: TcpConnection,
    stopped: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl FtpWorkers {
    pub fn start(connection: TcpConnection) -> std::io::Result<(Self, Sender<FTPPacket>)> {
        let mut workers = Self {
            connection,
            stopped: Arc::new(AtomicBool::new(false)),
            threads: Vec::new(),
        };
        let mut reader = workers.connection.clone();
        let receiver_stopped = workers.stopped.clone();
        workers.threads.push(
            std::thread::Builder::new()
                .name("ftp receiver worker".to_string())
                .spawn(move || {
                    let mut partial = Vec::new();
                    while !receiver_stopped.load(Ordering::Acquire) {
                        let data = match reader.recv_poll(&mut partial, Duration::from_millis(100))
                        {
                            Ok(Some(data)) => data,
                            Ok(None) => continue,
                            Err(_) => break,
                        };
                        let packet = match FTPPacket::parse(&data) {
                            Ok(packet) => packet,
                            Err(error) => {
                                log::warn!("invalid local FTP packet: {error}");
                                break;
                            }
                        };
                        match packet.id() {
                            Ok(FTPId::RPC) => match RpcMessage::parse(&packet.data) {
                                Ok(message) => crate::G_RPCCLIENT.write(&message),
                                Err(error) => {
                                    log::warn!("invalid local FTP RPC: {error}");
                                    break;
                                }
                            },
                            Ok(FTPId::Close) => break,
                            Ok(FTPId::Get | FTPId::Put) => {}
                            Err(error) => log::warn!("unknown ftp packet id: {error}"),
                        }
                    }
                    receiver_stopped.store(true, Ordering::Release);
                })?,
        );

        let (sender, receiver) = channel::<FTPPacket>();
        let mut writer = workers.connection.clone();
        let sender_stopped = workers.stopped.clone();
        workers.threads.push(
            std::thread::Builder::new()
                .name("ftp sender worker".to_string())
                .spawn(move || {
                    while !sender_stopped.load(Ordering::Acquire) {
                        let packet = match receiver.recv_timeout(Duration::from_millis(100)) {
                            Ok(packet) => packet,
                            Err(RecvTimeoutError::Timeout) => continue,
                            Err(RecvTimeoutError::Disconnected) => break,
                        };
                        let result = packet.serialize().and_then(|data| writer.send(&data));
                        if let Err(error) = result {
                            log::warn!("local FTP send failed: {error}");
                            break;
                        }
                    }
                    sender_stopped.store(true, Ordering::Release);
                })?,
        );
        Ok((workers, sender))
    }

    pub fn stopped(&self) -> Arc<AtomicBool> {
        self.stopped.clone()
    }

    pub fn shutdown(&mut self) {
        self.stopped.store(true, Ordering::Release);
        self.connection.close();
        for thread in self.threads.drain(..) {
            thread.join().expect("FTP connection worker panicked");
        }
    }
}

impl Drop for FtpWorkers {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heroinn_util::protocol::frame::{read_frame, write_frame};
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc::channel;
    use std::time::Instant;

    fn connected_workers() -> (FtpWorkers, Sender<FTPPacket>, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let connection =
            TcpConnection::connect_local_plain(&listener.local_addr().unwrap().to_string())
                .unwrap();
        let (peer, _) = listener.accept().unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let (workers, sender) = FtpWorkers::start(connection).unwrap();
        (workers, sender, peer)
    }

    fn wait_stopped(workers: &FtpWorkers) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !workers.stopped.load(Ordering::Acquire) {
            assert!(
                Instant::now() < deadline,
                "FTP worker did not report closure"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn shutdown_quickly(workers: &mut FtpWorkers) {
        let started = Instant::now();
        workers.shutdown();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(workers.threads.is_empty());
    }

    #[test]
    fn dropping_gui_sender_returns_workers_without_exiting_the_process() {
        let (mut workers, sender, mut peer) = connected_workers();
        let (dropped, receiver) = channel();
        struct GuiResource(Sender<()>);
        impl Drop for GuiResource {
            fn drop(&mut self) {
                self.0.send(()).unwrap();
            }
        }
        let resource = GuiResource(dropped);
        drop(sender);
        wait_stopped(&workers);
        assert!(receiver.try_recv().is_err());
        drop(resource);
        receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        shutdown_quickly(&mut workers);
        assert!(workers.threads.is_empty());
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    }

    #[test]
    fn remote_close_requests_gui_shutdown_without_exiting_the_process() {
        let (mut workers, _sender, mut peer) = connected_workers();
        let packet = FTPPacket {
            id: FTPId::Close.to_u8(),
            data: Vec::new(),
        };
        write_frame(&mut peer, &packet.serialize().unwrap()).unwrap();
        wait_stopped(&workers);
        shutdown_quickly(&mut workers);
        assert!(workers.threads.is_empty());
    }

    #[test]
    fn remote_eof_requests_gui_shutdown_and_reaps_both_workers() {
        let (mut workers, _sender, peer) = connected_workers();
        drop(peer);
        wait_stopped(&workers);
        shutdown_quickly(&mut workers);
        assert!(workers.threads.is_empty());
    }

    #[test]
    fn shutdown_cancels_a_partial_frame_with_a_live_sender_and_is_idempotent() {
        let (mut workers, sender, mut peer) = connected_workers();
        peer.write_all(&[0, 0]).unwrap();
        let started = Instant::now();
        workers.shutdown();
        workers.shutdown();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(workers.threads.is_empty());
        assert!(
            sender
                .send(FTPPacket {
                    id: FTPId::Get.to_u8(),
                    data: Vec::new()
                })
                .is_err()
        );
        let received = peer.read(&mut [0]);
        assert!(
            matches!(received, Ok(0))
                || cfg!(windows)
                    && matches!(&received, Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset),
            "cancelled partial frame must close the socket without returning payload: {received:?}"
        );
    }

    #[test]
    fn sender_preserves_fifo_packets_and_local_frame_boundaries() {
        let (mut workers, sender, mut peer) = connected_workers();
        peer.set_read_timeout(Some(Duration::from_millis(350)))
            .unwrap();
        let idle = peer.read(&mut [0]);
        assert!(
            matches!(idle, Err(error) if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut))
        );
        assert!(!workers.stopped.load(Ordering::Acquire));
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        for payload in [Vec::new(), vec![17; 262144]] {
            sender
                .send(FTPPacket {
                    id: FTPId::Put.to_u8(),
                    data: payload.clone(),
                })
                .unwrap();
            let packet = FTPPacket::parse(&read_frame(&mut peer).unwrap()).unwrap();
            assert_eq!(packet.id().unwrap(), FTPId::Put);
            assert_eq!(packet.data, payload);
        }
        shutdown_quickly(&mut workers);
        assert!(workers.threads.is_empty());
    }
}
