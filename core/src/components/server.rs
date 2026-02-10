use std::collections::{VecDeque};
use std::sync::{Arc, RwLock};
use rand::prelude::*;
use crate::traits::{Component, NodeId};
use crate::engine::{Event, EventType, ScheduleCmd, SystemInspector};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub service_time: u64, // Still in MS from UI
    pub concurrency: u32,
    pub backlog_limit: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { service_time: 200, concurrency: 4, backlog_limit: 50 }
    }
}

pub struct Server {
    pub name: String,
    pub config: Arc<RwLock<ServerConfig>>, 
    pub active_threads: u32,
    pub queue: VecDeque<(Vec<NodeId>, u64, u64)>, 
    pub next_hop: Option<NodeId>,
    pub errors: u64,
    pub healthy: bool,
    pub rng: StdRng,
    pub arrival_window: VecDeque<u64>, 
}

impl Server {
    pub fn new(name: &str, service_time: u64, concurrency: u32, backlog: u32) -> Self {
        Self {
            name: name.to_string(),
            config: Arc::new(RwLock::new(ServerConfig { service_time, concurrency, backlog_limit: backlog })),
            active_threads: 0, queue: VecDeque::new(), next_hop: None, errors: 0,
            healthy: true, rng: StdRng::from_entropy(), arrival_window: VecDeque::new(),
        }
    }

    pub fn with_next_hop(mut self, hop: NodeId) -> Self {
        self.next_hop = Some(hop);
        self
    }

    fn update_rps_window(&mut self, current_time_us: u64) {
        let window_size_us = 1_000_000;
        while let Some(&t) = self.arrival_window.front() {
            if current_time_us > t + window_size_us { self.arrival_window.pop_front(); } else { break; }
        }
    }
}

impl Default for Server { fn default() -> Self { let cfg = ServerConfig::default(); Self::new("Server", cfg.service_time, cfg.concurrency, cfg.backlog_limit) } }

impl Component for Server {
    fn on_event(&mut self, event: Event, _inspector: &dyn SystemInspector) -> Vec<ScheduleCmd> {
        self.update_rps_window(event.time);
        let config = self.config.read().unwrap();

        match event.event_type {
            EventType::Arrival { mut path, start_time, timeout } => {
                self.arrival_window.push_back(event.time);
                if !self.healthy {
                    self.errors += 1;
                    if let Some(&prev) = path.last() { return vec![ScheduleCmd { delay: 0, node_id: prev, event_type: EventType::Response { path, start_time, success: false, timeout } }]; }
                    return vec![];
                }
                if self.active_threads < config.concurrency {
                    self.active_threads += 1;
                    let jitter = self.rng.gen_range(0.95..1.05);
                    path.push(event.node_id);
                    // CONVERT MS TO US
                    let delay_us = (config.service_time as f64 * 1000.0 * jitter) as u64;
                    vec![ScheduleCmd { delay: delay_us, node_id: event.node_id, event_type: EventType::ProcessComplete { success: true, start_time, path, timeout } }]
                } else {
                    if self.queue.len() >= config.backlog_limit as usize {
                        self.errors += 1;
                        if let Some(&prev) = path.last() { return vec![ScheduleCmd { delay: 0, node_id: prev, event_type: EventType::Response { path, start_time, success: false, timeout } }]; }
                        vec![]
                    } else { self.queue.push_back((path, start_time, timeout)); vec![] }
                }
            }
            EventType::ProcessComplete { success, start_time, mut path, timeout } => {
                let mut cmds = Vec::new();
                if success {
                    if let Some(hop) = self.next_hop { 
                        cmds.push(ScheduleCmd { delay: crate::NETWORK_DELAY_US, node_id: hop, event_type: EventType::Arrival { path, start_time, timeout } }); 
                    }
                    else {
                        path.pop(); 
                        if let Some(&prev) = path.last() { 
                            cmds.push(ScheduleCmd { delay: crate::NETWORK_DELAY_US, node_id: prev, event_type: EventType::Response { path: path.clone(), start_time, success: true, timeout } }); 
                        }
                    }
                } else {
                    path.pop();
                    if let Some(&prev) = path.last() { cmds.push(ScheduleCmd { delay: 0, node_id: prev, event_type: EventType::Response { path: path.clone(), start_time, success: false, timeout } }); }
                }
                if let Some((next_path, next_start, next_timeout)) = self.queue.pop_front() {
                    let jitter = self.rng.gen_range(0.95..1.05);
                    let mut p = next_path; p.push(event.node_id);
                    let delay_us = (config.service_time as f64 * 1000.0 * jitter) as u64;
                    cmds.push(ScheduleCmd { delay: delay_us, node_id: event.node_id, event_type: EventType::ProcessComplete { success: true, start_time: next_start, path: p, timeout: next_timeout } });
                } else { self.active_threads = self.active_threads.saturating_sub(1); }
                cmds
            }
            EventType::Response { mut path, start_time, success, timeout } => {
                path.pop();
                if let Some(&prev_node) = path.last() { vec![ScheduleCmd { delay: crate::NETWORK_DELAY_US, node_id: prev_node, event_type: EventType::Response { path, start_time, success, timeout } }] }
                else { vec![] }
            }
            _ => vec![],
        }
    }
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> &str { "Server" }
    fn palette_color_rgb(&self) -> [u8; 3] { [129, 161, 193] }
    fn palette_description(&self) -> &str { "Application logic & queues" }
    fn encode_config(&self) -> serde_json::Value { serde_json::to_value(&*self.config.read().unwrap()).unwrap_or(serde_json::Value::Null) }
    fn get_visual_snapshot(&self) -> serde_json::Value {
        let config = self.config.read().unwrap();
        serde_json::json!({ "rps": self.active_throughput(), "threads": self.active_threads, "concurrency": config.concurrency, "queue_len": self.queue.len() })
    }
    fn sync_display_stats(&mut self) {}
    fn active_requests(&self) -> u32 { self.active_threads + self.queue.len() as u32 }
    fn active_threads(&self) -> u32 { self.active_threads }
    fn active_throughput(&self) -> f32 { self.arrival_window.len() as f32 }
    fn error_count(&self) -> u64 { self.errors }
    fn set_healthy(&mut self, h: bool) { self.healthy = h; }
    fn is_healthy(&self) -> bool { self.healthy }
    fn add_target(&mut self, target: NodeId) { self.next_hop = Some(target); }
    fn remove_target(&mut self, target: NodeId) { if self.next_hop == Some(target) { self.next_hop = None; } }
    fn get_targets(&self) -> Vec<NodeId> { self.next_hop.map(|id| vec![id]).unwrap_or_default() }
    fn clear_targets(&mut self) { self.next_hop = None; }
    fn reset_internal_stats(&mut self) { self.errors = 0; self.arrival_window.clear(); self.queue.clear(); self.active_threads = 0; }
    fn wake_up(&self, _node_id: NodeId, _current_time: u64) -> Vec<ScheduleCmd> { vec![] }
}