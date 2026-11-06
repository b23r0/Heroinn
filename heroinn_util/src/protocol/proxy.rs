//! Proxy discovery and handshakes for outbound HTTP and TCP transports.
//!
//! Discovery only looks at explicit user configuration: environment variables
//! first, then the current desktop user's manual proxy settings. It never scans
//! listening ports. PAC/WPAD is rejected as unsupported rather than silently
//! bypassed.

use std::env;
use std::fmt;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv6Addr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use native_tls::{HandshakeError, TlsConnector, TlsStream};
use thiserror::Error;
use url::{Host, Url};

use crate::error::Result;

pub(crate) const PROXY_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const PROXY_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const MAX_HTTP_PROXY_RESPONSE: usize = 8 * 1024;

/// A proxy configuration or handshake failure.
#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("invalid proxy configuration: {0}")]
    InvalidConfiguration(&'static str),

    #[error("unsupported proxy scheme: {0}")]
    UnsupportedScheme(String),

    #[error("invalid proxy target: {0}")]
    InvalidTarget(&'static str),

    #[error("proxy DNS/connection failed for {0}: {1}")]
    Connect(String, String),

    #[error("proxy handshake timed out")]
    Timeout,

    #[error("proxy handshake I/O failed: {0}")]
    Handshake(String),

    #[error("TLS-to-proxy handshake failed: {0}")]
    TlsHandshake(String),

    #[error("HTTP proxy authentication required (407)")]
    HttpAuthenticationRequired,

    #[error("HTTP proxy returned status {0}")]
    HttpStatus(u16),

    #[error("invalid HTTP proxy response: {0}")]
    HttpResponse(&'static str),

    #[error("SOCKS5 proxy protocol error: {0}")]
    SocksProtocol(&'static str),

    #[error("SOCKS5 proxy authentication failed")]
    SocksAuthentication,

    #[error("SOCKS5 proxy rejected target (REP={0:#04x})")]
    SocksTargetRejected(u8),

    #[error("proxy at {endpoint} failed: {source}")]
    ProxyAtEndpoint {
        source_kind: Option<ProxySource>,
        endpoint: String,
        #[source]
        source: Box<ProxyError>,
    },

    #[error("proxy configuration from {source_kind} failed: {source}")]
    ProxyAtSource {
        source_kind: ProxySource,
        #[source]
        source: Box<ProxyError>,
    },
}

/// Stable, credential-free categories used by client diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProxyFailureKind {
    Configuration,
    UnsupportedScheme,
    InvalidTarget,
    DnsOrConnect,
    Timeout,
    Handshake,
    TlsHandshake,
    HttpAuthenticationRequired,
    HttpStatus,
    HttpResponse,
    SocksProtocol,
    SocksAuthentication,
    SocksTargetRejected,
}

impl ProxyFailureKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Configuration => "proxy_configuration",
            Self::UnsupportedScheme => "proxy_unsupported_scheme",
            Self::InvalidTarget => "proxy_target_invalid",
            Self::DnsOrConnect => "proxy_dns_or_connect",
            Self::Timeout => "proxy_timeout",
            Self::Handshake => "proxy_handshake",
            Self::TlsHandshake => "proxy_tls_handshake",
            Self::HttpAuthenticationRequired => "http_407",
            Self::HttpStatus => "http_status",
            Self::HttpResponse => "http_response",
            Self::SocksProtocol => "socks_protocol",
            Self::SocksAuthentication => "socks_authentication",
            Self::SocksTargetRejected => "socks_target_rejected",
        }
    }
}

/// A connected proxy tunnel before it is wrapped by the outbound adapter.
///
/// Keeping this low-level representation in the proxy module lets the TLS
/// session remain owned by one object. The outbound layer is responsible for
/// splitting it into non-blocking reader/writer handles.
#[derive(Debug)]
pub(crate) enum ProxyTransport {
    Plain(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

/// A successfully negotiated proxy transport and its credential-free identity.
pub(crate) struct ConnectedProxyTransport {
    pub transport: ProxyTransport,
    pub source: ProxySource,
    pub endpoint: String,
}

/// A proxy failure rendered with only its safe endpoint and non-secret detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxyFailure {
    pub kind: ProxyFailureKind,
    pub source: Option<ProxySource>,
    pub endpoint: Option<String>,
    pub detail: String,
}

impl ProxyError {
    /// Convert the error into a stable category and credential-free detail.
    pub fn failure(&self) -> ProxyFailure {
        match self {
            Self::ProxyAtEndpoint {
                source_kind,
                endpoint,
                source,
            } => {
                let mut failure = source.failure();
                failure.source = *source_kind;
                failure.endpoint = Some(endpoint.clone());
                failure
            }
            Self::ProxyAtSource {
                source_kind,
                source,
            } => {
                let mut failure = source.failure();
                failure.source = Some(*source_kind);
                failure
            }
            Self::InvalidConfiguration(detail) => ProxyFailure {
                kind: ProxyFailureKind::Configuration,
                source: None,
                endpoint: None,
                detail: (*detail).to_string(),
            },
            Self::UnsupportedScheme(scheme) => ProxyFailure {
                kind: ProxyFailureKind::UnsupportedScheme,
                source: None,
                endpoint: None,
                detail: format!("unsupported proxy scheme {scheme}"),
            },
            Self::InvalidTarget(detail) => ProxyFailure {
                kind: ProxyFailureKind::InvalidTarget,
                source: None,
                endpoint: None,
                detail: (*detail).to_string(),
            },
            Self::Connect(endpoint, detail) => ProxyFailure {
                kind: ProxyFailureKind::DnsOrConnect,
                source: None,
                endpoint: Some(endpoint.clone()),
                detail: detail.clone(),
            },
            Self::Timeout => ProxyFailure {
                kind: ProxyFailureKind::Timeout,
                source: None,
                endpoint: None,
                detail: "proxy connection or handshake timed out".to_string(),
            },
            Self::Handshake(detail) => ProxyFailure {
                kind: ProxyFailureKind::Handshake,
                source: None,
                endpoint: None,
                detail: detail.clone(),
            },
            Self::TlsHandshake(detail) => ProxyFailure {
                kind: ProxyFailureKind::TlsHandshake,
                source: None,
                endpoint: None,
                detail: detail.clone(),
            },
            Self::HttpAuthenticationRequired => ProxyFailure {
                kind: ProxyFailureKind::HttpAuthenticationRequired,
                source: None,
                endpoint: None,
                detail: "HTTP proxy authentication required (407)".to_string(),
            },
            Self::HttpStatus(status) => ProxyFailure {
                kind: ProxyFailureKind::HttpStatus,
                source: None,
                endpoint: None,
                detail: format!("HTTP proxy returned status {status}"),
            },
            Self::HttpResponse(detail) => ProxyFailure {
                kind: ProxyFailureKind::HttpResponse,
                source: None,
                endpoint: None,
                detail: (*detail).to_string(),
            },
            Self::SocksProtocol(detail) => ProxyFailure {
                kind: ProxyFailureKind::SocksProtocol,
                source: None,
                endpoint: None,
                detail: (*detail).to_string(),
            },
            Self::SocksAuthentication => ProxyFailure {
                kind: ProxyFailureKind::SocksAuthentication,
                source: None,
                endpoint: None,
                detail: "SOCKS5 proxy authentication failed".to_string(),
            },
            Self::SocksTargetRejected(reply) => ProxyFailure {
                kind: ProxyFailureKind::SocksTargetRejected,
                source: None,
                endpoint: None,
                detail: format!("SOCKS5 proxy rejected target (REP={reply:#04x})"),
            },
        }
    }
}

/// Supported proxy protocols.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProxyKind {
    Http,
    Https,
    Socks5,
}

impl ProxyKind {
    pub const fn scheme(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::Socks5 => "socks5",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Http => 0,
            Self::Https => 1,
            Self::Socks5 => 2,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct ProxyCredentials {
    username: Vec<u8>,
    password: Vec<u8>,
}

/// One concrete proxy endpoint. Credentials are intentionally omitted from debug output.
#[derive(Clone, PartialEq, Eq)]
pub struct ProxyEndpoint {
    kind: ProxyKind,
    host: String,
    port: u16,
    credentials: Option<ProxyCredentials>,
}

impl fmt::Debug for ProxyEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyEndpoint")
            .field("kind", &self.kind)
            .field("host", &self.host)
            .field("port", &self.port)
            .field(
                "credentials",
                &self.credentials.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

impl ProxyEndpoint {
    pub fn kind(&self) -> ProxyKind {
        self.kind
    }

    pub fn safe_label(&self) -> String {
        let host = if self.host.parse::<Ipv6Addr>().is_ok() {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        format!("{}://{}:{}", self.kind.scheme(), host, self.port)
    }

    fn open(&self) -> Result<TcpStream> {
        let addrs = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|e| ProxyError::Connect(self.safe_label(), e.to_string()))?;

        let mut last_error = None;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, PROXY_CONNECT_TIMEOUT) {
                Ok(stream) => {
                    stream
                        .set_nodelay(true)
                        .map_err(|e| ProxyError::Connect(self.safe_label(), e.to_string()))?;
                    stream
                        .set_read_timeout(Some(PROXY_HANDSHAKE_TIMEOUT))
                        .map_err(|e| ProxyError::Connect(self.safe_label(), e.to_string()))?;
                    stream
                        .set_write_timeout(Some(PROXY_HANDSHAKE_TIMEOUT))
                        .map_err(|e| ProxyError::Connect(self.safe_label(), e.to_string()))?;
                    return Ok(stream);
                }
                Err(e) => last_error = Some(e),
            }
        }

        match last_error {
            Some(e) if is_timeout(&e) => Err(ProxyError::Timeout.into()),
            Some(e) => Err(ProxyError::Connect(self.safe_label(), e.to_string()).into()),
            None => Err(ProxyError::Connect(
                self.safe_label(),
                "proxy host resolved to no addresses".to_string(),
            )
            .into()),
        }
    }

    fn connect(
        &self,
        target: &Target,
        tls: std::result::Result<&TlsConnector, &str>,
    ) -> Result<ProxyTransport> {
        let mut stream = self.open()?;
        match self.kind {
            ProxyKind::Http => {
                http_connect(&mut stream, self, target)?;
                Ok(ProxyTransport::Plain(stream))
            }
            ProxyKind::Https => {
                let tls = tls.map_err(|detail| ProxyError::TlsHandshake(detail.to_string()))?;
                let mut tls_stream = tls.connect(&self.host, stream).map_err(map_tls_handshake)?;
                http_connect(&mut tls_stream, self, target)?;
                Ok(ProxyTransport::Tls(Box::new(tls_stream)))
            }
            ProxyKind::Socks5 => {
                socks5_connect(&mut stream, self, target)?;
                Ok(ProxyTransport::Plain(stream))
            }
        }
    }
}

/// Where a route was discovered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProxySource {
    Environment,
    System,
}

