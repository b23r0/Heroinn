//! Spawning and talking to one `heroinn_shell` worker process.

use std::collections::VecDeque;
use std::env;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Maximum time the server waits for the terminal worker to connect back.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const STDERR_BUFFER_LIMIT: usize = 8 * 1024;
const STDERR_DIAGNOSTIC_LIMIT: usize = 2 * 1024;
const MAX_STDERR_LINES: usize = 8;
const STDERR_READ_CHUNK: usize = 4 * 1024;

/// Classified failures while locating or launching a terminal worker.
#[derive(Debug)]
pub enum ShellLaunchError {
    CurrentExecutable(io::Error),
    CurrentDirectory(io::Error),
    NotFound {
        searched: Vec<PathBuf>,
    },
    NotExecutable {
        searched: Vec<PathBuf>,
    },
    Spawn {
        path: PathBuf,
        source: io::Error,
    },
    Listener(io::Error),
    HandshakeTimeout {
        path: PathBuf,
        timeout: Duration,
        stderr: String,
    },
    ChildExited {
        path: PathBuf,
        status: Option<ExitStatus>,
        stderr: String,
    },
    HandshakeIo(io::Error),
}

impl Display for ShellLaunchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentExecutable(error) => {
                write!(
                    formatter,
                    "cannot locate the Heroinn executable directory: {error}"
                )
            }
            Self::CurrentDirectory(error) => {
                write!(formatter, "cannot determine the current directory: {error}")
            }
            Self::NotFound { searched } => write!(
                formatter,
                "heroinn_shell was not found; searched {}",
                display_paths(searched)
            ),
            Self::NotExecutable { searched } => write!(
                formatter,
                "heroinn_shell exists but is not executable; checked {}",
                display_paths(searched)
            ),
            Self::Spawn { path, source } => {
                write!(
                    formatter,
                    "failed to start heroinn_shell at {}: {source}",
                    path.display()
                )
            }
            Self::Listener(error) => {
                write!(formatter, "cannot bind shell loopback listener: {error}")
            }
            Self::HandshakeTimeout {
                path,
                timeout,
                stderr,
            } => {
                write!(
                    formatter,
                    "heroinn_shell at {} did not connect back within {timeout:?}",
                    path.display()
                )?;
                write_stderr_suffix(formatter, stderr)
            }
            Self::ChildExited {
                path,
                status,
                stderr,
            } => {
                match status {
                    Some(status) => write!(
                        formatter,
                        "heroinn_shell at {} exited before connecting back: {status}",
                        path.display()
                    )?,
                    None => write!(
                        formatter,
                        "heroinn_shell at {} exited before connecting back",
                        path.display()
                    )?,
                }
                write_stderr_suffix(formatter, stderr)
            }
            Self::HandshakeIo(error) => {
                write!(
                    formatter,
                    "loopback handshake with heroinn_shell failed: {error}"
                )
            }
        }
    }
}

fn write_stderr_suffix(formatter: &mut Formatter<'_>, stderr: &str) -> fmt::Result {
    if stderr.is_empty() {
        Ok(())
    } else {
        write!(formatter, "; stderr: {stderr}")
    }
}

impl ShellLaunchError {
    fn with_stderr(self, stderr: String) -> Self {
        match self {
            Self::ChildExited { path, status, .. } => Self::ChildExited {
                path,
                status,
                stderr,
            },
            Self::HandshakeTimeout { path, timeout, .. } => Self::HandshakeTimeout {
                path,
                timeout,
                stderr,
            },
            other => other,
        }
    }
}

impl std::error::Error for ShellLaunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CurrentExecutable(error)
            | Self::CurrentDirectory(error)
            | Self::Listener(error)
            | Self::HandshakeIo(error) => Some(error),
            Self::Spawn { source, .. } => Some(source),
            Self::NotFound { .. }
            | Self::NotExecutable { .. }
            | Self::HandshakeTimeout { .. }
            | Self::ChildExited { .. } => None,
        }
    }
}

