//! A library which provides an interface for [ConPTY].
//!
//! ```ignore
//! # // todo: determine why this test timeouts if runnin as a doc test but not as an example.
//! use std::io::prelude::*;
//!
//! fn main() {
//!     let proc = conpty::spawn("echo Hello World").unwrap();
//!     let mut reader = proc.output().unwrap();
//!
//!     println!("Process has pid={}", proc.pid());
//!
//!     proc.wait(None).unwrap();
//!
//!     let mut buf = [0; 1028];
//!     let n = reader.read(&mut buf).unwrap();
//!     assert!(String::from_utf8_lossy(&buf[..n]).contains("Hello World"));
//! }
//! ```
//!
//! [ConPTY]: https://devblogs.microsoft.com/commandline/windows-command-line-introducing-the-windows-pseudo-console-conpty/

#![allow(non_snake_case)]

pub mod console;
pub mod error;
pub mod io;
pub mod util;

use error::Error;
use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt;
use std::time::Duration;
use std::{mem::size_of, ptr::null_mut};
use windows::core::{self as win, IntoParam, Param, HRESULT};
use windows::Win32::Foundation::{CloseHandle, HANDLE, PWSTR, WAIT_TIMEOUT};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Console::{
    ClosePseudoConsole, CreatePseudoConsole, GetConsoleMode, GetConsoleScreenBufferInfo,
    ResizePseudoConsole, SetConsoleMode, CONSOLE_MODE, CONSOLE_SCREEN_BUFFER_INFO, COORD,
    ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING, HPCON, AllocConsole, AttachConsole, ATTACH_PARENT_PROCESS, GetConsoleWindow,
};
use windows::Win32::System::Pipes::CreatePipe;
use windows::Win32::System::Threading::{
    CreateProcessW, GetExitCodeProcess, GetProcessId,
    InitializeProcThreadAttributeList, TerminateProcess, UpdateProcThreadAttribute,
    WaitForSingleObject, CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, STARTUPINFOEXW,
};
use windows::Win32::System::WindowsProgramming::INFINITE;
use windows::Win32::UI::WindowsAndMessaging::ShowWindow;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

/// Spawns a command using `cmd.exe`.
pub fn spawn(cmd: impl Into<String>) -> Result<Process, Error> {
    Process::spawn(ProcAttr::cmd(cmd.into()))
}

/// The structure is resposible for interations with spawned process.
/// It handles IO and other operations related to a spawned process.
pub struct Process {
    pty_input: HANDLE,
    pty_output: HANDLE,
    _proc: PROCESS_INFORMATION,
    //_proc_info: STARTUPINFOEXW,
    _console: HPCON,
}

impl Process {
    fn spawn(attr: ProcAttr) -> Result<Self, Error> {
        enableVirtualTerminalSequenceProcessing()?;
        let (mut console, pty_reader, pty_writer) = createPseudoConsole()?;
        let startup_info = initializeStartupInfoAttachedToConPTY(&mut console)?;
        let proc = execProc(startup_info, attr)?;

        Ok(Self {
            pty_input: pty_writer,
            pty_output: pty_reader,
            _console: console,
            _proc: proc,
            //_proc_info: startup_info,
        })
    }

    /// Resizes virtuall terminal.
    pub fn resize(&self, x: i16, y: i16) -> Result<(), Error> {
        unsafe { ResizePseudoConsole(self._console, COORD { X: x, Y: y }) }?;
        Ok(())
    }

    /// Returns a process's pid.
    pub fn pid(&self) -> u32 {
        unsafe { GetProcessId(self._proc.hProcess) }
    }

    /// Termianates process with exit_code.
    pub fn exit(&self, code: u32) -> Result<(), Error> {
        unsafe { TerminateProcess(self._proc.hProcess, code).ok() }?;
        Ok(())
    }

    /// Waits before process exists.
    pub fn wait(&self, timeout_millis: Option<u32>) -> Result<u32, Error> {
        unsafe {
            match timeout_millis {
                Some(timeout) => {
                    if WaitForSingleObject(self._proc.hProcess, timeout) == WAIT_TIMEOUT {
                        return Err(Error::Timeout(Duration::from_millis(timeout as u64)));
                    }
                }
                None => {
                    WaitForSingleObject(self._proc.hProcess, INFINITE);
                }
            }

            let mut code = 0;
            GetExitCodeProcess(self._proc.hProcess, &mut code).ok()?;

            Ok(code)
        }
    }

