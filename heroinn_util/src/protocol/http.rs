use std::{net::{SocketAddr, TcpStream}, sync::{Arc, Mutex, atomic::AtomicBool, mpsc::channel}, collections::HashMap};
use websocket::{OwnedMessage, sync::{Writer}};
use crate::{HeroinnProtocol, protocol::{TUNNEL_FLAG, tcp::TcpConnection}};
use super::{Server, Client};

pub struct WSServer{
    local_addr : SocketAddr,
    closed : Arc<AtomicBool>,
    connections : Arc<Mutex<HashMap<SocketAddr , Writer<TcpStream>>>>
}

pub struct WSConnection{
    s : Option<websocket::sync::Client<TcpStream>>
}

impl Server for WSServer{
    fn new<
        CBCB: 'static + Fn(crate::packet::Message) + Send + Copy , 
        CB: 'static + Fn(crate::HeroinnProtocol , Vec<u8>, SocketAddr, CBCB) + Send
    >(
        address : &str , 
        cb_data : CB,
        cbcb : CBCB,
    ) -> std::io::Result<Self> where Self: Sized {
        let mut server = websocket::sync::Server::bind(address).unwrap();
        server.set_nonblocking(true).unwrap();

        let connections : Arc<Mutex<HashMap<SocketAddr , Writer<TcpStream>>>> = Arc::new(Mutex::new(HashMap::new()));
        let closed = Arc::new(AtomicBool::new(false));

        let local_addr = server.local_addr().unwrap();

        let connections_1 = connections.clone();
        let closed_1 = closed.clone();
        let cb_data = Arc::new(Mutex::new(cb_data));
        std::thread::Builder::new().name(format!("ws main worker : {}" , local_addr.clone())).spawn(move || {

            loop{
                let client = match server.accept(){
                    Ok(p) => p.accept().unwrap(),
                    Err(_) => {
                        
                        if closed_1.load(std::sync::atomic::Ordering::Relaxed){
                            break;
                        }

                        std::thread::sleep(std::time::Duration::from_millis(200));
                        continue;
                    },
                };

                let connections_2 = connections_1.clone();
                let cb_data = cb_data.clone();
                std::thread::Builder::new().name(format!("ws client worker : {}" , local_addr.clone())).spawn(move || {
                    
                    client.set_nonblocking(false).unwrap();
                    let remote_addr = client.peer_addr().unwrap();

                    log::info!("ws accept from : {}" , remote_addr);
        
                    let (mut receiver, sender) = client.split().unwrap();

                    {
                        let mut conns = connections_2.lock().unwrap();
                        conns.insert(remote_addr, sender);
                    }

                    
                    for message in receiver.incoming_messages() {
                        let message = message.unwrap();
        
                        match message {
                            OwnedMessage::Close(_) => {
                                let mut conns = connections_2.lock().unwrap();
                                conns.remove(&remote_addr);
                                log::info!("ws connection closed : {}", remote_addr);
                                return;
                            }
                            OwnedMessage::Binary(buf) => {

                                if buf.len() == 6 {
                                    if buf[..4] == TUNNEL_FLAG{
                                        let mut conns = connections_2.lock().unwrap();
                                        let mut sender = conns.remove(&remote_addr).unwrap();

                                        let port = [buf[4] , buf[5]];
                                        let port = u16::from_be_bytes(port);

                                        let full_addr = format!("127.0.0.1:{}", port);
                                        let tunnel_client = match TcpConnection::connect(&full_addr){
                                            Ok(p) => p,
                                            Err(e) => {
                                                log::error!("tunnel connect faild : {}" , e);
                                                break;
                                            },
                                        };

                                        let (tx1, rx1) = channel::<Vec<u8>>();
                                        let (tx2, rx2) = channel::<Vec<u8>>();

                                        std::thread::Builder::new().name(format!("ws sender worker1")).spawn(move || {

                                            loop{
                                                let buf = match rx1.recv(){
                                                    Ok(p) => p,
                                                    Err(_) => break,
                                                };

                                                if let Err(e) = sender.send_message(&OwnedMessage::Binary(buf)){
                                                    log::error!("ws sender error : {}" , e);
                                                    break;
                                                };
                                            }

                                            log::debug!("ws sender worker1!");
                                        }).unwrap();

                                        let mut tunnel_client_1 = tunnel_client.clone();
                                        std::thread::Builder::new().name(format!("ws sender worker2")).spawn(move || {

                                            loop{
                                                let mut buf = match rx2.recv(){
                                                    Ok(p) => p,
                                                    Err(_) => break,
                                                };

                                                if let Err(e) = tunnel_client_1.send(&mut buf){
                                                    log::error!("ws sender error : {}" , e);
                                                    break;
                                                };
                                            }

                                            log::debug!("ws sender worker2!");
                                        }).unwrap();

                                        let mut tunnel_client_2 = tunnel_client.clone();
                                        std::thread::Builder::new().name(format!("ws receiver worker1 : {}" , tunnel_client.local_addr().unwrap())).spawn(move || {
                                            loop{
                                                let buf = match tunnel_client_2.recv(){
                                                    Ok(p) => p,
                                                    Err(e) => {
                                                        log::error!("tunnel read faild : {}" , e);
                                                        break;
                                                    },
                                                };

                                                if let Err(e) = tx1.send(buf){
                                                    log::error!("ws sender error : {}" , e);
                                                    break;
                                                }
                                                
                                            }
                                            log::debug!("receiver worker1 finished!");
                                        }).unwrap();

                                        std::thread::Builder::new().name(format!("ws receiver worker2 : {}" , tunnel_client.local_addr().unwrap())).spawn(move || {
                                            
                                            loop{
                                                let buf = match receiver.recv_message(){
                                                    Ok(p) => match p{
                                                        OwnedMessage::Binary(p) => p,
                                                        OwnedMessage::Close(_) => {
                                                            break;
                                                        },
                                                        _ => continue
                                                    },
                                                    Err(e) => {
                                                        log::error!("ws receiver error : {}" , e);
                                                        break;
                                                    },
                                                };

                                                if let Err(e) = tx2.send(buf){
                                                    log::error!("ws sender error : {}" , e);
                                                    break;
                                                }
                                            }
                                            
                                            log::debug!("receiver worker2 finished!");
                                        }).unwrap();

                                        break;
                                    }
                                }

                                cb_data.lock().unwrap()(HeroinnProtocol::TCP , buf, remote_addr, cbcb);
                            }
                            _ => {},
                        }
                    }
                }).unwrap();
            }

            log::info!("ws main worker finished");
        }).unwrap();



        Ok(Self{
            local_addr,
            closed,
            connections,
        })
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok(self.local_addr.clone())
    }

