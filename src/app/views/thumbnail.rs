use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Rect, Stroke, StrokeKind, Ui, pos2, vec2,
};

use crate::app::ShootingSortApp;
use crate::library::Decision;
use crate::ui::painting;
use crate::ui::theme;
use crate::ui::widgets::format_tag;

impl ShootingSortApp {
    /// A thumbnail of the filmstrip or the gallery. Asks for the image the first time it is shown.
    pub(super) fn paint_thumb(
        &mut self,
        ui: &Ui,
        rect: Rect,
        i: usize,
        is_current: bool,
        hovered: bool,
    ) {
        let photo = &self.culling.photos[i];
        let painter = ui.painter_at(rect.expand(2.0));
        let corner = CornerRadius::same(8);
        painter.rect_filled(rect, corner, theme::GRAPHITE);

        if let Some(texture) = self.images.thumb(i) {
            // Crop to fill the cell, like a contact sheet.
            let size = texture.size_vec2();
            let cell_aspect = rect.width() / rect.height();
            let img_aspect = size.x / size.y;
            let uv = if img_aspect > cell_aspect {
                let w = cell_aspect / img_aspect;
                Rect::from_min_max(pos2((1.0 - w) / 2.0, 0.0), pos2((1.0 + w) / 2.0, 1.0))
            } else {
                let h = img_aspect / cell_aspect;
                Rect::from_min_max(pos2(0.0, (1.0 - h) / 2.0), pos2(1.0, (1.0 + h) / 2.0))
            };
            let tint = if photo.decision == Some(Decision::Remove) {
                Color32::from_gray(90)
            } else {
                Color32::WHITE
            };
            painter.add(
                egui::epaint::RectShape::filled(rect, corner, tint).with_texture(texture.id(), uv),
            );
        } else if self.images.error(i).is_some() {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "!",
                FontId::proportional(18.0),
                theme::REMOVE,
            );
        } else {
            self.images.request_thumb(i, photo.display_source());
        }

        let tag_color = theme::IVORY.gamma_multiply(0.9);
        let tag = match (&photo.jpeg, &photo.raw) {
            (Some(_), Some(_)) => "RAW",
            (Some(_), None) => "JPEG only",
            _ => "RAW only",
        };
        let tag_color = if photo.jpeg.is_some() && photo.raw.is_some() {
            tag_color
        } else {
            theme::BLUE
        };
        format_tag(
            &painter,
            rect.left_bottom() + vec2(6.0, -22.0),
            tag,
            tag_color,
        );

        if let Some(decision) = photo.decision {
            painting::paint_decision_badge(
                &painter,
                rect.right_top() + vec2(-14.0, 14.0),
                9.0,
                decision == Decision::Keep,
            );
        }

        if is_current {
            painter.rect_stroke(
                rect,
                corner,
                Stroke::new(3.0, theme::BLUE),
                StrokeKind::Outside,
            );
        } else if hovered {
            painter.rect_stroke(
                rect,
                corner,
                Stroke::new(1.5, theme::IVORY.gamma_multiply(0.5)),
                StrokeKind::Outside,
            );
        }
    }
}
