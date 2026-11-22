mod app;
mod library;
mod loader;
mod session;
mod sorter;
mod theme;
mod zoom;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Shooting Sort")
            .with_inner_size([1280.0, 860.0])
            .with_min_inner_size([900.0, 600.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Shooting Sort",
        options,
        Box::new(|cc| Ok(Box::new(app::ShootingSortApp::new(cc)))),
    )
}
