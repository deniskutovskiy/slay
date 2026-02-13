use crate::engine::{Event, ScheduleCmd, SystemInspector};
use std::any::Any;

pub type NodeId = u32;

pub trait Component: Any {
    fn on_event(&mut self, event: Event, inspector: &dyn SystemInspector) -> Vec<ScheduleCmd>;
    fn name(&self) -> &str;
    fn kind(&self) -> &str;

    // Metadata for Palette
    fn palette_color_rgb(&self) -> [u8; 3];
    fn palette_description(&self) -> &str;

    // Serialization & Hot Update
    fn encode_config(&self) -> serde_json::Value;
    // apply_config can return commands (e.g. to schedule MaintenanceComplete)
    fn apply_config(&mut self, config: serde_json::Value, node_id: NodeId) -> Vec<ScheduleCmd>;

    // Metrics
    fn active_requests(&self) -> u32;
    fn active_threads(&self) -> u32 {
        0
    }
    fn active_throughput(&self) -> f32 {
        0.0
    }
    fn error_count(&self) -> u64;

    fn set_healthy(&mut self, healthy: bool);
    fn is_healthy(&self) -> bool;

    fn add_target(&mut self, target: NodeId);
    fn remove_target(&mut self, target: NodeId);
    fn get_targets(&self) -> Vec<NodeId>;
    fn clear_targets(&mut self);

    fn get_visual_snapshot(&self) -> serde_json::Value;
    fn sync_display_stats(&mut self, current_time_us: u64);

    fn reset_internal_stats(&mut self);
    fn wake_up(&self, node_id: NodeId, current_time: u64) -> Vec<ScheduleCmd>;
}
