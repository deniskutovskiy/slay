use crate::components::ComponentView;
use eframe::egui;
use serde_json::Value;

#[derive(Default)]
pub struct ServerView;

impl ComponentView for ServerView {
    fn name(&self) -> &'static str {
        "Server"
    }

    fn description(&self) -> &'static str {
        "Application logic & queues"
    }

    fn color(&self) -> egui::Color32 {
        egui::Color32::from_rgb(129, 161, 193) // Blue
    }

    fn render_canvas(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        snapshot: &slay_core::traits::VisualState,
        zoom: f32,
    ) {
        let painter = ui.painter();
        let f_m = egui::FontId::proportional(22.0 * zoom);
        let f_s = egui::FontId::proportional(9.0 * zoom);
        let f_xs = egui::FontId::proportional(11.0 * zoom);

        if let slay_core::traits::VisualState::Server(stats) = snapshot {
            let rps = stats.rps;
            let threads = stats.threads;
            let concurrency = stats.concurrency;
            let queue = stats.queue_len;

            // Calculate load factor for visualization
            let load_factor = if concurrency > 0 {
                threads as f32 / concurrency as f32
            } else {
                0.0
            };

            let mut text_color = egui::Color32::WHITE;
            if load_factor > 0.9 {
                text_color = egui::Color32::from_rgb(255, 100, 100); // Red
            } else if load_factor > 0.7 {
                text_color = egui::Color32::from_rgb(255, 200, 100); // Yellow/Orange
            }

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
                text_color,
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

        ui.add_space(10.0);
        ui.label(egui::RichText::new("SIMULATION").small().strong());
        if let Some(fail_prob) = config.get_mut("failure_probability") {
            let mut val = fail_prob.as_f64().unwrap_or(0.0) as f32;
            if ui
                .add(
                    egui::Slider::new(&mut val, 0.0..=1.0)
                        .show_value(true)
                        .text("Failure %"),
                )
                .changed()
            {
                *fail_prob = Value::from(val);
                changed = true;
            }
        }

        if let Some(sat_penalty) = config.get_mut("saturation_penalty") {
            let mut val = sat_penalty.as_f64().unwrap_or(0.5) as f32;
            if ui
                .add(
                    egui::Slider::new(&mut val, 0.0..=5.0)
                        .show_value(true)
                        .text("Saturation Penalty"),
                )
                .on_hover_text("Slowdown factor at max concurrency")
                .changed()
            {
                *sat_penalty = Value::from(val);
                changed = true;
            }
        }

        changed
    }
}
