//! Reliable-UDP transport built on `rust-raknet`.

pub mod wrapper;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use self::wrapper::{RUdpClient, RUdpServer};

use super::tcp::TcpConnection;
use crate::HeroinnProtocol;
use crate::error::{Error, Result};
use crate::packet::Message;
use crate::protocol::{DisconnectHandler, MessageHandler, TUNNEL_FLAG};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConnectionEnd {
    Closed,
    Tunnel,
}

/// Byte prefixed to every UDP payload to keep tunnel and data frames apart.
const UDP_PREFIX: u8 = 0xfe;

/// Accepting side of a reliable-UDP listener.
pub struct UDPServer {
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
    connections: Arc<Mutex<HashMap<SocketAddr, Arc<RUdpClient>>>>,
}

/// One outbound reliable-UDP connection.
pub struct UDPConnection {
    s: Option<RUdpClient>,
    local_addr: SocketAddr,
    closed: Arc<AtomicBool>,
}

impl UDPServer {
    /// Bind `address` and pass every decoded message to `on_message`.
    pub fn new(address: &str, on_message: MessageHandler) -> Result<Self> {
        Self::new_with_disconnect(address, on_message, Arc::new(|_, _| {}))
    }

    pub fn new_with_disconnect(
        address: &str,
        on_message: MessageHandler,
        on_disconnect: DisconnectHandler,
    ) -> Result<Self> {
        let mut server = RUdpServer::new(address)?;

        let connections: Arc<Mutex<HashMap<SocketAddr, Arc<RUdpClient>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let closed = Arc::new(AtomicBool::new(false));

        let local_addr = server.local_addr().unwrap();

        let connections_1 = connections.clone();
        let closed_1 = closed.clone();
        std::thread::Builder::new()
            .name(format!("udp listener : {}", local_addr))
            .spawn(move || {
                loop {
                    let client = match server.accept(500) {
                        Ok(p) => p,
                        Err(e) => {
                            if closed_1.load(Ordering::Relaxed) {
                                break;
                            }

                            if matches!(e, Error::Timeout) {
                                std::thread::sleep(std::time::Duration::from_millis(200));
                                continue;
                            } else {
                                closed_1.store(true, Ordering::Relaxed);
                                break;
                            }
                        }
                    };

                    let client = Arc::new(client);
                    let connections_2 = connections_1.clone();
                    let on_message = on_message.clone();
                    let on_disconnect = on_disconnect.clone();
                    let remote_addr = client.peer_addr().unwrap();

                    connections_1
                        .lock()
                        .unwrap()
                        .insert(remote_addr, client.clone());

                    std::thread::Builder::new()
                        .name(format!("udp connection : {}", local_addr))
                        .spawn(move || {
                            let end = handle_connection(client, remote_addr, &on_message);
                            connections_2.lock().unwrap().remove(&remote_addr);
                            match end {
                                ConnectionEnd::Closed => {
                                    log::info!("udp client worker finished");
                                    on_disconnect(remote_addr, HeroinnProtocol::UDP);
                                }
                                ConnectionEnd::Tunnel => {
                                    log::info!("udp client worker entered tunnel");
                                }
                            }
                        })
                        .unwrap();
                }

                let mut conns = connections_1.lock().unwrap();
                for i in conns.values_mut() {
                    let _ = i.close();
                }
                conns.clear();

                let _ = server.close();
                log::info!("udp main worker finished");
            })
            .unwrap();

        Ok(Self {
            local_addr,
            closed,
            connections,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn sendto(&mut self, peer_addr: &SocketAddr, buf: &[u8]) -> Result<()> {
        let client = self
            .connections
            .lock()
            .unwrap()
            .get(peer_addr)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("udp client {}", peer_addr)))?;

        client
            .send(prefixed(buf))
            .map_err(|e| Error::Other(format!("udp send msg error : {}", e)))
    }

    pub fn contains_addr(&mut self, peer_addr: &SocketAddr) -> bool {
        self.connections.lock().unwrap().contains_key(peer_addr)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}

/// Read datagrams until the peer disconnects or requests a tunnel.
fn handle_connection(
    client: Arc<RUdpClient>,
    remote_addr: SocketAddr,
    on_message: &MessageHandler,
) -> ConnectionEnd {
    loop {
        let buf = match client.recv() {
            Ok(p) => p,
            Err(e) => {
                log::error!("udp client recv error : {}", e);
                return ConnectionEnd::Closed;
            }
        };

        if buf.len() == TUNNEL_FLAG.len() + 3 && buf[1..5] == TUNNEL_FLAG {
            let port = u16::from_be_bytes([buf[5], buf[6]]);
            serve_tunnel(client, port);
            return ConnectionEnd::Tunnel;
        }

        match Message::new(remote_addr, HeroinnProtocol::UDP, &buf[1..]) {
            Ok(msg) => on_message(msg),
            Err(e) => log::warn!("failed to decode udp message from {}: {}", remote_addr, e),
        }
    }
}

/// Pump bytes both ways between the UDP peer and a local TCP connection.
fn serve_tunnel(client: Arc<RUdpClient>, port: u16) {
    let full_addr = format!("127.0.0.1:{}", port);

    let tunnel_client = match TcpConnection::connect_direct(&full_addr) {
        Ok(p) => p,
        Err(e) => {
            log::error!("tunnel connect failed : {}", e);
            return;
        }
    };

    let mut local_to_udp = tunnel_client.clone();
    let client_1 = client.clone();
    std::thread::Builder::new()
        .name(format!("udp tunnel worker1 : {}", full_addr))
        .spawn(move || {
            loop {
                let buf = match local_to_udp.recv() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("tunnel read failed : {}", e);
                        break;
                    }
                };

                if buf.is_empty() {
                    break;
                }

                if let Err(e) = client_1.send(prefixed(&buf)) {
                    log::error!("udp sender error : {}", e);
                    break;
                }
            }
            let _ = local_to_udp.shutdown(std::net::Shutdown::Both);
            let _ = client_1.close();
            log::debug!("udp tunnel1 finished!");
        })
        .unwrap();

