use crate::engine::{Event, EventType, ScheduleCmd, SystemInspector};
use crate::traits::{Component, NodeId};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub arrival_rate: f32,
    pub timeout: u64,
    pub generation_id: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            arrival_rate: 5.0,
            timeout: 5000,
            generation_id: 1,
        }
    }
}

pub struct Client {
    pub name: String,
    pub config: Arc<RwLock<ClientConfig>>,
    pub target_id: Option<NodeId>,
    pub rng: StdRng,
    pub window: VecDeque<u64>,
    pub request_counter: u64,
    pub healthy: bool,
    pub display_throughput: f32,
    pub display_snapshot: serde_json::Value,
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
            request_counter: 0,
            healthy: true,
            display_throughput: 0.0,
            display_snapshot: serde_json::Value::Null,
        }
    }
    fn update_window(&mut self, current_time_us: u64) {
        let window_size_us = 1_000_000;
        while let Some(&t) = self.window.front() {
            if current_time_us > t + window_size_us {
                self.window.pop_front();
            } else {
                break;
            }
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        let cfg = ClientConfig::default();
        Self::new("Client", cfg.arrival_rate)
    }
}

impl Component for Client {
    fn on_event(&mut self, event: Event, _inspector: &dyn SystemInspector) -> Vec<ScheduleCmd> {
        self.update_window(event.time);
        let config = self.config.read().unwrap();

        match event.event_type {
            EventType::GenerateNext { generation_id } => {
                if !self.healthy || generation_id != config.generation_id {
                    return vec![];
                }

                let interval_us = if config.arrival_rate > 0.0 {
                    (1_000_000.0 / config.arrival_rate) as u64
                } else {
                    1_000_000_000
                };
                let jitter = self.rng.gen_range(0.95..1.05);
                let next_delay_us = (interval_us as f64 * jitter) as u64;

                let mut cmds = vec![ScheduleCmd {
                    delay: next_delay_us,
                    node_id: event.node_id,
                    event_type: EventType::GenerateNext { generation_id },
                }];

                if let Some(target) = self.target_id {
                    self.request_counter += 1;
                    // Structure: [NodeId: 32 bits] [Random Salt: 32 bits] [Counter: 64 bits]
                    let rid = ((event.node_id as u128) << 96)
                        | ((self.rng.next_u32() as u128) << 64)
                        | (self.request_counter as u128);
                    self.window.push_back(event.time);
                    cmds.push(ScheduleCmd {
                        delay: 0,
                        node_id: target,
                        event_type: EventType::Arrival {
                            request_id: rid,
                            path: vec![event.node_id],
                            start_time: event.time,
                            timeout: config.timeout * 1000,
                        },
                    });
                }
                cmds
            }
            _ => vec![],
        }
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        "Client"
    }
    fn palette_color_rgb(&self) -> [u8; 3] {
        [163, 190, 140]
    }
    fn palette_description(&self) -> &str {
        "External load source (RPS)"
    }
    fn encode_config(&self) -> serde_json::Value {
        serde_json::to_value(&*self.config.read().unwrap()).unwrap_or(serde_json::Value::Null)
    }

    fn apply_config(&mut self, config: serde_json::Value, node_id: NodeId) -> Vec<ScheduleCmd> {
        if let Ok(new_cfg) = serde_json::from_value(config) {
            let mut cfg_lock = self.config.write().unwrap();
            *cfg_lock = new_cfg;
            if self.healthy {
                return vec![ScheduleCmd {
                    delay: 0,
                    node_id,
                    event_type: EventType::GenerateNext {
                        generation_id: cfg_lock.generation_id,
                    },
                }];
            }
        }
        vec![]
    }

    fn get_visual_snapshot(&self) -> serde_json::Value {
        self.display_snapshot.clone()
    }

    fn sync_display_stats(&mut self, current_time_us: u64) {
        self.update_window(current_time_us);
        self.display_throughput = if self.healthy {
            self.window.len() as f32
        } else {
            0.0
        };

        let config = self.config.read().unwrap();
        self.display_snapshot = serde_json::json!({ "rate": config.arrival_rate });
    }

    fn active_requests(&self) -> u32 {
        0
    }

    fn display_throughput(&self) -> f32 {
        self.display_throughput
    }
    fn error_count(&self) -> u64 {
        0
    }
    fn set_healthy(&mut self, h: bool) {
        self.healthy = h;
    }
    fn is_healthy(&self) -> bool {
        self.healthy
    }
    fn add_target(&mut self, target: NodeId) {
        self.target_id = Some(target);
    }
    fn remove_target(&mut self, target: NodeId) {
        if self.target_id == Some(target) {
            self.target_id = None;
        }
    }
    fn get_targets(&self) -> Vec<NodeId> {
        self.target_id.map(|id| vec![id]).unwrap_or_default()
    }
    fn clear_targets(&mut self) {
        self.target_id = None;
    }
    fn reset_internal_stats(&mut self) {
        self.window.clear();
        self.display_throughput = 0.0;
        self.display_snapshot = serde_json::Value::Null;
    }

    fn set_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }
}
