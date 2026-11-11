//! A library which provides an interface for [ConPTY].
//!
//! ```ignore
//! use std::io::prelude::*;
//!
//! let mut proc = conpty::spawn("echo Hello World").unwrap();
//! let mut reader = proc.output().unwrap();
//!
//! let mut buf = [0; 1028];
//! let n = reader.read(&mut buf).unwrap();
//!
//! assert!(String::from_utf8_lossy(&buf).contains("Hello World"));
//! ```
//!
//! [ConPTY]: https://devblogs.microsoft.com/commandline/windows-command-line-introducing-the-windows-pseudo-console-conpty/

#![warn(
    missing_docs,
    future_incompatible,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results,
    unused_variables,
    variant_size_differences,
    missing_debug_implementations,
    rust_2018_idioms
)]

use std::{
    ffi::{OsStr, OsString},
    process::Command,
};

use error::Error;

pub mod console;
pub mod error;
pub mod io;

mod process;
mod util;

pub use process::Process;
pub use process::ProcessOptions;

/// Spawns a command using `cmd.exe`.
pub fn spawn(command: impl AsRef<OsStr>) -> Result<Process, Error> {
    let mut cmd = OsString::new();
    cmd.push("cmd /C ");
    cmd.push(command);

    Process::spawn(Command::new(&cmd))
}
