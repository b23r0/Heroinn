use windows::core as win;
use windows::Win32::{
    Foundation::{DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE},
    System::Threading::GetCurrentProcess,
};

/// clone_handle can be used to clone a general HANDLE.
pub(crate) fn clone_handle(handle: HANDLE) -> win::Result<HANDLE> {
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
        )?;
    }

    Ok(cloned_handle)
}
