//! Spawning and talking to one `heroinn_shell` worker process.

use std::env;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Maximum time the server waits for the terminal worker to connect back.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

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
    },
    ChildExited {
        path: PathBuf,
        status: Option<ExitStatus>,
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
            Self::HandshakeTimeout { path, timeout } => write!(
                formatter,
                "heroinn_shell at {} did not connect back within {timeout:?}",
                path.display()
            ),
            Self::ChildExited { path, status } => match status {
                Some(status) => write!(
                    formatter,
                    "heroinn_shell at {} exited before connecting back: {status}",
                    path.display()
                ),
                None => write!(
                    formatter,
                    "heroinn_shell at {} exited before connecting back",
                    path.display()
                ),
            },
            Self::HandshakeIo(error) => {
                write!(
                    formatter,
                    "loopback handshake with heroinn_shell failed: {error}"
                )
            }
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
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

struct ProcessHandle {
    child: Mutex<Child>,
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
    fn new(child: Child) -> Self {
        Self {
            child: Mutex::new(child),
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
        let mut child = self
            .child
            .lock()
            .map_err(|_| io::Error::other("shell process mutex poisoned"))?;

        if child.try_wait()?.is_none() {
            kill_process_tree(&mut child)?;
        }
        let _ = child.wait()?;
        Ok(())
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        let child = self.child.get_mut();
        let Ok(child) = child else {
            return;
        };
        let _ = kill_process_tree(child);
        let _ = child.wait();
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

        let child = command.spawn().map_err(|source| ShellLaunchError::Spawn {
            path: driver_path.to_path_buf(),
            source,
        })?;
        let process = Arc::new(ProcessHandle::new(child));

        let socket = match wait_for_handshake(&listener, &process, driver_path, timeout) {
            Ok(socket) => socket,
            Err(error) => {
                let _ = process.close();
                return Err(error);
            }
        };
        socket
            .set_nonblocking(false)
            .map_err(ShellLaunchError::HandshakeIo)?;

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
                    });
                }
                if Instant::now() >= deadline {
                    return Err(ShellLaunchError::HandshakeTimeout {
                        path: driver_path.to_path_buf(),
                        timeout,
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
