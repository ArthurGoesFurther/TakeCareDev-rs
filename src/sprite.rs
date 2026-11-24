//! The pixel face, drawn in code into any `put(x, y, rgba)` sink — no image assets.
use crate::exercises::{Brows, Lids, Mouth, Pose};

pub type Rgba = [u8; 4];
pub const FACE_W: i32 = 26;
pub const FACE_H: i32 = 23;
pub const WALKER_H: i32 = FACE_H + 4;
pub const TRAY_SIZE: i32 = 16;

const fn hex(c: u32) -> Rgba {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]
}

pub const OUTLINE: Rgba = hex(0x3b2a40);
pub const SKIN: Rgba = hex(0xffd45e);
const SHADE: Rgba = hex(0xf0aa3c);
const LIGHT: Rgba = hex(0xfff3bd);
const WHITE: Rgba = hex(0xffffff);
const PUPIL: Rgba = hex(0x2a1f3d);
const BLUSH_C: Rgba = hex(0xff9a8f);
const MOUTH_C: Rgba = hex(0xb8325a);
const SHOE: Rgba = hex(0xe0475b);

/// RGBA canvas where every sprite pixel becomes a `scale`×`scale` block.
pub struct Raster {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
    w: i32,
    h: i32,
    scale: i32,
}

impl Raster {
    pub fn new(w: i32, h: i32, scale: i32) -> Self {
        Self { width: w * scale, height: h * scale, data: vec![0; (w * h * scale * scale * 4) as usize], w, h, scale }
    }

    pub fn put(&mut self, x: i32, y: i32, c: Rgba) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        for dy in 0..self.scale {
            for dx in 0..self.scale {
                let i = (((y * self.scale + dy) * self.width + x * self.scale + dx) * 4) as usize;
                self.data[i..i + 4].copy_from_slice(&c);
            }
        }
    }
}

struct Mask {
    w: i32,
    h: i32,
    cells: Vec<bool>,
}

impl Mask {
    fn new(w: i32, h: i32, inside: impl Fn(i32, i32) -> bool) -> Self {
        let cells = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| inside(x, y)).collect();
        Self { w, h, cells }
    }
    fn has(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h && self.cells[(y * self.w + x) as usize]
    }
    fn edge(&self, x: i32, y: i32) -> bool {
        self.has(x, y) && !(self.has(x - 1, y) && self.has(x + 1, y) && self.has(x, y - 1) && self.has(x, y + 1))
    }
}

fn masks() -> (Mask, Mask) {
    let head = Mask::new(FACE_W, FACE_H, |x, y| {
        let nx = (x as f32 + 0.5 - FACE_W as f32 / 2.0) / (FACE_W as f32 / 2.0);
        let ny = (y as f32 + 0.5 - FACE_H as f32 / 2.0) / (FACE_H as f32 / 2.0);
        nx.abs().powf(2.3) + ny.abs().powf(2.3) <= 1.0
    });
    let eye = Mask::new(9, 9, |x, y| ((x - 4).pow(2) + (y - 4).pow(2)) as f32 <= 4.4 * 4.4);
    (head, eye)
}

const BLUSH: [(i32, i32); 4] = [(3, 15), (4, 15), (21, 15), (22, 15)];
const SQUEEZE_L: [(i32, i32); 9] = [(2, 2), (3, 2), (4, 3), (5, 3), (6, 4), (4, 5), (5, 5), (2, 6), (3, 6)];
const SQUEEZE_R: [(i32, i32); 9] = [(5, 2), (6, 2), (3, 3), (4, 3), (2, 4), (3, 5), (4, 5), (5, 6), (6, 6)];
const BROWS_RAISED: [(i32, i32); 8] = [(5, 3), (6, 3), (7, 3), (8, 3), (17, 3), (18, 3), (19, 3), (20, 3)];
const BROWS_TENSE: [(i32, i32); 8] = [(4, 5), (5, 5), (6, 6), (7, 6), (18, 6), (19, 6), (20, 5), (21, 5)];

