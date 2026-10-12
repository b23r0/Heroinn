//! Spawning and talking to one `heroinn_ftp` worker process.
//!
//! The worker is addressed through the shared length-prefixed framing helpers,
//! so both sides agree on the 4-byte big-endian length prefix.

use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, ExitStatus};
use std::sync::{Arc, Mutex};

use heroinn_util::Result;
use heroinn_util::protocol::frame::{read_frame, write_frame};

/// A worker process plus the local socket it connects back on.
pub struct FtpInstance {
    socket: TcpStream,
    pid: u32,
    cmd: Arc<Mutex<Child>>,
}

impl FtpInstance {
    /// Launch `driver_path` and wait for it to connect back.
    pub fn new(driver_path: &str, sub_title: &str) -> Result<Self> {
        let server = TcpListener::bind("127.0.0.1:0")?;
        let local_socket_port = format!("{}", server.local_addr().unwrap().port());

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

        let cmd = cmd.spawn()?;
        let (socket, _) = server.accept()?;

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
        self.cmd.lock().unwrap().wait()
    }

    /// Terminate the worker process.
    pub fn close(&self) -> std::io::Result<()> {
        if cfg!(target_os = "windows") {
            #[cfg(target_os = "windows")]
            unsafe {
                match windows::Win32::System::Threading::OpenProcess(
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
}

impl Clone for FtpInstance {
    fn clone(&self) -> Self {
        Self {
            socket: self.socket.try_clone().unwrap(),
            cmd: self.cmd.clone(),
            pid: self.pid,
        }
    }
}
