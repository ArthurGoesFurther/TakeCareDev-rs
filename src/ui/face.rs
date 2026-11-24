//! The pixel face popup: transparent, click-through, never takes focus.
use super::{ACCENT, INK, PAPER, Sprite, TRACK};
use crate::exercises::{self, pose_at};
use crate::i18n::tr;
use crate::platform::{self, Pointer, Rect};
use crate::sound;
use crate::sprite::{FACE_H, FACE_W, draw_face};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Stroke, Vec2, ViewportCommand, pos2, vec2};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const PAD: f32 = 10.0;
const GAP: f32 = 12.0;
const BUBBLE_W: f32 = 236.0;
const BUBBLE_H: f32 = 124.0;
const MINI_BAR_H: f32 = 14.0;
const CURSOR_GAP: f32 = 22.0;
const EDGE: f32 = 24.0;
const FADE_IN: f32 = 0.15;
const FADE_OUT: f32 = 0.22;
/// How long to wait for XWayland to report the real pointer position.
const PROBE_MAX: Duration = Duration::from_millis(400);

#[derive(Clone, Serialize, Deserialize)]
pub struct FaceCfg {
    pub exercise: String,
    pub duration_sec: f64,
    pub scale: i32,
    pub show_hint: bool,
    pub sound: bool,
    pub position: String,
    pub follow_cursor: bool,
    pub cursor_mode: String,
    pub lang: String,
}

fn window_size(cfg: &FaceCfg) -> Vec2 {
    let scale = cfg.scale as f32;
    let face_h = FACE_H as f32 * scale + scale + if cfg.show_hint { 0.0 } else { MINI_BAR_H };
    vec2(
        PAD * 2.0 + FACE_W as f32 * scale + if cfg.show_hint { GAP + BUBBLE_W } else { 0.0 },
        PAD * 2.0 + face_h.max(if cfg.show_hint { BUBBLE_H } else { 0.0 }),
    )
}

/// Diagonally below-right of the pointer, mirrored when that would leave the screen.
fn near_cursor(p: Pos2, size: Vec2, area: Rect, side: Option<bool>) -> (Pos2, bool) {
    let (ax, ay, aw, ah) = (area.x as f32, area.y as f32, area.w as f32, area.h as f32);
    let left = side.unwrap_or(p.x + CURSOR_GAP - PAD + size.x > ax + aw);
    let x = if left { p.x - CURSOR_GAP + PAD - size.x } else { p.x + CURSOR_GAP - PAD };
    let below = p.y + CURSOR_GAP - PAD;
    let y = if below + size.y > ay + ah { p.y - CURSOR_GAP + PAD - size.y } else { below };
    let clamp = |v: f32, lo: f32, hi: f32| v.min(hi).max(lo);
    (pos2(clamp(x.round(), ax, ax + aw - size.x), clamp(y.round(), ay, ay + ah - size.y)), left)
}

/// Returns the window's top-left corner and whether the face sits on the right (bubble to its left).
fn place(position: &str, pointer: Option<Pos2>, size: Vec2, area: Rect) -> (Pos2, bool) {
    if position == "cursor" {
        if let Some(p) = pointer {
            return near_cursor(p, size, area, None);
        }
    }
    let (ax, ay, aw, ah) = (area.x as f32, area.y as f32, area.w as f32, area.h as f32);
    let (right, bottom) = (ax + aw - size.x - EDGE, ay + ah - size.y - EDGE);
    let (x, y, left) = match position {
        "top-left" => (ax + EDGE, ay + EDGE, false),
        "top-right" => (right, ay + EDGE, true),
        "bottom-left" => (ax + EDGE, bottom, false),
        "center" => (ax + (aw - size.x) / 2.0, ay + (ah - size.y) / 2.0, false),
        _ => (right, bottom, true),
    };
    (pos2(x.round(), y.round()), left)
}

struct Placed {
    at: Instant,
    left: bool,
    size_px: Vec2,
    pos: Pos2,
}

struct FaceApp {
    cfg: FaceCfg,
    size: Vec2,
    pointer: Pointer,
    start_pointer: Option<(i32, i32)>,
    born: Instant,
    placed: Option<Placed>,
    face: Sprite,
    next_move: Instant,
    monitors: Vec<Rect>,
}

