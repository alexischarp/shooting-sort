//! Custom widgets in the Shooting Sort style.

use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2,
    vec2,
};

use crate::ui::theme;

#[derive(Clone, Copy)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Toggle { on: bool, color: Color32 },
}

/// A button of a fixed `size`, with an optional keyboard shortcut `hint` drawn on its right.
pub fn button(
    ui: &mut Ui,
    label: &str,
    hint: Option<&str>,
    kind: ButtonKind,
    size: Vec2,
    enabled: bool,
) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let hovered = enabled && response.hovered();
    let (fill, text, stroke) = match kind {
        ButtonKind::Primary => (
            if hovered {
                theme::BLUE.gamma_multiply(1.12)
            } else {
                theme::BLUE
            },
            theme::CHARCOAL,
            Stroke::NONE,
        ),
        ButtonKind::Secondary => (
            if hovered {
                theme::GRAPHITE_HOVER
            } else {
                theme::GRAPHITE
            },
            theme::IVORY,
            Stroke::new(1.0, theme::BORDER),
        ),
        ButtonKind::Toggle { on: true, color } => {
            (color.gamma_multiply(0.22), color, Stroke::new(1.5, color))
        }
        ButtonKind::Toggle { on: false, .. } => (
            if hovered {
                theme::GRAPHITE_HOVER
            } else {
                theme::GRAPHITE
            },
            theme::IVORY,
            Stroke::new(1.0, theme::BORDER),
        ),
    };
    let (fill, text) = if enabled {
        (fill, text)
    } else {
        (fill.gamma_multiply(0.45), text.gamma_multiply(0.45))
    };
    let painter = ui.painter();
    painter.rect(rect, 8.0, fill, stroke, StrokeKind::Inside);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::new(15.0, FontFamily::Proportional),
        text,
    );
    if let Some(hint) = hint {
        let key_rect =
            Rect::from_center_size(pos2(rect.right() - 22.0, rect.center().y), vec2(20.0, 20.0));
        painter.rect_stroke(
            key_rect,
            4.0,
            Stroke::new(1.0, text.gamma_multiply(0.5)),
            StrokeKind::Inside,
        );
        painter.text(
            key_rect.center(),
            Align2::CENTER_CENTER,
            hint,
            FontId::new(11.0, FontFamily::Monospace),
            text.gamma_multiply(0.7),
        );
    }
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

/// A small label on a dark background, such as the file format on a thumbnail.
pub fn format_tag(
    painter: &egui::Painter,
    anchor: egui::Pos2,
    label: &str,
    color: Color32,
) -> Rect {
    let font = FontId::new(10.0, FontFamily::Monospace);
    let galley = painter.layout_no_wrap(label.to_owned(), font, color);
    let rect = Rect::from_min_size(anchor, galley.size() + vec2(8.0, 4.0));
    painter.rect_filled(rect, 4.0, theme::CHARCOAL.gamma_multiply(0.85));
    painter.galley(rect.min + vec2(4.0, 2.0), galley, color);
    rect
}
