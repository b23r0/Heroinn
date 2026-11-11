# heroinn-shell-driver

> Heroinn's server-side terminal implementation. The nested workspace manifest
> was dropped so its terminal engine and configuration derive crates build as
> members of the Heroinn workspace. The upstream Apache-2.0 attribution is
> retained in `LICENSE-APACHE`.

## About

`heroinn-shell-driver` drives a remote PTY and connects to the server through a local
TCP socket.

## Run Driver

```sh
.\heroinn_shell.exe --local-socket-port [port]
```

## Set Window Size

`heroinn-shell-driver` sends packets in the following format to report window size
changes:

```
+-----------------+---------------------+---------------------+
| FLAG(0x37,0x37) |         ROWS        |         COLS        |
+-----------------+---------------------+---------------------+
|      u16        |        be_u16       |        be_u16       |
+-----------------+---------------------+---------------------+
```

## License

The vendored terminal engine and configuration derive crates retain their
upstream Apache-2.0 license in `LICENSE-APACHE`.