    /// Is alive determines if a process is still running.
    ///
    /// IMPORTANT: Beware to use it in a way to stop reading when is_alive is false.
    //  Because at the point of calling method it may be alive but at the point of `read` call it may already not.
    pub fn is_alive(&self) -> bool {
        // https://stackoverflow.com/questions/1591342/c-how-to-determine-if-a-windows-process-is-running/5303889
        unsafe { WaitForSingleObject(self._proc.hProcess, 0) == WAIT_TIMEOUT }
    }

    /// Sets echo mode for a session.
    pub fn set_echo(&self, on: bool) -> Result<(), Error> {
        // todo: determine if this function is usefull and it works?
        let stdout_h = stdout_handle()?;
        unsafe {
            let mut mode = CONSOLE_MODE::default();
            GetConsoleMode(stdout_h, &mut mode).ok()?;

            match on {
                true => mode |= ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT,
                false => mode &= !ENABLE_ECHO_INPUT,
            };

            SetConsoleMode(stdout_h, mode).ok()?;
            CloseHandle(stdout_h).ok()?;
        }

        Ok(())
    }

    /// Returns a pipe writer to conPTY.
    pub fn input(&self) -> Result<io::PipeWriter, Error> {
        // see [Self::output]
        let handle = util::clone_handle(self.pty_input)?;
        Ok(io::PipeWriter::new(handle))
    }

    /// Returns a pipe reader from conPTY.
    pub fn output(&self) -> Result<io::PipeReader, Error> {
        // It's crusial to clone first and not affect original HANDLE
        // as closing it closes all other's handles even though it's kindof unxpected.
        //
        // "
        // Closing a handle does not close the object.  It merely reduces the
        // "reference count".  When the reference count goes to zero, the object
        // itself is closed.  So, if you have a file handle, and you duplicate that
        // handle, the file now has two "references".  If you close one handle, the
        // file still has one reference, so the FILE cannot be closed.
        // "
        //
        // https://social.msdn.microsoft.com/Forums/windowsdesktop/en-US/1754715c-45b7-4d8c-ba56-a501ccaec12c/closehandle-amp-duplicatehandle?forum=windowsgeneraldevelopmentissues
        let handle = util::clone_handle(self.pty_output)?;
        Ok(io::PipeReader::new(handle))
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            ClosePseudoConsole(self._console);

            let _ = CloseHandle(self._proc.hProcess);
            let _ = CloseHandle(self._proc.hThread);

            //DeleteProcThreadAttributeList(self._proc_info.lpAttributeList);
            //let _ = Box::from_raw(self._proc_info.lpAttributeList as _);

            let _ = CloseHandle(self.pty_input);
            let _ = CloseHandle(self.pty_output);
        }
    }
}

impl fmt::Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PipeReader")
            .field("pty_output", &(self.pty_output.0))
            .field("pty_output(ptr)", &(self.pty_output.0 as *const c_void))
            .field("pty_input", &(self.pty_input.0))
            .field("pty_input(ptr)", &(self.pty_input.0 as *const c_void))
            .finish_non_exhaustive()
    }
}

/// ProcAttr represents parameters for process to be spawned.
///
/// Interface is inspired by win32 `CreateProcess` function.
///
/// Generally to run a common process you can set commandline to a path to binary.
/// But if you're trying to spawn just a command in shell if must provide your shell first, like cmd.exe.
///
/// # Example
///
/// ```ignore
/// let attr = conpty::ProcAttr::default().commandline("pwsh").arg("echo", "world");
/// ```
#[derive(Default, Debug)]
pub struct ProcAttr {
    application: Option<String>,
    commandline: Option<String>,
    current_dir: Option<String>,
    args: Vec<String>,
    env: Option<HashMap<String, String>>,
}

impl ProcAttr {
    /// Runs a batch file in a default `CMD` interpretator.
    pub fn batch(file: impl AsRef<str>) -> Self {
        // To run a batch file, you must start the command interpreter; set lpApplicationName to cmd.exe and
        // set lpCommandLine to the following arguments: /c plus the name of the batch file.
        //
        // https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw
        let inter = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd".to_string());
        let args = format!("/C {:?}", file.as_ref());

        Self::default().application(inter).commandline(args)
    }

