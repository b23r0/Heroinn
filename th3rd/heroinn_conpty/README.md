# conpty [![Crate](https://img.shields.io/crates/v/conpty)](https://crates.io/crates/conpty) [![docs.rs](https://img.shields.io/docsrs/conpty?color=blue)](https://docs.rs/conpty/0.1.0/conpty/) [![license](https://img.shields.io/crates/l/conpty)](./LICENSE.txt)

A library which provides an interface for [ConPTY](https://devblogs.microsoft.com/commandline/windows-command-line-introducing-the-windows-pseudo-console-conpty/).

It is originally developed to be a windows backend for [zhiburt/expectrl](https://github.com/zhiburt/expectrl).

## Usage

Include the library to your `Cargo.toml`.

```toml
# Cargo.toml
conpty = "0.5"
```

## Get started

Running `echo` and reading its output.

```rust
use std::io::{Read, Result};

fn main() -> Result<()> {
    let mut proc = conpty::spawn("echo Hello World")?;
    let mut reader = proc.output()?;

    println!("Process has pid={}", proc.pid());

    let mut buf = [0; 1028];
    reader.read(&mut buf)?;

    assert!(String::from_utf8_lossy(&buf).contains("Hello World"));

    Ok(())
}
```
