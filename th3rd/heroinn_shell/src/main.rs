//! Alacritty - The GPU Enhanced Terminal.

#![warn(rust_2018_idioms, future_incompatible)]
#![deny(clippy::all, clippy::if_not_else, clippy::enum_glob_use)]
// With the default subsystem, 'console', windows creates an additional console
// window for the program.
// This is silently ignored on non-windows systems.
// See https://msdn.microsoft.com/en-us/library/4cc7ya5b.aspx for more details.
#![windows_subsystem = "windows"]

#[cfg(not(any(feature = "x11", feature = "wayland", target_os = "macos", windows)))]
compile_error!(r#"at least one of the "x11"/"wayland" features must be enabled"#);

#[cfg(target_os = "macos")]
use std::env;
use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::PathBuf;
use std::string::ToString;
use std::{fs, process};

use glutin::event_loop::EventLoop as GlutinEventLoop;
#[cfg(all(feature = "x11", not(any(target_os = "macos", windows))))]
use glutin::platform::unix::EventLoopWindowTargetExtUnix;
use log::info;
#[cfg(windows)]
use winapi::um::wincon::{AttachConsole, FreeConsole, ATTACH_PARENT_PROCESS};

use alacritty_terminal::tty;

mod cli;
mod clipboard;
mod config;
mod daemon;
mod display;
mod event;
mod input;
mod logging;
#[cfg(target_os = "macos")]
mod macos;
mod message_bar;
#[cfg(windows)]
mod panic;
mod renderer;
mod scheduler;
mod string;
mod window_context;

mod gl {
    #![allow(clippy::all)]
    include!("gl_bindings.rs");
}

use crate::cli::{Options, WindowOptions};
use crate::config::{monitor, UiConfig};
use crate::event::{Event, Processor};
#[cfg(target_os = "macos")]
use crate::macos::locale;

fn main() {
    #[cfg(windows)]
    panic::attach_handler();

    // When linked with the windows subsystem windows won't automatically attach
    // to the console of the parent process, so we do it explicitly. This fails
    // silently if the parent has no console.
    #[cfg(windows)]
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }

    // Load command line options.
    let options = match Options::parse_env() {
        Ok(options) => options,
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(2);
        },
    };

    let result = alacritty(options);

    // Handle command failure.
    if let Err(err) = result {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

/// Temporary files stored for Alacritty.
///
/// This stores temporary files to automate their destruction through its `Drop` implementation.
struct TemporaryFiles {
    log_file: Option<PathBuf>,
}

impl Drop for TemporaryFiles {
    fn drop(&mut self) {
        // Clean up logfile.
        if let Some(log_file) = &self.log_file {
            if fs::remove_file(log_file).is_ok() {
                let _ = writeln!(io::stdout(), "Deleted log file at \"{}\"", log_file.display());
            }
        }
    }
}

/// Run main Alacritty entrypoint.
///
/// Creates a window, the terminal state, PTY, I/O event loop, input processor,
/// config change monitor, and runs the main display loop.
fn alacritty(options: Options) -> Result<(), String> {
    // Setup glutin event loop.
    let window_event_loop = GlutinEventLoop::<Event>::with_user_event();

    // Initialize the logger as soon as possible as to capture output from other subsystems.
    let log_file =
        logging::initialize(window_event_loop.create_proxy()).expect("Unable to initialize logger");

    info!("Welcome to Alacritty");

    #[cfg(all(feature = "x11", not(any(target_os = "macos", windows))))]
    info!("Running on {}", if window_event_loop.is_x11() { "X11" } else { "Wayland" });
    #[cfg(not(any(feature = "x11", target_os = "macos", windows)))]
    info!("Running on Wayland");

    // Load configuration file.
    let config = config::load(&options);
    log_config_path(&config);

    // Update the log level from config.
    log::set_max_level(config.debug.log_level);

    // Set environment variables.
    tty::setup_env(&config.terminal_config);

    // Switch to home directory.
    #[cfg(target_os = "macos")]
    env::set_current_dir(dirs::home_dir().unwrap()).unwrap();

    // Set macOS locale.
    #[cfg(target_os = "macos")]
    locale::set_locale_environment();

    // Create a config monitor when config was loaded from path.
    //
    // The monitor watches the config file for changes and reloads it. Pending
    // config changes are processed in the main loop.
    if config.live_config_reload {
        monitor::watch(config.config_paths.clone(), window_event_loop.create_proxy());
    }

    // Setup automatic RAII cleanup for our files.
    let log_cleanup = log_file.filter(|_| !config.debug.persistent_logging);
    let _files = TemporaryFiles { log_file: log_cleanup };

    // Event processor.
    let window_options = WindowOptions::default();
    let mut processor = Processor::new(config, &window_event_loop);

    // Create the first Alacritty window.
    let proxy = window_event_loop.create_proxy();
    processor
        .create_window(&window_event_loop, proxy, window_options)
        .map_err(|err| err.to_string())?;

    info!("Initialisation complete");

    // Start event loop and block until shutdown.
    processor.run(window_event_loop);

    // This explicit drop is needed for Windows, ConPTY backend. Otherwise a deadlock can occur.
    // The cause:
    //   - Drop for ConPTY will deadlock if the conout pipe has already been dropped
    //   - ConPTY is dropped when the last of processor and window context are dropped, because both
    //     of them own an Arc<ConPTY>
    //
    // The fix is to ensure that processor is dropped first. That way, when window context (i.e.
    // PTY) is dropped, it can ensure ConPTY is dropped before the conout pipe in the PTY drop
    // order.
    //
    // FIXME: Change PTY API to enforce the correct drop order with the typesystem.
    drop(processor);

    // FIXME patch notify library to have a shutdown method.
    // config_reloader.join().ok();

    // Without explicitly detaching the console cmd won't redraw it's prompt.
    #[cfg(windows)]
    unsafe {
        FreeConsole();
    }

    info!("Goodbye");
    Ok(())
}

fn log_config_path(config: &UiConfig) {
    if config.config_paths.is_empty() {
        return;
    }

    let mut msg = String::from("Configuration files loaded from:");
    for path in &config.config_paths {
        let _ = write!(msg, "\n  {:?}", path.display());
    }

    info!("{}", msg);
}