    let mut udp_to_local = tunnel_client;
    std::thread::Builder::new()
        .name(format!("udp tunnel worker2 : {}", full_addr))
        .spawn(move || {
            loop {
                let buf = match client.recv() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("udp receiver error : {}", e);
                        break;
                    }
                };
                if buf.is_empty() {
                    log::warn!("udp tunnel received an empty frame");
                    break;
                }

                if let Err(e) = udp_to_local.send(&buf[1..]) {
                    log::error!("udp sender error : {}", e);
                    break;
                }
            }

            let _ = udp_to_local.shutdown(std::net::Shutdown::Both);
            log::debug!("udp tunnel2 finished!");
        })
        .unwrap();
}

/// Prefix `buf` with [`UDP_PREFIX`].
fn prefixed(buf: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(buf.len() + 1);
    data.push(UDP_PREFIX);
    data.extend_from_slice(buf);
    data
}

impl UDPConnection {
    /// Dial `address`.
    pub fn connect(address: &str) -> Result<Self> {
        let s = RUdpClient::new(address.to_string())?;
        let local_addr = s.local_addr().unwrap();

        Ok(Self {
            s: Some(s),
            closed: Arc::new(AtomicBool::new(false)),
            local_addr,
        })
    }

    /// Dial `remote_addr` and ask it to tunnel to `server_local_port`.
    pub fn tunnel(remote_addr: &str, server_local_port: u16) -> Result<Self> {
        log::info!("start tunnel [{}] [{}]", remote_addr, server_local_port);

        let mut ret = Self::connect(remote_addr)?;

        let mut buf = TUNNEL_FLAG.to_vec();
        buf.append(&mut server_local_port.to_be_bytes().to_vec());
        ret.send(&buf)?;

        Ok(ret)
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        let s = self.client()?;

        let msg = s
            .recv()
            .map_err(|e| Error::Other(format!("udp receive error : {}", e)))?;

        Ok(msg[1..].to_vec())
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<()> {
        let s = self.client()?;
        s.send(prefixed(buf))?;
        Ok(())
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        if let Some(client) = self.s.take() {
            let _ = client.close();
        }
    }

    fn client(&self) -> Result<&RUdpClient> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Other("socket closed".to_string()));
        }

        self.s
            .as_ref()
            .ok_or_else(|| Error::Other("socket closed".to_string()))
    }
}

impl Clone for UDPConnection {
    fn clone(&self) -> Self {
        Self {
            s: self.s.clone(),
            closed: self.closed.clone(),
            local_addr: self.local_addr,
        }
    }
}

