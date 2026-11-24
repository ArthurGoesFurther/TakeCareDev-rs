//! What the face does at each second of an exercise.
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Lids { Open, Half, Closed, Relaxed, Squeeze }
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mouth { Smile, Open, O, Tense }
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Brows { None, Raised, Tense }

/// look_x/look_y in [-1, 1], cross in [0, 2] (pupils toward the nose).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pose {
    pub look_x: f32,
    pub look_y: f32,
    pub cross: f32,
    pub lids: Lids,
    pub mouth: Mouth,
    pub brows: Brows,
}

impl Default for Pose {
    fn default() -> Self {
        Self { look_x: 0.0, look_y: 0.0, cross: 0.0, lids: Lids::Open, mouth: Mouth::Smile, brows: Brows::None }
    }
}

impl Pose {
    /// Fingerprint at sprite resolution, to skip redrawing identical frames.
    pub fn key(&self) -> [i32; 6] {
        [
            (self.look_x * 2.0).round() as i32,
            (self.look_y * 2.0).round() as i32,
            self.cross as i32,
            self.lids as i32,
            self.mouth as i32,
            self.brows as i32,
        ]
    }
}

/// Sine that lingers at the extremes, so the pupils hold each side for a moment.
fn swing(t: f32, period: f32) -> f32 {
    ((TAU * t / period).sin() * 1.7).clamp(-1.0, 1.0)
}

/// Half-period boundary closest to the middle: switching there keeps the pupils from jumping.
fn mid_switch(duration: f32, period: f32) -> f32 {
    (period / 2.0).max((duration / period).round() * (period / 2.0))
}

pub struct Exercise {
    pub id: &'static str,
    pose: fn(f32, f32) -> Pose,
}

const P: Pose = Pose { look_x: 0.0, look_y: 0.0, cross: 0.0, lids: Lids::Open, mouth: Mouth::Smile, brows: Brows::None };

pub const EXERCISES: [Exercise; 10] = [
    Exercise { id: "leftRight", pose: |t, _| Pose { look_x: swing(t, 2.4), ..P } },
    Exercise { id: "upDown", pose: |t, _| Pose { look_y: -swing(t, 2.4), ..P } },
    Exercise {
        id: "circles",
        pose: |t, d| {
            let w = TAU / 2.8;
            let half = d / 2.0;
            let a = if t < half { w * t } else { w * (2.0 * half - t) };
            Pose { look_x: a.cos(), look_y: a.sin(), ..P }
        },
    },
    Exercise {
        id: "diagonals",
        pose: |t, d| {
            let s = swing(t, 2.4);
            let flip = if t < mid_switch(d, 2.4) { 1.0 } else { -1.0 };
            Pose { look_x: s, look_y: s * flip, ..P }
        },
    },
    Exercise {
        id: "figureEight",
        pose: |t, _| {
            let a = TAU * t / 3.4;
            Pose { look_x: a.sin(), look_y: (2.0 * a).sin() * 0.9, ..P }
        },
    },
    Exercise {
        id: "blink",
        pose: |t, _| Pose { lids: if t % 0.6 < 0.16 { Lids::Closed } else { Lids::Open }, ..P },
    },
    Exercise {
        id: "squeeze",
        pose: |t, _| {
            if t % 3.6 < 2.2 {
                Pose { lids: Lids::Squeeze, mouth: Mouth::Tense, brows: Brows::Tense, ..P }
            } else {
                Pose { mouth: Mouth::O, brows: Brows::Raised, ..P }
            }
        },
    },
    Exercise {
        id: "farAway",
        pose: |t, _| Pose {
            look_x: 0.6 + (t * 0.9).sin() * 0.4,
            look_y: -0.8,
            lids: if t % 4.0 < 0.18 { Lids::Closed } else { Lids::Open },
            ..P
        },
    },
    Exercise {
        id: "nearFar",
        pose: |t, _| {
            if t % 3.2 < 1.6 {
                Pose { cross: 2.0, look_y: 0.6, mouth: Mouth::O, ..P }
            } else {
                Pose { look_y: -0.6, ..P }
            }
        },
    },
    Exercise { id: "palming", pose: |_, _| Pose { lids: Lids::Relaxed, ..P } },
];

pub const EXERCISE_IDS: [&str; 10] = [
    "leftRight", "upDown", "circles", "diagonals", "figureEight", "blink", "squeeze", "farAway", "nearFar", "palming",
];

pub fn by_id(id: &str) -> &'static Exercise {
    EXERCISES.iter().find(|e| e.id == id).unwrap_or(&EXERCISES[0])
}

pub const INTRO_SEC: f32 = 0.6;

/// Pose for the whole popup: a short greeting, then the exercise itself.
pub fn pose_at(ex: &Exercise, t: f32, duration: f32) -> Pose {
    if t < INTRO_SEC {
        Pose { mouth: Mouth::Open, ..P }
    } else {
        (ex.pose)(t - INTRO_SEC, duration - INTRO_SEC)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_match_and_poses_stay_in_range() {
        assert!(EXERCISES.iter().zip(EXERCISE_IDS).all(|(e, id)| e.id == id));
        for ex in &EXERCISES {
            for i in 0..300 {
                let p = pose_at(ex, i as f32 / 10.0, 30.0);
                assert!(p.look_x.abs() <= 1.0001 && p.look_y.abs() <= 1.0001, "{}", ex.id);
            }
        }
    }
}
