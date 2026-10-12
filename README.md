# Heroinn
A cross platform C2/post-exploitation framework implementation by Rust.

![image]( https://github.com/b23r0/Heroinn/blob/master/image/ui.png)

_This project is for research and study purposes only, and any illegal behavior using this project has nothing to do with this project._

# Features

* GUI
* Interactive PTY shell
* System infomations collect
* File manager support with resume broken transfer and largefile
* Support Win10+(Windows Server 2019+) & Linux & BSD & OSX
* Multiple communication protocols (TCP & HTTP & reliable UDP)

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
  - [ ] http proxy auto detect
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
