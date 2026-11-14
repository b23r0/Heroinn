//! Client-side reverse SOCKS5 streams.
//!
//! Target connections are made directly by the client. Only the reverse
//! tunnel back to the server sidecar uses the exact proxy route frozen from
//! the main connection.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use heroinn_util::socks5::{
    SOCKS5_MAX_CONCURRENCY, SOCKS5_TARGET_CONNECT_TIMEOUT, SOCKS5_TUNNEL_PUMP_CHUNK, Socks5Address,
    Socks5ClientEvent, Socks5FailureCategory, Socks5ServerCommand, Socks5ServerOpen,
    Socks5TunnelFrame, sanitize_detail,
};
use heroinn_util::{
    Error, HeroinnClientMsgID, HeroinnProtocol,
    packet::Message,
    protocol::{FrozenRoute, Transport, TransportFactory},
};

use crate::module::transfer;

type OutgoingSender = std::sync::mpsc::Sender<Vec<u8>>;
type StreamKey = (String, u64);
type StreamRegistry = Arc<Mutex<HashMap<StreamKey, Arc<StreamCancel>>>>;

#[derive(Clone)]
struct SessionRoute {
    protocol: HeroinnProtocol,
    server_address: String,
    frozen: Option<FrozenRoute>,
}

struct StreamCancel {
    cancelled: Arc<AtomicBool>,
    target: Mutex<Option<TcpStream>>,
    tunnel: Mutex<Option<Transport>>,
}

impl StreamCancel {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            target: Mutex::new(None),
            tunnel: Mutex::new(None),
        }
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(target) = hlock(&self.target).as_ref() {
            let _ = target.shutdown(Shutdown::Both);
        }
        if let Some(tunnel) = hlock(&self.tunnel).as_ref() {
            let _ = tunnel.shutdown(Shutdown::Both);
        }
    }

    fn set_target(&self, target: &TcpStream) {
        if let Ok(clone) = target.try_clone() {
            *hlock(&self.target) = Some(clone);
        }
    }

    fn set_tunnel(&self, tunnel: &Transport) {
        *hlock(&self.tunnel) = Some(tunnel.clone());
    }
}

/// Per-main-connection reverse SOCKS5 session and stream registry.
pub struct Socks5Manager {
    clientid: String,
    route: SessionRoute,
    factory: Arc<TransportFactory>,
    outgoing: OutgoingSender,
    sessions: HashMap<String, Arc<SessionRoute>>,
    streams: StreamRegistry,
}

