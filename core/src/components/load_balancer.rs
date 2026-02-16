use crate::engine::{Event, EventType, ScheduleCmd, SystemInspector};
use crate::traits::{Component, NodeId};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

/// Strategy used to select the next target for a request
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BalancingStrategy {
    /// Distributes requests sequentially
    RoundRobin,
    /// Selects a target randomly
    Random,
    /// Selects the target with the fewest active connections
    LeastConnections,
}

/// Strategy for retry backoff calculation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RetryStrategy {
    /// Retry immediately without delay
    Immediate,
    /// Retry after a constant delay
    Constant,
    /// Retry with exponential backoff
    Exponential,
}

#[derive(Debug, Clone)]
struct RetryState {
    /// Number of retries attempted for this request
    retry_count: u32,
    /// List of nodes that have already failed for this request
    failed_targets: Vec<NodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// The balancing algorithm to use
    pub strategy: BalancingStrategy,
    /// Maximum number of retry attempts per request
    pub max_retries: u32,
    /// Base delay between retries in milliseconds
    pub retry_backoff_ms: u64,
    /// Algorithm for calculating retry delays
    pub retry_strategy: RetryStrategy,
    /// Token bucket refill rate (tokens per request)
    /// Example: 0.2 means 1 retry allowed for every 5 requests
    pub retry_budget_ratio: f32,
    /// Minimum retry rate allowed regardless of traffic volume (tokens/sec)
    pub min_retry_rate: u32,
    /// Maximum number of tokens that can be accumulated (burst limit)
    pub retry_budget_max_tokens: f32,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: BalancingStrategy::RoundRobin,
            max_retries: 2,
            retry_backoff_ms: 50,
            retry_strategy: RetryStrategy::Constant,
            retry_budget_ratio: 0.2,
            min_retry_rate: 10,
            retry_budget_max_tokens: 10.0,
        }
    }
}

