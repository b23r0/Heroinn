use std::net::{TcpStream, TcpListener};
use std::io::*;
use std::process::{Command, Child, ExitStatus};
use std::sync::{Mutex, Arc};

pub struct FtpInstance{
    socket: TcpStream,
    pid : u32,
    cmd : Arc<Mutex<Child>>
}

impl FtpInstance{
    pub fn new(driver_path : &String , sub_title : &String ) -> Result<FtpInstance>{

        let server = TcpListener::bind("127.0.0.1:0")?;
        let local_socket_port = format!("{}" , server.local_addr().unwrap().port());
        
        let cmd = if cfg!(target_os = "windows") {
            let mut cmd = Command::new(driver_path);
            cmd.args([local_socket_port.as_str()]);
            if !sub_title.is_empty(){
                cmd.args([sub_title.as_str()]);
            }
            cmd.spawn()
        } else {
            let mut cmd = Command::new(driver_path);
            cmd.arg(local_socket_port.as_str());
            if !sub_title.is_empty(){
                cmd.arg(sub_title);
            }
            cmd.spawn()
        }?;

        let (socket , _) = server.accept()?;

        Ok(FtpInstance{
            socket,
            pid : cmd.id(),
            cmd : Arc::new(Mutex::new(cmd))
        })
    }

    pub fn write(&mut self , buf : &[u8]) -> Result<()>{
        self.socket.write_all(buf)
    }
    pub fn read(&mut self , buf : &mut [u8]) -> Result<usize>{
        self.socket.read(buf)
    }
    pub fn wait_for_exit(&mut self) -> Result<ExitStatus>{
        self.cmd.lock().unwrap().wait()
    }
    pub fn close(&self) -> Result<()>{
        if cfg!(target_os = "windows") {
            let mut cmd = Command::new("taskkill");
            cmd.args(["/F"]);
            cmd.args(["/pid", self.pid.to_string().as_str()]);
            cmd.spawn()
        } else {
            let mut cmd = Command::new("kill");
            cmd.arg("-9");
            cmd.arg(self.pid.to_string().as_str());
            cmd.spawn()
        }?;
        Ok(())
    }

}

pub fn new_ftp(driver_path : &String , sub_title : &String) -> Result<FtpInstance>{
    FtpInstance::new(driver_path , sub_title)
}

impl Clone for FtpInstance{
    fn clone(&self) -> Self {
        Self { socket: self.socket.try_clone().unwrap(), cmd: self.cmd.clone(), pid: self.pid }
    }
}
