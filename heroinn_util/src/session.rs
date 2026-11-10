//! Session lifecycle shared by shells and file-transfer channels.
//!
//! A "session" is one logical interactive channel between the server and a
//! client. Both sides keep a [`SessionManager`] of live sessions keyed by the
//! session id that travels on the wire; the two sides only differ in *how* a
//! session is created, which is why construction lives behind [`ServerSession`]
//! and [`ClientSession`] rather than on [`Session`] itself.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::mpsc::Sender;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// A packet routed to (or from) a single session.
pub struct SessionBase {
    pub id: String,
    pub clientid: String,
    pub packet: SessionPacket,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionPacket {
    pub id: String,
    pub data: Vec<u8>,
}

/// Behaviour every session exposes, independent of which side created it.
pub trait Session {
    fn id(&self) -> &str;
    fn clientid(&self) -> &str;
    fn write(&mut self, data: &[u8]) -> Result<()>;
    fn alive(&self) -> bool;
    fn close(&mut self);
}

/// A session created on the server side, backed by a local worker process.
pub trait ServerSession: Session {
    fn new(sender: Sender<SessionBase>, clientid: &str, peer_addr: &str) -> Result<Self>
    where
        Self: Sized;
}

/// A session created on the client side, backed by a local shell or file tree.
pub trait ClientSession: Session {
    fn new_client(sender: Sender<SessionBase>, clientid: &str, id: &str) -> Result<Self>
    where
        Self: Sized;
}

/// Owns the live sessions of one kind and drives their lifecycle.
pub struct SessionManager<T> {
    sessions: HashMap<String, T>,
}

impl<T: Session> Default for SessionManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Session> SessionManager<T> {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Insert a session unless one with the same id is already tracked.
    pub fn register(&mut self, session: T) {
        if let Entry::Vacant(entry) = self.sessions.entry(session.id().to_string()) {
            entry.insert(session);
        }
    }

    /// Forward `data` to the session `id`, if it is still tracked.
    pub fn write(&mut self, id: &str, data: &[u8]) -> Result<()> {
        if let Some(session) = self.sessions.get_mut(id) {
            log::debug!("found session : {}", id);
            session.write(data)?;
        }
        Ok(())
    }

    pub fn contains(&self, id: &str) -> bool {
        self.sessions.contains_key(id)
    }

    pub fn contains_clientid(&self, clientid: &str) -> bool {
        self.sessions
            .values()
            .any(|session| session.clientid() == clientid && session.alive())
    }

    /// Close and remove one session after a write failure.
    pub fn close(&mut self, id: &str) {
        if let Some(mut session) = self.sessions.remove(id) {
            session.close();
        }
    }

    /// Close and drop every session belonging to `clientid`.
    pub fn close_by_clientid(&mut self, clientid: &str) {
        self.close_matching(|session| session.clientid() == clientid);
    }

    /// Close and drop every session that is no longer alive.
    pub fn gc(&mut self) {
        self.close_matching(|session| !session.alive());
    }

    /// Close and drop all sessions.
    pub fn close_all(&mut self) {
        for session in self.sessions.values_mut() {
            session.close();
        }
        self.sessions.clear();
    }

    pub fn count(&self) -> usize {
        self.sessions.len()
    }

