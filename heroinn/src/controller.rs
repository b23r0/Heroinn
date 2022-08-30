use std::{
    collections::HashMap,
    io::*,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU8, Ordering},
        mpsc::{channel, Sender},
        Mutex,
    },
    time::Duration,
};

use heroinn_core::{
    module::{ftp::FtpServer, shell::ShellServer},
    HeroinnServer,
};
use heroinn_util::{
    packet::*,
    session::{Session, SessionBase, SessionManager, SessionPacket},
    *,
};
use lazy_static::*;

#[derive(Clone)]
pub struct UIHostInfo {
    pub clientid: String,
    pub peer_addr: SocketAddr,
    pub proto: HeroinnProtocol,
    pub in_rate: u64,
    pub out_rate: u64,
    pub last_heartbeat: u64,
    pub info: HostInfo,
}

#[derive(Clone)]
pub struct UIListener {
    pub id: u8,
    pub protocol: HeroinnProtocol,
    pub addr: SocketAddr,
}

macro_rules! close_session_by_clientid_in_lock {
    ($session_mgr:ident,$clientid:ident) => {
        let mut mgr = $session_mgr.lock().unwrap();
        mgr.close_by_clientid(&$clientid);
        drop(mgr);
    };
}

lazy_static! {
    static ref G_ONLINE_HOSTS: Mutex<HashMap<String, UIHostInfo>> = Mutex::new(HashMap::new());
    static ref G_LISTENERS: Mutex<HashMap<u8, HeroinnServer>> = Mutex::new(HashMap::new());
    static ref G_LISTENER_ID: AtomicU8 = AtomicU8::new(0);
    static ref G_SHELL_SESSION: Mutex<SessionManager<ShellServer>> =
        Mutex::new(SessionManager::new());
    static ref G_FTP_SESSION: Mutex<SessionManager<FtpServer>> = Mutex::new(SessionManager::new());
    static ref G_SESSION_SENDER: Mutex<Sender<SessionBase>> = Mutex::new({
        let (sender, receiver) = channel::<SessionBase>();

        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(HEART_BEAT_TIME));
            let mut session = G_SHELL_SESSION.lock().unwrap();
            session.gc();

            log::info!("shell session : {}", session.count());

            let mut session = G_FTP_SESSION.lock().unwrap();
            session.gc();

            log::info!("ftp session : {}", session.count());
        });

        std::thread::spawn(move || loop {
            match receiver.recv() {
                Ok(packet) => {
                    let buf = Message::build(
                        HeroinnServerCommandID::SessionPacket.to_u8(),
                        &packet.clientid,
                        packet.packet,
                    )
                    .unwrap();
                    send_data_by_clientid(&packet.clientid, &buf).unwrap();
                }
                Err(e) => {
                    log::error!("session loop error : {}", e);
                    break;
                }
            }
        });

        sender
    });
}

pub fn cb_msg(msg: Message) {
    let mut hosts = G_ONLINE_HOSTS.lock().unwrap();

    match HeroinnClientMsgID::from(msg.id()) {
        HeroinnClientMsgID::HostInfo => {
            log::info!("hostinfo : {}", msg.clientid());

            if let std::collections::hash_map::Entry::Vacant(e) = hosts.entry(msg.clientid()) {
                e.insert(UIHostInfo {
                    clientid: msg.clientid(),
                    peer_addr: msg.peer_addr(),
                    proto: msg.proto(),
                    in_rate: 0,
                    out_rate: msg.length() as u64,
                    last_heartbeat: cur_timestamp_secs(),
                    info: msg.parser_hostinfo().unwrap(),
                });
            } else {
                let v = hosts.get_mut(&msg.clientid()).unwrap();
                *v = UIHostInfo {
                    clientid: msg.clientid(),
                    peer_addr: msg.peer_addr(),
                    proto: msg.proto(),
                    in_rate: 0,
                    out_rate: msg.length() as u64,
                    last_heartbeat: cur_timestamp_secs(),
                    info: msg.parser_hostinfo().unwrap(),
                };
            }
        }
        HeroinnClientMsgID::Heartbeat => {
            log::info!("heartbeat : {}", msg.clientid());
            if hosts.contains_key(&msg.clientid()) {
                let v = hosts.get_mut(&msg.clientid()).unwrap();
                v.last_heartbeat = cur_timestamp_secs();
                let heartbeat = msg.parser_heartbeat().unwrap();
                v.in_rate = heartbeat.in_rate;
                v.out_rate = heartbeat.out_rate;
            }
        }
        HeroinnClientMsgID::Unknow => {
            log::warn!("unknow packet id");
        }
        HeroinnClientMsgID::SessionPacket => {
            log::info!("recv SessionPacket");
            send_data_to_session(msg);
        }
    }
}

