//! Window processes: each one is a small eframe app started by the daemon.
pub mod break_screen;
pub mod face;
pub mod settings;

use crate::sprite::Raster;
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};

pub const INK: Color32 = Color32::from_rgb(0x3b, 0x2a, 0x40);
pub const PAPER: Color32 = Color32::from_rgb(0xff, 0xfd, 0xf6);
pub const ACCENT: Color32 = Color32::from_rgb(0x7b, 0x6c, 0xf6);
pub const TRACK: Color32 = Color32::from_rgb(0xef, 0xe6, 0xd6);
pub const MUTED: Color32 = Color32::from_rgb(0x6f, 0x63, 0x78);

pub fn icon() -> egui::IconData {
    let r = crate::sprite::app_icon(8);
    egui::IconData { rgba: r.data, width: r.width as u32, height: r.height as u32 }
}

/// Runs `app` as the only window of this process; the process exits when it closes.
/// `overlay` windows (face, break) stay out of the macOS Dock and app switcher.
#[allow(unused_variables)]
pub fn run(title: &str, viewport: egui::ViewportBuilder, overlay: bool, app: impl FnOnce(&egui::Context) -> Box<dyn eframe::App> + 'static) {
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions { viewport: viewport.with_icon(icon()), ..Default::default() };
    #[cfg(target_os = "macos")]
    if overlay {
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        options.event_loop_builder = Some(Box::new(|builder| {
            builder.with_activation_policy(ActivationPolicy::Accessory);
        }));
    }
    let result = eframe::run_native(title, options, Box::new(|cc| Ok(app(&cc.egui_ctx))));
    if let Err(e) = result {
        eprintln!("[tcdev] window failed: {e}");
        std::process::exit(1);
    }
}

/// A sprite uploaded as a texture; redrawn only when its `key` changes.
#[derive(Default)]
pub struct Sprite {
    tex: Option<TextureHandle>,
    key: Option<Vec<i32>>,
}

impl Sprite {
    pub fn update(&mut self, ctx: &egui::Context, key: Vec<i32>, w: i32, h: i32, draw: impl FnOnce(&mut Raster)) {
        if self.key.as_ref() == Some(&key) {
            return;
        }
        let mut raster = Raster::new(w, h, 1);
        draw(&mut raster);
        let image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &raster.data);
        match &mut self.tex {
            Some(tex) => tex.set(image, TextureOptions::NEAREST),
            None => self.tex = Some(ctx.load_texture("sprite", image, TextureOptions::NEAREST)),
        }
        self.key = Some(key);
    }

    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, tint: Color32) {
        if let Some(tex) = &self.tex {
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            painter.image(tex.id(), rect, uv, tint);
        }
    }
}

/// Retro "button with a hard shadow" look shared by the break screen.
pub fn pixel_button(ui: &mut egui::Ui, text: &str, primary: bool) -> egui::Response {
    let color = if primary { Color32::WHITE } else { INK };
    let button = egui::Button::new(egui::RichText::new(text).color(color).size(14.0).strong())
        .fill(if primary { ACCENT } else { Color32::WHITE })
        .stroke(egui::Stroke::new(2.0, INK))
        .corner_radius(8.0)
        .min_size(egui::vec2(0.0, 34.0));
    ui.add(button)
}
