use std::ffi::OsString;
use std::path::PathBuf;

use alacritty_terminal::config::{Program, PtyConfig};

use crate::config::window::{Class, Identity};

/// Command-line options required by Heroinn's terminal worker.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub local_socket_port: String,
    pub sub_title: String,
}

impl Options {
    /// Parse the process arguments.
    pub fn parse_env() -> Result<Self, String> {
        Self::parse_from(std::env::args_os().skip(1))
    }

    /// Parse an argument iterator without consulting process globals.
    pub fn parse_from<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let mut local_socket_port = None;
        let mut sub_title = None;
        let mut args = args.into_iter().map(Into::into);

        while let Some(argument) = args.next() {
            let argument =
                argument.into_string().map_err(|_| "arguments must be valid UTF-8".to_string())?;

            let (name, inline_value) = match argument.split_once('=') {
                Some((name, value)) => (name, Some(value.to_string())),
                None => (argument.as_str(), None),
            };

            match name {
                "--local-socket-port" => {
                    if local_socket_port.is_some() {
                        return Err(format!("duplicate argument: {name}"));
                    }
                    let value = take_value(name, inline_value, &mut args)?;
                    validate_port(&value)?;
                    local_socket_port = Some(value);
                },
                "--sub-title" => {
                    if sub_title.is_some() {
                        return Err(format!("duplicate argument: {name}"));
                    }
                    let value = take_value(name, inline_value, &mut args)?;
                    sub_title = Some(value);
                },
                "--help" | "-h" => {
                    return Err(usage());
                },
                _ => return Err(format!("unknown argument: {name}\n{}", usage())),
            }
        }

        Ok(Self {
            local_socket_port: local_socket_port
                .ok_or_else(|| format!("missing --local-socket-port\n{}", usage()))?,
            sub_title: sub_title.ok_or_else(|| format!("missing --sub-title\n{}", usage()))?,
        })
    }
}

fn take_value<I>(name: &str, inline_value: Option<String>, args: &mut I) -> Result<String, String>
where
    I: Iterator<Item = OsString>,
{
    if let Some(value) = inline_value {
        return Ok(value);
    }

    args.next()
        .ok_or_else(|| format!("missing value for {name}"))?
        .into_string()
        .map_err(|_| format!("value for {name} must be valid UTF-8"))
}

fn validate_port(value: &str) -> Result<(), String> {
    match value.parse::<u16>() {
        Ok(port) if port != 0 => Ok(()),
        _ => Err(format!("--local-socket-port must be between 1 and 65535: {value}")),
    }
}

fn usage() -> String {
    "usage: heroinn_shell --local-socket-port <1-65535> --sub-title <title>".to_string()
}

/// Terminal-specific options used when creating additional windows.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct TerminalOptions {
    pub working_directory: Option<PathBuf>,
    pub hold: bool,
    command: Vec<String>,
}

impl TerminalOptions {
    /// Shell override used by the terminal window.
    pub fn command(&self) -> Option<Program> {
        let (program, args) = self.command.split_first()?;
        Some(Program::WithArgs { program: program.clone(), args: args.to_vec() })
    }

    /// Override the [`PtyConfig`]'s fields with the [`TerminalOptions`].
    pub fn override_pty_config(&self, pty_config: &mut PtyConfig) {
        if let Some(working_directory) = &self.working_directory {
            if working_directory.is_dir() {
                pty_config.working_directory = Some(working_directory.to_owned());
            } else {
                log::error!("Invalid working directory: {:?}", working_directory);
            }
        }

        if let Some(command) = self.command() {
            pty_config.shell = Some(command);
        }

        pty_config.hold |= self.hold;
    }
}

impl From<TerminalOptions> for PtyConfig {
    fn from(mut options: TerminalOptions) -> Self {
        PtyConfig {
            working_directory: options.working_directory.take(),
            shell: options.command(),
            hold: options.hold,
            local_socket_port: String::new(),
        }
    }
}

/// Window-specific options used when creating additional windows.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct WindowIdentity {
    pub title: Option<String>,
    pub class: Option<Class>,
}

impl WindowIdentity {
    /// Override the [`Identity`]'s fields with the [`WindowIdentity`].
    pub fn override_identity_config(&self, identity: &mut Identity) {
        if let Some(title) = &self.title {
            identity.title = title.clone();
        }
        if let Some(class) = &self.class {
            identity.class = class.clone();
        }
    }
}

/// Options passed to the main terminal window and any additional windows.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct WindowOptions {
    pub terminal_options: TerminalOptions,
    pub window_identity: WindowIdentity,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::window::DEFAULT_NAME;

    fn parse(args: &[&str]) -> Result<Options, String> {
        Options::parse_from(args.iter().copied())
    }

    #[test]
    fn parses_required_heroinn_arguments() {
        let options =
            parse(&["--local-socket-port", "4242", "--sub-title", "alice@example"]).unwrap();
        assert_eq!(options.local_socket_port, "4242");
        assert_eq!(options.sub_title, "alice@example");
    }

    #[test]
    fn parses_equals_form() {
        let options = parse(&["--local-socket-port=65535", "--sub-title=test"]).unwrap();
        assert_eq!(options.local_socket_port, "65535");
        assert_eq!(options.sub_title, "test");
    }

    #[test]
    fn accepts_an_empty_subtitle_for_loopback_only_sessions() {
        let options = parse(&["--local-socket-port", "4242", "--sub-title="]).unwrap();
        assert_eq!(options.local_socket_port, "4242");
        assert!(options.sub_title.is_empty());
    }

    #[test]
    fn rejects_missing_duplicate_and_unknown_arguments() {
        assert!(parse(&[]).is_err());
        assert!(parse(&["--local-socket-port", "1"]).is_err());
        assert!(parse(&[
            "--local-socket-port",
            "1",
            "--local-socket-port",
            "2",
            "--sub-title",
            "x"
        ])
        .is_err());
        assert!(parse(&["--local-socket-port", "1", "--sub-title", "x", "--other"]).is_err());
    }

    #[test]
    fn rejects_invalid_ports() {
        for port in ["0", "65536", "abc", "-1"] {
            assert!(parse(&["--local-socket-port", port, "--sub-title", "x"]).is_err());
        }
    }

    #[test]
    fn dynamic_title_is_preserved_by_default() {
        let config = crate::config::UiConfig::default();
        let old_dynamic_title = config.window.dynamic_title;
        let identity = WindowIdentity::default();
        let mut target = config.window.identity.clone();
        identity.override_identity_config(&mut target);
        assert_eq!(old_dynamic_title, config.window.dynamic_title);
        assert_eq!(target.class.general, DEFAULT_NAME);
    }
}
