//! Loopback SOCKS5 facade for one remote Heroinn client.

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use eframe::{App, egui};
#[cfg(test)]
use heroinn_util::protocol::frame::read_frame;
use heroinn_util::protocol::frame::{MAX_PACKET, write_frame};
use heroinn_util::socks5::{
    SOCKS5_MAX_CONCURRENCY, SOCKS5_MAX_TUNNEL_CHUNK, SOCKS5_REP_COMMAND_NOT_SUPPORTED,
    SOCKS5_REP_GENERAL_FAILURE, SOCKS5_TARGET_CONNECT_TIMEOUT, SOCKS5_TUNNEL_CONNECT_TIMEOUT,
    SOCKS5_TUNNEL_PUMP_CHUNK, Socks5FailureCategory, Socks5Request, Socks5RequestCommand,
    Socks5SidecarCommand, Socks5SidecarEvent, Socks5SidecarReady, Socks5Target, Socks5TunnelFrame,
    negotiate_noauth, sanitize_detail, write_socks5_reply,
};
use socket2::{Domain, Socket, Type};

const LOG_LIMIT: usize = 512;
const LOG_BYTES_LIMIT: usize = 256 * 1024;
const STOP_POLL: Duration = Duration::from_millis(100);
const PREAUTH_READ_POLL: Duration = Duration::from_millis(100);
const CONTROL_POLL: Duration = Duration::from_millis(100);
const LISTENER_JOIN_TIMEOUT: Duration = Duration::from_secs(3);
const RUNTIME_JOIN_TIMEOUT: Duration = Duration::from_secs(5);
const DATA_PLANE_POLL: Duration = Duration::from_millis(250);

#[derive(Clone, Debug)]
struct Arguments {
    control_port: u16,
    session_id: String,
    client_label: String,
    proxy_chain: String,
    transport: String,
}

impl Arguments {
    fn parse() -> Result<Self, String> {
        let mut control_port = None;
        let mut session_id = None;
        let mut client_label = String::new();
        let mut proxy_chain = String::new();
        let mut transport = String::new();
        let mut args = std::env::args().skip(1);
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--control-port" => {
                    control_port = Some(
                        args.next()
                            .ok_or_else(|| "missing --control-port value".to_string())?
                            .parse()
                            .map_err(|_| "invalid --control-port value".to_string())?,
                    )
                }
                "--session-id" => {
                    session_id = Some(
                        args.next()
                            .ok_or_else(|| "missing --session-id value".to_string())?,
                    )
                }
                "--client-label" => {
                    client_label = args
                        .next()
                        .ok_or_else(|| "missing --client-label value".to_string())?
                }
                "--proxy-chain" => {
                    proxy_chain = args
                        .next()
                        .ok_or_else(|| "missing --proxy-chain value".to_string())?
                }
                "--transport" => {
                    transport = args
                        .next()
                        .ok_or_else(|| "missing --transport value".to_string())?
                }
                other => return Err(format!("unknown argument {other}")),
            }
        }
        Ok(Self {
            control_port: control_port.ok_or_else(|| "--control-port is required".to_string())?,
            session_id: session_id.ok_or_else(|| "--session-id is required".to_string())?,
            client_label: sanitize_detail(&client_label),
            proxy_chain: sanitize_detail(&proxy_chain),
            transport: sanitize_detail(&transport),
        })
    }
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub time: String,
    pub stream_id: Option<u64>,
    pub target: String,
    pub status: String,
    pub bytes_to_target: u64,
    pub bytes_from_target: u64,
    pub category: Option<String>,
    pub detail: Option<String>,
}

#[derive(Clone)]
struct LogBuffer {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
}

impl LogBuffer {
    fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn push(&self, mut entry: LogEntry) {
        entry.status = sanitize_detail(&entry.status);
        entry.detail = entry.detail.as_deref().map(sanitize_detail);
        entry.category = entry.category.as_deref().map(sanitize_detail);
        log::info!(
            "stream={:?} target={} status={} bytes_in={} bytes_out={} category={:?}",
            entry.stream_id,
            entry.target,
            entry.status,
            entry.bytes_from_target,
            entry.bytes_to_target,
            entry.category
        );
        let mut entries = hlock(&self.entries);
        entries.push_front(entry);
        while entries.len() > LOG_LIMIT {
            entries.pop_back();
        }
        while entries
            .iter()
            .map(|entry| {
                entry.target.len()
                    + entry.status.len()
                    + entry.detail.as_deref().unwrap_or("").len()
                    + 96
            })
            .sum::<usize>()
            > LOG_BYTES_LIMIT
        {
            entries.pop_back();
        }
    }

    fn snapshot(&self) -> Vec<LogEntry> {
        hlock(&self.entries).iter().cloned().collect()
    }

    fn clear(&self) {
        hlock(&self.entries).clear();
    }
}

#[derive(Clone)]
struct StreamState {
    target: Socks5Target,
    control: Arc<Mutex<StreamControl>>,
    ready: Arc<Condvar>,
    cancelled: Arc<AtomicBool>,
}

struct StreamControl {
    client: Option<TcpStream>,
    tunnel: Option<TcpStream>,
    cancel_tunnel: Option<TcpStream>,
    outcome: Outcome,
}

#[derive(Clone, Debug)]
enum Outcome {
    Pending,
    TunnelReady,
    Failed(Socks5FailureCategory, String),
    Closed,
}

struct Registry {
    streams: Mutex<HashMap<u64, StreamState>>,
    next_id: Arc<AtomicU64>,
    active: AtomicUsize,
    stopped: Arc<AtomicBool>,
    logs: LogBuffer,
}

impl Registry {
    #[cfg(test)]
    fn new(stopped: Arc<AtomicBool>, logs: LogBuffer) -> Self {
        Self::new_with_id(stopped, logs, Arc::new(AtomicU64::new(1)))
    }

    fn new_with_id(stopped: Arc<AtomicBool>, logs: LogBuffer, next_id: Arc<AtomicU64>) -> Self {
        Self {
            streams: Mutex::new(HashMap::new()),
            next_id,
            active: AtomicUsize::new(0),
            stopped,
            logs,
        }
    }

    fn allocate(
        &self,
        target: Socks5Target,
        client: &TcpStream,
    ) -> Result<(u64, StreamState), String> {
        let client = client
            .try_clone()
            .map_err(|error| format!("clone SOCKS5 client socket: {error}"))?;
        let mut streams = hlock(&self.streams);
        if streams.len() >= SOCKS5_MAX_CONCURRENCY {
            return Err("concurrency limit reached".to_string());
        }
        let stream_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let state = StreamState {
            target: target.clone(),
            control: Arc::new(Mutex::new(StreamControl {
                client: Some(client),
                tunnel: None,
                cancel_tunnel: None,
                outcome: Outcome::Pending,
            })),
            ready: Arc::new(Condvar::new()),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        streams.insert(stream_id, state.clone());
        self.active.store(streams.len(), Ordering::Relaxed);
        self.logs.push(log(
            Some(stream_id),
            target.authority(),
            "opening",
            0,
            0,
            None,
            None,
        ));
        Ok((stream_id, state))
    }

    fn accept_tunnel(&self, stream_id: u64, tunnel: TcpStream) -> Result<(), &'static str> {
        let state = {
            let streams = hlock(&self.streams);
            streams.get(&stream_id).cloned()
        };
        let Some(state) = state else {
            return Err("unknown stream id");
        };
        if state.cancelled.load(Ordering::SeqCst) {
            return Err("stream was cancelled");
        }
        let mut control = hlock(&state.control);
        if !matches!(control.outcome, Outcome::Pending) {
            return Err("stream is no longer pending");
        }
        control.tunnel = Some(tunnel);
        control.outcome = Outcome::TunnelReady;
        drop(control);
        state.ready.notify_all();
        Ok(())
    }

    fn fail(&self, stream_id: u64, category: Socks5FailureCategory, detail: String) -> bool {
        let state = {
            let streams = hlock(&self.streams);
            streams.get(&stream_id).cloned()
        };
        let Some(state) = state else {
            return false;
        };
        if state.cancelled.load(Ordering::SeqCst) {
            return false;
        }
        let mut control = hlock(&state.control);
        if matches!(control.outcome, Outcome::TunnelReady | Outcome::Closed) {
            return false;
        }
        control.outcome = Outcome::Failed(category, sanitize_detail(&detail));
        drop(control);
        state.ready.notify_all();
        true
    }

    fn wait_ready(
        &self,
        state: &StreamState,
        timeout: Duration,
    ) -> Result<TcpStream, (Socks5FailureCategory, String)> {
        let deadline = Instant::now() + timeout;
        let mut control = hlock(&state.control);
        loop {
            match &control.outcome {
                Outcome::TunnelReady => {
                    let tunnel = control.tunnel.take().ok_or_else(|| {
                        (
                            Socks5FailureCategory::Internal,
                            "reverse tunnel disappeared".to_string(),
                        )
                    })?;
                    control.cancel_tunnel = Some(tunnel.try_clone().map_err(|error| {
                        (
                            Socks5FailureCategory::Internal,
                            format!("clone active reverse tunnel for cancellation: {error}"),
                        )
                    })?);
                    return Ok(tunnel);
                }
                Outcome::Failed(category, detail) => return Err((*category, detail.clone())),
                Outcome::Closed => {
                    return Err((
                        Socks5FailureCategory::Closed,
                        "stream closed while opening".to_string(),
                    ));
                }
                Outcome::Pending => {}
            }
            if self.stopped.load(Ordering::SeqCst) {
                return Err((
                    Socks5FailureCategory::Closed,
                    "SOCKS5 listener stopped".to_string(),
                ));
            }
            if state.cancelled.load(Ordering::SeqCst) {
                return Err((
                    Socks5FailureCategory::Closed,
                    "SOCKS5 stream was cancelled".to_string(),
                ));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err((
                    Socks5FailureCategory::Timeout,
                    "reverse tunnel was not established within 10 seconds".to_string(),
                ));
            }
            let (next, _) = state
                .ready
                .wait_timeout(control, remaining)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            control = next;
        }
    }