/// (outline pixels, mouth-colored fill)
fn mouth(m: Mouth) -> (&'static [(i32, i32)], &'static [(i32, i32)]) {
    match m {
        Mouth::Smile => (&[(10, 16), (15, 16), (11, 17), (12, 17), (13, 17), (14, 17)], &[]),
        Mouth::Open => (
            &[(10, 16), (11, 16), (12, 16), (13, 16), (14, 16), (15, 16), (10, 17), (15, 17), (11, 18), (14, 18), (12, 19), (13, 19)],
            &[(11, 17), (12, 17), (13, 17), (14, 17), (12, 18), (13, 18)],
        ),
        Mouth::O => (&[(12, 16), (13, 16), (11, 17), (14, 17), (12, 18), (13, 18)], &[(12, 17), (13, 17)]),
        Mouth::Tense => (&[(10, 17), (11, 16), (12, 17), (13, 17), (14, 16), (15, 17)], &[]),
    }
}

/// JS-style rounding (halves go up), so the sprites match the original pixel for pixel.
fn round(v: f32) -> i32 {
    (v + 0.5).floor() as i32
}

fn head_color(head: &Mask, x: i32, y: i32) -> Rgba {
    let (cx, cy) = (FACE_W as f32 / 2.0, FACE_H as f32 / 2.0);
    if head.edge(x, y) {
        OUTLINE
    } else if y as f32 > cy && (head.edge(x, y + 1) || (x as f32 > cx && head.edge(x + 1, y))) {
        SHADE
    } else if (y as f32) < cy && (x as f32) < cx && (head.edge(x, y - 1) || head.edge(x - 1, y)) {
        LIGHT
    } else {
        SKIN
    }
}

fn draw_eye(put: &mut impl FnMut(i32, i32, Rgba), eye: &Mask, (ex, ey, side): (i32, i32, i32), p: &Pose) {
    let mut at = |x: i32, y: i32, c: Rgba| put(ex + x, ey + y, c);
    let inner = |x: i32, y: i32| eye.has(x, y) && !eye.edge(x, y);

    if matches!(p.lids, Lids::Open | Lids::Half) {
        let lid = if p.lids == Lids::Half { 3 } else { -1 };
        for y in 0..9 {
            for x in 0..9 {
                if !eye.has(x, y) {
                    continue;
                }
                if y < lid {
                    at(x, y, SKIN);
                } else if y == lid || eye.edge(x, y) {
                    at(x, y, OUTLINE);
                } else {
                    at(x, y, WHITE);
                }
            }
        }
        let dx = round(p.look_x.clamp(-1.0, 1.0) * 2.0 - side as f32 * p.cross.clamp(0.0, 2.0));
        let dy = round(p.look_y.clamp(-1.0, 1.0) * 2.0);
        let (x0, y0) = (3 + dx.clamp(-2, 2), 3 + dy);
        for y in y0..y0 + 3 {
            for x in x0..x0 + 3 {
                if inner(x, y) && y > lid {
                    at(x, y, PUPIL);
                }
            }
        }
        if inner(x0, y0) && y0 > lid {
            at(x0, y0, WHITE);
        }
        return;
    }

    for y in 0..9 {
        for x in 0..9 {
            if eye.has(x, y) {
                at(x, y, SKIN);
            }
        }
    }
    match p.lids {
        Lids::Closed => (1..8).for_each(|x| at(x, 4, OUTLINE)),
        Lids::Relaxed => {
            at(1, 4, OUTLINE);
            at(7, 4, OUTLINE);
            (2..7).for_each(|x| at(x, 5, OUTLINE));
        }
        Lids::Squeeze => {
            for &(x, y) in if side < 0 { &SQUEEZE_L } else { &SQUEEZE_R } {
                at(x, y, OUTLINE);
            }
        }
        _ => {}
    }
}