pub struct LoadBalancer {
    /// Component name
    pub name: String,
    /// Thread-safe configuration
    pub config: Arc<RwLock<LoadBalancerConfig>>,
    /// List of available backend nodes
    pub targets: Vec<NodeId>,
    /// Index for Round Robin strategy
    pub next_rr_idx: usize,
    /// Map of active connections to each backend
    pub active_loads: HashMap<NodeId, u32>,
    /// Random number generator for strategies and jitter
    pub rng: StdRng,
    /// Rolling window of request timestamps for RPS calculation
    pub arrival_window: VecDeque<u64>,
    /// Component health status
    pub is_healthy: bool,
    /// Map of RequestId to selected Target NodeId
    pub state_table: HashMap<u64, NodeId>,
    /// Tracking state for requests currently being retried
    in_flight_retries: HashMap<u64, RetryState>,
    /// Total number of retries performed since start
    total_retries: u64,
    /// Current balance of retry tokens (max 10.0)
    retry_token_balance: f32,
    /// Cached throughput for UI display (syncs with ui_refresh_rate)
    pub display_throughput: f32,
    /// Cached visual snapshot for UI display
    pub display_snapshot: serde_json::Value,
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
            in_flight_retries: HashMap::new(),
            total_retries: 0,
            retry_token_balance: 10.0, // Start with full budget
            display_throughput: 0.0,
            display_snapshot: serde_json::Value::Null,
        }
    }

    fn select_target(
        &mut self,
        strategy: BalancingStrategy,
        inspector: &dyn SystemInspector,
        exclusions: &[NodeId],
    ) -> Option<NodeId> {
        let healthy_targets: Vec<NodeId> = self
            .targets
            .iter()
            .copied()
            .filter(|&id| inspector.is_node_healthy(id) && !exclusions.contains(&id))
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
                // Pick the next valid target in sequence to handle exclusions
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
            EventType::Arrival {
                request_id,
                mut path,
                start_time,
                timeout,
            } => {
                // Add tokens to the retry budget for every new incoming request.
                // Since retries are sent directly to target nodes, every Arrival
                // handled here represents a new request from an upstream client.
                {
                    let config = self.config.read().unwrap();
                    self.retry_token_balance += config.retry_budget_ratio;
                    // Cap the accumulated budget to prevent excessive bursts
                    if self.retry_token_balance > config.retry_budget_max_tokens {
                        self.retry_token_balance = config.retry_budget_max_tokens;
                    }
                }

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
                // Process the new request
                self.arrival_window.push_back(event.time);
                let strategy = self.config.read().unwrap().strategy;

                if let Some(target_id) = self.select_target(strategy, inspector, &[]) {
                    let entry = self.active_loads.entry(target_id).or_insert(0);
                    *entry += 1;
                    self.state_table.insert(request_id, target_id);
                    // Push LB to path so we get the response back
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
                // Cleanup load tracking
                if let Some(server_id) = self.state_table.remove(&request_id) {
                    if let Some(load) = self.active_loads.get_mut(&server_id) {
                        *load = load.saturating_sub(1);
                    }

                    // RETRY LOGIC
                    if !success {
                        let config = self.config.read().unwrap().clone();
                        let mut retry_state =
                            self.in_flight_retries
                                .remove(&request_id)
                                .unwrap_or(RetryState {
                                    retry_count: 0,
                                    failed_targets: Vec::new(),
                                });

                        // CHECK BUDGET
                        // We need 1.0 token to retry.
                        let has_budget = self.retry_token_balance >= 1.0;

                        // Verify if we can retry
                        if retry_state.retry_count < config.max_retries && has_budget {
                            retry_state.failed_targets.push(server_id);

                            // Select new target excluding failed ones
                            if let Some(new_target) = self.select_target(
                                config.strategy,
                                inspector,
                                &retry_state.failed_targets,
                            ) {
                                // Consume budget
                                self.retry_token_balance -= 1.0;

                                // Update retry state
                                retry_state.retry_count += 1;
                                self.in_flight_retries.insert(request_id, retry_state);
                                self.total_retries += 1;

                                // Update load stats for new target
                                *self.active_loads.entry(new_target).or_insert(0) += 1;
                                self.state_table.insert(request_id, new_target);

                                // Calculate delay
                                let mut delay_us = config.retry_backoff_ms * 1000;
                                // Add Jitter (approx 10%)
                                let jitter = self.rng.gen_range(0..=(delay_us / 10).max(1));
                                delay_us += jitter;

                                return vec![ScheduleCmd {
                                    delay: delay_us,
                                    node_id: new_target,
                                    event_type: EventType::Arrival {
                                        request_id,
                                        // We reuse the path which already contains [..., Client, LoadBalancer].
                                        // This ensures the new target sends the response back to US (the LoadBalancer),
                                        // allowing us to track the retry result and update stats.
                                        path,
                                        start_time,
                                        timeout,
                                    },
                                }];
                            }
                        }
                        // If we fall here: limits reached, no healthy targets, or no budget.
                        // Fall through to standard response handler below to return failure to client.
                        self.in_flight_retries.remove(&request_id); // Cleanup retry state
                    } else {
                        // Success: cleanup retry state
                        self.in_flight_retries.remove(&request_id);
                    }
                }

                // If success or gave up retrying: return up the stack
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
        self.display_throughput = self.active_throughput();

        // Update cached snapshot
        let mut filtered_loads = HashMap::new();
        for &tid in &self.targets {
            filtered_loads.insert(tid, *self.active_loads.get(&tid).unwrap_or(&0));
        }
        let config = self.config.read().unwrap();
        self.display_snapshot = serde_json::json!({
            "rps": self.active_throughput(),
            "strategy": format!("{:?}", config.strategy),
            "targets": self.targets,
            "loads": filtered_loads,
            "failed_count": self.error_count(),
            "total_retries": self.total_retries,
            "active_retries": self.in_flight_retries.len() as u64,
            "config": {
                "max_retries": config.max_retries,
                "retry_backoff_ms": config.retry_backoff_ms
            }
        });
    }
    fn active_requests(&self) -> u32 {
        self.active_loads.values().sum()
    }
    fn active_throughput(&self) -> f32 {
        self.arrival_window.len() as f32
    }
    fn display_throughput(&self) -> f32 {
        self.display_throughput
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
        self.in_flight_retries.clear();
        self.total_retries = 0;
        self.display_throughput = 0.0;
        self.display_snapshot = serde_json::Value::Null;
    }
    fn wake_up(&self, _node_id: NodeId, _current_time: u64) -> Vec<ScheduleCmd> {
        vec![]
    }
}
