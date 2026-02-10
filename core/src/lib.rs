pub mod engine;
pub mod traits;
pub mod components;

pub use engine::{Simulation, Event, EventType, ScheduleCmd, SystemInspector};
pub use traits::{Component, NodeId};
pub use components::{create_component, get_palette_info};
pub use components::server::{Server, ServerConfig};
pub use components::client::{Client, ClientConfig};
pub use components::load_balancer::{LoadBalancer, LoadBalancerConfig};

/// Global simulation "physics" constants
pub const NETWORK_DELAY_MS: u64 = 10;
pub const PROCESS_OVERHEAD_MS: u64 = 5;