fn display_paths(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        "<no candidates>".to_string()
    } else {
        paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn shell_winit_backend_override(
    is_linux: bool,
    wayland_display: Option<&OsStr>,
    x11_display: Option<&OsStr>,
    configured_backend: Option<&OsStr>,
) -> Option<&'static str> {
    if !is_linux
        || configured_backend.is_some()
        || wayland_display.is_none_or(OsStr::is_empty)
        || x11_display.is_none_or(OsStr::is_empty)
    {
        return None;
    }

    Some("x11")
}

fn apply_shell_backend_fallback(command: &mut Command) {
    let wayland_display = env::var_os("WAYLAND_DISPLAY");
    let x11_display = env::var_os("DISPLAY");
    let configured_backend = env::var_os("WINIT_UNIX_BACKEND");
    let override_backend = shell_winit_backend_override(
        cfg!(target_os = "linux"),
        wayland_display.as_deref(),
        x11_display.as_deref(),
        configured_backend.as_deref(),
    );

    if let Some(backend) = override_backend {
        log::info!(
            "using local X11 backend for heroinn_shell because Wayland and Xwayland are available"
        );
        command.env("WINIT_UNIX_BACKEND", backend);
    }
}

struct BoundedStderr {
    bytes: VecDeque<u8>,
}

impl BoundedStderr {
    fn new() -> Self {
        Self {
            bytes: VecDeque::with_capacity(STDERR_BUFFER_LIMIT),
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        if bytes.len() >= STDERR_BUFFER_LIMIT {
            self.bytes.clear();
            self.bytes
                .extend(bytes[bytes.len() - STDERR_BUFFER_LIMIT..].iter().copied());
            return;
        }

        let excess = (self.bytes.len() + bytes.len()).saturating_sub(STDERR_BUFFER_LIMIT);
        self.bytes.drain(..excess);
        self.bytes.extend(bytes.iter().copied());
    }

    fn sanitized_tail(&self) -> String {
        let raw = self.bytes.iter().copied().collect::<Vec<_>>();
        sanitize_stderr(&raw)
    }

    #[cfg(test)]
    fn retained_bytes(&self) -> usize {
        self.bytes.len()
    }
}

fn sanitize_stderr(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    let mut lines = Vec::new();

    for line in text.split(['\n', '\r']) {
        let mut cleaned = String::with_capacity(line.len());
        for character in line.chars() {
            if character.is_control() {
                cleaned.push(' ');
            } else {
                cleaned.push(character);
            }
        }

        let cleaned = redact_url_userinfo(&cleaned);
        let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
        if !cleaned.is_empty() {
            lines.push(cleaned);
        }
    }

    let tail = lines
        .into_iter()
        .rev()
        .take(MAX_STDERR_LINES)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" | ");
    truncate_utf8(&tail, STDERR_DIAGNOSTIC_LIMIT)
}

fn redact_url_userinfo(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut remaining = line;

    while let Some(scheme_end) = remaining.find("://") {
        let authority_start = scheme_end + 3;
        output.push_str(&remaining[..authority_start]);
        let after_scheme = &remaining[authority_start..];
        let authority_end = after_scheme
            .find(|character: char| {
                character.is_whitespace()
                    || matches!(character, '"' | '\'' | '<' | '>' | ',' | ';' | ')')
            })
            .unwrap_or(after_scheme.len());
        let authority = &after_scheme[..authority_end];

        if let Some(userinfo_end) = authority.rfind('@') {
            output.push_str("<redacted>@");
            output.push_str(&authority[userinfo_end + 1..]);
        } else {
            output.push_str(authority);
        }
        remaining = &after_scheme[authority_end..];
    }

    output.push_str(remaining);
    output
}

