//! The settings window: header with a live preview face, then the form.
use super::{ACCENT, MUTED, Sprite};
use crate::exercises::{self, EXERCISE_IDS, Lids, Pose, pose_at};
use crate::i18n::{format_clock, tr};
use crate::ipc::{self, FromChild, Info, StatusMsg, ToChild};
use crate::settings::*;
use crate::sprite::{FACE_H, FACE_W, draw_face};
use eframe::egui::{self, Align, Color32, DragValue, Layout, RichText, Ui, ViewportCommand, vec2};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

struct SettingsApp {
    rx: Receiver<ToChild>,
    settings: Settings,
    lang: String,
    info: Option<Info>,
    status: StatusMsg,
    sprite: Sprite,
    started: Instant,
    previewing: Option<(&'static str, Instant)>,
    title: String,
}

pub fn run() {
    let viewport = egui::ViewportBuilder::default()
        .with_title("TakeCareDev")
        .with_inner_size([640.0, 860.0])
        .with_min_inner_size([520.0, 480.0]);
    super::run("TakeCareDev", viewport, false, |ctx| {
        let repaint = ctx.clone();
        let rx = ipc::listen(move || repaint.request_repaint());
        ctx.set_visuals_of(egui::Theme::Light, visuals(egui::Visuals::light()));
        ctx.set_visuals_of(egui::Theme::Dark, visuals(egui::Visuals::dark()));
        Box::new(SettingsApp {
            rx,
            settings: Settings::default(),
            lang: "en".into(),
            info: None,
            status: StatusMsg::default(),
            sprite: Sprite::default(),
            started: Instant::now(),
            previewing: None,
            title: String::new(),
        })
    });
}

fn visuals(mut v: egui::Visuals) -> egui::Visuals {
    v.selection.bg_fill = ACCENT;
    v.widgets.hovered.bg_stroke.color = ACCENT;
    v
}

const POSITION_KEYS: [&str; 6] = POSITIONS;

impl SettingsApp {
    fn t(&self, key: &str) -> String {
        tr(&self.lang, key, &[])
    }

    fn drain(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                ToChild::Hello { settings, lang, info, status } => {
                    (self.settings, self.lang, self.info, self.status) = (settings, lang, Some(info), status);
                }
                ToChild::Status(s) => self.status = s,
                // Echoes of our own edits would fight a drag or text edit in progress.
                ToChild::Changed { settings, lang } => {
                    if !ctx.egui_is_using_pointer() && ctx.memory(|m| m.focused()).is_none() {
                        self.settings = settings;
                    }
                    self.lang = lang;
                }
                ToChild::Focus => ctx.send_viewport_cmd(ViewportCommand::Focus),
                ToChild::Break(_) => {}
            }
        }
    }

    fn header(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let duration = self.settings.eyes.duration_sec as f32;
        if let Some((_, at)) = self.previewing {
            if at.elapsed().as_secs_f32() > duration {
                self.previewing = None;
            }
        }
        let sec = self.started.elapsed().as_secs_f32();
        let pose = match self.previewing {
            Some((id, at)) => pose_at(exercises::by_id(id), at.elapsed().as_secs_f32(), duration),
            None => Pose {
                look_x: (sec * 0.7).sin() * 0.8,
                look_y: (sec * 0.45).sin() * 0.4,
                lids: if sec % 3.7 < 0.15 { Lids::Closed } else { Lids::Open },
                ..Pose::default()
            },
        };
        self.sprite.update(&ctx, pose.key().to_vec(), FACE_W, FACE_H, |r| draw_face(&mut |x, y, c| r.put(x, y, c), &pose, 0, 0));

        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(FACE_W as f32, FACE_H as f32) * 3.0, egui::Sense::hover());
            self.sprite.paint(ui.painter(), rect, Color32::WHITE);
            ui.vertical(|ui| {
                ui.label(RichText::new(self.t("appName")).size(20.0).strong());
                ui.label(RichText::new(self.status_line()).strong());
                let today = &self.status.today;
                ui.label(RichText::new(tr(&self.lang, "set.stats", &[&today.eyes, &today.walks, &today.skipped])).size(12.5).color(MUTED));
                if let Some((id, _)) = self.previewing {
                    let caption = format!("{}: {}", self.t(&format!("ex.{id}.title")), self.t(&format!("ex.{id}.hint")));
                    ui.label(RichText::new(caption).size(12.5).color(MUTED));
                }
            });
            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                ui.vertical(|ui| {
                    let paused = self.status.status.paused;
                    if ui.button(self.t(if paused { "set.btn.resume" } else { "set.btn.pause" })).clicked() {
                        if paused { command("resume", None) } else { command("pause", Some("30".into())) }
                    }
                    if ui.button(self.t("set.btn.reset")).clicked() {
                        command("reset", None);
                    }
                });
            });
        });
    }

    fn status_line(&self) -> String {
        let s = &self.status.status;
        if s.paused {
            return match s.paused_until {
                Some(until) => {
                    let at = chrono::DateTime::from_timestamp(until as i64, 0).map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string());
                    tr(&self.lang, "tray.pausedUntil", &[&at.unwrap_or_default()])
                }
                None => self.t("tray.paused"),
            };
        }
        let part = |left: Option<u64>, kind: &str| match left {
            None => self.t("set.statusOff"),
            Some(_) if s.busy.as_deref() == Some(kind) => self.t("set.statusBusy"),
            Some(l) => format_clock(l as f64),
        };
        format!(
            "{} · {}",
            tr(&self.lang, "set.statusEyes", &[&part(s.eye_left, "eye")]),
            tr(&self.lang, "set.statusWalk", &[&part(s.walk_left, "walk")])
        )
    }
}