    /// Runs a command from `cmd.exe`
    pub fn cmd(commandline: impl AsRef<str>) -> Self {
        let args = format!("cmd /C {}", commandline.as_ref());

        Self::default().commandline(args)
    }

    /// Sets commandline argument.
    pub fn commandline(mut self, cmd: impl Into<String>) -> Self {
        self.commandline = Some(cmd.into());
        self
    }

    /// Sets application argument.
    /// Must be a path to a binary.
    pub fn application(mut self, application: impl Into<String>) -> Self {
        self.application = Some(application.into());
        self
    }

    /// Sets current dir.
    pub fn current_dir(mut self, dir: impl Into<String>) -> Self {
        self.current_dir = Some(dir.into());
        self
    }

    /// Sets a list of arguments as process arguments.
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Adds an argument to a list of process arguments.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Sets a list of env variables as process env variables.
    ///
    /// If envs isn't set they will be inhirited from parent process.
    pub fn envs(mut self, env: HashMap<String, String>) -> Self {
        self.env = Some(env);
        self
    }

    /// Adds an env variable to process env variables list.
    ///
    /// If any envs isn't added the environment list will be inhirited from parent process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        match &mut self.env {
            Some(env) => {
                env.insert(key.into(), value.into());
                self
            }
            None => self.envs(HashMap::new()).env(key.into(), value.into()),
        }
    }

    /// Spawns a process with set attributes.
    pub fn spawn(self) -> Result<Process, Error> {
        Process::spawn(self)
    }
}

fn enableVirtualTerminalSequenceProcessing() -> win::Result<()> {
    let stdout_h = stdout_handle()?;
    unsafe {
        let mut mode = CONSOLE_MODE::default();
        GetConsoleMode(stdout_h, &mut mode).ok()?;
        mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING; // DISABLE_NEWLINE_AUTO_RETURN
        SetConsoleMode(stdout_h, mode).ok()?;

        CloseHandle(stdout_h);
    }

    Ok(())
}

fn createPseudoConsole() -> win::Result<(HPCON, HANDLE, HANDLE)> {
    let (pty_in, con_writer) = pipe()?;
    let (con_reader, pty_out) = pipe()?;

    let size = inhirentConsoleSize()?;

    let console = unsafe { CreatePseudoConsole(size, pty_in, pty_out, 0)? };

    // Note: We can close the handles to the PTY-end of the pipes here
    // because the handles are dup'ed into the ConHost and will be released
    // when the ConPTY is destroyed.
    unsafe {
        CloseHandle(pty_in);
    }
    unsafe {
        CloseHandle(pty_out);
    }

    Ok((console, con_reader, con_writer))
}

fn inhirentConsoleSize() -> win::Result<COORD> {
    let stdout_h = stdout_handle()?;
    let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
    unsafe {
        GetConsoleScreenBufferInfo(stdout_h, &mut info).ok()?;
        CloseHandle(stdout_h);
    };

    let mut size = COORD { X: 24, Y: 80 };
    size.X = info.srWindow.Right - info.srWindow.Left + 1;
    size.Y = info.srWindow.Bottom - info.srWindow.Top + 1;

    Ok(size)
}

// const PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE: usize = 22 | 0x0002_0000;
const PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE: usize = 0x00020016;

fn initializeStartupInfoAttachedToConPTY(hPC: &mut HPCON) -> win::Result<STARTUPINFOEXW> {
    let mut siEx = STARTUPINFOEXW::default();
    siEx.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;

    let mut size: usize = 0;
    let res = unsafe { InitializeProcThreadAttributeList(null_mut() as _, 1, 0, &mut size) };
    if res.as_bool() || size == 0 {
        return Err(win::Error::new(
            HRESULT::default(),
            "failed initialize proc attribute list".into(),
        ));
    }

    // SAFETY
    // we leak the memory intentionally,
    // it will be freed on DROP.
    let lpAttributeList = vec![0u8; size].into_boxed_slice();
    let lpAttributeList = Box::leak(lpAttributeList);

    siEx.lpAttributeList = lpAttributeList.as_mut_ptr().cast() as LPPROC_THREAD_ATTRIBUTE_LIST;

    unsafe {
        InitializeProcThreadAttributeList(siEx.lpAttributeList, 1, 0, &mut size).ok()?;
        UpdateProcThreadAttribute(
            siEx.lpAttributeList,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
            *hPC as _,
            size_of::<HPCON>(),
            null_mut(),
            null_mut(),
        )
        .ok()?;
    }

    Ok(siEx)
}

