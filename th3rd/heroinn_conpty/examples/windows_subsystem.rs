#![windows_subsystem = "windows"]

use std::{
    fs,
    io::{Read, Write},
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn main() {
    let marker = std::env::var_os("HEROINN_CONPTY_HIDDEN_MARKER")
        .expect("HEROINN_CONPTY_HIDDEN_MARKER is required");

    let mut process =
        conpty::Process::spawn(Command::new("cmd.exe")).expect("spawn interactive cmd.exe");
    let mut writer = process.input().expect("open ConPTY input");
    let mut reader = process.output().expect("open ConPTY output");
    writer
        .write_all(b"echo HEROINN_HIDDEN_CONSOLE\r\n")
        .expect("write marker");
    writer.flush().expect("flush marker");

    reader.blocking(false);
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    while Instant::now() < deadline {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                output.extend_from_slice(&buffer[..read]);
                if output
                    .windows(b"HEROINN_HIDDEN_CONSOLE".len())
                    .any(|window| window == b"HEROINN_HIDDEN_CONSOLE")
                {
                    fs::write(&marker, format!("pid={}\n", process.pid()))
                        .expect("write hidden-console marker");
                    // Keep the ConPTY and console alive long enough for the
                    // interactive harness to inspect visible top-level windows.
                    thread::sleep(Duration::from_secs(2));
                    process.exit(0).expect("terminate cmd");
                    let _ = process.wait(Some(5_000));
                    return;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("read ConPTY output: {error}"),
        }
    }

    panic!(
        "timed out waiting for output: {:?}",
        String::from_utf8_lossy(&output)
    );
}
