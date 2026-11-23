//! Keyboard, drag-and-drop and zoom gestures.

use std::path::PathBuf;

use eframe::egui::{self, Key, Rect, Ui, Vec2};

use crate::app::ShootingSortApp;
use crate::library::Decision;
use crate::zoom;

/// Largest zoom, relative to 100 % (one photo pixel per screen pixel).
const MAX_ZOOM: f32 = 4.0;
/// Zoom step for the `+` and `-` keys.
const ZOOM_STEP: f32 = 1.5;

impl ShootingSortApp {
    pub(super) fn handle_keys(&mut self, ctx: &egui::Context) {
        let open = ctx.input(|i| i.modifiers.command && i.key_pressed(Key::O));
        if open {
            self.open_folder_dialog();
            return;
        }
        if self.folder.is_some() && ctx.input(|i| i.key_pressed(Key::G)) {
            self.set_gallery(!self.gallery);
        }
        if self.culling.current.is_none() {
            return;
        }
        let (left, right, up, down, enter, keep, remove, clear) = ctx.input(|i| {
            (
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::ArrowDown),
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::K),
                i.key_pressed(Key::X),
                i.key_pressed(Key::U),
            )
        });
        if self.gallery {
            if up {
                self.scroll_to_current |= self.culling.step_row(false, self.gallery_columns);
            }
            if down {
                self.scroll_to_current |= self.culling.step_row(true, self.gallery_columns);
            }
            if enter {
                self.set_gallery(false);
            }
        }
        if left {
            self.scroll_to_current |= self.culling.step(false);
        }
        if right {
            self.scroll_to_current |= self.culling.step(true);
        }
        if keep {
            self.set_decision(Some(Decision::Keep), true);
        }
        if remove {
            self.set_decision(Some(Decision::Remove), true);
        }
        if clear {
            self.set_decision(None, false);
        }
    }

    pub(super) fn handle_dropped_folder(&mut self, ctx: &egui::Context) {
        let dropped: Option<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_owned())
                .find(|p| p.is_dir())
        });
        if let Some(folder) = dropped {
            self.load_folder(folder);
        }
    }

    /// Double-click or `Z` toggles fit / 100 %. Pinch or ⌘-scroll zooms, `+` / `-` step,
    /// `Esc` returns to fit. While zoomed, drag or scroll to pan.
    pub(super) fn handle_zoom_input(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        fit: Rect,
        view: Rect,
        actual_size: f32,
    ) {
        let hover = response.hover_pos();
        let anchor = hover.unwrap_or(view.center());
        let max_factor = actual_size * MAX_ZOOM;
        let factor = self.zoom.map_or(1.0, |z| z.factor);

        let keys_active = self.dialog.is_none();
        let (toggle_key, zoom_in, zoom_out, reset) = ui.input(|i| {
            (
                keys_active && i.key_pressed(Key::Z),
                keys_active && (i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals)),
                keys_active && i.key_pressed(Key::Minus),
                keys_active && i.key_pressed(Key::Escape),
            )
        });

        let mut target = None;
        if response.double_clicked() || toggle_key {
            // A photo smaller than the view is already at 100 % when fitted: zoom 2× instead.
            let actual = if actual_size > 1.01 { actual_size } else { 2.0 };
            target = Some(if self.zoom.is_some() { 1.0 } else { actual });
        } else if zoom_in {
            target = Some(factor * ZOOM_STEP);
        } else if zoom_out {
            target = Some(factor / ZOOM_STEP);
        } else if reset {
            target = Some(1.0);
        } else if hover.is_some() {
            let pinch = ui.input(|i| i.zoom_delta());
            if pinch != 1.0 {
                target = Some(factor * pinch);
            }
        }
        if let Some(target) = target {
            self.zoom = zoom::zoom_to(self.zoom, target.min(max_factor), anchor, fit, view);
        }

        if let Some(zoom) = &mut self.zoom {
            let mut delta = response.drag_delta();
            if hover.is_some() {
                delta += ui.input(|i| i.smooth_scroll_delta());
            }
            if delta != Vec2::ZERO {
                zoom.pan(delta, fit, view);
            }
            if response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            } else if hover.is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        } else if hover.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ZoomIn);
        }
    }
}
