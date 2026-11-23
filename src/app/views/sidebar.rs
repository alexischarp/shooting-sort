use eframe::egui::{
    self, Align, Align2, FontFamily, FontId, Rect, RichText, Sense, Ui, Vec2, pos2, vec2,
};

use crate::app::ShootingSortApp;
use crate::app::culling::Filter;
use crate::library::Decision;
use crate::ui::painting::{self, Icon};
use crate::ui::theme;

impl ShootingSortApp {
    /// Filters, progress and keyboard shortcuts.
    pub(in crate::app) fn sidebar(&mut self, ui: &mut Ui) {
        let total = self.culling.photos.len();
        let kept = self.culling.count(Some(Decision::Keep));
        let removed = self.culling.count(Some(Decision::Remove));
        let unsorted = total - kept - removed;

        ui.add_space(4.0);
        for (filter, icon, label, count) in [
            (Filter::All, Icon::Photos, "All Photos", total),
            (Filter::Unsorted, Icon::Unsorted, "Unsorted", unsorted),
            (Filter::Keep, Icon::Keep, "Keep", kept),
            (Filter::Remove, Icon::Remove, "To Remove", removed),
        ] {
            let (rect, response) =
                ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::click());
            let selected = self.culling.filter() == filter;
            let painter = ui.painter();
            if selected {
                painter.rect_filled(rect, 8.0, theme::GRAPHITE);
            } else if response.hovered() {
                painter.rect_filled(rect, 8.0, theme::GRAPHITE.gamma_multiply(0.5));
            }
            let color = if selected {
                theme::IVORY
            } else {
                theme::TEXT_MUTED
            };
            let icon_rect = Rect::from_center_size(
                pos2(rect.left() + 22.0, rect.center().y),
                Vec2::splat(18.0),
            );
            painting::paint_icon(painter, icon_rect, icon, color);
            painter.text(
                pos2(rect.left() + 44.0, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::new(15.0, FontFamily::Proportional),
                color,
            );
            painter.text(
                pos2(rect.right() - 12.0, rect.center().y),
                Align2::RIGHT_CENTER,
                count.to_string(),
                FontId::new(12.0, FontFamily::Monospace),
                theme::TEXT_FAINT,
            );
            if response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                self.culling.set_filter(filter);
                self.scroll_to_current = true;
            }
            ui.add_space(2.0);
        }

        ui.add_space(20.0);
        ui.label(
            RichText::new("PROGRESS")
                .size(11.0)
                .color(theme::TEXT_FAINT),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new(format!("{} of {total} handled", kept + removed)).color(theme::IVORY),
        );
        let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
        ui.painter().rect_filled(bar, 3.0, theme::GRAPHITE);
        if total > 0 {
            let mut filled = bar;
            filled.set_width(bar.width() * (kept + removed) as f32 / total as f32);
            ui.painter().rect_filled(filled, 3.0, theme::BLUE);
        }

        if let Some(error) = &self.save_error {
            ui.add_space(12.0);
            ui.label(RichText::new(error).size(12.0).color(theme::REMOVE));
        }

        ui.with_layout(egui::Layout::bottom_up(Align::LEFT), |ui| {
            let mut shortcuts = vec![("G", "Gallery view")];
            if self.gallery {
                shortcuts.push(("Enter", "Open photo"));
            }
            shortcuts.extend([
                ("U", "Clear mark"),
                ("X", "To Remove"),
                ("K", "Keep"),
                ("← →", "Previous / next"),
            ]);
            if self.gallery {
                shortcuts.push(("↑ ↓", "Row above / below"));
            }
            for (key, action) in shortcuts {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        vec2(40.0, 18.0),
                        egui::Label::new(
                            RichText::new(key)
                                .monospace()
                                .size(11.0)
                                .color(theme::TEXT_MUTED),
                        ),
                    );
                    ui.label(RichText::new(action).size(12.0).color(theme::TEXT_FAINT));
                });
            }
            ui.label(
                RichText::new("SHORTCUTS")
                    .size(11.0)
                    .color(theme::TEXT_FAINT),
            );
        });
    }
}
