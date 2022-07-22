use std::sync::{mpsc::{Sender}, atomic::AtomicBool, Arc};
use heroinn_util::session::{SessionPacket, SessionBase, Session};
#[cfg(target_os = "windows")]
use windows::Win32::System::{Threading::{OpenProcess, WaitForSingleObject}};
use std::io::*;

use super::conpty::{self, Process};

pub struct ShellClient{
    id : String,
    clientid : String,
    closed : Arc<AtomicBool>,
    #[cfg(target_os = "windows")]
    process : Process,
    #[cfg(target_os = "windows")]
    writer : conpty::io::PipeWriter,
    
}

static MAGIC_FLAG : [u8;2] = [0x37, 0x37];

pub fn makeword(a : u8, b : u8) -> u16{
    ((a as u16) << 8) | b as u16
}

impl Session for ShellClient{

    #[cfg(target_os = "windows")]
    fn new_client( sender : Sender<SessionBase> ,clientid : &String, id : &String) -> Result<Self> {
        let closed = Arc::new(AtomicBool::new(false));
        
        let process = match conpty::spawn("cmd"){
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()));
            },
        };
        
        let pid = process.pid();

        let closed_2 = closed.clone();
        std::thread::spawn(move || {
            let handle = unsafe { OpenProcess(windows::Win32::System::Threading::PROCESS_ALL_ACCESS, false, pid) };
    
            if !handle.is_invalid() {
                unsafe { WaitForSingleObject(handle, 0xffffffff)};
            }

            log::info!("shell process exit");
            closed_2.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let writer = match process.input() {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()));
            }
        };
        let mut reader = match process.output()  {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()));
            }
        };

        let id_1 = id.clone();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.clone();

        std::thread::spawn(move || {
            loop{
                if closed_1.load(std::sync::atomic::Ordering::Relaxed){
                    break; 
                }
                let mut buf : [u8;1024] = [0;1024];
                let size = match reader.read(&mut buf){
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("shell process reader thread exit : {}" , e);
                        break;
                    },
                };

                let packet = SessionPacket{
                    id: id_1.clone(),
                    data: buf[..size].to_vec(),
                };

                match sender.send(SessionBase{
                    id: id_1.clone(),
                    clientid : clientid_1.clone(),
                    packet : packet
                }){
                    Ok(_) => {},
                    Err(e) => {
                        log::info!("sender closed : {}" , e);
                        break;
                    },
                };
            }
            log::info!("shell worker closed");
            closed_1.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        Ok(Self { id : id.clone(), clientid : clientid.clone() , closed , writer , process})
    }

    #[cfg(target_os = "linux")]
    fn new_client( sender : Sender<SessionBase> ,clientid : &String, id : &String) -> Result<Self> {
        let closed = Arc::new(AtomicBool::new(false));
        
        let id_1 = id.clone();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.clone();
        std::thread::spawn(move || {
            loop{
                if closed_1.load(std::sync::atomic::Ordering::Relaxed){
                    break; 
                }

                let packet = SessionPacket{
                    id: id_1.clone(),
                    data: vec![4,5,6],
                };

                match sender.send(SessionBase{
                    id: id_1.clone(),
                    clientid : clientid_1.clone(),
                    packet : packet
                }){
                    Ok(_) => {},
                    Err(e) => {
                        log::info!("sender closed : {}" , e);
                        break;
                    },
                };
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            log::info!("TestSession worker closed");
            closed_1.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        Ok(Self { id : id.clone(), clientid : clientid.clone() , closed})
    }

    fn id(&self) -> String {
        return self.id.clone()
    }

    fn write(&mut self,data : &Vec<u8>) -> std::io::Result<()> {
        if data.len() == 6 && data[0] == MAGIC_FLAG[0] && data[1] == MAGIC_FLAG[1] {

            let row = makeword(data[2] , data[3]);
            let col = makeword(data[4] , data[5]);

            self.process.resize(col as i16 , row as i16).unwrap();
            return Ok(());
        }

        self.writer.write_all(data)
    }

    fn close(&mut self) {
        log::info!("shell closed");
        self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn alive(&self) -> bool {
        !self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn clientid(&self) -> String {
        self.clientid.clone()
    }

    fn new(_sender : Sender<SessionBase> , _id : &String) -> Result<Self>  {
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "not server"))
    }
}