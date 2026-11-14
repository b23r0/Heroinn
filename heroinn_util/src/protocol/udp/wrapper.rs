//! Thin blocking facade over the async `rust-raknet` API.
//!
//! A single shared Tokio runtime drives every RakNet socket so callers can keep
//! the rest of the transport layer synchronous.

use std::net::SocketAddr;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use rust_raknet::error::RaknetError;
use rust_raknet::*;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::error::{Error, Result};

/// RakNet retransmits handshake packets on its own schedule. At the 5% loss
/// and 64-peer concurrency covered by the native UDP tests a retransmit can
/// land beyond ten seconds, so a healthy peer would be reported as a connect
/// timeout. The handshake itself is unchanged; this only bounds how long a
/// genuinely unreachable peer may take to fail.
const RAKNET_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

fn map_error(error: RaknetError) -> Error {
    Error::Other(format!("{:?}", error))
}

fn runtime(worker_threads: usize) -> Result<Arc<tokio::runtime::Runtime>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .build()
        .map_err(|error| Error::Other(format!("raknet runtime: {error}")))?;
    Ok(Arc::new(runtime))
}

static CLIENT_RUNTIME: LazyLock<Arc<tokio::runtime::Runtime>> =
    LazyLock::new(|| runtime(4).expect("failed to create RakNet client runtime"));

fn bind_listener_socket(address: &SocketAddr) -> Result<std::net::UdpSocket> {
    let socket = std::net::UdpSocket::bind(address)
        .map_err(|error| Error::Other(format!("raknet bind {address}: {error}")))?;
    if let Err(error) = socket2::SockRef::from(&socket).set_recv_buffer_size(2 * 1024 * 1024) {
        log::debug!("could not enlarge the RakNet listener receive buffer: {error}");
    }

    #[cfg(target_os = "windows")]
    disable_windows_udp_connection_reset(&socket)?;

    Ok(socket)
}