fn execProc(mut startup_info: STARTUPINFOEXW, attr: ProcAttr) -> win::Result<PROCESS_INFORMATION> {
    if attr.commandline.is_none() && attr.application.is_none() {
        panic!("")
    }

    let commandline = pwstr_param(attr.commandline);
    let application = pwstr_param(attr.application);
    let current_dir = pwstr_param(attr.current_dir);
    let env = match attr.env {
        Some(env) => Box::<[u16]>::into_raw(environment_block_unicode(env).into_boxed_slice()) as _,
        None => null_mut(),
    };

    let mut proc_info = PROCESS_INFORMATION::default();
    let result = unsafe {
        CreateProcessW(
            application.abi(),
            commandline.abi(),
            null_mut(),
            null_mut(),
            false,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT, // CREATE_UNICODE_ENVIRONMENT | CREATE_NEW_CONSOLE
            env,
            current_dir.abi(),
            &mut startup_info.StartupInfo,
            &mut proc_info,
        )
        .ok()
    };

    if !env.is_null() {
        unsafe {
            ::std::boxed::Box::from_raw(env);
        }
    }

    result?;

    Ok(proc_info)
}

fn pipe() -> win::Result<(HANDLE, HANDLE)> {
    let mut p_in = HANDLE::default();
    let mut p_out = HANDLE::default();
    unsafe { CreatePipe(&mut p_in, &mut p_out, std::ptr::null_mut(), 0).ok()? };

    Ok((p_in, p_out))
}

fn stdout_handle() -> win::Result<HANDLE> {
    // we can't use `GetStdHandle(STD_OUTPUT_HANDLE)`
    // because it doesn't work when the IO is redirected
    //
    // https://stackoverflow.com/questions/33476316/win32-getconsolemode-error-code-6

    if !unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.as_bool() {
        
        unsafe { 
            AllocConsole();
            ShowWindow(GetConsoleWindow(), SW_HIDE);
        } ;
    }

    let hConsole = unsafe {
        CreateFileW(
            "CONOUT$",
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            HANDLE::default(),
        )
        .ok()?
    };

    Ok(hConsole)
}

fn environment_block_unicode(env: HashMap<String, String>) -> Vec<u16> {
    if env.is_empty() {
        // two '\0' in UTF-16/UCS-2
        // four '\0' in UTF-8
        return vec![0, 0];
    }

    let mut b = Vec::new();
    for (key, value) in env {
        let part = format!("{}={}\0", key, value);
        b.extend(part.encode_utf16());
    }

    b.push(0);

    b
}

// if given string is empty there will be produced a "\0" string in UTF-16
fn pwstr_param(s: Option<String>) -> Param<'static, PWSTR> {
    match s {
        Some(s) => {
            // https://github.com/microsoft/windows-rs/blob/ba61866b51bafac94844a242f971739583ffa70e/crates/gen/src/pwstr.rs
            s.into_param()
        }
        None => {
            // the memory will be zeroed
            // https://github.com/microsoft/windows-rs/blob/e1ab47c00b10b220d1372e4cdbe9a689d6365001/src/runtime/param.rs
            Param::None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::iter::FromIterator;

    use super::*;

    #[test]
    fn env_block_test() {
        assert_eq!(
            environment_block_unicode(HashMap::from_iter([("asd".to_string(), "qwe".to_string())])),
            str_to_utf16("asd=qwe\0\0")
        );
        assert!(matches!(environment_block_unicode(HashMap::from_iter([
                ("asd".to_string(), "qwe".to_string()),
                ("zxc".to_string(), "123".to_string())
            ])), s if s == str_to_utf16("asd=qwe\0zxc=123\0\0") || s == str_to_utf16("zxc=123\0asd=qwe\0\0")));
        assert_eq!(
            environment_block_unicode(HashMap::from_iter([])),
            str_to_utf16("\0\0")
        );
    }

    fn str_to_utf16(s: impl AsRef<str>) -> Vec<u16> {
        s.as_ref().encode_utf16().collect()
    }
}