pub fn run(json: &str) {
    let Ok(cfg) = serde_json::from_str::<FaceCfg>(json) else {
        eprintln!("[tcdev] bad face config");
        std::process::exit(2);
    };
    let pointer = Pointer::new();
    let probe = cfg.position == "cursor" && cfg.cursor_mode == "probe";
    let start_pointer = pointer.get();
    let size = window_size(&cfg);
    // The probe window must cover the whole screen: XWayland reports the pointer only over our windows.
    let initial = if probe {
        pointer.screen_size().map_or(size, |(w, h)| vec2(w as f32, h as f32))
    } else {
        size
    };
    #[allow(unused_mut)]
    let mut viewport = egui::ViewportBuilder::default()
        .with_decorations(false)
        .with_transparent(true)
        .with_taskbar(false)
        .with_active(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_inner_size(initial)
        .with_position(pos2(0.0, 0.0))
        .with_mouse_passthrough(!probe);
    #[cfg(target_os = "linux")]
    {
        viewport = viewport.with_window_type(egui::viewport::X11WindowType::Notification);
    }
    if cfg.sound {
        sound::chirp(&[660.0, 990.0]);
    }
    super::run("TakeCareDev", viewport, true, move |_| {
        Box::new(FaceApp {
            cfg,
            size,
            pointer,
            start_pointer,
            born: Instant::now(),
            placed: None,
            face: Sprite::default(),
            next_move: Instant::now(),
            monitors: platform::monitors(),
        })
    });
}

impl FaceApp {
    fn area(&self, ctx: &egui::Context) -> (Rect, f32) {
        let ppp = ctx.pixels_per_point();
        let point = self.pointer_px(ppp);
        let found = point.and_then(|p| self.monitors.iter().find(|m| m.contains(p))).or(self.monitors.first()).copied();
        let fallback = || {
            let m = ctx.input(|i| i.viewport().monitor_size).unwrap_or(vec2(1920.0, 1080.0));
            Rect { x: 0, y: 0, w: (m.x * ppp) as i32, h: (m.y * ppp) as i32 }
        };
        (found.unwrap_or_else(fallback), ppp)
    }

    /// Pointer in physical pixels (macOS reports points).
    fn pointer_px(&self, ppp: f32) -> Option<(i32, i32)> {
        let unit = if platform::POINTER_IN_POINTS { ppp } else { 1.0 };
        self.pointer.get().or(self.start_pointer).map(|(x, y)| ((x as f32 * unit) as i32, (y as f32 * unit) as i32))
    }

    /// Picks the final spot (pixels) and moves/resizes the window there.
    fn do_place(&mut self, ctx: &egui::Context, pointer: Option<(i32, i32)>) {
        let (area, ppp) = self.area(ctx);
        let size_px = self.size * ppp;
        let cursor_known = pointer.is_some();
        let position = if self.cfg.position == "cursor" && !cursor_known { "bottom-right" } else { self.cfg.position.as_str() };
        let p = pointer.map(|(x, y)| pos2(x as f32, y as f32));
        let (pos, left) = place(position, p, size_px, area);
        eprintln!("[tcdev] eyes: {} at {},{} ({position}, pointer {pointer:?})", self.cfg.exercise, pos.x, pos.y);
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(self.size));
        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos / ppp));
        ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(true));
        self.placed = Some(Placed { at: Instant::now(), left, size_px, pos });
    }

    fn follow(&mut self, ctx: &egui::Context) {
        let (follow, mode) = (self.cfg.follow_cursor, self.cfg.cursor_mode.as_str());
        if self.placed.is_none() || !(follow && mode == "direct" && self.cfg.position == "cursor") || Instant::now() < self.next_move {
            return;
        }
        let Some((x, y)) = self.pointer_px(ctx.pixels_per_point()) else { return };
        let (area, _) = self.area(ctx);
        let Some(placed) = &mut self.placed else { return };
        let (target, _) = near_cursor(pos2(x as f32, y as f32), placed.size_px, area, Some(placed.left));
        let approach = |from: f32, to: f32| if (to - from).abs() <= 1.0 { to } else { (from + (to - from) * 0.35).round() };
        let next = pos2(approach(placed.pos.x, target.x), approach(placed.pos.y, target.y));
        if next != placed.pos {
            placed.pos = next;
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(next / ctx.pixels_per_point()));
        }
        self.next_move = Instant::now() + Duration::from_millis(33);
    }
}

impl eframe::App for FaceApp {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.placed.is_none() {
            let wait_probe = self.cfg.position == "cursor" && self.cfg.cursor_mode == "probe";
            let now = self.pointer.get();
            let moved = now != self.start_pointer;
            if wait_probe && !moved && self.born.elapsed() < PROBE_MAX {
                ctx.request_repaint_after(Duration::from_millis(30));
                return;
            }
            let pointer = match self.cfg.cursor_mode.as_str() {
                "none" => None,
                // A probe that saw no movement still has a usable (possibly stale) position.
                _ => now.and(self.pointer_px(ctx.pixels_per_point())),
            };
            self.do_place(&ctx, pointer);
        }
        self.follow(&ctx);

        let Some(placed) = &self.placed else { return };
        let t = placed.at.elapsed().as_secs_f32();
        let duration = self.cfg.duration_sec as f32;
        let left = (duration - t).max(0.0);
        let alpha = if left > 0.0 { (t / FADE_IN).min(1.0) } else { 1.0 - (t - duration) / FADE_OUT };
        if alpha <= 0.0 {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }

        let ex = exercises::by_id(&self.cfg.exercise);
        let pose = pose_at(ex, t.min(duration), duration);
        let bob = ((t / 0.5).floor() as i32) % 2;
        let mut key = pose.key().to_vec();
        key.push(0);
        self.face.update(&ctx, key, FACE_W, FACE_H, |r| draw_face(&mut |x, y, c| r.put(x, y, c), &pose, 0, 0));

        let scale = self.cfg.scale as f32;
        let flip = placed.left;
        let bubble_w = BUBBLE_W - 6.0;
        let face_size = vec2(FACE_W as f32, FACE_H as f32) * scale;
        let col_h = face_size.y + scale + if self.cfg.show_hint { 0.0 } else { MINI_BAR_H };
        let face_x = if flip { self.size.x - PAD - face_size.x } else { PAD };
        let face_y = (self.size.y - col_h) / 2.0 - bob as f32 * scale + scale;
        let fade = |c: Color32| c.gamma_multiply(alpha.clamp(0.0, 1.0));

        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            let p = ui.painter().clone();
            let face_rect = egui::Rect::from_min_size(pos2(face_x, face_y), face_size);
            self.face.paint(&p, face_rect.translate(vec2(0.0, 2.0)), fade(Color32::from_black_alpha(80)));
            self.face.paint(&p, face_rect, fade(Color32::WHITE));
            let progress = left / duration;

            let bar = |rect: egui::Rect| {
                p.rect(rect, 4.0, fade(TRACK), Stroke::new(1.5, fade(INK)), egui::StrokeKind::Inside);
                let fill = egui::Rect::from_min_size(rect.min, vec2(rect.width() * progress, rect.height()));
                p.rect_filled(fill.shrink(1.5), 3.0, fade(ACCENT));
            };
            if !self.cfg.show_hint {
                let w = face_size.x * 0.7;
                bar(egui::Rect::from_min_size(pos2(face_x + (face_size.x - w) / 2.0, face_y + bob as f32 * scale + face_size.y + scale + 6.0), vec2(w, 6.0)));
                return;
            }

            let lang = &self.cfg.lang;
            let title = p.layout(tr(lang, &format!("ex.{}.title", ex.id), &[]), FontId::proportional(14.0), fade(INK), bubble_w - 20.0);
            let hint = p.layout(tr(lang, &format!("ex.{}.hint", ex.id), &[]), FontId::proportional(12.0), fade(INK), bubble_w - 20.0);
            let height = 8.0 + title.size().y + 3.0 + hint.size().y + 7.0 + 6.0 + 9.0;
            let bx = if flip { face_x - GAP - bubble_w } else { face_x + face_size.x + GAP };
            let rect = egui::Rect::from_min_size(pos2(bx, (self.size.y - height) / 2.0), vec2(bubble_w, height));

            p.rect_filled(rect.translate(vec2(3.0, 3.0)), 8.0, fade(Color32::from_rgba_unmultiplied(0x3b, 0x2a, 0x40, 77)));
            // Tail pointing at the face.
            let (tx, dir) = if flip { (rect.right(), 1.0) } else { (rect.left(), -1.0) };
            let cy = rect.center().y;
            let tail = vec![pos2(tx, cy - 7.0), pos2(tx + dir * 8.0, cy), pos2(tx, cy + 7.0)];
            p.add(egui::Shape::convex_polygon(tail, fade(PAPER), Stroke::new(2.0, fade(INK))));
            p.rect(rect, 8.0, fade(PAPER), Stroke::new(2.0, fade(INK)), egui::StrokeKind::Inside);
            // Cover the part of the border that crosses the tail's base.
            p.line_segment([pos2(tx, cy - 6.0), pos2(tx, cy + 6.0)], Stroke::new(3.0, fade(PAPER)));

            let (x0, mut y) = (rect.left() + 10.0, rect.top() + 8.0);
            p.galley(pos2(x0, y), title.clone(), fade(INK));
            y += title.size().y + 3.0;
            p.galley(pos2(x0, y), hint.clone(), fade(INK));
            y += hint.size().y + 7.0;
            let secs = tr(lang, "time.sec", &[&(left.ceil() as u32)]);
            let secs_w = 40.0;
            bar(egui::Rect::from_min_size(pos2(x0, y), vec2(rect.width() - 20.0 - secs_w - 8.0, 6.0)));
            p.text(pos2(rect.right() - 10.0, y + 3.0), Align2::RIGHT_CENTER, secs, FontId::monospace(11.0), fade(INK));
        });
        ctx.request_repaint_after(Duration::from_millis(33));
    }
}
