use std::sync::{mpsc::Sender, atomic::AtomicBool, Arc};

use heroinn_util::session::{Session, SessionPacket, SessionBase};

pub struct ShellServer{
    id : String,
    clientid : String,
    closed : Arc<AtomicBool>
}

impl Session for ShellServer{

    fn new(sender : Sender<SessionBase> , clientid : &String) -> Self {
        let closed = Arc::new(AtomicBool::new(false));

        let id = uuid::Uuid::new_v4().to_string();
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
                    data: vec![1,2,3],
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

        Self { id, closed , clientid : clientid.clone() }
    }

    fn id(&self) -> String {
        return self.id.clone()
    }

    fn write(&mut self,data : &Vec<u8>) -> std::io::Result<()> {
        log::info!("testsession : {:?}" , data);
        Ok(())
    }

    fn close(&mut self) {
        log::info!("testsession closed");
        self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn alive(&self) -> bool {
        !self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    fn clientid(&self) -> String{
        self.clientid.clone()
    }

    fn new_client( _sender : Sender<SessionBase> ,_clientid : &String, _id : &String) -> Self {
        todo!()
    }
}