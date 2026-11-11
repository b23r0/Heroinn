use std::{
    ffi::c_void,
    fmt,
    io::{self, Write},
};

use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Storage::FileSystem::{FlushFileBuffers, WriteFile},
};

use crate::{error::Error, util::clone_handle};

/// PipeWriter implements [std::io::Write] interface for win32 pipe.
pub struct PipeWriter {
    handle: HANDLE,
}

impl PipeWriter {
    /// Creates a new instance of PipeWriter.
    ///
    /// It owns a HANDLE.
    pub fn new(handle: HANDLE) -> Self {
        Self { handle }
    }

    /// Tries to make a clone of PipeWriter.
    pub fn try_clone(&self) -> Result<Self, Error> {
        clone_handle(self.handle).map_err(Into::into).map(Self::new)
    }
}

impl Write for PipeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_to_pipe(self.handle, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        flush_pipe(self.handle)
    }
}

impl Drop for PipeWriter {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle).ok().unwrap();
        }
    }
}

impl From<PipeWriter> for std::fs::File {
    fn from(pipe: PipeWriter) -> Self {
        use std::os::windows::io::FromRawHandle;
        // If we wouldn't wrap the writer in `ManuallyDrop`
        // the handle would be closed before the function
        // returned making the handle invalid.
        let pipe = std::mem::ManuallyDrop::new(pipe);
        unsafe { std::fs::File::from_raw_handle(pipe.handle.0 as _) }
    }
}

impl fmt::Debug for PipeWriter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PipeWriter")
            .field("handle", &(self.handle.0))
            .field("handle(ptr)", &(self.handle.0 as *const c_void))
            .finish()
    }
}

unsafe impl Send for PipeWriter {}
unsafe impl Sync for PipeWriter {}

fn write_to_pipe(h: HANDLE, buf: &[u8]) -> io::Result<usize> {
    let mut n = 0;

    unsafe {
        WriteFile(h, Some(buf), Some(&mut n), None)?;
    }

    Ok(n as usize)
}

fn flush_pipe(h: HANDLE) -> Result<(), io::Error> {
    unsafe {
        FlushFileBuffers(h)?;
    }

    Ok(())
}
