use crate::components::{get_view, ComponentView};
use crate::theme::*;
use eframe::egui;
use slay_core::{NodeId, Simulation};

pub fn render_inspector(
    ui: &mut egui::Ui,
    simulation: &mut Simulation,
    selected_node: &mut Option<NodeId>,
    selected_edge: &mut Option<(NodeId, NodeId)>,
    node_states: &mut std::collections::HashMap<NodeId, crate::app::NodeVisualState>,
) {
    ui.add_space(15.0);
    ui.heading("Properties");
    ui.add_space(10.0);

    let mut to_remove = None;
    let mut pending_cmds = Vec::new();

    if let Some(id) = *selected_node {
        if let Some(comp) = simulation.components.get_mut(&id) {
            // Header
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("#{} {}", id, comp.name())).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(egui::RichText::new("🗑").color(COLOR_CRITICAL))
                        .on_hover_text("Delete Node")
                        .clicked()
                    {
                        to_remove = Some(id);
                    }
                });
            });
            ui.separator();
            ui.add_space(10.0);

            // Hot Update Logic
            if let Some(view) = get_view(comp.kind()) {
                let mut config_json = comp.encode_config();
                let v: &dyn ComponentView = view.as_ref();
                if v.render_inspector(ui, &mut config_json) {
                    pending_cmds = comp.apply_config(config_json, id);
                }
            }

            // Chaos Section
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("CHAOS ENGINEERING")
                    .small()
                    .strong()
                    .color(COLOR_WARN),
            );

            let is_healthy = comp.is_healthy();
            let btn_text = if is_healthy {
                "🔴 KILL"
            } else {
                "🟢 REVIVE"
            };
            let btn_col = if is_healthy {
                COLOR_CRITICAL
            } else {
                COLOR_SUCCESS
            };

            if ui
                .add(
                    egui::Button::new(egui::RichText::new(btn_text).strong())
                        .fill(btn_col.gamma_multiply(0.2)),
                )
                .clicked()
            {
                comp.set_healthy(!is_healthy);
                if !is_healthy {
                    pending_cmds.extend(comp.wake_up(id, simulation.time));
                }
            }
        }
    } else if let Some((from, to)) = *selected_edge {
        ui.label(egui::RichText::new(format!("Edge {} <-> {}", from, to)).strong());
        ui.separator();

        let link_key = slay_core::canonical_key(from, to);

        // Link might not exist yet if created implicitly, but usually canvas creates it?
        // We need to ensure it exists if we selected it.
        let link = simulation
            .links
            .entry(link_key)
            .or_insert(slay_core::Link::default());

        // Check if data is actually asymmetric to initialize UI state correctly
        let data_is_asymmetric = link.min_to_max != link.max_to_min;

        let mut is_asymmetric = ui
            .data(|d| d.get_temp(egui::Id::new("asymmetric_edge").with(link_key)))
            .unwrap_or(data_is_asymmetric);

        // Forward Path
        {
            ui.push_id((from, to), |ui| {
                ui.label(
                    egui::RichText::new(format!("Forward Path ({} -> {})", from, to))
                        .strong()
                        .small()
                        .color(COLOR_ACCENT),
                );
                let edge = link.get_config_mut(from, to);
                render_edge_config(ui, edge);
            });
        }

        ui.add_space(10.0);

        if ui
            .checkbox(&mut is_asymmetric, "Separate Return Config")
            .changed()
        {
            ui.data_mut(|d| {
                d.insert_temp(
                    egui::Id::new("asymmetric_edge").with(link_key),
                    is_asymmetric,
                )
            });
        }

        ui.add_space(10.0);
        ui.separator();

        // Return Path
        if is_asymmetric {
            ui.push_id((to, from), |ui| {
                ui.label(
                    egui::RichText::new(format!("Return Path ({} -> {})", to, from))
                        .strong()
                        .small()
                        .color(COLOR_ACCENT),
                );
                let edge = link.get_config_mut(to, from);
                render_edge_config(ui, edge);
            });
        } else {
            // Sync Logic: Copy Forward to Return
            let fwd_cfg = *link.get_config(from, to);
            *link.get_config_mut(to, from) = fwd_cfg;

            // Render disabled return controls
            ui.push_id((to, from), |ui| {
                ui.label(
                    egui::RichText::new(format!("Return Path ({} -> {}) [Synced]", to, from))
                        .strong()
                        .small()
                        .color(COLOR_ACCENT),
                );

                ui.add_enabled_ui(false, |ui| {
                    let edge = link.get_config_mut(to, from); // Mut needed for helper signature, but UI disabled
                    render_edge_config(ui, edge);
                });
            });
        }

        ui.add_space(10.0);
        ui.small(
            egui::RichText::new(
                "Desynced timelines or lost packets simulate poor network conditions.",
            )
            .italics()
            .weak(),
        );
    } else {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label(egui::RichText::new("Select a node\nto see properties").color(COLOR_TEXT_DIM));
        });
    }

    // Schedule commands
    for cmd in pending_cmds {
        simulation.schedule(simulation.time + cmd.delay, cmd.node_id, cmd.event_type);
    }

    if let Some(id) = to_remove {
        simulation.remove_node(id);
        node_states.remove(&id);
        *selected_node = None;
    }
}

fn render_edge_config(ui: &mut egui::Ui, edge: &mut slay_core::EdgeConfig) {
    ui.horizontal(|ui| {
        ui.label("Latency");
        let mut latency_ms = edge.latency_us as f32 / 1000.0;
        if ui
            .add(
                egui::Slider::new(&mut latency_ms, 0.0..=2000.0)
                    .logarithmic(true)
                    .suffix("ms"),
            )
            .changed()
        {
            edge.latency_us = (latency_ms as f64 * 1000.0) as u64;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Jitter  ");
        let mut jitter_ms = edge.jitter_us as f32 / 1000.0;
        if ui
            .add(egui::Slider::new(&mut jitter_ms, 0.0..=500.0).suffix("ms"))
            .changed()
        {
            edge.jitter_us = (jitter_ms as f64 * 1000.0) as u64;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Loss    ");
        let mut loss_pct = edge.packet_loss_rate * 100.0;
        if ui
            .add(egui::Slider::new(&mut loss_pct, 0.0..=100.0).suffix("%"))
            .changed()
        {
            edge.packet_loss_rate = loss_pct / 100.0;
        }
    });
}
