use crate::components::ComponentView;
use eframe::egui;
use serde_json::Value;

pub struct LoadBalancerView;

impl ComponentView for LoadBalancerView {
    fn name(&self) -> &'static str {
        "LoadBalancer"
    }

    fn description(&self) -> &'static str {
        "Distributes traffic to backends"
    }

    fn color(&self) -> egui::Color32 {
        egui::Color32::from_rgb(136, 192, 208) // Cyan
    }

    fn render_canvas(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        snapshot: &slay_core::traits::VisualState,
        zoom: f32,
    ) {
        let painter = ui.painter();
        let f_xl = egui::FontId::proportional(22.0 * zoom);
        let f_s = egui::FontId::proportional(10.0 * zoom);
        let f_xs = egui::FontId::proportional(9.0 * zoom);

        if let slay_core::traits::VisualState::LoadBalancer(stats) = snapshot {
            let center = rect.center();

            // 1. Strategy Name
            let strategy_name = match stats.strategy.as_str() {
                "RoundRobin" => "RR",
                "Random" => "RND",
                "LeastConnections" => "L-CONN",
                s => s,
            };
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                strategy_name,
                f_xl,
                egui::Color32::WHITE,
            );

            // 2. RPS Badge
            let rps = stats.rps;
            if rps > 0.0 {
                painter.text(
                    rect.right_top() + egui::vec2(-8.0 * zoom, 25.0 * zoom),
                    egui::Align2::RIGHT_TOP,
                    format!("{:.0} RPS", rps),
                    f_s.clone(),
                    egui::Color32::from_rgb(0, 255, 150),
                );
            }

            // 2a. Retry Active Badge
            let active_retries = stats.active_retries;
            if active_retries > 0 {
                painter.text(
                    rect.left_top() + egui::vec2(8.0 * zoom, 25.0 * zoom),
                    egui::Align2::LEFT_TOP,
                    format!("↻ {}", active_retries),
                    f_s.clone(),
                    egui::Color32::from_rgb(255, 200, 0),
                );
            }

            // 2b. Failed Count Badge
            let failed = stats.failed_count;
            if failed > 0 {
                painter.text(
                    rect.left_top() + egui::vec2(8.0 * zoom, 40.0 * zoom),
                    egui::Align2::LEFT_TOP,
                    format!("x {}", failed),
                    f_s,
                    egui::Color32::from_rgb(255, 80, 80),
                );
            }

            // 3. Distribution Visualization (Using 'targets' as source of truth)
            let targets = &stats.targets;
            let num_targets = targets.len();
            if num_targets > 0 {
                let bar_width = (140.0 / num_targets as f32).min(20.0) * zoom;
                let spacing = 4.0 * zoom;
                let total_width = (bar_width + spacing) * num_targets as f32 - spacing;
                let start_x = rect.center().x - total_width / 2.0;
                let baseline_y = rect.bottom() - 18.0 * zoom;

                let loads = &stats.loads;

                for (i, target_id) in targets.iter().enumerate() {
                    let load = *loads.get(target_id).unwrap_or(&0);

                    let x = start_x + (i as f32 * (bar_width + spacing));
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(x, baseline_y),
                        egui::vec2(bar_width, 4.0 * zoom),
                    );

                    let col = if load > 0 {
                        egui::Color32::from_rgb(136, 192, 208)
                    } else {
                        egui::Color32::from_gray(50)
                    };
                    painter.rect_filled(bar_rect, 1.0 * zoom, col);

                    if zoom > 0.8 {
                        painter.text(
                            egui::pos2(x + bar_width / 2.0, baseline_y - 4.0 * zoom),
                            egui::Align2::CENTER_BOTTOM,
                            format!("#{}", target_id),
                            f_xs.clone(),
                            egui::Color32::from_gray(150),
                        );
                        if load > 0 {
                            painter.text(
                                egui::pos2(x + bar_width / 2.0, baseline_y + 6.0 * zoom),
                                egui::Align2::CENTER_TOP,
                                format!("{}", load),
                                f_xs.clone(),
                                egui::Color32::GOLD,
                            );
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
                    if ui
                        .selectable_value(&mut current, "RoundRobin".to_string(), "Round Robin")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(&mut current, "Random".to_string(), "Random")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(
                            &mut current,
                            "LeastConnections".to_string(),
                            "Least Connections",
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
            if changed {
                *strategy = Value::from(current);
            }
        }

        ui.add_space(10.0);
        ui.label(egui::RichText::new("RESILIENCE").small().strong());

        // Ensure defaults exist (auto-migration since strict compatibility is not required)
        if let Some(obj) = config.as_object_mut() {
            let max_retries = obj
                .entry("max_retries")
                .or_insert(serde_json::Value::from(2));
            if let Some(val_ref) = max_retries.as_u64() {
                let mut val = val_ref as u32;
                if ui
                    .add(egui::Slider::new(&mut val, 0..=5).text("Max Retries"))
                    .changed()
                {
                    *max_retries = serde_json::Value::from(val);
                    changed = true;
                }
            }

            let backoff = obj
                .entry("retry_backoff_ms")
                .or_insert(serde_json::Value::from(50));
            if let Some(val_ref) = backoff.as_u64() {
                let mut val = val_ref;
                if ui
                    .add(egui::Slider::new(&mut val, 0..=500).text("Backoff (ms)"))
                    .changed()
                {
                    *backoff = serde_json::Value::from(val);
                    changed = true;
                }
            }

            let budget = obj
                .entry("retry_budget_ratio")
                .or_insert(serde_json::Value::from(0.2));
            if let Some(val_ref) = budget.as_f64() {
                let mut val = val_ref;
                if ui
                    .add(egui::Slider::new(&mut val, 0.0..=1.0).text("Retry Budget %"))
                    .changed()
                {
                    *budget = serde_json::Value::from(val);
                    changed = true;
                }
            }
        }

        ui.add_space(10.0);
        ui.label(egui::RichText::new("DISTRIBUTION").small().strong());
        ui.label("Load Balancer tracks active requests to each backend.");

        changed
    }
}

impl Default for LoadBalancerView {
    fn default() -> Self {
        Self
    }
}
