use crate::components::{get_view, ComponentView};
use crate::theme::*;
use eframe::egui;
use slay_core::{NodeId, Simulation};

pub fn render_inspector(
    ui: &mut egui::Ui,
    simulation: &mut Simulation,
    selected_node: &mut Option<NodeId>,
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
