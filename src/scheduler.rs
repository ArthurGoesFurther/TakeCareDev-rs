//! Counts active screen time and decides when reminders are due.
use crate::settings::Settings;

/// Longer gaps between ticks mean the machine slept; they never count as screen time.
const MAX_STEP_SEC: f64 = 15.0;
/// An eye exercise this close to a walking break is dropped: the break rests the eyes too.
const WALK_COVERS_EYES_SEC: f64 = 90.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Busy { Eye, Walk }

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event { EyeDue, WalkDue, WalkWarning(u32), NaturalBreak }

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WalkResult { Done, Skipped, Snoozed }

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Status {
    pub busy: Option<String>,
    pub paused: bool,
    /// Unix seconds; `None` while paused means "until resumed".
    pub paused_until: Option<f64>,
    pub eye_left: Option<u64>,
    pub walk_left: Option<u64>,
}

pub struct Scheduler {
    pub settings: Settings,
    pub idle_reliable: bool,
    pub eye_sec: f64,
    pub walk_sec: f64,
    pub busy: Option<Busy>,
    /// `Some(f64::INFINITY)` pauses until resumed.
    paused_until: Option<f64>,
    away: bool,
    warned: bool,
    last: f64,
}

impl Scheduler {
    pub fn new(settings: Settings, idle_reliable: bool, now: f64) -> Self {
        Self {
            settings,
            idle_reliable,
            eye_sec: 0.0,
            walk_sec: 0.0,
            busy: None,
            paused_until: None,
            away: false,
            warned: false,
            last: now,
        }
    }

    pub fn paused(&self, now: f64) -> bool {
        self.paused_until.is_some_and(|until| until > now)
    }

    pub fn status(&self, now: f64) -> Status {
        let Settings { eyes, walk, .. } = &self.settings;
        let left = |on: bool, interval_min: f64, spent: f64| {
            on.then(|| (interval_min * 60.0 - spent).max(0.0).ceil() as u64)
        };
        let paused = self.paused(now);
        Status {
            busy: self.busy.map(|b| if b == Busy::Eye { "eye" } else { "walk" }.to_string()),
            paused,
            paused_until: self.paused_until.filter(|u| paused && u.is_finite()),
            eye_left: left(eyes.enabled, eyes.interval_min, self.eye_sec),
            walk_left: left(walk.enabled, walk.interval_min, self.walk_sec),
        }
    }

    /// Advances the timers to `now` (seconds). `read_idle` is only called when it is needed.
    pub fn tick(&mut self, now: f64, read_idle: impl FnOnce() -> f64) -> Option<Event> {
        let dt = (now - self.last).max(0.0);
        self.last = now;

        if let Some(until) = self.paused_until {
            if until > now {
                return None;
            }
            self.paused_until = None;
        }
        if self.busy.is_some() {
            return None;
        }

        let Settings { eyes, walk, idle, .. } = self.settings.clone();
        let reset_after = idle.reset_after_min * 60.0;
        if reset_after > 0.0 {
            let idle_sec = if self.idle_reliable { read_idle() } else { 0.0 };
            let slept = if dt > MAX_STEP_SEC { dt } else { 0.0 };
            if idle_sec.max(slept) >= reset_after {
                if self.away {
                    return None;
                }
                self.away = true;
                self.reset_timers();
                return Some(Event::NaturalBreak);
            }
        }
        self.away = false;

        let step = dt.min(MAX_STEP_SEC);
        self.eye_sec += step;
        self.walk_sec += step;

        let walk_left = walk.interval_min * 60.0 - self.walk_sec;
        if walk.enabled {
            if walk_left <= 0.0 {
                self.busy = Some(Busy::Walk);
                return Some(Event::WalkDue);
            }
            if walk.warn_sec > 0.0 && !self.warned && walk_left <= walk.warn_sec {
                self.warned = true;
                return Some(Event::WalkWarning(walk_left.ceil() as u32));
            }
        }
        if eyes.enabled && self.eye_sec >= eyes.interval_min * 60.0 {
            if walk.enabled && walk_left <= WALK_COVERS_EYES_SEC {
                self.eye_sec = 0.0;
                return None;
            }
            self.busy = Some(Busy::Eye);
            return Some(Event::EyeDue);
        }
        None
    }

