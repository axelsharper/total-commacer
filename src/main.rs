#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod commands;
mod config;
mod dialogs;
mod fs;
mod icons;
mod lister;
mod ops;
mod panel;
mod xp;

use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let cfg = config::Config::load();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("total-commacer")
            .with_decorations(false)
            .with_inner_size([cfg.win_w.max(640.0), cfg.win_h.max(400.0)])
            .with_min_inner_size([640.0, 400.0])
            .with_maximized(cfg.maximized),
        ..Default::default()
    };
    eframe::run_native(
        "total-commacer",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(app::App::new(cc)))
        }),
    )
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("tahoma".into(), Arc::new(egui::FontData::from_static(include_bytes!("../assets/fonts/tahoma.ttf"))));
    fonts.font_data.insert("tahomabd".into(), Arc::new(egui::FontData::from_static(include_bytes!("../assets/fonts/tahomabd.ttf"))));
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "tahoma".into());
    fonts.families.insert(egui::FontFamily::Name("bold".into()), vec!["tahomabd".into(), "tahoma".into()]);
    ctx.set_fonts(fonts);
    // egui's own Ctrl+Q (quit) and Ctrl+plus/minus (zoom) would shadow TC's keys
    ctx.options_mut(|o| {
        o.quit_shortcuts.clear();
        o.zoom_with_keyboard = false;
    });
    ctx.all_styles_mut(|style| {
        style.text_styles.insert(egui::TextStyle::Body, egui::FontId::new(11.0, egui::FontFamily::Proportional));
        style.text_styles.insert(egui::TextStyle::Button, egui::FontId::new(11.0, egui::FontFamily::Proportional));
        style.text_styles.insert(egui::TextStyle::Monospace, egui::FontId::new(12.0, egui::FontFamily::Monospace));
        style.visuals.override_text_color = Some(xp::TEXT);
        style.visuals.selection.bg_fill = xp::HIGHLIGHT;
        style.visuals.selection.stroke = egui::Stroke::new(1.0, xp::HIGHLIGHT_TEXT);
        style.visuals.text_cursor.stroke = egui::Stroke::new(1.0, xp::TEXT);
        style.visuals.window_fill = xp::FACE;
        style.visuals.panel_fill = xp::FACE;
        style.visuals.extreme_bg_color = xp::WINDOW;
        style.spacing.item_spacing = egui::vec2(0.0, 0.0);
        style.interaction.tooltip_delay = 0.5;
    });
    // XP had no text anti-aliasing tricks to speak of; keep egui's but at 1:1 pixels.
    ctx.set_pixels_per_point(ctx.pixels_per_point().max(1.0));
}
