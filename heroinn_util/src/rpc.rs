//! Minimal request/response RPC used to drive file-system operations remotely.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use serde::{Deserialize, Serialize};

use crate::cur_timestamp_secs;
use crate::error::{Error, Result};

/// A single RPC request or response.
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
    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Build an outgoing call to `name` with `data` as its arguments.
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

/// A synchronous, locally registered RPC implementation.
pub type RpcType = fn(Vec<String>) -> Result<Vec<String>>;

/// Dispatches incoming calls to registered [`RpcType`] functions.
pub struct RpcServer {
    funcs: HashMap<String, RpcType>,
}

/// Tracks outstanding calls until their responses arrive.
pub struct RpcClient {
    wait_response_list: Arc<RwLock<HashMap<String, RpcMessage>>>,
    is_closed: Arc<AtomicBool>,
}

impl RpcServer {
    pub fn new() -> Self {
        Self {
            funcs: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, func: RpcType) {
        self.funcs.insert(name.to_string(), func);
    }

    /// Execute `data` and wrap the outcome in a response message.
    pub fn call(&self, data: &RpcMessage) -> RpcMessage {
        let func = match self.funcs.get(&data.name) {
            Some(f) => f,
            None => return error_response(data, -1, format!("not found rpc [{}]", data.name)),
        };

        match func(data.data.clone()) {
            Ok(param) => RpcMessage {
                id: data.id.clone(),
                name: data.name.clone(),
                data: param,
                time: cur_timestamp_secs(),
                msg: String::new(),
                retcode: 0,
            },
            Err(e) => error_response(data, -2, format!("error : {}", e)),
        }
    }
}

impl Default for RpcServer {
    fn default() -> Self {
        Self::new()
    }
}

fn error_response(data: &RpcMessage, retcode: i32, msg: String) -> RpcMessage {
    RpcMessage {
        id: data.id.clone(),
        name: data.name.clone(),
        data: vec![],
        time: cur_timestamp_secs(),
        msg,
        retcode,
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
                    if is_closed_1.load(Ordering::Relaxed) {
                        break;
                    }
                    write(&wait_response_list_1).retain(|_: &String, v: &mut RpcMessage| {
                        cur_timestamp_secs() - v.time < RpcClient::RESPONSE_EXPIRED_SECS
                    });
                }
                log::info!("RpcClient exit");
            })
            .unwrap();

        Self {
            wait_response_list,
            is_closed,
        }
    }

    /// Record an outstanding call so [`Self::wait_msg`] can pick up its answer.
    pub fn write(&self, msg: &RpcMessage) {
        let mut v = msg.clone();
        v.time = cur_timestamp_secs();

        write(&self.wait_response_list).insert(msg.id.clone(), v);
    }

    /// Block until the response for `id` arrives, up to `timeout_secs`.
    pub fn wait_msg(&self, id: &str, timeout_secs: u64) -> Result<RpcMessage> {
        let start = cur_timestamp_secs();

        loop {
            if read(&self.wait_response_list).contains_key(id) {
                break;
            }

            if cur_timestamp_secs() - start > timeout_secs {
                log::warn!("rpc [{}] timed out", id);
                return Err(Error::Timeout);
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        read(&self.wait_response_list)
            .get(id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("rpc response {}", id)))
    }
}

impl Default for RpcClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Read a lock, recovering the guard when a previous holder panicked.
fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Write a lock, recovering the guard when a previous holder panicked.
fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Drop for RpcClient {
    fn drop(&mut self) {
        self.is_closed.store(true, Ordering::Relaxed);
    }
}
