//! Release-safe client diagnostics.
//!
//! Proxy failures are written to a bounded, per-user log with a stable path.
//! Only the proxy scheme/host/port and a categorized error are retained; URL
//! userinfo and raw credentials are never part of a diagnostic.

use heroinn_util::{
    Error,
    protocol::{
        ProxyContext,
        proxy::{ProxyError, ProxySource},
    },
};
#[cfg(any(not(target_os = "windows"), debug_assertions))]
use simplelog::{ColorChoice, TermLogger, TerminalMode};
use simplelog::{CombinedLogger, Config, LevelFilter, SharedLogger, WriteLogger};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const LOG_FILE_NAME: &str = "heroinn_client.log";
pub const MAX_LOG_BYTES: u64 = 512 * 1024;
pub const ROTATED_LOG_COUNT: usize = 1;
const MAX_DETAIL_CHARS: usize = 2_048;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Install persistent logging. The returned path is the active bounded log.
pub fn init_logging() -> Option<PathBuf> {
    for path in candidate_log_paths() {
        let writer = match BoundedLogWriter::open(path.clone(), MAX_LOG_BYTES) {
            Ok(writer) => writer,
            Err(_error) => {
                #[cfg(not(target_os = "windows"))]
                eprintln!(
                    "heroinn_client: cannot create diagnostic log {}: {_error}",
                    path.display()
                );
                continue;
            }
        };

        let config = Config::default();
        let loggers: Vec<Box<dyn SharedLogger>> =
            vec![WriteLogger::new(LevelFilter::Info, config.clone(), writer)];

        // Release Windows binaries use the GUI subsystem and therefore have no
        // console. The file logger above is their persistent diagnostic path.
        #[cfg(any(not(target_os = "windows"), debug_assertions))]
        let loggers = {
            let mut loggers = loggers;
            loggers.push(TermLogger::new(
                LevelFilter::Info,
                config,
                TerminalMode::Mixed,
                ColorChoice::Auto,
            ));
            loggers
        };

        if CombinedLogger::init(loggers).is_err() {
            return None;
        }

        let _ = LOG_PATH.set(path.clone());
        log::info!("diagnostic log initialized: {}", path.display());
        log::logger().flush();
        return Some(path);
    }

    #[cfg(not(target_os = "windows"))]
    eprintln!("heroinn_client: no writable diagnostic log path is available");
    None
}

