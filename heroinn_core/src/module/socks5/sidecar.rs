use std::collections::VecDeque;
use std::fmt;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use heroinn_util::protocol::frame::read_frame;
use heroinn_util::socks5::{Socks5SidecarEvent, Socks5SidecarReady};

const ACCEPT_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
const STDERR_LIMIT_BYTES: usize = 8 * 1024;
const STDERR_LIMIT_LINES: usize = 8;

#[derive(Debug)]
pub enum SidecarProcessError {
    NotFound(Vec<PathBuf>),
    NotExecutable(PathBuf),
    Spawn(String),
    Handshake(String),
    Protocol(String),
}

impl fmt::Display for SidecarProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(paths) => write!(
                formatter,
                "socks5_sidecar_not_found: {}",
                paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::NotExecutable(path) => {
                write!(
                    formatter,
                    "socks5_sidecar_not_executable: {}",
                    path.display()
                )
            }
            Self::Spawn(detail) => write!(formatter, "socks5_sidecar_spawn_failed: {detail}"),
            Self::Handshake(detail) => {
                write!(formatter, "socks5_sidecar_handshake_failed: {detail}")
            }
            Self::Protocol(detail) => write!(formatter, "socks5_sidecar_protocol_error: {detail}"),
        }
    }
}

impl std::error::Error for SidecarProcessError {}

pub fn sidecar_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "heroinn_socks5proxy.exe"
    } else {
        "heroinn_socks5proxy"
    }
}

fn sidecar_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        dirs.push(exe_dir);
    }
    if let Ok(cwd) = std::env::current_dir()
        && !dirs.iter().any(|path| path == &cwd)
    {
        dirs.push(cwd);
    }
    dirs
}

fn locate_in(dirs: &[PathBuf], name: &str) -> std::result::Result<PathBuf, SidecarProcessError> {
    let candidates = dirs.iter().map(|dir| dir.join(name)).collect::<Vec<_>>();
    for candidate in &candidates {
        let metadata = match std::fs::metadata(candidate) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(SidecarProcessError::NotExecutable(candidate.clone())),
        };
        if !metadata.is_file() {
            return Err(SidecarProcessError::NotExecutable(candidate.clone()));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 == 0 {
                return Err(SidecarProcessError::NotExecutable(candidate.clone()));
            }
        }
        return Ok(candidate.clone());
    }
    Err(SidecarProcessError::NotFound(candidates))
}

pub fn locate_sidecar() -> std::result::Result<PathBuf, SidecarProcessError> {
    locate_in(&sidecar_dirs(), sidecar_binary_name())
}

pub fn sidecar_available() -> bool {
    locate_sidecar().is_ok()
}

struct ProcessGuard {
    pid: u32,
    child: Arc<Mutex<Child>>,
    alive: Arc<AtomicBool>,
    monitor: Option<thread::JoinHandle<()>>,
    stderr: Arc<Mutex<VecDeque<String>>>,
    stderr_reader: Option<thread::JoinHandle<()>>,
}

impl ProcessGuard {
    fn new(child: Child, stderr: std::process::ChildStderr) -> Self {
        let pid = child.id();
        let child = Arc::new(Mutex::new(child));
        let alive = Arc::new(AtomicBool::new(true));
        let monitor_child = child.clone();
        let monitor_alive = alive.clone();
        let monitor = thread::Builder::new()
            .name(format!("socks5-sidecar-wait-{pid}"))
            .spawn(move || {
                loop {
                    let result = {
                        let mut child = hlock(&monitor_child);
                        child.try_wait()
                    };
                    match result {
                        Ok(Some(status)) => {
                            log::info!("SOCKS5 sidecar pid={pid} exited: {status}");
                            break;
                        }
                        Ok(None) => thread::sleep(Duration::from_millis(100)),
                        Err(error) => {
                            log::warn!("SOCKS5 sidecar pid={pid} wait failed: {error}");
                            break;
                        }
                    }
                }
                monitor_alive.store(false, Ordering::SeqCst);
            })
            .ok();

        let stderr_buffer = Arc::new(Mutex::new(VecDeque::new()));
        let stderr_output = stderr_buffer.clone();
        let stderr_reader = thread::Builder::new()
            .name(format!("socks5-sidecar-stderr-{pid}"))
            .spawn(move || drain_stderr(stderr, stderr_output))
            .ok();

        Self {
            pid,
            child,
            alive,
            monitor,
            stderr: stderr_buffer,
            stderr_reader,
        }
    }