    fn remove(&self, stream_id: u64) {
        let state = {
            let mut streams = hlock(&self.streams);
            let state = streams.remove(&stream_id);
            self.active.store(streams.len(), Ordering::Relaxed);
            state
        };
        if let Some(state) = state {
            state.cancelled.store(true, Ordering::SeqCst);
            let mut control = hlock(&state.control);
            if let Some(client) = control.client.take() {
                let _ = client.shutdown(Shutdown::Both);
            }
            if let Some(tunnel) = control.tunnel.take() {
                let _ = tunnel.shutdown(Shutdown::Both);
            }
            if let Some(tunnel) = control.cancel_tunnel.take() {
                let _ = tunnel.shutdown(Shutdown::Both);
            }
            control.outcome = Outcome::Closed;
            drop(control);
            state.ready.notify_all();
        }
    }

    fn cancel_all(&self) {
        let ids = hlock(&self.streams).keys().copied().collect::<Vec<_>>();
        for id in ids {
            self.remove(id);
        }
    }

    fn log_closed(&self, stream_id: u64, bytes_to_target: u64, bytes_from_target: u64) {
        let target = hlock(&self.streams)
            .get(&stream_id)
            .map(|state| state.target.authority())
            .unwrap_or_else(|| "unknown".to_string());
        self.logs.push(log(
            Some(stream_id),
            target,
            "closed",
            bytes_to_target,
            bytes_from_target,
            None,
            None,
        ));
    }
}

struct ThreadRegistry {
    state: Mutex<ThreadRegistryState>,
    finished: Condvar,
}

struct ThreadRegistryState {
    closed: bool,
    handles: Vec<JoinHandle<()>>,
    running: usize,
}

impl ThreadRegistry {
    fn new() -> Self {
        Self {
            state: Mutex::new(ThreadRegistryState {
                closed: false,
                handles: Vec::new(),
                running: 0,
            }),
            finished: Condvar::new(),
        }
    }

    fn spawn<F>(self: &Arc<Self>, name: String, work: F) -> Result<(), String>
    where
        F: FnOnce() + Send + 'static,
    {
        let mut state = hlock(&self.state);
        if state.closed {
            return Err("thread registry is closed".to_string());
        }
        let registry = self.clone();
        let handle = std::thread::Builder::new()
            .name(name)
            .spawn(move || {
                let _ = catch_unwind(AssertUnwindSafe(work));
                let mut state = hlock(&registry.state);
                state.running = state.running.saturating_sub(1);
                registry.finished.notify_all();
            })
            .map_err(|error| format!("create worker thread: {error}"))?;
        state.running += 1;
        state.handles.push(handle);
        Ok(())
    }

    fn join_all(&self, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        let (handles, running) = {
            let mut state = hlock(&self.state);
            state.closed = true;
            while state.running > 0 {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                let (next, _) = self
                    .finished
                    .wait_timeout(state, remaining)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state = next;
            }
            (std::mem::take(&mut state.handles), state.running)
        };
        if running != 0 {
            return Err(format!(
                "{running} worker thread(s) did not stop within {timeout:?}"
            ));
        }
        for handle in handles {
            let _ = handle.join();
        }
        Ok(())
    }
}

#[derive(Default)]
struct PendingTunnels {
    next_id: AtomicU64,
    sockets: Mutex<HashMap<u64, TcpStream>>,
}

#[derive(Default)]
struct PendingClients {
    next_id: AtomicU64,
    cancelled: Arc<AtomicBool>,
    state: Mutex<PendingClientsState>,
}

#[derive(Default)]
struct PendingClientsState {
    closed: bool,
    sockets: HashMap<u64, TcpStream>,
}

struct PendingTunnel {
    registry: Arc<PendingTunnels>,
    id: u64,
    armed: bool,
}

struct PendingClient {
    registry: Arc<PendingClients>,
    id: u64,
    cancelled: Arc<AtomicBool>,
    armed: bool,
}

struct CancellableReader<'a> {
    stream: &'a mut TcpStream,
    cancelled: Arc<AtomicBool>,
    timeout: Duration,
}

impl PendingTunnels {
    fn track(self: &Arc<Self>, socket: &TcpStream) -> Result<PendingTunnel, String> {
        let clone = socket
            .try_clone()
            .map_err(|error| format!("clone reverse tunnel socket: {error}"))?;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        hlock(&self.sockets).insert(id, clone);
        Ok(PendingTunnel {
            registry: self.clone(),
            id,
            armed: true,
        })
    }

