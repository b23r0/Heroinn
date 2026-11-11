//! Module contains a handy functions for terminal.

use windows::core::Result as WinResult;
use windows::Win32::Foundation::WAIT_OBJECT_0;
use windows::Win32::{
    Foundation::HANDLE,
    System::{
        Console::{
            GetConsoleMode, GetStdHandle, SetConsoleMode, CONSOLE_MODE,
            DISABLE_NEWLINE_AUTO_RETURN, ENABLE_ECHO_INPUT, ENABLE_EXTENDED_FLAGS,
            ENABLE_INSERT_MODE, ENABLE_LINE_INPUT, ENABLE_MOUSE_INPUT, ENABLE_PROCESSED_INPUT,
            ENABLE_QUICK_EDIT_MODE, ENABLE_VIRTUAL_TERMINAL_INPUT, STD_ERROR_HANDLE,
            STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
        },
        Threading::WaitForSingleObject,
    },
};

use crate::error::Error;

/// Console represents a terminal session with opened stdin, stdout and stderr.
#[derive(Debug, Clone)]
pub struct Console {
    stdin: HANDLE,
    stdout: HANDLE,
    stderr: HANDLE,
    stdin_mode: CONSOLE_MODE,
    stdout_mode: CONSOLE_MODE,
    stderr_mode: CONSOLE_MODE,
}

impl Console {
    /// Creates a console from default stdin, stdout and stderr.
    pub fn current() -> Result<Self, Error> {
        // We don't close these handle on drop because:
        //  It is not required to CloseHandle when done with the handle retrieved from GetStdHandle.
        //  The returned value is simply a copy of the value stored in the process table.
        let stdin = unsafe { GetStdHandle(STD_INPUT_HANDLE)? };
        let stdout = unsafe { GetStdHandle(STD_OUTPUT_HANDLE)? };
        let stderr = unsafe { GetStdHandle(STD_ERROR_HANDLE)? };

        let stdin_mode = get_console_mode(stdin)?;
        let stdout_mode = get_console_mode(stdout)?;
        let stderr_mode = get_console_mode(stderr)?;

        Ok(Self {
            stderr,
            stderr_mode,
            stdin,
            stdin_mode,
            stdout,
            stdout_mode,
        })
    }

    /// Sets terminal in a raw mode.
    /// Raw mode is a mode where most of consoles processing is ommited.
    pub fn set_raw(&self) -> Result<(), Error> {
        set_raw_stdin(self.stdin, self.stdin_mode)?;

        unsafe {
            SetConsoleMode(self.stdout, self.stdout_mode | DISABLE_NEWLINE_AUTO_RETURN)?;
        }
        unsafe {
            SetConsoleMode(self.stderr, self.stderr_mode | DISABLE_NEWLINE_AUTO_RETURN)?;
        }

        Ok(())
    }

    /// Sets terminal in a mode which was initially used on handles.
    pub fn reset(&self) -> Result<(), Error> {
        for (handle, mode) in self.streams() {
            unsafe { SetConsoleMode(handle, mode)? };
        }

        Ok(())
    }

    /// Verifies if there's something in stdin to read.
    ///
    /// It can be used to determine if the call to `[std::io::stdin].read()` will block
    pub fn is_stdin_empty(&self) -> Result<bool, Error> {
        // https://stackoverflow.com/questions/23164492/how-can-i-detect-if-there-is-input-waiting-on-stdin-on-windows
        let empty = unsafe { WaitForSingleObject(self.stdin, 0) == WAIT_OBJECT_0 };
        Ok(empty)
    }

    fn streams(&self) -> [(HANDLE, CONSOLE_MODE); 3] {
        [
            (self.stdin, self.stdin_mode),
            (self.stdout, self.stdout_mode),
            (self.stderr, self.stderr_mode),
        ]
    }
}

fn get_console_mode(h: HANDLE) -> WinResult<CONSOLE_MODE> {
    let mut mode = CONSOLE_MODE::default();
    unsafe {
        GetConsoleMode(h, &mut mode)?;
    }
    Ok(mode)
}

fn set_raw_stdin(stdin: HANDLE, mut mode: CONSOLE_MODE) -> WinResult<()> {
    mode &= !ENABLE_ECHO_INPUT;
    mode &= !ENABLE_LINE_INPUT;
    mode &= !ENABLE_MOUSE_INPUT;
    mode &= !ENABLE_LINE_INPUT;
    mode &= !ENABLE_PROCESSED_INPUT;

    mode |= ENABLE_EXTENDED_FLAGS;
    mode |= ENABLE_INSERT_MODE;
    mode |= ENABLE_QUICK_EDIT_MODE;

    let vt_input_supported = true;
    if vt_input_supported {
        mode |= ENABLE_VIRTUAL_TERMINAL_INPUT;
    }

    unsafe {
        SetConsoleMode(stdin, mode)?;
    }

    Ok(())
}
