mod app;
mod theme;
mod palette;
mod inspector;
mod canvas;
pub mod components; // NEW: Mirrored components (Views)

use app::SlayApp;
use egui::ViewportBuilder;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1300.0, 900.0])
            .with_title("Slay"),
        ..Default::default()
    };
    eframe::run_native(
        "Slay",
        options,
        Box::new(|cc| Ok(Box::new(SlayApp::new(cc)))),
    )
}