    fn close_all(&self) {
        let sockets = {
            let mut sockets = hlock(&self.sockets);
            sockets
                .drain()
                .map(|(_, socket)| socket)
                .collect::<Vec<_>>()
        };
        for socket in sockets {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
}

impl PendingTunnel {
    fn finish(mut self) {
        hlock(&self.registry.sockets).remove(&self.id);
        self.armed = false;
    }
}

impl Drop for PendingTunnel {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Some(socket) = hlock(&self.registry.sockets).remove(&self.id) {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
}

impl PendingClients {
    fn track(self: &Arc<Self>, socket: &TcpStream) -> Result<PendingClient, String> {
        let mut state = hlock(&self.state);
        if state.closed {
            return Err("SOCKS5 listener is stopping".to_string());
        }
        let clone = socket
            .try_clone()
            .map_err(|error| format!("clone SOCKS5 client for cancellation: {error}"))?;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        state.sockets.insert(id, clone);
        Ok(PendingClient {
            registry: self.clone(),
            id,
            cancelled: self.cancelled.clone(),
            armed: true,
        })
    }

    fn close_all(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let sockets = {
            let mut state = hlock(&self.state);
            state.closed = true;
            state
                .sockets
                .drain()
                .map(|(_, socket)| socket)
                .collect::<Vec<_>>()
        };
        for socket in sockets {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
}

impl<'a> CancellableReader<'a> {
    fn new(stream: &'a mut TcpStream, cancelled: Arc<AtomicBool>, timeout: Duration) -> Self {
        Self {
            stream,
            cancelled,
            timeout,
        }
    }
}

impl Read for CancellableReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let deadline = Instant::now() + self.timeout;
        loop {
            if self.cancelled.load(Ordering::SeqCst) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "SOCKS5 listener stopped",
                ));
            }
            match self.stream.read(buffer) {
                Ok(count) => return Ok(count),
                Err(error) if is_read_timeout(&error) => {
                    if Instant::now() >= deadline {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "SOCKS5 pre-authentication read timed out",
                        ));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }
}

impl PendingClient {
    fn finish(mut self) {
        hlock(&self.registry.state).sockets.remove(&self.id);
        self.armed = false;
    }
}

impl Drop for PendingClient {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Some(socket) = hlock(&self.registry.state).sockets.remove(&self.id) {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
}

#[derive(Clone)]
struct ActiveListener {
    registry: Arc<Registry>,
    stopped: Arc<AtomicBool>,
    threads: Arc<ThreadRegistry>,
}

struct ListenerRuntime {
    address: String,
    registry: Arc<Registry>,
    stopped: Arc<AtomicBool>,
    threads: Arc<ThreadRegistry>,
    active: Arc<Mutex<Option<ActiveListener>>>,
    pending_tunnels: Arc<PendingTunnels>,
    pending_clients: Arc<PendingClients>,
    logs: LogBuffer,
}

impl ListenerRuntime {
    #[allow(clippy::too_many_arguments)]
    fn start(
        listener: TcpListener,
        control: Arc<Mutex<TcpStream>>,
        active: Arc<Mutex<Option<ActiveListener>>>,
        shutdown: Arc<AtomicBool>,
        logs: LogBuffer,
        next_stream_id: Arc<AtomicU64>,
        pending_tunnels: Arc<PendingTunnels>,
        pending_clients: Arc<PendingClients>,
    ) -> Result<Self, String> {
        Self::start_with_threads(
            listener,
            control,
            active,
            shutdown,
            logs,
            next_stream_id,
            pending_tunnels,
            pending_clients,
            Arc::new(ThreadRegistry::new()),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn start_with_threads(
        listener: TcpListener,
        control: Arc<Mutex<TcpStream>>,
        active: Arc<Mutex<Option<ActiveListener>>>,
        shutdown: Arc<AtomicBool>,
        logs: LogBuffer,
        next_stream_id: Arc<AtomicU64>,
        pending_tunnels: Arc<PendingTunnels>,
        pending_clients: Arc<PendingClients>,
        threads: Arc<ThreadRegistry>,
    ) -> Result<Self, String> {
        if shutdown.load(Ordering::SeqCst) {
            return Err("SOCKS5 sidecar is shutting down".to_string());
        }
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("configure SOCKS5 listener: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("read SOCKS5 listener address: {error}"))?
            .to_string();
        let stopped = Arc::new(AtomicBool::new(false));
        let registry = Arc::new(Registry::new_with_id(
            stopped.clone(),
            logs.clone(),
            next_stream_id,
        ));
        spawn_socks_acceptor(
            listener,
            control,
            registry.clone(),
            stopped.clone(),
            logs.clone(),
            threads.clone(),
            pending_clients.clone(),
        )?;
        *hlock(&active) = Some(ActiveListener {
            registry: registry.clone(),
            stopped: stopped.clone(),
            threads: threads.clone(),
        });
        logs.push(log(
            None,
            "listener".to_string(),
            "started",
            0,
            0,
            None,
            Some(address.clone()),
        ));
        Ok(Self {
            address,
            registry,
            stopped,
            threads,
            active,
            pending_tunnels,
            pending_clients,
            logs,
        })
    }

    fn stop(&self) -> Result<(), String> {
        let first_stop = !self.stopped.swap(true, Ordering::SeqCst);
        if !first_stop {
            return Ok(());
        }
        let mut active = hlock(&self.active);
        if active
            .as_ref()
            .is_some_and(|listener| Arc::ptr_eq(&listener.registry, &self.registry))
        {
            *active = None;
        }
        drop(active);
        self.registry.cancel_all();
        self.pending_clients.close_all();
        self.pending_tunnels.close_all();
        let joined = self.threads.join_all(LISTENER_JOIN_TIMEOUT);
        self.logs.push(log(
            None,
            "listener".to_string(),
            "stopped",
            0,
            0,
            None,
            Some(self.address.clone()),
        ));
        joined
    }
}

impl Drop for ListenerRuntime {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListenerState {
    Running,
    Reserved,
    Stopped,
}

pub struct Runtime {
    args: Arguments,
    default_port: u16,
    tunnel_port: u16,
    stopped: Arc<AtomicBool>,
    control: Arc<Mutex<TcpStream>>,
    control_reader: Arc<Mutex<Option<TcpStream>>>,
    active: Arc<Mutex<Option<ActiveListener>>>,
    next_stream_id: Arc<AtomicU64>,
    reservation: Mutex<Option<Socket>>,
    listener: Mutex<Option<ListenerRuntime>>,
    pending_tunnels: Arc<PendingTunnels>,
    logs: LogBuffer,
    threads: Arc<ThreadRegistry>,
    close_started: AtomicBool,
    ready_sent: AtomicBool,
}

impl Runtime {
    fn start(args: Arguments) -> Result<Self, String> {
        let control = connect_control(args.control_port)?;
        let tunnel_listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("bind reverse tunnel listener: {error}"))?;
        let tunnel_port = tunnel_listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .port();
        let reservation = Socket::new(Domain::IPV4, Type::STREAM, None)
            .map_err(|error| format!("create initial SOCKS5 listener reservation: {error}"))?;
        reservation
            .bind(
                &"127.0.0.1:0"
                    .parse::<std::net::SocketAddr>()
                    .unwrap()
                    .into(),
            )
            .map_err(|error| format!("reserve initial SOCKS5 listener port: {error}"))?;
        let default_port = reservation
            .local_addr()
            .map_err(|error| format!("read reserved SOCKS5 listener port: {error}"))?
            .as_socket()
            .ok_or_else(|| "reserved SOCKS5 listener address is not IP".to_string())?
            .port();

        let stopped = Arc::new(AtomicBool::new(false));
        let logs = LogBuffer::new();
        let active = Arc::new(Mutex::new(None));
        let control = Arc::new(Mutex::new(control));
        let control_reader = Arc::new(Mutex::new(None));
        let threads = Arc::new(ThreadRegistry::new());

        let runtime = Self {
            args,
            default_port,
            tunnel_port,
            stopped,
            control,
            control_reader,
            active,
            next_stream_id: Arc::new(AtomicU64::new(1)),
            reservation: Mutex::new(Some(reservation)),
            listener: Mutex::new(None),
            pending_tunnels: Arc::new(PendingTunnels::default()),
            logs,
            threads,
            close_started: AtomicBool::new(false),
            ready_sent: AtomicBool::new(false),
        };
        if let Err(error) = runtime.spawn_runtime_threads(tunnel_listener) {
            runtime.close_with_reason("sidecar startup failed");
            return Err(error);
        }
        if let Err(error) = send_control(
            &runtime.control,
            &Socks5SidecarEvent::Ready(Socks5SidecarReady {
                session_id: runtime.args.session_id.clone(),
                socks_address: String::new(),
                tunnel_port: runtime.tunnel_port,
                proxy_chain: runtime.args.proxy_chain.clone(),
                transport: runtime.args.transport.clone(),
            }),
        ) {
            runtime.close_with_reason("ready event failed");
            return Err(error);
        }
        runtime.ready_sent.store(true, Ordering::SeqCst);
        Ok(runtime)
    }

    fn spawn_runtime_threads(&self, tunnel_listener: TcpListener) -> Result<(), String> {
        tunnel_listener
            .set_nonblocking(true)
            .map_err(|error| format!("configure reverse tunnel listener: {error}"))?;
        spawn_control_reader(
            self.control.clone(),
            self.control_reader.clone(),
            self.active.clone(),
            self.stopped.clone(),
            self.logs.clone(),
            self.threads.clone(),
        )?;
        spawn_tunnel_acceptor(
            tunnel_listener,
            self.active.clone(),
            self.stopped.clone(),
            self.pending_tunnels.clone(),
            self.logs.clone(),
            self.threads.clone(),
        )?;
        Ok(())
    }

    pub fn default_port(&self) -> u16 {
        self.default_port
    }

    pub fn listener_address(&self) -> Option<String> {
        hlock(&self.listener)
            .as_ref()
            .map(|listener| listener.address.clone())
    }

    fn listener_state(&self) -> ListenerState {
        let listener = hlock(&self.listener);
        if listener.is_some() {
            ListenerState::Running
        } else if hlock(&self.reservation).is_some() {
            ListenerState::Reserved
        } else {
            ListenerState::Stopped
        }
    }

    pub fn start_listener(&self, port: u16) -> Result<String, String> {
        if port == 0 {
            return Err("SOCKS5 listener port must be between 1 and 65535".to_string());
        }
        let mut slot = hlock(&self.listener);
        if slot.is_some() {
            return Err("SOCKS5 listener is already running".to_string());
        }
        let listener = {
            let mut reservation = hlock(&self.reservation);
            if port == self.default_port {
                reservation.take()
            } else {
                reservation.take();
                None
            }
        };
        let listener = match listener {
            Some(socket) => {
                #[cfg(unix)]
                socket.set_reuse_address(true).map_err(|error| {
                    format!("configure reserved loopback SOCKS5 listener on port {port}: {error}")
                })?;
                socket.listen(128).map_err(|error| {
                    format!("activate reserved loopback SOCKS5 listener on port {port}: {error}")
                })?;
                TcpListener::from(socket)
            }
            None => bind_socks_listener(port)?,
        };
        let listener = ListenerRuntime::start(
            listener,
            self.control.clone(),
            self.active.clone(),
            self.stopped.clone(),
            self.logs.clone(),
            self.next_stream_id.clone(),
            self.pending_tunnels.clone(),
            Arc::new(PendingClients::default()),
        )?;
        let address = listener.address.clone();
        *slot = Some(listener);
        Ok(address)
    }

    pub fn stop_listener(&self) -> Result<(), String> {
        let listener = hlock(&self.listener).take();
        if let Some(listener) = listener {
            listener.stop()
        } else {
            Ok(())
        }
    }

    pub fn tunnel_port(&self) -> u16 {
        self.tunnel_port
    }

    pub fn args(&self) -> (&str, &str, &str) {
        (
            &self.args.client_label,
            &self.args.proxy_chain,
            &self.args.transport,
        )
    }

    pub fn active_streams(&self) -> usize {
        hlock(&self.active)
            .as_ref()
            .map(|listener| listener.registry.active.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub fn logs(&self) -> Vec<LogEntry> {
        self.logs.snapshot()
    }

    pub fn clear_logs(&self) {
        self.logs.clear();
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    pub fn request_stop(&self, reason: &str) {
        self.close_with_reason(reason);
    }

    fn close(&self) {
        self.close_with_reason("GUI closed");
    }

    fn close_with_reason(&self, reason: &str) {
        if self.close_started.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Err(error) = self.stop_listener() {
            self.logs.push(log(
                None,
                "listener".to_string(),
                "stop_error",
                0,
                0,
                Some("internal".to_string()),
                Some(error),
            ));
        }
        *hlock(&self.active) = None;
        self.stopped.store(true, Ordering::SeqCst);
        self.cancel_active_streams();
        self.pending_tunnels.close_all();
        if self.ready_sent.load(Ordering::SeqCst) {
            let _ = send_control(
                &self.control,
                &Socks5SidecarEvent::Stopped {
                    reason: sanitize_detail(reason),
                },
            );
        }
        if let Some(reader) = hlock(&self.control_reader).as_ref() {
            let _ = reader.shutdown(Shutdown::Both);
        }
        let _ = hlock(&self.control).shutdown(Shutdown::Both);
        let _ = self.threads.join_all(RUNTIME_JOIN_TIMEOUT);
    }

    fn cancel_active_streams(&self) {
        if let Some(active) = hlock(&self.active).clone() {
            active.registry.cancel_all();
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn run() -> Result<(), String> {
    let args = Arguments::parse()?;
    let runtime = Arc::new(Runtime::start(args)?);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 620.0])
            .with_min_inner_size([760.0, 420.0]),
        ..Default::default()
    };
    let app_runtime = runtime.clone();
    eframe::run_native(
        &format!("Heroinn SOCKS5 Proxy - {}", runtime.args.client_label),
        options,
        Box::new(move |_cc| Ok(Box::new(Socks5App::new(app_runtime)))),
    )
    .map_err(|error| error.to_string())
}

fn connect_control(port: u16) -> Result<TcpStream, String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(stream) => {
                stream
                    .set_nodelay(true)
                    .map_err(|error| error.to_string())?;
                stream
                    .set_write_timeout(Some(SOCKS5_TARGET_CONNECT_TIMEOUT))
                    .map_err(|error| error.to_string())?;
                return Ok(stream);
            }
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(STOP_POLL);
            }
            Err(error) => {
                return Err(format!(
                    "connect to server control port 127.0.0.1:{port}: {error}"
                ));
            }
        }
    }
}

fn bind_socks_listener(port: u16) -> Result<TcpListener, String> {
    if port == 0 {
        return Err("SOCKS5 listener port must be between 1 and 65535".to_string());
    }
    let socket = new_loopback_socket()
        .map_err(|error| format!("create loopback SOCKS5 listener on port {port}: {error}"))?;
    socket
        .bind(
            &format!("127.0.0.1:{port}")
                .parse::<std::net::SocketAddr>()
                .expect("valid loopback address")
                .into(),
        )
        .map_err(|error| format!("bind loopback SOCKS5 listener on port {port}: {error}"))?;
    socket
        .listen(128)
        .map_err(|error| format!("listen on loopback SOCKS5 port {port}: {error}"))?;
    Ok(TcpListener::from(socket))
}

fn new_loopback_socket() -> std::io::Result<Socket> {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
    #[cfg(unix)]
    socket.set_reuse_address(true)?;
    Ok(socket)
}

fn send_control(control: &Arc<Mutex<TcpStream>>, event: &Socks5SidecarEvent) -> Result<(), String> {
    let payload = serde_json::to_vec(event).map_err(|error| error.to_string())?;
    let mut control = hlock(control);
    write_frame(&mut *control, &payload).map_err(|error| error.to_string())
}

fn spawn_control_reader(
    control: Arc<Mutex<TcpStream>>,
    control_reader: Arc<Mutex<Option<TcpStream>>>,
    active: Arc<Mutex<Option<ActiveListener>>>,
    stopped: Arc<AtomicBool>,
    logs: LogBuffer,
    threads: Arc<ThreadRegistry>,
) -> Result<(), String> {
    let reader = hlock(&control)
        .try_clone()
        .map_err(|error| format!("clone sidecar control socket: {error}"))?;
    reader
        .set_read_timeout(Some(CONTROL_POLL))
        .map_err(|error| format!("configure sidecar control reader: {error}"))?;
    let shutdown_reader = reader
        .try_clone()
        .map_err(|error| format!("clone sidecar control shutdown socket: {error}"))?;
    *hlock(&control_reader) = Some(shutdown_reader);
    threads.spawn("socks5-control-reader".to_string(), move || {
        let mut reader = ControlFrameReader::new(reader);
        loop {
            if stopped.load(Ordering::SeqCst) {
                break;
            }
            let payload = match reader.try_read_frame() {
                Ok(Some(payload)) => payload,
                Ok(None) => continue,
                Err(_) => break,
            };
            let command = match serde_json::from_slice::<Socks5SidecarCommand>(&payload) {
                Ok(command) => command,
                Err(error) => {
                    logs.push(log(
                        None,
                        "local".to_string(),
                        "control_error",
                        0,
                        0,
                        Some("protocol".to_string()),
                        Some(error.to_string()),
                    ));
                    continue;
                }
            };
            match command {
                Socks5SidecarCommand::OpenResult {
                    stream_id,
                    accepted,
                    category,
                    detail,
                } => {
                    if let Some(listener) = hlock(&active).clone() {
                        if accepted {
                            logs.push(log(
                                Some(stream_id),
                                "remote".to_string(),
                                "target_connected",
                                0,
                                0,
                                None,
                                None,
                            ));
                        } else {
                            listener.registry.fail(
                                stream_id,
                                category.unwrap_or(Socks5FailureCategory::Internal),
                                detail.unwrap_or_else(|| "client rejected stream".to_string()),
                            );
                        }
                    }
                }
                Socks5SidecarCommand::Closed {
                    stream_id,
                    bytes_to_target,
                    bytes_from_target,
                } => {
                    if let Some(listener) = hlock(&active).clone() {
                        listener
                            .registry
                            .log_closed(stream_id, bytes_to_target, bytes_from_target);
                    }
                }
                Socks5SidecarCommand::Shutdown => {
                    stopped.store(true, Ordering::SeqCst);
                    if let Some(listener) = hlock(&active).as_ref() {
                        listener.stopped.store(true, Ordering::SeqCst);
                        listener.registry.cancel_all();
                    }
                    break;
                }
            }
        }
        stopped.store(true, Ordering::SeqCst);
        if let Some(listener) = hlock(&active).as_ref() {
            listener.stopped.store(true, Ordering::SeqCst);
            listener.registry.cancel_all();
        }
        logs.push(log(
            None,
            "local".to_string(),
            "stopped",
            0,
            0,
            None,
            Some("server control closed".to_string()),
        ));
    })
}

struct ControlFrameReader {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl ControlFrameReader {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buffer: Vec::new(),
        }
    }

    fn try_read_frame(&mut self) -> std::io::Result<Option<Vec<u8>>> {
        loop {
            if self.buffer.len() >= 4 {
                let size =
                    u32::from_be_bytes(self.buffer[..4].try_into().expect("four byte length"))
                        as usize;
                if size > MAX_PACKET {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("control frame exceeds {MAX_PACKET} bytes"),
                    ));
                }
                let total = 4 + size;
                if self.buffer.len() >= total {
                    let payload = self.buffer[4..total].to_vec();
                    self.buffer.drain(..total);
                    return Ok(Some(payload));
                }
            }

            let mut chunk = [0u8; 4096];
            match self.stream.read(&mut chunk) {
                Ok(0) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "server control closed",
                    ));
                }
                Ok(count) => self.buffer.extend_from_slice(&chunk[..count]),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    return Ok(None);
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
    }
}

fn spawn_socks_acceptor(
    listener: TcpListener,
    control: Arc<Mutex<TcpStream>>,
    registry: Arc<Registry>,
    stopped: Arc<AtomicBool>,
    logs: LogBuffer,
    threads: Arc<ThreadRegistry>,
    pending_clients: Arc<PendingClients>,
) -> Result<(), String> {
    let acceptor_threads = threads.clone();
    threads.spawn("socks5-acceptor".to_string(), move || {
        let nested_threads = acceptor_threads;
        while !stopped.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((client, _)) => {
                    if let Err(error) = client.set_nonblocking(false) {
                        logs.push(log(
                            None,
                            "local".to_string(),
                            "accept_error",
                            0,
                            0,
                            Some("internal".to_string()),
                            Some(error.to_string()),
                        ));
                        continue;
                    }
                    let control = control.clone();
                    let registry = registry.clone();
                    let stopped = stopped.clone();
                    let logs = logs.clone();
                    let thread_logs = logs.clone();
                    let pending_client = match pending_clients.track(&client) {
                        Ok(pending) => pending,
                        Err(error) => {
                            logs.push(log(
                                None,
                                "local".to_string(),
                                "client_tracking_failed",
                                0,
                                0,
                                Some("internal".to_string()),
                                Some(error),
                            ));
                            let _ = client.shutdown(Shutdown::Both);
                            continue;
                        }
                    };
                    if let Err(error) =
                        nested_threads.spawn("socks5-stream".to_string(), move || {
                            handle_socks_client(
                                client,
                                control,
                                registry,
                                stopped,
                                logs,
                                pending_client,
                            );
                        })
                    {
                        thread_logs.push(log(
                            None,
                            "local".to_string(),
                            "worker_spawn_failed",
                            0,
                            0,
                            Some("internal".to_string()),
                            Some(error),
                        ));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(STOP_POLL);
                }
                Err(error) => {
                    logs.push(log(
                        None,
                        "local".to_string(),
                        "accept_error",
                        0,
                        0,
                        Some("internal".to_string()),
                        Some(error.to_string()),
                    ));
                    std::thread::sleep(STOP_POLL);
                }
            }
        }
    })
}

