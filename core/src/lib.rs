pub mod engine;
pub mod traits;
pub mod components;

pub use engine::{Simulation, Event, EventType, ScheduleCmd, SystemInspector};
pub use traits::{Component, NodeId};
pub use components::{create_component, get_palette_info};
pub use components::server::{Server, ServerConfig};
pub use components::client::{Client, ClientConfig};
pub use components::load_balancer::{LoadBalancer, LoadBalancerConfig};

/// High-precision physics constants (in Microseconds)
pub const NETWORK_DELAY_US: u64 = 10_000;    // 10ms
pub const PROCESS_OVERHEAD_US: u64 = 2_000;  // 2ms