pub fn current_log_path() -> Option<&'static Path> {
    LOG_PATH.get().map(PathBuf::as_path)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectFailure {
    pub category: String,
    pub source: Option<ProxySource>,
    pub endpoint: Option<String>,
    pub detail: String,
    pub proxy_related: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataPlaneFailure {
    pub category: String,
    pub operation: String,
    pub source: Option<ProxySource>,
    pub endpoint: Option<String>,
    pub detail: String,
}

impl DataPlaneFailure {
    pub fn from_error(
        operation: &'static str,
        proxy: Option<&ProxyContext>,
        error: &Error,
    ) -> Self {
        let category = match proxy.map(|context| context.endpoint.as_str()) {
            Some(endpoint) if endpoint.starts_with("https://") => "proxy_tls_data_plane",
            Some(endpoint) if endpoint.starts_with("http://") => "proxy_http_data_plane",
            Some(endpoint) if endpoint.starts_with("socks5://") => "proxy_socks_data_plane",
            Some(_) => "proxy_data_plane",
            None => "transport_data_plane",
        };
        Self {
            category: category.to_string(),
            operation: operation.to_string(),
            source: proxy.map(|context| context.source),
            endpoint: proxy.map(|context| safe_endpoint(&context.endpoint)),
            detail: safe_detail(&error.to_string()),
        }
    }

    pub fn log_line(&self) -> String {
        let source = self.source.map(|source| source.to_string());
        let source = source.as_deref().unwrap_or("none");
        let endpoint = self.endpoint.as_deref().unwrap_or("none");
        let log_path = current_log_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unavailable".to_string());
        format!(
            "data_plane_failure category={} source={} endpoint={} operation={} \
             detail=\"{}\" fail_closed=true log=\"{}\"",
            self.category, source, endpoint, self.operation, self.detail, log_path
        )
    }
}

impl ConnectFailure {
    pub fn from_error(error: &Error) -> Self {
        match error {
            Error::Proxy(error) => Self::from_proxy(error),
            error => Self {
                category: "direct_connect".to_string(),
                source: None,
                endpoint: None,
                detail: safe_detail(&error.to_string()),
                proxy_related: false,
            },
        }
    }

    fn from_proxy(error: &ProxyError) -> Self {
        let failure = error.failure();
        Self {
            category: failure.kind.as_str().to_string(),
            source: failure.source,
            endpoint: failure.endpoint.as_deref().map(safe_endpoint),
            detail: safe_detail(&failure.detail),
            proxy_related: true,
        }
    }

    pub fn log_line(&self) -> String {
        let source = self.source.map(|source| source.to_string());
        let source = source.as_deref().unwrap_or("none");
        let endpoint = self.endpoint.as_deref().unwrap_or("none");
        let log_path = current_log_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unavailable".to_string());
        format!(
            "connect_failure category={} source={} endpoint={} detail=\"{}\" \
             fail_closed=true log=\"{}\"",
            self.category, source, endpoint, self.detail, log_path
        )
    }

    #[cfg_attr(not(any(target_os = "windows", test)), allow(dead_code))]
    pub fn user_message(&self) -> String {
        let endpoint = self
            .endpoint
            .as_deref()
            .unwrap_or("no proxy (direct connection)");
        let source = self
            .source
            .map(|source| source.to_string())
            .unwrap_or_else(|| "none".to_string());
        let log_path = current_log_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "diagnostic log unavailable".to_string());

        format!(
            "Heroinn could not connect.\n\nCategory: {}\nSource: {}\nProxy: {}\nDetail: {}\n\n\
             The proxy was not bypassed. The client will keep retrying.\n\
             Diagnostic log: {}",
            self.category, source, endpoint, self.detail, log_path
        )
    }
}

pub fn report_connect_failure(error: &Error) {
    let failure = ConnectFailure::from_error(error);
    log::error!("{}", failure.log_line());
    log::logger().flush();

    #[cfg(target_os = "windows")]
    if failure.proxy_related {
        show_windows_notice_once(&failure);
    }
}

pub fn report_data_plane_failure(
    operation: &'static str,
    proxy: Option<&ProxyContext>,
    error: &Error,
) {
    let failure = DataPlaneFailure::from_error(operation, proxy, error);
    log::error!("{}", failure.log_line());
    log::logger().flush();
}

pub fn report_local_failure(operation: &'static str, detail: impl std::fmt::Display) {
    let log_path = current_log_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    log::error!(
        "data_plane_failure category=ftp_local_io source=none endpoint=none operation={} \
         detail=\"{}\" fail_closed=true log=\"{}\"",
        operation,
        safe_detail(&detail.to_string()),
        log_path
    );
    log::logger().flush();
}

/// Remove URL userinfo from any text that may reach a log or a dialog.
pub fn redact_userinfo(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;

    while let Some(relative_scheme) = input[cursor..].find("://") {
        let authority_start = cursor + relative_scheme + 3;
        output.push_str(&input[cursor..authority_start]);

        let tail = &input[authority_start..];
        let authority_end = tail
            .find(|character: char| {
                matches!(
                    character,
                    '/' | '?' | '#' | ' ' | '\t' | '\r' | '\n' | ',' | ';' | '"'
                )
            })
            .unwrap_or(tail.len());

        if let Some(relative_at) = tail[..authority_end].rfind('@') {
            output.push_str("<redacted>@");
            cursor = authority_start + relative_at + 1;
        } else {
            cursor = authority_start;
        }
    }

    output.push_str(&input[cursor..]);
    output
}