    fn sendto(&mut self , peer_addr : &SocketAddr , buf : &[u8]) -> std::io::Result<()> {
        match self.connections.lock().unwrap().get_mut(peer_addr){
            Some(k) => {
                let msg = OwnedMessage::Binary(buf.to_vec());
                match k.send_message(&msg){
                    Ok(_) => {},
                    Err(e) => {
                        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted , format!("ws send msg error : {}", e)));
                    },
                };
                Ok(())
            },
            None => Err(std::io::Error::new(std::io::ErrorKind::NotFound, "not found client")),
        }
    }

    fn contains_addr(&mut self , peer_addr : &SocketAddr) -> bool {
        self.connections.lock().unwrap().contains_key(peer_addr)
    }

    fn close(&mut self) {
        self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Drop for WSServer{
    fn drop(&mut self) {
        self.close();
    }
}

impl Client for WSConnection{
    fn connect(address : &str) -> std::io::Result<Self> where Self: Sized {

        let s = match websocket::ClientBuilder::new(&format!("ws:://{}" , address))
		.unwrap()
		.connect_insecure(){
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted , format!("ws connect error : {}", e)));
            },
        };

        Ok(Self{
            s : Some(s)
        })
    }

    fn tunnel(remote_addr : &str , server_local_port : u16) -> std::io::Result<Self> where Self: Sized {

        log::info!("start tunnel [{}] [{}]", remote_addr , server_local_port);
        let s = match websocket::ClientBuilder::new(&format!("ws://{}" , remote_addr))
		.unwrap()
		.connect_insecure(){
            Ok(p) => p,
            Err(e) => {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted , format!("ws connect error : {}", e)));
            },
        };

        let mut buf = TUNNEL_FLAG.to_vec();
        buf.append(&mut server_local_port.to_be_bytes().to_vec());

        let mut ret = Self{
            s : Some(s)
        };

        ret.send(&mut buf)?;

        Ok(ret)
    }

    fn recv(&mut self) -> std::io::Result<Vec<u8>> {
        let s = match self.s.as_mut(){
            Some(p) => p,
            None => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData , "socket closed"));
            },
        };

        match s.recv_message(){
            Ok(msg) => {
                match msg{
                    OwnedMessage::Binary(buf) => return Ok(buf),
                    OwnedMessage::Close(_) => {
                        self.close();
                        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted , format!("ws closed")));
                    }
                    _ => return Ok(vec![])
                }
            },
            Err(e) => {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted , format!("ws receive error : {}", e)));
            },
        }
    }

    fn send(&mut self,buf : &mut [u8]) -> std::io::Result<()> {
        let s = match self.s.as_mut(){
            Some(p) => p,
            None => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData , "socket closed"));
            },
        };

        let msg = OwnedMessage::Binary(buf.to_vec());
        if let Err(e) = s.send_message(&msg){
            return Err(std::io::Error::new(std::io::ErrorKind::Interrupted , format!("ws send msg error : {}", e)));
        };
        Ok(())
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        let s = match self.s.as_ref(){
            Some(p) => p,
            None => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData , "socket closed"));
            },
        };
        s.local_addr()
    }

    fn close(&mut self) {
        self.s = None;
    }
}

impl Drop for WSConnection{
    fn drop(&mut self) {
        self.close();
    }
}

#[test]
fn test_ws_tunnel(){
    
    let server = WSServer::new(&"127.0.0.1:0", |_ , _ ,_ ,_| {} , |_| {}).unwrap();
    let server2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let remote_local_port = server2.local_addr().unwrap().port();

    let remote = &format!("127.0.0.1:{}" , server.local_addr().unwrap().port());
    let mut client1 = WSConnection::tunnel(remote, remote_local_port).unwrap();

    let (mut client2 , _) = super::tcp::TcpConnection::tunnel_server(server2, 10).unwrap();

    for _ in 0..3{

        client1.send(&mut [0,1,2,3]).unwrap();
        let buf = client2.recv().unwrap();
        assert!(buf == [0,1,2,3]);

        client2.send(&mut [4,5,6,7]).unwrap();
        let buf = client1.recv().unwrap();
        assert!(buf == [4,5,6,7]);
    }
}