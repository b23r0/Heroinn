use std::sync::{mpsc::Sender, atomic::AtomicBool, Arc};

use heroinn_util::session::{SessionPacket, SessionBase, Session};

pub struct ShellClient{
    id : String,
    clientid : String,
    closed : Arc<AtomicBool>
}

impl Session for ShellClient{

    fn new_client( sender : Sender<SessionBase> ,clientid : &String, id : &String) -> Self {
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

        Self { id : id.clone(), clientid : clientid.clone() , closed}
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

    fn clientid(&self) -> String {
        self.clientid.clone()
    }

    fn new(_sender : Sender<SessionBase> , _id : &String) -> Self {
        todo!()
    }
}