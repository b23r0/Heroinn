use std::sync::{mpsc::Sender, atomic::AtomicBool, Arc};

use heroinn_util::session::{Session, SessionPacket, SessionBase};
use termport::*;

pub struct ShellServer{
    id : String,
    clientid : String,
    closed : Arc<AtomicBool>,
    term : TermInstance
}

impl Session for ShellServer{

    fn new(sender : Sender<SessionBase> , clientid : &String) -> std::io::Result<Self> {
        let closed = Arc::new(AtomicBool::new(false));

        let term = match new_term(&"alacritty_driver.exe".to_string()){
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()));
            },
        };

        let mut term_1 = term.clone();
        let closed_2 = closed.clone();
        std::thread::spawn(move || {
            term_1.wait_for_exit().unwrap();
            closed_2.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let id = uuid::Uuid::new_v4().to_string();
        let id_1 = id.clone();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.clone();
        let mut term_2 = term.clone();
        std::thread::spawn(move || {
            loop{
                if closed_1.load(std::sync::atomic::Ordering::Relaxed){
                    break; 
                }

                let mut buf = [0u8;1024];
                let size = match term_2.read(&mut buf){
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("term instance read error : {}" , e);
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

        Ok(Self { id, closed , clientid : clientid.clone() ,term : term.clone()})
    }

    fn id(&self) -> String {
        return self.id.clone()
    }

    fn write(&mut self,data : &Vec<u8>) -> std::io::Result<()> {
        self.term.write(data)
    }

    fn close(&mut self) {
        log::info!("shell closed");
        self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn alive(&self) -> bool {
        !self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    fn clientid(&self) -> String{
        self.clientid.clone()
    }

    fn new_client( _sender : Sender<SessionBase> ,_clientid : &String, _id : &String) -> std::io::Result<ShellServer> {
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "not client"))
    }
}