impl Socks5Manager {
    pub fn new(
        clientid: &str,
        protocol: HeroinnProtocol,
        server_address: &str,
        frozen: Option<FrozenRoute>,
        factory: Arc<TransportFactory>,
        outgoing: OutgoingSender,
    ) -> Self {
        Self {
            clientid: clientid.to_string(),
            route: SessionRoute {
                protocol,
                server_address: server_address.to_string(),
                frozen,
            },
            factory,
            outgoing,
            sessions: HashMap::new(),
            streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn handle(&mut self, command: Socks5ServerCommand) {
        match command {
            Socks5ServerCommand::Start { session_id } => {
                self.sessions
                    .entry(session_id)
                    .or_insert_with(|| Arc::new(self.route.clone()));
            }
            Socks5ServerCommand::Open(open) => self.open(open),
            Socks5ServerCommand::Shutdown { session_id } => self.shutdown(&session_id),
        }
    }

    fn open(&mut self, open: Socks5ServerOpen) {
        let key = (open.session_id.clone(), open.stream_id);
        if open.tunnel_port == 0 {
            self.send_failure(
                &open.session_id,
                open.stream_id,
                Socks5FailureCategory::Protocol,
                "invalid reverse tunnel port",
            );
            return;
        }

        let Some(route) = self.sessions.get(&open.session_id).cloned() else {
            self.send_failure(
                &open.session_id,
                open.stream_id,
                Socks5FailureCategory::Closed,
                "unknown reverse SOCKS5 session",
            );
            return;
        };
        let cancel = Arc::new(StreamCancel::new());
        {
            let mut streams = hlock(&self.streams);
            if streams.contains_key(&key) {
                drop(streams);
                self.send_failure(
                    &open.session_id,
                    open.stream_id,
                    Socks5FailureCategory::Internal,
                    "duplicate SOCKS5 stream id",
                );
                return;
            }
            if streams.len() >= SOCKS5_MAX_CONCURRENCY {
                drop(streams);
                self.send_failure(
                    &open.session_id,
                    open.stream_id,
                    Socks5FailureCategory::Rejected,
                    "SOCKS5 concurrency limit reached",
                );
                return;
            }
            streams.insert(key.clone(), cancel.clone());
        }

        let factory = self.factory.clone();
        let outgoing = self.outgoing.clone();
        let clientid = self.clientid.clone();
        let streams = self.streams.clone();
        let result = std::thread::Builder::new()
            .name(format!(
                "socks5-stream-{}-{}",
                open.session_id, open.stream_id
            ))
            .spawn(move || {
                run_stream(clientid, route, open, cancel, factory, outgoing, streams);
            });
        if let Err(error) = result {
            hlock(&self.streams).remove(&key);
            self.send_failure(
                &key.0,
                key.1,
                Socks5FailureCategory::Internal,
                &format!("cannot start SOCKS5 stream worker: {error}"),
            );
        }
    }

    fn shutdown(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
        let cancelled = {
            let mut streams = hlock(&self.streams);
            let keys = streams
                .keys()
                .filter(|(id, _)| id == session_id)
                .cloned()
                .collect::<Vec<_>>();
            keys.into_iter()
                .filter_map(|key| streams.remove(&key))
                .collect::<Vec<_>>()
        };
        for cancel in cancelled {
            cancel.cancel();
        }
        self.send_event(Socks5ClientEvent::ShutdownAck {
            session_id: session_id.to_string(),
        });
    }

    pub fn close_all(&mut self) {
        let sessions = self.sessions.keys().cloned().collect::<Vec<_>>();
        for session in sessions {
            self.shutdown(&session);
        }
        for (_, cancel) in hlock(&self.streams).drain() {
            cancel.cancel();
        }
    }

    fn send_failure(
        &self,
        session_id: &str,
        stream_id: u64,
        category: Socks5FailureCategory,
        detail: &str,
    ) {
        self.send_event(Socks5ClientEvent::OpenResult {
            session_id: session_id.to_string(),
            stream_id,
            accepted: false,
            category: Some(category),
            detail: Some(sanitize_detail(detail)),
        });
    }

    fn send_event(&self, event: Socks5ClientEvent) {
        let Ok(frame) = Message::build(HeroinnClientMsgID::Socks5.to_u8(), &self.clientid, event)
        else {
            return;
        };
        let _ = self.outgoing.send(frame);
    }
}

impl Drop for Socks5Manager {
    fn drop(&mut self) {
        self.close_all();
    }
}

fn run_stream(
    clientid: String,
    route: Arc<SessionRoute>,
    open: Socks5ServerOpen,
    cancel: Arc<StreamCancel>,
    factory: Arc<TransportFactory>,
    outgoing: OutgoingSender,
    streams: StreamRegistry,
) {
    let session_id = open.session_id.clone();
    let stream_id = open.stream_id;
    let target_address = open.target.address();

    let target = match target_address.and_then(|address| connect_target(&address, open.target.port))
    {
        Ok(target) => target,
        Err(error) => {
            let category = classify_error(&error);
            send_client_event(
                &outgoing,
                &clientid,
                Socks5ClientEvent::OpenResult {
                    session_id: session_id.clone(),
                    stream_id,
                    accepted: false,
                    category: Some(category),
                    detail: Some(sanitize_detail(&error.to_string())),
                },
            );
            hlock(&streams).remove(&(session_id.clone(), stream_id));
            return;
        }
    };
    if cancel.cancelled.load(Ordering::SeqCst) {
        let _ = target.shutdown(Shutdown::Both);
        hlock(&streams).remove(&(session_id, stream_id));
        return;
    }
    cancel.set_target(&target);

    let mut tunnel = match open_reverse_tunnel(&route, open.tunnel_port, &factory) {
        Ok(tunnel) => tunnel,
        Err(error) => {
            let _ = target.shutdown(Shutdown::Both);
            send_client_event(
                &outgoing,
                &clientid,
                Socks5ClientEvent::OpenResult {
                    session_id: session_id.clone(),
                    stream_id,
                    accepted: false,
                    category: Some(Socks5FailureCategory::TunnelConnect),
                    detail: Some(sanitize_detail(&error.to_string())),
                },
            );
            hlock(&streams).remove(&(session_id.clone(), stream_id));
            return;
        }
    };
    cancel.set_tunnel(&tunnel);

    if let Err(error) = send_tunnel_frame(&mut tunnel, &Socks5TunnelFrame::Hello { stream_id }) {
        let _ = target.shutdown(Shutdown::Both);
        let _ = tunnel.shutdown(Shutdown::Both);
        send_client_event(
            &outgoing,
            &clientid,
            Socks5ClientEvent::OpenResult {
                session_id: session_id.clone(),
                stream_id,
                accepted: false,
                category: Some(Socks5FailureCategory::TunnelConnect),
                detail: Some(sanitize_detail(&error.to_string())),
            },
        );
        hlock(&streams).remove(&(session_id.clone(), stream_id));
        return;
    }

    send_client_event(
        &outgoing,
        &clientid,
        Socks5ClientEvent::OpenResult {
            session_id: session_id.clone(),
            stream_id,
            accepted: true,
            category: None,
            detail: None,
        },
    );

    let (bytes_to_target, bytes_from_target) = pump_stream(&target, &tunnel, cancel.clone());
    let _ = target.shutdown(Shutdown::Both);
    // `send` enqueues Binary frames for the WebSocket actor.  A raw socket
    // shutdown here would race that writer and truncate the final target
    // payload and FIN.  The transport's close path drains the queue, flushes
    // the close frame and joins the actor without waiting indefinitely.
    tunnel.close();
    send_client_event(
        &outgoing,
        &clientid,
        Socks5ClientEvent::Closed {
            session_id: session_id.clone(),
            stream_id,
            bytes_to_target,
            bytes_from_target,
        },
    );
    hlock(&streams).remove(&(session_id, stream_id));
}

fn open_reverse_tunnel(
    route: &SessionRoute,
    tunnel_port: u16,
    factory: &TransportFactory,
) -> heroinn_util::Result<Transport> {
    match route.protocol {
        HeroinnProtocol::TCP | HeroinnProtocol::HTTP => {
            let frozen = route.frozen.as_ref().ok_or_else(|| {
                Error::Other("main connection has no frozen proxy route".to_string())
            })?;
            Transport::tunnel_with_frozen_route(
                factory,
                &route.protocol,
                &route.server_address,
                tunnel_port,
                frozen,
            )
        }
        HeroinnProtocol::UDP => {
            Transport::tunnel_direct(&HeroinnProtocol::UDP, &route.server_address, tunnel_port)
        }
    }
}

fn connect_target(address: &Socks5Address, port: u16) -> heroinn_util::Result<TcpStream> {
    let deadline = Instant::now() + SOCKS5_TARGET_CONNECT_TIMEOUT;
    let addresses = address.resolve(port)?;
    let mut last_error = None;
    for address in addresses {
        let now = Instant::now();
        if now >= deadline {
            return Err(Error::Timeout);
        }
        match TcpStream::connect_timeout(&address, deadline.saturating_duration_since(now)) {
            Ok(stream) => {
                stream.set_nodelay(true)?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error
        .map(Error::Io)
        .unwrap_or_else(|| Error::NotFound("target resolved to no addresses".to_string())))
}

fn pump_stream(target: &TcpStream, tunnel: &Transport, cancel: Arc<StreamCancel>) -> (u64, u64) {
    let mut target_reader = match target.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            log::warn!("socks5 target reader clone failed: {error}");
            return (0, 0);
        }
    };
    let target_to_shutdown = match target.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            log::warn!("socks5 target shutdown clone failed: {error}");
            return (0, 0);
        }
    };
    let mut target_from_tunnel = match target.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            log::warn!("socks5 target writer clone failed: {error}");
            return (0, 0);
        }
    };
    let mut tunnel_sender = tunnel.clone();
    let mut tunnel_receiver = tunnel.clone();
    let cancel_to_tunnel = cancel.clone();
    let cancel_from_tunnel = cancel.clone();

    let to_tunnel = std::thread::spawn(move || {
        let mut bytes = 0u64;
        let mut buf = vec![0u8; SOCKS5_TUNNEL_PUMP_CHUNK];
        'pump: loop {
            if cancel_to_tunnel.cancelled.load(Ordering::SeqCst) {
                log::info!("socks5 target-to-tunnel cancelled");
                break;
            }
            if let Err(error) = target_reader.set_read_timeout(None) {
                log::warn!("socks5 target read timeout setup failed: {error}");
                break;
            }
            match target_reader.read(&mut buf) {
                Ok(0) => {
                    log::info!("socks5 target reached EOF");
                    let _ = send_tunnel_frame(&mut tunnel_sender, &Socks5TunnelFrame::Fin);
                    break;
                }
                Ok(count) => {
                    bytes += count as u64;
                    if let Err(error) = send_socks5_data(&mut tunnel_sender, buf[..count].to_vec())
                    {
                        log::warn!("socks5 target-to-tunnel send failed: {error}");
                        let _ = target_to_shutdown.shutdown(Shutdown::Both);
                        let _ = tunnel_sender.shutdown(Shutdown::Both);
                        break 'pump;
                    }
                }
                Err(error) => {
                    log::warn!("socks5 target read failed: {error}");
                    let _ = send_tunnel_frame(&mut tunnel_sender, &Socks5TunnelFrame::Reset);
                    let _ = target_to_shutdown.shutdown(Shutdown::Both);
                    break;
                }
            }
        }
        let _ = target_reader.set_read_timeout(None);
        log::info!("socks5 target-to-tunnel worker exited bytes={bytes}");
        bytes
    });

    let from_tunnel = std::thread::spawn(move || {
        let mut bytes = 0u64;
        loop {
            if cancel_from_tunnel.cancelled.load(Ordering::SeqCst) {
                log::info!("socks5 tunnel-to-target cancelled");
                break;
            }
            let frame = match recv_tunnel_frame(&mut tunnel_receiver) {
                Ok(frame) => frame,
                Err(error) => {
                    log::warn!("socks5 tunnel-to-target receive failed: {error}");
                    let _ = target_from_tunnel.shutdown(Shutdown::Both);
                    break;
                }
            };
            match frame {
                Socks5TunnelFrame::Data(data) => {
                    if let Err(error) = target_from_tunnel.write_all(&data) {
                        log::warn!("socks5 tunnel-to-target write failed: {error}");
                        let _ = tunnel_receiver.shutdown(Shutdown::Both);
                        break;
                    }
                    bytes += data.len() as u64;
                }
                Socks5TunnelFrame::Fin => {
                    let _ = target_from_tunnel.shutdown(Shutdown::Write);
                    break;
                }
                Socks5TunnelFrame::Reset => {
                    let _ = target_from_tunnel.shutdown(Shutdown::Both);
                    let _ = tunnel_receiver.shutdown(Shutdown::Both);
                    break;
                }
                Socks5TunnelFrame::Hello { .. } => {}
            }
        }
        log::info!("socks5 tunnel-to-target worker exited bytes={bytes}");
        bytes
    });

    let bytes_to_target = to_tunnel.join().unwrap_or(0);
    let bytes_from_target = from_tunnel.join().unwrap_or(0);
    log::info!(
        "socks5 stream pump finished: target_to_tunnel={bytes_to_target} tunnel_to_target={bytes_from_target}"
    );
    (bytes_to_target, bytes_from_target)
}

