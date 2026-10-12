//! Server-side session implementations: interactive shells and file transfer.
//!
//! Transport and dispatch live in `heroinn_util`; this crate only provides the
//! worker-backed [`module::shell::ShellServer`] and
//! [`module::ftp::FtpServer`] sessions.

pub mod module;
