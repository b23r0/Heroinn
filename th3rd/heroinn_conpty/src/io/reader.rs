use std::{
    ffi::c_void,
    fmt,
    io::{self, Read},
    mem::MaybeUninit,
    ptr,
};

use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Storage::FileSystem::ReadFile,
    System::Pipes::PeekNamedPipe,
};

use crate::{error::Error, util::clone_handle};

/// PipeReader wraps a win32 pipe to provide a [std::io::Read] interface.
/// It also provides a non_blocking mode settings.
pub struct PipeReader {
    handle: HANDLE,
    blocking: bool,
}

impl PipeReader {
    /// Returns a new instance of PipeReader.
    pub fn new(handle: HANDLE) -> Self {
        Self {
            handle,
            blocking: true,
        }
    }

    /// Sets a pipe to a non blocking mode.
    ///
    /// It doesn't changes DUPed handles.
    ///
    /// Mainly developed to not pile down libraries to include any windows API crate.
    pub fn blocking(&mut self, on: bool) {
        self.blocking = on;
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
        read_pipe(self.handle, buf, self.blocking)
    }
}

impl Drop for PipeReader {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle).ok().unwrap();
        }
    }
}

impl From<PipeReader> for std::fs::File {
    fn from(pipe: PipeReader) -> Self {
        use std::os::windows::io::FromRawHandle;
        // If we wouldn't wrap the reader in `ManuallyDrop`
        // the handle would be closed before the function
        // returned making the handle invalid.
        let pipe = std::mem::ManuallyDrop::new(pipe);
        unsafe { std::fs::File::from_raw_handle(pipe.handle.0 as _) }
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

unsafe impl Send for PipeReader {}
unsafe impl Sync for PipeReader {}

fn pipe_available_bytes(h: HANDLE) -> io::Result<u32> {
    let mut bytes = MaybeUninit::<u32>::uninit();
    let bytes_ptr: *mut u32 = ptr::addr_of_mut!(*bytes.as_mut_ptr());

    unsafe {
        PeekNamedPipe(h, None, 0, None, Some(bytes_ptr), None)?;
    }

    let bytes = unsafe { bytes.assume_init() };
    Ok(bytes)
}

fn read_pipe(h: HANDLE, buf: &mut [u8], blocking: bool) -> io::Result<usize> {
    if !blocking {
        // We could use SetNamedPipeHandleState but seems like it doesn't work sometimes?
        // Plus it changes all DUPed handles

        let available = pipe_available_bytes(h)?;
        if available == 0 {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, ""));
        }
    }

    read_from_pipe(h, buf)
}

fn read_from_pipe(h: HANDLE, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;

    unsafe {
        ReadFile(h, Some(buf), Some(&mut n), None)?;
    }

    Ok(n as usize)
}