fn safe_endpoint(endpoint: &str) -> String {
    redact_userinfo(endpoint)
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(MAX_DETAIL_CHARS)
        .collect()
}

fn safe_detail(detail: &str) -> String {
    redact_userinfo(detail)
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(MAX_DETAIL_CHARS)
        .collect::<String>()
        .trim()
        .to_string()
}

fn candidate_log_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(not(target_os = "windows"))]
    {
        let xdg_state_home = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute());
        let home = std::env::var_os("HOME").map(PathBuf::from);
        if let Some(path) = unix_log_path(xdg_state_home.as_deref(), home.as_deref()) {
            paths.push(path);
        }
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        let user_profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
        if let Some(path) = windows_log_path(local_app_data.as_deref(), user_profile.as_deref()) {
            paths.push(path);
        }
    }

    paths.push(
        std::env::temp_dir()
            .join("Heroinn")
            .join("Logs")
            .join(LOG_FILE_NAME),
    );
    paths.dedup();
    paths
}

#[cfg(not(target_os = "windows"))]
fn unix_log_path(xdg_state_home: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    let state_home = xdg_state_home
        .filter(|path| path.is_absolute())
        .map(Path::to_path_buf)
        .or_else(|| home.map(|path| path.join(".local").join("state")))?;
    Some(state_home.join("heroinn").join(LOG_FILE_NAME))
}

#[cfg(target_os = "windows")]
fn windows_log_path(local_app_data: Option<&Path>, user_profile: Option<&Path>) -> Option<PathBuf> {
    let local_app_data = local_app_data
        .map(Path::to_path_buf)
        .or_else(|| user_profile.map(|path| path.join("AppData").join("Local")))?;
    Some(
        local_app_data
            .join("Heroinn")
            .join("Logs")
            .join(LOG_FILE_NAME),
    )
}

struct BoundedLogWriter {
    path: PathBuf,
    file: Option<File>,
    size: u64,
    max_bytes: u64,
    rotated_count: usize,
}

impl BoundedLogWriter {
    fn open(path: PathBuf, max_bytes: u64) -> io::Result<Self> {
        let mut writer = Self {
            path,
            file: None,
            size: 0,
            max_bytes: max_bytes.max(1),
            rotated_count: ROTATED_LOG_COUNT,
        };
        writer.ensure_current()?;
        Ok(writer)
    }

    #[cfg(test)]
    fn open_with_limits(path: PathBuf, max_bytes: u64, rotated_count: usize) -> io::Result<Self> {
        let mut writer = Self {
            path,
            file: None,
            size: 0,
            max_bytes: max_bytes.max(1),
            rotated_count,
        };
        writer.ensure_current()?;
        Ok(writer)
    }

    fn ensure_current(&mut self) -> io::Result<()> {
        if self.file.is_some() {
            return Ok(());
        }

        if fs::metadata(&self.path)
            .map(|metadata| metadata.len() >= self.max_bytes)
            .unwrap_or(false)
        {
            self.rotate_files()?;
        }

        if let Some(parent) = self.path.parent() {
            prepare_directory(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        restrict_file_permissions(&file)?;
        self.size = file.metadata()?.len();
        self.file = Some(file);
        Ok(())
    }

    fn rotate_files(&mut self) -> io::Result<()> {
        self.file.take();
        if self.rotated_count == 0 {
            match fs::remove_file(&self.path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            return Ok(());
        }

        let oldest = self.archive_path(self.rotated_count);
        match fs::remove_file(&oldest) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }

        for index in (1..self.rotated_count).rev() {
            let source = self.archive_path(index);
            let destination = self.archive_path(index + 1);
            match fs::rename(&source, &destination) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }

        match fs::rename(&self.path, self.archive_path(1)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        Ok(())
    }

    fn archive_path(&self, index: usize) -> PathBuf {
        let mut name = OsString::from(self.path.as_os_str());
        name.push(format!(".{index}"));
        PathBuf::from(name)
    }
}

impl Write for BoundedLogWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }

        self.ensure_current()?;
        let bytes_to_keep = buffer.len().min(self.max_bytes as usize);
        if self.size + bytes_to_keep as u64 > self.max_bytes {
            self.rotate_files()?;
            self.ensure_current()?;
        }

        self.file
            .as_mut()
            .expect("log file initialized")
            .write_all(&buffer[..bytes_to_keep])?;
        self.size += bytes_to_keep as u64;
        // A single oversized log record is deliberately truncated. Report the
        // whole input as consumed so callers do not loop and create more files.
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

impl Drop for BoundedLogWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

fn prepare_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }

