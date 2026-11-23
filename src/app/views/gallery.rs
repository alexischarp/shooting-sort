use eframe::egui::{self, Sense, Ui, vec2};

use crate::app::ShootingSortApp;

impl ShootingSortApp {
    /// Grid of the photos in the list. A click opens a photo in the main view.
    pub(in crate::app) fn gallery_view(&mut self, ui: &mut Ui) {
        let visible = self.culling.visible();
        if visible.is_empty() {
            super::empty_list(ui, self.culling.photos.is_empty());
            return;
        }

        const GAP: f32 = 12.0;
        const MIN_CELL_WIDTH: f32 = 200.0;
        // Room for the current photo's outline at the edges.
        let width = ui.available_width() - 6.0;
        let columns = (((width + GAP) / (MIN_CELL_WIDTH + GAP)).floor() as usize).max(1);
        self.gallery_columns = columns;
        let cell_width = (width - GAP * (columns - 1) as f32) / columns as f32;
        let cell = vec2(cell_width, cell_width / 1.5);
        let scroll_to_current = std::mem::take(&mut self.scroll_to_current);
        let mut clicked = None;

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add_space(3.0);
                ui.spacing_mut().item_spacing = vec2(GAP, GAP);
                for row in visible.chunks(columns) {
                    ui.horizontal(|ui| {
                        ui.add_space(3.0);
                        for &i in row {
                            let (rect, response) = ui.allocate_exact_size(cell, Sense::click());
                            let is_current = self.culling.current == Some(i);
                            if is_current && scroll_to_current {
                                ui.scroll_to_rect(rect.expand(GAP), None);
                            }
                            if response.clicked() {
                                clicked = Some(i);
                            }
                            if !ui.is_rect_visible(rect) {
                                continue;
                            }
                            let hovered = response
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .hovered();
                            self.paint_thumb(ui, rect, i, is_current, hovered);
                        }
                    });
                }
                ui.add_space(3.0);
            });

        if let Some(i) = clicked {
            self.culling.current = Some(i);
            self.set_gallery(false);
        }
    }
}
