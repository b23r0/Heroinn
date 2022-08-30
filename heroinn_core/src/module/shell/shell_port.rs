use std::io::*;
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, ExitStatus};
use std::sync::{Arc, Mutex};

pub struct TermInstance {
    socket: TcpStream,
    pid: u32,
    cmd: Arc<Mutex<Child>>,
}

impl TermInstance {
    pub fn new(driver_path: &String, sub_title: &String) -> Result<TermInstance> {
        let server = TcpListener::bind("127.0.0.1:0")?;
        let local_socket_port = format!("{}", server.local_addr().unwrap().port());

        let cmd = if cfg!(target_os = "windows") {
            let mut cmd = Command::new(driver_path);
            cmd.args(["--local-socket-port", local_socket_port.as_str()]);
            if !sub_title.is_empty() {
                cmd.args(["--sub-title", sub_title.as_str()]);
            }
            cmd.spawn()
        } else {
            let mut cmd = Command::new(driver_path);
            cmd.arg("--local-socket-port");
            cmd.arg(local_socket_port.as_str());
            if !sub_title.is_empty() {
                cmd.arg("--sub-title");
                cmd.arg(sub_title);
            }
            cmd.spawn()
        }?;

        let (socket, _) = server.accept()?;

        Ok(TermInstance {
            socket,
            pid: cmd.id(),
            cmd: Arc::new(Mutex::new(cmd)),
        })
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<()> {
        self.socket.write_all(buf)
    }
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.socket.read(buf)
    }
    pub fn wait_for_exit(&mut self) -> Result<ExitStatus> {
        self.cmd.lock().unwrap().wait()
    }
    pub fn close(&self) -> Result<()> {
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
                            std::io::ErrorKind::InvalidData,
                            "process not found",
                        ))
                    }
                };
            };
        } else {
            let mut cmd = Command::new("kill");
            cmd.arg("-9");
            cmd.arg(self.pid.to_string().as_str());
            cmd.spawn()?;
        };
        Ok(())
    }
}

impl Clone for TermInstance {
    fn clone(&self) -> Self {
        Self {
            socket: self.socket.try_clone().unwrap(),
            cmd: self.cmd.clone(),
            pid: self.pid,
        }
    }
}
