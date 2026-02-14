pub mod analytics;
pub mod components;
pub mod engine;
pub mod traits;

pub use analytics::{MetricPoint, MetricsCollector};
pub use components::client::{Client, ClientConfig};
pub use components::load_balancer::{LoadBalancer, LoadBalancerConfig};
pub use components::server::{Server, ServerConfig};
pub use components::{create_component, get_palette_info};
pub use engine::{Event, EventType, ScheduleCmd, Simulation, SystemInspector};
pub use network::{canonical_key, EdgeConfig, Link};
pub use traits::{Component, NodeId};
pub mod network;

pub const PROCESS_OVERHEAD_US: u64 = 2_000;
