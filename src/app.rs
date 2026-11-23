//! The application: its state, the actions on it and the layout of the window.

mod culling;
mod images;
mod input;
mod views;

use std::path::PathBuf;

use eframe::egui::{self, Margin, Ui};

use crate::library::{self, Decision};
use crate::session;
use crate::sorter::{self, SortReport};
use crate::ui::theme;
use crate::zoom::Zoom;
use culling::Culling;
use images::ImageCache;

enum Dialog {
    ConfirmSort,
    SortDone(SortReport),
    Error(String),
}

pub struct ShootingSortApp {
    images: ImageCache,
    folder: Option<PathBuf>,
    culling: Culling,
    zoom: Option<Zoom>,
    /// Shows a grid of all the photos in the list instead of the current photo.
    gallery: bool,
    /// Number of columns in the gallery at the last frame, for the `↑` and `↓` keys.
    gallery_columns: usize,
    /// Scroll the filmstrip or the gallery to the current photo on the next frame.
    scroll_to_current: bool,
    dialog: Option<Dialog>,
    save_error: Option<String>,
}

impl ShootingSortApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut app = Self {
            images: ImageCache::new(cc.egui_ctx.clone()),
            folder: None,
            culling: Culling::default(),
            zoom: None,
            gallery: false,
            gallery_columns: 1,
            scroll_to_current: false,
            dialog: None,
            save_error: None,
        };
        // `shooting-sort <folder>` opens that folder directly.
        if let Some(folder) = std::env::args_os().nth(1) {
            app.load_folder(PathBuf::from(folder));
        }
        app
    }

    fn open_folder_dialog(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Open a shoot folder");
        if let Some(folder) = &self.folder {
            dialog = dialog.set_directory(folder);
        }
        if let Some(folder) = dialog.pick_folder() {
            self.load_folder(folder);
        }
    }

    fn load_folder(&mut self, folder: PathBuf) {
        let mut photos = match library::scan(&folder) {
            Ok(photos) => photos,
            Err(e) => {
                self.dialog = Some(Dialog::Error(format!(
                    "Could not read {}:\n{e}",
                    folder.display()
                )));
                return;
            }
        };
        session::load(&folder, &mut photos);

        self.images.reset();
        self.zoom = None;
        self.save_error = None;
        self.culling = Culling::new(photos);
        self.folder = Some(folder);
        self.scroll_to_current = true;
    }

    fn set_gallery(&mut self, gallery: bool) {
        self.gallery = gallery;
        self.scroll_to_current = true;
    }

    /// Sets the decision of the current photo and saves the session.
    fn set_decision(&mut self, decision: Option<Decision>, advance: bool) {
        if self.culling.current.is_none() {
            return;
        }
        self.scroll_to_current |= self.culling.set_decision(decision, advance);
        self.save_session();
    }

    fn save_session(&mut self) {
        if let Some(folder) = &self.folder {
            self.save_error = session::save(folder, &self.culling.photos)
                .err()
                .map(|e| format!("Could not save selections: {e}"));
        }
    }

    fn run_sort(&mut self) {
        let Some(folder) = self.folder.clone() else {
            return;
        };
        let report = sorter::sort(&folder, &self.culling.photos);
        self.load_folder(folder);
        // Forget decisions for photos that were moved away.
        self.save_session();
        self.dialog = Some(Dialog::SortDone(report));
    }
}

impl eframe::App for ShootingSortApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.images.receive(&ctx);
        self.handle_dropped_folder(&ctx);
        if self.dialog.is_none() {
            self.handle_keys(&ctx);
        }
        let max_texture_side = ctx.input(|i| i.max_texture_side) as u32;
        self.images
            .update_requests(&self.culling, self.zoom.is_some(), max_texture_side);

        egui::Panel::top("top_bar")
            .frame(theme::panel_frame(
                theme::CHARCOAL_TOP,
                Margin::symmetric(20, 14),
            ))
            .show(ui, |ui| self.top_bar(ui));

        if self.folder.is_none() {
            egui::CentralPanel::no_frame().show(ui, |ui| self.empty_state(ui));
        } else {
            egui::Panel::left("sidebar")
                .resizable(false)
                .exact_size(232.0)
                .frame(theme::panel_frame(theme::CHARCOAL, Margin::same(14)))
                .show(ui, |ui| self.sidebar(ui));

            if !self.gallery {
                egui::Panel::bottom("filmstrip")
                    .exact_size(132.0)
                    .frame(theme::panel_frame(
                        theme::CHARCOAL,
                        Margin::symmetric(16, 14),
                    ))
                    .show(ui, |ui| self.filmstrip(ui));
            }

            egui::CentralPanel::no_frame()
                .frame(
                    egui::Frame::new()
                        .fill(theme::SURFACE)
                        .inner_margin(Margin::same(20)),
                )
                .show(ui, |ui| {
                    if self.gallery {
                        self.gallery_view(ui);
                    } else {
                        self.main_view(ui);
                    }
                });
        }

        self.dialogs(&ctx);
    }
}
