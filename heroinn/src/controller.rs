use std::{io::*, collections::HashMap, sync::{Mutex, atomic::{AtomicU8, Ordering}}, net::SocketAddr};

use heroinn_core::HeroinnServer;
use lazy_static::*;
use heroinn_util::{packet::{HostInfo, Message}, HeroinnProtocol, cur_timestamp_millis, HeroinnClientMsgID};

#[derive(Clone)]
pub struct UIHostInfo{
    up_trans_rate : u64,
    down_trans_rate : u64,
    last_heartbeat : u64,
    info : HostInfo
}

#[derive(Clone)]
pub struct UIListener{
    pub id : u8,
    pub protocol : HeroinnProtocol,
    pub addr : SocketAddr
}


lazy_static!{
    static ref  G_ONLINE_HOSTS : Mutex<HashMap<String , UIHostInfo>> = Mutex::new(HashMap::new());
    static ref  G_LISTENERS : Mutex<HashMap<u8 , HeroinnServer>> = Mutex::new(HashMap::new());

    static ref LISTENER_ID : AtomicU8 = AtomicU8::new(0);
}

pub fn cb_msg(msg : Message){

    match HeroinnClientMsgID::from(msg.id()){
        HeroinnClientMsgID::HostInfo => {

            let mut hosts = G_ONLINE_HOSTS.lock().unwrap();

            if hosts.contains_key(&msg.clientid()){
                let v = hosts.get_mut(&msg.clientid()).unwrap();
                *v = UIHostInfo{ up_trans_rate: 0, down_trans_rate: 0, last_heartbeat: cur_timestamp_millis(), info: msg.to_host_info().unwrap() };
            } else {
                hosts.insert(msg.clientid() ,UIHostInfo{ up_trans_rate: 0, down_trans_rate: 0, last_heartbeat: cur_timestamp_millis(), info: msg.to_host_info().unwrap() } );
            }
        },
        HeroinnClientMsgID::Heartbeat => {

        },
        HeroinnClientMsgID::Unknow => {

        },
    }
}

pub fn all_listener() -> Vec<UIListener>{
    let mut ret : Vec<UIListener> = vec![];
    let mut listeners = G_LISTENERS.lock().unwrap();

    for k in listeners.keys(){
        if let Some(v) = listeners.get(k){
            ret.push(UIListener { id : *k ,addr : v.local_addr().unwrap(), protocol: v.proto() });
        }
    }

    ret
}

pub fn add_listener(proto : &HeroinnProtocol, port : u16) -> Result<u8>{

    let id = LISTENER_ID.load(Ordering::Relaxed);

    let server = HeroinnServer::new(proto.clone(), port, cb_msg)?;
    G_LISTENERS.lock().unwrap().insert(LISTENER_ID.load(Ordering::Relaxed), server);
    LISTENER_ID.store(id +1 ,Ordering::Relaxed);
    Ok(id)
}

pub fn remove_listener(id : u8){
    let mut listener = G_LISTENERS.lock().unwrap();

    if listener.contains_key(&id){
        let v = listener.get_mut(&id).unwrap();
        v.close();
        listener.remove(&id);
    }
}