fn truncate_utf8(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }

    let mut output = String::with_capacity(max_bytes);
    let content_limit = max_bytes.saturating_sub(3);
    for character in text.chars() {
        if output.len() + character.len_utf8() > content_limit {
            break;
        }
        output.push(character);
    }
    output.push_str("...");
    output
}

struct StderrCapture {
    buffer: Arc<Mutex<BoundedStderr>>,
    reader: Mutex<Option<JoinHandle<()>>>,
}

impl StderrCapture {
    fn empty() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(BoundedStderr::new())),
            reader: Mutex::new(None),
        }
    }

    fn from_pipe(mut stderr: ChildStderr) -> Self {
        let buffer = Arc::new(Mutex::new(BoundedStderr::new()));
        let reader_buffer = Arc::clone(&buffer);
        let reader = std::thread::Builder::new()
            .name("heroinn-shell-stderr".to_string())
            .spawn(move || {
                let mut chunk = [0u8; STDERR_READ_CHUNK];
                loop {
                    match stderr.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(size) => {
                            let Ok(mut buffer) = reader_buffer.lock() else {
                                break;
                            };
                            buffer.push(&chunk[..size]);
                        }
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
            })
            .ok();

        Self {
            buffer,
            reader: Mutex::new(reader),
        }
    }

    fn finish(&self) -> String {
        let reader = self.reader.lock().ok().and_then(|mut reader| reader.take());
        if let Some(reader) = reader {
            let _ = reader.join();
        }

        self.buffer
            .lock()
            .map(|buffer| buffer.sanitized_tail())
            .unwrap_or_default()
    }
}

/// Locate the terminal binary beside the server, then in the current directory.
pub fn resolve_shell_path(driver_name: &OsStr) -> Result<PathBuf, ShellLaunchError> {
    let executable = env::current_exe().map_err(ShellLaunchError::CurrentExecutable)?;
    let executable_dir = executable.parent().ok_or_else(|| {
        ShellLaunchError::CurrentExecutable(io::Error::new(
            io::ErrorKind::InvalidInput,
            "current executable has no parent directory",
        ))
    })?;
    let current_dir = env::current_dir().map_err(ShellLaunchError::CurrentDirectory)?;

    resolve_shell_path_from(driver_name, executable_dir, &current_dir)
}

fn resolve_shell_path_from(
    driver_name: &OsStr,
    executable_dir: &Path,
    current_dir: &Path,
) -> Result<PathBuf, ShellLaunchError> {
    let mut candidates = Vec::with_capacity(2);
    for directory in [executable_dir, current_dir] {
        let candidate = directory.join(driver_name);
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }

    if let Some(path) = candidates.iter().find(|path| is_executable_file(path)) {
        return Ok(path.clone());
    }

    if candidates.iter().any(|path| path.is_file()) {
        return Err(ShellLaunchError::NotExecutable {
            searched: candidates,
        });
    }

    Err(ShellLaunchError::NotFound {
        searched: candidates,
    })
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
}

struct ProcessHandle {
    child: Mutex<Child>,
    stderr: StderrCapture,
}

fn kill_process_tree(child: &mut Child) -> io::Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        unsafe {
            if libc::kill(-pid, libc::SIGTERM) != 0 {
                child.kill()?;
            }
        }
        for _ in 0..20 {
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        unsafe {
            let _ = libc::kill(-pid, libc::SIGKILL);
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        let mut command = Command::new("taskkill");
        command
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000);
        if !command.status().is_ok_and(|status| status.success()) {
            child.kill()?;
        }
    }

    Ok(())
}

impl ProcessHandle {
    fn new(mut child: Child) -> Self {
        let stderr = child
            .stderr
            .take()
            .map(StderrCapture::from_pipe)
            .unwrap_or_else(StderrCapture::empty);
        Self {
            child: Mutex::new(child),
            stderr,
        }
    }

    fn try_wait(&self) -> io::Result<Option<ExitStatus>> {
        self.child
            .lock()
            .map_err(|_| io::Error::other("shell process mutex poisoned"))?
            .try_wait()
    }

