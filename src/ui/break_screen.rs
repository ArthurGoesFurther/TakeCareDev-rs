//! The walking-break card (or a full-screen cover per monitor in strict mode).
use super::{ACCENT, INK, MUTED, PAPER, Sprite, TRACK, pixel_button};
use crate::exercises::{Brows, Lids, Mouth, Pose};
use crate::i18n::{format_clock, tip, tr};
use crate::ipc::{self, BreakView, FromChild, ToChild};
use crate::sound;
use crate::sprite::{FACE_W, WALKER_H, draw_face, draw_walker};
use eframe::egui::{self, Color32, FontId, RichText, Stroke, ViewportCommand, pos2, vec2};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

const SCALE: f32 = 6.0;
const CARD_W: f32 = 440.0;
/// Clicks right after the window pops up are most likely meant for something else.
const ARM_DELAY: Duration = Duration::from_millis(1200);

struct BreakApp {
    rx: Receiver<ToChild>,
    state: Option<BreakView>,
    born: Instant,
    strict: bool,
    sprite: Sprite,
    placed: bool,
    bounds: Option<[f32; 4]>,
    card_h: f32,
    buttons_w: f32,
}

pub fn run(args: &[String]) {
    let strict = args.iter().any(|a| a == "--strict");
    let bounds = args
        .iter()
        .position(|a| a == "--bounds")
        .and_then(|i| args.get(i + 1))
        .and_then(|b| b.split(',').map(|v| v.parse::<f32>().ok()).collect::<Option<Vec<_>>>())
        .and_then(|v| <[f32; 4]>::try_from(v).ok());
    let size = bounds.map_or(vec2(480.0, 560.0), |b| vec2(b[2], b[3]));
    #[allow(unused_mut)]
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("TakeCareDev")
        .with_decorations(false)
        .with_transparent(!strict)
        .with_taskbar(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_active(strict)
        .with_inner_size(size)
        .with_position(bounds.map_or(pos2(0.0, 0.0), |b| pos2(b[0], b[1])));
    #[cfg(target_os = "linux")]
    {
        // Strict covers must sit exactly on the monitor, above panels: only an override-redirect window does.
        viewport = if strict { viewport.with_override_redirect(true) } else { viewport.with_window_type(egui::viewport::X11WindowType::Dialog) };
    }
    super::run("TakeCareDev", viewport, true, move |ctx| {
        let repaint = ctx.clone();
        let rx = ipc::listen(move || repaint.request_repaint());
        Box::new(BreakApp { rx, state: None, born: Instant::now(), strict, sprite: Sprite::default(), placed: false, bounds, card_h: 0.0, buttons_w: 0.0 })
    });
}

impl eframe::App for BreakApp {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        if self.strict { [0x17 as f32 / 255.0, 0x12 as f32 / 255.0, 0x1f as f32 / 255.0, 1.0] } else { [0.0; 4] }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.placed {
            self.placed = true;
            // The builder position assumed 1 px per point; correct it once the real scale is known.
            if let Some([x, y, w, h]) = self.bounds {
                let ppp = ctx.pixels_per_point();
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos2(x / ppp, y / ppp)));
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(w / ppp, h / ppp)));
            } else if self.strict {
                // Monitor layout unknown here (macOS): cover the monitor the window opened on.
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true));
            } else if let Some(center) = ViewportCommand::center_on_screen(&ctx) {
                ctx.send_viewport_cmd(center);
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.strict {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        }
        while let Ok(msg) = self.rx.try_recv() {
            if let ToChild::Break(next) = msg {
                let finished = self.state.as_ref().is_some_and(|s| !s.done) && next.done;
                if next.sound && (self.state.is_none() || finished) {
                    sound::chirp(if finished { &[523.0, 659.0, 784.0, 1047.0] } else { &[784.0, 523.0] });
                }
                self.state = Some(next);
            }
        }

        let armed = self.born.elapsed() > ARM_DELAY;
        let sec = self.born.elapsed().as_secs_f32();
        let (waiting, done) = self.state.as_ref().map_or((true, false), |s| (s.waiting, s.done));
        let look = ((sec * 1.3).sin() * 1.2).round();
        let step = (sec / 0.32).floor() as i32 % 2;
        let mode = if done { 2 } else if waiting { 0 } else { 1 };
        let key = if mode == 1 { vec![1, step, look as i32] } else { vec![mode] };
        self.sprite.update(&ctx, key, FACE_W, WALKER_H, |r| {
            let mut put = |x, y, c| r.put(x, y, c);
            match mode {
                1 => draw_walker(&mut put, sec, &Pose { look_x: look / 2.0, ..Pose::default() }),
                2 => draw_face(&mut put, &Pose { lids: Lids::Relaxed, mouth: Mouth::Open, ..Pose::default() }, 0, 2),
                _ => draw_face(&mut put, &Pose { look_y: -0.5, mouth: Mouth::O, brows: Brows::Raised, ..Pose::default() }, 0, 2),
            }
        });

        let mut action = None;
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            let Some(s) = &self.state else { return };
            let lang = s.lang.as_str();
            ui.add_space(((ui.available_height() - self.card_h) / 2.0).max(0.0));
            ui.vertical_centered(|ui| {
                let frame = egui::Frame::NONE
                    .fill(PAPER)
                    .stroke(Stroke::new(3.0, INK))
                    .corner_radius(14.0)
                    .inner_margin(egui::Margin { left: 23, right: 23, top: 17, bottom: 15 })
                    .shadow(egui::Shadow { offset: [5, 5], blur: 0, spread: 0, color: Color32::from_rgba_unmultiplied(0x3b, 0x2a, 0x40, 90) });
                let out = frame.show(ui, |ui| {
                    ui.set_width(CARD_W - 52.0);
                    ui.visuals_mut().override_text_color = Some(INK);
                    let (rect, _) = ui.allocate_exact_size(vec2(FACE_W as f32, WALKER_H as f32) * SCALE, egui::Sense::hover());
                    self.sprite.paint(ui.painter(), rect, Color32::WHITE);
                    ui.add_space(4.0);
                    ui.label(RichText::new(tr(lang, if s.done { "break.doneTitle" } else { "break.title" }, &[])).size(24.0).strong().color(INK));
                    ui.add_space(6.0);
                    let text = if s.done { tr(lang, "break.doneText", &[]) } else { tip(lang, s.tip).to_string() };
                    ui.allocate_ui(vec2(ui.available_width(), 38.0), |ui| ui.vertical_centered(|ui| ui.label(RichText::new(text).size(15.0))));
                    ui.add_space(8.0);
                    let color = if s.done { Color32::from_rgb(0x2f, 0x9e, 0x6e) } else { INK };
                    ui.label(RichText::new(format_clock(s.remaining)).font(FontId::monospace(44.0)).color(color));
                    ui.add_space(8.0);
                    let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 10.0), egui::Sense::hover());
                    let p = ui.painter();
                    p.rect(bar, 6.0, TRACK, Stroke::new(2.0, INK), egui::StrokeKind::Inside);
                    let fill = egui::Rect::from_min_size(bar.min, vec2(bar.width() * (s.remaining / s.total.max(1.0)) as f32, bar.height()));
                    p.rect_filled(fill.shrink(2.0), 4.0, ACCENT);
                    ui.add_space(8.0);
                    let (status_key, status_color) = if !s.require_idle {
                        ("break.status.plain", MUTED)
                    } else if s.waiting {
                        ("break.status.waiting", Color32::from_rgb(0xc2, 0x40, 0x5a))
                    } else {
                        ("break.status.running", MUTED)
                    };
                    let status = if s.done { String::new() } else { tr(lang, status_key, &[]) };
                    ui.label(RichText::new(status).size(13.0).color(status_color));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let snooze = tr(lang, "break.snooze", &[&tr(lang, "time.min", &[&s.snooze_min])]);
                        let buttons: Vec<(&str, String, bool)> = if s.done {
                            vec![("close", tr(lang, "break.close", &[]), true)]
                        } else if s.strict {
                            vec![]
                        } else {
                            vec![("snooze", snooze, false), ("skip", tr(lang, "break.skip", &[]), false)]
                        };
                        // Centering needs the row's width, which is only known after it was laid out once.
                        ui.add_space(((ui.available_width() - self.buttons_w) / 2.0).max(0.0));
                        let x0 = ui.cursor().left();
                        for (name, label, primary) in buttons {
                            if pixel_button(ui, &label, primary).clicked() && armed {
                                action = Some(name);
                            }
                        }
                        self.buttons_w = ui.min_rect().right() - x0;
                    });
                    if s.strict && !s.done {
                        ui.add_space(10.0);
                        ui.label(RichText::new(tr(lang, "break.strictNote", &[])).size(12.0).color(MUTED));
                    }
                });
                self.card_h = out.response.rect.height();
            });
        });
        if let Some(name) = action {
            ipc::say(&FromChild::BreakAction(name.into()));
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