#[cfg(target_os = "windows")]
fn disable_windows_udp_connection_reset(socket: &std::net::UdpSocket) -> Result<()> {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawSocket;

    const SIO_UDP_CONNRESET: u32 = 0x9800_000c;

    #[link(name = "ws2_32")]
    unsafe extern "system" {
        fn WSAIoctl(
            socket: usize,
            control: u32,
            input: *mut c_void,
            input_len: u32,
            output: *mut c_void,
            output_len: u32,
            bytes_returned: *mut u32,
            overlapped: *mut c_void,
            completion_routine: *mut c_void,
        ) -> i32;
    }

    let mut report_connection_reset = 0u32;
    let mut bytes_returned = 0u32;
    let result = unsafe {
        WSAIoctl(
            socket.as_raw_socket() as usize,
            SIO_UDP_CONNRESET,
            (&mut report_connection_reset as *mut u32).cast::<c_void>(),
            std::mem::size_of_val(&report_connection_reset) as u32,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(Error::Other(format!(
            "disable Windows UDP connection-reset reporting: {}",
            std::io::Error::last_os_error()
        )))
    }
}

/// Accepting side of a RakNet socket.
pub struct RUdpServer {
    server: Arc<tokio::sync::Mutex<RaknetListener>>,
    runtime: Arc<tokio::runtime::Runtime>,
    accepted: Receiver<Result<RUdpClient>>,
    close_sender: Option<oneshot::Sender<()>>,
    accept_worker: Option<JoinHandle<()>>,
    local_addr: SocketAddr,
}

enum ClientCommand {
    Send {
        data: Vec<u8>,
        reply: SyncSender<Result<()>>,
    },
    Close {
        reply: SyncSender<Result<()>>,
    },
}

/// Connected side of a RakNet socket.
#[derive(Clone)]
pub struct RUdpClient {
    commands: mpsc::UnboundedSender<ClientCommand>,
    incoming: Arc<Mutex<mpsc::Receiver<Result<Vec<u8>>>>>,
    peer_addr: SocketAddr,
    local_addr: SocketAddr,
}

impl RUdpClient {
    /// Connect to `address`, giving up after ten seconds.
    pub fn new(address: String) -> Result<Self> {
        let address: SocketAddr = address
            .parse()
            .map_err(|e| Error::Other(format!("address format error : {:?}", e)))?;

        let runtime = CLIENT_RUNTIME.clone();
        let client = runtime
            .block_on(async {
                tokio::time::timeout(RAKNET_CONNECT_TIMEOUT, RaknetSocket::connect(&address)).await
            })
            .map_err(|_| Error::Timeout)?
            .map_err(map_error)?;

        Self::from_socket(client, runtime.handle().clone())
    }

    pub fn send(&self, buf: Vec<u8>) -> Result<()> {
        let (reply, receiver) = std::sync::mpsc::sync_channel(1);
        self.commands
            .send(ClientCommand::Send { data: buf, reply })
            .map_err(|_| Error::Other("raknet send actor stopped".to_string()))?;
        receiver
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| Error::Timeout)?
    }

    pub fn recv(&self) -> Result<Vec<u8>> {
        let mut incoming = self
            .incoming
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        incoming
            .blocking_recv()
            .unwrap_or_else(|| Err(Error::Other("raknet receive actor stopped".to_string())))
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<Vec<u8>> {
        let mut incoming = self
            .incoming
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match CLIENT_RUNTIME
            .block_on(async { tokio::time::timeout(timeout, incoming.recv()).await })
        {
            Ok(Some(result)) => result,
            Ok(None) => Err(Error::Other("raknet receive actor stopped".to_string())),
            Err(_) => Err(Error::Timeout),
        }
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        Ok(self.peer_addr)
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn close(&self) -> Result<()> {
        let (reply, receiver) = std::sync::mpsc::sync_channel(1);
        if self.commands.send(ClientCommand::Close { reply }).is_err() {
            return Ok(());
        }
        receiver
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| Error::Timeout)?
    }

    fn from_socket(socket: RaknetSocket, handle: tokio::runtime::Handle) -> Result<Self> {
        let peer_addr = socket.peer_addr().map_err(map_error)?;
        let local_addr = socket.local_addr().map_err(map_error)?;
        let socket = Arc::new(socket);
        let (commands, mut command_receiver) = mpsc::unbounded_channel();
        let (incoming, incoming_receiver) = mpsc::channel(128);
        let reader_socket = socket.clone();
        let reader = handle.spawn(async move {
            loop {
                match reader_socket.recv().await {
                    Ok(data) => {
                        if incoming.send(Ok(data)).await.is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = incoming.send(Err(map_error(error))).await;
                        break;
                    }
                }
            }
        });
        handle.spawn(async move {
            let mut close_reply = None;
            while let Some(command) = command_receiver.recv().await {
                match command {
                    ClientCommand::Send { data, reply } => {
                        let result = socket
                            .send(&data, Reliability::ReliableOrdered)
                            .await
                            .map_err(map_error);
                        let _ = reply.send(result);
                    }
                    ClientCommand::Close { reply } => {
                        close_reply = Some(reply);
                        break;
                    }
                }
            }
            // `send` only places data in RakNet's reliable queue. Let the
            // transport acknowledge the pending reliable frames before
            // closing the peer, otherwise the final application frame can be
            // discarded during teardown. The timeout keeps cancellation and
            // dead-peer teardown bounded; the socket is closed below in all
            // cases.
            let _ = tokio::time::timeout(Duration::from_secs(10), socket.flush()).await;
            let close_result = socket.close().await.map_err(map_error);
            reader.abort();
            let _ = reader.await;
            if let Some(reply) = close_reply {
                let _ = reply.send(close_result);
            }
        });

        Ok(Self {
            commands,
            incoming: Arc::new(Mutex::new(incoming_receiver)),
            peer_addr,
            local_addr,
        })
    }
}

impl RUdpServer {
    /// Bind a RakNet listener to `address` and start listening.
    pub fn new(address: &str) -> Result<Self> {
        let address: SocketAddr = address
            .parse()
            .map_err(|e| Error::Other(format!("address format error : {:?}", e)))?;

        let listener_socket = bind_listener_socket(&address)?;
        let runtime = runtime(4)?;
        let mut server = runtime
            .block_on(RaknetListener::from_std(listener_socket))
            .map_err(map_error)?;
        runtime.block_on(server.listen());
        let local_addr = server.local_addr().map_err(map_error)?;
        let server = Arc::new(tokio::sync::Mutex::new(server));
        let (accepted_sender, accepted) = std::sync::mpsc::sync_channel(128);
        let (close_sender, mut close_receiver) = oneshot::channel();
        let server_worker = server.clone();
        let accept_worker = runtime.spawn(async move {
            loop {
                let mut listener = server_worker.lock().await;
                let accepted_client = tokio::select! {
                    _ = &mut close_receiver => {
                        let _ = listener.close().await;
                        break;
                    }
                    accepted = listener.accept() => accepted,
                };
                drop(listener);

                match accepted_client {
                    Ok(client) => {
                        let handle = tokio::runtime::Handle::current();
                        if accepted_sender
                            .send(RUdpClient::from_socket(client, handle))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = accepted_sender.send(Err(map_error(error)));
                        break;
                    }
                }
            }
        });

        Ok(Self {
            server,
            runtime,
            accepted,
            close_sender: Some(close_sender),
            accept_worker: Some(accept_worker),
            local_addr,
        })
    }

    /// Accept one peer, giving up after `timeout_mills`.
    pub fn accept(&mut self, timeout_mills: u64) -> Result<RUdpClient> {
        match self
            .accepted
            .recv_timeout(Duration::from_millis(timeout_mills))
        {
            Ok(client) => client,
            Err(RecvTimeoutError::Timeout) => Err(Error::Timeout),
            Err(RecvTimeoutError::Disconnected) => {
                Err(Error::Other("raknet accept worker stopped".to_string()))
            }
        }
    }

    pub fn local_addr(&mut self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn close(&mut self) -> Result<()> {
        if let Some(sender) = self.close_sender.take() {
            let _ = sender.send(());
        }
        if let Some(worker) = self.accept_worker.take() {
            self.runtime
                .block_on(worker)
                .map_err(|error| Error::Other(format!("raknet accept worker: {error}")))?;
        }
        Ok(())
    }

    pub fn set_motd(&mut self, motd: String) {
        let server = self.server.clone();
        self.runtime
            .block_on(async move { server.lock().await.set_full_motd(motd).await })
            .unwrap();
    }
}

impl Drop for RUdpServer {
    fn drop(&mut self) {
        if let Some(sender) = self.close_sender.take() {
            let _ = sender.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Barrier, Condvar, Mutex};

    struct TestGate {
        active: Mutex<usize>,
        finished: Condvar,
        limit: usize,
    }

    impl TestGate {
        fn new(limit: usize) -> Self {
            Self {
                active: Mutex::new(0),
                finished: Condvar::new(),
                limit,
            }
        }

        fn enter(&self) -> TestGatePermit<'_> {
            let mut active = self
                .active
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            while *active >= self.limit {
                active = self
                    .finished
                    .wait(active)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            *active += 1;
            TestGatePermit { gate: self }
        }
    }

    struct TestGatePermit<'a> {
        gate: &'a TestGate,
    }

    impl Drop for TestGatePermit<'_> {
        fn drop(&mut self) {
            let mut active = self
                .gate
                .active
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *active -= 1;
            self.gate.finished.notify_one();
        }
    }

    #[test]
    fn raw_raknet_peer_exchanges_multiple_messages() {
        let mut server = RUdpServer::new("127.0.0.1:0").unwrap();
        let remote = server.local_addr().unwrap().to_string();
        let accept_thread = std::thread::spawn(move || {
            let client = server.accept(10_000).unwrap();
            for expected in [b"first".as_slice(), b"second".as_slice()] {
                let message = client.recv().expect("server recv");
                assert_eq!(message, [vec![0xfe], expected.to_vec()].concat());
                client.send(message).expect("server echo");
            }
            server.close().expect("server close");
        });

        let client = RUdpClient::new(remote).expect("client connect");
        for payload in [b"first".as_slice(), b"second".as_slice()] {
            let message = [vec![0xfe], payload.to_vec()].concat();
            client.send(message.clone()).expect("client send");
            assert_eq!(client.recv().expect("client recv"), message);
        }
        client.close().expect("client close");
        accept_thread.join().expect("accept thread");
    }

    #[test]
    fn four_raw_raknet_peers_exchange_multiple_messages() {
        const PEERS: usize = 4;
        let mut server = RUdpServer::new("127.0.0.1:0").unwrap();
        let remote = server.local_addr().unwrap().to_string();
        let accept_thread = std::thread::spawn(move || {
            let mut workers = Vec::with_capacity(PEERS);
            for _ in 0..PEERS {
                let client = server.accept(10_000).unwrap();
                workers.push(std::thread::spawn(move || {
                    for _ in [b"first".as_slice(), b"second".as_slice()] {
                        let message = client.recv().expect("server recv");
                        client.send(message).expect("server echo");
                    }
                }));
            }
            for worker in workers {
                worker.join().expect("server worker");
            }
            server.close().expect("server close");
        });

        let barrier = Arc::new(Barrier::new(PEERS));
        let mut clients = Vec::with_capacity(PEERS);
        for index in 0..PEERS {
            let remote = remote.clone();
            let barrier = barrier.clone();
            clients.push(std::thread::spawn(move || {
                let client = RUdpClient::new(remote).expect("client connect");
                barrier.wait();
                for stage in [b"first".as_slice(), b"second".as_slice()] {
                    let message = [
                        vec![0xfe],
                        stage.to_vec(),
                        format!("{index:04}").into_bytes(),
                    ]
                    .concat();
                    client.send(message.clone()).expect("client send");
                    assert_eq!(client.recv().expect("client recv"), message);
                }
                client.close().expect("client close");
            }));
        }
        for client in clients {
            client.join().expect("client worker");
        }
        accept_thread.join().expect("accept thread");
    }

    #[test]
    fn sixty_four_raw_raknet_peers_echo_without_cross_streams() {
        const PEERS: usize = 64;
        const CONNECT_PARALLEL: usize = 8;
        let mut server = RUdpServer::new("127.0.0.1:0").unwrap();
        let remote = server.local_addr().unwrap().to_string();
        let accept_thread = std::thread::spawn(move || {
            let mut accepted = Vec::with_capacity(PEERS);
            for index in 0..PEERS {
                let client = server
                    .accept(10_000)
                    .unwrap_or_else(|error| panic!("accept {index}: {error}"));
                accepted.push(std::thread::spawn(move || {
                    let message = client.recv().expect("server recv");
                    client.send(message).expect("server echo");
                }));
            }
            for worker in accepted {
                worker.join().expect("server worker");
            }
            server.close().expect("server close");
        });

        let barrier = Arc::new(Barrier::new(PEERS));
        let connect_gate = Arc::new(TestGate::new(CONNECT_PARALLEL));
        let mut clients = Vec::with_capacity(PEERS);
        for index in 0..PEERS {
            let remote = remote.clone();
            let barrier = barrier.clone();
            let connect_gate = connect_gate.clone();
            clients.push(std::thread::spawn(move || {
                let client = {
                    let _permit = connect_gate.enter();
                    RUdpClient::new(remote).expect("client connect")
                };
                barrier.wait();
                let message = [vec![0xfe], format!("raw-peer-{index:04}").into_bytes()].concat();
                client.send(message.clone()).expect("client send");
                assert_eq!(client.recv().expect("client recv"), message);
                client.close().expect("client close");
            }));
        }

        for client in clients {
            client.join().expect("client worker");
        }
        accept_thread.join().expect("accept thread");
    }

    #[test]
    fn close_flushes_reliable_payload_before_disconnect() {
        for round in 0..5 {
            let mut server = RUdpServer::new("127.0.0.1:0").unwrap();
            let remote = server.local_addr().unwrap().to_string();
            let (bytes_sender, bytes_receiver) = std::sync::mpsc::channel();
            let server_thread = std::thread::spawn(move || {
                let client = server.accept(10_000).unwrap();
                // Give the sender a chance to queue every reliable frame.
                std::thread::sleep(Duration::from_millis(200));
                let mut bytes = 0usize;
                while bytes < 256 * 1024 {
                    match client.recv() {
                        Ok(message) => bytes += message.len().saturating_sub(1),
                        Err(error) => {
                            let _ = bytes_sender.send((bytes, Some(error.to_string())));
                            return;
                        }
                    }
                }
                let _ = bytes_sender.send((bytes, None));
                server.close().unwrap();
            });

            let client = RUdpClient::new(remote).unwrap();
            let payload = (0..(256 * 1024))
                .map(|index| (index % 251) as u8)
                .collect::<Vec<_>>();
            for chunk in payload.chunks(16 * 1024) {
                let mut message = Vec::with_capacity(chunk.len() + 1);
                message.push(0xfe);
                message.extend_from_slice(chunk);
                client.send(message).unwrap();
            }
            client.close().unwrap();

            let (bytes, error) = bytes_receiver
                .recv_timeout(Duration::from_secs(10))
                .unwrap_or_else(|_| panic!("round {round}: server did not finish"));
            server_thread.join().expect("server thread");
            assert_eq!(
                bytes,
                256 * 1024,
                "round {round}: reliable payload was truncated ({error:?})"
            );
        }
    }
}