    fn alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    fn close(&mut self) {
        if let Ok(stderr) = self.stderr.lock()
            && !stderr.is_empty()
        {
            log::debug!(
                "SOCKS5 sidecar pid={} stderr tail: {}",
                self.pid,
                stderr
                    .iter()
                    .map(|line| heroinn_util::socks5::sanitize_detail(line))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }
        if !self.alive.load(Ordering::SeqCst) {
            self.join();
            return;
        }
        kill_tree(self.pid);
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.alive.load(Ordering::SeqCst) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(25));
        }
        if self.alive.load(Ordering::SeqCst) {
            kill_tree(self.pid);
            let deadline = Instant::now() + Duration::from_secs(2);
            while self.alive.load(Ordering::SeqCst) && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(25));
            }
        }
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.try_wait();
        }
        self.join();
    }

    fn join(&mut self) {
        if let Some(handle) = self.monitor.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.stderr_reader.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct SidecarProcess {
    guard: ProcessGuard,
}

impl SidecarProcess {
    pub fn alive(&self) -> bool {
        self.guard.alive()
    }

    pub fn close(&mut self) {
        self.guard.close();
    }
}

pub struct SidecarInstance {
    control: Arc<Mutex<TcpStream>>,
    ready: Socks5SidecarReady,
    closed: Arc<AtomicBool>,
    process: SidecarProcess,
}

impl SidecarInstance {
    pub fn new(
        driver_path: &Path,
        session_id: &str,
        peer_label: &str,
        proxy_chain: &str,
        transport: &str,
    ) -> std::result::Result<Self, SidecarProcessError> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|error| SidecarProcessError::Spawn(error.to_string()))?;
        let control_port = listener
            .local_addr()
            .map_err(|error| SidecarProcessError::Spawn(error.to_string()))?
            .port();

        let mut command = Command::new(driver_path);
        command
            .arg("--control-port")
            .arg(control_port.to_string())
            .arg("--session-id")
            .arg(session_id)
            .arg("--client-label")
            .arg(peer_label)
            .arg("--proxy-chain")
            .arg(proxy_chain)
            .arg("--transport")
            .arg(transport)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
            command.creation_flags(CREATE_NEW_PROCESS_GROUP);
        }

        let mut child = command.spawn().map_err(|error| {
            SidecarProcessError::Spawn(format!("{}: {error}", driver_path.display()))
        })?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| SidecarProcessError::Spawn("stderr pipe unavailable".to_string()))?;

        let socket = match accept_control(&listener, &mut child) {
            Ok(socket) => socket,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        socket
            .set_read_timeout(Some(ACCEPT_TIMEOUT))
            .and_then(|_| socket.set_write_timeout(Some(ACCEPT_TIMEOUT)))
            .map_err(|error| SidecarProcessError::Handshake(error.to_string()))?;

        let mut reader = socket
            .try_clone()
            .map_err(|error| SidecarProcessError::Handshake(error.to_string()))?;
        let payload = read_frame(&mut reader)
            .map_err(|error| SidecarProcessError::Handshake(error.to_string()))?;
        let event: Socks5SidecarEvent = serde_json::from_slice(&payload)
            .map_err(|error| SidecarProcessError::Protocol(error.to_string()))?;
        let ready = match event {
            Socks5SidecarEvent::Ready(ready) => ready,
            other => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(SidecarProcessError::Protocol(format!(
                    "expected ready event, got {other:?}"
                )));
            }
        };
        if ready.session_id != session_id || ready.tunnel_port == 0 {
            let _ = child.kill();
            let _ = child.wait();
            return Err(SidecarProcessError::Protocol(
                "sidecar ready event has an invalid session or tunnel port".to_string(),
            ));
        }
        socket
            .set_read_timeout(None)
            .and_then(|_| socket.set_write_timeout(None))
            .map_err(|error| SidecarProcessError::Handshake(error.to_string()))?;

        Ok(Self {
            control: Arc::new(Mutex::new(socket)),
            ready,
            closed: Arc::new(AtomicBool::new(false)),
            process: SidecarProcess {
                guard: ProcessGuard::new(child, stderr),
            },
        })
    }

    pub fn ready(&self) -> &Socks5SidecarReady {
        &self.ready
    }

    pub fn control(&self) -> Arc<Mutex<TcpStream>> {
        self.control.clone()
    }

    pub fn closed_flag(&self) -> Arc<AtomicBool> {
        self.closed.clone()
    }

    pub fn into_process(self) -> SidecarProcess {
        self.process
    }
}

