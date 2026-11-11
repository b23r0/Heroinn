//! This module contains [crate::Process]'s `Input` and `Output` pipes.
//!
//! Input - PipeWriter
//! Output - PipeReader

mod reader;
mod writer;

pub use reader::PipeReader;
pub use writer::PipeWriter;