fn send_tunnel_frame(
    tunnel: &mut Transport,
    frame: &Socks5TunnelFrame,
) -> heroinn_util::Result<()> {
    transfer::send(tunnel, &frame.to_payload()?)
}

fn send_socks5_data(tunnel: &mut Transport, data: Vec<u8>) -> heroinn_util::Result<()> {
    send_tunnel_frame(tunnel, &Socks5TunnelFrame::Data(data))
}

fn recv_tunnel_frame(tunnel: &mut Transport) -> heroinn_util::Result<Socks5TunnelFrame> {
    let payload = transfer::recv(tunnel)?;
    Socks5TunnelFrame::from_payload(&payload)
}

fn send_client_event(outgoing: &OutgoingSender, clientid: &str, event: Socks5ClientEvent) {
    let Ok(frame) = Message::build(HeroinnClientMsgID::Socks5.to_u8(), clientid, event) else {
        return;
    };
    let _ = outgoing.send(frame);
}

fn classify_error(error: &Error) -> Socks5FailureCategory {
    match error {
        Error::Timeout => Socks5FailureCategory::Timeout,
        Error::Proxy(_) => Socks5FailureCategory::TunnelConnect,
        Error::Io(io) if io.kind() == std::io::ErrorKind::ConnectionRefused => {
            Socks5FailureCategory::Rejected
        }
        Error::Io(_) => Socks5FailureCategory::TargetConnect,
        Error::NotFound(_) => Socks5FailureCategory::TargetDns,
        Error::Malformed(_) => Socks5FailureCategory::Protocol,
        _ => Socks5FailureCategory::Internal,
    }
}