fn spawn_tunnel_acceptor(
    listener: TcpListener,
    active: Arc<Mutex<Option<ActiveListener>>>,
    stopped: Arc<AtomicBool>,
    pending_tunnels: Arc<PendingTunnels>,
    logs: LogBuffer,
    threads: Arc<ThreadRegistry>,
) -> Result<(), String> {
    threads.spawn("socks5-tunnel-acceptor".to_string(), move || {
        while !stopped.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((tunnel, _)) => {
                    if let Err(error) = tunnel.set_nonblocking(false) {
                        logs.push(log(
                            None,
                            "tunnel".to_string(),
                            "accept_error",
                            0,
                            0,
                            Some("internal".to_string()),
                            Some(error.to_string()),
                        ));
                        continue;
                    }
                    let listener = hlock(&active).clone();
                    let logs = logs.clone();
                    let pending = match pending_tunnels.track(&tunnel) {
                        Ok(pending) => pending,
                        Err(error) => {
                            logs.push(log(
                                None,
                                "tunnel".to_string(),
                                "tunnel_tracking_failed",
                                0,
                                0,
                                Some("internal".to_string()),
                                Some(error),
                            ));
                            let _ = tunnel.shutdown(Shutdown::Both);
                            continue;
                        }
                    };
                    let Some(listener) = listener else {
                        logs.push(log(
                            None,
                            "tunnel".to_string(),
                            "listener_stopped",
                            0,
                            0,
                            None,
                            Some(
                                "reverse tunnel arrived while the listener was stopped".to_string(),
                            ),
                        ));
                        let _ = tunnel.shutdown(Shutdown::Both);
                        continue;
                    };
                    if listener.stopped.load(Ordering::SeqCst) {
                        let _ = tunnel.shutdown(Shutdown::Both);
                        continue;
                    }
                    let handshake_threads = listener.threads.clone();
                    let handshake_logs = logs.clone();
                    if let Err(error) =
                        handshake_threads.spawn("socks5-tunnel-handshake".to_string(), move || {
                            if listener.stopped.load(Ordering::SeqCst) {
                                return;
                            }
                            match register_tunnel(tunnel, listener.registry, handshake_logs.clone())
                            {
                                Ok(()) => pending.finish(),
                                Err(error) => {
                                    handshake_logs.push(log(
                                        None,
                                        "tunnel".to_string(),
                                        "tunnel_rejected",
                                        0,
                                        0,
                                        Some("protocol".to_string()),
                                        Some(error),
                                    ));
                                }
                            }
                        })
                    {
                        logs.push(log(
                            None,
                            "tunnel".to_string(),
                            "worker_spawn_failed",
                            0,
                            0,
                            Some("internal".to_string()),
                            Some(error),
                        ));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(STOP_POLL);
                }
                Err(error) => {
                    logs.push(log(
                        None,
                        "tunnel".to_string(),
                        "accept_error",
                        0,
                        0,
                        Some("internal".to_string()),
                        Some(error.to_string()),
                    ));
                    std::thread::sleep(STOP_POLL);
                }
            }
        }
    })
}