    fn wait_for_exit(&self) -> io::Result<ExitStatus> {
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn close(&self) -> io::Result<()> {
        let result = (|| -> io::Result<()> {
            let mut child = self
                .child
                .lock()
                .map_err(|_| io::Error::other("shell process mutex poisoned"))?;

            if child.try_wait()?.is_none() {
                kill_process_tree(&mut child)?;
            }
            let _ = child.wait()?;
            Ok(())
        })();

        let _ = self.stderr.finish();
        result
    }

    fn stderr_tail(&self) -> String {
        self.stderr.finish()
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        if let Ok(child) = self.child.get_mut() {
            let _ = kill_process_tree(child);
            let _ = child.wait();
        }
        let _ = self.stderr.finish();
    }
}

/// A worker process plus the local loopback connection it opened.
pub struct TermInstance {
    socket: TcpStream,
    process: Arc<ProcessHandle>,
}

impl TermInstance {
    /// Launch `driver_path` and wait up to [`HANDSHAKE_TIMEOUT`] for it to connect.
    pub fn new(driver_path: &Path, sub_title: &str) -> Result<Self, ShellLaunchError> {
        Self::new_with_timeout(driver_path, sub_title, HANDSHAKE_TIMEOUT)
    }

    fn new_with_timeout(
        driver_path: &Path,
        sub_title: &str,
        timeout: Duration,
    ) -> Result<Self, ShellLaunchError> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(ShellLaunchError::Listener)?;
        listener
            .set_nonblocking(true)
            .map_err(ShellLaunchError::Listener)?;
        let port = listener
            .local_addr()
            .map_err(ShellLaunchError::Listener)?
            .port();

        let mut command = Command::new(driver_path);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        command.args(["--local-socket-port", port.to_string().as_str()]);
        command.args(["--sub-title", sub_title]);
        command.stderr(Stdio::piped());
        apply_shell_backend_fallback(&mut command);

        let child = command.spawn().map_err(|source| ShellLaunchError::Spawn {
            path: driver_path.to_path_buf(),
            source,
        })?;
        let process = Arc::new(ProcessHandle::new(child));

        let socket = match wait_for_handshake(&listener, &process, driver_path, timeout) {
            Ok(socket) => socket,
            Err(error) => {
                let close_result = process.close();
                if let Err(close_error) = close_result {
                    log::warn!(
                        "failed to clean up heroinn_shell after startup failure: {close_error}"
                    );
                }
                return Err(error.with_stderr(process.stderr_tail()));
            }
        };
        if let Err(error) = socket.set_nonblocking(false) {
            let _ = process.close();
            return Err(ShellLaunchError::HandshakeIo(error));
        }

        Ok(Self { socket, process })
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            socket: self.socket.try_clone()?,
            process: Arc::clone(&self.process),
        })
    }

    pub fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        self.socket.write_all(buf)
    }

    pub fn wait_for_exit(&self) -> io::Result<ExitStatus> {
        self.process.wait_for_exit()
    }

    /// Terminate and reap the worker process.
    pub fn close(&self) -> io::Result<()> {
        let shutdown = match self.socket.shutdown(Shutdown::Both) {
            Err(error) if error.kind() == io::ErrorKind::NotConnected => Ok(()),
            result => result,
        };
        let process = self.process.close();
        shutdown?;
        process
    }
}

impl Read for TermInstance {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.socket.read(buf)
    }
}