impl fmt::Display for ProxySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Environment => "environment",
            Self::System => "system",
        })
    }
}

/// The ordered proxy candidates for one target, or an explicit direct route.
#[derive(Clone, PartialEq, Eq)]
pub struct ProxyRoute {
    source: Option<ProxySource>,
    candidates: Vec<ProxyEndpoint>,
}

impl fmt::Debug for ProxyRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyRoute")
            .field("source", &self.source)
            .field("candidates", &self.candidates)
            .finish()
    }
}

impl ProxyRoute {
    pub fn direct() -> Self {
        Self {
            source: None,
            candidates: Vec::new(),
        }
    }

    pub fn from_candidates(source: ProxySource, mut candidates: Vec<ProxyEndpoint>) -> Self {
        deduplicate_and_sort(&mut candidates);
        if candidates.is_empty() {
            return Self::direct();
        }
        Self {
            source: Some(source),
            candidates,
        }
    }

    pub fn is_direct(&self) -> bool {
        self.candidates.is_empty()
    }

    pub fn source(&self) -> Option<ProxySource> {
        self.source
    }

    pub fn candidates(&self) -> &[ProxyEndpoint] {
        &self.candidates
    }
}

/// Explicit proxy values read from the process environment.
#[derive(Clone, Default)]
pub struct ProxyEnvironment {
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
    pub all_proxy: Option<String>,
    pub no_proxy: Option<String>,
}

impl ProxyEnvironment {
    pub fn from_process() -> Result<Self> {
        Ok(Self {
            http_proxy: first_environment_value(&["http_proxy", "HTTP_PROXY"])?,
            https_proxy: first_environment_value(&["https_proxy", "HTTPS_PROXY"])?,
            all_proxy: first_environment_value(&["all_proxy", "ALL_PROXY"])?,
            no_proxy: first_environment_value(&["no_proxy", "NO_PROXY"])?,
        })
    }

    fn has_proxy(&self) -> bool {
        self.http_proxy.is_some() || self.https_proxy.is_some() || self.all_proxy.is_some()
    }
}

/// Manual proxy settings for the current desktop user.
#[derive(Clone, Debug)]
pub(crate) struct SystemProxy {
    http: Option<ProxyEndpoint>,
    https: Option<ProxyEndpoint>,
    socks5: Option<ProxyEndpoint>,
    bypass: String,
}

impl SystemProxy {
    fn new(
        http: Option<ProxyEndpoint>,
        https: Option<ProxyEndpoint>,
        socks5: Option<ProxyEndpoint>,
        bypass: String,
    ) -> Option<Self> {
        (http.is_some() || https.is_some() || socks5.is_some()).then_some(Self {
            http,
            https,
            socks5,
            bypass,
        })
    }

    fn candidates(&self) -> Vec<ProxyEndpoint> {
        [self.http.clone(), self.https.clone(), self.socks5.clone()]
            .into_iter()
            .flatten()
            .collect()
    }
}

/// A `host:port` destination, with bracketed IPv6 support.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Destination {
    host: String,
    port: u16,
}

/// Internal compatibility name used by transport implementations.
pub(crate) type Target = Destination;