impl Drop for UDPServer {
    fn drop(&mut self) {
        self.close();
    }
}

impl Drop for UDPConnection {
    fn drop(&mut self) {
        self.s = None;
    }
}

#[test]
fn test_udp_tunnel() {
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let server2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let remote_local_port = server2.local_addr().unwrap().port();

    let remote = &format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let mut client1 = UDPConnection::tunnel(remote, remote_local_port).unwrap();

    let (mut client2, _) = TcpConnection::tunnel_server(server2, 10).unwrap();

    for _ in 0..3 {
        client1.send(&[0, 1, 2, 3]).unwrap();
        let buf = client2.recv().unwrap();
        assert_eq!(buf, [0, 1, 2, 3]);

        client2.send(&[4, 5, 6, 7]).unwrap();
        let buf = client1.recv().unwrap();
        assert_eq!(buf, [4, 5, 6, 7]);
    }

    server.close();
}

#[test]
fn udp_server_reports_transport_disconnect_immediately() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut server = UDPServer::new_with_disconnect(
        "127.0.0.1:0",
        Arc::new(|_| {}),
        Arc::new(move |peer, protocol| {
            let _ = sender.send((peer, protocol));
        }),
    )
    .unwrap();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let client = UDPConnection::connect(&remote).unwrap();
    drop(client);
    let (_peer, protocol) = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("disconnect callback");
    assert_eq!(protocol, HeroinnProtocol::UDP);
    server.close();
}

#[test]
fn udp_tunnel_does_not_report_a_main_transport_disconnect() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut server = UDPServer::new_with_disconnect(
        "127.0.0.1:0",
        Arc::new(|_| {}),
        Arc::new(move |peer, protocol| {
            let _ = sender.send((peer, protocol));
        }),
    )
    .unwrap();
    let tunnel_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let tunnel_port = tunnel_listener.local_addr().unwrap().port();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());

    let mut tunnel = UDPConnection::tunnel(&remote, tunnel_port).unwrap();
    let (mut local, _) = TcpConnection::tunnel_server(tunnel_listener, 5).unwrap();
    tunnel.send(b"still-connected").unwrap();
    assert_eq!(local.recv().unwrap(), b"still-connected");

    assert!(
        receiver
            .recv_timeout(std::time::Duration::from_millis(500))
            .is_err(),
        "starting a reliable-UDP tunnel must not be reported as main-transport disconnect"
    );
    server.close();
}

#[test]
fn udp_transport_shutdown_unblocks_a_pending_receiver() {
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let tunnel_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let tunnel_port = tunnel_listener.local_addr().unwrap().port();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());

    let tunnel = UDPConnection::tunnel(&remote, tunnel_port).unwrap();
    let (local, _) = TcpConnection::tunnel_server(tunnel_listener, 5).unwrap();
    let transport = crate::protocol::Transport::Udp(tunnel);
    let receiver = transport.clone();
    let (ended_sender, ended_receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut receiver = receiver;
        let _ = ended_sender.send(receiver.recv().is_err());
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    transport
        .shutdown(std::net::Shutdown::Both)
        .expect("shutdown UDP transport");
    assert!(
        ended_receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("pending UDP receiver did not unblock"),
        "a closed UDP transport must report an error to its receiver"
    );
    worker.join().expect("receiver worker");
    let _ = local.shutdown(std::net::Shutdown::Both);
    server.close();
}

#[test]
fn udp_tunnel_closes_both_directions_after_target_eof() {
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let tunnel_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let tunnel_port = tunnel_listener.local_addr().unwrap().port();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());

    let mut tunnel = UDPConnection::tunnel(&remote, tunnel_port).unwrap();
    let (mut local, _) = TcpConnection::tunnel_server(tunnel_listener, 5).unwrap();
    local.send(b"final-data").unwrap();
    assert_eq!(tunnel.recv().unwrap(), b"final-data");
    local.shutdown(std::net::Shutdown::Both).unwrap();

    let (closed_sender, closed_receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = closed_sender.send(tunnel.recv().is_err());
    });
    assert!(
        closed_receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("target EOF did not close the reverse UDP tunnel"),
        "target EOF must make the client tunnel receiver terminate"
    );
    worker.join().expect("tunnel receiver worker");
    server.close();
}

