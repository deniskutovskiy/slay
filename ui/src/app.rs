use crate::analytics::SparklineWidget;
use crate::inspector::render_inspector;
use crate::palette::render_palette;
use crate::theme::*;
use eframe::egui;
use serde::{Deserialize, Serialize};
use slay_core::{create_component, Link, MetricsCollector, NodeId, Simulation};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct Vec2Serde {
    pub x: f32,
    pub y: f32,
}

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
    pub target_pan: egui::Vec2,
    pub zoom: f32,
    pub target_zoom: f32,
    pub selected_node: Option<NodeId>,
    pub selected_edge: Option<(NodeId, NodeId)>,
    pub linking_from: Option<NodeId>,
    pub drag_node_kind: Option<String>,
    pub should_fit_to_view: bool,
    pub is_running: bool,
    pub sim_speed: f32,
    pub stats_window_seconds: f32,
    pub ui_refresh_rate: f32,
    pub last_frame_time: f64,
    pub frames_since_start: u32,
    pub is_initialized: bool,

    pub metrics: MetricsCollector,
}

impl SlayApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        if let Some(storage) = cc.storage {
            if let Some(state) = eframe::get_value::<PersistedState>(storage, eframe::APP_KEY) {
                app.apply_state(state);
            } else {
                app.setup_default_topology();
            }
        } else {
            app.setup_default_topology();
        }
        app
    }

    pub fn apply_state(&mut self, state: PersistedState) {
        self.node_states = state.visuals;
        // Reset sync timestamps — persisted values are from a previous session
        // and would block sync until real time catches up (potentially hours).
        for state in self.node_states.values_mut() {
            state.last_sync_time = 0.0;
        }
        self.next_node_id = state.next_id;
        for (id, kind, config_json, targets) in state.nodes {
            if let Some(mut comp) = create_component(&kind, config_json) {
                for t in targets {
                    comp.add_target(t);
                }
                let current_conf = comp.encode_config();
                let cmds = comp.apply_config(current_conf, id);
                for cmd in cmds {
                    self.simulation.schedule(
                        self.simulation.time + cmd.delay,
                        cmd.node_id,
                        cmd.event_type,
                    );
                }
                self.simulation.add_component(id, comp);
            }
        }

        if !state.links.is_empty() {
            for ((min, max), link) in state.links {
                self.simulation.links.insert((min, max), link);
            }
        }
    }

    pub fn setup_default_topology(&mut self) {
        self.reset();
        self.spawn_node(egui::pos2(-300.0, 0.0), "Client");
        self.spawn_node(egui::pos2(0.0, 0.0), "LoadBalancer");
        self.spawn_node(egui::pos2(300.0, -70.0), "Server");
        self.spawn_node(egui::pos2(300.0, 70.0), "Server");

        if let Some(c) = self.simulation.components.get_mut(&1) {
            c.add_target(2);
        }
        if let Some(lb) = self.simulation.components.get_mut(&2) {
            lb.add_target(3);
            lb.add_target(4);
        }
        self.selected_node = None;
    }

    pub fn reset(&mut self) {
        self.simulation = Simulation::new();
        self.node_states.clear();
        self.next_node_id = 1;
        self.linking_from = None;
        self.selected_node = None;
        self.selected_edge = None;
        self.is_running = false;
        self.drag_node_kind = None;
        self.should_fit_to_view = true;
        self.pan = egui::Vec2::ZERO; // Reset pan to zero, will be overridden by fit_to_view
        self.target_pan = egui::Vec2::ZERO;
        self.zoom = 1.0;
        self.target_zoom = 1.0;
        self.is_initialized = false;
        self.metrics.reset();
    }

    pub fn spawn_node(&mut self, world_pos: egui::Pos2, kind: &str) {
        let id = self.next_node_id;
        self.next_node_id += 1;
        if let Some(mut comp) = create_component(kind, serde_json::Value::Null) {
            let current_conf = comp.encode_config();
            let cmds = comp.apply_config(current_conf, id);
            for cmd in cmds {
                self.simulation.schedule(
                    self.simulation.time + cmd.delay,
                    cmd.node_id,
                    cmd.event_type,
                );
            }
            self.simulation.add_component(id, comp);
        } else {
            return;
        }
        self.node_states.insert(
            id,
            NodeVisualState {
                pos: Vec2Serde {
                    x: world_pos.x,
                    y: world_pos.y,
                },
                last_sync_time: 0.0,
            },
        );
        self.selected_node = Some(id);
    }
}