    /// Marks a manually started reminder as running; false if another one is on screen.
    pub fn begin(&mut self, kind: Busy) -> bool {
        if self.busy.is_some() {
            return false;
        }
        self.busy = Some(kind);
        true
    }

    pub fn finish_eye(&mut self) {
        self.busy = None;
        self.eye_sec = 0.0;
    }

    pub fn finish_walk(&mut self, result: WalkResult) {
        let walk = &self.settings.walk;
        self.busy = None;
        if result == WalkResult::Snoozed {
            let snooze = walk.snooze_min * 60.0;
            self.walk_sec = (walk.interval_min * 60.0 - snooze).max(0.0);
            self.warned = snooze <= walk.warn_sec;
            return;
        }
        self.walk_sec = 0.0;
        self.warned = false;
        if result == WalkResult::Done {
            self.eye_sec = 0.0;
        }
    }

    pub fn pause(&mut self, minutes: Option<f64>, now: f64) {
        self.paused_until = Some(minutes.map_or(f64::INFINITY, |m| now + m * 60.0));
    }

    pub fn resume(&mut self, now: f64) {
        if self.paused_until.take().is_some() {
            self.last = now;
        }
    }

    pub fn reset_timers(&mut self) {
        self.eye_sec = 0.0;
        self.walk_sec = 0.0;
        self.warned = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Eyes, Idle, Walk};

    struct Sim {
        s: Scheduler,
        now: f64,
        idle: f64,
        events: Vec<(&'static str, u32)>,
    }

    impl Sim {
        fn new(settings: Settings, idle_reliable: bool) -> Self {
            Sim { s: Scheduler::new(settings, idle_reliable, 1000.0), now: 1000.0, idle: 0.0, events: vec![] }
        }
        fn at(&self) -> u32 {
            (self.now - 1000.0) as u32
        }
        fn jump(&mut self, sec: f64) {
            self.now += sec;
            let idle = self.idle;
            let name = match self.s.tick(self.now, || idle) {
                Some(Event::EyeDue) => "eye-due",
                Some(Event::WalkDue) => "walk-due",
                Some(Event::WalkWarning(_)) => "walk-warning",
                Some(Event::NaturalBreak) => "natural-break",
                None => return,
            };
            let at = self.at();
            self.events.push((name, at));
        }
        fn run(&mut self, sec: u32) {
            (0..sec).for_each(|_| self.jump(1.0));
        }
        fn names(&self) -> Vec<&str> {
            self.events.iter().map(|e| e.0).collect()
        }
    }

    fn cfg(eyes: Eyes, walk: Walk, idle: Idle) -> Settings {
        Settings { eyes, walk, idle, ..Settings::default() }
    }
    fn eyes(interval_min: f64) -> Eyes {
        Eyes { interval_min, ..Eyes::default() }
    }
    fn walk(interval_min: f64) -> Walk {
        Walk { interval_min, warn_sec: 0.0, ..Walk::default() }
    }
    fn no_walk() -> Walk {
        Walk { enabled: false, ..Walk::default() }
    }
    fn no_eyes() -> Eyes {
        Eyes { enabled: false, ..Eyes::default() }
    }

    #[test]
    fn eye_exercise_is_due_after_the_interval_and_restarts() {
        let mut sim = Sim::new(cfg(eyes(1.0), no_walk(), Idle::default()), true);
        sim.run(59);
        assert!(sim.events.is_empty());
        sim.run(1);
        assert_eq!(sim.events, [("eye-due", 60)]);
        sim.run(30);
        assert_eq!(sim.events.len(), 1, "nothing piles up while the face is on screen");
        sim.s.finish_eye();
        sim.run(60);
        assert_eq!(sim.events, [("eye-due", 60), ("eye-due", 150)]);
    }

    #[test]
    fn walking_break_warns_in_advance_and_then_becomes_due() {
        let walk = Walk { interval_min: 2.0, warn_sec: 30.0, ..Walk::default() };
        let mut sim = Sim::new(cfg(no_eyes(), walk, Idle::default()), true);
        sim.run(120);
        assert_eq!(sim.events, [("walk-warning", 90), ("walk-due", 120)]);
    }

