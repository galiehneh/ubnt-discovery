#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod ui;

#[cfg(windows)]
fn main() {
    ui::main_window::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("UbntDiscovery GUI is only supported on Windows");
    std::process::exit(1);
}