impl Default for SlayApp {
    fn default() -> Self {
        Self {
            simulation: Simulation::new(),
            node_states: HashMap::new(),
            next_node_id: 1,
            pan: egui::Vec2::ZERO,
            target_pan: egui::Vec2::ZERO,
            zoom: 1.0,
            target_zoom: 1.0,
            selected_node: None,
            selected_edge: None,
            linking_from: None,
            drag_node_kind: None,
            should_fit_to_view: true,
            is_running: false,
            sim_speed: 1.0,
            stats_window_seconds: 10.0,
            ui_refresh_rate: 0.2,
            last_frame_time: 0.0,
            frames_since_start: 0,
            is_initialized: false,
            metrics: MetricsCollector::new(300),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PersistedState {
    nodes: Vec<(NodeId, String, serde_json::Value, Vec<NodeId>)>,
    visuals: HashMap<NodeId, NodeVisualState>,
    next_id: NodeId,
    #[serde(default)]
    links: Vec<((NodeId, NodeId), Link)>,
}

impl eframe::App for SlayApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let mut nodes = Vec::new();
        for (id, comp) in &self.simulation.components {
            nodes.push((
                *id,
                comp.kind().to_string(),
                comp.encode_config(),
                comp.get_targets(),
            ));
        }
        let state = PersistedState {
            nodes,
            visuals: self.node_states.clone(),
            next_id: self.next_node_id,

            links: self
                .simulation
                .links
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
        };
        eframe::set_value(storage, eframe::APP_KEY, &state);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let current_real_time = ctx.input(|i| i.time);
        let dt = (current_real_time - self.last_frame_time).max(0.0);
        self.last_frame_time = current_real_time;

        if self.is_running {
            let virtual_dt = (dt as f32 * self.sim_speed * 1_000_000.0) as u64;
            let target_virtual_time = self.simulation.time + virtual_dt;
            let mut processed = 0;
            let max_events_per_frame = 10000;
            while let Some(event) = self.simulation.events.peek() {
                if event.0.time <= target_virtual_time && processed < max_events_per_frame {
                    self.simulation.step();
                    processed += 1;
                } else {
                    break;
                }
            }
            if processed < max_events_per_frame {
                self.simulation.time = target_virtual_time;
            }

            // Sync metrics window with UI slider
            let w_us = (self.stats_window_seconds * 1_000_000.0) as u64;
            self.metrics.update(&self.simulation, w_us);
            ctx.request_repaint();
        }

        for (id, state) in self.node_states.iter_mut() {
            if current_real_time - state.last_sync_time >= self.ui_refresh_rate as f64 {
                if let Some(comp) = self.simulation.components.get_mut(id) {
                    comp.sync_display_stats(self.simulation.time);
                    state.last_sync_time = current_real_time;
                }
            }
        }

        egui::TopBottomPanel::bottom("bottom_dashboard")
            .frame(egui::Frame::none().fill(COLOR_PANEL).inner_margin(12.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let history = self.metrics.history.as_slices().0;

                    // 1. LATENCY
                    let w_us = (self.stats_window_seconds * 1_000_000.0) as u64;
                    let p99_opt = self.simulation.get_percentile(99.0, w_us);
                    let p99_ms = p99_opt.map_or(0.0, |v| v as f32 / 1000.0);

                    let lat_label = if p99_opt.is_none() && self.simulation.failure_count > 0 {
                        "TIMEOUT".to_string()
                    } else {
                        format!("{:.1}ms", p99_ms)
                    };

                    ui.add(SparklineWidget::new(
                        "LATENCY",
                        history,
                        |m| m.p99_ms,
                        COLOR_ACCENT,
                        lat_label,
                    ));

                    ui.add_space(8.0);

                    // 2. SUCCESS
                    let cur_success = history.last().map(|m| m.success_rps).unwrap_or(0.0);
                    ui.add(SparklineWidget::new(
                        "SUCCESS",
                        history,
                        |m| m.success_rps,
                        COLOR_SUCCESS,
                        format!("{:.0} RPS", cur_success),
                    ));

                    ui.add_space(8.0);

                    // 3. ERRORS
                    let cur_fail = history.last().map(|m| m.failure_rps).unwrap_or(0.0);
                    let total = self.simulation.success_count + self.simulation.failure_count;
                    let sla = if total > 0 {
                        (self.simulation.success_count as f32 / total as f32) * 100.0
                    } else {
                        100.0
                    };
                    ui.add(SparklineWidget::new(
                        "ERRORS",
                        history,
                        |m| m.failure_rps,
                        COLOR_CRITICAL,
                        format!("{:.0} / {:.1}%", cur_fail, sla),
                    ));

                    ui.add_space(ui.available_width() - 320.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("RESET").clicked() {
                            self.simulation.reset_stats();
                            self.metrics.reset();
                            for comp in self.simulation.components.values_mut() {
                                comp.reset_internal_stats();
                            }
                        }
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:.1}s",
                                    self.simulation.time as f32 / 1_000_000.0
                                ))
                                .strong()
                                .color(COLOR_TEXT),
                            );
                            ui.label(egui::RichText::new("V-CLOCK").small().color(COLOR_TEXT_DIM));
                        });
                        ui.add_space(15.0);
                        ui.add(
                            egui::Slider::new(&mut self.stats_window_seconds, 1.0..=60.0)
                                .suffix("s wnd"),
                        );
                    });
                });
            });

        egui::SidePanel::left("palette")
            .default_width(200.0)
            .show(ctx, |ui| {
                render_palette(ui, self);
            });
        egui::SidePanel::right("inspector")
            .default_width(240.0)
            .show(ctx, |ui| {
                render_inspector(
                    ui,
                    &mut self.simulation,
                    &mut self.selected_node,
                    &mut self.selected_edge,
                    &mut self.node_states,
                    &mut self.should_fit_to_view,
                );
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(COLOR_BG))
            .show(ctx, |ui| {
                self.render_canvas(ui, ctx);
            });
    }
}
