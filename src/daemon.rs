//! The tray process: owns the timers and settings, and runs every window as a child process.
use crate::exercises::EXERCISE_IDS;
use crate::i18n::{format_left, resolve_language, system_locale, tr};
use crate::ipc::{self, BreakView, FromChild, Info, Proc, StatusMsg, ToChild};
use crate::notify::notify;
use crate::platform::{self, Idle, Pointer, Rect};
use crate::scheduler::{Busy, Event, Scheduler, WalkResult};
use crate::store::{Stats, SettingsStore, data_dir};
use crate::tray::{Callback, Cmd, CmdKey, Item, Menu, Tray};
use crate::ui::face::FaceCfg;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A fixed local port doubles as the single-instance lock.
const LOCK_ADDR: &str = "127.0.0.1:47821";
const CARD: (i32, i32) = (480, 560);
/// Below this many seconds of idle time the user counts as sitting at the computer.
const ACTIVE_IDLE_SEC: f64 = 1.5;

enum Ev {
    Tick,
    Cmd(Cmd),
    FromChild(u64, FromChild),
    Exited(u64),
}

struct Face {
    id: u64,
    proc: Proc,
    deadline: Instant,
}

struct Break {
    procs: Vec<(u64, Proc)>,
    done: bool,
    total: f64,
    remaining: f64,
    waiting: bool,
    require_idle: bool,
    strict: bool,
    snooze_min: f64,
    sound: bool,
    tip: usize,
    lang: String,
    last: Instant,
    done_at: Instant,
}

struct Daemon {
    tx: Sender<Ev>,
    store: SettingsStore,
    stats: Stats,
    sched: Scheduler,
    idle: Idle,
    tray: Option<Tray>,
    tray_menu: Option<Menu>,
    info: Info,
    settings_win: Option<(u64, Proc)>,
    face: Option<Face>,
    brk: Option<Break>,
    next_id: u64,
    last_exercise: Option<String>,
    sequence: usize,
    rng: u64,
}

fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

pub fn run() {
    let listener = match TcpListener::bind(LOCK_ADDR) {
        Ok(l) => l,
        Err(_) => {
            // Already running: ask that instance to show its settings.
            let _ = TcpStream::connect(LOCK_ADDR);
            return;
        }
    };
    let (tx, rx) = channel();
    let dir = data_dir();
    let store = SettingsStore::open(dir.join("settings.json"));
    let idle = Idle::detect();
    let info = Info {
        version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
        wayland: platform::session().wayland,
        cursor_mode: platform::cursor_mode().into(),
        can_follow: platform::cursor_mode() == "direct",
        idle_source: idle.name.into(),
        idle_reliable: idle.reliable,
    };
    eprintln!("[tcdev] {}, cursor: {}, idle: {}, data: {}", info.platform, info.cursor_mode, info.idle_source, dir.display());

    let first_run = !store.existed;
    let mut d = Daemon {
        tx: tx.clone(),
        sched: Scheduler::new(store.current.clone(), idle.reliable, now()),
        stats: Stats::open(dir.join("stats.json")),
        store,
        idle,
        tray: None,
        tray_menu: None,
        info,
        settings_win: None,
        face: None,
        brk: None,
        next_id: 0,
        last_exercise: None,
        sequence: 0,
        rng: SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |t| t.as_nanos() as u64) | 1,
    };

    let on_cmd: Callback = {
        let tx = tx.clone();
        std::sync::Arc::new(move |cmd| {
            let _ = tx.send(Ev::Cmd(cmd));
        })
    };
    let menu = d.menu();
    d.tray = Tray::start(menu.clone(), on_cmd);
    d.tray_menu = Some(menu);

    let tx2 = tx.clone();
    std::thread::spawn(move || {
        for _ in listener.incoming().flatten() {
            let _ = tx2.send(Ev::Cmd(Cmd::Settings));
        }
    });
    let tx3 = tx.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(1));
            if tx3.send(Ev::Tick).is_err() {
                break;
            }
        }
    });

    if d.store.current.autostart {
        d.apply_autostart(true);
    }
    if first_run {
        d.open_settings();
        let (title, body) = (d.t("notify.runningTitle", &[]), d.t("notify.runningBody", &[]));
        notify(&title, &body);
    }
    d.event_loop(rx);
}

