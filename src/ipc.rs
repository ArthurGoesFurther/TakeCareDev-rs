//! JSON-lines protocol between the tray daemon and its window processes (settings, break, face).
use crate::scheduler::Status;
use crate::settings::Settings;
use crate::store::Today;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::io::{BufRead, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::Sender;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Info {
    pub version: String,
    pub platform: String,
    pub wayland: bool,
    pub cursor_mode: String,
    pub can_follow: bool,
    pub idle_source: String,
    pub idle_reliable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct StatusMsg {
    pub status: Status,
    pub today: Today,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BreakView {
    pub done: bool,
    pub total: f64,
    pub remaining: f64,
    pub waiting: bool,
    pub require_idle: bool,
    pub strict: bool,
    pub snooze_min: f64,
    pub sound: bool,
    pub tip: usize,
    pub lang: String,
}

/// Daemon → window.
#[derive(Debug, Serialize, Deserialize)]
pub enum ToChild {
    Hello { settings: Settings, lang: String, info: Info, status: StatusMsg },
    Status(StatusMsg),
    Changed { settings: Settings, lang: String },
    Break(BreakView),
    Focus,
}

/// Window → daemon.
#[derive(Debug, Serialize, Deserialize)]
pub enum FromChild {
    Update(Settings),
    Command { name: String, arg: Option<String> },
    BreakAction(String),
}

pub struct Proc {
    pub child: Child,
    stdin: Option<ChildStdin>,
}

impl Proc {
    pub fn send(&mut self, msg: &ToChild) {
        let Some(stdin) = &mut self.stdin else { return };
        let line = serde_json::to_string(msg).unwrap_or_default();
        if writeln!(stdin, "{line}").and_then(|_| stdin.flush()).is_err() {
            self.stdin = None;
        }
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}

/// Starts `takecaredev <args>` as a window process. `on_line` sees each line it prints;
/// `on_exit` runs once its output closes.
pub fn spawn(
    args: &[String],
    on_line: impl Fn(FromChild) + Send + 'static,
    on_exit: impl FnOnce() + Send + 'static,
) -> std::io::Result<Proc> {
    let mut cmd = Command::new(std::env::current_exe()?);
    cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::piped());
    #[cfg(target_os = "linux")]
    if crate::platform::session().xwayland {
        // Native Wayland hides the pointer and ignores window positions; XWayland allows both.
        cmd.env_remove("WAYLAND_DISPLAY").env("XDG_SESSION_TYPE", "x11");
    }
    let mut child = cmd.spawn()?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take().expect("piped stdout");
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(msg) = serde_json::from_str(&line) {
                on_line(msg);
            }
        }
        on_exit();
    });
    Ok(Proc { child, stdin })
}

/// Window side: messages from the daemon arrive on the returned channel; `repaint` wakes the UI.
pub fn listen<T: DeserializeOwned + Send + 'static>(repaint: impl Fn() + Send + 'static) -> std::sync::mpsc::Receiver<T> {
    let (tx, rx): (Sender<T>, _) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if let Ok(msg) = serde_json::from_str(&line) {
                if tx.send(msg).is_err() {
                    break;
                }
                repaint();
            }
        }
        // The daemon is gone (or closed our stdin): there is nobody to remind anymore.
        std::process::exit(0);
    });
    rx
}

/// Window side: one message to the daemon.
pub fn say(msg: &FromChild) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", serde_json::to_string(msg).unwrap_or_default());
    let _ = out.flush();
}
