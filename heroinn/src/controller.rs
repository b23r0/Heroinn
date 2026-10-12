//! Server state shared by the GUI: online hosts, listeners and sessions.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use heroinn_core::module::ftp::FtpServer;
use heroinn_core::module::shell::ShellServer;
use heroinn_util::session::{ServerSession, Session, SessionBase, SessionManager, SessionPacket};
use heroinn_util::{
    Error, HEART_BEAT_TIME, HeroinnClientMsgID, HeroinnProtocol, HeroinnServerCommandID, Result,
    cur_timestamp_secs,
    packet::{HostInfo, Message},
    protocol::Listener,
};

/// Snapshot of one connected client, as shown in the host table.
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

/// Snapshot of one bound listener, as shown in the listener table.
#[derive(Clone)]
pub struct UIListener {
    pub id: u8,
    pub protocol: HeroinnProtocol,
    pub addr: SocketAddr,
}

static G_ONLINE_HOSTS: LazyLock<Mutex<HashMap<String, UIHostInfo>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static G_LISTENERS: LazyLock<Mutex<HashMap<u8, Listener>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static G_LISTENER_ID: LazyLock<AtomicU8> = LazyLock::new(|| AtomicU8::new(0));
static G_SHELL_SESSION: LazyLock<Mutex<SessionManager<ShellServer>>> =
    LazyLock::new(|| Mutex::new(SessionManager::new()));
static G_FTP_SESSION: LazyLock<Mutex<SessionManager<FtpServer>>> =
    LazyLock::new(|| Mutex::new(SessionManager::new()));
static G_SESSION_SENDER: LazyLock<Mutex<Sender<SessionBase>>> = LazyLock::new(|| {
    let (sender, receiver) = channel::<SessionBase>();

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(HEART_BEAT_TIME));
            let mut session = G_SHELL_SESSION.lock().unwrap();
            session.gc();
            log::info!("shell session : {}", session.count());

            let mut session = G_FTP_SESSION.lock().unwrap();
            session.gc();
            log::info!("ftp session : {}", session.count());
        }
    });

    std::thread::spawn(move || {
        loop {
            match receiver.recv() {
                Ok(packet) => {
                    let Ok(buf) = Message::build(
                        HeroinnServerCommandID::SessionPacket.to_u8(),
                        &packet.clientid,
                        packet.packet,
                    ) else {
                        continue;
                    };

                    if let Err(e) = send_data_by_clientid(&packet.clientid, &buf) {
                        log::error!("send session packet failed : {}", e);
                    }
                }
                Err(e) => {
                    log::error!("session loop error : {}", e);
                    break;
                }
            }
        }
    });

    Mutex::new(sender)
});

/// Handle one decoded client message: refresh host state or route a session packet.
pub fn cb_msg(msg: Message) {
    match HeroinnClientMsgID::try_from(msg.id()) {
        Ok(HeroinnClientMsgID::HostInfo) => {
            let Ok(info) = msg.parse_hostinfo() else {
                log::warn!("malformed hostinfo from {}", msg.clientid());
                return;
            };

            log::info!("hostinfo : {}", msg.clientid());

            G_ONLINE_HOSTS.lock().unwrap().insert(
                msg.clientid().to_string(),
                UIHostInfo {
                    clientid: msg.clientid().to_string(),
                    peer_addr: msg.peer_addr(),
                    proto: msg.proto(),
                    in_rate: 0,
                    out_rate: msg.length() as u64,
                    last_heartbeat: cur_timestamp_secs(),
                    info,
                },
            );
        }
        Ok(HeroinnClientMsgID::Heartbeat) => {
            log::info!("heartbeat : {}", msg.clientid());

            let Ok(heartbeat) = msg.parse_heartbeat() else {
                return;
            };

            let mut hosts = G_ONLINE_HOSTS.lock().unwrap();
            if let Some(v) = hosts.get_mut(msg.clientid()) {
                v.last_heartbeat = cur_timestamp_secs();
                v.in_rate = heartbeat.in_rate;
                v.out_rate = heartbeat.out_rate;
            }
        }
        Ok(HeroinnClientMsgID::SessionPacket) => {
            log::info!("recv SessionPacket");
            send_data_to_session(msg);
        }
        Err(e) => log::warn!("unknown packet id : {}", e),
    }
}

