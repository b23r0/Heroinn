use std::{io::*, collections::HashMap, sync::{Mutex, atomic::{AtomicU8, Ordering}}, net::SocketAddr};

use heroinn_core::HeroinnServer;
use heroinn_util::{packet::*, *};
use lazy_static::*;

#[derive(Clone)]
pub struct UIHostInfo{
    pub clientid : String,
    pub peer_addr : SocketAddr,
    pub proto : HeroinnProtocol,
    pub up_trans_rate : u64,
    pub down_trans_rate : u64,
    pub last_heartbeat : u64,
    pub info : HostInfo
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

    let mut hosts = G_ONLINE_HOSTS.lock().unwrap();

    match HeroinnClientMsgID::from(msg.id()){
        HeroinnClientMsgID::HostInfo => {
            log::info!("hostinfo : {}" , msg.clientid());
            if let std::collections::hash_map::Entry::Vacant(e) = hosts.entry(msg.clientid()) {
                e.insert(UIHostInfo{clientid : msg.clientid() , peer_addr : msg.peer_addr(), proto : msg.proto() , up_trans_rate: 0, down_trans_rate: 0, last_heartbeat: cur_timestamp_secs(), info: msg.parser().unwrap() });
            } else {
                let v = hosts.get_mut(&msg.clientid()).unwrap();
                *v = UIHostInfo{ clientid : msg.clientid() , peer_addr : msg.peer_addr(), proto : msg.proto() , up_trans_rate: 0, down_trans_rate: 0, last_heartbeat: cur_timestamp_secs(), info: msg.parser().unwrap() };
            }
        },
        HeroinnClientMsgID::Heartbeat => {
            log::info!("heartbeat : {}" , msg.clientid());
            if hosts.contains_key(&msg.clientid()){
                let v = hosts.get_mut(&msg.clientid()).unwrap();
                v.last_heartbeat = cur_timestamp_secs();
            }
        },
        HeroinnClientMsgID::Unknow => {

        },
    }
}

pub fn all_listener() -> Vec<UIListener>{
    let mut ret : Vec<UIListener> = vec![];
    let listeners = G_LISTENERS.lock().unwrap();

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

pub fn all_host() -> Vec<UIHostInfo>{
    let mut ret : Vec<UIHostInfo> = vec![];

    let hosts = G_ONLINE_HOSTS.lock().unwrap();

    for k in hosts.keys(){
        if let Some(v) = hosts.get(k){
            ret.push(v.clone());
        }
    }

    ret
}

pub fn remove_host(clientid : String){
    let mut host = G_ONLINE_HOSTS.lock().unwrap();

    if host.contains_key(&clientid){
        host.remove(&clientid);
    }
}