impl Destination {
    /// Parse `host:port`, accepting bracketed IPv6 literals.
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();
        if input.is_empty() || input.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(ProxyError::InvalidTarget("destination is empty or malformed").into());
        }

        let (host, port) = if let Some(rest) = input.strip_prefix('[') {
            let (host, tail) = rest
                .split_once(']')
                .ok_or(ProxyError::InvalidTarget("unterminated IPv6 address"))?;
            let port = tail
                .strip_prefix(':')
                .ok_or(ProxyError::InvalidTarget("destination port is missing"))?;
            (host, port)
        } else {
            if input.matches(':').count() != 1 {
                return Err(ProxyError::InvalidTarget(
                    "IPv6 destinations must be bracketed and ports are required",
                )
                .into());
            }
            input
                .rsplit_once(':')
                .ok_or(ProxyError::InvalidTarget("destination port is missing"))?
        };

        if host.is_empty() || port.is_empty() {
            return Err(ProxyError::InvalidTarget("destination is malformed").into());
        }

        let port =
            port.parse::<u16>()
                .ok()
                .filter(|port| *port != 0)
                .ok_or(ProxyError::InvalidTarget(
                    "destination port must be between 1 and 65535",
                ))?;

        if host.parse::<IpAddr>().is_err() && !valid_domain(host) {
            return Err(ProxyError::InvalidTarget("destination host is invalid").into());
        }

        Ok(Self {
            host: host.to_string(),
            port,
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn authority(&self) -> String {
        if self.host.parse::<Ipv6Addr>().is_ok() {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    fn match_host(&self) -> String {
        self.host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim_end_matches('.')
            .to_ascii_lowercase()
    }
}

/// Resolve proxy configuration for one outbound destination.
pub trait ProxyResolver: Send + Sync {
    fn resolve(&self, destination: &Destination) -> Result<ProxyRoute>;
}

/// Production resolver: environment first, then the current desktop user.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemProxyResolver;

impl ProxyResolver for SystemProxyResolver {
    fn resolve(&self, destination: &Destination) -> Result<ProxyRoute> {
        resolve_target(&destination.authority())
    }
}

/// Test and future-module resolver that returns one fixed route.
#[derive(Clone, Debug)]
pub struct StaticProxyResolver {
    route: ProxyRoute,
}

impl StaticProxyResolver {
    pub fn new(route: ProxyRoute) -> Self {
        Self { route }
    }
}

impl ProxyResolver for StaticProxyResolver {
    fn resolve(&self, _destination: &Destination) -> Result<ProxyRoute> {
        Ok(self.route.clone())
    }
}

/// Explicit direct-only resolver for internal IPC and tests.
#[derive(Clone, Copy, Debug, Default)]
pub struct DirectProxyResolver;

impl ProxyResolver for DirectProxyResolver {
    fn resolve(&self, _destination: &Destination) -> Result<ProxyRoute> {
        Ok(ProxyRoute::direct())
    }
}

/// Resolve the proxy route for a destination using the current process and desktop user.
pub fn resolve_target(target: &str) -> Result<ProxyRoute> {
    let environment = ProxyEnvironment::from_process()
        .map_err(|error| proxy_source_error(error, ProxySource::Environment))?;
    let system = if environment.has_proxy() {
        None
    } else {
        read_system_proxy().map_err(|error| proxy_source_error(error, ProxySource::System))?
    };
    resolve_target_with(target, &environment, system.as_ref())
}

fn proxy_source_error(error: crate::Error, source: ProxySource) -> crate::Error {
    match error {
        crate::Error::Proxy(error) => crate::Error::Proxy(ProxyError::ProxyAtSource {
            source_kind: source,
            source: Box::new(error),
        }),
        error => error,
    }
}

pub(crate) fn resolve_target_with(
    target: &str,
    environment: &ProxyEnvironment,
    system: Option<&SystemProxy>,
) -> Result<ProxyRoute> {
    let (source, candidates, system_bypass) = if environment.has_proxy() {
        let mut candidates = Vec::new();
        for raw in [
            environment.http_proxy.as_deref(),
            environment.https_proxy.as_deref(),
            environment.all_proxy.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            let endpoint = parse_proxy_url(raw)
                .map_err(|error| proxy_source_error(error, ProxySource::Environment))?;
            candidates.push(endpoint);
        }
        (ProxySource::Environment, candidates, None)
    } else if let Some(system) = system {
        (
            ProxySource::System,
            system.candidates(),
            Some(system.bypass.as_str()),
        )
    } else {
        return Ok(ProxyRoute::direct());
    };

    if candidates.is_empty() {
        return Ok(ProxyRoute::direct());
    }
    let mut candidates = candidates;
    deduplicate_and_sort(&mut candidates);
    if candidates.is_empty() {
        return Ok(ProxyRoute::direct());
    }

    let target = Target::parse(target)?;
    let mut bypass = String::new();
    if let Some(value) = environment.no_proxy.as_deref() {
        bypass.push_str(value);
    }
    if let Some(value) = system_bypass {
        if !bypass.is_empty() && !value.is_empty() {
            bypass.push(',');
        }
        bypass.push_str(value);
    }

    if BypassRules::parse(&bypass).matches(&target) {
        return Ok(ProxyRoute::direct());
    }

    Ok(ProxyRoute {
        source: Some(source),
        candidates,
    })
}

fn deduplicate_and_sort(candidates: &mut Vec<ProxyEndpoint>) {
    candidates.sort_by_key(|endpoint| endpoint.kind.rank());

    let mut deduplicated = Vec::with_capacity(candidates.len());
    for candidate in candidates.drain(..) {
        if !deduplicated.contains(&candidate) {
            deduplicated.push(candidate);
        }
    }
    *candidates = deduplicated;
}

pub(crate) fn connect_via_route_with_tls(
    target: &Target,
    route: &ProxyRoute,
    tls: std::result::Result<&TlsConnector, &str>,
) -> Result<ProxyTransport> {
    Ok(connect_via_route_with_tls_metadata(target, route, tls)?.transport)
}

pub(crate) fn connect_via_route_with_tls_metadata(
    target: &Target,
    route: &ProxyRoute,
    tls: std::result::Result<&TlsConnector, &str>,
) -> Result<ConnectedProxyTransport> {
    let mut last_error: Option<crate::Error> = None;

    for candidate in route.candidates() {
        match candidate.connect(target, tls) {
            Ok(stream) => {
                log::debug!(
                    "connected through {} proxy at {}",
                    candidate.kind.scheme(),
                    candidate.safe_label()
                );
                return Ok(ConnectedProxyTransport {
                    transport: stream,
                    source: route
                        .source()
                        .expect("a non-direct proxy route has a source"),
                    endpoint: candidate.safe_label(),
                });
            }
            Err(e) => {
                let proxy_error = match e {
                    crate::Error::Proxy(error) => error,
                    error => ProxyError::Handshake(error.to_string()),
                };
                log::debug!(
                    "{} proxy at {} failed: {}",
                    candidate.kind.scheme(),
                    candidate.safe_label(),
                    proxy_error
                );
                last_error = Some(
                    ProxyError::ProxyAtEndpoint {
                        source_kind: route.source(),
                        endpoint: candidate.safe_label(),
                        source: Box::new(proxy_error),
                    }
                    .into(),
                );
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        ProxyError::InvalidConfiguration("proxy route has no candidates").into()
    }))
}

#[allow(dead_code)]
pub(crate) fn connect_via_route(target: &Target, route: &ProxyRoute) -> Result<ProxyTransport> {
    match TlsConnector::new() {
        Ok(tls) => connect_via_route_with_tls(target, route, Ok(&tls)),
        Err(error) => {
            let detail = error.to_string();
            connect_via_route_with_tls(target, route, Err(&detail))
        }
    }
}

fn first_environment_value(keys: &[&str]) -> Result<Option<String>> {
    for key in keys {
        match env::var(key) {
            Ok(value) if !value.trim().is_empty() => return Ok(Some(value)),
            Ok(_) => {}
            Err(env::VarError::NotPresent) => {}
            Err(env::VarError::NotUnicode(_)) => {
                return Err(ProxyError::InvalidConfiguration(
                    "proxy environment variable is not valid UTF-8",
                )
                .into());
            }
        }
    }
    Ok(None)
}

fn parse_proxy_url(raw: &str) -> Result<ProxyEndpoint> {
    if raw.is_empty() || raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(ProxyError::InvalidConfiguration("proxy URL is malformed").into());
    }

    let url =
        Url::parse(raw).map_err(|_| ProxyError::InvalidConfiguration("proxy URL is malformed"))?;
    let kind = match url.scheme() {
        "http" => ProxyKind::Http,
        "https" => ProxyKind::Https,
        "socks5" => ProxyKind::Socks5,
        scheme => return Err(ProxyError::UnsupportedScheme(scheme.to_string()).into()),
    };

    if url.query().is_some() || url.fragment().is_some() || !matches!(url.path(), "" | "/") {
        return Err(ProxyError::InvalidConfiguration(
            "proxy URL must not contain a path, query or fragment",
        )
        .into());
    }

    let host = match url.host() {
        Some(Host::Domain(domain)) if !domain.is_empty() => domain.to_string(),
        Some(Host::Ipv4(ip)) => ip.to_string(),
        Some(Host::Ipv6(ip)) => ip.to_string(),
        _ => {
            return Err(ProxyError::InvalidConfiguration("proxy host is missing").into());
        }
    };

    let default_port = match kind {
        ProxyKind::Http => Some(80),
        ProxyKind::Https => Some(443),
        ProxyKind::Socks5 => Some(1080),
    };
    let port = url
        .port()
        .or(default_port)
        .filter(|port| *port != 0)
        .ok_or(ProxyError::InvalidConfiguration(
            "proxy port is missing or invalid",
        ))?;

    let credentials = if !url.username().is_empty() || url.password().is_some() {
        let username = percent_decode(url.username())?;
        let password = url
            .password()
            .map(percent_decode)
            .transpose()?
            .unwrap_or_default();
        validate_credentials(&username, &password)?;
        Some(ProxyCredentials { username, password })
    } else {
        None
    };

    Ok(ProxyEndpoint {
        kind,
        host,
        port,
        credentials,
    })
}

fn percent_decode(input: &str) -> Result<Vec<u8>> {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(ProxyError::InvalidConfiguration(
                    "proxy credentials contain invalid percent encoding",
                )
                .into());
            }
            let high = hex_value(bytes[index + 1]).ok_or(ProxyError::InvalidConfiguration(
                "proxy credentials contain invalid percent encoding",
            ))?;
            let low = hex_value(bytes[index + 2]).ok_or(ProxyError::InvalidConfiguration(
                "proxy credentials contain invalid percent encoding",
            ))?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    Ok(decoded)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn validate_credentials(username: &[u8], password: &[u8]) -> Result<()> {
    if username.len() > u8::MAX as usize || password.len() > u8::MAX as usize {
        return Err(ProxyError::InvalidConfiguration(
            "proxy credentials must be at most 255 bytes each",
        )
        .into());
    }

    if username
        .iter()
        .chain(password)
        .any(|byte| *byte == b'\0' || *byte == b'\r' || *byte == b'\n')
    {
        return Err(ProxyError::InvalidConfiguration(
            "proxy credentials contain forbidden control characters",
        )
        .into());
    }

    Ok(())
}

fn valid_domain(host: &str) -> bool {
    if host.len() > 253 || !host.is_ascii() {
        return false;
    }

    host.trim_end_matches('.').split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    })
}

fn http_connect<S>(stream: &mut S, proxy: &ProxyEndpoint, target: &Target) -> Result<()>
where
    S: Read + Write,
{
    let authority = target.authority();
    let mut request = format!(
        "CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\nProxy-Connection: Keep-Alive\r\n"
    )
    .into_bytes();

    if let Some(credentials) = &proxy.credentials {
        let mut basic =
            Vec::with_capacity(credentials.username.len() + credentials.password.len() + 1);
        basic.extend_from_slice(&credentials.username);
        basic.push(b':');
        basic.extend_from_slice(&credentials.password);
        request.extend_from_slice(b"Proxy-Authorization: Basic ");
        request.extend_from_slice(base64_encode(&basic).as_bytes());
        request.extend_from_slice(b"\r\n");
    }
    request.extend_from_slice(b"\r\n");

    stream
        .write_all(&request)
        .and_then(|_| stream.flush())
        .map_err(|e| map_handshake_io("write", e))?;

    let mut response = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        stream
            .read_exact(&mut byte)
            .map_err(|e| map_handshake_io("read", e))?;
        response.push(byte[0]);

        if response.len() > MAX_HTTP_PROXY_RESPONSE {
            return Err(ProxyError::HttpResponse("response headers are too large").into());
        }
        if response.ends_with(b"\r\n\r\n") {
            break;
        }
    }

    let response = std::str::from_utf8(&response)
        .map_err(|_| ProxyError::HttpResponse("response headers are not UTF-8"))?;
    let status_line = response
        .lines()
        .next()
        .ok_or(ProxyError::HttpResponse("status line is missing"))?;
    let mut parts = status_line.split_whitespace();
    let protocol = parts
        .next()
        .ok_or(ProxyError::HttpResponse("status line is malformed"))?;
    if !protocol.starts_with("HTTP/") {
        return Err(ProxyError::HttpResponse("status line is not HTTP").into());
    }
    let status = parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or(ProxyError::HttpResponse("status code is malformed"))?;

    match status {
        200..=299 => Ok(()),
        407 => Err(ProxyError::HttpAuthenticationRequired.into()),
        status => Err(ProxyError::HttpStatus(status).into()),
    }
}

