//! TTY related functionality.
use crate::config::{PtyConfig};
use crate::event::{OnResize, WindowSize};
use crate::tty::{ChildEvent, EventedPty, EventedReadWrite};
use mio::net::TcpStream;
use mio::Token;

use std::io::*;

pub static MAGIC_FLAG : [u8;2] = [0x37, 0x37];

pub struct Pty {
    socket : TcpStream,
    token: mio::Token,
    child_event_token : mio::Token
}

pub fn splitword(a : u16) -> (u8 , u8) {
    ((a >> 8) as u8  , a as u8)
}

fn make_winsize_packet(window_size : WindowSize) -> Vec<u8>{
    let mut ret = vec![];
    ret.append(&mut MAGIC_FLAG.to_vec());
    let rows = splitword(window_size.num_lines);
    let cols = splitword(window_size.num_cols);

    ret.push(rows.0);
    ret.push(rows.1);
    ret.push(cols.0);
    ret.push(cols.1);

    ret
}

/// Create a new localsocket and return a handle to interact with it.
pub fn new(config: &PtyConfig, window_size: WindowSize, _window_id: Option<usize>) -> Result<Pty> {

    let local_addr = format!("127.0.0.1:{}" , config.local_socket_port);

    let mut s = TcpStream::connect(local_addr.parse().unwrap()).unwrap();

    let mut set_window_packet = make_winsize_packet(window_size);
    match s.write_all(&mut set_window_packet){
        Ok(_) => {},
        Err(_) => {},
    };

    Ok(Pty {
        socket : s,
        token: Token(0),
        child_event_token : Token(0)
    })
}

impl EventedReadWrite for Pty {
    type Reader = TcpStream;
    type Writer = TcpStream;

    #[inline]
    fn register(
        &mut self,
        registry: &mio::Registry,
        token: &mut dyn Iterator<Item = mio::Token>,
        interest: mio::Interest,
    ) -> Result<()> {
        self.token = token.next().unwrap();

        registry.register(&mut self.socket, self.token, interest)?;

        self.child_event_token = token.next().unwrap();
        Ok(())
    }

    #[inline]
    fn reregister(&mut self, registry: &mio::Registry, interest: mio::Interest) -> Result<()> {
        registry.reregister(&mut self.socket, self.token, interest)?;
        Ok(())
    }

    #[inline]
    fn deregister(&mut self, registry: &mio::Registry) -> Result<()> {
        registry.deregister(&mut self.socket)?;
        Ok(())
    }

    #[inline]
    fn reader(&mut self) -> &mut Self::Reader {
        &mut self.socket
    }

    #[inline]
    fn read_token(&self) -> mio::Token {
        self.token
    }

    #[inline]
    fn writer(&mut self) -> &mut Self::Writer {
        &mut self.socket
    }

    #[inline]
    fn write_token(&self) -> mio::Token {
        self.token
    }
}

impl EventedPty for Pty {
    fn child_event_token(&self) -> mio::Token {
        self.child_event_token
    }

    fn next_child_event(&mut self) -> Option<ChildEvent> {
        None
    }
}

impl OnResize for Pty {
    /// Resize the PTY.
    ///
    /// Tells the kernel that the window size changed with the new pixel
    /// dimensions and line/column counts.
    fn on_resize(&mut self, window_size: WindowSize) {
        let mut set_window_packet = make_winsize_packet(window_size);
        match self.socket.write_all(&mut set_window_packet){
            Ok(_) => {},
            Err(_) => {},
        };
    }
}