impl Daemon {
    fn lang(&self) -> String {
        resolve_language(&self.store.current.language, &system_locale()).to_string()
    }

    fn t(&self, key: &str, args: &[&dyn ToString]) -> String {
        tr(&self.lang(), key, args)
    }

    fn rand(&mut self, n: usize) -> usize {
        // xorshift: reminders need variety, not randomness quality.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng % n.max(1) as u64) as usize
    }

    fn new_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn spawn(&mut self, args: &[String]) -> Option<(u64, Proc)> {
        let id = self.new_id();
        let (tx, tx2) = (self.tx.clone(), self.tx.clone());
        let on_line = move |msg| {
            let _ = tx.send(Ev::FromChild(id, msg));
        };
        let on_exit = move || {
            let _ = tx2.send(Ev::Exited(id));
        };
        match ipc::spawn(args, on_line, on_exit) {
            Ok(proc) => Some((id, proc)),
            Err(e) => {
                eprintln!("[tcdev] cannot start window {args:?}: {e}");
                None
            }
        }
    }

    fn event_loop(&mut self, rx: Receiver<Ev>) {
        for ev in rx {
            match ev {
                Ev::Tick => self.tick(),
                Ev::Cmd(cmd) => {
                    if matches!(cmd, Cmd::Quit) {
                        self.shutdown();
                        return;
                    }
                    self.command(cmd);
                }
                Ev::FromChild(id, msg) => self.from_child(id, msg),
                Ev::Exited(id) => self.exited(id),
            }
        }
    }

    fn shutdown(&mut self) {
        self.cancel_face();
        self.end_break(None);
        if let Some((_, mut p)) = self.settings_win.take() {
            p.kill();
            let _ = p.child.wait();
        }
    }

    // ---------- timers ----------

    fn tick(&mut self) {
        let idle = &mut self.idle;
        match self.sched.tick(now(), || idle.read(4000)) {
            Some(Event::EyeDue) => self.run_eye(None),
            Some(Event::WalkDue) => self.run_walk(),
            Some(Event::WalkWarning(sec)) => {
                let left = if sec < 60 { self.t("time.sec", &[&sec]) } else { format_left(&self.lang(), sec as u64) };
                notify(&self.t("notify.walkSoonTitle", &[]), &self.t("notify.walkSoonBody", &[&left]));
            }
            Some(Event::NaturalBreak) => eprintln!("[tcdev] away long enough, timers restarted"),
            None => {}
        }
        if self.face.as_ref().is_some_and(|f| Instant::now() > f.deadline) {
            self.cancel_face();
            self.refresh();
        }
        self.tick_break();
        self.refresh();
    }

    // ---------- eye exercise ----------

    fn pick_exercise(&mut self) -> String {
        let eyes = self.store.current.eyes.clone();
        if eyes.order == "sequential" {
            self.sequence += 1;
            return eyes.exercises[(self.sequence - 1) % eyes.exercises.len()].clone();
        }
        let pool: Vec<&String> = if eyes.exercises.len() > 1 {
            eyes.exercises.iter().filter(|id| Some(*id) != self.last_exercise.as_ref()).collect()
        } else {
            eyes.exercises.iter().collect()
        };
        pool[self.rand(pool.len())].clone()
    }

