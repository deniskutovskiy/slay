use eframe::egui;
use serde_json::Value;

pub mod client;
pub mod load_balancer;
pub mod server;

/// The visual counterpart of a core component.
pub trait ComponentView {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn color(&self) -> egui::Color32;
    fn render_canvas(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        snapshot: &slay_core::traits::VisualState,
        zoom: f32,
    );
    fn render_inspector(&self, ui: &mut egui::Ui, config: &mut Value) -> bool;
}

macro_rules! register_views {
    ($($kind:expr => $view_type:ty),* $(,)?) => {
        pub fn get_view(kind: &str) -> Option<&'static dyn ComponentView> {
            match kind {
                $(
                    $kind => {
                        static INSTANCE: std::sync::OnceLock<$view_type> = std::sync::OnceLock::new();
                        Some(INSTANCE.get_or_init(Default::default))
                    }
                )*
                _ => None,
            }
        }

        pub fn get_all_views() -> Vec<(&'static str, &'static dyn ComponentView)> {
            vec![
                $(
                    {
                        static INSTANCE: std::sync::OnceLock<$view_type> = std::sync::OnceLock::new();
                        ($kind, INSTANCE.get_or_init(Default::default) as &dyn ComponentView)
                    },
                )*
            ]
        }
    };
}

// Must match the kinds defined in core/src/components/mod.rs
register_views!(
    "Client" => client::ClientView,
    "Server" => server::ServerView,
    "LoadBalancer" => load_balancer::LoadBalancerView,
);
