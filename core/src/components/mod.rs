use crate::traits::Component;
use serde_json::Value;

pub mod client;
pub mod load_balancer;
pub mod server;

macro_rules! register_components {
    ($($variant:ident => $type:path, $stats:path),* $(,)?) => {
        pub fn create_component(kind: &str, data: Value) -> Option<Box<dyn Component>> {
            match kind {
                $(
                    stringify!($variant) => {
                        let mut obj: $type = Default::default();
                        if !data.is_null() {
                            if let Ok(cfg) = serde_json::from_value(data) {
                                obj.config = std::sync::Arc::new(std::sync::RwLock::new(cfg));
                            }
                        }
                        Some(Box::new(obj))
                    }
                )*
                _ => None,
            }
        }

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub enum VisualState {
            None,
            $(
                $variant($stats),
            )*
        }
    };
}

register_components!(
    Client => client::Client, client::ClientStats,
    Server => server::Server, server::ServerStats,
    LoadBalancer => load_balancer::LoadBalancer, load_balancer::LBStats,
);
