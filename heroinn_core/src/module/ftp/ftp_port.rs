//! Spawning and talking to one `heroinn_ftp` worker process.
//!
//! The worker is addressed through the shared length-prefixed framing helpers,
//! so both sides agree on the 4-byte big-endian length prefix.

use std::fmt;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use heroinn_util::Result;
use heroinn_util::protocol::frame::{read_frame, write_frame};

const SIDECAR_ACCEPT_TIMEOUT: Duration = Duration::from_secs(10);
const SIDECAR_ACCEPT_POLL: Duration = Duration::from_millis(25);

#[derive(Debug)]
enum SidecarError {
    NotFound(Vec<PathBuf>),
    NotExecutable(PathBuf),
}

impl fmt::Display for SidecarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(paths) => {
                let candidates = paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "ftp_worker_not_found: {candidates}")
            }
            Self::NotExecutable(path) => {
                write!(f, "ftp_worker_not_executable: {}", path.display())
            }
        }
    }
}

fn sidecar_dirs(exe_dir: Option<&Path>, cwd: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(exe_dir) = exe_dir {
        dirs.push(exe_dir.to_path_buf());
    }
    if let Some(cwd) = cwd
        && !dirs.iter().any(|dir| dir == cwd)
    {
        dirs.push(cwd.to_path_buf());
    }
    dirs
}

fn locate_sidecar_in(dirs: &[PathBuf], name: &str) -> std::result::Result<PathBuf, SidecarError> {
    let candidates = dirs.iter().map(|dir| dir.join(name)).collect::<Vec<_>>();
    for candidate in &candidates {
        let metadata = match std::fs::metadata(candidate) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(SidecarError::NotExecutable(candidate.clone())),
        };
        if !metadata.is_file() {
            return Err(SidecarError::NotExecutable(candidate.clone()));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 == 0 {
                return Err(SidecarError::NotExecutable(candidate.clone()));
            }
        }
        return Ok(candidate.clone());
    }
    Err(SidecarError::NotFound(candidates))
}

/// Locate a worker next to the running executable, then in the working
/// directory. The executable directory is the primary deployment layout.
pub fn locate_sidecar(name: &str) -> std::result::Result<PathBuf, String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let cwd = std::env::current_dir().ok();
    let dirs = sidecar_dirs(exe_dir.as_deref(), cwd.as_deref());
    locate_sidecar_in(&dirs, name).map_err(|error| error.to_string())
}

fn accept_sidecar(
    listener: &TcpListener,
    child: &mut Child,
) -> std::result::Result<TcpStream, String> {
    wait_for_sidecar_connection(
        listener,
        SIDECAR_ACCEPT_TIMEOUT,
        SIDECAR_ACCEPT_POLL,
        || {
            child
                .try_wait()
                .ok()
                .flatten()
                .map(|status| status.to_string())
        },
    )
}

fn wait_for_sidecar_connection<F>(
    listener: &TcpListener,
    timeout: Duration,
    poll_interval: Duration,
    mut child_exit: F,
) -> std::result::Result<TcpStream, String>
where
    F: FnMut() -> Option<String>,
{
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("ftp_worker_accept_config_failed: {error}"))?;
    let deadline = Instant::now() + timeout;
    loop {
        match listener.accept() {
            Ok((socket, _)) => {
                listener
                    .set_nonblocking(false)
                    .map_err(|error| format!("ftp_worker_accept_restore_failed: {error}"))?;
                socket
                    .set_nonblocking(false)
                    .map_err(|error| format!("ftp_worker_socket_config_failed: {error}"))?;
                return Ok(socket);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if let Some(status) = child_exit() {
                    let _ = listener.set_nonblocking(false);
                    return Err(format!("ftp_worker_exited_before_handshake: {status}"));
                }
                if Instant::now() >= deadline {
                    let _ = listener.set_nonblocking(false);
                    return Err("ftp_worker_handshake_timeout".to_string());
                }
                thread::sleep(poll_interval);
            }
            Err(error) => return Err(format!("ftp_worker_accept_failed: {error}")),
        }
    }
}

