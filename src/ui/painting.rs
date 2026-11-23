//! Hand-drawn logo, icons and badges from the Shooting Sort visual identity.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke, vec2};
use eframe::epaint::CubicBezierShape;

use crate::ui::theme::{BLUE, CHARCOAL, IVORY, KEEP, REMOVE};

/// The double-S monogram, drawn from the `shooting-sort-mark.svg` paths (100×100 view box).
pub fn paint_logo(painter: &Painter, rect: Rect) {
    let scale = rect.width().min(rect.height()) / 100.0;
    let origin = rect.center() - vec2(50.0, 50.0) * scale;
    let at = |x: f32, y: f32, dx: f32, dy: f32| origin + vec2(x + dx, y + dy) * scale;
    let stroke = Stroke::new(9.0 * scale, IVORY);

    for (dx, dy) in [(0.0, 0.0), (39.0, 11.0)] {
        let curves = [
            [(45.0, 30.0), (39.0, 23.0), (21.0, 22.0), (19.0, 34.0)],
            [(19.0, 34.0), (17.0, 46.0), (45.0, 40.0), (45.0, 54.0)],
            [(45.0, 54.0), (45.0, 67.0), (24.0, 69.0), (16.0, 61.0)],
        ];
        for c in curves {
            painter.add(CubicBezierShape::from_points_stroke(
                c.map(|(x, y)| at(x, y, dx, dy)),
                false,
                Color32::TRANSPARENT,
                stroke,
            ));
        }
    }
    // The blue index mark at the start of the first S.
    painter.add(CubicBezierShape::from_points_stroke(
        [
            at(45.0, 30.0, 0.0, 0.0),
            at(44.1, 28.95, 0.0, 0.0),
            at(42.93, 28.03, 0.0, 0.0),
            at(41.59, 27.27, 0.0, 0.0),
        ],
        false,
        Color32::TRANSPARENT,
        Stroke::new(9.0 * scale, BLUE),
    ));
}

#[derive(Clone, Copy)]
pub enum Icon {
    Photos,
    Unsorted,
    Keep,
    Remove,
}

/// Small line icons, drawn in a square `rect`.
pub fn paint_icon(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width();
    let p = |x: f32, y: f32| -> Pos2 { rect.min + vec2(x, y) * s };
    let stroke = Stroke::new(1.6, color);
    let frame = Rect::from_min_max(p(0.1, 0.12), p(0.9, 0.88));

    match icon {
        Icon::Photos => {
            painter.rect_stroke(frame, 3.0, stroke, egui::StrokeKind::Inside);
            painter.line(
                vec![
                    p(0.18, 0.78),
                    p(0.42, 0.5),
                    p(0.58, 0.66),
                    p(0.68, 0.56),
                    p(0.82, 0.72),
                ],
                stroke,
            );
            painter.circle_filled(p(0.66, 0.34), s * 0.07, color);
        }
        Icon::Unsorted => {
            painter.circle_stroke(rect.center(), s * 0.38, stroke);
            painter.line_segment([p(0.5, 0.3), p(0.5, 0.52)], stroke);
            painter.line_segment([p(0.5, 0.52), p(0.64, 0.62)], stroke);
        }
        Icon::Keep => {
            painter.rect_stroke(frame, 3.0, stroke, egui::StrokeKind::Inside);
            painter.line(vec![p(0.3, 0.5), p(0.45, 0.65), p(0.72, 0.36)], stroke);
        }
        Icon::Remove => paint_trash(painter, rect, stroke),
    }
}

fn paint_trash(painter: &Painter, rect: Rect, stroke: Stroke) {
    let s = rect.width();
    let p = |x: f32, y: f32| -> Pos2 { rect.min + vec2(x, y) * s };
    painter.line_segment([p(0.14, 0.24), p(0.86, 0.24)], stroke);
    painter.line(
        vec![p(0.38, 0.24), p(0.38, 0.12), p(0.62, 0.12), p(0.62, 0.24)],
        stroke,
    );
    painter.line(
        vec![p(0.22, 0.24), p(0.27, 0.9), p(0.73, 0.9), p(0.78, 0.24)],
        stroke,
    );
    painter.line_segment([p(0.42, 0.4), p(0.42, 0.74)], stroke);
    painter.line_segment([p(0.58, 0.4), p(0.58, 0.74)], stroke);
}

/// A round status badge with a check (keep) or a cross (remove).
pub fn paint_decision_badge(painter: &Painter, center: Pos2, radius: f32, keep: bool) {
    let fill = if keep { KEEP } else { REMOVE };
    painter.circle(center, radius, fill, Stroke::new(1.5, CHARCOAL));
    let stroke = Stroke::new(radius * 0.22, CHARCOAL);
    let r = radius * 0.42;
    if keep {
        painter.line(
            vec![
                center + vec2(-r, 0.0),
                center + vec2(-r * 0.25, r * 0.7),
                center + vec2(r, -r * 0.6),
            ],
            stroke,
        );
    } else {
        painter.line_segment([center + vec2(-r, -r), center + vec2(r, r)], stroke);
        painter.line_segment([center + vec2(-r, r), center + vec2(r, -r)], stroke);
    }
}
