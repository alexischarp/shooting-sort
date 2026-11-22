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
    let options = eframe::NativeOptions {
        viewport: viewport.with_icon(app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Shooting Sort",
        options,
        Box::new(|cc| Ok(Box::new(app::ShootingSortApp::new(cc)))),
    )
}

/// Window and Dock icon. Without one, eframe shows its own egui icon, even inside a macOS app bundle.
fn app_icon() -> eframe::egui::IconData {
    // Inside "Shooting Sort.app", an empty icon lets macOS use the sharper `icon.icns` of the bundle.
    if cfg!(target_os = "macos")
        && std::env::current_exe().is_ok_and(|exe| exe.to_string_lossy().contains(".app/Contents/MacOS/"))
    {
        return eframe::egui::IconData::default();
    }
    eframe::egui::IconData {
        // 128×128 RGBA pixels, made from `assets/icon.svg`.
        rgba: include_bytes!("../assets/icon-128.rgba").to_vec(),
        width: 128,
        height: 128,
    }
}
