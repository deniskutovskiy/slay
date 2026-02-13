use crate::engine::{Event, EventType, ScheduleCmd, SystemInspector};
use crate::traits::{Component, NodeId};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BalancingStrategy {
    RoundRobin,
    Random,
    LeastConnections,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub strategy: BalancingStrategy,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: BalancingStrategy::RoundRobin,
        }
    }
}

pub struct LoadBalancer {
    pub name: String,
    pub config: Arc<RwLock<LoadBalancerConfig>>,
    pub targets: Vec<NodeId>,
    pub next_rr_idx: usize,
    pub active_loads: HashMap<NodeId, u32>,
    pub rng: StdRng,
    pub arrival_window: VecDeque<u64>,
    pub is_healthy: bool,
    pub state_table: HashMap<u64, NodeId>,
}

impl LoadBalancer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            config: Arc::new(RwLock::new(LoadBalancerConfig::default())),
            targets: Vec::new(),
            next_rr_idx: 0,
            active_loads: HashMap::new(),
            rng: StdRng::from_entropy(),
            arrival_window: VecDeque::new(),
            is_healthy: true,
            state_table: HashMap::new(),
        }
    }
    fn select_target(
        &mut self,
        strategy: BalancingStrategy,
        inspector: &dyn SystemInspector,
    ) -> Option<NodeId> {
        let healthy_targets: Vec<NodeId> = self
            .targets
            .iter()
            .copied()
            .filter(|&id| inspector.is_node_healthy(id))
            .collect();
        if healthy_targets.is_empty() {
            return None;
        }
        match strategy {
            BalancingStrategy::Random => {
                let idx = self.rng.gen_range(0..healthy_targets.len());
                Some(healthy_targets[idx])
            }
            BalancingStrategy::RoundRobin => {
                for i in 0..self.targets.len() {
                    let idx = (self.next_rr_idx + i) % self.targets.len();
                    let target = self.targets[idx];
                    if healthy_targets.contains(&target) {
                        self.next_rr_idx = (idx + 1) % self.targets.len();
                        return Some(target);
                    }
                }
                None
            }
            BalancingStrategy::LeastConnections => healthy_targets
                .iter()
                .min_by_key(|&&id| self.active_loads.get(&id).unwrap_or(&0))
                .copied(),
        }
    }
    fn update_rps_window(&mut self, current_time_us: u64) {
        let window_size_us = 1_000_000;
        while let Some(&t) = self.arrival_window.front() {
            if current_time_us > t + window_size_us {
                self.arrival_window.pop_front();
            } else {
                break;
            }
        }
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new("Load Balancer")
    }
}

impl Component for LoadBalancer {
    fn on_event(&mut self, event: Event, inspector: &dyn SystemInspector) -> Vec<ScheduleCmd> {
        self.update_rps_window(event.time);
        match event.event_type {
            EventType::MaintenanceComplete => {
                self.is_healthy = true;
                return vec![];
            }
            EventType::Arrival {
                request_id,
                mut path,
                start_time,
                timeout,
            } => {
                if !self.is_healthy {
                    if let Some(&prev) = path.last() {
                        return vec![ScheduleCmd {
                            delay: 0,
                            node_id: prev,
                            event_type: EventType::Response {
                                request_id,
                                path,
                                start_time,
                                success: false,
                                timeout,
                            },
                        }];
                    }
                    return vec![];
                }
                self.arrival_window.push_back(event.time);
                let strategy = self.config.read().unwrap().strategy;
                if let Some(target_id) = self.select_target(strategy, inspector) {
                    let entry = self.active_loads.entry(target_id).or_insert(0);
                    *entry += 1;
                    self.state_table.insert(request_id, target_id);
                    path.push(event.node_id);
                    vec![ScheduleCmd {
                        delay: crate::PROCESS_OVERHEAD_US,
                        node_id: target_id,
                        event_type: EventType::Arrival {
                            request_id,
                            path,
                            start_time,
                            timeout,
                        },
                    }]
                } else {
                    if let Some(&prev) = path.last() {
                        vec![ScheduleCmd {
                            delay: 0,
                            node_id: prev,
                            event_type: EventType::Response {
                                request_id,
                                path,
                                start_time,
                                success: false,
                                timeout,
                            },
                        }]
                    } else {
                        vec![]
                    }
                }
            }
            EventType::Response {
                request_id,
                mut path,
                start_time,
                success,
                timeout,
            } => {
                if let Some(server_id) = self.state_table.remove(&request_id) {
                    if let Some(load) = self.active_loads.get_mut(&server_id) {
                        *load = load.saturating_sub(1);
                    }
                }
                path.pop();
                if let Some(&prev_node) = path.last() {
                    vec![ScheduleCmd {
                        delay: crate::PROCESS_OVERHEAD_US,
                        node_id: prev_node,
                        event_type: EventType::Response {
                            request_id,
                            path,
                            start_time,
                            success,
                            timeout,
                        },
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        "LoadBalancer"
    }
    fn palette_color_rgb(&self) -> [u8; 3] {
        [136, 192, 208]
    }
    fn palette_description(&self) -> &str {
        "Distributes traffic to backends"
    }
    fn encode_config(&self) -> serde_json::Value {
        serde_json::to_value(&*self.config.read().unwrap()).unwrap_or(serde_json::Value::Null)
    }
    fn apply_config(&mut self, config: serde_json::Value, node_id: NodeId) -> Vec<ScheduleCmd> {
        if let Ok(new_cfg) = serde_json::from_value(config) {
            *self.config.write().unwrap() = new_cfg;
            self.is_healthy = false;
            return vec![ScheduleCmd {
                delay: 500_000,
                node_id,
                event_type: EventType::MaintenanceComplete,
            }];
        }
        vec![]
    }
    fn get_visual_snapshot(&self) -> serde_json::Value {
        let mut filtered_loads = HashMap::new();
        for &tid in &self.targets {
            filtered_loads.insert(tid, *self.active_loads.get(&tid).unwrap_or(&0));
        }
        serde_json::json!({ "rps": self.active_throughput(), "strategy": format!("{:?}", self.config.read().unwrap().strategy), "targets": self.targets, "loads": filtered_loads })
    }
    fn sync_display_stats(&mut self, current_time_us: u64) {
        self.update_rps_window(current_time_us);
    }
    fn active_requests(&self) -> u32 {
        self.active_loads.values().sum()
    }
    fn active_throughput(&self) -> f32 {
        self.arrival_window.len() as f32
    }
    fn error_count(&self) -> u64 {
        0
    }
    fn set_healthy(&mut self, h: bool) {
        self.is_healthy = h;
    }
    fn is_healthy(&self) -> bool {
        self.is_healthy
    }
    fn add_target(&mut self, target: NodeId) {
        if !self.targets.contains(&target) {
            self.targets.push(target);
        }
    }
    fn remove_target(&mut self, target: NodeId) {
        self.targets.retain(|&id| id != target);
        self.active_loads.remove(&target);
        // Clean up connection tracking entries that pointed to the removed target.
        self.state_table
            .retain(|_, &mut server_id| server_id != target);
    }
    fn get_targets(&self) -> Vec<NodeId> {
        self.targets.clone()
    }
    fn clear_targets(&mut self) {
        self.targets.clear();
        self.active_loads.clear();
        self.state_table.clear();
    }
    fn reset_internal_stats(&mut self) {
        self.arrival_window.clear();
        self.active_loads.clear();
        self.state_table.clear();
    }
    fn wake_up(&self, _node_id: NodeId, _current_time: u64) -> Vec<ScheduleCmd> {
        vec![]
    }
}
