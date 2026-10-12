//! Session lifecycle shared by shells and file-transfer channels.
//!
//! A "session" is one logical interactive channel between the server and a
//! client. Both sides keep a [`SessionManager`] of live sessions keyed by the
//! session id that travels on the wire; the two sides only differ in *how* a
//! session is created, which is why construction lives behind [`ServerSession`]
//! and [`ClientSession`] rather than on [`Session`] itself.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
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
