use crate::network::{canonical_key, Link};
use crate::traits::{Component, NodeId};
use hdrhistogram::Histogram;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    Arrival {
        request_id: u128,
        path: Vec<NodeId>,
        start_time: u64,
        timeout: u64,
    },
    ProcessComplete {
        request_id: u128,
        success: bool,
        start_time: u64,
        path: Vec<NodeId>,
        timeout: u64,
    },
    Response {
        request_id: u128,
        path: Vec<NodeId>,
        start_time: u64,
        success: bool,
        timeout: u64,
    },
    GenerateNext {
        generation_id: u64,
    },
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
    pub latencies: VecDeque<(u64, u64)>,
    pub histogram: Histogram<u64>,
    pub links: HashMap<(NodeId, NodeId), Link>,
    pub health_buffer: HashMap<NodeId, bool>,
    pub rng: StdRng,
    pub seed: u64,
}

impl Simulation {
    pub fn new(seed: u64) -> Self {
        Self {
            time: 0,
            components: HashMap::new(),
            events: BinaryHeap::new(),
            success_count: 0,
            failure_count: 0,
            latencies: VecDeque::new(),
            histogram: Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).unwrap(),
            links: HashMap::new(),
            health_buffer: HashMap::new(),
            rng: StdRng::seed_from_u64(seed),
            seed,
        }
    }

    pub fn add_component(&mut self, id: NodeId, mut component: Box<dyn Component>) {
        let component_seed = self.rng.next_u64();
        component.set_seed(component_seed);
        self.components.insert(id, component);
    }

    pub fn remove_node(&mut self, id: NodeId) {
        self.components.remove(&id);
        for comp in self.components.values_mut() {
            comp.remove_target(id);
        }
        self.links.retain(|(min, max), _| *min != id && *max != id);
    }

    pub fn connect_node(&mut self, from: NodeId, to: NodeId, link: Link) {
        let key = canonical_key(from, to);
        self.links.insert(key, link);

        if let Some(comp) = self.components.get_mut(&from) {
            comp.add_target(to);
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
                        self.latencies.push_back((self.time, total_time_us));
                        self.histogram.record(total_time_us).ok();

                        let cutoff = self.time.saturating_sub(60_000_000);
                        while let Some((t, _)) = self.latencies.front() {
                            if *t < cutoff {
                                self.latencies.pop_front();
                            } else {
                                break;
                            }
                        }
                    } else {
                        self.failure_count += 1;
                    }
                }
            }

            self.health_buffer.clear();
            for (id, comp) in &self.components {
                self.health_buffer.insert(*id, comp.is_healthy());
            }

            if let Some(comp) = self.components.get_mut(&node_id) {
                let cmds = comp.on_event(
                    event,
                    &StaticInspector {
                        health_map: &self.health_buffer,
                    },
                );
                for cmd in cmds {
                    let mut delay = cmd.delay;
                    let mut should_schedule = true;

                    if matches!(
                        cmd.event_type,
                        EventType::Arrival { .. } | EventType::Response { .. }
                    ) {
                        if cmd.node_id != node_id {
                            let key = canonical_key(node_id, cmd.node_id);
                            let link = self.links.entry(key).or_insert(Link::default());
                            let edge = link.get_config(node_id, cmd.node_id);

                            if edge.packet_loss_rate > 0.0
                                && self.rng.gen::<f32>() < edge.packet_loss_rate
                            {
                                should_schedule = false;
                                self.failure_count += 1;
                            } else {
                                let jitter = if edge.jitter_us > 0 {
                                    self.rng.gen_range(0..=edge.jitter_us)
                                } else {
                                    0
                                };
                                delay += edge.latency_us + jitter;
                            }
                        }
                    }

                    if should_schedule {
                        self.schedule(self.time + delay, cmd.node_id, cmd.event_type);
                    }
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
        self.histogram.reset();
    }

    pub fn get_percentile(&self, p: f32, _window_us: u64) -> Option<u64> {
        if self.histogram.len() == 0 {
            return None;
        }
        Some(self.histogram.value_at_percentile(p as f64))
    }
}

struct StaticInspector<'a> {
    health_map: &'a HashMap<NodeId, bool>,
}
impl<'a> SystemInspector for StaticInspector<'a> {
    fn is_node_healthy(&self, id: NodeId) -> bool {
        *self.health_map.get(&id).unwrap_or(&false)
    }
}
