mod analytics;
mod app;
mod canvas;
pub mod components;
mod inspector;
mod palette;
mod theme;

use app::SlayApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use egui::ViewportBuilder;
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

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` messages to the browser console.
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let runner = eframe::WebRunner::new();
        let start_result = runner
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(SlayApp::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(&format!(
                        "<p> The app has crashed: </p> <pre style=\"color:red\"> {e:?} </pre>"
                    ));
                }
            }
        }
    });
}
