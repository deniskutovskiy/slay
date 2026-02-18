use crate::engine::{Event, EventType, ScheduleCmd, SystemInspector};
use crate::traits::{Component, NodeId};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

/// Configuration for the Server component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Time taken to process a request (in milliseconds)
    pub service_time: u64,
    /// Maximum number of concurrent requests processed
    pub concurrency: u32,
    /// Maximum number of pending requests in the queue
    pub backlog_limit: u32,
    /// Probability of a request failing (0.0 - 1.0)
    pub failure_probability: f32,
    /// Saturation penalty factor (0.0 - disable, 1.0 - high penalty)
    ///
    /// Determines how much latency increases when the server is near max concurrency.
    /// The penalty is calculated as:
    /// `penalty = 1.0 + (load_factor^2 * saturation_penalty)`
    ///
    /// Example:
    /// - `saturation_penalty = 0.5`
    /// - `load_factor = 1.0` (100% load)
    /// - `penalty = 1.0 + (1.0 * 0.5) = 1.5` (50% slower)
    pub saturation_penalty: f32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            service_time: 200,
            concurrency: 4,
            backlog_limit: 50,
            failure_probability: 0.0,
            saturation_penalty: 0.0,
        }
    }
}

/// Represents a backend server that processes requests
pub struct Server {
    /// Component name
    pub name: String,
    /// Thread-safe configuration
    pub config: Arc<RwLock<ServerConfig>>,
    /// Current number of requests being processed
    pub active_threads: u32,
    /// Queue of pending requests (RequestID, Path, StartTime, Timeout)
    pub queue: VecDeque<(u128, Vec<NodeId>, u64, u64)>, // RID, Path, Start, Timeout
    /// Next node to forward requests to (if any)
    pub next_hop: Option<NodeId>,
    /// Total number of errors (failures + dropped requests)
    pub errors: u64,
    /// Health status (Maintenance mode)
    pub healthy: bool,
    /// Random number generator for jitter and failure simulation
    pub rng: StdRng,
    /// Rolling window of request timestamps for RPS calculation
    pub arrival_window: VecDeque<u64>,
    /// Cached throughput for UI display
    pub display_throughput: f32,
    /// Cached visual snapshot for UI display
    pub display_snapshot: serde_json::Value,
}