/// A small play triangle, painted: the bundled fonts have no ▶ glyph.
fn play_button(ui: &mut Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(22.0, 18.0), egui::Sense::click());
    let visuals = ui.style().interact(&response);
    ui.painter().rect(rect, 4.0, visuals.weak_bg_fill, visuals.bg_stroke, egui::StrokeKind::Inside);
    let c = rect.center();
    let tri = vec![c + vec2(-3.0, -5.0), c + vec2(-3.0, 5.0), c + vec2(5.0, 0.0)];
    ui.painter().add(egui::Shape::convex_polygon(tri, visuals.fg_stroke.color, egui::Stroke::NONE));
    response
}

fn command(name: &str, arg: Option<String>) {
    ipc::say(&FromChild::Command { name: name.into(), arg });
}

/// One settings row: label (with an optional note) on the left, the control on the right.
fn row(ui: &mut Ui, label: &str, note: Option<String>, enabled: bool, control: impl FnOnce(&mut Ui)) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_max_width(ui.available_width() * 0.62);
            ui.label(label);
            if let Some(note) = note {
                ui.label(RichText::new(note).size(12.0).color(MUTED));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.add_enabled_ui(enabled, control));
    });
    ui.add_space(2.0);
    ui.separator();
}

fn number(ui: &mut Ui, value: &mut f64, limit: Limit) {
    ui.add(DragValue::new(value).range(limit.min..=limit.max).speed(limit.step).update_while_editing(false));
}

