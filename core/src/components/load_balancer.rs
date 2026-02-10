use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, RwLock};
use rand::prelude::*;
use crate::traits::{Component, NodeId};
use crate::engine::{Event, EventType, ScheduleCmd, SystemInspector};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BalancingStrategy { RoundRobin, Random, LeastConnections }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig { pub strategy: BalancingStrategy }

impl Default for LoadBalancerConfig { fn default() -> Self { Self { strategy: BalancingStrategy::RoundRobin } } }

pub struct LoadBalancer {
    pub name: String,
    pub config: Arc<RwLock<LoadBalancerConfig>>,
    pub targets: Vec<NodeId>,
    pub next_rr_idx: usize,
    pub active_loads: HashMap<NodeId, u32>,
    pub rng: StdRng,
    pub arrival_window: VecDeque<u64>,
}

impl LoadBalancer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            config: Arc::new(RwLock::new(LoadBalancerConfig::default())),
            targets: Vec::new(), next_rr_idx: 0, active_loads: HashMap::new(),
            rng: StdRng::from_entropy(), arrival_window: VecDeque::new(),
        }
    }
    fn select_target(&mut self, strategy: BalancingStrategy) -> Option<NodeId> {
        if self.targets.is_empty() { return None; }
        match strategy {
            BalancingStrategy::Random => { let idx = self.rng.gen_range(0..self.targets.len()); Some(self.targets[idx]) }
            BalancingStrategy::RoundRobin => { let target = self.targets[self.next_rr_idx % self.targets.len()]; self.next_rr_idx = (self.next_rr_idx + 1) % self.targets.len(); Some(target) }
            BalancingStrategy::LeastConnections => { self.targets.iter().min_by_key(|&&id| self.active_loads.get(&id).unwrap_or(&0)).copied() }
        }
    }
    fn update_rps_window(&mut self, current_time: u64) {
        let window_size = 1000;
        while let Some(&t) = self.arrival_window.front() { if current_time > t + window_size { self.arrival_window.pop_front(); } else { break; } }
    }
}

impl Default for LoadBalancer { fn default() -> Self { Self::new("Load Balancer") } }

impl Component for LoadBalancer {
    fn on_event(&mut self, event: Event, _inspector: &dyn SystemInspector) -> Vec<ScheduleCmd> {
        self.update_rps_window(event.time);
        let strategy = self.config.read().unwrap().strategy;
        match event.event_type {
            EventType::Arrival { mut path, start_time, timeout } => {
                self.arrival_window.push_back(event.time);
                if let Some(target_id) = self.select_target(strategy) {
                    let entry = self.active_loads.entry(target_id).or_insert(0); *entry += 1;
                    path.push(event.node_id);
                    vec![ScheduleCmd { delay: crate::PROCESS_OVERHEAD_MS, node_id: target_id, event_type: EventType::Arrival { path, start_time, timeout } }]
                } else {
                    if let Some(&prev) = path.last() { vec![ScheduleCmd { delay: 0, node_id: prev, event_type: EventType::Response { path, start_time, success: false, timeout } }] } else { vec![] }
                }
            }
            EventType::Response { mut path, start_time, success, timeout } => {
                if let Some(&sender_id) = path.last() { let entry = self.active_loads.entry(sender_id).or_insert(1); *entry = entry.saturating_sub(1); }
                path.pop();
                if let Some(&prev_node) = path.last() { vec![ScheduleCmd { delay: crate::PROCESS_OVERHEAD_MS, node_id: prev_node, event_type: EventType::Response { path, start_time, success, timeout } }] } else { vec![] }
            }
            _ => vec![],
        }
    }
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> &str { "LoadBalancer" }
    fn palette_color_rgb(&self) -> [u8; 3] { [136, 192, 208] }
    fn palette_description(&self) -> &str { "Distributes traffic to backends" }
    fn encode_config(&self) -> serde_json::Value { serde_json::to_value(&*self.config.read().unwrap()).unwrap_or(serde_json::Value::Null) }
    fn get_visual_snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "rps": self.active_throughput(),
            "strategy": format!("{:?}", self.config.read().unwrap().strategy),
            "loads": self.active_loads,
        })
    }
    fn sync_display_stats(&mut self) {}
    fn active_requests(&self) -> u32 { self.active_loads.values().sum() }
    fn active_throughput(&self) -> f32 { self.arrival_window.len() as f32 }
    fn error_count(&self) -> u64 { 0 }
    fn set_healthy(&mut self, _: bool) {}
    fn is_healthy(&self) -> bool { true }
    fn add_target(&mut self, target: NodeId) { if !self.targets.contains(&target) { self.targets.push(target); } }
    fn get_targets(&self) -> Vec<NodeId> { self.targets.clone() }
    fn clear_targets(&mut self) { self.targets.clear(); self.active_loads.clear(); }
    fn reset_internal_stats(&mut self) { self.arrival_window.clear(); self.active_loads.clear(); }
    fn wake_up(&self, _node_id: NodeId, _current_time: u64) -> Vec<ScheduleCmd> { vec![] }
}