impl Server {
    pub fn new(name: &str, service_time: u64, concurrency: u32, backlog: u32) -> Self {
        Self {
            name: name.to_string(),
            config: Arc::new(RwLock::new(ServerConfig {
                service_time,
                concurrency,
                backlog_limit: backlog,
                failure_probability: 0.0,
                saturation_penalty: 0.5,
            })),
            active_threads: 0,
            queue: VecDeque::new(),
            next_hop: None,
            errors: 0,
            healthy: true,
            rng: StdRng::from_entropy(),
            arrival_window: VecDeque::new(),
            display_throughput: 0.0,
            display_snapshot: serde_json::Value::Null,
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

    fn calculate_processing_delay(
        rng: &mut StdRng,
        config: &ServerConfig,
        load_factor: f32,
    ) -> u64 {
        let penalty = 1.0 + (load_factor * load_factor * config.saturation_penalty);
        let jitter = rng.gen_range(0.95..1.05);
        (config.service_time as f64 * 1000.0 * jitter * penalty as f64) as u64
    }
}

impl Default for Server {
    fn default() -> Self {
        let cfg = ServerConfig::default();
        Self::new(
            "Server",
            cfg.service_time,
            cfg.concurrency,
            cfg.backlog_limit,
        )
    }
}

impl Component for Server {
    fn on_event(&mut self, event: Event, _inspector: &dyn SystemInspector) -> Vec<ScheduleCmd> {
        self.update_rps_window(event.time);
        let config = self.config.read().unwrap();

        match event.event_type {
            EventType::Arrival {
                request_id,
                path,
                start_time,
                timeout,
            } => {
                self.arrival_window.push_back(event.time);
                if !self.healthy {
                    self.errors += 1;
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

                if config.failure_probability > 0.0
                    && self.rng.gen::<f32>() < config.failure_probability
                {
                    self.errors += 1;
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

                if self.active_threads < config.concurrency {
                    self.active_threads += 1;

                    // Calculate Saturation Penalty based on CURRENT load (including this request).
                    // The rationale: the new request will execute in a system with N concurrent threads,
                    // so it should experience the contention level of N threads, not N-1.
                    let load_factor = self.active_threads as f32 / config.concurrency as f32;
                    let delay_us =
                        Self::calculate_processing_delay(&mut self.rng, &config, load_factor);
                    vec![ScheduleCmd {
                        delay: delay_us,
                        node_id: event.node_id,
                        event_type: EventType::ProcessComplete {
                            request_id,
                            success: true,
                            start_time,
                            path,
                            timeout,
                        },
                    }]
                } else {
                    if self.queue.len() >= config.backlog_limit as usize {
                        self.errors += 1;
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
                        vec![]
                    } else {
                        self.queue
                            .push_back((request_id, path, start_time, timeout));
                        vec![]
                    }
                }
            }
            EventType::ProcessComplete {
                request_id,
                success,
                start_time,
                path,
                timeout,
            } => {
                let mut cmds = Vec::new();
                if success {
                    if let Some(hop) = self.next_hop {
                        let mut p = path.clone();
                        p.push(event.node_id);
                        cmds.push(ScheduleCmd {
                            delay: 0,
                            node_id: hop,
                            event_type: EventType::Arrival {
                                request_id,
                                path: p,
                                start_time,
                                timeout,
                            },
                        });
                    } else {
                        if let Some(&prev) = path.last() {
                            cmds.push(ScheduleCmd {
                                delay: 0,
                                node_id: prev,
                                event_type: EventType::Response {
                                    request_id,
                                    path: path.clone(),
                                    start_time,
                                    success: true,
                                    timeout,
                                },
                            });
                        }
                    }
                } else {
                    if let Some(&prev) = path.last() {
                        cmds.push(ScheduleCmd {
                            delay: 0,
                            node_id: prev,
                            event_type: EventType::Response {
                                request_id,
                                path: path.clone(),
                                start_time,
                                success: false,
                                timeout,
                            },
                        });
                    }
                }
                if let Some((next_rid, next_path, next_start, next_timeout)) =
                    self.queue.pop_front()
                {
                    // For a request coming from queue, the system is implicitly at max concurrency
                    // (since we just finished one but popped another one immediately).
                    // However, we calculate it dynamically to be safe against config changes (e.g. if concurrency increased).
                    let load_factor = self.active_threads as f32 / config.concurrency as f32;
                    let delay_us =
                        Self::calculate_processing_delay(&mut self.rng, &config, load_factor);
                    cmds.push(ScheduleCmd {
                        delay: delay_us,
                        node_id: event.node_id,
                        event_type: EventType::ProcessComplete {
                            request_id: next_rid,
                            success: true,
                            start_time: next_start,
                            path: next_path,
                            timeout: next_timeout,
                        },
                    });
                } else {
                    self.active_threads = self.active_threads.saturating_sub(1);
                }
                cmds
            }
            EventType::Response {
                request_id,
                mut path,
                start_time,
                success,
                timeout,
            } => {
                if let Some(_prev_node) = path.pop() {
                    if let Some(actual_prev) = path.last() {
                        vec![ScheduleCmd {
                            delay: 0,
                            node_id: *actual_prev,
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
        "Server"
    }
    fn palette_color_rgb(&self) -> [u8; 3] {
        [129, 161, 193]
    }
    fn palette_description(&self) -> &str {
        "Application logic & queues"
    }
    fn encode_config(&self) -> serde_json::Value {
        serde_json::to_value(&*self.config.read().unwrap()).unwrap_or(serde_json::Value::Null)
    }
    fn apply_config(&mut self, config: serde_json::Value, _node_id: NodeId) -> Vec<ScheduleCmd> {
        if let Ok(new_cfg) = serde_json::from_value(config) {
            *self.config.write().unwrap() = new_cfg;
        }
        vec![]
    }
    fn get_visual_snapshot(&self) -> serde_json::Value {
        self.display_snapshot.clone()
    }
    fn sync_display_stats(&mut self, current_time_us: u64) {
        self.update_rps_window(current_time_us);
        self.display_throughput = self.arrival_window.len() as f32;

        // Update cached snapshot
        let config = self.config.read().unwrap();
        let load_factor = if config.concurrency > 0 {
            self.active_threads as f32 / config.concurrency as f32
        } else {
            0.0
        };
        let current_penalty = 1.0 + (load_factor * load_factor * config.saturation_penalty);
        self.display_snapshot = serde_json::json!({
            "rps": self.display_throughput,
            "threads": self.active_threads,
            "concurrency": config.concurrency,
            "queue_len": self.queue.len(),
            "current_penalty": current_penalty
        });
    }
    fn active_requests(&self) -> u32 {
        self.active_threads + self.queue.len() as u32
    }

    fn display_throughput(&self) -> f32 {
        self.display_throughput
    }
    fn error_count(&self) -> u64 {
        self.errors
    }
    fn set_healthy(&mut self, h: bool) {
        self.healthy = h;
    }
    fn is_healthy(&self) -> bool {
        self.healthy
    }
    fn add_target(&mut self, target: NodeId) {
        self.next_hop = Some(target);
    }
    fn remove_target(&mut self, target: NodeId) {
        if self.next_hop == Some(target) {
            self.next_hop = None;
        }
    }
    fn get_targets(&self) -> Vec<NodeId> {
        self.next_hop.map(|id| vec![id]).unwrap_or_default()
    }
    fn clear_targets(&mut self) {
        self.next_hop = None;
    }
    fn reset_internal_stats(&mut self) {
        self.errors = 0;
        self.arrival_window.clear();
        self.queue.clear();
        self.active_threads = 0;
        self.display_throughput = 0.0;
        self.display_snapshot = serde_json::Value::Null;
    }

    fn set_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saturation_penalty_calculation() {
        let mut rng = StdRng::seed_from_u64(42);
        let config = ServerConfig {
            service_time: 10, // 10ms
            concurrency: 10,
            backlog_limit: 10,
            failure_probability: 0.0,
            saturation_penalty: 1.0,
        };

        // 1. Zero load: No penalty (approx 10ms)
        let delay_0 = Server::calculate_processing_delay(&mut rng, &config, 0.0);
        assert!(delay_0 >= 9_500 && delay_0 <= 10_500);

        // 2. 50% load: Small penalty (1.0 + 0.25*1.0 = 1.25x -> approx 12.5ms)
        let delay_50 = Server::calculate_processing_delay(&mut rng, &config, 0.5);
        assert!(delay_50 >= 11_800 && delay_50 <= 13_200);

        // 3. 100% load: Max penalty (1.0 + 1.0*1.0 = 2.0x -> approx 20ms)
        let delay_100 = Server::calculate_processing_delay(&mut rng, &config, 1.0);
        assert!(delay_100 >= 19_000 && delay_100 <= 21_000);
    }
}
