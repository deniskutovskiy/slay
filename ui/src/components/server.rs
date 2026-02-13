use crate::components::ComponentView;
use eframe::egui;
use serde_json::Value;

#[derive(Default)]
pub struct ServerView;

impl ComponentView for ServerView {
    fn render_canvas(&self, ui: &mut egui::Ui, rect: egui::Rect, snapshot: &Value, zoom: f32) {
        let painter = ui.painter();
        let f_m = egui::FontId::proportional(22.0 * zoom);
        let f_s = egui::FontId::proportional(9.0 * zoom);
        let f_xs = egui::FontId::proportional(11.0 * zoom);

        let rps = snapshot["rps"].as_f64().unwrap_or(0.0);
        let threads = snapshot["threads"].as_u64().unwrap_or(0) as u32;
        let concurrency = snapshot["concurrency"].as_u64().unwrap_or(1) as u32;
        let queue = snapshot["queue_len"].as_u64().unwrap_or(0);

        if rps > 0.0 {
            painter.text(
                rect.right_top() + egui::vec2(-10.0 * zoom, 15.0 * zoom),
                egui::Align2::RIGHT_TOP,
                format!("{:.0} RPS", rps),
                f_xs,
                egui::Color32::from_rgb(0, 255, 150),
            );
        }

        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{} / {}", threads, concurrency),
            f_m,
            egui::Color32::WHITE,
        );
        painter.text(
            rect.center() + egui::vec2(0., 20. * zoom),
            egui::Align2::CENTER_CENTER,
            "BUSY THREADS",
            f_s,
            egui::Color32::from_gray(180),
        );

        let grid_width = (concurrency as f32 * 10.0).min(160.0);
        let start_x = rect.center().x - (grid_width / 2.0) * zoom;
        for i in 0..concurrency {
            let tr = egui::Rect::from_min_size(
                egui::pos2(start_x + (i as f32 * 10.0 * zoom), rect.top() + 75.0 * zoom),
                egui::vec2(7.0, 8.0) * zoom,
            );
            let col = if i < threads {
                egui::Color32::from_rgb(0, 255, 255)
            } else {
                egui::Color32::from_gray(40)
            };
            painter.rect_filled(tr, 1.0 * zoom, col);
        }

        if queue > 0 {
            painter.text(
                rect.right_top() + egui::vec2(-10.0 * zoom, 35.0 * zoom),
                egui::Align2::RIGHT_TOP,
                format!("Q: {}", queue),
                egui::FontId::proportional(12.0 * zoom),
                egui::Color32::GOLD,
            );
        }
    }

    fn render_inspector(&self, ui: &mut egui::Ui, config: &mut Value) -> bool {
        let mut changed = false;
        ui.label(egui::RichText::new("PERFORMANCE").small().strong());

        if let Some(service_time) = config.get_mut("service_time") {
            let mut val = service_time.as_u64().unwrap_or(200);
            if ui
                .add(egui::Slider::new(&mut val, 1..=2000).suffix("ms"))
                .changed()
            {
                *service_time = Value::from(val);
                changed = true;
            }
        }

        if let Some(concurrency) = config.get_mut("concurrency") {
            let mut val = concurrency.as_u64().unwrap_or(4);
            if ui
                .add(egui::Slider::new(&mut val, 1..=64).suffix(" threads"))
                .changed()
            {
                *concurrency = Value::from(val);
                changed = true;
            }
        }

        ui.add_space(10.0);
        ui.label(egui::RichText::new("QUEUEING").small().strong());
        if let Some(backlog) = config.get_mut("backlog_limit") {
            let mut val = backlog.as_u64().unwrap_or(50);
            if ui
                .add(egui::Slider::new(&mut val, 0..=500).suffix(" reqs"))
                .changed()
            {
                *backlog = Value::from(val);
                changed = true;
            }
        }

        changed
    }
}