/// Draws the face at offset (ox, oy).
pub fn draw_face(put: &mut impl FnMut(i32, i32, Rgba), pose: &Pose, ox: i32, oy: i32) {
    let (head, eye) = masks();
    let mut px = |x: i32, y: i32, c: Rgba| put(x + ox, y + oy, c);
    for y in 0..FACE_H {
        for x in 0..FACE_W {
            if head.has(x, y) {
                px(x, y, head_color(&head, x, y));
            }
        }
    }
    for (x, y) in BLUSH {
        px(x, y, BLUSH_C);
    }
    for e in [(3, 5, -1), (14, 5, 1)] {
        draw_eye(&mut px, &eye, e, pose);
    }
    let brows: &[(i32, i32)] = match pose.brows {
        Brows::None => &[],
        Brows::Raised => &BROWS_RAISED,
        Brows::Tense => &BROWS_TENSE,
    };
    for &(x, y) in brows {
        px(x, y, OUTLINE);
    }
    let (line, fill) = mouth(pose.mouth);
    for &(x, y) in fill {
        px(x, y, MOUTH_C);
    }
    for &(x, y) in line {
        px(x, y, OUTLINE);
    }
}

fn draw_shoe(put: &mut impl FnMut(i32, i32, Rgba), x0: i32, y0: i32) {
    for y in 0..3 {
        for x in 0..5 {
            if (y == 0 || y == 2) && (x == 0 || x == 4) {
                continue;
            }
            let edge = y == 0 || y == 2 || x == 0 || x == 4;
            put(x0 + x, y0 + y, if edge { OUTLINE } else { SHOE });
        }
    }
}

/// The face stepping in place: shoes alternate, head bobs on each step.
pub fn draw_walker(put: &mut impl FnMut(i32, i32, Rgba), t: f32, pose: &Pose) {
    let step = (t / 0.32).floor() as i32 % 2;
    draw_face(put, pose, 0, step);
    draw_shoe(put, 5, if step == 0 { FACE_H } else { FACE_H + 1 });
    draw_shoe(put, 16, if step == 0 { FACE_H + 1 } else { FACE_H });
}

const TRAY_MAP: [&str; 16] = [
    ".....######.....",
    "...##oooooo##...",
    "..#oooooooooo#..",
    ".#oooooooooooo#.",
    ".#oooooooooooo#.",
    "#oooEEooooEEooo#",
    "#ooEwpEooEwpEoo#",
    "#ooEppEooEppEoo#",
    "#oooEEooooEEooo#",
    "#oooooooooooooo#",
    "#bboo#oooo#oobb#",
    ".#oooo####oooo#.",
    ".#oooooooooooo#.",
    "..#oooooooooo#..",
    "...##oooooo##...",
    ".....######.....",
];

/// 16×16 tray glyph; `mono` draws a silhouette for macOS template images.
pub fn draw_tray_icon(put: &mut impl FnMut(i32, i32, Rgba), sleeping: bool, mono: bool) {
    for (y, row) in TRAY_MAP.iter().enumerate() {
        let row = match (sleeping, y) {
            (true, 5 | 6 | 8) => "#oooooooooooooo#",
            (true, 7) => "#ooEEEEooEEEEoo#",
            _ => row,
        };
        for (x, ch) in row.chars().enumerate() {
            let c = match ch {
                '.' => continue,
                'w' | 'o' | 'b' if mono => continue,
                _ if mono => [0, 0, 0, 255],
                '#' | 'E' => OUTLINE,
                'o' => SKIN,
                'w' => WHITE,
                'p' => PUPIL,
                'b' => BLUSH_C,
                _ => continue,
            };
            put(x as i32, y as i32, c);
        }
    }
}

/// The window/app icon: the default face on a 32×32 grid.
pub fn app_icon(scale: i32) -> Raster {
    let mut r = Raster::new(32, 32, scale);
    draw_face(&mut |x, y, c| r.put(x, y, c), &Pose::default(), 3, 5);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_fits_its_box_and_is_not_empty() {
        let mut r = Raster::new(FACE_W, FACE_H, 1);
        let mut max = (0, 0);
        draw_face(
            &mut |x, y, c| {
                max = (max.0.max(x), max.1.max(y));
                r.put(x, y, c)
            },
            &Pose { mouth: Mouth::Open, ..Pose::default() },
            0,
            0,
        );
        assert!(max.0 < FACE_W && max.1 < FACE_H);
        assert!(r.data.chunks(4).filter(|p| p[3] == 255).count() > 300);
    }
}
