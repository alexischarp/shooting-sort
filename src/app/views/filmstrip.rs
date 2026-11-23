use eframe::egui::{self, Align, Sense, Ui, vec2};

use crate::app::ShootingSortApp;

impl ShootingSortApp {
    /// Row of thumbnails under the main photo. A click selects a photo.
    pub(in crate::app) fn filmstrip(&mut self, ui: &mut Ui) {
        let visible = self.culling.visible();
        let height = ui.available_height();
        let cell = vec2(height * 1.5, height);
        let scroll_to_current = std::mem::take(&mut self.scroll_to_current);
        let mut clicked = None;

        egui::ScrollArea::horizontal()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for &i in &visible {
                        let (rect, response) = ui.allocate_exact_size(cell, Sense::click());
                        let is_current = self.culling.current == Some(i);
                        if is_current && scroll_to_current {
                            ui.scroll_to_rect(rect, Some(Align::Center));
                        }
                        if response.clicked() {
                            clicked = Some(i);
                        }
                        if !ui.is_rect_visible(rect) {
                            continue;
                        }
                        self.paint_thumb(ui, rect, i, is_current, response.hovered());
                    }
                });
            });

        if let Some(i) = clicked {
            self.culling.current = Some(i);
        }
    }
}
