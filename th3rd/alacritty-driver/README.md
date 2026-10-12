# alacritty-driver

> Vendored into Heroinn from https://github.com/b23r0/alacritty-driver (commit
> 99a87da), which forks alacritty 0.11.0-dev. The only local changes are that the
> `alacritty` package and its binary target are renamed to `heroinn_shell`, and
> that the nested workspace manifest was dropped so the crates build as members
> of the Heroinn workspace. Upstream's Apache-2.0 license is kept in
> `LICENSE-APACHE`.

## About

alacritty-driver is an alacritty based driver. You can interact with alacritty in your program through TCP-based localsockets.

About alacritty : https://github.com/alacritty/alacritty

## Run Driver

```sh
.\alacritty_driver.exe --local-socket-port [port]
```

## Set Window Size

alacritty-driver will send packets in the following format to notify windows size changes

```
+-----------------+---------------------+---------------------+
| FLAG(0x37,0x37) |         ROWS        |         COLS        |
+-----------------+---------------------+---------------------+
|      u16        |        be_u16       |        be_u16       |
+-----------------+---------------------+---------------------+
```

## License

alacritty-driver license inherited from alacritty 

https://github.com/alacritty/alacritty/blob/master/LICENSE-APACHE