pub fn send_data_to_session(msg: Message) {
    let packet = msg.parser_sessionpacket().unwrap();

    // shell session
    let mut shell_session = G_SHELL_SESSION.lock().unwrap();
    if shell_session.contains(&packet.id) {
        shell_session.write(&packet.id, &packet.data).unwrap();
    }
    drop(shell_session);

    // ftp session
    let mut ftp_session = G_FTP_SESSION.lock().unwrap();
    if ftp_session.contains(&packet.id) {
        ftp_session.write(&packet.id, &packet.data).unwrap();
    }
    drop(ftp_session);
}

pub fn send_data_by_clientid(clientid: &String, buf: &[u8]) -> Result<()> {
    let host = G_ONLINE_HOSTS.lock().unwrap();

    if host.contains_key(clientid) {
        let mut listeners = G_LISTENERS.lock().unwrap();
        for server in listeners.values_mut() {
            if server.proto() == host[clientid].proto
                && server.contains_addr(&host[clientid].peer_addr)
            {
                server.sendto(&host[clientid].peer_addr, buf)?;
            }
        }
    }
    Ok(())
}

pub fn all_listener() -> Vec<UIListener> {
    let mut ret: Vec<UIListener> = vec![];
    let listeners = G_LISTENERS.lock().unwrap();

    for k in listeners.keys() {
        if let Some(v) = listeners.get(k) {
            ret.push(UIListener {
                id: *k,
                addr: v.local_addr().unwrap(),
                protocol: v.proto(),
            });
        }
    }

    ret
}

pub fn add_listener(proto: &HeroinnProtocol, port: u16) -> Result<u8> {
    let id = G_LISTENER_ID.load(Ordering::Relaxed);

    let server = HeroinnServer::new(proto.clone(), port, cb_msg)?;
    G_LISTENERS
        .lock()
        .unwrap()
        .insert(G_LISTENER_ID.load(Ordering::Relaxed), server);
    G_LISTENER_ID.store(id + 1, Ordering::Relaxed);
    Ok(id)
}

pub fn remove_listener(id: u8) -> Result<()> {
    let mut listener = G_LISTENERS.lock().unwrap();

    if listener.contains_key(&id) {
        let v = listener.get_mut(&id).unwrap();
        v.close();
        listener.remove(&id);
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "listener not found",
        ));
    }

    Ok(())
}

pub fn all_host() -> Vec<UIHostInfo> {
    let mut ret: Vec<UIHostInfo> = vec![];

    let hosts = G_ONLINE_HOSTS.lock().unwrap();

    for k in hosts.keys() {
        if let Some(v) = hosts.get(k) {
            ret.push(v.clone());
        }
    }

    ret
}

pub fn remove_host(clientid: String) {
    let mut host = G_ONLINE_HOSTS.lock().unwrap();

    if host.contains_key(&clientid) {
        close_session_by_clientid_in_lock!(G_SHELL_SESSION, clientid);
        close_session_by_clientid_in_lock!(G_FTP_SESSION, clientid);
        host.remove(&clientid);
    }
}

pub fn get_hostinfo_by_clientid(clientid: &String) -> Option<UIHostInfo> {
    let hosts = G_ONLINE_HOSTS.lock().unwrap();
    if hosts.contains_key(clientid) {
        return Some(hosts[clientid].clone());
    }
    None
}

pub fn open_shell(clientid: &String) -> Result<()> {
    if let Some(info) = get_hostinfo_by_clientid(clientid) {
        let sender = G_SESSION_SENDER.lock().unwrap();
        let session = ShellServer::new(sender.clone(), clientid, &format!("{}", info.peer_addr))?;
        drop(sender);

        log::info!("create shell session : {}", session.id());

        let data = SessionPacket {
            id: session.id(),
            data: vec![],
        };

        G_SHELL_SESSION.lock().unwrap().register(session);

        let data = Message::build(HeroinnServerCommandID::Shell.to_u8(), clientid, data)?;
        send_data_by_clientid(clientid, &data)?;
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "client not found",
        ));
    }

    Ok(())
}

pub fn open_ftp(clientid: &String) -> Result<()> {
    if let Some(info) = get_hostinfo_by_clientid(clientid) {
        let sender = G_SESSION_SENDER.lock().unwrap();
        let session = FtpServer::new(sender.clone(), clientid, &format!("{}", info.peer_addr))?;
        drop(sender);

        log::info!("create ftp session : {}", session.id());

        let data = SessionPacket {
            id: session.id(),
            data: vec![],
        };

        G_FTP_SESSION.lock().unwrap().register(session);

        let data = Message::build(HeroinnServerCommandID::File.to_u8(), clientid, data)?;
        send_data_by_clientid(clientid, &data)?;
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "client not found",
        ));
    }

    Ok(())
}
