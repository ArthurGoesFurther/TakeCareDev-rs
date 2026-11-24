//! TakeCareDev: pixel-face reminders for eye exercises and walking breaks.
//! `takecaredev` runs the tray daemon; `takecaredev face|break|settings` are its window processes.
// No console window for the release build on Windows (the window processes talk over pipes).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod autostart;
mod daemon;
mod exercises;
mod i18n;
mod ipc;
mod notify;
mod platform;
mod scheduler;
mod settings;
mod sound;
mod sprite;
mod store;
mod tray;
mod ui;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("face") => ui::face::run(args.get(1).map_or("", String::as_str)),
        Some("break") => ui::break_screen::run(&args[1..]),
        Some("settings") => ui::settings::run(),
        _ => run_daemon(),
    }
}

#[cfg(target_os = "linux")]
fn run_daemon() {
    daemon::run();
}

/// The tray's event loop has to own the main thread on Windows and macOS.
#[cfg(not(target_os = "linux"))]
fn run_daemon() {
    tray::run_main(daemon::run);
}
