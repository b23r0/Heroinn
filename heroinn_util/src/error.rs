//! Unified error type shared across the whole Heroinn workspace.

use std::sync::{Mutex, MutexGuard};

/// Every failure the protocol, session and transport layers can produce.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Wrapped [`std::io::Error`] from sockets, files and pipes.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// Wrapped [`serde_json::Error`] from packet (de)serialization.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// A byte that does not map onto any known [`crate::HeroinnProtocol`].
    #[error("unknown protocol id: {0:#04x}")]
    UnknownProtocol(u8),

    /// A byte that does not map onto any known [`crate::HeroinnClientMsgID`] or
    /// [`crate::HeroinnServerCommandID`].
    #[error("unknown message id: {0:#04x}")]
    UnknownMessageId(u8),

    /// A byte that does not map onto any known [`crate::ftp::FTPId`].
    #[error("unknown ftp id: {0:#04x}")]
    UnknownFtpId(u8),

    /// A packet was structurally invalid.
    #[error("malformed packet: {0}")]
    Malformed(&'static str),

    /// A lookup by key (session, listener, rpc) failed.
    #[error("not found: {0}")]
    NotFound(String),

    /// A frame exceeded [`crate::protocol::frame::MAX_PACKET`].
    #[error("packet too large: {0} bytes")]
    PacketTooLarge(usize),

    /// A bounded wait (accept, connect) expired.
    #[error("operation timed out")]
    Timeout,

    /// Any other, free-form failure.
    #[error("{0}")]
    Other(String),
}

/// Convenience alias used throughout the workspace.
pub type Result<T> = std::result::Result<T, Error>;

/// Lock a mutex, recovering the guard when a previous holder panicked.
///
/// A poisoned mutex would otherwise turn any unrelated panic into a cascade of
/// `unwrap` panics on every later lock; the data behind these locks is plain
/// state that stays consistent enough to keep using.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
