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
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Drop for RpcClient {
    fn drop(&mut self) {
        self.is_closed.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_call(params: Vec<String>) -> Result<Vec<String>> {
        Ok(params
            .into_iter()
            .map(|value| value.to_uppercase())
            .collect())
    }

    fn failed_call(_: Vec<String>) -> Result<Vec<String>> {
        Err(Error::Other("business failure".to_string()))
    }

    fn message(name: &str, data: Vec<String>) -> RpcMessage {
        RpcMessage {
            id: "request-id".to_string(),
            name: name.to_string(),
            retcode: 0,
            time: 1,
            msg: String::new(),
            data,
        }
    }

    #[test]
    fn rpc_message_round_trips_and_rejects_malformed_json() {
        let request = RpcMessage::build_call("echo", vec!["one".to_string()]);
        let encoded = request.serialize().unwrap();
        let decoded = RpcMessage::parse(&encoded).unwrap();

        assert_eq!(decoded.id, request.id);
        assert_eq!(decoded.name, "echo");
        assert_eq!(decoded.data, ["one"]);
        assert!(decoded.time > 0);
        assert!(UuidLike::valid(&decoded.id));
        assert!(matches!(
            RpcMessage::parse(b"not-json"),
            Err(Error::Json(_))
        ));
    }

    #[test]
    fn rpc_server_dispatches_success_and_preserves_request_identity() {
        let mut server = RpcServer::new();
        server.register("upper", ok_call);
        let request = message("upper", vec!["alpha".to_string(), "beta".to_string()]);

        let response = server.call(&request);

        assert_eq!(response.id, request.id);
        assert_eq!(response.name, request.name);
        assert_eq!(response.retcode, 0);
        assert_eq!(response.data, ["ALPHA", "BETA"]);
        assert!(response.msg.is_empty());
    }

    #[test]
    fn rpc_server_returns_stable_errors_for_unknown_and_failing_handlers() {
        let mut server = RpcServer::new();
        server.register("failed", failed_call);
        let unknown = server.call(&message("missing", vec![]));
        let failed = server.call(&message("failed", vec![]));

        assert_eq!(unknown.retcode, -1);
        assert_eq!(unknown.data, Vec::<String>::new());
        assert!(unknown.msg.contains("not found rpc [missing]"));
        assert_eq!(failed.retcode, -2);
        assert!(failed.msg.contains("business failure"));
    }

    #[test]
    fn rpc_client_records_and_returns_the_matching_response() {
        let client = RpcClient::new();
        let mut response = message("upper", vec!["value".to_string()]);
        response.id = "response-id".to_string();
        response.retcode = 7;

        client.write(&response);

        let received = client.wait_msg("response-id", 0).unwrap();
        assert_eq!(received.id, "response-id");
        assert_eq!(received.retcode, 7);
    }

    #[test]
    fn rpc_client_times_out_for_an_unknown_response() {
        let client = RpcClient::new();
        let start = std::time::Instant::now();
        let error = client.wait_msg("missing", 0).unwrap_err();
        assert!(matches!(error, Error::Timeout));
        assert!(start.elapsed() < std::time::Duration::from_secs(3));
    }

    #[test]
    fn poisoned_rpc_locks_remain_locked_out_safely() {
        let lock = Arc::new(RwLock::new(vec![1_u8]));
        let poison = lock.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poison.write().unwrap();
            panic!("intentional lock poison");
        })
        .join();

        assert_eq!(read(&lock).as_slice(), &[1]);
        write(&lock).push(2);
        assert_eq!(read(&lock).as_slice(), &[1, 2]);
    }

    struct UuidLike;

    impl UuidLike {
        fn valid(value: &str) -> bool {
            value.len() == 36
                && value
                    .as_bytes()
                    .iter()
                    .enumerate()
                    .all(|(index, byte)| matches!(index, 8 | 13 | 18 | 23) == (*byte == b'-'))
        }
    }
}
