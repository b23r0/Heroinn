use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::{
    io::*,
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};

use crate::cur_timestamp_secs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcMessage {
    pub id: String,
    pub name: String,
    pub retcode: i32,
    pub time: u64,
    pub msg: String,
    pub data: Vec<String>,
}

impl RpcMessage {
    pub fn parse(data: &Vec<u8>) -> Result<Self> {
        let ret: RpcMessage = serde_json::from_slice(data)?;
        Ok(ret)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match serde_json::to_vec(self) {
            Ok(p) => Ok(p),
            Err(_) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "serilize RpcMessage faild",
                ))
            }
        }
    }

    pub fn build_call(name: &str, data: Vec<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            retcode: 0,
            time: cur_timestamp_secs(),
            msg: String::new(),
            data,
        }
    }
}

pub type RpcType = fn(Vec<String>) -> Result<Vec<String>>;

pub struct RpcServer {
    funcs: HashMap<String, RpcType>,
}

pub struct RpcClient {
    wait_response_list: Arc<RwLock<HashMap<String, RpcMessage>>>,
    is_closed: Arc<AtomicBool>,
}

impl RpcServer {
    pub fn new() -> Self {
        RpcServer {
            funcs: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &String, func: RpcType) {
        self.funcs.insert(name.clone(), func);
    }

    pub fn call(&self, data: &RpcMessage) -> RpcMessage {
        if self.funcs.contains_key(&data.name) {
            let func = self.funcs.get(&data.name).unwrap();

            let param = match func(data.data.clone()) {
                Ok(p) => p,
                Err(e) => {
                    let ret = RpcMessage {
                        id: data.id.clone(),
                        name: data.name.clone(),
                        data: vec![],
                        time: cur_timestamp_secs(),
                        msg: format!("error : {}", e),
                        retcode: -2,
                    };

                    return ret;
                }
            };
            

            RpcMessage {
                id: data.id.clone(),
                name: data.name.clone(),
                data: param,
                time: cur_timestamp_secs(),
                msg: String::new(),
                retcode: 0,
            }
        } else {
            

            RpcMessage {
                id: data.id.clone(),
                name: data.name.clone(),
                data: vec![],
                time: cur_timestamp_secs(),
                msg: format!("not found rpc [{}]", data.name),
                retcode: -1,
            }
        }
    }
}

impl RpcClient {
    const RESPONSE_EXPIRED_SECS: u64 = 30;

    pub fn new() -> Self {
        let wait_response_list = Arc::new(RwLock::new(HashMap::new()));
        let is_closed = Arc::new(AtomicBool::new(false));
        let wait_response_list_1 = wait_response_list.clone();
        let is_closed_1 = is_closed.clone();
        std::thread::Builder::new()
            .name("rpc client gc worker".to_string())
            .spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(
                        RpcClient::RESPONSE_EXPIRED_SECS,
                    ));
                    if is_closed_1.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    wait_response_list_1.write().unwrap().retain(
                        |_: &String, v: &mut RpcMessage| {
                            cur_timestamp_secs() - v.time < RpcClient::RESPONSE_EXPIRED_SECS
                        },
                    );
                }
                log::info!("RpcClient exit");
            })
            .unwrap();
        Self {
            wait_response_list,
            is_closed,
        }
    }

    pub fn write(&self, msg: &RpcMessage) {
        let mut v = msg.clone();
        v.time = cur_timestamp_secs();

        self.wait_response_list
            .write()
            .unwrap()
            .insert(msg.id.clone(), v);
    }

    pub fn wait_msg(&self, id: &String, timeout_secs: u64) -> Result<RpcMessage> {
        let cur_time = cur_timestamp_secs();
        loop {
            {
                let lock = self.wait_response_list.read().unwrap();
                if lock.contains_key(id) {
                    break;
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
            if cur_timestamp_secs() - cur_time > timeout_secs {
                return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"));
            }
        }

        let ret = match self.wait_response_list.read().unwrap().get(id) {
            Some(p) => p.clone(),
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                ))
            }
        };

        Ok(ret)
    }
}

impl Drop for RpcClient {
    fn drop(&mut self) {
        self.is_closed
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