fn accept_control(
    listener: &TcpListener,
    child: &mut Child,
) -> std::result::Result<TcpStream, SidecarProcessError> {
    listener
        .set_nonblocking(true)
        .map_err(|error| SidecarProcessError::Handshake(error.to_string()))?;
    let deadline = Instant::now() + ACCEPT_TIMEOUT;
    loop {
        match listener.accept() {
            Ok((socket, _)) => {
                listener
                    .set_nonblocking(false)
                    .map_err(|error| SidecarProcessError::Handshake(error.to_string()))?;
                socket
                    .set_nonblocking(false)
                    .map_err(|error| SidecarProcessError::Handshake(error.to_string()))?;
                return Ok(socket);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if let Ok(Some(status)) = child.try_wait() {
                    return Err(SidecarProcessError::Handshake(format!(
                        "sidecar exited before connecting back: {status}"
                    )));
                }
                if Instant::now() >= deadline {
                    return Err(SidecarProcessError::Handshake(
                        "sidecar did not connect back within 10 seconds".to_string(),
                    ));
                }
                thread::sleep(POLL_INTERVAL);
            }
            Err(error) => return Err(SidecarProcessError::Handshake(error.to_string())),
        }
    }
}

fn drain_stderr(mut stderr: std::process::ChildStderr, output: Arc<Mutex<VecDeque<String>>>) {
    let mut buffer = [0u8; 1024];
    let mut partial = Vec::new();
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                partial.extend_from_slice(&buffer[..count]);
                while let Some(index) = partial.iter().position(|byte| *byte == b'\n') {
                    let line = partial.drain(..=index).collect::<Vec<_>>();
                    push_stderr_line(&output, &line);
                }
                if partial.len() > STDERR_LIMIT_BYTES {
                    push_stderr_line(&output, &partial);
                    partial.clear();
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    if !partial.is_empty() {
        push_stderr_line(&output, &partial);
    }
}

fn push_stderr_line(output: &Arc<Mutex<VecDeque<String>>>, line: &[u8]) {
    let mut text = String::from_utf8_lossy(line)
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    text = text.trim().to_string();
    if text.is_empty() {
        return;
    }
    if text.len() > 1024 {
        text.truncate(1024);
    }
    let mut output = hlock(output);
    output.push_back(text);
    while output.len() > STDERR_LIMIT_LINES {
        output.pop_front();
    }
    while output.iter().map(String::len).sum::<usize>() > STDERR_LIMIT_BYTES {
        output.pop_front();
    }
}

fn kill_tree(pid: u32) {
    #[cfg(unix)]
    unsafe {
        let _ = libc::kill(-(pid as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
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
    use std::fs;

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "heroinn-socks5-sidecar-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn locator_prefers_first_directory_and_reports_errors() {
        let first = temp_dir("first");
        let second = temp_dir("second");
        fs::write(first.join("worker"), b"first").unwrap();
        fs::write(second.join("worker"), b"second").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(first.join("worker"), fs::Permissions::from_mode(0o755)).unwrap();
            fs::set_permissions(second.join("worker"), fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert_eq!(
            locate_in(&[first.clone(), second.clone()], "worker").unwrap(),
            first.join("worker")
        );
        assert!(matches!(
            locate_in(&[first.join("missing")], "worker"),
            Err(SidecarProcessError::NotFound(_))
        ));
        let _ = fs::remove_dir_all(first);
        let _ = fs::remove_dir_all(second);
    }

    #[test]
    fn stderr_ring_is_bounded_and_sanitized() {
        let output = Arc::new(Mutex::new(VecDeque::new()));
        for index in 0..32 {
            push_stderr_line(&output, format!("line {index}\n").as_bytes());
        }
        let output = hlock(&output);
        assert!(output.len() <= STDERR_LIMIT_LINES);
        assert!(output.iter().map(String::len).sum::<usize>() <= STDERR_LIMIT_BYTES);
        assert!(!output.iter().any(|line| line.contains('\n')));
    }
}
