//! Server state shared by the GUI: online hosts, listeners and sessions.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use heroinn_core::module::ftp::FtpServer;
use heroinn_core::module::shell::ShellServer;
use heroinn_core::module::socks5::{Socks5Server, locate_sidecar};
use heroinn_util::session::{ServerSession, Session, SessionBase, SessionManager, SessionPacket};
use heroinn_util::socks5::Socks5ServerCommand;
use heroinn_util::{
    DEFAULT_TRANSFER_RATE, Error, HEART_BEAT_TIME, HeroinnClientMsgID, HeroinnProtocol,
    HeroinnServerCommandID, Result, cur_timestamp_secs,
    packet::{HostInfo, Message, TransferRate},
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
    pub transfer_rate: u64,
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
static G_SOCKS5_SESSION: LazyLock<Mutex<SessionManager<Socks5Server>>> =
    LazyLock::new(|| Mutex::new(SessionManager::new()));
static G_SOCKS5_STARTING: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
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

            let mut session = G_SOCKS5_SESSION.lock().unwrap();
            session.gc();
            log::info!("socks5 session : {}", session.count());
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

            let transfer_rate = {
                let mut hosts = G_ONLINE_HOSTS.lock().unwrap();
                let transfer_rate = hosts
                    .get(msg.clientid())
                    .map(|host| host.transfer_rate)
                    .unwrap_or(DEFAULT_TRANSFER_RATE);

                hosts.insert(
                    msg.clientid().to_string(),
                    UIHostInfo {
                        clientid: msg.clientid().to_string(),
                        peer_addr: msg.peer_addr(),
                        proto: msg.proto(),
                        in_rate: 0,
                        out_rate: msg.length() as u64,
                        transfer_rate,
                        last_heartbeat: cur_timestamp_secs(),
                        info,
                    },
                );

                transfer_rate
            };

            if let Err(e) = send_transfer_rate(msg.clientid(), transfer_rate) {
                log::warn!(
                    "failed to apply transfer rate to {} : {}",
                    msg.clientid(),
                    e
                );
            }
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
        Ok(HeroinnClientMsgID::Socks5) => {
            let Ok(event) = msg.parse_socks5_client_event() else {
                log::warn!("malformed SOCKS5 event from {}", msg.clientid());
                return;
            };
            let session_id = event.session_id().to_string();
            let Ok(data) = serde_json::to_vec(&event) else {
                return;
            };
            let result = {
                let mut sessions = G_SOCKS5_SESSION.lock().unwrap();
                if !sessions.contains(&session_id) {
                    log::warn!("SOCKS5 event for unknown session {}", session_id);
                    return;
                }
                sessions.write(&session_id, &data)
            };
            if let Err(error) = result {
                log::warn!("SOCKS5 session {} write failed: {}", session_id, error);
                G_SOCKS5_SESSION.lock().unwrap().close(&session_id);
            }
        }
        Err(e) => log::warn!("unknown packet id : {}", e),
    }
}