fn wait_for_handshake(
    listener: &TcpListener,
    process: &ProcessHandle,
    driver_path: &Path,
    timeout: Duration,
) -> Result<TcpStream, ShellLaunchError> {
    let deadline = Instant::now() + timeout;

    loop {
        match listener.accept() {
            Ok((socket, _)) => return Ok(socket),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if let Some(status) = process.try_wait().map_err(ShellLaunchError::HandshakeIo)? {
                    return Err(ShellLaunchError::ChildExited {
                        path: driver_path.to_path_buf(),
                        status: Some(status),
                        stderr: String::new(),
                    });
                }
                if Instant::now() >= deadline {
                    return Err(ShellLaunchError::HandshakeTimeout {
                        path: driver_path.to_path_buf(),
                        timeout,
                        stderr: String::new(),
                    });
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(ShellLaunchError::HandshakeIo(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path =
                env::temp_dir().join(format!("heroinn-shell-test-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_candidate(path: &Path, executable: bool) {
        fs::write(path, b"test").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = if executable { 0o755 } else { 0o644 };
            fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
        }
        #[cfg(not(unix))]
        let _ = executable;
    }

    #[test]
    fn executable_directory_wins_and_duplicate_paths_are_deduplicated() {
        let executable_dir = TempDir::new();
        let current_dir = TempDir::new();
        let name = OsString::from(if cfg!(windows) {
            "heroinn_shell.exe"
        } else {
            "heroinn_shell"
        });
        write_candidate(&executable_dir.0.join(&name), true);
        write_candidate(&current_dir.0.join(&name), true);

        assert_eq!(
            resolve_shell_path_from(&name, &executable_dir.0, &current_dir.0).unwrap(),
            executable_dir.0.join(&name)
        );
        assert_eq!(
            resolve_shell_path_from(&name, &executable_dir.0, &executable_dir.0).unwrap(),
            executable_dir.0.join(&name)
        );
    }

    #[test]
    fn current_directory_is_a_compatible_fallback() {
        let executable_dir = TempDir::new();
        let current_dir = TempDir::new();
        let name = OsString::from(if cfg!(windows) {
            "heroinn_shell.exe"
        } else {
            "heroinn_shell"
        });
        write_candidate(&current_dir.0.join(&name), true);

        assert_eq!(
            resolve_shell_path_from(&name, &executable_dir.0, &current_dir.0).unwrap(),
            current_dir.0.join(&name)
        );
    }

    #[test]
    fn missing_and_non_executable_binaries_have_distinct_errors() {
        let executable_dir = TempDir::new();
        let current_dir = TempDir::new();
        let name = OsString::from(if cfg!(windows) {
            "heroinn_shell.exe"
        } else {
            "heroinn_shell"
        });

        assert!(matches!(
            resolve_shell_path_from(&name, &executable_dir.0, &current_dir.0),
            Err(ShellLaunchError::NotFound { .. })
        ));

        let path = executable_dir.0.join(&name);
        write_candidate(&path, false);
        if cfg!(unix) {
            assert!(matches!(
                resolve_shell_path_from(&name, &executable_dir.0, &current_dir.0),
                Err(ShellLaunchError::NotExecutable { .. })
            ));
        }
    }

    #[test]
    fn executable_candidates_reject_missing_paths_and_directories() {
        let directory = TempDir::new();
        let missing = directory.0.join("missing-shell");
        assert!(!is_executable_file(&missing));
        assert!(!is_executable_file(&directory.0));

        let candidate = directory.0.join("shell-candidate");
        write_candidate(&candidate, true);
        assert!(is_executable_file(&candidate));

        write_candidate(&candidate, false);
        #[cfg(unix)]
        assert!(!is_executable_file(&candidate));
        #[cfg(not(unix))]
        assert!(is_executable_file(&candidate));
    }

    #[test]
    fn handshake_timeout_terminates_an_unresponsive_child() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let child = idle_child();
        let process = ProcessHandle::new(child);

        let error = wait_for_handshake(
            &listener,
            &process,
            Path::new("heroinn_shell-test"),
            Duration::from_millis(100),
        )
        .unwrap_err();
        assert!(matches!(error, ShellLaunchError::HandshakeTimeout { .. }));
        process.close().unwrap();
    }

    #[test]
    fn early_child_exit_is_reported_before_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let child = exiting_child();
        let process = ProcessHandle::new(child);

        let error = wait_for_handshake(
            &listener,
            &process,
            Path::new("heroinn_shell-test"),
            Duration::from_secs(2),
        )
        .unwrap_err();
        match error {
            ShellLaunchError::ChildExited {
                status: Some(status),
                ..
            } => {
                assert_eq!(status.code(), Some(7));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn linux_wayland_with_x11_selects_a_local_x11_fallback() {
        let wayland = OsStr::new("wayland-0");
        let x11 = OsStr::new(":0");

        assert_eq!(
            shell_winit_backend_override(true, Some(wayland), Some(x11), None),
            Some("x11")
        );
    }

    #[test]
    fn explicit_winit_backend_is_never_overridden() {
        let wayland = OsStr::new("wayland-0");
        let x11 = OsStr::new(":0");

        assert_eq!(
            shell_winit_backend_override(
                true,
                Some(wayland),
                Some(x11),
                Some(OsStr::new("wayland")),
            ),
            None
        );
        assert_eq!(
            shell_winit_backend_override(true, Some(wayland), Some(x11), Some(OsStr::new("x11")),),
            None
        );
        assert_eq!(
            shell_winit_backend_override(true, Some(wayland), Some(x11), Some(OsStr::new(""))),
            None
        );
    }

    #[test]
    fn backend_override_requires_linux_and_both_non_empty_displays() {
        let wayland = OsStr::new("wayland-0");
        let x11 = OsStr::new(":0");

        assert_eq!(
            shell_winit_backend_override(false, Some(wayland), Some(x11), None),
            None
        );
        assert_eq!(
            shell_winit_backend_override(true, None, Some(x11), None),
            None
        );
        assert_eq!(
            shell_winit_backend_override(true, Some(wayland), None, None),
            None
        );
        assert_eq!(
            shell_winit_backend_override(true, Some(OsStr::new("")), Some(x11), None),
            None
        );
        assert_eq!(
            shell_winit_backend_override(true, Some(wayland), Some(OsStr::new("")), None),
            None
        );
    }

    #[test]
    fn stderr_buffer_keeps_a_bounded_tail_and_sanitizes_controls() {
        let mut buffer = BoundedStderr::new();
        buffer.push(b"first line\n");
        buffer.push(b"Error:\x1b[31mred\x00 detail\r\n");

        let tail = buffer.sanitized_tail();
        assert!(tail.contains("Error:"));
        assert!(tail.contains("red detail"));
        assert!(!tail.contains('\x1b'));
        assert!(!tail.contains('\0'));
        assert!(!tail.contains('\r'));
        assert!(!tail.contains('\n'));
    }

    #[test]
    fn stderr_buffer_limits_bytes_and_lines() {
        let mut buffer = BoundedStderr::new();
        for index in 0..100 {
            buffer.push(format!("line-{index:03}-{}\n", "x".repeat(200)).as_bytes());
        }

        assert_eq!(buffer.retained_bytes(), STDERR_BUFFER_LIMIT);
        let tail = buffer.sanitized_tail();
        assert!(tail.len() <= STDERR_DIAGNOSTIC_LIMIT);
        assert!(tail.contains("line-099"));
        assert!(!tail.contains("line-000"));
        assert!(tail.matches(" | ").count() < MAX_STDERR_LINES);
    }

    #[test]
    fn stderr_buffer_handles_non_utf8_and_redacts_url_userinfo() {
        let mut buffer = BoundedStderr::new();
        buffer.push(b"proxy https://alice:s3cr3t@example.test:8443 failed: ");
        buffer.push(&[0xff, 0xfe, b'\n']);

        let tail = buffer.sanitized_tail();
        assert!(tail.contains("https://<redacted>@example.test:8443"));
        assert!(!tail.contains("alice"));
        assert!(!tail.contains("s3cr3t"));
        assert!(tail.contains('\u{fffd}'));
    }

    #[cfg(unix)]
    #[test]
    fn early_exit_error_contains_sanitized_stderr_tail() {
        let directory = TempDir::new();
        let path = directory.0.join("heroinn_shell");
        write_test_script(
            &path,
            "#!/bin/sh\nprintf '%s\\n' 'Error: Error creating GL context; eglInitialize failed' >&2\nexit 1\n",
        );

        let error = match TermInstance::new_with_timeout(&path, "peer", Duration::from_secs(2)) {
            Ok(_) => panic!("fake sidecar unexpectedly connected"),
            Err(error) => error,
        };
        let display = error.to_string();
        match error {
            ShellLaunchError::ChildExited {
                status: Some(status),
                stderr,
                ..
            } => {
                assert_eq!(status.code(), Some(1));
                assert!(stderr.contains("eglInitialize failed"));
                assert!(error_text_is_sanitized(&stderr));
                assert!(display.contains("exit status: 1"));
                assert!(display.contains("stderr: Error: Error creating GL context"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn early_exit_drains_large_stderr_without_blocking_or_leaking_userinfo() {
        let directory = TempDir::new();
        let path = directory.0.join("heroinn_shell");
        write_test_script(
            &path,
            "#!/bin/sh\n\
             printf '%s\\n' 'proxy https://alice:s3cr3t@example.test:8443 failed' >&2\n\
             i=0\n\
             while [ \"$i\" -lt 1024 ]; do\n\
               printf '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\\n' >&2\n\
               i=$((i + 1))\n\
             done\n\
             exit 9\n",
        );

        let started = Instant::now();
        let error = match TermInstance::new_with_timeout(&path, "peer", Duration::from_secs(3)) {
            Ok(_) => panic!("fake sidecar unexpectedly connected"),
            Err(error) => error,
        };
        assert!(started.elapsed() < Duration::from_secs(2));

        match error {
            ShellLaunchError::ChildExited {
                status: Some(status),
                stderr,
                ..
            } => {
                assert_eq!(status.code(), Some(9));
                assert!(!stderr.is_empty());
                assert!(stderr.len() <= STDERR_DIAGNOSTIC_LIMIT);
                assert!(!stderr.contains("alice"));
                assert!(!stderr.contains("s3cr3t"));
                assert!(error_text_is_sanitized(&stderr));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn handshake_timeout_keeps_classification_and_includes_stderr() {
        let directory = TempDir::new();
        let path = directory.0.join("heroinn_shell");
        write_test_script(
            &path,
            "#!/bin/sh\nprintf '%s\\n' 'EGL initialization stalled' >&2\nsleep 30\n",
        );

        let started = Instant::now();
        let error = match TermInstance::new_with_timeout(&path, "peer", Duration::from_millis(150))
        {
            Ok(_) => panic!("unresponsive fake sidecar unexpectedly connected"),
            Err(error) => error,
        };
        assert!(started.elapsed() < Duration::from_secs(2));

        match error {
            ShellLaunchError::HandshakeTimeout { stderr, .. } => {
                assert!(stderr.contains("EGL initialization stalled"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[cfg(unix)]
    fn write_test_script(path: &Path, contents: &str) {
        let status = Command::new("sh")
            .args([
                "-c",
                "printf '%s' \"$1\" > \"$2\" && chmod 755 \"$2\"",
                "heroinn-shell-fixture",
                contents,
            ])
            .arg(path)
            .status()
            .unwrap();
        assert!(status.success(), "fixture writer failed: {status}");
    }

    #[cfg(unix)]
    #[test]
    fn test_script_writer_preserves_literal_contents_and_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TempDir::new();
        let path = directory.0.join("shell ' fixture $ `% Ω");
        let contents =
            "#!/bin/sh\n# ' \"$HOME\" `uname` % Ω\nprintf '%s' 'fixture output'\nexit 7\n";
        write_test_script(&path, contents);

        assert_eq!(fs::read(&path).unwrap(), contents.as_bytes());
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );
        let output = Command::new(&path).output().unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"fixture output");
        assert!(output.stderr.is_empty());
    }

    #[cfg(windows)]
    fn write_test_script(path: &Path, contents: &str) {
        fs::write(path, contents.replace('\n', "\r\n")).unwrap();
    }

    #[cfg(unix)]
    fn error_text_is_sanitized(text: &str) -> bool {
        !text
            .chars()
            .any(|character| character.is_control() && character != '\t')
    }

    #[cfg(unix)]
    #[test]
    fn closing_a_worker_terminates_its_process_group() {
        use std::os::unix::process::CommandExt;

        let directory = TempDir::new();
        let pid_file = directory.0.join("grandchild.pid");
        let mut command = Command::new("sh");
        command
            .args([
                "-c",
                "sleep 30 & echo $! > \"$1\"; wait",
                "heroinn-shell-test",
            ])
            .arg(&pid_file)
            .process_group(0);
        let process = ProcessHandle::new(command.spawn().unwrap());

        let mut grandchild = None;
        for _ in 0..100 {
            if let Some(pid) = fs::read_to_string(&pid_file)
                .ok()
                .and_then(|pid| pid.trim().parse().ok())
            {
                grandchild = Some(pid);
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let grandchild: i32 = grandchild.expect("worker did not report its grandchild pid");

        assert_eq!(unsafe { libc::kill(grandchild, 0) }, 0);
        process.close().unwrap();
        for _ in 0..100 {
            if unsafe { libc::kill(grandchild, 0) } != 0 {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("grandchild {grandchild} survived worker cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn windows_early_exit_error_contains_sanitized_stderr_tail() {
        let directory = TempDir::new();
        let path = directory.0.join("heroinn_shell.cmd");
        write_test_script(
            &path,
            "@echo off\necho Error: Error creating GL context; eglInitialize failed 1>&2\nexit /b 1\n",
        );

        let error = match TermInstance::new_with_timeout(&path, "peer", Duration::from_secs(2)) {
            Ok(_) => panic!("fake sidecar unexpectedly connected"),
            Err(error) => error,
        };
        match error {
            ShellLaunchError::ChildExited {
                status: Some(status),
                stderr,
                ..
            } => {
                assert_eq!(status.code(), Some(1));
                assert!(stderr.contains("eglInitialize failed"));
                assert!(!stderr.chars().any(char::is_control));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_handshake_timeout_keeps_classification_and_includes_stderr() {
        let directory = TempDir::new();
        let path = directory.0.join("heroinn_shell.cmd");
        write_test_script(
            &path,
            "@echo off\necho EGL initialization stalled 1>&2\nping -n 30 127.0.0.1 >NUL\n",
        );

        let started = Instant::now();
        let error = match TermInstance::new_with_timeout(&path, "peer", Duration::from_millis(150))
        {
            Ok(_) => panic!("unresponsive fake sidecar unexpectedly connected"),
            Err(error) => error,
        };
        assert!(started.elapsed() < Duration::from_secs(3));
        match error {
            ShellLaunchError::HandshakeTimeout { stderr, .. } => {
                assert!(stderr.contains("EGL initialization stalled"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[cfg(unix)]
    fn idle_child() -> Child {
        Command::new("sh")
            .args(["-c", "while :; do sleep 1; done"])
            .spawn()
            .unwrap()
    }

    #[cfg(windows)]
    fn idle_child() -> Child {
        Command::new("cmd")
            .args(["/C", "ping -n 30 127.0.0.1 >NUL"])
            .spawn()
            .unwrap()
    }

    #[cfg(unix)]
    fn exiting_child() -> Child {
        Command::new("sh").args(["-c", "exit 7"]).spawn().unwrap()
    }

    #[cfg(windows)]
    fn exiting_child() -> Child {
        Command::new("cmd").args(["/C", "exit 7"]).spawn().unwrap()
    }
}