fn handle_socks_client(
    mut client: TcpStream,
    control: Arc<Mutex<TcpStream>>,
    registry: Arc<Registry>,
    stopped: Arc<AtomicBool>,
    logs: LogBuffer,
    pending_client: PendingClient,
) {
    if client
        .set_read_timeout(Some(PREAUTH_READ_POLL))
        .and_then(|_| client.set_write_timeout(Some(SOCKS5_TARGET_CONNECT_TIMEOUT)))
        .is_err()
    {
        return;
    }
    let mut writer = match client.try_clone() {
        Ok(writer) => writer,
        Err(_) => return,
    };
    let mut reader = CancellableReader::new(
        &mut client,
        pending_client.cancelled.clone(),
        SOCKS5_TARGET_CONNECT_TIMEOUT,
    );
    if negotiate_noauth(&mut reader, &mut writer).is_err() {
        logs.push(log(
            None,
            "local".to_string(),
            "auth_rejected",
            0,
            0,
            Some("protocol".to_string()),
            Some("client did not offer NOAUTH".to_string()),
        ));
        return;
    }
    let request = match Socks5Request::read_from(&mut reader) {
        Ok(request) => request,
        Err(error) => {
            let _ = write_socks5_reply(&mut writer, SOCKS5_REP_GENERAL_FAILURE);
            logs.push(log(
                None,
                "local".to_string(),
                "request_rejected",
                0,
                0,
                Some("protocol".to_string()),
                Some(error.to_string()),
            ));
            return;
        }
    };
    drop(reader);
    if request.command != Socks5RequestCommand::Connect {
        let _ = write_socks5_reply(&mut writer, SOCKS5_REP_COMMAND_NOT_SUPPORTED);
        logs.push(log(
            None,
            format!("{}:{}", request.target, request.port),
            "command_rejected",
            0,
            0,
            Some("rejected".to_string()),
            Some("only CONNECT is supported".to_string()),
        ));
        return;
    }
    let target = match Socks5Target::new(&request.target.host(), request.port) {
        Ok(target) => target,
        Err(error) => {
            let _ = write_socks5_reply(&mut writer, SOCKS5_REP_GENERAL_FAILURE);
            logs.push(log(
                None,
                request.target.authority(request.port),
                "target_rejected",
                0,
                0,
                Some("protocol".to_string()),
                Some(error.to_string()),
            ));
            return;
        }
    };
    let (stream_id, state) = match registry.allocate(target.clone(), &client) {
        Ok(value) => value,
        Err(error) => {
            let _ = write_socks5_reply(&mut writer, SOCKS5_REP_GENERAL_FAILURE);
            logs.push(log(
                None,
                target.authority(),
                "stream_allocate_failed",
                0,
                0,
                Some("internal".to_string()),
                Some(error),
            ));
            return;
        }
    };
    pending_client.finish();

    if send_control(
        &control,
        &Socks5SidecarEvent::Open {
            stream_id,
            target: target.clone(),
        },
    )
    .is_err()
    {
        registry.remove(stream_id);
        let _ = write_socks5_reply(&mut writer, SOCKS5_REP_GENERAL_FAILURE);
        return;
    }

    let tunnel = match registry.wait_ready(&state, SOCKS5_TUNNEL_CONNECT_TIMEOUT) {
        Ok(tunnel) => tunnel,
        Err((category, detail)) => {
            let _ = write_socks5_reply(&mut writer, category.socks_reply());
            logs.push(log(
                Some(stream_id),
                target.authority(),
                "open_failed",
                0,
                0,
                Some(format!("{category:?}")),
                Some(detail),
            ));
            registry.remove(stream_id);
            return;
        }
    };

    if write_socks5_reply(&mut writer, 0).is_err() {
        registry.remove(stream_id);
        return;
    }
    let _ = client.set_read_timeout(None);
    let _ = client.set_write_timeout(None);
    let _ = tunnel.set_read_timeout(None);
    let _ = tunnel.set_write_timeout(None);
    logs.push(log(
        Some(stream_id),
        target.authority(),
        "connected",
        0,
        0,
        None,
        None,
    ));

    let (bytes_to_target, bytes_from_target) = pump_stream(
        &client,
        &tunnel,
        stream_id,
        stopped,
        registry.clone(),
        state.cancelled.clone(),
    );
    registry.log_closed(stream_id, bytes_to_target, bytes_from_target);
    registry.remove(stream_id);
}

fn register_tunnel(
    tunnel: TcpStream,
    registry: Arc<Registry>,
    logs: LogBuffer,
) -> Result<(), String> {
    tunnel
        .set_read_timeout(Some(SOCKS5_TUNNEL_CONNECT_TIMEOUT))
        .map_err(|error| error.to_string())?;
    let mut reader = tunnel.try_clone().map_err(|error| error.to_string())?;
    let frame = Socks5TunnelFrame::read_from(&mut reader).map_err(|error| error.to_string())?;
    let Socks5TunnelFrame::Hello { stream_id } = frame else {
        return Err("first tunnel frame was not hello".to_string());
    };
    registry
        .accept_tunnel(stream_id, tunnel)
        .map_err(str::to_string)?;
    logs.push(log(
        Some(stream_id),
        "tunnel".to_string(),
        "tunnel_ready",
        0,
        0,
        None,
        None,
    ));
    Ok(())
}

fn pump_stream(
    client: &TcpStream,
    tunnel: &TcpStream,
    stream_id: u64,
    stopped: Arc<AtomicBool>,
    registry: Arc<Registry>,
    cancelled: Arc<AtomicBool>,
) -> (u64, u64) {
    let mut client_reader = match client.try_clone() {
        Ok(stream) => stream,
        Err(_) => return (0, 0),
    };
    if client_reader
        .set_read_timeout(Some(DATA_PLANE_POLL))
        .is_err()
    {
        return (0, 0);
    }
    let client_to_shutdown = match client.try_clone() {
        Ok(stream) => stream,
        Err(_) => return (0, 0),
    };
    let mut client_from_tunnel = match client.try_clone() {
        Ok(stream) => stream,
        Err(_) => return (0, 0),
    };
    let mut tunnel_writer = match tunnel.try_clone() {
        Ok(stream) => stream,
        Err(_) => return (0, 0),
    };
    let tunnel_reader = match tunnel.try_clone() {
        Ok(stream) => stream,
        Err(_) => return (0, 0),
    };
    let stopped_to_tunnel = stopped.clone();
    let stopped_from_tunnel = stopped.clone();
    let registry_to_tunnel = registry.clone();
    let registry_from_tunnel = registry.clone();
    let cancelled_to_tunnel = cancelled.clone();
    let cancelled_from_tunnel = cancelled;

    let to_tunnel = std::thread::Builder::new()
        .name(format!("socks5-up-{stream_id}"))
        .spawn(move || {
            let mut bytes = 0u64;
            let mut buffer = vec![0u8; SOCKS5_TUNNEL_PUMP_CHUNK];
            loop {
                if stopped_to_tunnel.load(Ordering::SeqCst) {
                    break;
                }
                if client_reader
                    .set_read_timeout(Some(DATA_PLANE_POLL))
                    .is_err()
                {
                    break;
                }
                match client_reader.read(&mut buffer) {
                    Ok(0) => {
                        let _ = Socks5TunnelFrame::Fin.write_to(&mut tunnel_writer);
                        break;
                    }
                    Ok(count) => {
                        bytes += count as u64;
                        if Socks5TunnelFrame::Data(buffer[..count].to_vec())
                            .write_to(&mut tunnel_writer)
                            .is_err()
                        {
                            let _ = client_to_shutdown.shutdown(Shutdown::Both);
                            registry_to_tunnel.remove(stream_id);
                            break;
                        }
                    }
                    Err(error) if is_read_timeout(&error) => {
                        if is_cancelled(&stopped_to_tunnel, &cancelled_to_tunnel) {
                            let _ = client_to_shutdown.shutdown(Shutdown::Both);
                            registry_to_tunnel.remove(stream_id);
                            break;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(_) => {
                        let _ = Socks5TunnelFrame::Reset.write_to(&mut tunnel_writer);
                        let _ = client_to_shutdown.shutdown(Shutdown::Both);
                        registry_to_tunnel.remove(stream_id);
                        break;
                    }
                }
            }
            bytes
        });
    let from_tunnel = std::thread::Builder::new()
        .name(format!("socks5-down-{stream_id}"))
        .spawn(move || {
            let mut bytes = 0u64;
            let mut frames = match TunnelFrameReader::new(tunnel_reader) {
                Ok(reader) => reader,
                Err(_) => {
                    let _ = client_from_tunnel.shutdown(Shutdown::Both);
                    registry_from_tunnel.remove(stream_id);
                    return 0;
                }
            };
            loop {
                if stopped_from_tunnel.load(Ordering::SeqCst) {
                    break;
                }
                let frame =
                    match frames.read_or_cancel(&stopped_from_tunnel, &cancelled_from_tunnel) {
                        Ok(Some(frame)) => frame,
                        Ok(None) => {
                            let _ = client_from_tunnel.shutdown(Shutdown::Both);
                            registry_from_tunnel.remove(stream_id);
                            break;
                        }
                        Err(_) => {
                            let _ = client_from_tunnel.shutdown(Shutdown::Both);
                            registry_from_tunnel.remove(stream_id);
                            break;
                        }
                    };
                match frame {
                    Socks5TunnelFrame::Data(data) => {
                        if client_from_tunnel.write_all(&data).is_err() {
                            let _ = frames.shutdown();
                            registry_from_tunnel.remove(stream_id);
                            break;
                        }
                        bytes += data.len() as u64;
                    }
                    Socks5TunnelFrame::Fin => {
                        let _ = client_from_tunnel.shutdown(Shutdown::Write);
                        break;
                    }
                    Socks5TunnelFrame::Reset => {
                        let _ = client_from_tunnel.shutdown(Shutdown::Both);
                        let _ = frames.shutdown();
                        registry_from_tunnel.remove(stream_id);
                        break;
                    }
                    Socks5TunnelFrame::Hello { .. } => {}
                }
            }
            bytes
        });
    let (to_tunnel, from_tunnel) = match (to_tunnel, from_tunnel) {
        (Ok(to_tunnel), Ok(from_tunnel)) => (to_tunnel, from_tunnel),
        (to_tunnel, from_tunnel) => {
            let _ = client.shutdown(Shutdown::Both);
            let _ = tunnel.shutdown(Shutdown::Both);
            if let Ok(handle) = to_tunnel {
                let _ = handle.join();
            }
            if let Ok(handle) = from_tunnel {
                let _ = handle.join();
            }
            registry.logs.push(log(
                Some(stream_id),
                "stream".to_string(),
                "worker_spawn_failed",
                0,
                0,
                Some("internal".to_string()),
                Some("cannot create SOCKS5 data-plane workers".to_string()),
            ));
            return (0, 0);
        }
    };
    let to = to_tunnel.join().unwrap_or(0);
    let from = from_tunnel.join().unwrap_or(0);
    let _ = client.shutdown(Shutdown::Both);
    let _ = tunnel.shutdown(Shutdown::Both);
    (to, from)
}

struct TunnelFrameReader {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl TunnelFrameReader {
    fn new(stream: TcpStream) -> std::io::Result<Self> {
        stream.set_read_timeout(Some(DATA_PLANE_POLL))?;
        Ok(Self {
            stream,
            buffer: Vec::new(),
        })
    }

    fn read_or_cancel(
        &mut self,
        stopped: &AtomicBool,
        cancelled: &AtomicBool,
    ) -> std::io::Result<Option<Socks5TunnelFrame>> {
        loop {
            if self.buffer.len() >= 4 {
                let size = u32::from_be_bytes(
                    self.buffer[..4].try_into().expect("four-byte frame length"),
                ) as usize;
                if size > SOCKS5_MAX_TUNNEL_CHUNK + 1 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "SOCKS5 tunnel frame exceeds {} bytes",
                            SOCKS5_MAX_TUNNEL_CHUNK + 1
                        ),
                    ));
                }
                let total = 4 + size;
                if self.buffer.len() >= total {
                    let payload = self.buffer[4..total].to_vec();
                    self.buffer.drain(..total);
                    return Socks5TunnelFrame::from_payload(&payload)
                        .map(Some)
                        .map_err(|error| {
                            std::io::Error::new(std::io::ErrorKind::InvalidData, error)
                        });
                }
            }

            let mut chunk = [0u8; 4096];
            match self.stream.read(&mut chunk) {
                Ok(0) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "SOCKS5 tunnel closed",
                    ));
                }
                Ok(count) => self.buffer.extend_from_slice(&chunk[..count]),
                Err(error) if is_read_timeout(&error) => {
                    if is_cancelled(stopped, cancelled) {
                        return Ok(None);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }

    fn shutdown(&self) -> std::io::Result<()> {
        self.stream.shutdown(Shutdown::Both)
    }
}

