use std::{sync::mpsc::channel, time::Duration};
use uuid::Uuid;
use heroinn_util::{protocol::{tcp::{TcpConnection}, Client}, packet::{Message, HostInfo, Heartbeat}, HeroinnClientMsgID, cur_timestamp_secs};
use simple_logger::SimpleLogger;
use log::LevelFilter;

fn main() {

    SimpleLogger::new().with_utc_timestamps().with_utc_timestamps().with_colors(true).init().unwrap();
	::log::set_max_level(LevelFilter::Info);

    let clientid = Uuid::new_v4().to_string();

    loop{
        let mut client = match TcpConnection::connect("127.0.0.1:8000"){
            Ok(p) => p,
            Err(e) => {
                log::info!("connect faild : {}" , e);
                std::thread::sleep(Duration::from_secs(5));
                continue;
            },
        };

        log::info!("connect success!");
        
        let hostinfo = HostInfo{
            ip: "127.0.0.1".to_string(),
            host_name: "Z2CLL6T3K50D7JL".to_string(),
            os: "Windows 11".to_string(),
            remark: "test remark".to_string(),
        };

        let mut buf = match Message::make(HeroinnClientMsgID::HostInfo.to_u8(), &clientid, hostinfo){
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

                match client_1.send(&mut buf){
                    Ok(p) => p,
                    Err(e) => {
                        log::info!("sender channel closed : {}" , e);
                        break;
                    },
                };
                log::info!("id : {} send [{}] bytes", buf[0] , buf.len());
            }
            
        });

        let mut client_2 = client.clone();
        let clientid = clientid.clone();
        std::thread::spawn(move || {
            loop{
                let heartbeat = Heartbeat{
                    time: cur_timestamp_secs(),
                };
        
                let buf = match Message::make(HeroinnClientMsgID::Heartbeat.to_u8(), &clientid, heartbeat){
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("make Heartbeat packet faild : {}" ,e);
                        break;
                    },
                };

                match sender.send(buf){
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("send Heartbeat packet to channel faild : {}" ,e);
                        break;
                    },
                };

                std::thread::sleep(Duration::from_secs(5));
            }
            client_2.close();
        });

        loop{

            match client.recv(){
                Ok(buf) => {
                    log::info!("recv [{}] bytes" , buf.len());
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