fn hlock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use heroinn_util::protocol::http::WSServer;
    use heroinn_util::protocol::tcp::TcpServer;
    use heroinn_util::socks5::{
        Socks5ClientEvent, Socks5ServerCommand, Socks5ServerOpen, Socks5Target, Socks5TunnelFrame,
    };
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn manager_rejects_duplicate_stream_without_replacing_live_cancel_handle() {
        let (sender, _receiver) = std::sync::mpsc::channel();
        let mut manager = Socks5Manager::new(
            "client",
            HeroinnProtocol::TCP,
            "127.0.0.1:1",
            Some(FrozenRoute::direct()),
            Arc::new(TransportFactory::direct()),
            sender,
        );
        let open = Socks5ServerOpen {
            session_id: "session".to_string(),
            stream_id: 1,
            target: Socks5Target::new("127.0.0.1", 9).unwrap(),
            tunnel_port: 1,
        };
        manager
            .sessions
            .insert("session".to_string(), Arc::new(manager.route.clone()));
        let existing = Arc::new(StreamCancel::new());
        hlock(&manager.streams).insert(("session".to_string(), 1), existing.clone());
        manager.open(open.clone());
        assert_eq!(hlock(&manager.streams).len(), 1);
        assert!(Arc::ptr_eq(
            hlock(&manager.streams)
                .get(&("session".to_string(), 1))
                .unwrap(),
            &existing
        ));
        manager.close_all();
    }

    #[test]
    fn manager_rejects_concurrency_overflow_atomically() {
        let (sender, _receiver) = std::sync::mpsc::channel();
        let mut manager = Socks5Manager::new(
            "client",
            HeroinnProtocol::TCP,
            "127.0.0.1:1",
            Some(FrozenRoute::direct()),
            Arc::new(TransportFactory::direct()),
            sender,
        );
        manager
            .sessions
            .insert("session".to_string(), Arc::new(manager.route.clone()));
        {
            let mut streams = hlock(&manager.streams);
            for stream_id in 0..SOCKS5_MAX_CONCURRENCY as u64 {
                streams.insert(
                    ("session".to_string(), stream_id),
                    Arc::new(StreamCancel::new()),
                );
            }
        }
        manager.open(Socks5ServerOpen {
            session_id: "session".to_string(),
            stream_id: SOCKS5_MAX_CONCURRENCY as u64,
            target: Socks5Target::new("127.0.0.1", 9).unwrap(),
            tunnel_port: 1,
        });
        assert_eq!(hlock(&manager.streams).len(), SOCKS5_MAX_CONCURRENCY);
        manager.close_all();
        assert!(hlock(&manager.streams).is_empty());
    }

    #[test]
    fn manager_relays_data_and_half_close_through_a_frozen_reverse_tunnel() {
        let target_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let target_address = target_listener.local_addr().unwrap();
        let target_worker = std::thread::spawn(move || {
            let (mut target, _) = target_listener.accept().unwrap();
            target
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut ping = [0u8; 4];
            target.read_exact(&mut ping).unwrap();
            assert_eq!(&ping, b"ping");
            target.write_all(b"pong").unwrap();
            let _ = target.shutdown(Shutdown::Write);
        });

        let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let tunnel_port = tunnel_listener.local_addr().unwrap().port();
        let mut server = TcpServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let server_address = server.local_addr().unwrap().to_string();
        let (sender, receiver) = mpsc::channel();
        let mut manager = Socks5Manager::new(
            "client",
            HeroinnProtocol::TCP,
            &server_address,
            Some(FrozenRoute::direct()),
            Arc::new(TransportFactory::direct()),
            sender,
        );
        manager.handle(Socks5ServerCommand::Start {
            session_id: "session".to_string(),
        });
        manager.handle(Socks5ServerCommand::Open(Socks5ServerOpen {
            session_id: "session".to_string(),
            stream_id: 1,
            target: Socks5Target::new(&target_address.ip().to_string(), target_address.port())
                .unwrap(),
            tunnel_port,
        }));

        let (mut tunnel, _) = tunnel_listener.accept().unwrap();
        tunnel
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        assert_eq!(
            Socks5TunnelFrame::read_from(&mut tunnel).unwrap(),
            Socks5TunnelFrame::Hello { stream_id: 1 }
        );
        let accepted = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("open result");
        let frame = Message::new(
            "127.0.0.1:1".parse().unwrap(),
            HeroinnProtocol::TCP,
            &accepted,
        )
        .unwrap();
        assert!(matches!(
            frame.parse_socks5_client_event().unwrap(),
            Socks5ClientEvent::OpenResult { accepted: true, .. }
        ));

        Socks5TunnelFrame::Data(b"ping".to_vec())
            .write_to(&mut tunnel)
            .unwrap();
        assert_eq!(
            Socks5TunnelFrame::read_from(&mut tunnel).unwrap(),
            Socks5TunnelFrame::Data(b"pong".to_vec())
        );
        assert_eq!(
            Socks5TunnelFrame::read_from(&mut tunnel).unwrap(),
            Socks5TunnelFrame::Fin
        );
        target_worker.join().unwrap();

        manager.handle(Socks5ServerCommand::Shutdown {
            session_id: "session".to_string(),
        });
        manager.close_all();
        server.close();
    }

    #[test]
    fn manager_flushes_the_echo_and_fin_through_a_websocket_tunnel() {
        let payload = (0..64 * 1024)
            .map(|index| (index * 17 + 3) as u8)
            .collect::<Vec<_>>();

        let target_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let target_address = target_listener.local_addr().unwrap();
        let target_payload = payload.clone();
        let target_worker = std::thread::spawn(move || {
            let (mut target, _) = target_listener.accept().unwrap();
            target
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut received = vec![0u8; target_payload.len()];
            target.read_exact(&mut received).unwrap();
            assert_eq!(received, target_payload);
            target.write_all(&target_payload).unwrap();
            let _ = target.shutdown(Shutdown::Write);
        });

        let sidecar_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let sidecar_port = sidecar_listener.local_addr().unwrap().port();
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let server_address = server.local_addr().unwrap().to_string();
        let (sender, receiver) = mpsc::channel();
        let mut manager = Socks5Manager::new(
            "client",
            HeroinnProtocol::HTTP,
            &server_address,
            Some(FrozenRoute::direct()),
            Arc::new(TransportFactory::direct()),
            sender,
        );
        manager.handle(Socks5ServerCommand::Start {
            session_id: "session".to_string(),
        });
        manager.handle(Socks5ServerCommand::Open(Socks5ServerOpen {
            session_id: "session".to_string(),
            stream_id: 7,
            target: Socks5Target::new(&target_address.ip().to_string(), target_address.port())
                .unwrap(),
            tunnel_port: sidecar_port,
        }));

        let (mut sidecar_tunnel, _) = sidecar_listener.accept().unwrap();
        sidecar_tunnel
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        assert_eq!(
            Socks5TunnelFrame::read_from(&mut sidecar_tunnel).unwrap(),
            Socks5TunnelFrame::Hello { stream_id: 7 }
        );
        let accepted = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("open result");
        let frame = Message::new(
            "127.0.0.1:1".parse().unwrap(),
            HeroinnProtocol::HTTP,
            &accepted,
        )
        .unwrap();
        assert!(matches!(
            frame.parse_socks5_client_event().unwrap(),
            Socks5ClientEvent::OpenResult { accepted: true, .. }
        ));

        Socks5TunnelFrame::Data(payload.clone())
            .write_to(&mut sidecar_tunnel)
            .unwrap();
        Socks5TunnelFrame::Fin
            .write_to(&mut sidecar_tunnel)
            .unwrap();

        let mut echoed = Vec::new();
        let mut saw_fin = false;
        while !saw_fin {
            match Socks5TunnelFrame::read_from(&mut sidecar_tunnel).unwrap() {
                Socks5TunnelFrame::Data(data) => echoed.extend_from_slice(&data),
                Socks5TunnelFrame::Fin => saw_fin = true,
                other => panic!("unexpected tunnel frame: {other:?}"),
            }
        }
        assert_eq!(echoed, payload);
        target_worker.join().unwrap();
        drop(sidecar_tunnel);

        manager.handle(Socks5ServerCommand::Shutdown {
            session_id: "session".to_string(),
        });
        manager.close_all();
        server.close();
    }

    #[test]
    fn manager_flushes_sixteen_mebibytes_and_fin_through_a_websocket_tunnel() {
        const SIZE: usize = 16 * 1024 * 1024;
        const CHUNK: usize = 64 * 1024;
        struct RestoreRate(u64);
        impl Drop for RestoreRate {
            fn drop(&mut self) {
                let _ = crate::module::transfer::set_rate(self.0);
            }
        }
        let _restore_rate = RestoreRate(crate::module::transfer::current_rate());
        crate::module::transfer::set_rate(1024 * 1024 * 1024).unwrap();

        let payload = (0..SIZE)
            .map(|index| (index.wrapping_mul(31).wrapping_add(11)) as u8)
            .collect::<Vec<_>>();

        let target_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let target_address = target_listener.local_addr().unwrap();
        let target_payload = payload.clone();
        let target_worker = std::thread::spawn(move || {
            let (mut target, _) = target_listener.accept().unwrap();
            target
                .set_read_timeout(Some(Duration::from_secs(30)))
                .unwrap();
            let mut received = Vec::with_capacity(target_payload.len());
            let mut chunk = vec![0u8; CHUNK];
            while received.len() < target_payload.len() {
                let count = target.read(&mut chunk).unwrap();
                if count == 0 {
                    break;
                }
                received.extend_from_slice(&chunk[..count]);
            }
            assert_eq!(received.len(), target_payload.len());
            assert_eq!(received, target_payload);
            target.write_all(&target_payload).unwrap();
            let _ = target.shutdown(Shutdown::Write);
        });

        let sidecar_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let sidecar_port = sidecar_listener.local_addr().unwrap().port();
        let mut server = WSServer::new("127.0.0.1:0", Arc::new(|_| {})).unwrap();
        let server_address = server.local_addr().unwrap().to_string();
        let (sender, receiver) = mpsc::channel();
        let mut manager = Socks5Manager::new(
            "client",
            HeroinnProtocol::HTTP,
            &server_address,
            Some(FrozenRoute::direct()),
            Arc::new(TransportFactory::direct()),
            sender,
        );
        manager.handle(Socks5ServerCommand::Start {
            session_id: "session".to_string(),
        });
        manager.handle(Socks5ServerCommand::Open(Socks5ServerOpen {
            session_id: "session".to_string(),
            stream_id: 8,
            target: Socks5Target::new(&target_address.ip().to_string(), target_address.port())
                .unwrap(),
            tunnel_port: sidecar_port,
        }));

        let (mut sidecar_tunnel, _) = sidecar_listener.accept().unwrap();
        sidecar_tunnel
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        sidecar_tunnel
            .set_write_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        assert_eq!(
            Socks5TunnelFrame::read_from(&mut sidecar_tunnel).unwrap(),
            Socks5TunnelFrame::Hello { stream_id: 8 }
        );
        let accepted = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("open result");
        let frame = Message::new(
            "127.0.0.1:1".parse().unwrap(),
            HeroinnProtocol::HTTP,
            &accepted,
        )
        .unwrap();
        assert!(matches!(
            frame.parse_socks5_client_event().unwrap(),
            Socks5ClientEvent::OpenResult { accepted: true, .. }
        ));

        let mut sidecar_reader = sidecar_tunnel.try_clone().unwrap();
        sidecar_reader
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        let echo_reader = std::thread::spawn(move || {
            let mut echoed = Vec::with_capacity(SIZE);
            let mut saw_fin = false;
            while !saw_fin {
                match Socks5TunnelFrame::read_from(&mut sidecar_reader).expect("large tunnel frame")
                {
                    Socks5TunnelFrame::Data(data) => echoed.extend_from_slice(&data),
                    Socks5TunnelFrame::Fin => saw_fin = true,
                    other => panic!("unexpected tunnel frame: {other:?}"),
                }
            }
            echoed
        });

        let mut sidecar_written = 0usize;
        for chunk in payload.chunks(CHUNK) {
            if let Err(error) =
                Socks5TunnelFrame::Data(chunk.to_vec()).write_to(&mut sidecar_tunnel)
            {
                panic!("sidecar write failed after {sidecar_written} bytes: {error}");
            }
            sidecar_written += chunk.len();
        }
        Socks5TunnelFrame::Fin
            .write_to(&mut sidecar_tunnel)
            .unwrap();

        let echoed = echo_reader.join().unwrap();
        assert_eq!(echoed.len(), SIZE);
        assert_eq!(echoed, payload);
        target_worker.join().unwrap();
        drop(sidecar_tunnel);

        manager.handle(Socks5ServerCommand::Shutdown {
            session_id: "session".to_string(),
        });
        manager.close_all();
        server.close();
    }

    #[test]
    fn failures_are_classified_without_collapsing_distinct_causes() {
        assert_eq!(
            classify_error(&Error::Timeout),
            Socks5FailureCategory::Timeout
        );
        assert_eq!(
            classify_error(&Error::NotFound("dns".to_string())),
            Socks5FailureCategory::TargetDns
        );
        assert_eq!(
            classify_error(&Error::Malformed("bad")),
            Socks5FailureCategory::Protocol
        );
        assert_eq!(
            classify_error(&Error::Other("internal".to_string())),
            Socks5FailureCategory::Internal
        );
        assert_eq!(
            classify_error(&Error::Io(std::io::Error::from(
                std::io::ErrorKind::ConnectionRefused
            ))),
            Socks5FailureCategory::Rejected
        );
        assert_eq!(
            classify_error(&Error::Io(std::io::Error::from(
                std::io::ErrorKind::ConnectionReset
            ))),
            Socks5FailureCategory::TargetConnect
        );
    }
}