    fn run_eye(&mut self, exercise: Option<String>) {
        let exercise = exercise.unwrap_or_else(|| self.pick_exercise());
        self.last_exercise = Some(exercise.clone());
        let eyes = &self.store.current.eyes;
        let cfg = FaceCfg {
            exercise,
            duration_sec: eyes.duration_sec,
            scale: eyes.face_scale as i32,
            show_hint: eyes.show_hint,
            sound: eyes.sound,
            position: eyes.position.clone(),
            follow_cursor: eyes.follow_cursor,
            cursor_mode: self.info.cursor_mode.clone(),
            lang: self.lang(),
        };
        let deadline = Instant::now() + Duration::from_secs_f64(eyes.duration_sec + 8.0);
        let args = vec!["face".to_string(), serde_json::to_string(&cfg).unwrap_or_default()];
        match self.spawn(&args) {
            Some((id, proc)) => self.face = Some(Face { id, proc, deadline }),
            None => self.sched.finish_eye(),
        }
    }

    fn cancel_face(&mut self) {
        if let Some(mut f) = self.face.take() {
            f.proc.kill();
            let _ = f.proc.child.wait();
            self.sched.finish_eye();
        }
    }

    // ---------- walking break ----------

    fn run_walk(&mut self) {
        if self.brk.is_some() {
            return;
        }
        let walk = self.store.current.walk.clone();
        let lang = self.lang();
        let strict = walk.strict;
        let pointer = Pointer::new().get();
        let monitors = platform::monitors();
        let target = pointer.and_then(|p| monitors.iter().find(|m| m.contains(p))).or(monitors.first()).copied();

        let windows: Vec<Option<Rect>> = if strict && !monitors.is_empty() {
            monitors.iter().map(|m| Some(*m)).collect()
        } else {
            vec![target.map(|m| Rect { x: m.x + (m.w - CARD.0) / 2, y: m.y + (m.h - CARD.1) / 2, w: CARD.0, h: CARD.1 })]
        };
        let mut procs = vec![];
        for bounds in windows {
            let mut args = vec!["break".to_string()];
            if let Some(b) = bounds {
                args.extend(["--bounds".into(), format!("{},{},{},{}", b.x, b.y, b.w, b.h)]);
            }
            if strict {
                args.push("--strict".into());
            }
            procs.extend(self.spawn(&args));
        }
        if procs.is_empty() {
            self.sched.finish_walk(WalkResult::Skipped);
            return;
        }
        let total = (walk.duration_min * 60.0).round();
        let tip = self.rand(1000);
        self.brk = Some(Break {
            procs,
            done: false,
            total,
            remaining: total,
            waiting: false,
            require_idle: walk.require_idle && self.idle.reliable,
            strict,
            snooze_min: walk.snooze_min,
            sound: walk.sound,
            tip,
            lang,
            last: Instant::now(),
            done_at: Instant::now(),
        });
        self.broadcast_break();
    }

    fn broadcast_break(&mut self) {
        let Some(b) = &mut self.brk else { return };
        let view = BreakView {
            done: b.done,
            total: b.total,
            remaining: b.remaining,
            waiting: b.waiting,
            require_idle: b.require_idle,
            strict: b.strict,
            snooze_min: b.snooze_min,
            sound: b.sound,
            tip: b.tip,
            lang: b.lang.clone(),
        };
        for (_, p) in &mut b.procs {
            p.send(&ToChild::Break(view.clone()));
        }
    }

    fn tick_break(&mut self) {
        let Some(b) = &mut self.brk else { return };
        let t = Instant::now();
        let dt = t.duration_since(b.last).as_secs_f64().min(2.0);
        b.last = t;
        if !b.done {
            b.waiting = b.require_idle && self.idle.read(0) < ACTIVE_IDLE_SEC;
            if !b.waiting {
                b.remaining = (b.remaining - dt).max(0.0);
            }
            if b.remaining == 0.0 {
                b.done = true;
                b.done_at = t;
            }
        } else {
            let since = t.duration_since(b.done_at);
            let back = self.idle.reliable && self.idle.read(0) < ACTIVE_IDLE_SEC;
            if (since > Duration::from_secs(5) && back) || since > Duration::from_secs(120) {
                self.end_break(Some(WalkResult::Done));
                return;
            }
        }
        self.broadcast_break();
    }

