//! This module contains [crate::Process]'s `Input` and `Output` pipes.
//!
//! Input - PipeWriter
//! Output - PipeReader

use windows::core::HRESULT;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{FlushFileBuffers, ReadFile, WriteFile};
use windows::Win32::System::Pipes::{SetNamedPipeHandleState, PIPE_NOWAIT, PIPE_WAIT};

use crate::error::Error;
use crate::util::{clone_handle, win_error_to_io};
use std::ffi::c_void;
use std::io::{Read, Write};
use std::ptr::null_mut;
use std::{fmt, io};

/// PipeReader wraps a win32 pipe to provide a [std::io::Read] interface.
/// It also provides a non_blocking mode settings.
pub struct PipeReader {
    handle: HANDLE,
}

impl PipeReader {
    /// Returns a new instance of PipeReader.
    pub fn new(handle: HANDLE) -> Self {
        Self { handle }
    }

    /// Sets a pipe to a non blocking mode.
    ///
    /// IMPORTANT: It affects all dupped descriptors (All cloned `HANDLE`s, `FILE`s, `PipeReader`s)
    ///
    /// Mainly developed to not pile down libraries to include any windows API crate.
    pub fn set_non_blocking_mode(&mut self) -> Result<(), Error> {
        unsafe { SetNamedPipeHandleState(self.handle, &PIPE_NOWAIT, null_mut(), null_mut()).ok()? }
        Ok(())
    }

    /// Sets a pipe to a blocking mode.
    ///
    /// IMPORTANT: It affects all dupped descriptors (All cloned `HANDLE`s, `FILE`s, `PipeReader`s)
    ///
    /// Mainly developed to not pile down libraries to include any windows API crate.
    pub fn set_blocking_mode(&mut self) -> Result<(), Error> {
        unsafe {
            SetNamedPipeHandleState(self.handle, &PIPE_WAIT, null_mut(), null_mut()).ok()?;
        }
        Ok(())
    }

    /// Tries to clone a instance to a new one.
    /// All cloned instances share the same underlaying data so
    /// Reading from one cloned pipe will affect an original pipe.
    pub fn try_clone(&self) -> Result<Self, Error> {
        clone_handle(self.handle).map_err(Into::into).map(Self::new)
    }
}

impl Read for PipeReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut n = 0;
        let buf_size = buf.len() as u32;

        let result = unsafe {
            ReadFile(
                self.handle,
                buf.as_mut_ptr() as _,
                buf_size,
                &mut n,
                null_mut(),
            )
            .ok()
        };
        match result {
            Ok(()) => Ok(n as usize),
            // https://stackoverflow.com/questions/34504970/non-blocking-read-on-os-pipe-on-windows
            Err(err) if err.code() == HRESULT::from_win32(232) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                err.message().to_string(),
            )),
            Err(err) => Err(win_error_to_io(err)),
        }
    }
}

impl Drop for PipeReader {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle).ok().unwrap();
        }
    }
}

impl Into<std::fs::File> for PipeReader {
    fn into(self) -> std::fs::File {
        use std::os::windows::io::FromRawHandle;
        unsafe { std::fs::File::from_raw_handle(self.handle.0 as _) }
    }
}

impl fmt::Debug for PipeReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PipeReader")
            .field("handle", &(self.handle.0))
            .field("handle(ptr)", &(self.handle.0 as *const c_void))
            .finish()
    }
}

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
        let mut n = 0;
        let buf_size = buf.len() as u32;

        unsafe {
            WriteFile(self.handle, buf.as_ptr() as _, buf_size, &mut n, null_mut())
                .ok()
                .map_err(win_error_to_io)?;
        }

        Ok(n as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        unsafe {
            FlushFileBuffers(self.handle)
                .ok()
                .map_err(win_error_to_io)?;
        }
        Ok(())
    }
}

impl Drop for PipeWriter {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle).ok().unwrap();
        }
    }
}

impl Into<std::fs::File> for PipeWriter {
    fn into(self) -> std::fs::File {
        use std::os::windows::io::FromRawHandle;
        unsafe { std::fs::File::from_raw_handle(self.handle.0 as _) }
    }
}

impl fmt::Debug for PipeWriter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PipeReader")
            .field("handle", &(self.handle.0))
            .field("handle(ptr)", &(self.handle.0 as *const c_void))
            .finish()
    }
}
