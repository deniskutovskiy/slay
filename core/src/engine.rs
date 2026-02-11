use crate::traits::{Component, NodeId};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    Arrival {
        request_id: u64,
        path: Vec<NodeId>,
        start_time: u64,
        timeout: u64,
    },
    ProcessComplete {
        request_id: u64,
        success: bool,
        start_time: u64,
        path: Vec<NodeId>,
        timeout: u64,
    },
    Response {
        request_id: u64,
        path: Vec<NodeId>,
        start_time: u64,
        success: bool,
        timeout: u64,
    },
    GenerateNext {
        generation_id: u64,
    },
    MaintenanceComplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub time: u64,
    pub node_id: NodeId,
    pub event_type: EventType,
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}
impl Eq for Event {}
impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.time.partial_cmp(&other.time)
    }
}
impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

pub struct ScheduleCmd {
    pub delay: u64,
    pub node_id: NodeId,
    pub event_type: EventType,
}

pub trait SystemInspector {
    fn is_node_healthy(&self, id: NodeId) -> bool;
}

pub struct Simulation {
    pub time: u64,
    pub components: HashMap<NodeId, Box<dyn Component>>,
    pub events: BinaryHeap<Reverse<Event>>,
    pub success_count: u64,
    pub failure_count: u64,
    pub latencies: Vec<(u64, u64)>,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            time: 0,
            components: HashMap::new(),
            events: BinaryHeap::new(),
            success_count: 0,
            failure_count: 0,
            latencies: Vec::new(),
        }
    }

    pub fn add_component(&mut self, id: NodeId, component: Box<dyn Component>) {
        self.components.insert(id, component);
    }

    pub fn remove_node(&mut self, id: NodeId) {
        self.components.remove(&id);
        for comp in self.components.values_mut() {
            comp.remove_target(id);
        }
    }

    pub fn schedule(&mut self, time: u64, node_id: NodeId, event_type: EventType) {
        self.events.push(Reverse(Event {
            time,
            node_id,
            event_type,
        }));
    }

    pub fn step(&mut self) -> bool {
        if let Some(Reverse(event)) = self.events.pop() {
            self.time = event.time;
            let node_id = event.node_id;

            if let EventType::Response {
                success,
                start_time,
                timeout,
                path,
                ..
            } = &event.event_type
            {
                if path.len() == 1 {
                    let total_time_us = self.time.saturating_sub(*start_time);
                    if total_time_us > *timeout {
                        self.failure_count += 1;
                    } else if *success {
                        self.success_count += 1;
                        self.latencies.push((self.time, total_time_us));
                        if self.latencies.len() > 10000 {
                            let cutoff = self.time.saturating_sub(60_000_000);
                            self.latencies.retain(|(t, _)| *t >= cutoff);
                        }
                    } else {
                        self.failure_count += 1;
                    }
                }
            }

            let mut health_map = HashMap::new();
            for (id, comp) in &self.components {
                health_map.insert(*id, comp.is_healthy());
            }

            if let Some(comp) = self.components.get_mut(&node_id) {
                let cmds = comp.on_event(event, &StaticInspector { health_map });
                for cmd in cmds {
                    self.schedule(self.time + cmd.delay, cmd.node_id, cmd.event_type);
                }
            }
            return true;
        }
        false
    }

    pub fn reset_stats(&mut self) {
        self.success_count = 0;
        self.failure_count = 0;
        self.latencies.clear();
    }
    pub fn get_percentile(&self, p: f32, window_us: u64) -> Option<u64> {
        let cutoff = self.time.saturating_sub(window_us);
        let mut sample: Vec<u64> = self
            .latencies
            .iter()
            .filter(|(t, _)| *t >= cutoff)
            .map(|(_, l)| *l)
            .collect();
        if sample.is_empty() {
            return None;
        }
        sample.sort_unstable();
        let idx = ((p / 100.0) * (sample.len() as f32 - 1.0)) as usize;
        Some(sample[idx])
    }
}

struct StaticInspector {
    health_map: HashMap<NodeId, bool>,
}
impl SystemInspector for StaticInspector {
    fn is_node_healthy(&self, id: NodeId) -> bool {
        *self.health_map.get(&id).unwrap_or(&false)
    }
}