    /// `None` just closes the windows (app quitting).
    fn end_break(&mut self, result: Option<WalkResult>) {
        let Some(mut b) = self.brk.take() else { return };
        for (_, p) in &mut b.procs {
            p.kill();
            let _ = p.child.wait();
        }
        if let Some(result) = result {
            self.sched.finish_walk(result);
            self.stats.bump(match result {
                WalkResult::Done => "walks",
                WalkResult::Skipped => "skipped",
                WalkResult::Snoozed => "snoozed",
            });
            self.refresh();
        }
    }

    // ---------- commands and windows ----------

    fn command(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::EyeNow(exercise) => {
                if self.sched.begin(Busy::Eye) {
                    self.run_eye(exercise.filter(|e| EXERCISE_IDS.contains(&e.as_str())));
                }
            }
            Cmd::WalkNow => {
                if self.sched.begin(Busy::Walk) {
                    self.run_walk();
                }
            }
            Cmd::SkipBreak => self.end_break(Some(WalkResult::Skipped)),
            Cmd::Pause(minutes) => {
                self.cancel_face();
                self.end_break(Some(WalkResult::Skipped));
                self.sched.pause(minutes, now());
            }
            Cmd::Resume => self.sched.resume(now()),
            Cmd::Reset => self.sched.reset_timers(),
            Cmd::Settings => self.open_settings(),
            Cmd::Quit => {}
        }
        self.refresh();
    }

    fn open_settings(&mut self) {
        if let Some((_, p)) = &mut self.settings_win {
            p.send(&ToChild::Focus);
            return;
        }
        let Some((id, mut proc)) = self.spawn(&["settings".to_string()]) else { return };
        proc.send(&ToChild::Hello {
            settings: self.store.current.clone(),
            lang: self.lang(),
            info: self.info.clone(),
            status: self.status_msg(),
        });
        self.settings_win = Some((id, proc));
    }

    fn from_child(&mut self, id: u64, msg: FromChild) {
        match msg {
            FromChild::Update(next) => {
                let prev = self.store.current.clone();
                let current = self.store.update(next);
                self.sched.settings = current.clone();
                if current.autostart != prev.autostart {
                    self.apply_autostart(current.autostart);
                }
                let lang = self.lang();
                if let Some((_, p)) = &mut self.settings_win {
                    p.send(&ToChild::Changed { settings: current, lang });
                }
                self.refresh();
            }
            FromChild::Command { name, arg } => {
                let minutes = arg.as_deref().and_then(|a| a.parse::<f64>().ok());
                let cmd = match name.as_str() {
                    "eyeNow" => Cmd::EyeNow(arg),
                    "walkNow" => Cmd::WalkNow,
                    "pause" => Cmd::Pause(minutes.or(Some(30.0))),
                    "resume" => Cmd::Resume,
                    "reset" => Cmd::Reset,
                    _ => return,
                };
                self.command(cmd);
            }
            FromChild::BreakAction(action) => {
                let Some(b) = &self.brk else { return };
                if !b.procs.iter().any(|(pid, _)| *pid == id) {
                    return;
                }
                if action == "close" && b.done {
                    return self.end_break(Some(WalkResult::Done));
                }
                if b.strict {
                    return;
                }
                match action.as_str() {
                    "snooze" => self.end_break(Some(WalkResult::Snoozed)),
                    "skip" => self.end_break(Some(WalkResult::Skipped)),
                    _ => {}
                }
            }
        }
    }

    fn exited(&mut self, id: u64) {
        if self.face.as_ref().is_some_and(|f| f.id == id) {
            let mut f = self.face.take().expect("checked above");
            let ok = f.proc.child.wait().is_ok_and(|s| s.success());
            if ok {
                self.stats.bump("eyes");
            }
            self.sched.finish_eye();
        } else if self.settings_win.as_ref().is_some_and(|(sid, _)| *sid == id) {
            if let Some((_, mut p)) = self.settings_win.take() {
                let _ = p.child.wait();
            }
        } else if let Some(b) = &mut self.brk {
            if let Some(i) = b.procs.iter().position(|(pid, _)| *pid == id) {
                let (_, mut p) = b.procs.remove(i);
                let _ = p.child.wait();
                // Closing the card counts as skipping; strict windows can only vanish if all are killed.
                if !b.strict || b.procs.is_empty() {
                    self.end_break(Some(WalkResult::Skipped));
                }
            }
        }
        self.refresh();
    }

    fn apply_autostart(&self, enabled: bool) {
        if let Err(e) = crate::autostart::set(enabled) {
            eprintln!("[tcdev] autostart failed: {e}");
        }
    }

    // ---------- status and tray ----------

    fn status_msg(&mut self) -> StatusMsg {
        StatusMsg { status: self.sched.status(now()), today: self.stats.today() }
    }

    fn refresh(&mut self) {
        let menu = self.menu();
        if self.tray_menu.as_ref() != Some(&menu) {
            if let Some(tray) = &self.tray {
                tray.set(menu.clone());
            }
            self.tray_menu = Some(menu);
        }
        let msg = self.status_msg();
        if let Some((_, p)) = &mut self.settings_win {
            p.send(&ToChild::Status(msg));
        }
    }

    fn menu(&mut self) -> Menu {
        let lang = self.lang();
        let status = self.sched.status(now());
        let today = self.stats.today();
        let left = |sec: Option<u64>, on: &str, off: &str| match sec {
            Some(s) => tr(&lang, on, &[&format_left(&lang, s)]),
            None => tr(&lang, off, &[]),
        };
        let mut items = if status.paused {
            let label = match status.paused_until {
                Some(until) => {
                    let at = chrono::DateTime::from_timestamp(until as i64, 0).map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string());
                    tr(&lang, "tray.pausedUntil", &[&at.unwrap_or_default()])
                }
                None => tr(&lang, "tray.paused", &[]),
            };
            vec![Item::Label(label)]
        } else {
            vec![
                Item::Label(left(status.eye_left, "tray.eyesIn", "tray.eyesOff")),
                Item::Label(left(status.walk_left, "tray.walkIn", "tray.walkOff")),
            ]
        };
        let busy = status.busy.is_some();
        let action = |key: &str, enabled: bool, cmd: CmdKey| Item::Action { label: tr(&lang, key, &[]), enabled, cmd };
        items.push(Item::Label(tr(&lang, "tray.today", &[&today.eyes, &today.walks])));
        items.push(Item::Sep);
        items.push(action("tray.eyeNow", !busy, CmdKey::EyeNow));
        items.push(if status.busy.as_deref() == Some("walk") {
            action("tray.skipBreak", true, CmdKey::SkipBreak)
        } else {
            action("tray.walkNow", !busy, CmdKey::WalkNow)
        });
        items.push(Item::Sep);
        items.push(if status.paused {
            action("tray.resume", true, CmdKey::Resume)
        } else {
            Item::Sub(
                tr(&lang, "tray.pause", &[]),
                vec![
                    action("tray.pause30", true, CmdKey::Pause(Some(30))),
                    action("tray.pause60", true, CmdKey::Pause(Some(60))),
                    action("tray.pause120", true, CmdKey::Pause(Some(120))),
                    action("tray.pauseForever", true, CmdKey::Pause(None)),
                ],
            )
        });
        items.push(action("tray.reset", true, CmdKey::Reset));
        items.push(Item::Sep);
        items.push(action("tray.settings", true, CmdKey::Settings));
        items.push(action("tray.quit", true, CmdKey::Quit));
        Menu { items, sleeping: status.paused }
    }
}
