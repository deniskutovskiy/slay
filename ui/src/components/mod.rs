use eframe::egui;
use serde_json::Value;

pub mod client;
pub mod load_balancer;
pub mod server;

/// The visual counterpart of a core component.
pub trait ComponentView {
    fn render_canvas(&self, ui: &mut egui::Ui, rect: egui::Rect, snapshot: &Value, zoom: f32);
    fn render_inspector(&self, ui: &mut egui::Ui, config: &mut Value) -> bool;
}

macro_rules! register_views {
    ($($kind:expr => $view_type:ty),* $(,)?) => {
        pub fn get_view(kind: &str) -> Option<Box<dyn ComponentView>> {
            match kind {
                $(
                    $kind => Some(Box::new(<$view_type>::default())),
                )*
                _ => None,
            }
        }
    };
}

// Must match the kinds defined in core/src/components/mod.rs
register_views!(
    "Client" => client::ClientView,
    "Server" => server::ServerView,
    "LoadBalancer" => load_balancer::LoadBalancerView,
);
