use crate::app::SlayApp;
use crate::components::get_view;
use crate::theme::*;
use eframe::egui;
use slay_core::NodeId;

impl SlayApp {
    fn world_to_screen(&self, pos: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            pos.x * self.zoom + self.pan.x,
            pos.y * self.zoom + self.pan.y,
        )
    }

    fn screen_to_world(&self, pos: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            (pos.x - self.pan.x) / self.zoom,
            (pos.y - self.pan.y) / self.zoom,
        )
    }

    pub fn render_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let canvas_rect = ui.available_rect_before_wrap();

        // 0. Handle Global Inputs (Delete)
        self.handle_global_inputs(ctx);

        // 1. Update Camera (Animation & Input)
        self.update_camera(ui, ctx, canvas_rect);

        // 2. Auto-Fit Logic
        self.perform_auto_fit(canvas_rect);

        // 3. Draw Grid
        self.draw_grid(ui, canvas_rect);

        // 4. Draw Edges (Links)
        self.draw_edges(ui, ctx, canvas_rect);

        // 5. Draw Nodes
        self.draw_nodes(ui, ctx);

        // 6. Draw Interactive Linking
        self.draw_linking_interaction(ui, ctx);

        // 7. Loading Overlay
        if !self.is_initialized {
            self.draw_loading_overlay(ui, canvas_rect);
        }
    }

    fn update_camera(&mut self, ui: &egui::Ui, ctx: &egui::Context, _rect: egui::Rect) {
        let lerp_speed = 10.0;
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        let t = 1.0 - (-lerp_speed * dt).exp();

        self.zoom += (self.target_zoom - self.zoom) * t;
        self.pan += (self.target_pan - self.pan) * t;

        if (self.target_zoom - self.zoom).abs() > 0.001
            || (self.target_pan - self.pan).length() > 0.1
        {
            ctx.request_repaint();
        }

        if ui.ui_contains_pointer()
            && ctx.input(|i| i.pointer.button_down(egui::PointerButton::Secondary))
        {
            let delta = ctx.input(|i| i.pointer.delta());
            self.pan += delta;
            self.target_pan += delta;
        }

        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let mouse = ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::pos2(0., 0.)));
            let old_zoom = self.target_zoom;
            self.target_zoom *= (scroll * 0.001).exp();
            self.target_zoom = self.target_zoom.clamp(0.1, 5.0);
            let world_mouse = (mouse.to_vec2() - self.target_pan) / old_zoom;
            self.target_pan = mouse.to_vec2() - world_mouse * self.target_zoom;
            ctx.request_repaint();
        }
    }

    fn perform_auto_fit(&mut self, rect: egui::Rect) {
        self.frames_since_start += 1;
        let is_stabilized = self.frames_since_start > 5;

        if self.should_fit_to_view && is_stabilized && rect.width() > 10.0 && rect.height() > 10.0 {
            if !self.node_states.is_empty() {
                let mut min_x = f32::INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut max_y = f32::NEG_INFINITY;

                for s in self.node_states.values() {
                    min_x = min_x.min(s.pos.x);
                    min_y = min_y.min(s.pos.y);
                    max_x = max_x.max(s.pos.x + 180.0);
                    max_y = max_y.max(s.pos.y + 90.0);
                }
                let w = max_x - min_x;
                let h = max_y - min_y;

                if w > 0.0 && h > 0.0 {
                    let padding = 100.0;
                    let avail_w = (rect.width() - padding * 2.0).max(100.0);
                    let avail_h = (rect.height() - padding * 2.0).max(100.0);
                    let scale = (avail_w / w).min(avail_h / h).clamp(0.5, 1.0);

                    self.target_zoom = scale;
                    let content_cx = min_x + w / 2.0;
                    let content_cy = min_y + h / 2.0;
                    let screen_c = rect.center();

                    self.target_pan.x = screen_c.x - content_cx * scale;
                    self.target_pan.y = screen_c.y - content_cy * scale;

                    self.is_initialized = true;
                    log::info!(
                        "Auto-Fit: Zoom={}, Pan={:?}",
                        self.target_zoom,
                        self.target_pan
                    );
                }
            }
            self.should_fit_to_view = false;
        } else if self.should_fit_to_view && !is_stabilized {
            // Keep requesting repaint until stabilized
            // Note: external repaint request is handled by loop, or we can't do it easily here without ctx
        }
    }

    fn draw_grid(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let grid_sz = 50.0 * self.zoom;
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(35));

        let start_x = ((rect.left() - self.pan.x) / grid_sz).floor() as i32;
        let end_x = ((rect.right() - self.pan.x) / grid_sz).ceil() as i32;
        for i in start_x..=end_x {
            let x = self.pan.x + i as f32 * grid_sz;
            if x >= rect.left() && x <= rect.right() {
                ui.painter().line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    stroke,
                );
            }
        }

        let start_y = ((rect.top() - self.pan.y) / grid_sz).floor() as i32;
        let end_y = ((rect.bottom() - self.pan.y) / grid_sz).ceil() as i32;
        for i in start_y..=end_y {
            let y = self.pan.y + i as f32 * grid_sz;
            if y >= rect.top() && y <= rect.bottom() {
                ui.painter().line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    stroke,
                );
            }
        }
    }

    fn draw_loading_overlay(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(200));
        ui.centered_and_justified(|ui| {
            ui.add(egui::Spinner::new().size(32.0));
            ui.label(
                egui::RichText::new(" Loading...")
                    .heading()
                    .color(egui::Color32::WHITE),
            );
        });
    }

    fn draw_edges(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect) {
        let mouse_pos = ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::pos2(0., 0.)));

        for (id, comp) in &self.simulation.components {
            let throughput = comp.display_throughput();
            for target_id in comp.get_targets() {
                if let (Some(source_state), Some(target_state)) =
                    (self.node_states.get(id), self.node_states.get(&target_id))
                {
                    let p1 = self.world_to_screen(egui::pos2(
                        source_state.pos.x + 180.,
                        source_state.pos.y + 45.,
                    ));
                    let p2 = self
                        .world_to_screen(egui::pos2(target_state.pos.x, target_state.pos.y + 45.));
                    let cp_offset = (p2.x - p1.x).abs().max(40.0 * self.zoom) * 0.5;
                    let points = [
                        p1,
                        p1 + egui::vec2(cp_offset, 0.),
                        p2 - egui::vec2(cp_offset, 0.),
                        p2,
                    ];

                    let mut color = egui::Color32::from_gray(100);
                    let mut width = 1.5 * self.zoom;
                    let edge_key = (*id, target_id);
                    let link_key = slay_core::canonical_key(*id, target_id);

                    if let Some(link) = self.simulation.links.get(&link_key) {
                        let conf = link.get_config(*id, target_id);
                        if conf.packet_loss_rate > 0.0 {
                            let t = conf.packet_loss_rate.min(1.0);
                            let r = (100.0 + (155.0 * t)) as u8;
                            let gb = (100.0 * (1.0 - t)) as u8;
                            color = egui::Color32::from_rgb(r, gb, gb);
                        }
                    }

                    if self.selected_edge == Some(edge_key) {
                        color = COLOR_ACCENT;
                        width = 3.0 * self.zoom;
                    }

                    // Hover check
                    if !ui.ctx().is_using_pointer() && ui.rect_contains_pointer(rect) {
                        let hit_threshold = 10.0 * self.zoom;
                        for i in 0..=20 {
                            let t = i as f32 / 20.0;
                            if self.sample_bezier(points, t).distance(mouse_pos) < hit_threshold {
                                width = 3.0 * self.zoom;
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                if ctx.input(|i| i.pointer.any_click()) {
                                    self.selected_edge = Some(edge_key);
                                    self.selected_node = None;
                                }
                                break;
                            }
                        }
                    }

                    ui.painter()
                        .add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
                            points,
                            closed: false,
                            fill: egui::Color32::TRANSPARENT,
                            stroke: egui::Stroke::new(width, color).into(),
                        }));

                    // Traffic dots
                    if throughput > 0.0 && comp.is_healthy() {
                        let num_dots = (throughput / 5.0).clamp(1.0, 5.0) as i32;
                        let time = ctx.input(|i| i.time);
                        for i in 0..num_dots {
                            let t = (time as f32 * 0.5 + (i as f32 / num_dots as f32)) % 1.0;
                            ui.painter().circle_filled(
                                self.sample_bezier(points, t),
                                3.0 * self.zoom,
                                egui::Color32::from_rgb(0, 255, 255),
                            );
                        }
                    }
                }
            }
        }
    }

    fn draw_nodes(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let mut node_ids: Vec<NodeId> = self.node_states.keys().cloned().collect();
        node_ids.sort();
        let mut pending_move = Vec::new();
        let mouse_pos = ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::pos2(0., 0.)));

        // Node Spawning Ghost
        if let Some(kind) = &self.drag_node_kind {
            let ghost_rect =
                egui::Rect::from_center_size(mouse_pos, egui::vec2(180., 90.) * self.zoom);
            ui.painter().rect_filled(
                ghost_rect,
                6.0 * self.zoom,
                egui::Color32::from_rgba_unmultiplied(150, 150, 150, 80),
            );
            ui.painter().rect_stroke(
                ghost_rect,
                6.0 * self.zoom,
                egui::Stroke::new(1.5, egui::Color32::WHITE),
            );
            if ctx.input(|i| i.pointer.any_released()) {
                let world_pos =
                    self.screen_to_world(mouse_pos - egui::vec2(90.0, 45.0) * self.zoom);
                let k = kind.clone();
                self.drag_node_kind = None;
                self.spawn_node(world_pos, &k);
            }
        }

        for id in node_ids {
            let comp = self.simulation.components.get(&id).unwrap(); // Safe unwrap due to keys source
            let visual = self.node_states.get(&id).unwrap();
            let screen_pos = self.world_to_screen(egui::pos2(visual.pos.x, visual.pos.y));
            let drop_zone_padding = 20.0 * self.zoom;
            let drop_rect = egui::Rect::from_min_size(
                screen_pos - egui::vec2(drop_zone_padding, drop_zone_padding),
                egui::vec2(180.0, 90.0) * self.zoom
                    + egui::vec2(drop_zone_padding * 2.0, drop_zone_padding * 2.0),
            );

            let rect = egui::Rect::from_min_size(screen_pos, egui::vec2(180.0, 90.0) * self.zoom);

            let interact = ui.interact(rect, egui::Id::new(id), egui::Sense::click_and_drag());
            if interact.clicked() {
                self.selected_node = Some(id);
                self.selected_edge = None;
            }
            if interact.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if interact.dragged() {
                pending_move.push((id, interact.drag_delta() / self.zoom));
            }

            let rgb = comp.palette_color_rgb();
            let base_col = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            let is_healthy = comp.is_healthy();
            let fill = if is_healthy {
                base_col.gamma_multiply(0.1)
            } else {
                egui::Color32::from_gray(30)
            };

            let mut border = if self.selected_node == Some(id) {
                COLOR_WARN
            } else if self.linking_from == Some(id) {
                COLOR_ACCENT
            } else if interact.hovered() {
                base_col
            } else {
                base_col.gamma_multiply(0.5)
            };
            if !is_healthy {
                border = COLOR_CRITICAL;
            }

            ui.painter().rect_filled(rect, 6.0 * self.zoom, fill);
            ui.painter().rect_stroke(
                rect,
                6.0 * self.zoom,
                egui::Stroke::new(1.5 * self.zoom, border),
            );

            ui.painter().text(
                rect.left_top() + egui::vec2(10., 15.) * self.zoom,
                egui::Align2::LEFT_TOP,
                comp.name().to_uppercase(),
                egui::FontId::proportional(11.0 * self.zoom),
                COLOR_TEXT_DIM,
            );

            if !is_healthy {
                let m = 20.0 * self.zoom;
                let s = egui::Stroke::new(2.0 * self.zoom, COLOR_CRITICAL.gamma_multiply(0.5));
                ui.painter().line_segment(
                    [
                        rect.left_top() + egui::vec2(m, m),
                        rect.right_bottom() - egui::vec2(m, m),
                    ],
                    s,
                );
                ui.painter().line_segment(
                    [
                        rect.right_top() + egui::vec2(-m, m),
                        rect.left_bottom() + egui::vec2(m, -m),
                    ],
                    s,
                );
            }

            if let Some(view) = get_view(comp.kind()) {
                view.render_canvas(ui, rect, &comp.get_visual_snapshot(), self.zoom);
            }

            let errs = comp.error_count();
            if errs > 0 {
                ui.painter().text(
                    rect.right_bottom() - egui::vec2(10., 10.) * self.zoom,
                    egui::Align2::RIGHT_BOTTOM,
                    format!("! {} ERR", errs),
                    egui::FontId::proportional(11.0 * self.zoom),
                    COLOR_CRITICAL,
                );
            }

            // Ports
            let out_pos = screen_pos + egui::vec2(180., 45.) * self.zoom;
            ui.painter().circle_filled(
                screen_pos + egui::vec2(0., 45.) * self.zoom,
                4.5 * self.zoom,
                egui::Color32::from_gray(80),
            );
            ui.painter().circle_filled(out_pos, 4.5 * self.zoom, border);

            // Drag from output
            let out_rect =
                egui::Rect::from_center_size(out_pos, egui::vec2(30.0, 30.0) * self.zoom);
            let out_resp = ui.interact(out_rect, egui::Id::new(("out", id)), egui::Sense::drag());
            if out_resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if out_resp.drag_started() {
                self.linking_from = Some(id);
            }

            if ui.rect_contains_pointer(drop_rect) && ctx.input(|i| i.pointer.any_released()) {
                if let Some(src) = self.linking_from {
                    if src != id {
                        if let Some(c) = self.simulation.components.get_mut(&src) {
                            c.add_target(id);
                        }
                    }
                }
            }
        }

        for (id, d) in pending_move {
            if let Some(s) = self.node_states.get_mut(&id) {
                s.pos.x += d.x;
                s.pos.y += d.y;
            }
        }
    }

    fn draw_linking_interaction(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(src) = self.linking_from {
            if let Some(vis) = self.node_states.get(&src) {
                let start = self.world_to_screen(egui::pos2(vis.pos.x + 180., vis.pos.y + 45.));
                let end = ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::pos2(0., 0.)));
                ui.painter().line_segment(
                    [start, end],
                    egui::Stroke::new(1.5 * self.zoom, COLOR_ACCENT),
                );
            }
            if ctx.input(|i| i.pointer.any_released()) {
                self.linking_from = None;
            }
        }
    }

    fn sample_bezier(&self, p: [egui::Pos2; 4], t: f32) -> egui::Pos2 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        egui::pos2(
            p[0].x * mt3 + 3.0 * p[1].x * mt2 * t + 3.0 * p[2].x * mt * t2 + p[3].x * t3,
            p[0].y * mt3 + 3.0 * p[1].y * mt2 * t + 3.0 * p[2].y * mt * t2 + p[3].y * t3,
        )
    }

    fn handle_global_inputs(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Backspace) || i.key_pressed(egui::Key::Delete)) {
            if let Some((src, dst)) = self.selected_edge {
                if let Some(comp) = self.simulation.components.get_mut(&src) {
                    comp.remove_target(dst);
                    self.simulation
                        .links
                        .remove(&slay_core::canonical_key(src, dst));
                    self.selected_edge = None;
                }
            } else if let Some(id) = self.selected_node {
                self.simulation.remove_node(id);
                self.node_states.remove(&id);
                self.selected_node = None;
            }
        }
    }
}
