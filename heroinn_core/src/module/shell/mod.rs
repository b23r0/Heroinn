mod shell_port;
use self::shell_port::*;
use heroinn_util::session::{Session, SessionBase, SessionPacket};
use std::env::current_dir;
use std::sync::{atomic::AtomicBool, mpsc::Sender, Arc};

static MAGIC_FLAG: [u8; 2] = [0x37, 0x37];

pub struct ShellServer {
    id: String,
    clientid: String,
    closed: Arc<AtomicBool>,
    term: TermInstance,
    sender: Sender<SessionBase>,
}

impl Session for ShellServer {
    fn new(
        sender: Sender<SessionBase>,
        clientid: &String,
        peer_addr: &String,
    ) -> std::io::Result<Self> {
        let closed = Arc::new(AtomicBool::new(false));

        #[cfg(not(target_os = "windows"))]
        let driver_path = current_dir()
            .unwrap()
            .join("heroinn_shell")
            .to_str()
            .unwrap()
            .to_string();

        #[cfg(target_os = "windows")]
        let driver_path = current_dir()
            .unwrap()
            .join("heroinn_shell.exe")
            .to_str()
            .unwrap()
            .to_string();

        let term = match TermInstance::new(&driver_path, peer_addr) {
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                ));
            }
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
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                if closed_1.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                let mut buf = [0u8; 1024];
                let size = match term_2.read(&mut buf) {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("term instance read error : {}", e);
                        break;
                    }
                };

                let packet = SessionPacket {
                    id: id_1.clone(),
                    data: buf[..size].to_vec(),
                };

                match sender_1.send(SessionBase {
                    id: id_1.clone(),
                    clientid: clientid_1.clone(),
                    packet,
                }) {
                    Ok(_) => {}
                    Err(e) => {
                        log::info!("sender closed : {}", e);
                        break;
                    }
                };
            }
            log::info!("shell worker closed");
            closed_1.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        Ok(Self {
            id,
            closed,
            clientid: clientid.clone(),
            term,
            sender,
        })
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn write(&mut self, data: &Vec<u8>) -> std::io::Result<()> {
        if data.len() == 3 && self.alive() && data == &vec![MAGIC_FLAG[0], MAGIC_FLAG[1], 0xff] {
            log::info!("client closed");
            self.close();
            return Ok(());
        }

        self.term.write(data)
    }

    fn close(&mut self) {
        log::info!("shell closed");

        let packet = SessionPacket {
            id: self.id.clone(),
            data: vec![MAGIC_FLAG[0], MAGIC_FLAG[1], 0xff],
        };

        match self.sender.send(SessionBase {
            id: self.id.clone(),
            clientid: self.clientid.clone(),
            packet,
        }) {
            Ok(_) => {}
            Err(_) => {}
        };

        self.term.close().unwrap();
        self.closed
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn alive(&self) -> bool {
        !self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn clientid(&self) -> String {
        self.clientid.clone()
    }

    fn new_client(
        _sender: Sender<SessionBase>,
        _clientid: &String,
        _id: &String,
    ) -> std::io::Result<ShellServer> {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not client",
        ))
    }
}
