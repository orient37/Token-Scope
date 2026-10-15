#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(feature = "desktop")]
fn main() {
    tokenscope_lib::desktop::run();
}
#[cfg(not(feature = "desktop"))]
fn main() {
    eprintln!("Use npm run desktop, or cargo run --bin verify -- <database-path>");
}