fn socks5_connect(stream: &mut TcpStream, proxy: &ProxyEndpoint, target: &Target) -> Result<()> {
    let methods: &[u8] = if proxy.credentials.is_some() {
        &[0x00, 0x02]
    } else {
        &[0x00]
    };

    let mut greeting = Vec::with_capacity(2 + methods.len());
    greeting.push(0x05);
    greeting.push(methods.len() as u8);
    greeting.extend_from_slice(methods);
    write_all(stream, &greeting)?;

    let mut selected = [0u8; 2];
    read_exact(stream, &mut selected)?;
    if selected[0] != 0x05 {
        return Err(ProxyError::SocksProtocol("invalid greeting version").into());
    }

    match selected[1] {
        0x00 => {}
        0x02 => {
            let credentials = proxy.credentials.as_ref().ok_or(ProxyError::SocksProtocol(
                "proxy selected username/password without credentials",
            ))?;
            let mut auth =
                Vec::with_capacity(3 + credentials.username.len() + credentials.password.len());
            auth.push(0x01);
            auth.push(credentials.username.len() as u8);
            auth.extend_from_slice(&credentials.username);
            auth.push(credentials.password.len() as u8);
            auth.extend_from_slice(&credentials.password);
            write_all(stream, &auth)?;

            let mut reply = [0u8; 2];
            read_exact(stream, &mut reply)?;
            if reply[0] != 0x01 || reply[1] != 0x00 {
                return Err(ProxyError::SocksAuthentication.into());
            }
        }
        0xff => {
            return Err(ProxyError::SocksProtocol("no acceptable authentication method").into());
        }
        _ => {
            return Err(ProxyError::SocksProtocol(
                "proxy selected an unsupported authentication method",
            )
            .into());
        }
    }

    let mut request = Vec::new();
    request.extend_from_slice(&[0x05, 0x01, 0x00]);
    append_socks_address(&mut request, &target.host)?;
    request.extend_from_slice(&target.port.to_be_bytes());
    write_all(stream, &request)?;

    let mut reply = [0u8; 4];
    read_exact(stream, &mut reply)?;
    if reply[0] != 0x05 {
        return Err(ProxyError::SocksProtocol("invalid reply version").into());
    }
    if reply[1] != 0x00 {
        return Err(ProxyError::SocksTargetRejected(reply[1]).into());
    }
    if reply[2] != 0x00 {
        return Err(ProxyError::SocksProtocol("invalid reserved byte in reply").into());
    }

    let address_len = match reply[3] {
        0x01 => 4,
        0x04 => 16,
        0x03 => {
            let mut length = [0u8; 1];
            read_exact(stream, &mut length)?;
            length[0] as usize
        }
        _ => {
            return Err(ProxyError::SocksProtocol("invalid address type in reply").into());
        }
    };

    let mut address_and_port = vec![0u8; address_len + 2];
    read_exact(stream, &mut address_and_port)?;
    Ok(())
}

fn append_socks_address(buffer: &mut Vec<u8>, host: &str) -> Result<()> {
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => {
            buffer.push(0x01);
            buffer.extend_from_slice(&ip.octets());
        }
        Ok(IpAddr::V6(ip)) => {
            buffer.push(0x04);
            buffer.extend_from_slice(&ip.octets());
        }
        Err(_) => {
            let bytes = host.as_bytes();
            if bytes.is_empty() || bytes.len() > u8::MAX as usize {
                return Err(ProxyError::InvalidTarget(
                    "SOCKS5 domain name must be between 1 and 255 bytes",
                )
                .into());
            }
            buffer.push(0x03);
            buffer.push(bytes.len() as u8);
            buffer.extend_from_slice(bytes);
        }
    }
    Ok(())
}

fn write_all(stream: &mut TcpStream, bytes: &[u8]) -> Result<()> {
    stream
        .write_all(bytes)
        .and_then(|_| stream.flush())
        .map_err(|e| map_handshake_io("write", e))
}

fn read_exact(stream: &mut TcpStream, bytes: &mut [u8]) -> Result<()> {
    stream
        .read_exact(bytes)
        .map_err(|e| map_handshake_io("read", e))
}

fn map_handshake_io(operation: &'static str, error: std::io::Error) -> crate::Error {
    if is_timeout(&error) {
        ProxyError::Timeout.into()
    } else {
        ProxyError::Handshake(format!("{operation}: {error}")).into()
    }
}

fn map_tls_handshake(error: HandshakeError<TcpStream>) -> crate::Error {
    match error {
        HandshakeError::Failure(error) => {
            let detail = error.to_string();
            let lower = detail.to_ascii_lowercase();
            if lower.contains("timed out")
                || lower.contains("timeout")
                || lower.contains("would block")
            {
                ProxyError::Timeout.into()
            } else {
                ProxyError::TlsHandshake(detail).into()
            }
        }
        HandshakeError::WouldBlock(_) => {
            ProxyError::TlsHandshake("TLS handshake would block".to_string()).into()
        }
    }
}

fn is_timeout(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
}

fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);

    for chunk in input.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        output.push(ALPHABET[(first >> 2) as usize] as char);
        output.push(ALPHABET[(((first & 0x03) << 4) | (second >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[(((second & 0x0f) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[(third & 0x3f) as usize] as char
        } else {
            '='
        });
    }

    output
}

#[derive(Default)]
struct BypassRules {
    rules: Vec<String>,
}

impl BypassRules {
    fn parse(value: &str) -> Self {
        Self {
            rules: value
                .split([',', ';'])
                .map(str::trim)
                .filter(|rule| !rule.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        }
    }

    fn matches(&self, target: &Target) -> bool {
        self.rules
            .iter()
            .any(|rule| bypass_rule_matches(rule, target))
    }
}

fn bypass_rule_matches(rule: &str, target: &Target) -> bool {
    if rule == "*" {
        return true;
    }

    if rule.eq_ignore_ascii_case("<local>") {
        return target_host_is_local(target);
    }

    if let Some((network, prefix)) = rule.split_once('/') {
        if let (Ok(network), Ok(prefix)) = (network.parse::<IpAddr>(), prefix.parse::<u8>()) {
            if cidr_contains(network, prefix, target) {
                return true;
            }
        }
    }

    let Some((rule_host, rule_port)) = parse_bypass_host(rule) else {
        return false;
    };
    if rule_port.is_some_and(|port| port != target.port) {
        return false;
    }

    let mut subdomains_only = false;
    let mut rule_host = rule_host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if let Some(domain) = rule_host.strip_prefix("*.") {
        rule_host = domain.to_string();
        subdomains_only = true;
    } else if let Some(domain) = rule_host.strip_prefix('.') {
        rule_host = domain.to_string();
    }

    let target_host = target.match_host();
    if rule_host.parse::<IpAddr>().is_ok() {
        return !subdomains_only && target_host == rule_host;
    }
    if subdomains_only {
        target_host
            .strip_suffix(&rule_host)
            .is_some_and(|prefix| prefix.ends_with('.'))
    } else {
        target_host == rule_host
            || target_host
                .strip_suffix(&rule_host)
                .is_some_and(|prefix| prefix.ends_with('.'))
    }
}

fn parse_bypass_host(rule: &str) -> Option<(String, Option<u16>)> {
    if let Some(rest) = rule.strip_prefix('[') {
        let (host, tail) = rest.split_once(']')?;
        if tail.is_empty() {
            return Some((host.to_string(), None));
        }
        let port = tail.strip_prefix(':')?.parse::<u16>().ok()?;
        return (port != 0).then(|| (host.to_string(), Some(port)));
    }

    if rule.matches(':').count() > 1 {
        return Some((rule.to_string(), None));
    }

    match rule.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => {
            let port = port.parse::<u16>().ok().filter(|port| *port != 0)?;
            Some((host.to_string(), Some(port)))
        }
        _ => Some((rule.to_string(), None)),
    }
}

fn target_host_is_local(target: &Target) -> bool {
    match target.host.parse::<IpAddr>() {
        Ok(ip) => ip.is_loopback() || ip.is_unspecified(),
        Err(_) => !target.host.contains('.') || target.host.eq_ignore_ascii_case("localhost"),
    }
}

fn cidr_contains(network: IpAddr, prefix: u8, target: &Target) -> bool {
    let Ok(target) = target.host.parse::<IpAddr>() else {
        return false;
    };

    match (network, target) {
        (IpAddr::V4(network), IpAddr::V4(target)) if prefix <= 32 => {
            prefix_matches(&network.octets(), &target.octets(), prefix)
        }
        (IpAddr::V6(network), IpAddr::V6(target)) if prefix <= 128 => {
            prefix_matches(&network.octets(), &target.octets(), prefix)
        }
        _ => false,
    }
}

fn prefix_matches(left: &[u8], right: &[u8], prefix: u8) -> bool {
    let whole_bytes = prefix as usize / 8;
    let remaining_bits = prefix as usize % 8;
    if left[..whole_bytes] != right[..whole_bytes] {
        return false;
    }
    if remaining_bits == 0 {
        return true;
    }
    let mask = 0xffu8 << (8 - remaining_bits);
    left[whole_bytes] & mask == right[whole_bytes] & mask
}

#[cfg(target_os = "linux")]
fn read_system_proxy() -> Result<Option<SystemProxy>> {
    let Some(mode) = gsettings_get("org.gnome.system.proxy", "mode")? else {
        return Ok(None);
    };
    match parse_gvariant_string(&mode).as_deref() {
        Some("none") => return Ok(None),
        Some("auto") => {
            return Err(ProxyError::InvalidConfiguration(
                "GNOME automatic proxy (PAC/WPAD) is not supported",
            )
            .into());
        }
        Some("manual") => {}
        _ => {
            return Err(ProxyError::InvalidConfiguration("GNOME proxy mode is invalid").into());
        }
    }

    let http_host = parse_gvariant_string(
        &gsettings_get("org.gnome.system.proxy.http", "host")?.unwrap_or_default(),
    )
    .unwrap_or_default();
    let http_enabled = gsettings_get("org.gnome.system.proxy.http", "enabled")?
        .unwrap_or_default()
        .trim()
        .eq_ignore_ascii_case("true");
    let http_port = if http_enabled {
        gsettings_get("org.gnome.system.proxy.http", "port")?
            .unwrap_or_default()
            .trim()
            .parse::<u16>()
            .unwrap_or(0)
    } else {
        0
    };
    let https_host = parse_gvariant_string(
        &gsettings_get_optional("org.gnome.system.proxy.https", "host")?.unwrap_or_default(),
    )
    .unwrap_or_default();
    let https_enabled = gsettings_get_optional("org.gnome.system.proxy.https", "enabled")?
        .map(|value| value.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(true);
    let https_port = if https_enabled {
        gsettings_get_optional("org.gnome.system.proxy.https", "port")?
            .unwrap_or_default()
            .trim()
            .parse::<u16>()
            .unwrap_or(0)
    } else {
        0
    };
    let socks_host = parse_gvariant_string(
        &gsettings_get("org.gnome.system.proxy.socks", "host")?.unwrap_or_default(),
    )
    .unwrap_or_default();
    let socks_port = gsettings_get("org.gnome.system.proxy.socks", "port")?
        .unwrap_or_default()
        .trim()
        .parse::<u16>()
        .unwrap_or(0);
    let bypass = parse_gvariant_string_list(
        &gsettings_get("org.gnome.system.proxy", "ignore-hosts")?.unwrap_or_default(),
    )
    .join(",");

    system_proxy_from_parts(
        &http_host,
        http_port,
        &https_host,
        https_port,
        &socks_host,
        socks_port,
        bypass,
    )
}

#[cfg(target_os = "linux")]
fn gsettings_get(schema: &str, key: &str) -> Result<Option<String>> {
    use std::process::Command;

    match Command::new("gsettings")
        .args(["get", schema, key])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8(output.stdout).map(Some).map_err(|_| {
                ProxyError::InvalidConfiguration("GNOME proxy setting is not UTF-8").into()
            })
        }
        Ok(_) => {
            Err(ProxyError::InvalidConfiguration("GNOME proxy settings are unavailable").into())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(ProxyError::InvalidConfiguration("failed to run gsettings").into()),
    }
}

#[cfg(target_os = "linux")]
fn gsettings_get_optional(schema: &str, key: &str) -> Result<Option<String>> {
    use std::process::Command;

    match Command::new("gsettings")
        .args(["get", schema, key])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8(output.stdout).map(Some).map_err(|_| {
                ProxyError::InvalidConfiguration("GNOME proxy setting is not UTF-8").into()
            })
        }
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(ProxyError::InvalidConfiguration("failed to run gsettings").into()),
    }
}

#[cfg(target_os = "windows")]
fn read_system_proxy() -> Result<Option<SystemProxy>> {
    read_windows_system_proxy()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn read_system_proxy() -> Result<Option<SystemProxy>> {
    Ok(None)
}

#[cfg(target_os = "windows")]
fn read_windows_system_proxy() -> Result<Option<SystemProxy>> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let settings = match current_user.open_subkey(INTERNET_SETTINGS) {
        Ok(settings) => settings,
        Err(_) => return Ok(None),
    };

    let enabled = settings.get_value::<u32, _>("ProxyEnable").unwrap_or(0) != 0;
    let auto_config = settings
        .get_value::<String, _>("AutoConfigURL")
        .unwrap_or_default();
    let auto_detect = settings.get_value::<u32, _>("AutoDetect").unwrap_or(0) != 0;
    let server = settings
        .get_value::<String, _>("ProxyServer")
        .unwrap_or_default();
    let bypass = settings
        .get_value::<String, _>("ProxyOverride")
        .unwrap_or_default();

    windows_proxy_from_settings(
        enabled,
        &auto_config,
        auto_detect,
        &server,
        &bypass,
        read_windows_proxy_endpoint,
    )
}

#[cfg(any(target_os = "windows", test))]
fn windows_proxy_from_settings<F>(
    enabled: bool,
    auto_config: &str,
    auto_detect: bool,
    server: &str,
    bypass: &str,
    parse_endpoint: F,
) -> Result<Option<SystemProxy>>
where
    F: Fn(ProxyKind, &str) -> Result<ProxyEndpoint>,
{
    let pac_configured = !auto_config.trim().is_empty() || auto_detect;
    if !enabled && pac_configured {
        return Err(ProxyError::InvalidConfiguration(
            "Windows automatic proxy (PAC/WPAD) is not supported",
        )
        .into());
    }
    if !enabled {
        return Ok(None);
    }
    if server.trim().is_empty() {
        if pac_configured {
            return Err(ProxyError::InvalidConfiguration(
                "Windows automatic proxy (PAC/WPAD) is not supported",
            )
            .into());
        }
        return Err(ProxyError::InvalidConfiguration("Windows ProxyServer is missing").into());
    }

    system_proxy_from_windows_server(server, bypass, parse_endpoint)
}

#[cfg(any(target_os = "linux", test))]
fn system_proxy_from_parts(
    http_host: &str,
    http_port: u16,
    https_host: &str,
    https_port: u16,
    socks_host: &str,
    socks_port: u16,
    bypass: String,
) -> Result<Option<SystemProxy>> {
    let http_host = http_host.trim();
    let https_host = https_host.trim();
    let socks_host = socks_host.trim();

    let http = system_proxy_endpoint(ProxyKind::Http, http_host, http_port)?;
    let https = system_proxy_endpoint(ProxyKind::Http, https_host, https_port)?;
    let socks5 = system_proxy_endpoint(ProxyKind::Socks5, socks_host, socks_port)?;

    Ok(SystemProxy::new(http, https, socks5, bypass))
}

#[cfg(any(target_os = "linux", test))]
fn system_proxy_endpoint(
    default_kind: ProxyKind,
    host: &str,
    port: u16,
) -> Result<Option<ProxyEndpoint>> {
    let host = host.trim();
    if host.is_empty() || port == 0 {
        return Ok(None);
    }

    let value = if host.contains("://") {
        let mut url = Url::parse(host)
            .map_err(|_| ProxyError::InvalidConfiguration("system proxy URL is malformed"))?;
        if url.port().is_none() {
            url.set_port(Some(port)).map_err(|_| {
                ProxyError::InvalidConfiguration("system proxy URL cannot carry a port")
            })?;
        }
        url.to_string()
    } else {
        format!(
            "{}://{}:{}",
            default_kind.scheme(),
            bracket_if_ipv6(host),
            port
        )
    };

    parse_proxy_url(&value).map(Some)
}

#[cfg(any(target_os = "linux", test))]
fn bracket_if_ipv6(host: &str) -> String {
    if host.parse::<Ipv6Addr>().is_ok() && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

#[cfg(any(target_os = "windows", test))]
fn system_proxy_from_windows_server<F>(
    server: &str,
    bypass: &str,
    parse_endpoint: F,
) -> Result<Option<SystemProxy>>
where
    F: Fn(ProxyKind, &str) -> Result<ProxyEndpoint>,
{
    let server = server.trim();
    if server.is_empty() {
        return Err(ProxyError::InvalidConfiguration("Windows ProxyServer is empty").into());
    }

    let (mut http, mut https, mut socks5) = (None, None, None);
    if server.contains('=') {
        for entry in server.split(';').filter(|entry| !entry.trim().is_empty()) {
            let Some((name, value)) = entry.split_once('=') else {
                continue;
            };
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            match name.trim().to_ascii_lowercase().as_str() {
                "http" if http.is_none() => http = Some(parse_endpoint(ProxyKind::Http, value)?),
                "https" if https.is_none() => https = Some(parse_endpoint(ProxyKind::Http, value)?),
                "socks" | "socks5" if socks5.is_none() => {
                    socks5 = Some(parse_endpoint(ProxyKind::Socks5, value)?)
                }
                _ => {}
            }
        }
    } else {
        http = Some(parse_endpoint(ProxyKind::Http, server)?);
    }

    Ok(SystemProxy::new(http, https, socks5, bypass.to_string()))
}

#[cfg(any(target_os = "windows", test))]
fn read_windows_proxy_endpoint(kind: ProxyKind, value: &str) -> Result<ProxyEndpoint> {
    let value = value.trim();
    let value = if value.contains("://") {
        value.to_string()
    } else {
        format!("{}://{}", kind.scheme(), value)
    };
    parse_proxy_url(&value)
}

#[cfg(any(target_os = "linux", test))]
fn parse_gvariant_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() < 2 {
        return None;
    }
    let quote = value.as_bytes()[0];
    if (quote != b'\'' && quote != b'"') || value.as_bytes()[value.len() - 1] != quote {
        return None;
    }
    Some(
        value[1..value.len() - 1]
            .replace("\\'", "'")
            .replace("\\\"", "\""),
    )
}

#[cfg(any(target_os = "linux", test))]
fn parse_gvariant_string_list(value: &str) -> Vec<String> {
    let value = value.trim();
    let value = value
        .strip_prefix("@as ")
        .unwrap_or(value)
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    let mut items = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if character == active_quote {
                quote = None;
                items.push(std::mem::take(&mut current));
            } else {
                current.push(character);
            }
            continue;
        }
        if character == '\'' || character == '"' {
            quote = Some(character);
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::{Mutex, mpsc};
    use std::thread;

    static ENVIRONMENT_LOCK: Mutex<()> = Mutex::new(());

    struct EnvironmentRestore {
        values: Vec<(&'static str, Option<OsString>)>,
    }

    impl EnvironmentRestore {
        fn capture(keys: &[&'static str]) -> Self {
            Self {
                values: keys
                    .iter()
                    .map(|key| (*key, std::env::var_os(key)))
                    .collect(),
            }
        }

        fn set(&self, key: &str, value: Option<&str>) {
            // The caller holds ENVIRONMENT_LOCK for the complete mutation.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    impl Drop for EnvironmentRestore {
        fn drop(&mut self) {
            for (key, value) in &self.values {
                // The caller holds ENVIRONMENT_LOCK through this drop.
                unsafe {
                    match value {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    fn endpoint(url: &str) -> ProxyEndpoint {
        parse_proxy_url(url).unwrap()
    }

    fn environment(
        http: Option<&str>,
        all: Option<&str>,
        no_proxy: Option<&str>,
    ) -> ProxyEnvironment {
        ProxyEnvironment {
            http_proxy: http.map(ToOwned::to_owned),
            https_proxy: None,
            all_proxy: all.map(ToOwned::to_owned),
            no_proxy: no_proxy.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn supports_http_and_socks_urls_with_redacted_credentials() {
        let http = endpoint("http://alice:s%40cret@proxy.example:8080");
        assert_eq!(http.kind, ProxyKind::Http);
        assert_eq!(http.host, "proxy.example");
        assert_eq!(http.port, 8080);
        assert_eq!(
            http.credentials.as_ref().unwrap().username,
            b"alice".to_vec()
        );
        assert_eq!(
            http.credentials.as_ref().unwrap().password,
            b"s@cret".to_vec()
        );
        assert!(!format!("{http:?}").contains("s@cret"));
        assert!(!http.safe_label().contains("alice"));

        let socks = endpoint("socks5://bob:password@[::1]:1080");
        assert_eq!(socks.kind, ProxyKind::Socks5);
        assert_eq!(socks.host, "::1");
        assert_eq!(socks.safe_label(), "socks5://[::1]:1080");
    }

    #[test]
    fn first_environment_value_uses_key_order_and_ignores_empty_values() {
        let _guard = ENVIRONMENT_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let restore =
            EnvironmentRestore::capture(&["HEROINN_TEST_PROXY_LOWER", "HEROINN_TEST_PROXY_UPPER"]);
        restore.set(
            "HEROINN_TEST_PROXY_LOWER",
            Some("http://lower.example:8080"),
        );
        restore.set(
            "HEROINN_TEST_PROXY_UPPER",
            Some("http://upper.example:8080"),
        );
        assert_eq!(
            first_environment_value(&["HEROINN_TEST_PROXY_LOWER", "HEROINN_TEST_PROXY_UPPER",])
                .unwrap()
                .as_deref(),
            Some("http://lower.example:8080")
        );

        restore.set("HEROINN_TEST_PROXY_LOWER", Some("   "));
        assert_eq!(
            first_environment_value(&["HEROINN_TEST_PROXY_LOWER", "HEROINN_TEST_PROXY_UPPER",])
                .unwrap()
                .as_deref(),
            Some("http://upper.example:8080")
        );
    }

    #[test]
    fn process_environment_reads_https_proxy_and_no_proxy() {
        let _guard = ENVIRONMENT_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let restore = EnvironmentRestore::capture(&[
            "http_proxy",
            "HTTP_PROXY",
            "https_proxy",
            "HTTPS_PROXY",
            "all_proxy",
            "ALL_PROXY",
            "no_proxy",
            "NO_PROXY",
        ]);
        restore.set("http_proxy", None);
        restore.set("HTTP_PROXY", None);
        restore.set("https_proxy", None);
        restore.set("HTTPS_PROXY", None);
        restore.set("all_proxy", None);
        restore.set("ALL_PROXY", None);
        restore.set("no_proxy", None);
        restore.set("NO_PROXY", None);
        restore.set("https_proxy", Some("https://tls-proxy.example:9443"));
        restore.set("no_proxy", Some("example.com"));

        let environment = ProxyEnvironment::from_process().unwrap();
        assert_eq!(
            environment.https_proxy.as_deref(),
            Some("https://tls-proxy.example:9443")
        );
        assert_eq!(environment.no_proxy.as_deref(), Some("example.com"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn process_environment_prefers_lowercase_names() {
        let _guard = ENVIRONMENT_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let restore = EnvironmentRestore::capture(&[
            "http_proxy",
            "HTTP_PROXY",
            "all_proxy",
            "ALL_PROXY",
            "no_proxy",
            "NO_PROXY",
        ]);
        restore.set("http_proxy", Some("http://lower-http.example:8080"));
        restore.set("HTTP_PROXY", Some("http://upper-http.example:8080"));
        restore.set("all_proxy", None);
        restore.set("ALL_PROXY", Some("socks5://upper-all.example:1080"));
        restore.set("no_proxy", Some("lower.example"));
        restore.set("NO_PROXY", Some("upper.example"));

        let environment = ProxyEnvironment::from_process().unwrap();
        assert_eq!(
            environment.http_proxy.as_deref(),
            Some("http://lower-http.example:8080")
        );
        assert_eq!(
            environment.all_proxy.as_deref(),
            Some("socks5://upper-all.example:1080")
        );
        assert_eq!(environment.no_proxy.as_deref(), Some("lower.example"));
    }

    #[test]
    fn parses_ipv4_and_bracketed_ipv6_targets() {
        assert_eq!(
            Target::parse("127.0.0.1:443").unwrap().authority(),
            "127.0.0.1:443"
        );
        assert_eq!(
            Target::parse("[2001:db8::1]:8443").unwrap().authority(),
            "[2001:db8::1]:8443"
        );
        for invalid in [
            "",
            "example.com",
            "example.com:0",
            "example.com:65536",
            "2001:db8::1:443",
            "[2001:db8::1]",
        ] {
            assert!(Target::parse(invalid).is_err(), "{invalid} should fail");
        }
    }

    #[test]
    fn rejects_unsupported_or_ambiguous_proxy_urls() {
        for url in [
            "socks5h://proxy.example:1080",
            "http://proxy.example:8080/path",
            "http://proxy.example:8080?x=1",
            "http://proxy.example:0",
            "not a url",
        ] {
            assert!(parse_proxy_url(url).is_err(), "{url} should fail");
        }
        let https = endpoint("https://proxy.example");
        assert_eq!(https.kind, ProxyKind::Https);
        assert_eq!(https.port, 443);
        assert_eq!(endpoint("socks5://proxy.example").port, 1080);
    }

    #[test]
    fn environment_proxies_override_system_and_prefer_http() {
        let system = SystemProxy::new(
            Some(endpoint("http://system-http.example:8000")),
            None,
            Some(endpoint("socks5://system-socks.example:1080")),
            String::new(),
        );
        let route = resolve_target_with(
            "server.example:443",
            &environment(
                Some("http://env-http.example:8080"),
                Some("socks5://env-socks.example:1080"),
                None,
            ),
            system.as_ref(),
        )
        .unwrap();

        assert_eq!(route.source(), Some(ProxySource::Environment));
        let candidates = route.candidates();
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].kind, ProxyKind::Http);
        assert_eq!(candidates[0].host, "env-http.example");
        assert_eq!(candidates[1].kind, ProxyKind::Socks5);
        assert_eq!(candidates[1].host, "env-socks.example");
    }

    #[test]
    fn http_is_preferred_within_one_source_even_from_the_lower_priority_variable() {
        let route = resolve_target_with(
            "server.example:443",
            &environment(
                Some("socks5://env-socks.example:1080"),
                Some("http://env-http.example:8080"),
                None,
            ),
            None,
        )
        .unwrap();

        assert_eq!(route.candidates()[0].kind, ProxyKind::Http);
        assert_eq!(route.candidates()[0].host, "env-http.example");
        assert_eq!(route.candidates()[1].kind, ProxyKind::Socks5);
        assert_eq!(route.candidates()[1].host, "env-socks.example");
    }

    #[test]
    fn valid_environment_overrides_system_even_with_only_socks() {
        let system = SystemProxy::new(
            Some(endpoint("http://system-http.example:8000")),
            None,
            None,
            String::new(),
        );
        let route = resolve_target_with(
            "server.example:443",
            &environment(None, Some("socks5://env-socks.example:1080"), None),
            system.as_ref(),
        )
        .unwrap();
        assert_eq!(route.source(), Some(ProxySource::Environment));
        assert_eq!(route.candidates().len(), 1);
        assert_eq!(route.candidates()[0].kind, ProxyKind::Socks5);
    }

    #[test]
    fn https_proxy_variable_and_explicit_https_scheme_are_distinct() {
        let route = resolve_target_with(
            "server.example:443",
            &ProxyEnvironment {
                http_proxy: None,
                https_proxy: Some("https://tls-proxy.example".to_string()),
                all_proxy: None,
                no_proxy: None,
            },
            None,
        )
        .unwrap();
        assert_eq!(route.candidates()[0].kind, ProxyKind::Https);
        assert_eq!(route.candidates()[0].port, 443);

        let route = resolve_target_with(
            "server.example:443",
            &ProxyEnvironment {
                http_proxy: Some("https://tls-proxy.example:9443".to_string()),
                https_proxy: Some("http://plain-proxy.example:8080".to_string()),
                all_proxy: None,
                no_proxy: None,
            },
            None,
        )
        .unwrap();
        assert_eq!(route.candidates().len(), 2);
        assert_eq!(route.candidates()[0].kind, ProxyKind::Http);
        assert_eq!(route.candidates()[1].kind, ProxyKind::Https);
    }

    #[test]
    fn system_route_uses_http_then_socks_and_returns_direct_without_configuration() {
        let system = SystemProxy::new(
            Some(endpoint("http://system-http.example:8000")),
            Some(endpoint("https://system-https.example:8443")),
            Some(endpoint("socks5://system-socks.example:1080")),
            String::new(),
        );
        let route = resolve_target_with(
            "server.example:443",
            &environment(None, None, None),
            system.as_ref(),
        )
        .unwrap();
        assert_eq!(route.source(), Some(ProxySource::System));
        assert_eq!(route.candidates()[0].kind, ProxyKind::Http);
        assert_eq!(route.candidates()[1].kind, ProxyKind::Https);
        assert_eq!(route.candidates()[2].kind, ProxyKind::Socks5);

        let direct =
            resolve_target_with("server.example:443", &environment(None, None, None), None)
                .unwrap();
        assert!(direct.is_direct());

        let unparsed =
            resolve_target_with("not-a-host-port", &environment(None, None, None), None).unwrap();
        assert!(unparsed.is_direct());
    }

    #[test]
    fn invalid_environment_fails_closed_instead_of_using_system() {
        let system = SystemProxy::new(
            Some(endpoint("http://system-http.example:8000")),
            None,
            None,
            String::new(),
        );
        let error = resolve_target_with(
            "server.example:443",
            &environment(
                Some("ftp://proxy.example:21"),
                Some("socks5://fallback.example:1080"),
                None,
            ),
            system.as_ref(),
        )
        .unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected proxy error");
        };
        let failure = error.failure();
        assert_eq!(failure.kind, ProxyFailureKind::UnsupportedScheme);
        assert_eq!(failure.source, Some(ProxySource::Environment));
    }

    #[test]
    fn no_proxy_matches_host_boundaries_ips_and_ports() {
        let cases = [
            ("example.com", "example.com:443", true),
            ("example.com", "api.example.com:443", true),
            ("example.com", "notexample.com:443", false),
            ("*.example.com", "api.example.com:443", true),
            ("*.example.com", "example.com:443", false),
            (".example.com", "deep.api.example.com:443", true),
            ("example.com:8443", "example.com:8443", true),
            ("example.com:8443", "example.com:443", false),
            ("127.0.0.1", "127.0.0.1:443", true),
            ("127.0.0.1", "host.127.0.0.1:443", false),
            ("10.0.0.0/8", "10.4.5.6:443", true),
            ("10.0.0.0/8", "11.4.5.6:443", false),
            ("::1", "[::1]:443", true),
            ("2001:db8::/32", "[2001:db8::1]:443", true),
            ("<local>", "printer:443", true),
            ("<local>", "printer.example:443", false),
        ];

        for (rule, target, expected) in cases {
            let target = Target::parse(target).unwrap();
            assert_eq!(
                BypassRules::parse(rule).matches(&target),
                expected,
                "{rule} vs {target:?}"
            );
        }
    }

    #[test]
    fn bypassed_target_is_direct_even_when_proxy_is_configured() {
        let route = resolve_target_with(
            "notexample.com:443",
            &environment(Some("http://proxy.example:8080"), None, Some("example.com")),
            None,
        )
        .unwrap();
        assert_eq!(route.source(), Some(ProxySource::Environment));

        let route = resolve_target_with(
            "example.com:443",
            &environment(Some("http://proxy.example:8080"), None, Some("example.com")),
            None,
        )
        .unwrap();
        assert!(route.is_direct());
    }

    #[test]
    fn system_bypass_and_environment_no_proxy_are_both_honored() {
        let system = SystemProxy::new(
            Some(endpoint("http://system-http.example:8000")),
            None,
            None,
            "*.internal.example".to_string(),
        );
        let route = resolve_target_with(
            "api.internal.example:443",
            &environment(None, None, None),
            system.as_ref(),
        )
        .unwrap();
        assert!(route.is_direct());

        let route = resolve_target_with(
            "api.internal.example:443",
            &environment(None, None, Some("other.example")),
            system.as_ref(),
        )
        .unwrap();
        assert!(route.is_direct());
    }

    #[test]
    fn parses_windows_proxy_server_forms() {
        let system = system_proxy_from_windows_server(
            "http=proxy.example:8080;https=https://secure.example:8443;socks=localhost:1080",
            "*.corp.example;<local>",
            read_windows_proxy_endpoint,
        )
        .unwrap()
        .unwrap();
        assert_eq!(system.http.as_ref().unwrap().port, 8080);
        assert_eq!(system.https.as_ref().unwrap().kind, ProxyKind::Https);
        assert_eq!(system.https.as_ref().unwrap().port, 8443);
        assert_eq!(system.socks5.as_ref().unwrap().port, 1080);
        assert_eq!(system.bypass, "*.corp.example;<local>");

        let system =
            system_proxy_from_windows_server("proxy.example:3128", "", read_windows_proxy_endpoint)
                .unwrap()
                .unwrap();
        assert_eq!(system.http.as_ref().unwrap().kind, ProxyKind::Http);
        assert!(system.socks5.is_none());

        let system = system_proxy_from_windows_server(
            "https=proxy.example:8443",
            "",
            read_windows_proxy_endpoint,
        )
        .unwrap()
        .unwrap();
        assert_eq!(system.https.as_ref().unwrap().kind, ProxyKind::Http);
    }

    #[test]
    fn windows_pac_only_configuration_fails_closed() {
        let error = windows_proxy_from_settings(
            false,
            "https://config.example/proxy.pac",
            false,
            "",
            "",
            read_windows_proxy_endpoint,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            crate::Error::Proxy(ProxyError::InvalidConfiguration(
                "Windows automatic proxy (PAC/WPAD) is not supported"
            ))
        ));

        let error = windows_proxy_from_settings(
            true,
            "https://config.example/proxy.pac",
            true,
            "",
            "",
            read_windows_proxy_endpoint,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            crate::Error::Proxy(ProxyError::InvalidConfiguration(
                "Windows automatic proxy (PAC/WPAD) is not supported"
            ))
        ));

        assert!(
            windows_proxy_from_settings(false, "", false, "", "", read_windows_proxy_endpoint)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn parses_gnome_values() {
        assert_eq!(parse_gvariant_string("'manual'").as_deref(), Some("manual"));
        assert_eq!(
            parse_gvariant_string_list("['localhost', '10.0.0.0/8', '<local>']"),
            vec!["localhost", "10.0.0.0/8", "<local>"]
        );
        let system = system_proxy_from_parts(
            "proxy.example",
            8080,
            "proxy.example",
            8443,
            "::1",
            1080,
            "localhost,10.0.0.0/8".to_string(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(system.candidates()[0].kind, ProxyKind::Http);
        assert_eq!(system.candidates()[1].kind, ProxyKind::Http);
        assert_eq!(system.candidates()[2].kind, ProxyKind::Socks5);

        let system = system_proxy_from_parts(
            "",
            0,
            "https://tls-proxy.example",
            8443,
            "",
            0,
            String::new(),
        )
        .unwrap()
        .unwrap();
        let https = system.https.as_ref().unwrap();
        assert_eq!(https.kind, ProxyKind::Https);
        assert_eq!(https.port, 8443);
        assert_eq!(https.safe_label(), "https://tls-proxy.example:8443");
    }

    #[test]
    fn base64_matches_rfc_examples() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn http_connect_fixture_accepts_basic_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();

        let proxy_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                request.push_str(&line);
            }
            sender.send(request).unwrap();
            stream
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .unwrap();
        });

        let route = resolve_target_with(
            "127.0.0.1:443",
            &environment(Some(&format!("http://user:pass@{}", address)), None, None),
            None,
        )
        .unwrap();
        let target = Target::parse("127.0.0.1:443").unwrap();
        let stream = connect_via_route(&target, &route).unwrap();
        drop(stream);

        let request = receiver.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(request.starts_with("CONNECT 127.0.0.1:443 HTTP/1.1\r\n"));
        assert!(request.contains("Proxy-Authorization: Basic dXNlcjpwYXNz\r\n"));
        assert!(!request.contains("pass@"));
        proxy_thread.join().unwrap();
    }

    #[test]
    fn http_407_is_distinct_and_does_not_leak_credentials() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }
            stream
                .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                .unwrap();
        });

        let route = resolve_target_with(
            "example.com:443",
            &environment(
                Some(&format!("http://user:topsecret@{}", address)),
                None,
                None,
            ),
            None,
        )
        .unwrap();
        let error =
            connect_via_route(&Target::parse("example.com:443").unwrap(), &route).unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected proxy error");
        };
        let failure = error.failure();
        assert_eq!(failure.kind, ProxyFailureKind::HttpAuthenticationRequired);
        assert_eq!(failure.endpoint, Some(format!("http://{address}")));
        assert!(!format!("{error:?}").contains("topsecret"));
        assert!(!failure.detail.contains("topsecret"));
        proxy_thread.join().unwrap();
    }

    #[test]
    fn http_non_2xx_is_classified_without_falling_back_to_direct() {
        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_address = proxy_listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (mut stream, _) = proxy_listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
            }
            stream
                .write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n")
                .unwrap();
        });
        let direct_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        direct_listener.set_nonblocking(true).unwrap();
        let target = format!("127.0.0.1:{}", direct_listener.local_addr().unwrap().port());
        let route = resolve_target_with(
            &target,
            &environment(Some(&format!("http://{proxy_address}")), None, None),
            None,
        )
        .unwrap();

        let error = connect_via_route(&Target::parse(&target).unwrap(), &route).unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected proxy error");
        };
        let failure = error.failure();
        assert_eq!(failure.kind, ProxyFailureKind::HttpStatus);
        assert_eq!(failure.detail, "HTTP proxy returned status 502");
        assert_eq!(failure.endpoint, Some(format!("http://{proxy_address}")));
        assert!(direct_listener.accept().is_err());
        proxy_thread.join().unwrap();
    }

    #[test]
    fn environment_failure_never_crosses_over_to_the_system_source() {
        let dead_environment = TcpListener::bind("127.0.0.1:0").unwrap();
        let dead_environment_address = dead_environment.local_addr().unwrap();
        drop(dead_environment);
        let system_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        system_listener.set_nonblocking(true).unwrap();
        let direct_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        direct_listener.set_nonblocking(true).unwrap();
        let target = format!("127.0.0.1:{}", direct_listener.local_addr().unwrap().port());
        let system = SystemProxy::new(
            None,
            None,
            Some(endpoint(&format!(
                "socks5://{}",
                system_listener.local_addr().unwrap()
            ))),
            String::new(),
        );
        let route = resolve_target_with(
            &target,
            &environment(
                Some(&format!("http://{dead_environment_address}")),
                None,
                None,
            ),
            system.as_ref(),
        )
        .unwrap();
        assert_eq!(route.source(), Some(ProxySource::Environment));
        assert_eq!(route.candidates().len(), 1);

        assert!(connect_via_route(&Target::parse(&target).unwrap(), &route).is_err());
        assert!(system_listener.accept().is_err());
        assert!(direct_listener.accept().is_err());
    }

    #[test]
    fn http_proxy_response_size_is_bounded() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }
            let mut response = b"HTTP/1.1 200 Connection Established\r\nX-Oversized: ".to_vec();
            response.resize(MAX_HTTP_PROXY_RESPONSE + 1, b'a');
            let _ = stream.write_all(&response);
        });

        let route = resolve_target_with(
            "example.com:443",
            &environment(Some(&format!("http://{address}")), None, None),
            None,
        )
        .unwrap();
        let error =
            connect_via_route(&Target::parse("example.com:443").unwrap(), &route).unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected proxy error");
        };
        let failure = error.failure();
        assert_eq!(failure.kind, ProxyFailureKind::HttpResponse);
        assert_eq!(failure.endpoint, Some(format!("http://{address}")));
        assert_eq!(failure.detail, "response headers are too large");
        proxy_thread.join().unwrap();
    }

    #[test]
    fn socks5_fixture_uses_domain_address_and_rfc1929_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();

        let proxy_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut greeting = [0u8; 4];
            stream.read_exact(&mut greeting).unwrap();
            assert_eq!(greeting, [5, 2, 0, 2]);
            stream.write_all(&[5, 2]).unwrap();

            let mut auth_header = [0u8; 2];
            stream.read_exact(&mut auth_header).unwrap();
            assert_eq!(auth_header, [1, 4]);
            let mut username = [0u8; 4];
            stream.read_exact(&mut username).unwrap();
            let mut password_length = [0u8; 1];
            stream.read_exact(&mut password_length).unwrap();
            let mut password = vec![0u8; password_length[0] as usize];
            stream.read_exact(&mut password).unwrap();
            sender.send((username.to_vec(), password)).unwrap();
            stream.write_all(&[1, 0]).unwrap();

            let mut request = [0u8; 5];
            stream.read_exact(&mut request).unwrap();
            assert_eq!(&request[..4], &[5, 1, 0, 3]);
            let mut domain = vec![0u8; request[4] as usize];
            stream.read_exact(&mut domain).unwrap();
            let mut port = [0u8; 2];
            stream.read_exact(&mut port).unwrap();
            sender.send((domain, port.to_vec())).unwrap();

            stream
                .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 80])
                .unwrap();
        });

        let route = resolve_target_with(
            "server.example:443",
            &environment(None, Some(&format!("socks5://user:pass@{}", address)), None),
            None,
        )
        .unwrap();
        let stream =
            connect_via_route(&Target::parse("server.example:443").unwrap(), &route).unwrap();
        drop(stream);

        let credentials = receiver.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(credentials.0.as_slice(), b"user");
        assert_eq!(credentials.1.as_slice(), b"pass");
        let request = receiver.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(request.0.as_slice(), b"server.example");
        assert_eq!(request.1, 443u16.to_be_bytes());
        proxy_thread.join().unwrap();
    }

    #[test]
    fn socks5_auth_failure_is_explicit() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut greeting = [0u8; 4];
            stream.read_exact(&mut greeting).unwrap();
            stream.write_all(&[5, 2]).unwrap();
            let mut auth = [0u8; 2];
            stream.read_exact(&mut auth).unwrap();
            let mut rest = vec![0u8; auth[1] as usize + 1];
            stream.read_exact(&mut rest).unwrap();
            let password_length = rest[rest.len() - 1] as usize;
            let mut password = vec![0u8; password_length];
            stream.read_exact(&mut password).unwrap();
            stream.write_all(&[1, 1]).unwrap();
        });

        let route = resolve_target_with(
            "server.example:443",
            &environment(
                None,
                Some(&format!("socks5://user:wrong-secret@{}", address)),
                None,
            ),
            None,
        )
        .unwrap();
        let error =
            connect_via_route(&Target::parse("server.example:443").unwrap(), &route).unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected proxy error");
        };
        let failure = error.failure();
        assert_eq!(failure.kind, ProxyFailureKind::SocksAuthentication);
        assert_eq!(failure.endpoint, Some(format!("socks5://{address}")));
        assert!(!format!("{error:?}").contains("wrong-secret"));
        assert!(!failure.detail.contains("wrong-secret"));
        proxy_thread.join().unwrap();
    }

    #[test]
    fn socks5_target_rejection_preserves_the_reply_code() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut greeting_header = [0u8; 2];
            stream.read_exact(&mut greeting_header).unwrap();
            let mut methods = vec![0u8; greeting_header[1] as usize];
            stream.read_exact(&mut methods).unwrap();
            stream.write_all(&[5, 0]).unwrap();

            let mut request = [0u8; 4];
            stream.read_exact(&mut request).unwrap();
            let address_len = match request[3] {
                0x01 => 4,
                0x04 => 16,
                0x03 => {
                    let mut length = [0u8; 1];
                    stream.read_exact(&mut length).unwrap();
                    length[0] as usize
                }
                atyp => panic!("unexpected SOCKS5 ATYP {atyp}"),
            };
            let mut rest = vec![0u8; address_len + 2];
            stream.read_exact(&mut rest).unwrap();
            stream
                .write_all(&[5, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .unwrap();
            stream.flush().unwrap();
        });

        let route = resolve_target_with(
            "server.example:443",
            &environment(None, Some(&format!("socks5://{}", address)), None),
            None,
        )
        .unwrap();
        let error =
            connect_via_route(&Target::parse("server.example:443").unwrap(), &route).unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected proxy error");
        };
        let failure = error.failure();
        assert_eq!(failure.kind, ProxyFailureKind::SocksTargetRejected);
        assert_eq!(failure.endpoint, Some(format!("socks5://{address}")));
        proxy_thread.join().unwrap();
    }

    #[test]
    fn dead_proxy_has_stable_category_and_safe_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);

        let route = resolve_target_with(
            "server.example:443",
            &environment(Some(&format!("http://{address}")), None, None),
            None,
        )
        .unwrap();
        let error =
            connect_via_route(&Target::parse("server.example:443").unwrap(), &route).unwrap_err();
        let crate::Error::Proxy(error) = error else {
            panic!("expected proxy error");
        };
        let failure = error.failure();
        assert_eq!(failure.kind, ProxyFailureKind::DnsOrConnect);
        assert_eq!(failure.endpoint, Some(format!("http://{address}")));
    }

    #[test]
    fn failure_categories_remain_distinct() {
        let cases = [
            (
                ProxyError::InvalidConfiguration("bad configuration"),
                ProxyFailureKind::Configuration,
            ),
            (
                ProxyError::UnsupportedScheme("ftp".to_string()),
                ProxyFailureKind::UnsupportedScheme,
            ),
            (ProxyError::Timeout, ProxyFailureKind::Timeout),
            (
                ProxyError::HttpAuthenticationRequired,
                ProxyFailureKind::HttpAuthenticationRequired,
            ),
            (ProxyError::HttpStatus(502), ProxyFailureKind::HttpStatus),
            (
                ProxyError::SocksAuthentication,
                ProxyFailureKind::SocksAuthentication,
            ),
            (
                ProxyError::SocksTargetRejected(0x05),
                ProxyFailureKind::SocksTargetRejected,
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.failure().kind, expected);
        }
    }

    #[test]
    fn prefixes_match_full_bytes_and_partial_bytes() {
        assert!(prefix_matches(&[10, 1, 2, 3], &[10, 2, 2, 3], 8));
        assert!(prefix_matches(&[10, 1, 2, 3], &[10, 2, 2, 3], 12));
        assert!(!prefix_matches(&[10, 1, 2, 3], &[10, 2, 2, 3], 16));
        assert!(prefix_matches(&[10, 128, 2, 3], &[10, 191, 2, 3], 9));
        assert!(!prefix_matches(&[10, 128, 2, 3], &[10, 64, 2, 3], 9));
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    #[ignore = "requires a configured desktop proxy and proxy environment variables removed"]
    fn live_desktop_proxy_is_detected() {
        let target = env::var("HEROINN_LIVE_PROXY_TARGET")
            .unwrap_or_else(|_| "heroinn-proxy-test.invalid:443".to_string());
        let route = resolve_target(&target).unwrap();
        assert_eq!(route.source(), Some(ProxySource::System));
        assert!(!route.candidates().is_empty());
        println!("desktop proxy route: {route:?}");
    }
}