#[test]
fn udp_tunnel_delivers_full_payload_before_teardown() {
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let tunnel_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let tunnel_port = tunnel_listener.local_addr().unwrap().port();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let mut tunnel = UDPConnection::tunnel(&remote, tunnel_port).unwrap();
    let (mut local, _) = TcpConnection::tunnel_server(tunnel_listener, 20).unwrap();
    let payload = (0..65536)
        .map(|index| (index % 251) as u8)
        .collect::<Vec<_>>();

    local.send(&payload).unwrap();
    local.shutdown(std::net::Shutdown::Both).unwrap();

    let (sender, receiver) = std::sync::mpsc::channel();
    let receiver_thread = std::thread::spawn(move || {
        let mut bytes = Vec::with_capacity(65536);
        loop {
            match tunnel.recv() {
                Ok(data) => {
                    bytes.extend_from_slice(&data);
                    if bytes.len() >= 65536 {
                        break;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(error.to_string()));
                    return;
                }
            }
        }
        let _ = sender.send(Ok(bytes));
    });

    let result = receiver
        .recv_timeout(std::time::Duration::from_secs(20))
        .expect("reverse UDP tunnel did not finish");
    assert!(result.is_ok(), "reverse UDP tunnel failed: {result:?}");
    assert_eq!(result.unwrap(), payload);
    receiver_thread.join().expect("receiver thread");
    server.close();
}

#[cfg(test)]
struct ExchangeGate {
    active: Mutex<usize>,
    finished: std::sync::Condvar,
    limit: usize,
}

#[cfg(test)]
impl ExchangeGate {
    fn new(limit: usize) -> Self {
        Self {
            active: Mutex::new(0),
            finished: std::sync::Condvar::new(),
            limit,
        }
    }

    fn enter(&self) -> ExchangePermit<'_> {
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
        ExchangePermit { gate: self }
    }
}

#[cfg(test)]
struct ExchangePermit<'a> {
    gate: &'a ExchangeGate,
}

#[cfg(test)]
impl Drop for ExchangePermit<'_> {
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

#[cfg(test)]
fn run_independent_udp_tunnel_peers(count: usize) {
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    // Keep every tunnel alive until all streams have exchanged data. The
    // lifecycle tests cover teardown independently; this test isolates the
    // maximum-concurrency data path from RakNet shutdown races.
    let exchange_gate = Arc::new(ExchangeGate::new(count.max(1)));
    let setup_gate = Arc::new(ExchangeGate::new(count.clamp(1, 8)));
    let start = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
    let release = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
    let progress = Arc::new(Mutex::new(std::collections::BTreeMap::new()));
    let (setup_sender, setup_receiver) = std::sync::mpsc::channel();
    let (result_sender, result_receiver) = std::sync::mpsc::channel();
    let mut workers = Vec::with_capacity(count);

    for index in 0..count {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let local_port = listener.local_addr().unwrap().port();
        let remote = remote.clone();
        let exchange_gate = exchange_gate.clone();
        let setup_gate = setup_gate.clone();
        let start = start.clone();
        let release = release.clone();
        let progress = progress.clone();
        let setup_sender = setup_sender.clone();
        let result_sender = result_sender.clone();
        workers.push(std::thread::spawn(move || {
            let setup = (|| {
                let _setup_permit = setup_gate.enter();
                let tunnel = UDPConnection::tunnel(&remote, local_port)
                    .map_err(|error| format!("peer {index} tunnel: {error}"))?;
                let (local, _) = TcpConnection::tunnel_server(listener, 20)
                    .map_err(|error| format!("peer {index} accept: {error}"))?;
                Ok::<_, String>((tunnel, local))
            })();
            set_test_progress(&progress, index, "setup-complete");
            let _ = setup_sender.send(setup.as_ref().map(|_| ()).map_err(|error| error.clone()));
            wait_for_flag(&start);

            let mut setup = setup;
            let result = match setup.as_mut() {
                Err(error) => Err(error.clone()),
                Ok((tunnel, local)) => (|| {
                    let _exchange_permit = exchange_gate.enter();
                    set_test_progress(&progress, index, "exchange-start");
                    let to_target = format!("peer-{index:04}-to-target").into_bytes();
                    set_test_progress(&progress, index, "tunnel-send-start");
                    tunnel
                        .send(&to_target)
                        .map_err(|error| format!("peer {index} send: {error}"))?;
                    set_test_progress(&progress, index, "tunnel-send-done");
                    let received = local
                        .recv()
                        .map_err(|error| format!("peer {index} local recv: {error}"))?;
                    set_test_progress(&progress, index, "local-recv-done");
                    if received != to_target {
                        return Err(format!("peer {index} received another stream's payload"));
                    }

                    let to_client = format!("peer-{index:04}-to-client").into_bytes();
                    set_test_progress(&progress, index, "local-send-start");
                    local
                        .send(&to_client)
                        .map_err(|error| format!("peer {index} local send: {error}"))?;
                    set_test_progress(&progress, index, "local-send-done");
                    let received = tunnel
                        .recv()
                        .map_err(|error| format!("peer {index} tunnel recv: {error}"))?;
                    set_test_progress(&progress, index, "tunnel-recv-done");
                    if received != to_client {
                        return Err(format!("peer {index} received another stream's reply"));
                    }
                    Ok(())
                })(),
            };
            let _ = result_sender.send(result);
            set_test_progress(&progress, index, "done");
            wait_for_flag(&release);
            drop(setup);
        }));
    }

    drop(setup_sender);
    let mut failures = Vec::new();
    for _ in 0..count {
        match setup_receiver
            .recv_timeout(std::time::Duration::from_secs(60))
            .expect("parallel UDP peer setup timed out")
        {
            Ok(()) => {}
            Err(error) => failures.push(error),
        }
    }
    signal_flag(&start);

    for _ in 0..count {
        let result = match result_receiver.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(result) => result,
            Err(_) => {
                let progress = progress
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .clone();
                panic!("parallel UDP peer exchange timed out: {progress:#?}");
            }
        };
        match result {
            Ok(()) => {}
            Err(error) => failures.push(error),
        }
    }
    signal_flag(&release);

    drop(result_sender);
    for worker in workers {
        if worker.join().is_err() {
            failures.push("UDP peer worker panicked".to_string());
        }
    }
    server.close();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[cfg(test)]
