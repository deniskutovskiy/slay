use slay_core::*;
use std::sync::{Arc, RwLock};

pub struct TestHarness {
    pub sim: Simulation,
}

pub struct ClientHandle {
    pub _id: NodeId,
    pub config: Arc<RwLock<slay_core::components::client::ClientConfig>>,
}

pub struct ServerHandle {
    pub _id: NodeId,
    pub _config: Arc<RwLock<slay_core::components::server::ServerConfig>>,
}

impl TestHarness {
    pub fn new() -> Self {
        Self {
            sim: Simulation::new(),
        }
    }

    pub fn add(&mut self, id: NodeId, component: Box<dyn Component>) {
        self.sim.add_component(id, component);
    }

    pub fn add_client(&mut self, id: NodeId, rps: f32) -> ClientHandle {
        let client = Client::new("Client", rps);
        let config = Arc::clone(&client.config);
        self.add(id, Box::new(client));
        ClientHandle { _id: id, config }
    }

    pub fn add_server(
        &mut self,
        id: NodeId,
        name: &str,
        service_time: u64,
        concurrency: u32,
        backlog: u32,
    ) -> ServerHandle {
        let server = Server::new(name, service_time, concurrency, backlog);
        let config = Arc::clone(&server.config);
        self.add(id, Box::new(server));
        ServerHandle {
            _id: id,
            _config: config,
        }
    }

    pub fn connect(&mut self, from: NodeId, to: NodeId) {
        self.connect_with_link(from, to, Link::default());
    }

    pub fn connect_with_link(&mut self, from: NodeId, to: NodeId, link: Link) {
        self.sim.connect_node(from, to, link);
    }

    pub fn start(&mut self) {
        let ids: Vec<NodeId> = self.sim.components.keys().cloned().collect();
        for id in ids {
            if let Some(comp) = self.sim.components.get(&id) {
                let cmds = comp.wake_up(id, self.sim.time);
                for cmd in cmds {
                    self.sim
                        .schedule(self.sim.time + cmd.delay, cmd.node_id, cmd.event_type);
                }
            }
        }
    }

    pub fn run_for(&mut self, duration_ms: u64) {
        let duration_us = duration_ms * 1000;
        let end_time = self.sim.time + duration_us;
        while self.sim.time < end_time {
            if !self.sim.step() {
                if let Some(next_event) = self.sim.events.peek() {
                    let next_time = next_event.0.time;
                    if next_time < end_time {
                        self.sim.time = next_time;
                        continue;
                    }
                }
                self.sim.time = end_time;
                break;
            }
        }
        self.sync_stats();
    }

    pub fn sync_stats(&mut self) {
        for comp in self.sim.components.values_mut() {
            comp.sync_display_stats(self.sim.time);
        }
    }

    pub fn sla(&self) -> f32 {
        let total = self.sim.success_count + self.sim.failure_count;
        if total == 0 {
            return 100.0;
        }
        (self.sim.success_count as f32 / total as f32) * 100.0
    }

    pub fn p99(&self) -> u64 {
        self.sim.get_percentile(99.0, 5_000_000).unwrap_or(0) / 1000
    }
}
