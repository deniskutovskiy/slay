use std::collections::HashMap;
use eframe::egui;
use slay_core::{Simulation, NodeId, create_component};
use crate::palette::render_palette;
use crate::inspector::render_inspector;
use crate::theme::*;
use serde::{Serialize, Deserialize};

const STATE_FILE: &str = "target/slay_state.json";

#[derive(Serialize, Deserialize, Clone)]
pub struct Vec2Serde { pub x: f32, pub y: f32 }

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeVisualState {
    pub pos: Vec2Serde,
    pub last_sync_time: f64,
}

pub struct SlayApp {
    pub simulation: Simulation,
    pub node_states: HashMap<NodeId, NodeVisualState>,
    pub next_node_id: NodeId,
    pub pan: egui::Vec2,
    pub zoom: f32,
    pub selected_node: Option<NodeId>,
    pub linking_from: Option<NodeId>,
    pub drag_node_kind: Option<String>,
    pub is_running: bool,
    pub sim_speed: f32,
    pub stats_window_seconds: f32,
    pub ui_refresh_rate: f32,
    pub last_frame_time: f64,
}

impl SlayApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        if let Err(_) = app.load_state() {
            app.setup_default_topology();
        }
        app
    }

    pub fn setup_default_topology(&mut self) {
        self.reset();
        self.spawn_node(egui::pos2(-300.0, 0.0), "Client");
        self.spawn_node(egui::pos2(0.0, 0.0), "LoadBalancer");
        self.spawn_node(egui::pos2(300.0, -70.0), "Server");
        self.spawn_node(egui::pos2(300.0, 70.0), "Server");

        if let Some(c) = self.simulation.components.get_mut(&1) { c.add_target(2); }
        if let Some(lb) = self.simulation.components.get_mut(&2) { 
            lb.add_target(3); 
            lb.add_target(4); 
        }
        self.selected_node = None;
    }

    pub fn load_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(STATE_FILE)?;
        let state: PersistedState = serde_json::from_str(&data)?;
        self.node_states = state.visuals;
        self.next_node_id = state.next_id;
        for (id, kind, config_json, targets) in state.nodes {
            if let Some(mut comp) = create_component(&kind, config_json) {
                for t in targets { comp.add_target(t); }
                let cmds = comp.wake_up(id, self.simulation.time);
                for cmd in cmds { self.simulation.schedule(self.simulation.time + cmd.delay, cmd.node_id, cmd.event_type); }
                self.simulation.add_component(id, comp);
            }
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        self.simulation = Simulation::new();
        self.node_states.clear();
        self.next_node_id = 1;
        self.linking_from = None;
        self.selected_node = None;
        self.is_running = false;
        self.drag_node_kind = None;
        self.pan = egui::Vec2::ZERO;
        self.zoom = 1.0;
    }

    pub fn save_state(&self) {
        let mut nodes = Vec::new();
        for (id, comp) in &self.simulation.components {
            nodes.push((*id, comp.kind().to_string(), comp.encode_config(), comp.get_targets()));
        }
        let state = PersistedState { nodes, visuals: self.node_states.clone(), next_id: self.next_node_id };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(STATE_FILE, json);
        }
    }

    pub fn spawn_node(&mut self, world_pos: egui::Pos2, kind: &str) {
        let id = self.next_node_id;
        self.next_node_id += 1;
        if let Some(comp) = create_component(kind, serde_json::Value::Null) {
            let cmds = comp.wake_up(id, self.simulation.time);
            for cmd in cmds { self.simulation.schedule(self.simulation.time + cmd.delay, cmd.node_id, cmd.event_type); }
            self.simulation.add_component(id, comp);
        } else { return; }
        self.node_states.insert(id, NodeVisualState { pos: Vec2Serde { x: world_pos.x, y: world_pos.y }, last_sync_time: 0.0 });
        self.selected_node = Some(id);
    }
}

