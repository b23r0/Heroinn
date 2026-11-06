# Heroinn
A cross platform C2/post-exploitation framework implementation by Rust.

![image]( https://github.com/b23r0/Heroinn/blob/master/image/ui.png)

_This project is for research and study purposes only, and any illegal behavior using this project has nothing to do with this project._

# Features

* GUI
* Interactive PTY shell
* System infomations collect
* File manager support with resume broken transfer and largefile
* Per-host aggregate transfer throttling (default 1 MB/s) across shell and file traffic
* Support Win10+(Windows Server 2019+) & Linux & BSD & OSX
* Multiple communication protocols (TCP & HTTP & reliable UDP)

# HTTP and TCP proxy support

`HeroinnProtocol::HTTP` and `HeroinnProtocol::TCP` share one outbound
connection layer. Main connections and the reverse tunnels used by the shell
and file manager resolve and use the same proxy configuration for every new
connection. Reliable-UDP connections and loopback-only IPC remain direct.

Discovery checks explicit environment configuration first:

1. `http_proxy` / `HTTP_PROXY`
2. `https_proxy` / `HTTPS_PROXY`
3. `all_proxy` / `ALL_PROXY`

Lowercase names are preferred and uppercase names are the fallback. A
non-empty environment proxy overrides the desktop setting as a source. If no
environment proxy is configured, Heroinn reads the current user's manual proxy
from GNOME on Linux or the current-user Internet Settings registry values on
Windows. Within one source, the stable order is HTTP, HTTPS and SOCKS5. A
failed candidate may fall back only to another candidate from that same
source. Heroinn never falls back to a direct connection after a proxy was
selected.

Supported proxy URLs are `http://[user:password@]host:port` (HTTP CONNECT with
optional Basic authentication), `https://[user:password@]host:port` (TLS to the
proxy using the system trust roots, then HTTP CONNECT inside that TLS session)
and `socks5://[user:password@]host:port` (SOCKS5 with optional RFC 1929
username/password authentication). The URL scheme, not the variable name,
selects the transport: `HTTPS_PROXY=http://host:port` is a plaintext HTTP
CONNECT, while only an explicit `https://host:port` value uses TLS to the
proxy. This is not WSS target support; targets remain `ws://` WebSockets or
raw TCP streams.

Credentials are read only from proxy URL userinfo, held in memory, and
redacted from Heroinn logs and errors. TLS validation uses the operating
system trust store and the proxy hostname; there is no production certificate
verification bypass.

`no_proxy` / `NO_PROXY` supports host names, leading-dot or `*.` suffixes,
optional ports, IPv4/IPv6 addresses, CIDR ranges and `<local>`. A matching
destination connects directly. Windows `ProxyOverride` and GNOME
`ignore-hosts` are used for system-configured proxies.

PAC/WPAD, proxy auto-configuration, NTLM/Kerberos, system credential stores and
proxy UI are not supported. Linux support targets GNOME manual proxy settings;
KDE and XFCE settings are not read.

Release clients always write proxy and connection diagnostics to a bounded
per-user log:

* Linux: `$XDG_STATE_HOME/heroinn/heroinn_client.log`, falling back to
  `$HOME/.local/state/heroinn/heroinn_client.log`.
* Windows: `%LOCALAPPDATA%\Heroinn\Logs\heroinn_client.log`.

On Unix the log directory is created with mode `0700` and the log file with
mode `0600`. On Windows the file inherits the current user's LocalAppData
profile ACL. Each log is capped at 512 KiB and one previous file is retained
(`heroinn_client.log.1`), so the pair uses at most 1 MiB. Proxy failures record
a stable category, the scheme/host/port endpoint and fail-closed state; URL
userinfo, usernames, passwords and raw credentials are redacted. Windows also
shows one non-blocking error dialog per distinct proxy failure while the
client continues its retry loop. Linux additionally writes diagnostics to
stderr when one is available.

# Building

The workspace builds four binaries: `heroinn` (server GUI), `heroinn_client`
(payload), `heroinn_ftp` (file manager) and `heroinn_shell`.

`heroinn_shell` is the per-session terminal window the server spawns to drive a
remote pty. Its source is vendored under `th3rd/alacritty-driver/` — a fork of
[alacritty](https://github.com/alacritty/alacritty) that replaces the pty with a
TCP local socket, itself taken from
[b23r0/alacritty-driver](https://github.com/b23r0/alacritty-driver).

```sh
cargo build --release
```

The binaries land in `target/release/`, and `heroinn_shell` must sit next to
`heroinn` because the server spawns it from its working directory.

## Client generation

The server's `Generator` tab does not compile a client. It copies a prebuilt
`heroinn_client` and rewrites its embedded connection stamp in place.

Templates are searched next to the running `heroinn` binary first, then in the
current working directory. The generator accepts the legacy platform-suffixed
names and the normal Cargo output name:

| Platform | Template names |
| --- | --- |
| Windows x86_64 | `heroinn_client_windows.exe`, `heroinn_client.exe` |
| Linux x86_64 | `heroinn_client_linux`, `heroinn_client` |
| BSD x86_64 | `heroinn_client_bsd`, `heroinn_client` |
| macOS x86_64 | `heroinn_client_darwin`, `heroinn_client` |

Build the native client with `cargo build -p heroinn_client --release` and keep
its binary next to the server. Cross-platform payloads must first be built for
their target and copied under one of the matching names above.

The generated file inherits the template's permissions, including the Unix
executable bit. Connection information is limited to 1024 bytes of JSON, and
the generator refuses templates containing more than one valid stamp.

## Build requirements

The terminal window is an OpenGL application, so on Linux the build needs the
font and windowing headers, and the font stack is compiled through CMake:

* `cmake` (used to build the bundled freetype/fontconfig chain)
* `pkg-config`
* `libfontconfig` / `libfreetype` / `libexpat` development files
* `libxcb` development files
* at runtime: libX11 / libxcb / libEGL / libwayland-client / libxkbcommon / libvulkan

On Windows the driver builds against `winapi`, and no extra system packages are
required beyond a normal Rust toolchain.

## Dependencies

Every crate is on edition 2024 (`rust-version = "1.85"`), and all
cross-platform dependency versions are pinned once at the workspace root in
`[workspace.dependencies]`.

The vendored driver drops the two pty backends it inherited from alacritty —
`tty/unix.rs` and `tty/windows/`. Heroinn always attaches to the pty through
the local socket, so neither was reachable; removing them is what lets
`alacritty_terminal` build without `nix`, `signal-hook`, `mio-anonymous-pipes`,
`miow` or `winapi`.

Two pieces of the vendored driver are deliberately left as they were:

* the windowing stack (`glutin` 0.28 with its bundled `winit`). glutin 0.28 is
  the last release with the old `WindowedContext` API; anything newer means
  porting the whole event/window subsystem to the `glutin-winit` +
  `ApplicationHandler` model, which is a rewrite of the crate's primary UI that
  cannot be validated without a display.

# Todo

- [ ] Documentation
  - [ ] usage
- [ ] reverse socks5 proxy support
- [ ] shellcode
  - [ ] stage(.so/.dll) to shellcode
  - [ ] loader(no stage)
  - [ ] loader generator
- [ ] cross firewall
  - [x] http proxy auto detect
- [ ] basic
  - [ ] serilized communication  protocol convert json to bson
  - [ ] unit test


# Contributing

[![ChatOnDiscord](https://img.shields.io/badge/chat-on%20discord-blue)](https://discord.gg/ZKtYMvDFN4)

* Report a BUG
* Submit an ISSUE about suggestion
* Submit a improved PR
* Supplement the documentation about using `Heroinn`

Everyone for me, I for everyone.