fn card(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    ui.add_space(10.0);
    ui.label(RichText::new(title).size(15.0).strong());
    egui::Frame::group(ui.style()).corner_radius(12.0).inner_margin(egui::Margin::symmetric(14, 4)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        body(ui);
    });
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.drain(&ctx);
        let Some(info) = self.info.clone() else { return };
        let title = self.t("set.title");
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        let before = self.settings.clone();
        let l = self.lang.clone();
        let t = |key: &str| tr(&l, key, &[]);

        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(6.0);
            self.header(ui);
            ui.add_space(6.0);
        });
        egui::Panel::bottom("footer").show(ui, |ui| {
            let wayland = if info.wayland { " (Wayland)" } else { "" };
            ui.label(RichText::new(format!("v{} · {}{wayland} · cursor: {} · idle: {}", info.version, info.platform, info.cursor_mode, info.idle_source)).size(11.5).color(MUTED));
        });

        let s = &mut self.settings;
        let mut preview = None;
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                card(ui, &t("set.section.eyes"), |ui| {
                    let e = &mut s.eyes;
                    row(ui, &t("set.enabled"), None, true, |ui| drop(ui.checkbox(&mut e.enabled, "")));
                    row(ui, &t("set.eyes.intervalMin"), None, true, |ui| number(ui, &mut e.interval_min, EYES_INTERVAL));
                    row(ui, &t("set.eyes.durationSec"), None, true, |ui| number(ui, &mut e.duration_sec, EYES_DURATION));
                    let note = (info.cursor_mode == "none").then(|| t("set.note.noCursor"));
                    row(ui, &t("set.eyes.position"), note, true, |ui| {
                        egui::ComboBox::from_id_salt("position").selected_text(t(&format!("set.pos.{}", e.position))).show_ui(ui, |ui| {
                            for p in POSITION_KEYS {
                                ui.selectable_value(&mut e.position, p.to_string(), t(&format!("set.pos.{p}")));
                            }
                        });
                    });
                    let note = (info.cursor_mode == "probe").then(|| t("set.note.waylandFollow"));
                    row(ui, &t("set.eyes.followCursor"), note, info.can_follow, |ui| drop(ui.checkbox(&mut e.follow_cursor, "")));
                    row(ui, &t("set.eyes.faceScale"), None, true, |ui| {
                        let mut v = e.face_scale;
                        if ui.add(egui::Slider::new(&mut v, EYES_SCALE.min..=EYES_SCALE.max).step_by(EYES_SCALE.step).max_decimals(0).prefix("×")).changed() {
                            e.face_scale = v;
                        }
                    });
                    row(ui, &t("set.eyes.showHint"), None, true, |ui| drop(ui.checkbox(&mut e.show_hint, "")));
                    row(ui, &t("set.eyes.order"), None, true, |ui| {
                        egui::ComboBox::from_id_salt("order").selected_text(t(&format!("set.order.{}", e.order))).show_ui(ui, |ui| {
                            for o in ORDERS {
                                ui.selectable_value(&mut e.order, o.to_string(), t(&format!("set.order.{o}")));
                            }
                        });
                    });
                    row(ui, &t("set.eyes.sound"), None, true, |ui| drop(ui.checkbox(&mut e.sound, "")));
                    ui.add_space(4.0);
                    ui.label(t("set.eyes.exercises"));
                    ui.label(RichText::new(t("set.eyes.exercisesNote")).size(12.0).color(MUTED));
                    for id in EXERCISE_IDS {
                        ui.horizontal(|ui| {
                            let mut on = e.exercises.iter().any(|x| x == id);
                            if ui.checkbox(&mut on, "").changed() {
                                let next: Vec<String> = EXERCISE_IDS.iter().filter(|x| if **x == id { on } else { e.exercises.iter().any(|y| y == *x) }).map(|x| x.to_string()).collect();
                                if !next.is_empty() {
                                    e.exercises = next;
                                }
                            }
                            if ui.link(t(&format!("ex.{id}.title"))).on_hover_text(t(&format!("ex.{id}.hint"))).clicked() {
                                preview = Some(id);
                            }
                            if play_button(ui).on_hover_text(t("set.btn.eyeNow")).clicked() {
                                command("eyeNow", Some(id.to_string()));
                            }
                        });
                    }
                    ui.add_space(6.0);
                    if ui.button(t("set.btn.eyeNow")).clicked() {
                        command("eyeNow", None);
                    }
                    ui.add_space(4.0);
                });
                card(ui, &t("set.section.walk"), |ui| {
                    let w = &mut s.walk;
                    row(ui, &t("set.enabled"), None, true, |ui| drop(ui.checkbox(&mut w.enabled, "")));
                    row(ui, &t("set.walk.intervalMin"), None, true, |ui| number(ui, &mut w.interval_min, WALK_INTERVAL));
                    row(ui, &t("set.walk.durationMin"), None, true, |ui| number(ui, &mut w.duration_min, WALK_DURATION));
                    row(ui, &t("set.walk.snoozeMin"), None, true, |ui| number(ui, &mut w.snooze_min, WALK_SNOOZE));
                    row(ui, &t("set.walk.warnSec"), None, true, |ui| number(ui, &mut w.warn_sec, WALK_WARN));
                    let note = (!info.idle_reliable).then(|| t("set.note.idleUnreliable"));
                    row(ui, &t("set.walk.requireIdle"), note, info.idle_reliable, |ui| drop(ui.checkbox(&mut w.require_idle, "")));
                    row(ui, &t("set.walk.strict"), None, true, |ui| drop(ui.checkbox(&mut w.strict, "")));
                    row(ui, &t("set.walk.sound"), None, true, |ui| drop(ui.checkbox(&mut w.sound, "")));
                    ui.add_space(6.0);
                    if ui.button(t("set.btn.walkNow")).clicked() {
                        command("walkNow", None);
                    }
                    ui.add_space(4.0);
                });
                card(ui, &t("set.section.general"), |ui| {
                    row(ui, &t("set.idle.resetAfterMin"), None, info.idle_reliable, |ui| number(ui, &mut s.idle.reset_after_min, IDLE_RESET));
                    row(ui, &t("set.language"), None, true, |ui| {
                        let shown = match s.language.as_str() {
                            "ru" => "Русский".to_string(),
                            "en" => "English".to_string(),
                            _ => t("set.lang.auto"),
                        };
                        egui::ComboBox::from_id_salt("language").selected_text(shown).show_ui(ui, |ui| {
                            ui.selectable_value(&mut s.language, "auto".into(), t("set.lang.auto"));
                            ui.selectable_value(&mut s.language, "ru".into(), "Русский");
                            ui.selectable_value(&mut s.language, "en".into(), "English");
                        });
                    });
                    row(ui, &t("set.autostart"), None, true, |ui| drop(ui.checkbox(&mut s.autostart, "")));
                });
            });
        });

        if let Some(id) = preview {
            self.previewing = Some((id, Instant::now()));
        }
        if self.settings != before {
            ipc::say(&FromChild::Update(self.settings.clone()));
        }
        ctx.request_repaint_after(Duration::from_millis(50));
    }
}
