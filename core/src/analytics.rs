use crate::engine::Simulation;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct MetricPoint {
    pub sim_time_us: u64,
    pub p99_ms: f32,
    pub success_rps: f32,
    pub failure_rps: f32,
}

pub struct MetricsCollector {
    pub history: VecDeque<MetricPoint>,
    pub max_points: usize,
    last_sample_time_us: u64,
    last_success_count: u64,
    last_failure_count: u64,
    current_success_rps: f32,
    current_failure_rps: f32,
}

impl MetricsCollector {
    pub fn new(max_points: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_points),
            max_points,
            last_sample_time_us: 0,
            last_success_count: 0,
            last_failure_count: 0,
            current_success_rps: 0.0,
            current_failure_rps: 0.0,
        }
    }

    pub fn update(&mut self, sim: &Simulation, stats_window_us: u64) {
        let step_us = 200_000;
        if sim.time < self.last_sample_time_us + step_us {
            return;
        }

        let p99_ms = sim
            .get_percentile(99.0, stats_window_us)
            .map_or(0.0, |v| v as f32 / 1000.0);

        let success_delta = sim.success_count.saturating_sub(self.last_success_count);
        let failure_delta = sim.failure_count.saturating_sub(self.last_failure_count);

        let delta_t_s = (sim.time - self.last_sample_time_us) as f32 / 1_000_000.0;
        let raw_success_rps = success_delta as f32 / delta_t_s;
        let raw_failure_rps = failure_delta as f32 / delta_t_s;

        let alpha = 0.1;
        self.current_success_rps =
            self.current_success_rps * (1.0 - alpha) + raw_success_rps * alpha;
        self.current_failure_rps =
            self.current_failure_rps * (1.0 - alpha) + raw_failure_rps * alpha;

        self.history.push_back(MetricPoint {
            sim_time_us: sim.time,
            p99_ms,
            success_rps: self.current_success_rps,
            failure_rps: self.current_failure_rps,
        });

        if self.history.len() > self.max_points {
            self.history.pop_front();
        }

        self.last_sample_time_us = sim.time;
        self.last_success_count = sim.success_count;
        self.last_failure_count = sim.failure_count;
    }

    pub fn reset(&mut self) {
        self.history.clear();
        self.last_sample_time_us = 0;
        self.last_success_count = 0;
        self.last_failure_count = 0;
        self.current_success_rps = 0.0;
        self.current_failure_rps = 0.0;
    }
}
