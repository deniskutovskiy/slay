use crate::app::SlayApp;
use crate::theme::*;
use eframe::egui;
use slay_core::get_palette_info;

pub fn render_palette(ui: &mut egui::Ui, app: &mut SlayApp) {
    ui.add_space(15.0);
    ui.heading("Palette");
    ui.add_space(10.0);

    for (kind, description, rgb) in get_palette_info() {
        let (rect, response) = ui.allocate_at_least(egui::vec2(ui.available_width(), 40.0), egui::Sense::drag());

        let color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        let is_hovered = ui.rect_contains_pointer(rect);
        let bg_color = if is_hovered { COLOR_BG } else { COLOR_PANEL };

        ui.painter().rect_filled(rect, 4.0, bg_color);
        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, color.gamma_multiply(0.5)));
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, &kind, egui::FontId::proportional(12.0), COLOR_TEXT);

        let response = response.on_hover_text(description);

        if response.drag_started() {
            app.drag_node_kind = Some(kind);
        }
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.add_space(15.0);
        if ui.button(egui::RichText::new("🗑 Clear Canvas").color(COLOR_CRITICAL)).clicked() {
            app.reset();
        }

        ui.add_space(10.0);
        let btn = if app.is_running {
            "⏸ Stop"
        } else {
            "▶ Start"
        };
        if ui.add_sized([ui.available_width(), 35.0], egui::Button::new(btn).fill(COLOR_ACCENT.gamma_multiply(0.2))).clicked()
        {
            app.is_running = !app.is_running;
        }

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);
        ui.label(egui::RichText::new("SIMULATION").small().color(COLOR_TEXT_DIM));

        ui.label("Time Speed:");
        ui.add(egui::Slider::new(&mut app.sim_speed, 0.01..=10.0).suffix("x").logarithmic(true));

        ui.add_space(10.0);
        ui.label("UI Refresh Rate:");
        ui.add(egui::Slider::new(&mut app.ui_refresh_rate, 0.0..=2.0).suffix("s"));
        ui.add_space(10.0);
    });
}