#[cfg(test)]
fn read_or_cancel(
    reader: &mut TcpStream,
    buffer: &mut [u8],
    stopped: &AtomicBool,
    cancelled: &AtomicBool,
) -> std::io::Result<Option<usize>> {
    loop {
        match reader.read(buffer) {
            Ok(count) => return Ok(Some(count)),
            Err(error) if is_read_timeout(&error) => {
                if is_cancelled(stopped, cancelled) {
                    return Ok(None);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

fn is_read_timeout(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

fn is_cancelled(stopped: &AtomicBool, cancelled: &AtomicBool) -> bool {
    stopped.load(Ordering::SeqCst) || cancelled.load(Ordering::SeqCst)
}

fn log(
    stream_id: Option<u64>,
    target: String,
    status: &str,
    bytes_to_target: u64,
    bytes_from_target: u64,
    category: Option<String>,
    detail: Option<String>,
) -> LogEntry {
    LogEntry {
        time: chrono::Local::now().format("%H:%M:%S").to_string(),
        stream_id,
        target,
        status: status.to_string(),
        bytes_to_target,
        bytes_from_target,
        category,
        detail,
    }
}

fn initial_theme() -> egui::Theme {
    egui::Theme::Dark
}

fn next_theme(theme: egui::Theme) -> egui::Theme {
    if matches!(theme, egui::Theme::Dark) {
        egui::Theme::Light
    } else {
        egui::Theme::Dark
    }
}

fn toolbar_labels(theme: egui::Theme) -> [&'static str; 2] {
    [
        "Clear log",
        if matches!(theme, egui::Theme::Dark) {
            "Light"
        } else {
            "Dark"
        },
    ]
}

fn listener_status_text(state: ListenerState, reserved_port: u16) -> String {
    match state {
        ListenerState::Running => "Running".to_string(),
        ListenerState::Reserved => {
            format!("Stopped / Reserved (127.0.0.1:{reserved_port})")
        }
        ListenerState::Stopped => "Stopped".to_string(),
    }
}

struct Socks5App {
    runtime: Arc<Runtime>,
    close_requested: bool,
    theme_initialized: bool,
    port_input: String,
    listener_error: Option<String>,
}

impl Socks5App {
    fn new(runtime: Arc<Runtime>) -> Self {
        Self {
            port_input: runtime.default_port().to_string(),
            runtime,
            close_requested: false,
            theme_initialized: false,
            listener_error: None,
        }
    }
}

impl App for Socks5App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.theme_initialized {
            ctx.set_theme(initial_theme());
            self.theme_initialized = true;
        }
        if self.runtime.stopped() && !self.close_requested {
            self.close_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Reverse SOCKS5 Proxy");
            let (client, chain, transport) = self.runtime.args();
            let listener_address = self.runtime.listener_address();
            let listener_state = self.runtime.listener_state();
            egui::Grid::new("socks5-summary")
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.strong("Client");
                    ui.label(client);
                    ui.end_row();
                    ui.strong("Transport");
                    ui.label(transport);
                    ui.end_row();
                    ui.strong("Proxy chain");
                    ui.label(chain);
                    ui.end_row();
                    ui.strong("SOCKS5 listener");
                    match (listener_state, listener_address.as_deref()) {
                        (ListenerState::Running, Some(address)) => {
                            ui.add(egui::Label::new(address).selectable(true));
                        }
                        (state, _) => {
                            ui.strong(listener_status_text(state, self.runtime.default_port()));
                        }
                    }
                    ui.end_row();
                    ui.strong("Reverse tunnel port");
                    ui.label(self.runtime.tunnel_port().to_string());
                    ui.end_row();
                    ui.strong("Active streams");
                    ui.label(format!(
                        "{} / {}",
                        self.runtime.active_streams(),
                        SOCKS5_MAX_CONCURRENCY
                    ));
                    ui.end_row();
                });

            ui.separator();
            ui.horizontal(|ui| {
                if listener_state == ListenerState::Running {
                    if ui
                        .button("Stop")
                        .on_hover_text("Stop the listener and close every active SOCKS5 stream")
                        .clicked()
                    {
                        match self.runtime.stop_listener() {
                            Ok(()) => self.listener_error = None,
                            Err(error) => self.listener_error = Some(error),
                        }
                    }
                } else {
                    ui.label("Port");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.port_input)
                            .char_limit(5)
                            .desired_width(64.0),
                    );
                    let parsed_port = parse_listener_port(&self.port_input);
                    if ui
                        .button("Start")
                        .on_hover_text("Start the loopback SOCKS5 listener on the selected port")
                        .clicked()
                    {
                        match parsed_port {
                            Ok(port) => match self.runtime.start_listener(port) {
                                Ok(_) => {
                                    self.port_input = port.to_string();
                                    self.listener_error = None;
                                }
                                Err(error) => self.listener_error = Some(error),
                            },
                            Err(error) => self.listener_error = Some(error),
                        }
                    }
                }
                let theme = ui.ctx().theme();
                let [clear_label, theme_label] = toolbar_labels(theme);
                if ui.button(clear_label).clicked() {
                    self.runtime.clear_logs();
                }
                if ui
                    .button(theme_label)
                    .on_hover_text("Toggle light/dark theme")
                    .clicked()
                {
                    ui.ctx().set_theme(next_theme(theme));
                }
                ui.label("Loopback only / NOAUTH / CONNECT only");
            });
            if let Some(error) = self.listener_error.as_deref() {
                let error_color = ui.visuals().error_fg_color;
                ui.colored_label(error_color, error);
            }
            ui.separator();

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .max_height(ui.available_height() - 8.0)
                .show(ui, |ui| {
                    egui::Grid::new("socks5-log-grid")
                        .striped(true)
                        .min_col_width(72.0)
                        .show(ui, |ui| {
                            for heading in [
                                "Time",
                                "Stream",
                                "Target",
                                "Status",
                                "To target",
                                "From target",
                                "Error",
                            ] {
                                ui.strong(heading);
                            }
                            ui.end_row();
                            for entry in self.runtime.logs() {
                                ui.monospace(&entry.time);
                                ui.monospace(
                                    entry
                                        .stream_id
                                        .map(|id| id.to_string())
                                        .unwrap_or_else(|| "-".to_string()),
                                );
                                ui.label(&entry.target);
                                ui.label(&entry.status);
                                ui.monospace(entry.bytes_to_target.to_string());
                                ui.monospace(entry.bytes_from_target.to_string());
                                let error = entry
                                    .detail
                                    .as_deref()
                                    .or(entry.category.as_deref())
                                    .unwrap_or("");
                                ui.label(error);
                                ui.end_row();
                            }
                        });
                });
        });
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

fn parse_listener_port(input: &str) -> Result<u16, String> {
    let port = input
        .trim()
        .parse::<u16>()
        .map_err(|_| "SOCKS5 listener port must be a number between 1 and 65535".to_string())?;
    if port == 0 {
        return Err("SOCKS5 listener port must be between 1 and 65535".to_string());
    }
    Ok(port)
}

fn hlock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOCKS5_CMD_BIND_TEST: u8 = 0x02;
    const SOCKS5_CMD_UDP_ASSOCIATE_TEST: u8 = 0x03;
    static RUNTIME_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn start_test_runtime_without_listener(
        label: &str,
    ) -> (Runtime, TcpStream, Socks5SidecarReady) {
        let control_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let control_port = control_listener.local_addr().unwrap().port();
        let runtime = Runtime::start(Arguments {
            control_port,
            session_id: format!("test-session-{label}"),
            client_label: "127.0.0.1:1".to_string(),
            proxy_chain: "Direct".to_string(),
            transport: "TCP".to_string(),
        })
        .unwrap();
        let (mut control, _) = control_listener.accept().unwrap();
        control
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let ready: Socks5SidecarEvent =
            serde_json::from_slice(&read_frame(&mut control).unwrap()).unwrap();
        let Socks5SidecarEvent::Ready(ready) = ready else {
            panic!("expected ready");
        };
        (runtime, control, ready)
    }

    fn start_test_runtime(label: &str) -> (Runtime, TcpStream, Socks5SidecarReady) {
        let (runtime, control, ready) = start_test_runtime_without_listener(label);
        runtime
            .start_listener(runtime.default_port())
            .expect("start test listener");
        (runtime, control, ready)
    }

    fn closed_thread_registry() -> Arc<ThreadRegistry> {
        let threads = Arc::new(ThreadRegistry::new());
        threads.join_all(Duration::ZERO).unwrap();
        threads
    }

    #[test]
    fn listener_waits_for_an_explicit_start_and_can_restart() {
        let _guard = hlock(&RUNTIME_TEST_LOCK);
        let (runtime, mut control, ready) = start_test_runtime_without_listener("explicit-start");
        assert!(ready.socks_address.is_empty());
        assert!(runtime.listener_address().is_none());
        assert_eq!(runtime.listener_state(), ListenerState::Reserved);
        assert!(
            TcpListener::bind(("127.0.0.1", runtime.default_port())).is_err(),
            "the initial random port must remain reserved before Start"
        );
        assert!(
            TcpStream::connect_timeout(
                &format!("127.0.0.1:{}", runtime.default_port())
                    .parse()
                    .unwrap(),
                Duration::from_millis(500),
            )
            .is_err(),
            "the bound-only reservation must not accept TCP connections before Start"
        );

        let port = runtime.default_port();
        let address = runtime.start_listener(port).unwrap();
        assert_eq!(address, format!("127.0.0.1:{port}"));
        assert_eq!(runtime.listener_state(), ListenerState::Running);
        runtime.stop_listener().unwrap();
        assert!(runtime.listener_address().is_none());
        assert_eq!(runtime.listener_state(), ListenerState::Stopped);

        control
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        assert!(
            read_frame(&mut control).is_err(),
            "stopping the listener must not stop the persistent sidecar control session"
        );

        let restarted = runtime.start_listener(port).unwrap();
        assert_eq!(restarted, format!("127.0.0.1:{port}"));
        runtime.request_stop("explicit-start complete");
        assert!(runtime.stopped());
        assert_eq!(runtime.active_streams(), 0);
    }

    #[test]
    fn occupied_port_error_is_retryable_without_stopping_control() {
        let _guard = hlock(&RUNTIME_TEST_LOCK);
        let (runtime, mut control, _ready) = start_test_runtime_without_listener("occupied-port");
        let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
        let occupied_port = occupied.local_addr().unwrap().port();
        assert_ne!(occupied_port, runtime.default_port());
        let error = runtime.start_listener(occupied_port).unwrap_err();
        assert!(error.contains(&occupied_port.to_string()));
        assert!(runtime.listener_address().is_none());
        assert_eq!(runtime.listener_state(), ListenerState::Stopped);

        drop(occupied);
        control
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        assert!(
            read_frame(&mut control).is_err(),
            "a bind failure must leave the sidecar control channel open"
        );
        let address = runtime.start_listener(occupied_port).unwrap();
        assert_eq!(address, format!("127.0.0.1:{occupied_port}"));
        runtime.request_stop("occupied-port complete");
        assert!(runtime.stopped());
    }

    #[test]
    fn clearing_logs_does_not_change_listener_state() {
        let _guard = hlock(&RUNTIME_TEST_LOCK);
        let (runtime, _control, _ready) = start_test_runtime("clear-log");
        let address = runtime.listener_address().unwrap();
        runtime.clear_logs();
        assert_eq!(
            runtime.listener_address().as_deref(),
            Some(address.as_str())
        );
        assert_eq!(runtime.listener_state(), ListenerState::Running);
        runtime.request_stop("clear-log complete");
    }

    #[test]
    fn listener_worker_spawn_failure_does_not_publish_a_running_listener() {
        let _guard = hlock(&RUNTIME_TEST_LOCK);
        let (runtime, _control, _ready) = start_test_runtime_without_listener("spawn-failure");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let error = ListenerRuntime::start_with_threads(
            listener,
            runtime.control.clone(),
            runtime.active.clone(),
            runtime.stopped.clone(),
            runtime.logs.clone(),
            runtime.next_stream_id.clone(),
            runtime.pending_tunnels.clone(),
            Arc::new(PendingClients::default()),
            closed_thread_registry(),
        )
        .err()
        .expect("a closed thread registry must fail listener startup");
        assert!(error.contains("thread registry is closed"));
        assert!(hlock(&runtime.active).is_none());
        assert!(runtime.listener_address().is_none());
        TcpListener::bind(address).expect("failed listener startup must release the socket");
        runtime.request_stop("spawn-failure complete");
    }

    #[test]
    fn listener_stop_cancels_pending_and_active_streams_without_waiting_for_handshake_timeout() {
        let stopped = Arc::new(AtomicBool::new(false));
        let registry = Registry::new(stopped, LogBuffer::new());
        let client_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let pending_client = TcpStream::connect(client_listener.local_addr().unwrap()).unwrap();
        let (_pending_peer, _) = client_listener.accept().unwrap();
        let target = Socks5Target::new("127.0.0.1", 80).unwrap();
        let (_pending_id, pending_state) =
            registry.allocate(target.clone(), &pending_client).unwrap();

        let active_client = TcpStream::connect(client_listener.local_addr().unwrap()).unwrap();
        let (_active_peer, _) = client_listener.accept().unwrap();
        let (active_id, _active_state) = registry.allocate(target, &active_client).unwrap();
        let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let active_tunnel = TcpStream::connect(tunnel_listener.local_addr().unwrap()).unwrap();
        let (active_tunnel_peer, _) = tunnel_listener.accept().unwrap();
        registry
            .accept_tunnel(active_id, active_tunnel_peer)
            .unwrap();

        let started = Instant::now();
        registry.cancel_all();
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "cancelling pending and active streams must be bounded"
        );
        assert_eq!(registry.active.load(Ordering::Relaxed), 0);
        assert_eq!(
            registry
                .wait_ready(&pending_state, SOCKS5_TUNNEL_CONNECT_TIMEOUT)
                .unwrap_err()
                .0,
            Socks5FailureCategory::Closed
        );
        drop(active_tunnel);
    }

    #[test]
    fn pending_clients_reject_registration_after_stop() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let _client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        let pending = Arc::new(PendingClients::default());
        let tracked = pending.track(&server).unwrap();
        pending.close_all();
        drop(tracked);

        let error = pending
            .track(&server)
            .err()
            .expect("a stopping listener must reject new pending-client registration");
        assert!(error.contains("stopping"));
        assert!(hlock(&pending.state).closed);
        assert!(hlock(&pending.state).sockets.is_empty());
    }

    #[test]
    fn pending_client_close_wakes_a_blocking_greeting_reader() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let _client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        server.set_read_timeout(Some(PREAUTH_READ_POLL)).unwrap();
        let pending = Arc::new(PendingClients::default());
        let tracked = pending.track(&server).unwrap();
        let cancelled = tracked.cancelled.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut byte = [0u8; 1];
            let mut reader = CancellableReader::new(&mut server, cancelled, Duration::from_secs(5));
            let _ = sender.send(reader.read(&mut byte));
        });
        std::thread::sleep(Duration::from_millis(50));
        pending.close_all();
        let result = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("stopping the listener must wake a pre-greeting read");
        match result {
            Ok(0) => {}
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionAborted => {}
            other => panic!("unexpected pre-greeting read result: {other:?}"),
        }
        reader.join().unwrap();
        drop(tracked);
    }

    #[test]
    fn listener_stop_cancels_a_client_blocked_before_greeting() {
        let _guard = hlock(&RUNTIME_TEST_LOCK);
        let (runtime, mut control, _ready) = start_test_runtime("preauth-stop");
        let address = runtime.listener_address().unwrap();
        let port = runtime.default_port();
        let mut client = TcpStream::connect(&address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));

        let started = Instant::now();
        runtime
            .stop_listener()
            .expect("Stop must cancel a client blocked before the SOCKS5 greeting");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "pre-greeting cancellation took {:?}",
            started.elapsed()
        );
        assert_eq!(runtime.listener_state(), ListenerState::Stopped);
        let mut byte = [0u8; 1];
        let _ = client.read(&mut byte);
        drop(client);
        let released = TcpListener::bind(&address)
            .expect("Stop must release the pre-authentication listener port");
        drop(released);

        control
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        assert!(
            read_frame(&mut control).is_err(),
            "Stop must preserve the sidecar control channel"
        );

        let restarted = runtime.start_listener(port).unwrap();
        assert_eq!(restarted, address);
        assert_eq!(runtime.listener_state(), ListenerState::Running);
        assert_eq!(runtime.active_streams(), 0);
        let mut restarted_client = TcpStream::connect(&restarted).unwrap();
        restarted_client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        restarted_client.write_all(&[5, 1, 0]).unwrap();
        let mut greeting = [0u8; 2];
        restarted_client.read_exact(&mut greeting).unwrap();
        assert_eq!(greeting, [5, 0]);
        runtime.request_stop("preauth stop complete");
    }

    #[test]
    fn active_tunnel_cancellation_exits_the_polling_reader() {
        let stopped = Arc::new(AtomicBool::new(false));
        let registry = Registry::new(stopped.clone(), LogBuffer::new());
        let client_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(client_listener.local_addr().unwrap()).unwrap();
        let (_client_peer, _) = client_listener.accept().unwrap();
        let target = Socks5Target::new("127.0.0.1", 80).unwrap();
        let (stream_id, state) = registry.allocate(target, &client).unwrap();
        let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let tunnel_peer = TcpStream::connect(tunnel_listener.local_addr().unwrap()).unwrap();
        let (tunnel_server, _) = tunnel_listener.accept().unwrap();
        registry.accept_tunnel(stream_id, tunnel_server).unwrap();
        let tunnel = registry
            .wait_ready(&state, SOCKS5_TUNNEL_CONNECT_TIMEOUT)
            .unwrap();
        let mut reader = tunnel.try_clone().unwrap();
        reader.set_read_timeout(Some(DATA_PLANE_POLL)).unwrap();
        let cancelled = state.cancelled.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut byte = [0u8; 1];
            let result = read_or_cancel(&mut reader, &mut byte, &stopped, &cancelled);
            let _ = sender.send(result);
        });
        std::thread::sleep(Duration::from_millis(100));
        registry.cancel_all();
        let result = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("cancelling an active stream must wake a blocked tunnel reader");
        assert!(matches!(result.unwrap(), None | Some(0)));
        drop(tunnel_peer);
    }

    #[test]
    fn per_stream_cancellation_stops_idle_data_plane_within_a_bound() {
        let stopped = Arc::new(AtomicBool::new(false));
        let registry = Arc::new(Registry::new(stopped.clone(), LogBuffer::new()));
        let client_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(client_listener.local_addr().unwrap()).unwrap();
        let (_client_peer, _) = client_listener.accept().unwrap();
        let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let tunnel = TcpStream::connect(tunnel_listener.local_addr().unwrap()).unwrap();
        let (_tunnel_peer, _) = tunnel_listener.accept().unwrap();
        let target = Socks5Target::new("127.0.0.1", 80).unwrap();
        let (stream_id, state) = registry.allocate(target, &client).unwrap();
        let cancelled = state.cancelled.clone();
        let cleanup_registry = registry.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let result = pump_stream(&client, &tunnel, stream_id, stopped, registry, cancelled);
            let _ = sender.send(result);
        });

        std::thread::sleep(Duration::from_millis(100));
        let cancelled_at = Instant::now();
        state.cancelled.store(true, Ordering::SeqCst);
        let result = receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("cancelling an idle data plane must not wait for a peer");
        assert!(cancelled_at.elapsed() < Duration::from_secs(2));
        assert_eq!(result, (0, 0));
        worker.join().unwrap();
        cleanup_registry.cancel_all();
    }

    #[test]
    fn stream_ids_remain_unique_across_listener_restarts() {
        let next_id = Arc::new(AtomicU64::new(1));
        let client_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(client_listener.local_addr().unwrap()).unwrap();
        let (_client_peer, _) = client_listener.accept().unwrap();
        let target = Socks5Target::new("127.0.0.1", 80).unwrap();
        let first = Registry::new_with_id(
            Arc::new(AtomicBool::new(false)),
            LogBuffer::new(),
            next_id.clone(),
        );
        let (first_id, _) = first.allocate(target.clone(), &client).unwrap();
        first.cancel_all();
        let second =
            Registry::new_with_id(Arc::new(AtomicBool::new(false)), LogBuffer::new(), next_id);
        let (second_id, _) = second.allocate(target, &client).unwrap();
        assert!(second_id > first_id);
    }

    #[test]
    fn theme_and_toolbar_helpers_keep_the_ftp_default_and_button_order() {
        assert_eq!(initial_theme(), egui::Theme::Dark);
        assert_eq!(toolbar_labels(egui::Theme::Dark), ["Clear log", "Light"]);
        assert_eq!(toolbar_labels(egui::Theme::Light), ["Clear log", "Dark"]);
        assert_eq!(next_theme(egui::Theme::Dark), egui::Theme::Light);
        assert_eq!(next_theme(egui::Theme::Light), egui::Theme::Dark);
        assert_eq!(
            listener_status_text(ListenerState::Reserved, 43210),
            "Stopped / Reserved (127.0.0.1:43210)"
        );
        assert_eq!(
            listener_status_text(ListenerState::Stopped, 43210),
            "Stopped"
        );
        assert_eq!(
            listener_status_text(ListenerState::Running, 43210),
            "Running"
        );
    }

    #[test]
    fn thread_registry_returns_spawn_errors_after_sealing() {
        let threads = closed_thread_registry();
        let error = threads
            .spawn("must-not-start".to_string(), || {})
            .unwrap_err();
        assert!(error.contains("thread registry is closed"));
    }

    #[test]
    fn listener_port_input_accepts_only_nonzero_u16_ports() {
        assert_eq!(parse_listener_port("1"), Ok(1));
        assert_eq!(parse_listener_port(" 65535 "), Ok(65535));
        assert!(parse_listener_port("0").is_err());
        assert!(parse_listener_port("65536").is_err());
        assert!(parse_listener_port("").is_err());
        assert!(parse_listener_port("abc").is_err());
    }

    #[test]
    fn log_buffer_is_bounded() {
        let logs = LogBuffer::new();
        for index in 0..800 {
            logs.push(log(
                Some(index),
                "example.com:443".to_string(),
                "connected",
                1,
                2,
                None,
                Some("x".repeat(2048)),
            ));
        }
        assert!(logs.snapshot().len() <= LOG_LIMIT);
        assert!(
            logs.snapshot()
                .iter()
                .map(|entry| {
                    entry.target.len()
                        + entry.status.len()
                        + entry.detail.as_deref().unwrap_or("").len()
                        + 96
                })
                .sum::<usize>()
                <= LOG_BYTES_LIMIT
        );
    }

    #[test]
    fn control_frame_reader_reassembles_partial_frames() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut writer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let mut reader = ControlFrameReader::new(socket);

        writer.write_all(&[0, 0, 0, 3, 1]).unwrap();
        assert_eq!(reader.try_read_frame().unwrap(), None);
        writer.write_all(&[2, 3]).unwrap();
        assert_eq!(reader.try_read_frame().unwrap(), Some(vec![1, 2, 3]));
    }

    #[test]
    fn registry_rejects_after_limit_and_duplicate_hello_is_safe() {
        let stopped = Arc::new(AtomicBool::new(false));
        let registry = Registry::new(stopped, LogBuffer::new());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (_peer, _) = listener.accept().unwrap();
        let target = Socks5Target::new("127.0.0.1", 80).unwrap();
        let (id, _) = registry.allocate(target.clone(), &client).unwrap();
        let tunnel_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let tunnel = TcpStream::connect(tunnel_listener.local_addr().unwrap()).unwrap();
        let (tunnel_peer, _) = tunnel_listener.accept().unwrap();
        registry.accept_tunnel(id, tunnel_peer).unwrap();
        let duplicate = TcpStream::connect(tunnel_listener.local_addr().unwrap()).unwrap();
        let (duplicate_peer, _) = tunnel_listener.accept().unwrap();
        assert!(registry.accept_tunnel(id, duplicate_peer).is_err());
        drop((tunnel, duplicate));
        assert_eq!(registry.active.load(Ordering::Relaxed), 1);
        registry.remove(id);
        assert_eq!(registry.active.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn registry_enforces_the_configured_concurrency_limit() {
        let registry = Registry::new(Arc::new(AtomicBool::new(false)), LogBuffer::new());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (_peer, _) = listener.accept().unwrap();
        let target = Socks5Target::new("127.0.0.1", 80).unwrap();
        let mut ids = Vec::new();
        for _ in 0..SOCKS5_MAX_CONCURRENCY {
            ids.push(registry.allocate(target.clone(), &client).unwrap().0);
        }
        assert!(registry.allocate(target, &client).is_err());
        for id in ids {
            registry.remove(id);
        }
        assert_eq!(registry.active.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn repeated_registry_cleanup_leaves_no_active_streams() {
        let registry = Registry::new(Arc::new(AtomicBool::new(false)), LogBuffer::new());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (_peer, _) = listener.accept().unwrap();
        let target = Socks5Target::new("127.0.0.1", 80).unwrap();
        for _ in 0..100 {
            let (id, _) = registry.allocate(target.clone(), &client).unwrap();
            registry.remove(id);
        }
        assert_eq!(registry.active.load(Ordering::Relaxed), 0);
        assert!(hlock(&registry.streams).is_empty());
    }

    #[test]
    fn repeated_runtime_start_and_close_reclaims_threads_sockets_and_listeners() {
        let _guard = hlock(&RUNTIME_TEST_LOCK);
        let mut addresses: Vec<std::net::SocketAddr> = Vec::new();
        for index in 0..100 {
            let (runtime, control, ready) = start_test_runtime(&format!("lifecycle-{index}"));
            assert_eq!(runtime.active_streams(), 0);
            assert!(ready.socks_address.is_empty());
            assert!(ready.tunnel_port != 0);
            addresses.push(runtime.listener_address().unwrap().parse().unwrap());
            addresses.push(std::net::SocketAddr::from((
                [127, 0, 0, 1],
                ready.tunnel_port,
            )));
            runtime.request_stop("lifecycle test");
            let close_started = Instant::now();
            runtime.close();
            assert!(
                close_started.elapsed() < Duration::from_secs(5),
                "runtime close took {:?}",
                close_started.elapsed()
            );
            assert_eq!(runtime.active_streams(), 0);
            assert!(runtime.stopped());
            drop(control);
        }
        for address in addresses {
            let listener = TcpListener::bind(address).unwrap_or_else(|error| {
                panic!("listener at {address} was not released after runtime close: {error}")
            });
            drop(listener);
        }
    }

    #[test]
    fn runtime_rejects_bind_and_udp_associate_with_command_not_supported() {
        let _guard = hlock(&RUNTIME_TEST_LOCK);
        let (runtime, _control, _ready) = start_test_runtime("command-rejection");
        for command in [SOCKS5_CMD_BIND_TEST, SOCKS5_CMD_UDP_ASSOCIATE_TEST] {
            let mut client = TcpStream::connect(runtime.listener_address().unwrap()).unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            client.write_all(&[5, 1, 0]).unwrap();
            let mut greeting = [0u8; 2];
            client.read_exact(&mut greeting).unwrap();
            assert_eq!(greeting, [5, 0]);
            client
                .write_all(&[5, command, 0, 1, 127, 0, 0, 1, 0, 80])
                .unwrap();
            let mut reply = [0u8; 10];
            client.read_exact(&mut reply).unwrap();
            assert_eq!(reply[0], 5);
            assert_eq!(reply[1], 7, "command {command} must be rejected");
        }
        runtime.request_stop("command rejection complete");
        runtime.close();
        assert_eq!(runtime.active_streams(), 0);
    }

    #[test]
    fn runtime_socks_stream_carries_bidirectional_data_and_half_close() {
        let _guard = hlock(&RUNTIME_TEST_LOCK);
        let (runtime, mut control, ready) = start_test_runtime("data");
        assert_eq!(ready.session_id, "test-session-data");

        let mut client = TcpStream::connect(runtime.listener_address().unwrap()).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        client
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        client.write_all(&[5, 1, 0]).unwrap();
        let mut greeting = [0u8; 2];
        client.read_exact(&mut greeting).unwrap();
        assert_eq!(greeting, [5, 0]);
        client
            .write_all(&[
                5, 1, 0, 3, 11, b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c', b'o', b'm',
                0, 80,
            ])
            .unwrap();

        let open: Socks5SidecarEvent =
            serde_json::from_slice(&read_frame(&mut control).unwrap()).unwrap();
        let Socks5SidecarEvent::Open { stream_id, .. } = open else {
            panic!("expected open");
        };
        let mut tunnel = TcpStream::connect(("127.0.0.1", ready.tunnel_port)).unwrap();
        Socks5TunnelFrame::Hello { stream_id }
            .write_to(&mut tunnel)
            .unwrap();
        let mut reply = [0u8; 10];
        client.read_exact(&mut reply).unwrap();
        assert_eq!(reply[1], 0);

        client.write_all(b"ping").unwrap();
        assert_eq!(
            Socks5TunnelFrame::read_from(&mut tunnel).unwrap(),
            Socks5TunnelFrame::Data(b"ping".to_vec())
        );
        Socks5TunnelFrame::Data(b"pong".to_vec())
            .write_to(&mut tunnel)
            .unwrap();
        let mut pong = [0u8; 4];
        client.read_exact(&mut pong).unwrap();
        assert_eq!(&pong, b"pong");

        client.shutdown(Shutdown::Write).unwrap();
        assert_eq!(
            Socks5TunnelFrame::read_from(&mut tunnel).unwrap(),
            Socks5TunnelFrame::Fin
        );
        tunnel.shutdown(Shutdown::Write).unwrap();
        let mut eof = [0u8; 1];
        assert_eq!(client.read(&mut eof).unwrap(), 0);

        runtime.request_stop("test complete");
        runtime.close();
        assert_eq!(runtime.active_streams(), 0);
    }
}
