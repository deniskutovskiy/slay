use crate::components::ComponentView;
use eframe::egui;
use serde_json::Value;

#[derive(Default)]
pub struct ClientView;

impl ComponentView for ClientView {
    fn name(&self) -> &'static str {
        "Client"
    }

    fn description(&self) -> &'static str {
        "External load source (RPS)"
    }

    fn color(&self) -> egui::Color32 {
        egui::Color32::from_rgb(163, 190, 140) // Green
    }

    fn render_canvas(&self, ui: &mut egui::Ui, rect: egui::Rect, snapshot: &Value, zoom: f32) {
        let painter = ui.painter();
        let f_m = egui::FontId::proportional(22.0 * zoom);
        let f_s = egui::FontId::proportional(9.0 * zoom);

        let rate = snapshot["rate"].as_f64().unwrap_or(0.0);

        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{:.1} λ", rate),
            f_m,
            egui::Color32::WHITE,
        );
        painter.text(
            rect.center() + egui::vec2(0., 20. * zoom),
            egui::Align2::CENTER_CENTER,
            "REQUESTS / S",
            f_s,
            egui::Color32::from_gray(180),
        );
    }

    fn render_inspector(&self, ui: &mut egui::Ui, config: &mut Value) -> bool {
        let mut changed = false;
        ui.label("Arrival Rate:");
        if let Some(rate) = config.get_mut("arrival_rate") {
            let mut val = rate.as_f64().unwrap_or(5.0) as f32;
            if ui
                .add(
                    egui::Slider::new(&mut val, 1.0..=500000.0)
                        .suffix(" RPS")
                        .logarithmic(true),
                )
                .changed()
            {
                *rate = Value::from(val);
                changed = true;
            }
        }

        ui.add_space(10.0);
        ui.label("Request Timeout:");
        if let Some(timeout) = config.get_mut("timeout") {
            let mut val = timeout.as_u64().unwrap_or(5000);
            if ui
                .add(egui::Slider::new(&mut val, 10..=30000).suffix("ms"))
                .changed()
            {
                *timeout = Value::from(val);
                changed = true;
            }
        }

        if changed {
            if let Some(gen_id) = config.get_mut("generation_id") {
                let current = gen_id.as_u64().unwrap_or(1);
                *gen_id = Value::from(current + 1);
            }
        }

        changed
    }
}
