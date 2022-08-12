use std::sync::{Arc, atomic::AtomicBool, mpsc::Sender};
use std::io::*;
use heroinn_util::ftp::DiskInfo;
use heroinn_util::{session::{Session, SessionBase, SessionPacket}, rpc::{RpcServer, RpcMessage}};
use sysinfo::{SystemExt, DiskExt};

pub struct FtpClient{
    id : String,
    clientid : String,
    closed : Arc<AtomicBool>,
    rpc_server : RpcServer,
    sender : Sender<SessionBase>
}

fn get_disk_info(_ : Vec<String>) -> Result<Vec<String>>{
    
    let mut ret = vec![];

    let sys = sysinfo::System::new_all();
    for d in sys.disks(){
        let name = d.name().to_str().unwrap().to_string();
        let typ = format!("{:?}", d.file_system());
        let size = d.available_space();

        let info = DiskInfo{
            name,
            size,
            typ,
        };

        ret.push(info.serialize()?);
    }
    Ok(vec![])
}

impl Session for FtpClient{
    fn new_client( sender : std::sync::mpsc::Sender<heroinn_util::session::SessionBase> ,clientid : &String, id : &String) -> std::io::Result<Self> where Self: Sized {
        let mut rpc_server = RpcServer::new();
        rpc_server.register(&"get_disk_info".to_string(), get_disk_info);
        Ok(Self{
            id: id.clone(),
            clientid: clientid.clone(),
            closed: Arc::new(AtomicBool::new(false)),
            rpc_server,
            sender
        })
    }

    fn new( _ : std::sync::mpsc::Sender<heroinn_util::session::SessionBase> , _ : &String , _ : &String) -> std::io::Result<Self> where Self: Sized {
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "not server"))
    }

    fn id(&self) -> String {
        todo!()
    }

    fn write(&mut self, data : &Vec<u8>) -> std::io::Result<()> {
        let msg = RpcMessage::parse(data)?;
        let ret = self.rpc_server.call(&msg);
        let packet = SessionPacket{
            id: self.id.clone(),
            data: ret.serialize()?,
        };
        if let Err(e) = self.sender.send(SessionBase { id: self.id.clone(), clientid: self.clientid.clone() , packet }){
            log::error!("session sender error : {}", e );
            self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
        };
        Ok(())
    }

    fn alive(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn close(&mut self) {
        self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn clientid(&self) -> String {
        self.clientid.clone()
    }
}