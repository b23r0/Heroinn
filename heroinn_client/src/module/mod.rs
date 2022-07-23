#[allow(non_snake_case)]
pub mod shell;

#[cfg(target_os = "windows")]
mod conpty;