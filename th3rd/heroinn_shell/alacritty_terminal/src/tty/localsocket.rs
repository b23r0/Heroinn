//! TTY related functionality.
use crate::config::PtyConfig;
use crate::event::{OnResize, WindowSize};
use crate::tty::{ChildEvent, EventedPty, EventedReadWrite};
use heroinn_shell_protocol::{encode_resize, WindowSize as ControlWindowSize};
use mio::net::TcpStream;
use mio::Token;

use std::io::*;

pub struct Pty {
    socket: TcpStream,
    token: mio::Token,
    child_event_token: mio::Token,
}

fn make_winsize_packet(window_size: WindowSize) -> Vec<u8> {
    encode_resize(ControlWindowSize { rows: window_size.num_lines, cols: window_size.num_cols })
        .to_vec()
}

/// Create a new localsocket and return a handle to interact with it.
pub fn new(config: &PtyConfig, window_size: WindowSize, _window_id: Option<usize>) -> Result<Pty> {
    let local_addr = format!("127.0.0.1:{}", config.local_socket_port);

    let local_addr = local_addr.parse().map_err(|error| {
        Error::new(ErrorKind::InvalidInput, format!("invalid local shell address: {error}"))
    })?;
    let mut s = TcpStream::connect(local_addr).map_err(|error| {
        Error::new(error.kind(), format!("connect to local shell address failed: {error}"))
    })?;

    let set_window_packet = make_winsize_packet(window_size);
    s.write_all(&set_window_packet)?;

    Ok(Pty { socket: s, token: Token(0), child_event_token: Token(0) })
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
        self.token =
            token.next().ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing PTY token"))?;

        registry.register(&mut self.socket, self.token, interest)?;

        self.child_event_token = token
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing child token"))?;
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
        let set_window_packet = make_winsize_packet(window_size);
        if let Err(error) = self.socket.write_all(&set_window_packet) {
            log::warn!("unable to send terminal resize: {error}");
        }
    }
}
