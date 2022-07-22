use std::io;

use windows::core as win;
use windows::Win32::{
    Foundation::{DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE},
    System::Threading::GetCurrentProcess,
};

/// clone_handle can be used to clone a general HANDLE.
pub fn clone_handle(handle: HANDLE) -> win::Result<HANDLE> {
    let mut cloned_handle = HANDLE::default();
    unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            handle,
            GetCurrentProcess(),
            &mut cloned_handle,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
        .ok()?;
    }

    Ok(cloned_handle)
}

pub(crate) fn win_error_to_io(err: windows::core::Error) -> io::Error {
    let code = err.code();
    io::Error::from_raw_os_error(code.0 as i32)
}
