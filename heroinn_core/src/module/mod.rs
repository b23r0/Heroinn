use std::collections::HashMap;
use std::io::*;
use heroinn_util::session::Session;

pub mod shell;

pub struct SessionManager1<T>{
    sessions : HashMap<String , T>
}

impl<T : Session> SessionManager1<T>{

    pub fn new() -> Self{
        Self{
            sessions : HashMap::new()
        }
    }

    pub fn register(&mut self, session : T){
        if let std::collections::hash_map::Entry::Vacant(e) = self.sessions.entry(session.id()) {
            e.insert(session);
        }
    }

    pub fn write(&mut self , id : &String , data : &Vec<u8>) -> Result<()>{
        if self.sessions.contains_key(id) {
            let session = self.sessions.get_mut(id).unwrap();
            session.write(data)?;
        }
        Ok(())
    }

    pub fn contains(&self, id : &String) -> bool{
        self.sessions.contains_key(id)
    }

    pub fn close_by_clientid(&mut self , clientid : &String){
        let mut need_close = vec![];

        for i in self.sessions.keys(){
            if self.sessions[i].clientid() == *clientid{
                need_close.push(i.clone());
            }
        }

        for i in need_close{
            self.sessions.get_mut(&i).unwrap().close();
            self.sessions.remove(&i);
        }
    }
    pub fn gc(&mut self){
        let mut gc = vec![];
        for i in self.sessions.keys(){
            if !self.sessions[i].alive(){
                gc.push(i.clone());
            }
        }

        for i in gc{
            let session = self.sessions.get_mut(&i).unwrap();
            session.close();
            self.sessions.remove(&i);
        }
    }
}