    #[test]
    fn snooze_brings_the_break_back_after_the_snooze_time() {
        let walk = Walk { interval_min: 10.0, snooze_min: 1.0, warn_sec: 0.0, ..Walk::default() };
        let mut sim = Sim::new(cfg(no_eyes(), walk, Idle::default()), true);
        sim.run(600);
        assert_eq!(sim.names(), ["walk-due"]);
        sim.s.finish_walk(WalkResult::Snoozed);
        sim.run(60);
        assert_eq!(sim.events, [("walk-due", 600), ("walk-due", 660)]);
    }

    #[test]
    fn a_finished_break_also_resets_the_eye_timer_a_skipped_one_does_not() {
        let mut sim = Sim::new(cfg(eyes(20.0), walk(5.0), Idle::default()), true);
        sim.run(300);
        assert_eq!(sim.s.busy, Some(Busy::Walk));
        sim.s.finish_walk(WalkResult::Done);
        assert_eq!(sim.s.eye_sec, 0.0);
        sim.run(300);
        sim.s.finish_walk(WalkResult::Skipped);
        assert_eq!(sim.s.walk_sec, 0.0);
        assert_eq!(sim.s.eye_sec, 300.0);
    }

    #[test]
    fn eye_exercise_is_dropped_when_the_walking_break_is_about_to_start() {
        let mut sim = Sim::new(cfg(eyes(10.0), walk(11.0), Idle::default()), true);
        sim.run(660);
        assert_eq!(sim.names(), ["walk-due"]);
    }

    #[test]
    fn being_away_long_enough_resets_both_timers_once() {
        let mut sim = Sim::new(cfg(eyes(20.0), walk(50.0), Idle { reset_after_min: 5.0 }), true);
        sim.run(600);
        sim.idle = 300.0;
        sim.run(5);
        assert_eq!(sim.names(), ["natural-break"]);
        assert_eq!((sim.s.eye_sec, sim.s.walk_sec), (0.0, 0.0));
        sim.run(100);
        assert_eq!(sim.s.walk_sec, 0.0, "time away does not count");
        sim.idle = 0.0;
        sim.run(10);
        assert_eq!(sim.s.walk_sec, 10.0);
        assert_eq!(sim.names(), ["natural-break"]);
    }

    #[test]
    fn sleep_counts_as_being_away() {
        let mut sim = Sim::new(cfg(eyes(20.0), walk(50.0), Idle { reset_after_min: 5.0 }), true);
        sim.run(600);
        sim.jump(3600.0);
        assert_eq!(sim.names(), ["natural-break"]);
        assert_eq!(sim.s.walk_sec, 0.0);
    }

    #[test]
    fn without_idle_reset_a_long_sleep_adds_only_a_few_seconds() {
        let mut sim = Sim::new(cfg(eyes(20.0), walk(50.0), Idle { reset_after_min: 0.0 }), true);
        sim.run(100);
        sim.jump(3600.0);
        assert_eq!(sim.s.walk_sec, 115.0);
    }

    #[test]
    fn unreliable_idle_source_never_triggers_natural_breaks() {
        let mut sim = Sim::new(cfg(eyes(20.0), walk(50.0), Idle { reset_after_min: 1.0 }), false);
        sim.idle = 10_000.0;
        sim.run(120);
        assert!(sim.events.is_empty());
    }

    #[test]
    fn pause_freezes_the_timers_and_resumes_on_its_own() {
        let mut sim = Sim::new(cfg(eyes(1.0), no_walk(), Idle::default()), true);
        sim.run(30);
        sim.s.pause(Some(10.0), sim.now);
        sim.run(599);
        assert_eq!(sim.s.eye_sec, 30.0);
        assert!(sim.events.is_empty());
        sim.run(1);
        assert!(sim.events.is_empty());
        sim.run(29);
        assert_eq!(sim.names(), ["eye-due"]);
    }

    #[test]
    fn manual_reminders_cannot_overlap() {
        let mut s = Scheduler::new(Settings::default(), true, 0.0);
        assert!(s.begin(Busy::Eye));
        assert!(!s.begin(Busy::Walk));
        s.finish_eye();
        assert!(s.begin(Busy::Walk));
    }
}
