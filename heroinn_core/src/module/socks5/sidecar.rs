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
    accept_control_with(listener, ACCEPT_TIMEOUT, POLL_INTERVAL, || {
        child
            .try_wait()
            .ok()
            .flatten()
            .map(|status| status.to_string())
    })
}

fn accept_control_with<F>(
    listener: &TcpListener,
    timeout: Duration,
    poll_interval: Duration,
    mut child_exit: F,
) -> std::result::Result<TcpStream, SidecarProcessError>
where
    F: FnMut() -> Option<String>,
{
    listener
        .set_nonblocking(true)
        .map_err(|error| SidecarProcessError::Handshake(error.to_string()))?;
    let deadline = Instant::now() + timeout;
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
                if let Some(status) = child_exit() {
                    return Err(SidecarProcessError::Handshake(format!(
                        "sidecar exited before connecting back: {status}"
                    )));
                }
                if Instant::now() >= deadline {
                    return Err(SidecarProcessError::Handshake(
                        "sidecar did not connect back within 10 seconds".to_string(),
                    ));
                }
                thread::sleep(poll_interval);
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

    #[cfg(unix)]
    #[test]
    fn locator_rejects_non_executable_files_and_directories() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("nonexec");
        let worker = dir.join("worker");
        fs::write(&worker, b"not executable").unwrap();
        fs::set_permissions(&worker, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            locate_in(std::slice::from_ref(&dir), "worker"),
            Err(SidecarProcessError::NotExecutable(path)) if path == worker
        ));

        fs::remove_file(&worker).unwrap();
        fs::create_dir(dir.join("worker")).unwrap();
        assert!(matches!(
            locate_in(std::slice::from_ref(&dir), "worker"),
            Err(SidecarProcessError::NotExecutable(_))
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stderr_line_sanitizes_controls_and_enforces_the_byte_limit() {
        let output = Arc::new(Mutex::new(VecDeque::new()));
        push_stderr_line(&output, b"  hello\x00world\r\n");
        push_stderr_line(&output, "x".repeat(4096).as_bytes());
        let output = hlock(&output);
        assert_eq!(output[0], "hello world");
        assert!(output[1].len() <= 1024);
        assert!(!output.iter().any(|line| line.chars().any(char::is_control)));
    }

    #[cfg(unix)]
    #[test]
    fn early_exiting_child_is_reported_without_waiting_for_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut child = Command::new("sh")
            .args(["-c", "exit 7"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let started = Instant::now();
        let error = accept_control(&listener, &mut child).unwrap_err();
        assert!(matches!(
            error,
            SidecarProcessError::Handshake(message) if message.contains("exited before connecting back")
        ));
        assert!(started.elapsed() < ACCEPT_TIMEOUT);
        let _ = child.wait();
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

    #[test]
    fn sidecar_error_display_covers_every_stable_category() {
        let not_found = SidecarProcessError::NotFound(vec![
            PathBuf::from("/tmp/one"),
            PathBuf::from("/tmp/two"),
        ]);
        assert!(not_found.to_string().contains("socks5_sidecar_not_found"));
        assert!(not_found.to_string().contains("/tmp/one"));

        let not_executable = SidecarProcessError::NotExecutable(PathBuf::from("/tmp/worker"));
        assert!(
            not_executable
                .to_string()
                .contains("socks5_sidecar_not_executable")
        );
        assert_eq!(
            SidecarProcessError::Spawn("spawn detail".to_string()).to_string(),
            "socks5_sidecar_spawn_failed: spawn detail"
        );
        assert_eq!(
            SidecarProcessError::Handshake("handshake detail".to_string()).to_string(),
            "socks5_sidecar_handshake_failed: handshake detail"
        );
        assert_eq!(
            SidecarProcessError::Protocol("protocol detail".to_string()).to_string(),
            "socks5_sidecar_protocol_error: protocol detail"
        );
    }

    #[test]
    fn binary_name_and_directory_candidates_are_consistent() {
        let name = sidecar_binary_name();
        if cfg!(target_os = "windows") {
            assert!(name.ends_with(".exe"));
        } else {
            assert!(!name.ends_with(".exe"));
        }
        assert!(!sidecar_dirs().is_empty());
    }

    #[test]
    fn public_locator_wrappers_agree_on_sidecar_presence() {
        assert_eq!(sidecar_available(), locate_sidecar().is_ok());
    }

    #[test]
    fn accept_control_uses_the_default_timeout_and_detects_a_late_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let connector = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            let _ = TcpStream::connect(address);
        });
        let mut child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "ping -n 30 127.0.0.1 >NUL"])
                .spawn()
                .unwrap()
        } else {
            Command::new("sh").args(["-c", "sleep 30"]).spawn().unwrap()
        };
        let socket = accept_control(&listener, &mut child).unwrap();
        drop(socket);
        connector.join().unwrap();
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn accept_control_with_accepts_a_late_client_connection() {
        use std::io::Write;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let client = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            let mut socket = TcpStream::connect(address).unwrap();
            socket.write_all(b"x").unwrap();
            thread::sleep(Duration::from_millis(100));
        });

        let mut socket = accept_control_with(
            &listener,
            Duration::from_secs(2),
            Duration::from_millis(10),
            || None,
        )
        .unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut byte = [0u8; 1];
        socket.read_exact(&mut byte).unwrap();
        assert_eq!(byte[0], b'x');
        client.join().unwrap();
    }

    #[test]
    fn accept_control_with_reports_early_exit_and_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let started = Instant::now();
        let error = accept_control_with(
            &listener,
            Duration::from_secs(2),
            Duration::from_millis(5),
            || Some("exit status: 7".to_string()),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            SidecarProcessError::Handshake(message)
                if message.contains("exited before connecting back")
        ));
        assert!(started.elapsed() < Duration::from_secs(1));

        let started = Instant::now();
        let error = accept_control_with(
            &listener,
            Duration::from_millis(100),
            Duration::from_millis(5),
            || None,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            SidecarProcessError::Handshake(message) if message.contains("did not connect back")
        ));
        assert!(started.elapsed() >= Duration::from_millis(90));
    }

    #[test]
    fn drain_stderr_collects_lines_and_flushes_a_partial_tail() {
        #[cfg(windows)]
        let mut command = {
            let mut command = Command::new("cmd");
            command.args(["/C", "(echo alpha& echo beta) 1>&2"]);
            command
        };
        #[cfg(not(windows))]
        let mut command = {
            let mut command = Command::new("sh");
            command.args(["-c", "printf 'alpha\\nbeta' >&2"]);
            command
        };
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stderr = child.stderr.take().unwrap();
        let output = Arc::new(Mutex::new(VecDeque::new()));
        drain_stderr(stderr, output.clone());
        let _ = child.wait();
        let output = hlock(&output);
        let lines = output.iter().cloned().collect::<Vec<_>>();
        assert!(lines.len() >= 2);
        assert!(lines.iter().any(|line| line.contains("alpha")));
        assert!(lines.iter().any(|line| line.contains("beta")));
        assert!(!lines.iter().any(|line| line.contains('\n')));
    }

    #[cfg(unix)]
    #[test]
    fn process_guard_close_reaps_a_grouped_child_and_is_idempotent() {
        use std::os::unix::process::CommandExt;

        let mut command = Command::new("sh");
        command
            .args(["-c", "sleep 30"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .process_group(0);
        let mut child = command.spawn().unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut process = SidecarProcess {
            guard: ProcessGuard::new(child, stderr),
        };
        assert!(process.alive());
        process.close();
        assert!(!process.alive());
        process.close();
        assert!(!process.alive());
    }

    #[cfg(windows)]
    #[test]
    fn process_guard_close_reaps_a_windows_child_and_is_idempotent() {
        let mut command = Command::new("cmd");
        command
            .args(["/C", "ping -n 30 127.0.0.1 >NUL"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut process = SidecarProcess {
            guard: ProcessGuard::new(child, stderr),
        };
        assert!(process.alive());
        process.close();
        assert!(!process.alive());
        process.close();
        assert!(!process.alive());
    }
}
