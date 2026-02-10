use eframe::egui;
use serde_json::Value;
use crate::components::ComponentView;

#[derive(Default)]
pub struct LoadBalancerView;

impl ComponentView for LoadBalancerView {
    fn render_canvas(&self, ui: &mut egui::Ui, rect: egui::Rect, snapshot: &Value, zoom: f32) {
        let painter = ui.painter();
        let f_xl = egui::FontId::proportional(22.0 * zoom);
        let f_s = egui::FontId::proportional(10.0 * zoom);
        let f_xs = egui::FontId::proportional(9.0 * zoom);
        
        let center = rect.center();

        // 1. Strategy Abbreviation - Back to short form
        let strategy_name = match snapshot["strategy"].as_str().unwrap_or("RoundRobin") {
            "RoundRobin" => "RR",
            "Random" => "RND",
            "LeastConnections" => "L-CONN",
            s => s,
        };
        painter.text(center, egui::Align2::CENTER_CENTER, strategy_name, f_xl, egui::Color32::WHITE);

        // 2. RPS Badge
        let rps = snapshot["rps"].as_f64().unwrap_or(0.0);
        if rps > 0.0 {
            painter.text(rect.right_top() + egui::vec2(-8.0 * zoom, 25.0 * zoom), egui::Align2::RIGHT_TOP, format!("{:.0} RPS", rps), f_s, egui::Color32::from_rgb(0, 255, 150));
        }

        // 3. Distribution Visualization (Bottom)
        if let Some(loads) = snapshot["loads"].as_object() {
            let num_targets = loads.len();
            if num_targets > 0 {
                let bar_width = (140.0 / num_targets as f32).min(25.0) * zoom;
                let spacing = 4.0 * zoom;
                let total_width = (bar_width + spacing) * num_targets as f32 - spacing;
                let start_x = rect.center().x - total_width / 2.0;
                
                for (i, (target_id, load_val)) in loads.iter().enumerate() {
                    let load = load_val.as_u64().unwrap_or(0);
                    let x = start_x + (i as f32 * (bar_width + spacing));
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(x, rect.bottom() - 12.0 * zoom),
                        egui::vec2(bar_width, 4.0 * zoom)
                    );
                    
                    let col = if load > 0 { egui::Color32::from_rgb(136, 192, 208) } else { egui::Color32::from_gray(50) };
                    painter.rect_filled(bar_rect, 1.0 * zoom, col);
                    
                    if zoom > 0.8 {
                        painter.text(egui::pos2(x + bar_width/2.0, rect.bottom() - 18.0 * zoom), egui::Align2::CENTER_BOTTOM, format!("#{}", target_id), f_xs.clone(), egui::Color32::from_gray(150));
                        if load > 0 {
                            painter.text(egui::pos2(x + bar_width/2.0, rect.bottom() - 4.0 * zoom), egui::Align2::CENTER_TOP, format!("{}", load), f_xs.clone(), egui::Color32::GOLD);
                        }
                    }
                }
            }
        }
    }

    fn render_inspector(&self, ui: &mut egui::Ui, config: &mut Value) -> bool {
        let mut changed = false;
        ui.label(egui::RichText::new("STRATEGY").small().strong());
        
        if let Some(strategy) = config.get_mut("strategy") {
            let mut current = strategy.as_str().unwrap_or("RoundRobin").to_string();
            egui::ComboBox::from_id_salt("lb_strategy")
                .selected_text(&current)
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut current, "RoundRobin".to_string(), "Round Robin").changed() { changed = true; }
                    if ui.selectable_value(&mut current, "Random".to_string(), "Random").changed() { changed = true; }
                    if ui.selectable_value(&mut current, "LeastConnections".to_string(), "Least Connections").changed() { changed = true; }
                });
            if changed { *strategy = Value::from(current); }
        }
        
        ui.add_space(10.0);
        ui.label(egui::RichText::new("DISTRIBUTION").small().strong());
        ui.label("Load Balancer tracks active requests to each backend.");
        
        changed
    }
}
