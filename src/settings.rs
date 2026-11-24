//! Settings schema, defaults and clamping. The JSON layout matches the Electron version.
use crate::exercises::EXERCISE_IDS;
use serde::{Deserialize, Serialize};

pub const POSITIONS: [&str; 6] = ["cursor", "top-left", "top-right", "bottom-left", "bottom-right", "center"];
pub const ORDERS: [&str; 2] = ["random", "sequential"];
pub const LANGUAGES: [&str; 3] = ["auto", "ru", "en"];

#[derive(Clone, Copy)]
pub struct Limit {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

const fn lim(min: f64, max: f64, step: f64) -> Limit {
    Limit { min, max, step }
}

pub const EYES_INTERVAL: Limit = lim(1.0, 240.0, 0.5);
pub const EYES_DURATION: Limit = lim(3.0, 120.0, 1.0);
pub const EYES_SCALE: Limit = lim(2.0, 8.0, 1.0);
pub const WALK_INTERVAL: Limit = lim(1.0, 480.0, 0.5);
pub const WALK_DURATION: Limit = lim(0.5, 60.0, 0.5);
pub const WALK_SNOOZE: Limit = lim(1.0, 120.0, 1.0);
pub const WALK_WARN: Limit = lim(0.0, 600.0, 5.0);
pub const IDLE_RESET: Limit = lim(0.0, 240.0, 0.5);

impl Limit {
    pub fn clamp(self, v: f64) -> f64 {
        if !v.is_finite() {
            return self.min;
        }
        ((v / self.step).round() * self.step).clamp(self.min, self.max)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Eyes {
    pub enabled: bool,
    pub interval_min: f64,
    pub duration_sec: f64,
    pub exercises: Vec<String>,
    pub order: String,
    pub position: String,
    pub follow_cursor: bool,
    pub face_scale: f64,
    pub show_hint: bool,
    pub sound: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Walk {
    pub enabled: bool,
    pub interval_min: f64,
    pub duration_min: f64,
    pub snooze_min: f64,
    pub warn_sec: f64,
    pub require_idle: bool,
    pub strict: bool,
    pub sound: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Idle {
    pub reset_after_min: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub language: String,
    pub autostart: bool,
    pub eyes: Eyes,
    pub walk: Walk,
    pub idle: Idle,
}

impl Default for Eyes {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_min: 20.0,
            duration_sec: 10.0,
            exercises: EXERCISE_IDS.iter().map(|s| s.to_string()).collect(),
            order: "random".into(),
            position: "cursor".into(),
            follow_cursor: true,
            face_scale: 4.0,
            show_hint: true,
            sound: false,
        }
    }
}

impl Default for Walk {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_min: 50.0,
            duration_min: 5.0,
            snooze_min: 5.0,
            warn_sec: 30.0,
            require_idle: true,
            strict: false,
            sound: true,
        }
    }
}

impl Default for Idle {
    fn default() -> Self {
        Self { reset_after_min: 5.0 }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            autostart: false,
            eyes: Eyes::default(),
            walk: Walk::default(),
            idle: Idle::default(),
        }
    }
}

fn one_of(value: &mut String, list: &[&str], fallback: &str) {
    if !list.contains(&value.as_str()) {
        *value = fallback.into();
    }
}

impl Settings {
    /// Clamps numbers to their limits and replaces unknown values with defaults.
    pub fn normalized(mut self) -> Self {
        let d = Settings::default();
        one_of(&mut self.language, &LANGUAGES, &d.language);
        let e = &mut self.eyes;
        e.interval_min = EYES_INTERVAL.clamp(e.interval_min);
        e.duration_sec = EYES_DURATION.clamp(e.duration_sec);
        e.face_scale = EYES_SCALE.clamp(e.face_scale);
        one_of(&mut e.order, &ORDERS, &d.eyes.order);
        one_of(&mut e.position, &POSITIONS, &d.eyes.position);
        let picked: Vec<String> = EXERCISE_IDS
            .iter()
            .filter(|id| e.exercises.iter().any(|x| x == *id))
            .map(|s| s.to_string())
            .collect();
        e.exercises = if picked.is_empty() { d.eyes.exercises } else { picked };
        let w = &mut self.walk;
        w.interval_min = WALK_INTERVAL.clamp(w.interval_min);
        w.duration_min = WALK_DURATION.clamp(w.duration_min);
        w.snooze_min = WALK_SNOOZE.clamp(w.snooze_min);
        w.warn_sec = WALK_WARN.clamp(w.warn_sec);
        self.idle.reset_after_min = IDLE_RESET.clamp(self.idle.reset_after_min);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_and_falls_back_to_defaults() {
        let raw = r#"{"language":"de","eyes":{"intervalMin":0,"durationSec":9999,
            "exercises":["blink","nope"],"position":"moon"},"walk":{"durationMin":2.5,"warnSec":-5}}"#;
        let s = serde_json::from_str::<Settings>(raw).unwrap().normalized();
        assert_eq!(s.language, "auto");
        assert_eq!(s.eyes.interval_min, 1.0);
        assert_eq!(s.eyes.duration_sec, 120.0);
        assert_eq!(s.eyes.exercises, ["blink"]);
        assert_eq!(s.eyes.position, "cursor");
        assert_eq!(s.walk.duration_min, 2.5);
        assert_eq!(s.walk.warn_sec, 0.0);
        let none = Settings { eyes: Eyes { exercises: vec![], ..Eyes::default() }, ..Settings::default() };
        assert_eq!(none.normalized().eyes.exercises.len(), 10);
    }
}
