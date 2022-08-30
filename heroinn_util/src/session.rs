use std::{collections::HashMap, io::*, sync::mpsc::Sender};

use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! close_all_session_in_lock {
    ($mgr:ident) => {
        let mut mgr = $mgr.lock().unwrap();
        mgr.close_all();
        drop(mgr);
    };
}
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

pub trait Session {
    fn new_client(sender: Sender<SessionBase>, clientid: &String, id: &String) -> Result<Self>
    where
        Self: Sized;
    fn new(sender: Sender<SessionBase>, clientid: &String, peer_addr: &String) -> Result<Self>
    where
        Self: Sized;
    fn id(&self) -> String;
    fn write(&mut self, data: &Vec<u8>) -> Result<()>;
    fn alive(&self) -> bool;
    fn close(&mut self);
    fn clientid(&self) -> String;
}

pub struct SessionManager<T> {
    sessions: HashMap<String, T>,
}

impl<T: Session> SessionManager<T> {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn register(&mut self, session: T) {
        if let std::collections::hash_map::Entry::Vacant(e) = self.sessions.entry(session.id()) {
            e.insert(session);
        }
    }

    pub fn write(&mut self, id: &String, data: &Vec<u8>) -> Result<()> {
        if self.sessions.contains_key(id) {
            log::debug!("found session : {}", id);
            let session = self.sessions.get_mut(id).unwrap();
            session.write(data)?;
        }
        Ok(())
    }

    pub fn contains(&self, id: &String) -> bool {
        self.sessions.contains_key(id)
    }

    pub fn close_by_clientid(&mut self, clientid: &String) {
        let mut need_close = vec![];

        for i in self.sessions.keys() {
            if self.sessions[i].clientid() == *clientid {
                need_close.push(i.clone());
            }
        }

        for i in need_close {
            self.sessions.get_mut(&i).unwrap().close();
            self.sessions.remove(&i);
        }
    }

    pub fn close_all(&mut self) {
        for i in self.sessions.values_mut() {
            i.close();
        }
        self.sessions.clear();
    }

    pub fn gc(&mut self) {
        let mut gc = vec![];
        for i in self.sessions.keys() {
            if !self.sessions[i].alive() {
                gc.push(i.clone());
            }
        }

        for i in gc {
            let session = self.sessions.get_mut(&i).unwrap();
            session.close();
            self.sessions.remove(&i);
        }
    }

    pub fn count(&self) -> usize {
        self.sessions.len()
    }
}