/// Deliver a session packet to whichever local session owns its id.
pub fn send_data_to_session(msg: Message) {
    let Ok(packet) = msg.parse_sessionpacket() else {
        log::warn!("malformed session packet from {}", msg.clientid());
        return;
    };

    let mut shell_session = G_SHELL_SESSION.lock().unwrap();
    if shell_session.contains(&packet.id) {
        let _ = shell_session.write(&packet.id, &packet.data);
    }
    drop(shell_session);

    let mut ftp_session = G_FTP_SESSION.lock().unwrap();
    if ftp_session.contains(&packet.id) {
        let _ = ftp_session.write(&packet.id, &packet.data);
    }
}

/// Send `buf` to the client `clientid` over whichever listener accepted it.
pub fn send_data_by_clientid(clientid: &str, buf: &[u8]) -> Result<()> {
    let host = {
        let hosts = G_ONLINE_HOSTS.lock().unwrap();
        match hosts.get(clientid) {
            Some(h) => h.clone(),
            None => return Ok(()),
        }
    };

    let mut listeners = G_LISTENERS.lock().unwrap();
    for server in listeners.values_mut() {
        if server.proto() == host.proto && server.contains_addr(&host.peer_addr) {
            server.sendto(&host.peer_addr, buf)?;
        }
    }

    Ok(())
}

/// Snapshot of every bound listener.
pub fn all_listener() -> Vec<UIListener> {
    let listeners = G_LISTENERS.lock().unwrap();

    listeners
        .iter()
        .map(|(id, v)| UIListener {
            id: *id,
            addr: v.local_addr().unwrap(),
            protocol: v.proto(),
        })
        .collect()
}

/// Bind a new listener and return its id.
pub fn add_listener(proto: &HeroinnProtocol, port: u16) -> Result<u8> {
    let id = G_LISTENER_ID.load(Ordering::Relaxed);
    let server = Listener::bind(proto, port, Arc::new(cb_msg))?;

    G_LISTENERS.lock().unwrap().insert(id, server);
    G_LISTENER_ID.store(id.wrapping_add(1), Ordering::Relaxed);

    Ok(id)
}

/// Close and drop the listener `id`.
pub fn remove_listener(id: u8) -> Result<()> {
    let mut listeners = G_LISTENERS.lock().unwrap();

    match listeners.get_mut(&id) {
        Some(v) => {
            v.close();
            listeners.remove(&id);
            Ok(())
        }
        None => Err(Error::NotFound(format!("listener {}", id))),
    }
}

/// Snapshot of every connected client.
pub fn all_host() -> Vec<UIHostInfo> {
    G_ONLINE_HOSTS.lock().unwrap().values().cloned().collect()
}

/// Forget `clientid` and close everything it owns.
pub fn remove_host(clientid: String) {
    let mut hosts = G_ONLINE_HOSTS.lock().unwrap();

    if hosts.contains_key(&clientid) {
        G_SHELL_SESSION
            .lock()
            .unwrap()
            .close_by_clientid(&clientid);
        G_FTP_SESSION.lock().unwrap().close_by_clientid(&clientid);
        hosts.remove(&clientid);
    }
}

/// Look up the cached state of one client.
pub fn get_hostinfo_by_clientid(clientid: &str) -> Option<UIHostInfo> {
    G_ONLINE_HOSTS.lock().unwrap().get(clientid).cloned()
}

/// Start an interactive shell session against `clientid`.
pub fn open_shell(clientid: &str) -> Result<()> {
    let Some(info) = get_hostinfo_by_clientid(clientid) else {
        return Err(Error::NotFound(format!("client {}", clientid)));
    };

    let sender = G_SESSION_SENDER.lock().unwrap().clone();
    let session = ShellServer::new(sender, clientid, &format!("{}", info.peer_addr))?;

    log::info!("create shell session : {}", session.id());

    let data = SessionPacket {
        id: session.id().to_string(),
        data: vec![],
    };

    G_SHELL_SESSION.lock().unwrap().register(session);

    let data = Message::build(HeroinnServerCommandID::Shell.to_u8(), clientid, data)?;
    send_data_by_clientid(clientid, &data)
}

/// Start a file-transfer session against `clientid`.
pub fn open_ftp(clientid: &str) -> Result<()> {
    let Some(info) = get_hostinfo_by_clientid(clientid) else {
        return Err(Error::NotFound(format!("client {}", clientid)));
    };

    let sender = G_SESSION_SENDER.lock().unwrap().clone();
    let session = FtpServer::new(sender, clientid, &format!("{}", info.peer_addr))?;

    log::info!("create ftp session : {}", session.id());

    let data = SessionPacket {
        id: session.id().to_string(),
        data: vec![],
    };

    G_FTP_SESSION.lock().unwrap().register(session);

    let data = Message::build(HeroinnServerCommandID::File.to_u8(), clientid, data)?;
    send_data_by_clientid(clientid, &data)
}
