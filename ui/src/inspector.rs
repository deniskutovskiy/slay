use eframe::egui;
use slay_core::{Simulation, NodeId};
use crate::components::{get_view, ComponentView};
use crate::theme::*;

pub fn render_inspector(ui: &mut egui::Ui, simulation: &mut Simulation, selected_node: &mut Option<NodeId>, node_states: &mut std::collections::HashMap<NodeId, crate::app::NodeVisualState>) {
    ui.add_space(15.0);
    ui.heading("Properties");
    ui.add_space(10.0);

    let mut to_remove = None;
    let mut config_update = None;

    if let Some(id) = *selected_node {
        if let Some(comp) = simulation.components.get_mut(&id) {
            // Header with Delete button
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("#{} {}", id, comp.name())).strong());
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("🗑").color(COLOR_CRITICAL)).on_hover_text("Delete Node").clicked() {
                        to_remove = Some(id);
                    }
                });
            });
            
            ui.separator();
            ui.add_space(10.0);
            
            // Component-specific settings
            if let Some(view) = get_view(comp.kind()) {
                let mut config_json = comp.encode_config();
                let v: &dyn ComponentView = view.as_ref();
                if v.render_inspector(ui, &mut config_json) {
                    config_update = Some((id, config_json));
                }
            }

            // Chaos Engineering Section
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);
            ui.label(egui::RichText::new("CHAOS ENGINEERING").small().strong().color(COLOR_WARN));
            
            let is_healthy = comp.is_healthy();
            let btn_text = if is_healthy { "🔴 KILL" } else { "🟢 REVIVE" };
            let btn_col = if is_healthy { COLOR_CRITICAL } else { COLOR_SUCCESS };
            
            if ui.add(egui::Button::new(egui::RichText::new(btn_text).strong()).fill(btn_col.gamma_multiply(0.2))).clicked() {
                comp.set_healthy(!is_healthy);
            }
        }
    } else {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label(egui::RichText::new("Select a node\nto see properties").color(COLOR_TEXT_DIM));
        });
    }

    // Apply updates
    if let Some((id, new_config)) = config_update {
        if let Some(comp) = simulation.components.remove(&id) {
            if let Some(mut updated_comp) = slay_core::create_component(comp.kind(), new_config) {
                for t in comp.get_targets() { updated_comp.add_target(t); }
                let cmds = updated_comp.wake_up(id, simulation.time);
                for cmd in cmds { simulation.schedule(simulation.time + cmd.delay, cmd.node_id, cmd.event_type); }
                simulation.add_component(id, updated_comp);
            }
        }
    }

    if let Some(id) = to_remove {
        simulation.remove_node(id);
        node_states.remove(&id);
        *selected_node = None;
    }
}
