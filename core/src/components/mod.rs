use crate::traits::Component;
use serde_json::Value;

pub mod client;
pub mod load_balancer;
pub mod server;

macro_rules! register_components {
    ($($kind:expr => $type:ty),* $(,)?) => {
        pub fn create_component(kind: &str, data: Value) -> Option<Box<dyn Component>> {
            match kind {
                $(
                    $kind => {
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

        pub fn get_palette_info() -> Vec<(String, String, [u8; 3])> {
            vec![
                $(
                    {
                        let tmp: $type = Default::default();
                        (tmp.kind().to_string(), tmp.palette_description().to_string(), tmp.palette_color_rgb())
                    }
                ),*
            ]
        }
    };
}

register_components!(
    "Client" => client::Client,
    "Server" => server::Server,
    "LoadBalancer" => load_balancer::LoadBalancer,
);