impl Default for SlayApp {
    fn default() -> Self {
        Self {
            simulation: Simulation::new(), node_states: HashMap::new(), next_node_id: 1,
            pan: egui::Vec2::ZERO, zoom: 1.0, selected_node: None, linking_from: None,
            drag_node_kind: None, is_running: false, sim_speed: 1.0,
            stats_window_seconds: 10.0, ui_refresh_rate: 0.2, last_frame_time: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PersistedState {
    nodes: Vec<(NodeId, String, serde_json::Value, Vec<NodeId>)>,
    visuals: HashMap<NodeId, NodeVisualState>,
    next_id: NodeId,
}

impl eframe::App for SlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let current_real_time = ctx.input(|i| i.time);
        let dt = (current_real_time - self.last_frame_time).max(0.0);
        self.last_frame_time = current_real_time;

        if self.is_running {
            let virtual_dt = (dt as f32 * self.sim_speed * 1000.0) as u64;
            let target_virtual_time = self.simulation.time + virtual_dt;
            let mut processed = 0;
            while let Some(event) = self.simulation.events.peek() {
                if event.0.time <= target_virtual_time && processed < 5000 { 
                    self.simulation.step(); 
                    processed += 1;
                } else { break; }
            }
            self.simulation.time = target_virtual_time;
            ctx.request_repaint();
        }

        for (id, state) in self.node_states.iter_mut() {
            if current_real_time - state.last_sync_time >= self.ui_refresh_rate as f64 {
                if let Some(comp) = self.simulation.components.get_mut(id) {
                    comp.sync_display_stats();
                    state.last_sync_time = current_real_time;
                }
            }
        }

        egui::TopBottomPanel::bottom("bottom_dashboard").frame(egui::Frame::none().fill(COLOR_PANEL).inner_margin(12.0)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("HEALTH").strong().size(10.0).color(COLOR_TEXT_DIM));
                let total = self.simulation.success_count + self.simulation.failure_count;
                let sla = if total > 0 { (self.simulation.success_count as f32 / total as f32) * 100.0 } else { 100.0 };
                let sla_color = if sla < 99.0 { COLOR_CRITICAL } else { COLOR_SUCCESS };
                ui.label(egui::RichText::new("SLA:").color(sla_color));
                ui.label(egui::RichText::new(format!("{:.2}%", sla)).strong().color(sla_color));
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(20.0);
                ui.label(egui::RichText::new("PERFORMANCE").strong().size(10.0).color(COLOR_TEXT_DIM));
                let w_ms = (self.stats_window_seconds * 1000.0) as u64;
                let p99 = self.simulation.get_percentile(99.0, w_ms);
                ui.label(egui::RichText::new("P99:").color(COLOR_ACCENT));
                ui.label(egui::RichText::new(format!("{}ms", p99)).strong().color(COLOR_ACCENT));
                ui.add_space(10.0);
                ui.add(egui::Slider::new(&mut self.stats_window_seconds, 1.0..=60.0).suffix("s"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("{:.1}s", self.simulation.time as f32 / 1000.0)).strong().color(COLOR_TEXT));
                    ui.label(egui::RichText::new("CLOCK:").color(COLOR_TEXT));
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);
                    if ui.button("Reset Stats").clicked() { self.simulation.reset_stats(); }
                });
            });
        });

        egui::SidePanel::left("palette").default_width(200.0).show(ctx, |ui| { render_palette(ui, self); });
        egui::SidePanel::right("inspector").default_width(240.0).show(ctx, |ui| { render_inspector(ui, &mut self.simulation, &mut self.selected_node, &mut self.node_states); });
        egui::CentralPanel::default().frame(egui::Frame::none().fill(COLOR_BG)).show(ctx, |ui| { self.render_canvas(ui, ctx); });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_initial_state() {
        let app = SlayApp::default();
        assert_eq!(app.zoom, 1.0);
        assert!(!app.is_running);
    }

    #[test]
    fn test_node_spawning_and_selection() {
        let mut app = SlayApp::default();
        app.reset(); 
        app.spawn_node(egui::pos2(100.0, 100.0), "Client");
        assert_eq!(app.simulation.components.len(), 1);
        assert_eq!(app.node_states.len(), 1);
        assert!(app.selected_node.is_some());
    }
}