fn wait_for_flag(flag: &(Mutex<bool>, std::sync::Condvar)) {
    let mut ready = flag
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    while !*ready {
        ready = flag
            .1
            .wait(ready)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    }
}

#[cfg(test)]
fn signal_flag(flag: &(Mutex<bool>, std::sync::Condvar)) {
    *flag
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
    flag.1.notify_all();
}

#[cfg(test)]
fn set_test_progress(
    progress: &Mutex<std::collections::BTreeMap<usize, &'static str>>,
    index: usize,
    stage: &'static str,
) {
    progress
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(index, stage);
}

#[test]
fn two_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(2);
}

#[test]
fn four_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(4);
}

#[test]
fn closing_one_udp_tunnel_does_not_disrupt_other_streams() {
    let mut server = UDPServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
    let remote = format!("127.0.0.1:{}", server.local_addr().unwrap().port());
    let mut peers = Vec::new();

    for _ in 0..4 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let local_port = listener.local_addr().unwrap().port();
        let tunnel = UDPConnection::tunnel(&remote, local_port).unwrap();
        let (local, _) = TcpConnection::tunnel_server(listener, 20).unwrap();
        peers.push((tunnel, local));
    }

    let (closing, _) = peers.remove(0);
    drop(closing);
    std::thread::sleep(std::time::Duration::from_millis(300));

    for (index, (tunnel, local)) in peers.iter_mut().enumerate() {
        let downlink = format!("survivor-{index}-downlink").into_bytes();
        tunnel.send(&downlink).unwrap();
        assert_eq!(local.recv().unwrap(), downlink);

        let uplink = format!("survivor-{index}-uplink").into_bytes();
        local.send(&uplink).unwrap();
        assert_eq!(tunnel.recv().unwrap(), uplink);
    }

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let local_port = listener.local_addr().unwrap().port();
    let mut replacement = UDPConnection::tunnel(&remote, local_port).unwrap();
    let (mut replacement_local, _) = TcpConnection::tunnel_server(listener, 20).unwrap();
    replacement.send(b"replacement").unwrap();
    assert_eq!(replacement_local.recv().unwrap(), b"replacement");

    server.close();
}

#[test]
fn eight_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(8);
}

#[test]
fn sixteen_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(16);
}

#[test]
fn thirty_two_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(32);
}

#[test]
fn sixty_four_independent_udp_tunnel_peers_do_not_cross_streams() {
    run_independent_udp_tunnel_peers(64);
}
