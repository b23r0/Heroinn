use std::{
    io::{self, Read, Write},
    process::Command,
    thread,
    time::{Duration, Instant},
};

use conpty::{io::PipeReader, Process, ProcessOptions};

const IO_TIMEOUT: Duration = Duration::from_secs(15);

fn spawn_cmd() -> Process {
    Process::spawn(Command::new("cmd.exe")).expect("spawn interactive cmd.exe")
}

fn spawn_cmd_sized(columns: i16, rows: i16) -> Process {
    let mut options = ProcessOptions::default();
    options.set_console_size(Some((columns, rows)));
    options
        .spawn(Command::new("cmd.exe"))
        .expect("spawn interactive cmd.exe with explicit size")
}

fn read_until(reader: &mut PipeReader, needle: &[u8]) -> Vec<u8> {
    reader.blocking(false);

    let deadline = Instant::now() + IO_TIMEOUT;
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];

    while Instant::now() < deadline {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                output.extend_from_slice(&buffer[..read]);
                if contains(&output, needle) {
                    return output;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("read ConPTY output: {error}"),
        }
    }

    panic!(
        "timed out waiting for {:?}; output={:?}",
        String::from_utf8_lossy(needle),
        String::from_utf8_lossy(&output)
    );
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

#[test]
fn interactive_cmd_stays_alive_after_a_command() {
    let mut process = spawn_cmd();
    let mut writer = process.input().expect("open input");
    let mut reader = process.output().expect("open output");

    writer
        .write_all(b"echo HEROINN_CONPTY_INTERACTIVE\r\n")
        .expect("write marker");
    writer.flush().expect("flush marker");

    let output = read_until(&mut reader, b"HEROINN_CONPTY_INTERACTIVE");
    assert!(contains(&output, b"HEROINN_CONPTY_INTERACTIVE"));
    assert!(
        process.is_alive(),
        "interactive cmd.exe exited after one command"
    );

    writer.write_all(b"exit\r\n").expect("write exit");
    writer.flush().expect("flush exit");
    process.exit(0).expect("terminate cmd");
    process.wait(Some(5_000)).expect("wait for cmd");
}

#[test]
fn input_output_resize_and_normal_exit() {
    let mut process = spawn_cmd_sized(80, 25);
    let mut writer = process.input().expect("open input");
    let mut reader = process.output().expect("open output");

    writer
        .write_all(b"echo HEROINN_CONPTY_IO\r\n")
        .expect("write marker");
    writer.flush().expect("flush marker");
    assert!(contains(
        &read_until(&mut reader, b"HEROINN_CONPTY_IO"),
        b"HEROINN_CONPTY_IO"
    ));

    process.resize(40, 12).expect("resize pseudo console");
    read_until(&mut reader, b"\x1b[8;12;40t");

    writer.write_all(b"exit\r\n").expect("write exit");
    writer.flush().expect("flush exit");
    let exit_code = process.wait(Some(5_000)).expect("wait for cmd exit");
    assert_eq!(exit_code, 0);
}

#[test]
fn process_can_move_between_threads_and_be_reclaimed() {
    let process = spawn_cmd();
    let pid = process.pid();

    let worker = thread::spawn(move || {
        let mut process = process;
        let mut writer = process.input().expect("open input on worker");
        let mut reader = process.output().expect("open output on worker");

        writer
            .write_all(b"echo HEROINN_CONPTY_THREAD\r\n")
            .expect("write marker on worker");
        writer.flush().expect("flush marker on worker");
        assert!(contains(
            &read_until(&mut reader, b"HEROINN_CONPTY_THREAD"),
            b"HEROINN_CONPTY_THREAD"
        ));

        process.exit(0).expect("terminate on worker");
        process.wait(Some(5_000)).expect("wait on worker");
        drop(reader);
        drop(writer);
        drop(process);
        pid
    });

    let moved_pid = worker.join().expect("worker with Process must not panic");
    assert_eq!(moved_pid, pid);
}

#[test]
fn repeated_spawn_and_drop_reclaims_children() {
    for index in 0..12 {
        let mut process = spawn_cmd();
        let pid = process.pid();
        let mut writer = process.input().expect("open input");
        let mut reader = process.output().expect("open output");
        let marker = format!("HEROINN_CONPTY_REPEAT_{index}");

        writeln!(writer, "echo {marker}").expect("write repeat marker");
        writer.flush().expect("flush repeat marker");
        assert!(contains(
            &read_until(&mut reader, marker.as_bytes()),
            marker.as_bytes()
        ));

        process.exit(0).expect("terminate repeated cmd");
        let exit_code = process
            .wait(Some(5_000))
            .unwrap_or_else(|error| panic!("repeat {index} pid={pid} did not exit: {error}"));
        assert_eq!(exit_code, 0, "repeat {index}");
        drop(reader);
        drop(writer);
        drop(process);
    }
}
