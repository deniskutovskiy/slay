use eframe::egui;
use slay_core::NodeId;
use crate::app::SlayApp;
use crate::theme::*;
use crate::components::{get_view, ComponentView};

impl SlayApp {
    fn world_to_screen(&self, pos: egui::Pos2) -> egui::Pos2 {
        egui::pos2(pos.x * self.zoom + self.pan.x, pos.y * self.zoom + self.pan.y)
    }

    fn screen_to_world(&self, pos: egui::Pos2) -> egui::Pos2 {
        egui::pos2((pos.x - self.pan.x) / self.zoom, (pos.y - self.pan.y) / self.zoom)
    }

    pub fn render_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let mouse_pos = ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::pos2(0., 0.)));
        let canvas_rect = ui.max_rect();

        if ui.ui_contains_pointer() && ctx.input(|i| i.pointer.button_down(egui::PointerButton::Secondary)) {
            self.pan += ctx.input(|i| i.pointer.delta());
        }
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            let old_zoom = self.zoom;
            self.zoom *= (scroll_delta * 0.001).exp();
            self.zoom = self.zoom.clamp(0.1, 5.0);
            let world_mouse = egui::pos2((mouse_pos.x - self.pan.x) / old_zoom, (mouse_pos.y - self.pan.y) / old_zoom);
            self.pan = mouse_pos.to_vec2() - world_mouse.to_vec2() * self.zoom;
        }

        let grid_size = 50.0 * self.zoom;
        let grid_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(35));
        let start_x_idx = ((-self.pan.x) / grid_size).floor() as i32;
        let end_x_idx = ((-self.pan.x + canvas_rect.width()) / grid_size).ceil() as i32;
        for i in start_x_idx..=end_x_idx {
            let x = self.pan.x + i as f32 * grid_size;
            ui.painter().line_segment([egui::pos2(canvas_rect.left() + x, canvas_rect.top()), egui::pos2(canvas_rect.left() + x, canvas_rect.bottom())], grid_stroke);
        }
        let start_y_idx = ((-self.pan.y) / grid_size).floor() as i32;
        let end_y_idx = ((-self.pan.y + canvas_rect.height()) / grid_size).ceil() as i32;
        for i in start_y_idx..=end_y_idx {
            let y = self.pan.y + i as f32 * grid_size;
            ui.painter().line_segment([egui::pos2(canvas_rect.left(), canvas_rect.top() + y), egui::pos2(canvas_rect.right(), canvas_rect.top() + y)], grid_stroke);
        }

        for (id, comp) in &self.simulation.components {
            let throughput = comp.active_throughput();
            for target_id in comp.get_targets() {
                if let (Some(source_state), Some(target_state)) = (self.node_states.get(id), self.node_states.get(&target_id)) {
                    let p1 = self.world_to_screen(egui::pos2(source_state.pos.x + 180., source_state.pos.y + 45.));
                    let p2 = self.world_to_screen(egui::pos2(target_state.pos.x, target_state.pos.y + 45.));
                    let cp_offset = (p2.x - p1.x).abs().max(40.0 * self.zoom) * 0.5;
                    let points = [p1, p1 + egui::vec2(cp_offset, 0.), p2 - egui::vec2(cp_offset, 0.), p2];
                    ui.painter().add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
                        points, closed: false, fill: egui::Color32::TRANSPARENT, stroke: egui::Stroke::new(1.5 * self.zoom, egui::Color32::from_gray(100)).into(),
                    }));
                    if throughput > 0.0 {
                        let num_dots = (throughput / 5.0).clamp(1.0, 5.0) as i32;
                        let current_time = ctx.input(|i| i.time);
                        for i in 0..num_dots {
                            let t = (current_time as f32 * 0.5 + (i as f32 / num_dots as f32)) % 1.0;
                            let pos = self.sample_bezier(points, t);
                            ui.painter().circle_filled(pos, 3.0 * self.zoom, egui::Color32::from_rgb(0, 255, 255));
                        }
                    }
                }
            }
        }

        let mut node_ids: Vec<NodeId> = self.node_states.keys().cloned().collect();
        node_ids.sort();
        let mut pending_movements = Vec::new();

        for id in node_ids {
            let comp = if let Some(c) = self.simulation.components.get(&id) { c } else { continue };
            let visual_state = self.node_states.get(&id).unwrap();
            let screen_pos = self.world_to_screen(egui::pos2(visual_state.pos.x, visual_state.pos.y));
            let node_rect = egui::Rect::from_min_size(screen_pos, egui::vec2(180.0, 90.0) * self.zoom);

            let interaction = ui.interact(node_rect, egui::Id::new(id), egui::Sense::click_and_drag());
            if interaction.clicked() { self.selected_node = Some(id); }
            if interaction.dragged() { pending_movements.push((id, interaction.drag_delta() / self.zoom)); }

            // NEW: Distinct color for each node kind
            let rgb = comp.palette_color_rgb();
            let base_color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            
            // Draw Node Box tinted with base color
            ui.painter().rect_filled(node_rect, 6.0 * self.zoom, base_color.gamma_multiply(0.1));
            let border_color = if self.selected_node == Some(id) { COLOR_WARN } else if self.linking_from == Some(id) { COLOR_ACCENT } else { base_color.gamma_multiply(0.5) };
            ui.painter().rect_stroke(node_rect, 6.0 * self.zoom, egui::Stroke::new(1.5 * self.zoom, border_color));
            
            ui.painter().text(node_rect.left_top() + egui::vec2(10., 15.) * self.zoom, egui::Align2::LEFT_TOP, comp.name().to_uppercase(), egui::FontId::proportional(11.0 * self.zoom), COLOR_TEXT_DIM);

            if let Some(view) = get_view(comp.kind()) {
                let snapshot = comp.get_visual_snapshot();
                let v: &dyn ComponentView = view.as_ref();
                v.render_canvas(ui, node_rect, &snapshot, self.zoom);
            }

            let errors = comp.error_count();
            if errors > 0 { 
                ui.painter().text(node_rect.right_bottom() - egui::vec2(10., 10.) * self.zoom, egui::Align2::RIGHT_BOTTOM, format!("! {} ERR", errors), egui::FontId::proportional(11.0 * self.zoom), COLOR_CRITICAL); 
            }

            let input_port_pos = screen_pos + egui::vec2(0., 45.) * self.zoom;
            let output_port_pos = screen_pos + egui::vec2(180., 45.) * self.zoom;
            ui.painter().circle_filled(input_port_pos, 4.5 * self.zoom, egui::Color32::from_gray(80));
            ui.painter().circle_filled(output_port_pos, 4.5 * self.zoom, border_color);

            let port_interaction_radius = 25.0 * self.zoom;
            let input_rect = egui::Rect::from_center_size(input_port_pos, egui::vec2(port_interaction_radius, port_interaction_radius));
            if ui.rect_contains_pointer(input_rect) && ctx.input(|i| i.pointer.any_released()) {
                if let Some(source_id) = self.linking_from {
                    if source_id != id { 
                        if let Some(source_comp) = self.simulation.components.get_mut(&source_id) { source_comp.add_target(id); } 
                    }
                }
            }
            let output_rect = egui::Rect::from_center_size(output_port_pos, egui::vec2(port_interaction_radius, port_interaction_radius));
            let output_resp = ui.interact(output_rect, egui::Id::new(("out", id)), egui::Sense::drag());
            if output_resp.drag_started() { self.linking_from = Some(id); }
        }

        for (id, delta) in pending_movements { 
            let state = self.node_states.get_mut(&id).unwrap(); 
            state.pos.x += delta.x; state.pos.y += delta.y; 
        }

        if let Some(source_id) = self.linking_from {
            if let Some(source_state) = self.node_states.get(&source_id) {
                let start_p = self.world_to_screen(egui::pos2(source_state.pos.x + 180., source_state.pos.y + 45.));
                ui.painter().line_segment([start_p, mouse_pos], egui::Stroke::new(1.5 * self.zoom, COLOR_ACCENT));
            }
        }
        if ctx.input(|i| i.pointer.any_released()) { self.linking_from = None; }

        if let Some(kind) = &self.drag_node_kind {
            let ghost_rect = egui::Rect::from_center_size(mouse_pos, egui::vec2(180., 90.) * self.zoom);
            ui.painter().rect_filled(ghost_rect, 6.0 * self.zoom, egui::Color32::from_rgba_unmultiplied(150, 150, 150, 80));
            ui.painter().rect_stroke(ghost_rect, 6.0 * self.zoom, egui::Stroke::new(1.5, egui::Color32::WHITE));
            if ctx.input(|i| i.pointer.any_released()) {
                let drop_world_pos = self.screen_to_world(mouse_pos - egui::vec2(90.0, 45.0) * self.zoom);
                let kind_to_spawn = kind.clone();
                self.drag_node_kind = None;
                self.spawn_node(drop_world_pos, &kind_to_spawn);
            }
        }
    }

    fn sample_bezier(&self, p: [egui::Pos2; 4], t: f32) -> egui::Pos2 {
        let t2 = t * t; let t3 = t2 * t; let mt = 1.0 - t; let mt2 = mt * mt; let mt3 = mt2 * mt;
        egui::pos2(p[0].x * mt3 + 3.0 * p[1].x * mt2 * t + 3.0 * p[2].x * mt * t2 + p[3].x * t3, p[0].y * mt3 + 3.0 * p[1].y * mt2 * t + 3.0 * p[2].y * mt * t2 + p[3].y * t3)
    }
}