fn on_client_disconnected(peer_addr: SocketAddr, proto: HeroinnProtocol) {
    let clientid = {
        let hosts = G_ONLINE_HOSTS.lock().unwrap();
        hosts
            .values()
            .find(|host| host.peer_addr == peer_addr && host.proto == proto)
            .map(|host| host.clientid.clone())
    };
    if let Some(clientid) = clientid {
        log::info!("transport closed for {clientid} ({proto:?} {peer_addr})");
        remove_host(clientid);
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

fn send_transfer_rate(clientid: &str, bytes_per_sec: u64) -> Result<()> {
    let data = Message::build(
        HeroinnServerCommandID::SetTransferRate.to_u8(),
        clientid,
        TransferRate { bytes_per_sec },
    )?;

    send_data_by_clientid(clientid, &data)
}

/// Update and push the aggregate transfer-rate limit for `clientid`.
pub fn set_transfer_rate(clientid: &str, bytes_per_sec: u64) -> Result<()> {
    if bytes_per_sec == 0 {
        return Err(Error::Other(
            "transfer rate must be greater than zero".to_string(),
        ));
    }

    if get_hostinfo_by_clientid(clientid).is_none() {
        return Err(Error::NotFound(format!("client {}", clientid)));
    }

    send_transfer_rate(clientid, bytes_per_sec)?;

    if let Some(host) = G_ONLINE_HOSTS.lock().unwrap().get_mut(clientid) {
        host.transfer_rate = bytes_per_sec;
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
    let server = Listener::bind_with_disconnect(
        proto,
        port,
        Arc::new(cb_msg),
        Arc::new(on_client_disconnected),
    )?;

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
    let existed = G_ONLINE_HOSTS.lock().unwrap().remove(&clientid).is_some();

    if existed {
        G_SHELL_SESSION.lock().unwrap().close_by_clientid(&clientid);
        G_FTP_SESSION.lock().unwrap().close_by_clientid(&clientid);
        G_SOCKS5_SESSION
            .lock()
            .unwrap()
            .close_by_clientid(&clientid);
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

/// Whether this host and the current server installation can launch the
/// reverse-SOCKS5 sidecar.
pub fn socks5_launch_status(info: &UIHostInfo) -> std::result::Result<(), String> {
    if !cfg!(any(target_os = "linux", target_os = "windows")) {
        return Err("SOCKS5 reverse proxy is supported on Linux and Windows only".to_string());
    }
    if !info
        .info
        .capabilities
        .as_ref()
        .is_some_and(|capabilities| capabilities.socks5_reverse)
    {
        return Err("the connected client does not advertise reverse SOCKS5 support".to_string());
    }
    locate_sidecar().map_err(|error| error.to_string())?;
    Ok(())
}

fn socks5_chain_label(info: &UIHostInfo) -> String {
    if info.proto == HeroinnProtocol::UDP {
        return "UDP direct / proxy chain not applicable".to_string();
    }
    let system = match info.info.system_proxy.as_ref() {
        Some(proxy) if proxy.protocol == "Bypassed" => "Bypassed".to_string(),
        Some(proxy) if !proxy.address.is_empty() => {
            format!("{} {}", proxy.protocol, proxy.address)
        }
        Some(proxy) => proxy.protocol.clone(),
        None => "Direct".to_string(),
    };
    match info.info.socks5_proxy.as_ref() {
        Some(proxy) if proxy.protocol != "Disabled" => {
            let target = if proxy.address.is_empty() {
                proxy.protocol.clone()
            } else {
                format!("{} {}", proxy.protocol, proxy.address)
            };
            format!("{system} -> {target}")
        }
        _ => system,
    }
}

fn send_socks5_command(clientid: &str, command: Socks5ServerCommand) -> Result<()> {
    let data = Message::build(HeroinnServerCommandID::Socks5.to_u8(), clientid, command)?;
    send_data_by_clientid(clientid, &data)
}

/// Open one reverse-SOCKS5 GUI sidecar for `clientid`.
pub fn open_socks5(clientid: &str) -> Result<()> {
    if !G_SOCKS5_STARTING
        .lock()
        .unwrap()
        .insert(clientid.to_string())
    {
        return Err(Error::Other(
            "the reverse SOCKS5 proxy is already starting for this client".to_string(),
        ));
    }

    let result = open_socks5_inner(clientid);
    G_SOCKS5_STARTING.lock().unwrap().remove(clientid);
    result
}

fn open_socks5_inner(clientid: &str) -> Result<()> {
    let Some(info) = get_hostinfo_by_clientid(clientid) else {
        return Err(Error::NotFound(format!("client {}", clientid)));
    };
    socks5_launch_status(&info).map_err(Error::Other)?;

    {
        let sessions = G_SOCKS5_SESSION.lock().unwrap();
        if sessions.contains_clientid(clientid) {
            return Err(Error::Other(
                "a reverse SOCKS5 proxy is already open for this client".to_string(),
            ));
        }
    }

    let transport = format!("{:?}", info.proto);
    let proxy_chain = socks5_chain_label(&info);
    let sender: heroinn_core::module::socks5::ServerCommandSender =
        Arc::new(|target_client, command| {
            if let Err(error) = send_socks5_command(target_client, command) {
                log::warn!("send SOCKS5 command to {target_client} failed: {error}");
            }
        });
    let session = Socks5Server::new(
        clientid,
        &info.peer_addr.to_string(),
        &proxy_chain,
        &transport,
        sender,
    )?;
    if get_hostinfo_by_clientid(clientid).is_none() {
        return Err(Error::NotFound(format!(
            "client {clientid} disconnected while starting the SOCKS5 sidecar"
        )));
    }
    let session_id = session.id().to_string();
    G_SOCKS5_SESSION.lock().unwrap().register(session);
    if let Err(error) = send_socks5_command(
        clientid,
        Socks5ServerCommand::Start {
            session_id: session_id.clone(),
        },
    ) {
        G_SOCKS5_SESSION.lock().unwrap().close(&session_id);
        return Err(error);
    }
    log::info!("create SOCKS5 session {session_id} for {clientid}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::sync::MutexGuard;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn serial() -> MutexGuard<'static, ()> {
        TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn host_info(clientid: &str) -> HostInfo {
        HostInfo {
            ip: "127.0.0.1".to_string(),
            host_name: format!("host-{clientid}"),
            os: "linux".to_string(),
            platform_id: Some("linux".to_string()),
            whoami: "tester".to_string(),
            remark: clientid.to_string(),
            system_proxy: None,
            socks5_proxy: None,
            capabilities: Some(heroinn_util::packet::HostCapabilities {
                socks5_reverse: true,
            }),
        }
    }

    fn ui_host(
        clientid: &str,
        proto: HeroinnProtocol,
        system_proxy: Option<heroinn_util::packet::ProxyDisplay>,
        socks5_proxy: Option<heroinn_util::packet::ProxyDisplay>,
    ) -> UIHostInfo {
        UIHostInfo {
            clientid: clientid.to_string(),
            peer_addr: "127.0.0.1:39000".parse().unwrap(),
            proto,
            in_rate: 0,
            out_rate: 0,
            transfer_rate: DEFAULT_TRANSFER_RATE,
            last_heartbeat: 0,
            info: HostInfo {
                system_proxy,
                socks5_proxy,
                ..host_info(clientid)
            },
        }
    }

    fn hostinfo_message(clientid: &str, info: HostInfo) -> Message {
        hostinfo_message_at(clientid, info, 39000)
    }

    fn hostinfo_message_at(clientid: &str, info: HostInfo, port: u16) -> Message {
        let bytes = Message::build(HeroinnClientMsgID::HostInfo.to_u8(), clientid, info).unwrap();
        Message::new(
            format!("127.0.0.1:{port}").parse().unwrap(),
            HeroinnProtocol::TCP,
            &bytes,
        )
        .unwrap()
    }

    #[test]
    fn hostinfo_upsert_preserves_transfer_rate_and_heartbeat_updates_rates() {
        let _guard = serial();
        let clientid = "controller-upsert";
        remove_host(clientid.to_string());

        cb_msg(hostinfo_message(clientid, host_info(clientid)));
        set_transfer_rate(clientid, 4096).unwrap();
        cb_msg(hostinfo_message(clientid, host_info(clientid)));
        let stored = get_hostinfo_by_clientid(clientid).unwrap();
        assert_eq!(stored.transfer_rate, 4096);

        let heartbeat = heroinn_util::packet::Heartbeat {
            time: 1,
            in_rate: 12,
            out_rate: 34,
        };
        let bytes =
            Message::build(HeroinnClientMsgID::Heartbeat.to_u8(), clientid, heartbeat).unwrap();
        cb_msg(
            Message::new(
                "127.0.0.1:39000".parse().unwrap(),
                HeroinnProtocol::TCP,
                &bytes,
            )
            .unwrap(),
        );
        let stored = get_hostinfo_by_clientid(clientid).unwrap();
        assert_eq!(stored.in_rate, 12);
        assert_eq!(stored.out_rate, 34);
        assert!(stored.last_heartbeat > 0);
        remove_host(clientid.to_string());
    }

    #[test]
    fn heartbeat_for_unknown_host_has_no_side_effect() {
        let _guard = serial();
        let clientid = "controller-unknown-heartbeat";
        remove_host(clientid.to_string());
        let bytes = Message::build(
            HeroinnClientMsgID::Heartbeat.to_u8(),
            clientid,
            heroinn_util::packet::Heartbeat {
                time: 1,
                in_rate: 1,
                out_rate: 1,
            },
        )
        .unwrap();
        cb_msg(
            Message::new(
                "127.0.0.1:39001".parse().unwrap(),
                HeroinnProtocol::TCP,
                &bytes,
            )
            .unwrap(),
        );
        assert!(get_hostinfo_by_clientid(clientid).is_none());
    }

    #[test]
    fn malformed_and_unknown_messages_do_not_create_hosts() {
        let _guard = serial();
        let clientid = "controller-malformed";
        remove_host(clientid.to_string());
        let malformed = Message::build(
            HeroinnClientMsgID::HostInfo.to_u8(),
            clientid,
            "not-a-host-info",
        )
        .unwrap();
        cb_msg(
            Message::new(
                "127.0.0.1:39002".parse().unwrap(),
                HeroinnProtocol::TCP,
                &malformed,
            )
            .unwrap(),
        );
        let unknown = Message::build(0xfe, clientid, "ignored").unwrap();
        cb_msg(
            Message::new(
                "127.0.0.1:39002".parse().unwrap(),
                HeroinnProtocol::TCP,
                &unknown,
            )
            .unwrap(),
        );
        assert!(get_hostinfo_by_clientid(clientid).is_none());
    }

    #[test]
    fn disconnect_removes_only_the_matching_endpoint_and_closes_once() {
        let _guard = serial();
        let first = "controller-disconnect-first";
        let second = "controller-disconnect-second";
        remove_host(first.to_string());
        remove_host(second.to_string());
        cb_msg(hostinfo_message_at(first, host_info(first), 39000));
        cb_msg(hostinfo_message_at(second, host_info(second), 39001));

        on_client_disconnected("127.0.0.1:39000".parse().unwrap(), HeroinnProtocol::TCP);
        on_client_disconnected("127.0.0.1:39000".parse().unwrap(), HeroinnProtocol::TCP);

        assert!(get_hostinfo_by_clientid(first).is_none());
        assert!(get_hostinfo_by_clientid(second).is_some());
        remove_host(second.to_string());
    }

    #[test]
    fn transfer_rate_rejects_zero_missing_hosts_and_preserves_last_value() {
        let _guard = serial();
        let clientid = "controller-rate";
        remove_host(clientid.to_string());
        assert!(matches!(
            set_transfer_rate(clientid, 0),
            Err(Error::Other(message)) if message.contains("greater than zero")
        ));
        assert!(matches!(
            set_transfer_rate(clientid, 1024),
            Err(Error::NotFound(message)) if message.contains(clientid)
        ));
        cb_msg(hostinfo_message(clientid, host_info(clientid)));
        set_transfer_rate(clientid, 2048).unwrap();
        assert_eq!(
            get_hostinfo_by_clientid(clientid).unwrap().transfer_rate,
            2048
        );
        remove_host(clientid.to_string());
    }

    #[test]
    fn opening_sessions_or_socks_for_unknown_clients_is_a_clean_error() {
        let _guard = serial();
        let clientid = "controller-no-client";
        remove_host(clientid.to_string());
        assert!(matches!(open_shell(clientid), Err(Error::NotFound(_))));
        assert!(matches!(open_ftp(clientid), Err(Error::NotFound(_))));
        assert!(matches!(open_socks5(clientid), Err(Error::NotFound(_))));
        assert!(!G_SOCKS5_STARTING.lock().unwrap().contains(clientid));
    }

    #[test]
    fn duplicate_socks_start_is_rejected_and_the_guard_is_released() {
        let _guard = serial();
        let clientid = "controller-socks-starting";
        G_SOCKS5_STARTING
            .lock()
            .unwrap()
            .insert(clientid.to_string());
        let error = open_socks5(clientid).unwrap_err();
        assert!(error.to_string().contains("already starting"));
        G_SOCKS5_STARTING.lock().unwrap().remove(clientid);
        assert!(!G_SOCKS5_STARTING.lock().unwrap().contains(clientid));
    }

    #[test]
    fn launch_status_rejects_clients_without_the_reverse_capability() {
        let mut host = ui_host("controller-capability", HeroinnProtocol::TCP, None, None);
        host.info.capabilities = None;
        let error = socks5_launch_status(&host).unwrap_err();
        assert!(error.contains("does not advertise reverse SOCKS5"));
        host.info.capabilities = Some(heroinn_util::packet::HostCapabilities {
            socks5_reverse: false,
        });
        assert!(socks5_launch_status(&host).is_err());
    }

    #[test]
    fn chain_label_is_redacted_complete_and_explicit_for_udp() {
        let display = |protocol: &str, address: &str| heroinn_util::packet::ProxyDisplay {
            protocol: protocol.to_string(),
            address: address.to_string(),
            source: None,
        };

        let udp = ui_host("chain-udp", HeroinnProtocol::UDP, None, None);
        assert_eq!(
            socks5_chain_label(&udp),
            "UDP direct / proxy chain not applicable"
        );

        let direct = ui_host("chain-direct", HeroinnProtocol::TCP, None, None);
        assert_eq!(socks5_chain_label(&direct), "Direct");
        let bypass = ui_host(
            "chain-bypass",
            HeroinnProtocol::HTTP,
            Some(display("Bypassed", "ignored.example:1")),
            Some(display("Disabled", "")),
        );
        assert_eq!(socks5_chain_label(&bypass), "Bypassed");
        let layered = ui_host(
            "chain-layered",
            HeroinnProtocol::TCP,
            Some(display("HTTPS", "system.example:8443")),
            Some(display("SOCKS5", "specified.example:1080")),
        );
        assert_eq!(
            socks5_chain_label(&layered),
            "HTTPS system.example:8443 -> SOCKS5 specified.example:1080"
        );
        let protocol_only = ui_host(
            "chain-protocol",
            HeroinnProtocol::TCP,
            Some(display("HTTP", "")),
            Some(display("SOCKS5", "")),
        );
        assert_eq!(socks5_chain_label(&protocol_only), "HTTP -> SOCKS5");
    }

    #[test]
    fn listeners_get_ids_are_visible_and_remove_is_idempotent() {
        let _guard = serial();
        let id = add_listener(&HeroinnProtocol::TCP, 0).unwrap();
        let listener = all_listener().into_iter().find(|item| item.id == id);
        assert!(listener.is_some());
        assert_eq!(listener.unwrap().protocol, HeroinnProtocol::TCP);
        remove_listener(id).unwrap();
        assert!(matches!(remove_listener(id), Err(Error::NotFound(_))));
    }

    #[test]
    fn sending_to_an_unknown_client_is_a_noop() {
        let _guard = serial();
        send_data_by_clientid("controller-unknown-send", b"payload").unwrap();
    }

    #[test]
    fn a_bound_listener_can_send_to_a_known_host() {
        let _guard = serial();
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = probe.local_addr().unwrap();
        drop(probe);
        let id = add_listener(&HeroinnProtocol::TCP, addr.port()).unwrap();
        let clientid = "controller-live-send";
        remove_host(clientid.to_string());
        let host = host_info(clientid);
        let bytes = Message::build(HeroinnClientMsgID::HostInfo.to_u8(), clientid, host).unwrap();
        let mut client = TcpStream::connect(addr).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        heroinn_util::protocol::frame::write_frame(&mut client, &bytes).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while get_hostinfo_by_clientid(clientid).is_none() && std::time::Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert!(get_hostinfo_by_clientid(clientid).is_some());
        send_data_by_clientid(clientid, b"payload").unwrap();
        use std::io::Read;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let payload = loop {
            let mut response = [0u8; 4];
            client.read_exact(&mut response).unwrap();
            let size = u32::from_be_bytes(response) as usize;
            let mut payload = vec![0u8; size];
            client.read_exact(&mut payload).unwrap();
            if payload == b"payload" {
                break payload;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "payload was not observed before the deadline"
            );
        };
        assert_eq!(payload, b"payload");
        remove_host(clientid.to_string());
        remove_listener(id).unwrap();
        drop(client);
    }
}
