pub mod ftp_port;

use std::sync::{mpsc::Sender, atomic::AtomicBool, Arc};
use heroinn_util::{session::{Session, SessionBase, SessionPacket}};

use self::ftp_port::{FtpInstance, new_ftp};
pub struct FtpServer{
    id : String,
    clientid : String,
    closed : Arc<AtomicBool>,
    _sender : Sender<SessionBase>,
    instance : FtpInstance
}

impl Session for FtpServer{
    fn new_client( _ : Sender<SessionBase> ,_ : &String, _ : &String) -> std::io::Result<Self> where Self: Sized {
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "not client"))
    }

    fn new( sender : Sender<SessionBase> , clientid : &String , peer_addr : &String) -> std::io::Result<Self> where Self: Sized {

        let closed = Arc::new(AtomicBool::new(false));

        let ftp = new_ftp(&"heroinn_ftp.exe".to_string(), peer_addr)?;

        let mut ftp_1 = ftp.clone();
        let closed_2 = closed.clone();
        std::thread::spawn(move || {
            ftp_1.wait_for_exit().unwrap();
            closed_2.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let id = uuid::Uuid::new_v4().to_string();
        let id_1 = id.clone();
        let closed_1 = closed.clone();
        let clientid_1 = clientid.clone();
        let mut ftp_2 = ftp.clone();
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop{
                if closed_1.load(std::sync::atomic::Ordering::Relaxed){
                    break; 
                }

                let buf = match ftp_2.read(){
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("ftp instance read error : {}" , e);
                        break;
                    },
                };

                let packet = SessionPacket{
                    id: id_1.clone(),
                    data: buf,
                };

                match sender_1.send(SessionBase{
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
            log::info!("ftp worker closed");
            closed_1.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        Ok(Self{
            id,
            clientid: clientid.clone(),
            closed: Arc::new(AtomicBool::new(true)),
            _sender : sender,
            instance: ftp,
        })
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn write(&mut self, data : &Vec<u8>) -> std::io::Result<()> {
        self.instance.write(&data)
    }

    fn alive(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn close(&mut self) {
        self.closed.store(false, std::sync::atomic::Ordering::Relaxed)
    }

    fn clientid(&self) -> String {
        self.clientid.clone()
    }
}