use std::io::Write;
use std::{io, panic, ptr};

use winapi::um::winuser;

/// Convert a Rust string into a nul-terminated UTF-16 buffer, as the Win32 wide-char APIs expect.
fn win32_string(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

// Install a panic handler that renders the panic in a classical Windows error
// dialog box as well as writes the panic to STDERR.
pub fn attach_handler() {
    panic::set_hook(Box::new(|panic_info| {
        let _ = writeln!(io::stderr(), "{}", panic_info);
        let msg = format!("{}\n\nPress Ctrl-C to Copy", panic_info);
        unsafe {
            winuser::MessageBoxW(
                ptr::null_mut(),
                win32_string(&msg).as_ptr(),
                win32_string("Alacritty: Runtime Error").as_ptr(),
                winuser::MB_ICONERROR
                    | winuser::MB_OK
                    | winuser::MB_SETFOREGROUND
                    | winuser::MB_TASKMODAL,
            );
        }
    }));
}
