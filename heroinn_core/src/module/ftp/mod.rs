pub mod ftp_port;

use std::sync::{mpsc::Sender, atomic::AtomicBool, Arc};
use heroinn_util::{session::{Session, SessionPacket, SessionBase}, rpc::RpcClient};

use self::ftp_port::{FtpInstance, new_ftp};
pub struct FtpServer{
    id : String,
    clientid : String,
    closed : Arc<AtomicBool>,
    sender : Sender<SessionBase>,
    instance : FtpInstance
}

impl Session for FtpServer{
    fn new_client( sender : Sender<SessionBase> ,clientid : &String, id : &String) -> std::io::Result<Self> where Self: Sized {
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "not client"))
    }

    fn new( sender : Sender<SessionBase> , clientid : &String , peer_addr : &String) -> std::io::Result<Self> where Self: Sized {
        Ok(Self{
            id: uuid::Uuid::new_v4().to_string(),
            clientid: clientid.clone(),
            closed: Arc::new(AtomicBool::new(true)),
            sender,
            instance: new_ftp(&"heroinn_ftp.exe".to_string(), peer_addr)?,
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