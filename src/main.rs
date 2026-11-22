// Release builds on Windows open no console window next to the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod library;
mod loader;
mod session;
mod sorter;
mod theme;
mod zoom;

fn main() -> eframe::Result {
    let viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Shooting Sort")
        .with_inner_size([1280.0, 860.0])
        .with_min_inner_size([900.0, 600.0])
        .with_drag_and_drop(true);
    // On macOS the icon comes from the app bundle (`assets/icon.icns`).
    #[cfg(not(target_os = "macos"))]
    let viewport = viewport.with_icon(eframe::egui::IconData {
        // 128×128 RGBA pixels, made from `assets/icon.svg`.
        rgba: include_bytes!("../assets/icon-128.rgba").to_vec(),
        width: 128,
        height: 128,
    });
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Shooting Sort",
        options,
        Box::new(|cc| Ok(Box::new(app::ShootingSortApp::new(cc)))),
    )
}
