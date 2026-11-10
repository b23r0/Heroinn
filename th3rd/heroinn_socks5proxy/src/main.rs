#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = heroinn_socks5proxy::run() {
        eprintln!("heroinn_socks5proxy: {error}");
        std::process::exit(1);
    }
}
