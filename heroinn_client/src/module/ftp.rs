use std::sync::{Arc, atomic::AtomicBool, mpsc::Sender};
use heroinn_util::{session::{Session, SessionBase, SessionPacket}, rpc::{RpcServer, RpcMessage}, ftp::{method::*, FTPPacket, FTPId}};

pub struct FtpClient{
    id : String,
    clientid : String,
    closed : Arc<AtomicBool>,
    rpc_server : RpcServer,
    sender : Sender<SessionBase>
}

impl Session for FtpClient{
    fn new_client( sender : std::sync::mpsc::Sender<heroinn_util::session::SessionBase> ,clientid : &String, id : &String) -> std::io::Result<Self> where Self: Sized {
        let mut rpc_server = RpcServer::new();
        rpc_server.register(&"get_disk_info".to_string(), get_disk_info);
        rpc_server.register(&"get_folder_info".to_string(), get_folder_info);
        rpc_server.register(&"join_path".to_string(), join_path);
        rpc_server.register(&"remove_file".to_string(), remove_file);
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
        self.id.clone()
    }

    fn write(&mut self, data : &Vec<u8>) -> std::io::Result<()> {

        let packet = FTPPacket::parse(data)?;

        match packet.id(){
            FTPId::RPC => {
                log::debug!("recv rpc call");
                let msg = RpcMessage::parse(&packet.data)?;
                let ret = self.rpc_server.call(&msg);

                let packet = FTPPacket{
                    id: FTPId::RPC.to_u8(),
                    data: ret.serialize()?,
                };

                let packet = SessionPacket{
                    id: self.id.clone(),
                    data: packet.serialize()?,
                };
                log::debug!("call ret : {:?}" , ret);
                if let Err(e) = self.sender.send(SessionBase { id: self.id.clone(), clientid: self.clientid.clone() , packet }){
                    log::error!("session sender error : {}", e );
                    self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
                };
            },
            FTPId::Close => {
                self.close();
            },
            FTPId::Unknown => {

            },
        }


        Ok(())
    }

    fn alive(&self) -> bool {
        !self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn close(&mut self) {
        log::info!("ftp session closed");
        self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn clientid(&self) -> String {
        self.clientid.clone()
    }
}