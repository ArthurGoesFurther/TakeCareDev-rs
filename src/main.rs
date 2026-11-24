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
mod png;
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
        // Build-time helper: `takecaredev icon <out.png> <size>` writes the app icon (size a multiple of 32).
        Some("icon") => write_icon(&args[1..]),
        _ => run_daemon(),
    }
}

fn write_icon(args: &[String]) {
    let (Some(path), Some(size)) = (args.first(), args.get(1).and_then(|s| s.parse::<i32>().ok())) else {
        eprintln!("usage: takecaredev icon <out.png> <size>");
        std::process::exit(2);
    };
    let r = sprite::app_icon((size / 32).max(1));
    if let Err(e) = std::fs::write(path, png::encode_rgba(r.width as u32, r.height as u32, &r.data)) {
        eprintln!("cannot write {path}: {e}");
        std::process::exit(1);
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