    #[cfg(not(unix))]
    fs::create_dir_all(path)?;

    Ok(())
}

fn restrict_file_permissions(_file: &File) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        _file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn show_windows_notice_once(failure: &ConnectFailure) {
    use std::collections::HashSet;
    use std::sync::Mutex;

    static NOTIFIED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let notified = NOTIFIED.get_or_init(|| Mutex::new(HashSet::new()));
    let key = format!(
        "{}|{}",
        failure.category,
        failure.endpoint.as_deref().unwrap_or("none")
    );
    let should_show = match notified.lock() {
        Ok(mut entries) => entries.insert(key),
        Err(poisoned) => poisoned.into_inner().insert(key),
    };
    if !should_show {
        return;
    }

    let message = failure.user_message();
    let _ = std::thread::Builder::new()
        .name("heroinn-proxy-notice".to_string())
        .spawn(move || windows_notice::show(&message));
}

#[cfg(target_os = "windows")]
mod windows_notice {
    use std::ffi::c_void;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }

    pub fn show(message: &str) {
        let title = "Heroinn proxy connection failure\0"
            .encode_utf16()
            .collect::<Vec<_>>();
        let message = format!("{message}\0").encode_utf16().collect::<Vec<_>>();
        // MB_ICONERROR | MB_SETFOREGROUND. This runs on a detached thread so a
        // modal dialog can never stop the connect/retry loop.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                message.as_ptr(),
                title.as_ptr(),
                0x0001_0010,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "heroinn-diagnostics-{name}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn resolves_stable_linux_log_path() {
        assert_eq!(
            unix_log_path(
                Some(Path::new("/var/tmp/state")),
                Some(Path::new("/home/alice"))
            ),
            Some(PathBuf::from("/var/tmp/state/heroinn/heroinn_client.log"))
        );
        assert_eq!(
            unix_log_path(Some(Path::new("relative")), Some(Path::new("/home/alice"))),
            Some(PathBuf::from(
                "/home/alice/.local/state/heroinn/heroinn_client.log"
            ))
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn resolves_stable_windows_log_path() {
        assert_eq!(
            windows_log_path(Some(Path::new(r"C:\Users\alice\AppData\Local")), None),
            Some(PathBuf::from(
                r"C:\Users\alice\AppData\Local\Heroinn\Logs\heroinn_client.log"
            ))
        );
        assert_eq!(
            windows_log_path(None, Some(Path::new(r"C:\Users\alice"))),
            Some(PathBuf::from(
                r"C:\Users\alice\AppData\Local\Heroinn\Logs\heroinn_client.log"
            ))
        );
    }

    #[test]
    fn writer_rotates_and_keeps_both_files_bounded() {
        let directory = TestDirectory::new("rotation");
        let path = directory.0.join(LOG_FILE_NAME);
        let mut writer = BoundedLogWriter::open_with_limits(path.clone(), 64, 1).unwrap();

        for _ in 0..5 {
            writer.write_all(&[b'a'; 30]).unwrap();
        }
        writer.flush().unwrap();

        let active = fs::metadata(&path).unwrap().len();
        let rotated = fs::metadata(writer.archive_path(1)).unwrap().len();
        assert!(active <= 64, "active log grew to {active} bytes");
        assert!(rotated <= 64, "rotated log grew to {rotated} bytes");
        assert!(active + rotated <= 128);
    }

    #[test]
    fn oversized_record_is_truncated_to_the_file_limit() {
        let directory = TestDirectory::new("oversized");
        let path = directory.0.join(LOG_FILE_NAME);
        let mut writer = BoundedLogWriter::open_with_limits(path.clone(), 32, 1).unwrap();

        writer.write_all(&[b'x'; 256]).unwrap();
        assert_eq!(writer.size, 32);
        assert_eq!(fs::metadata(&path).unwrap().len(), 32);
    }

    #[cfg(unix)]
    #[test]
    fn log_directory_and_file_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new("permissions");
        let log_directory = directory.0.join("state").join("heroinn");
        let path = log_directory.join(LOG_FILE_NAME);
        let writer = BoundedLogWriter::open(path.clone(), 64).unwrap();

        let directory_mode = fs::metadata(&log_directory).unwrap().permissions().mode();
        let file_mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(directory_mode & 0o777, 0o700);
        assert_eq!(file_mode & 0o777, 0o600);
        drop(writer);
    }

    #[test]
    fn proxy_failure_exposes_category_and_safe_endpoint() {
        let error = Error::Proxy(ProxyError::ProxyAtEndpoint {
            source_kind: Some(ProxySource::Environment),
            endpoint: "http://proxy.example:8080".to_string(),
            source: Box::new(ProxyError::HttpAuthenticationRequired),
        });
        let failure = ConnectFailure::from_error(&error);
        assert_eq!(failure.category, "http_407");
        assert_eq!(failure.source, Some(ProxySource::Environment));
        assert_eq!(
            failure.endpoint.as_deref(),
            Some("http://proxy.example:8080")
        );
        assert!(failure.proxy_related);
        assert!(failure.log_line().contains("source=environment"));
        assert!(failure.log_line().contains("fail_closed=true"));
        assert!(failure.user_message().contains("was not bypassed"));
    }

    #[test]
    fn credentials_are_removed_from_details_and_endpoints() {
        let redacted = redact_userinfo("connect http://alice:top-secret@proxy.example:8080 failed");
        assert!(!redacted.contains("alice"));
        assert!(!redacted.contains("top-secret"));
        assert!(redacted.contains("<redacted>@proxy.example:8080"));

        let detail = safe_detail("http://alice:top-secret@proxy.example:8080\nnext line");
        assert!(!detail.contains("alice"));
        assert!(!detail.contains("top-secret"));
        assert!(!detail.contains('\n'));

        let endpoint = safe_endpoint("http://alice:top-secret@[::1]:8080");
        assert_eq!(endpoint, "http://<redacted>@[::1]:8080");
    }

    #[test]
    fn tls_data_plane_failure_has_category_endpoint_and_fail_closed() {
        let proxy = ProxyContext {
            source: ProxySource::Environment,
            endpoint: "https://proxy.example:8443".to_string(),
        };
        let failure = DataPlaneFailure::from_error(
            "ftp_download_frame",
            Some(&proxy),
            &Error::Other(
                "ws receive error: TLS connection closed; http://alice:secret@proxy.example"
                    .to_string(),
            ),
        );

        assert_eq!(failure.category, "proxy_tls_data_plane");
        assert_eq!(failure.source, Some(ProxySource::Environment));
        assert_eq!(
            failure.endpoint.as_deref(),
            Some("https://proxy.example:8443")
        );
        assert!(failure.log_line().contains("fail_closed=true"));
        assert!(failure.log_line().contains("operation=ftp_download_frame"));
        assert!(!failure.detail.contains("alice"));
        assert!(!failure.detail.contains("secret"));
    }

    #[test]
    fn direct_failure_is_distinct_and_never_mentions_a_proxy_bypass() {
        let failure = ConnectFailure::from_error(&Error::Other(
            "ws connect error: connection refused".to_string(),
        ));
        assert_eq!(failure.category, "direct_connect");
        assert!(failure.endpoint.is_none());
        assert!(!failure.proxy_related);
    }
}