    fn close_matching(&mut self, mut should_close: impl FnMut(&T) -> bool) {
        self.sessions.retain(|_, session| {
            if should_close(session) {
                session.close();
                false
            } else {
                true
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use super::*;
    use crate::error::Error;

    struct FakeSession {
        id: String,
        clientid: String,
        alive: Arc<AtomicBool>,
        writes: Arc<AtomicUsize>,
        closes: Arc<AtomicUsize>,
        fail_write: bool,
    }

    impl FakeSession {
        fn new(id: &str, clientid: &str) -> Self {
            Self {
                id: id.to_string(),
                clientid: clientid.to_string(),
                alive: Arc::new(AtomicBool::new(true)),
                writes: Arc::new(AtomicUsize::new(0)),
                closes: Arc::new(AtomicUsize::new(0)),
                fail_write: false,
            }
        }

        fn tracked(&self) -> (Arc<AtomicBool>, Arc<AtomicUsize>, Arc<AtomicUsize>) {
            (self.alive.clone(), self.writes.clone(), self.closes.clone())
        }
    }

    impl Session for FakeSession {
        fn id(&self) -> &str {
            &self.id
        }

        fn clientid(&self) -> &str {
            &self.clientid
        }

        fn write(&mut self, data: &[u8]) -> Result<()> {
            self.writes.fetch_add(1, Ordering::SeqCst);
            if self.fail_write {
                return Err(Error::Other("write failed".to_string()));
            }
            assert!(!data.is_empty());
            Ok(())
        }

        fn alive(&self) -> bool {
            self.alive.load(Ordering::SeqCst)
        }

        fn close(&mut self) {
            self.closes.fetch_add(1, Ordering::SeqCst);
            self.alive.store(false, Ordering::SeqCst);
        }
    }

    #[test]
    fn duplicate_registration_keeps_the_first_session() {
        let mut sessions = SessionManager::new();
        let first = FakeSession::new("session", "client");
        let second = FakeSession::new("session", "client");
        let (_, first_writes, _) = first.tracked();
        let (_, second_writes, _) = second.tracked();

        sessions.register(first);
        sessions.register(second);

        sessions.write("session", b"payload").unwrap();
        assert_eq!(sessions.count(), 1);
        assert_eq!(first_writes.load(Ordering::SeqCst), 1);
        assert_eq!(second_writes.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn writes_route_to_the_requested_session_and_unknown_ids_are_noops() {
        let mut sessions = SessionManager::new();
        let first = FakeSession::new("first", "client-a");
        let second = FakeSession::new("second", "client-b");
        let (_, first_writes, _) = first.tracked();
        let (_, second_writes, _) = second.tracked();
        sessions.register(first);
        sessions.register(second);

        sessions.write("second", b"payload").unwrap();
        sessions.write("missing", b"ignored").unwrap();

        assert_eq!(first_writes.load(Ordering::SeqCst), 0);
        assert_eq!(second_writes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn close_is_idempotent_and_client_lookup_ignores_dead_sessions() {
        let mut sessions = SessionManager::new();
        let session = FakeSession::new("session", "client");
        let (alive, _, closes) = session.tracked();
        sessions.register(session);

        assert!(sessions.contains_clientid("client"));
        sessions.close("session");
        sessions.close("session");

        assert_eq!(sessions.count(), 0);
        assert_eq!(closes.load(Ordering::SeqCst), 1);
        assert!(!alive.load(Ordering::SeqCst));
        assert!(!sessions.contains_clientid("client"));
    }

    #[test]
    fn client_close_only_removes_matching_sessions() {
        let mut sessions = SessionManager::new();
        let first = FakeSession::new("first", "client-a");
        let second = FakeSession::new("second", "client-a");
        let third = FakeSession::new("third", "client-b");
        let (_, _, first_closes) = first.tracked();
        let (_, _, second_closes) = second.tracked();
        let (_, _, third_closes) = third.tracked();
        sessions.register(first);
        sessions.register(second);
        sessions.register(third);

        sessions.close_by_clientid("client-a");

        assert_eq!(sessions.count(), 1);
        assert!(sessions.contains("third"));
        assert_eq!(first_closes.load(Ordering::SeqCst), 1);
        assert_eq!(second_closes.load(Ordering::SeqCst), 1);
        assert_eq!(third_closes.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn gc_removes_only_dead_sessions_and_closes_once() {
        let mut sessions = SessionManager::new();
        let live = FakeSession::new("live", "client-a");
        let dead = FakeSession::new("dead", "client-b");
        let (_, _, live_closes) = live.tracked();
        let (dead_alive, _, dead_closes) = dead.tracked();
        sessions.register(live);
        sessions.register(dead);
        dead_alive.store(false, Ordering::SeqCst);

        sessions.gc();
        sessions.gc();

        assert_eq!(sessions.count(), 1);
        assert!(sessions.contains("live"));
        assert_eq!(live_closes.load(Ordering::SeqCst), 0);
        assert_eq!(dead_closes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn close_all_closes_each_session_and_clears_the_manager() {
        let mut sessions = SessionManager::new();
        let first = FakeSession::new("first", "client-a");
        let second = FakeSession::new("second", "client-b");
        let (_, _, first_closes) = first.tracked();
        let (_, _, second_closes) = second.tracked();
        sessions.register(first);
        sessions.register(second);

        sessions.close_all();
        sessions.close_all();

        assert_eq!(sessions.count(), 0);
        assert_eq!(first_closes.load(Ordering::SeqCst), 1);
        assert_eq!(second_closes.load(Ordering::SeqCst), 1);
    }
}