/// A worker process plus the local socket it connects back on.
pub struct FtpInstance {
    socket: TcpStream,
    pid: u32,
    cmd: Arc<Mutex<Child>>,
}

impl FtpInstance {
    /// Launch `driver_path` and wait for it to connect back.
    pub fn new(driver_path: &Path, sub_title: &str) -> Result<Self> {
        let server = TcpListener::bind("127.0.0.1:0")?;
        let local_socket_port = format!("{}", server.local_addr()?.port());

        let mut cmd = Command::new(driver_path);

        if cfg!(target_os = "windows") {
            cmd.args([local_socket_port.as_str()]);
            if !sub_title.is_empty() {
                cmd.args([sub_title]);
            }
        } else {
            cmd.arg(local_socket_port.as_str());
            if !sub_title.is_empty() {
                cmd.arg(sub_title);
            }
        }

        let mut cmd = cmd.spawn().map_err(|error| {
            heroinn_util::Error::Other(format!(
                "ftp_worker_start_failed: {}: {error}",
                driver_path.display()
            ))
        })?;

        let socket = match accept_sidecar(&server, &mut cmd) {
            Ok(socket) => socket,
            Err(error) => {
                let _ = cmd.kill();
                let _ = cmd.wait();
                return Err(heroinn_util::Error::Other(error));
            }
        };

        Ok(Self {
            socket,
            pid: cmd.id(),
            cmd: Arc::new(Mutex::new(cmd)),
        })
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<()> {
        write_frame(&mut self.socket, buf)
    }

    pub fn read(&mut self) -> Result<Vec<u8>> {
        read_frame(&mut self.socket)
    }

    pub fn wait_for_exit(&mut self) -> std::io::Result<ExitStatus> {
        self.cmd
            .lock()
            .map_err(|_| std::io::Error::other("ftp worker process lock poisoned"))?
            .wait()
    }

    /// Terminate the worker process.
    pub fn close(&self) -> std::io::Result<()> {
        if cfg!(target_os = "windows") {
            #[cfg(target_os = "windows")]
            unsafe {
                let _ = match windows::Win32::System::Threading::OpenProcess(
                    windows::Win32::System::Threading::PROCESS_TERMINATE,
                    false,
                    self.pid,
                ) {
                    Ok(h) => windows::Win32::System::Threading::TerminateProcess(h, 1),
                    Err(_) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "process not found",
                        ));
                    }
                };
            };
        } else {
            Command::new("kill")
                .arg("-9")
                .arg(self.pid.to_string())
                .spawn()?;
        }
        Ok(())
    }

    pub fn try_clone(&self) -> std::io::Result<Self> {
        Ok(Self {
            socket: self.socket.try_clone()?,
            cmd: self.cmd.clone(),
            pid: self.pid,
        })
    }

    #[cfg(test)]
    pub(super) fn from_test_parts(socket: TcpStream, child: Child) -> Self {
        Self {
            socket,
            pid: child.id(),
            cmd: Arc::new(Mutex::new(child)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Stdio;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "heroinn-ftp-sidecar-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sidecar_search_prefers_executable_directory() {
        let exe_dir = temp_dir("exe");
        let cwd = temp_dir("cwd");
        fs::write(exe_dir.join("heroinn_ftp"), b"exe").unwrap();
        fs::write(cwd.join("heroinn_ftp"), b"cwd").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                exe_dir.join("heroinn_ftp"),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap();
            fs::set_permissions(cwd.join("heroinn_ftp"), fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        let dirs = sidecar_dirs(Some(&exe_dir), Some(&cwd));
        assert_eq!(
            locate_sidecar_in(&dirs, "heroinn_ftp").unwrap(),
            exe_dir.join("heroinn_ftp")
        );
        let _ = fs::remove_dir_all(exe_dir);
        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn sidecar_reports_not_found_and_non_executable() {
        let dir = temp_dir("errors");
        let dirs = sidecar_dirs(Some(&dir), None);
        let not_found = locate_sidecar_in(&dirs, "heroinn_ftp").unwrap_err();
        assert!(not_found.to_string().contains("ftp_worker_not_found"));

        fs::write(dir.join("heroinn_ftp"), b"not executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(dir.join("heroinn_ftp"), fs::Permissions::from_mode(0o644))
                .unwrap();
            let error = locate_sidecar_in(&dirs, "heroinn_ftp").unwrap_err();
            assert!(error.to_string().contains("ftp_worker_not_executable"));
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sidecar_wait_reports_early_child_exit() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let error = wait_for_sidecar_connection(
            &listener,
            Duration::from_millis(100),
            Duration::from_millis(5),
            || Some("exit status: 1".to_string()),
        )
        .unwrap_err();
        assert!(error.contains("ftp_worker_exited_before_handshake"));
    }

    #[test]
    fn sidecar_wait_is_bounded_when_child_stays_alive() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let started = Instant::now();
        let error = wait_for_sidecar_connection(
            &listener,
            Duration::from_millis(60),
            Duration::from_millis(5),
            || None,
        )
        .unwrap_err();
        assert_eq!(error, "ftp_worker_handshake_timeout");
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn sidecar_wait_returns_a_usable_connected_stream() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let connector = thread::spawn(move || {
            let mut connected = TcpStream::connect(address).unwrap();
            std::io::Write::write_all(&mut connected, b"x").unwrap();
        });
        let stream = wait_for_sidecar_connection(
            &listener,
            Duration::from_secs(1),
            Duration::from_millis(5),
            || None,
        )
        .unwrap();
        connector.join().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        let mut byte = [0_u8; 1];
        std::io::Read::read_exact(&mut &stream, &mut byte).unwrap();
        assert_eq!(byte, [b'x']);
    }

    #[test]
    fn sidecar_error_display_is_stable_and_includes_candidates() {
        let not_found =
            SidecarError::NotFound(vec![PathBuf::from("/tmp/one"), PathBuf::from("/tmp/two")]);
        assert!(not_found.to_string().contains("ftp_worker_not_found"));
        assert!(not_found.to_string().contains("/tmp/one"));
        assert!(not_found.to_string().contains("/tmp/two"));

        let not_executable = SidecarError::NotExecutable(PathBuf::from("/tmp/worker"));
        assert!(
            not_executable
                .to_string()
                .contains("ftp_worker_not_executable")
        );
    }

    #[test]
    fn sidecar_dirs_deduplicate_the_working_directory() {
        let dir = temp_dir("dedup");
        let dirs = sidecar_dirs(Some(&dir), Some(&dir));
        assert_eq!(dirs, vec![dir.clone()]);

        let nested = dir.join("nested");
        let dirs = sidecar_dirs(Some(&dir), Some(&nested));
        assert_eq!(dirs, vec![dir.clone(), nested.clone()]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ftp_instance_frames_round_trip_through_cloned_handles() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let client = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            let payload = read_frame(&mut stream).unwrap();
            write_frame(&mut stream, &payload).unwrap();
            stream
        });
        let (server, _) = listener.accept().unwrap();
        let child = Command::new(if cfg!(windows) { "cmd" } else { "sh" })
            .args(if cfg!(windows) {
                ["/C", "ping -n 30 127.0.0.1 >NUL"]
            } else {
                ["-c", "sleep 30"]
            })
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut instance = FtpInstance {
            socket: server,
            pid: child.id(),
            cmd: Arc::new(Mutex::new(child)),
        };
        let mut cloned = instance.try_clone().unwrap();
        instance.write(b"ping").unwrap();
        let echoed = cloned.read().unwrap();
        assert_eq!(echoed, b"ping");
        let client = client.join().unwrap();
        drop(client);

        instance.close().unwrap();
        let _ = cloned.wait_for_exit().unwrap();
    }
}
