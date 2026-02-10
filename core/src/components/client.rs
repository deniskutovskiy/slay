use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use rand::prelude::*;
use crate::traits::{Component, NodeId};
use crate::engine::{Event, EventType, ScheduleCmd, SystemInspector};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub arrival_rate: f32,
    pub timeout: u64,
    pub generation_id: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self { arrival_rate: 5.0, timeout: 5000, generation_id: 1 }
    }
}

pub struct Client {
    pub name: String,
    pub config: Arc<RwLock<ClientConfig>>,
    pub target_id: Option<NodeId>,
    pub rng: StdRng,
    pub window: VecDeque<u64>,
}

impl Client {
    pub fn new(name: &str, rps: f32) -> Self {
        let mut cfg = ClientConfig::default();
        cfg.arrival_rate = rps;
        Self {
            name: name.to_string(),
            config: Arc::new(RwLock::new(cfg)),
            target_id: None,
            rng: StdRng::from_entropy(),
            window: VecDeque::new(),
        }
    }

    pub fn with_timeout(self, ms: u64) -> Self {
        if let Ok(mut config) = self.config.write() { config.timeout = ms; }
        self
    }

    fn update_window(&mut self, current_time_us: u64) {
        let window_size_us = 1_000_000; // 1 second
        while let Some(&t) = self.window.front() {
            if current_time_us > t + window_size_us { self.window.pop_front(); } else { break; }
        }
    }
}

impl Default for Client { fn default() -> Self { let cfg = ClientConfig::default(); Self::new("Client", cfg.arrival_rate) } }

impl Component for Client {
    fn on_event(&mut self, event: Event, _inspector: &dyn SystemInspector) -> Vec<ScheduleCmd> {
        self.update_window(event.time);
        let config = self.config.read().unwrap();
        
        match event.event_type {
            EventType::GenerateNext { generation_id } => {
                if generation_id != config.generation_id { return vec![]; }

                // High Precision Interval Calculation
                let interval_us = if config.arrival_rate > 0.0 { (1_000_000.0 / config.arrival_rate) as u64 } else { 1_000_000_000 };
                let jitter = self.rng.gen_range(0.95..1.05);
                let next_delay_us = (interval_us as f64 * jitter) as u64;

                let mut cmds = vec![ScheduleCmd { 
                    delay: next_delay_us, 
                    node_id: event.node_id, 
                    event_type: EventType::GenerateNext { generation_id } 
                }];

                if let Some(target) = self.target_id {
                    self.window.push_back(event.time);
                    cmds.push(ScheduleCmd { 
                        delay: crate::NETWORK_DELAY_US, 
                        node_id: target, 
                        event_type: EventType::Arrival { 
                            path: vec![event.node_id], 
                            start_time: event.time, 
                            timeout: config.timeout * 1000 // ms to us
                        } 
                    });
                }
                cmds
            }
            _ => vec![],
        }
    }
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> &str { "Client" }
    fn palette_color_rgb(&self) -> [u8; 3] { [163, 190, 140] }
    fn palette_description(&self) -> &str { "External load source (RPS)" }
    fn encode_config(&self) -> serde_json::Value { serde_json::to_value(&*self.config.read().unwrap()).unwrap_or(serde_json::Value::Null) }
    fn get_visual_snapshot(&self) -> serde_json::Value {
        let config = self.config.read().unwrap();
        serde_json::json!({ "rate": config.arrival_rate })
    }
    fn sync_display_stats(&mut self) {}
    fn active_requests(&self) -> u32 { 0 }
    fn active_throughput(&self) -> f32 { self.window.len() as f32 }
    fn error_count(&self) -> u64 { 0 }
    fn set_healthy(&mut self, _: bool) {}
    fn is_healthy(&self) -> bool { true }
    fn add_target(&mut self, target: NodeId) { self.target_id = Some(target); }
    fn remove_target(&mut self, target: NodeId) { if self.target_id == Some(target) { self.target_id = None; } }
    fn get_targets(&self) -> Vec<NodeId> { self.target_id.map(|id| vec![id]).unwrap_or_default() }
    fn clear_targets(&mut self) { self.target_id = None; }
    fn reset_internal_stats(&mut self) { self.window.clear(); }
    fn wake_up(&self, node_id: NodeId, _current_time: u64) -> Vec<ScheduleCmd> {
        let config = self.config.read().unwrap();
        vec![ScheduleCmd { delay: 0, node_id, event_type: EventType::GenerateNext { generation_id: config.generation_id } }]
    }
}