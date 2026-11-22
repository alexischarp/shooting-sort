//! Zoom and pan of the main photo.

use eframe::egui::{Pos2, Rect, Vec2, vec2};

/// A zoomed view. "Fit to the view" is represented by no `Zoom` at all.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoom {
    /// Magnification relative to the fitted photo (1.0 = fit).
    pub factor: f32,
    /// Point of the photo at the centre of the view, from (0, 0) top-left to (1, 1) bottom-right.
    pub center: Vec2,
}

impl Zoom {
    /// Screen rectangle of the whole photo. `fit` is the photo fitted in `view`.
    pub fn rect(&self, fit: Rect, view: Rect) -> Rect {
        let size = fit.size() * self.factor;
        Rect::from_min_size(view.center() - self.center * size, size)
    }

    /// Moves the photo on screen by `delta` points.
    pub fn pan(&mut self, delta: Vec2, fit: Rect, view: Rect) {
        self.center -= delta / (fit.size() * self.factor);
        self.clamp(fit, view);
    }

    /// Keeps the photo covering the view: no empty band on a side the photo overflows.
    fn clamp(&mut self, fit: Rect, view: Rect) {
        let size = fit.size() * self.factor;
        let axis = |center: f32, size: f32, view: f32| {
            if size > view {
                let half = view / size / 2.0;
                center.clamp(half, 1.0 - half)
            } else {
                0.5
            }
        };
        self.center = vec2(
            axis(self.center.x, size.x, view.width()),
            axis(self.center.y, size.y, view.height()),
        );
    }
}

/// Zooms to `factor` (relative to fit), keeping the photo point under `anchor` in place.
/// Returns `None` (fit) when `factor` is 1 or less.
pub fn zoom_to(current: Option<Zoom>, factor: f32, anchor: Pos2, fit: Rect, view: Rect) -> Option<Zoom> {
    if factor <= 1.001 {
        return None;
    }
    let current_rect = current.map_or(fit, |z| z.rect(fit, view));
    let point = ((anchor - current_rect.min) / current_rect.size()).clamp(Vec2::ZERO, Vec2::splat(1.0));
    let size = fit.size() * factor;
    let min = anchor - point * size;
    let mut zoom = Zoom {
        factor,
        center: (view.center() - min) / size,
    };
    zoom.clamp(fit, view);
    Some(zoom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    fn view() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 200.0))
    }

    /// A 3:2 photo fills the 3:2 view exactly.
    fn fit() -> Rect {
        view()
    }

    #[test]
    fn zoom_keeps_the_point_under_the_anchor() {
        let anchor = pos2(75.0, 50.0); // A quarter of the photo, from the top-left.
        let zoom = zoom_to(None, 2.0, anchor, fit(), view()).unwrap();
        let rect = zoom.rect(fit(), view());
        let point = (anchor - rect.min) / rect.size();
        assert!((point - vec2(0.25, 0.25)).length() < 1e-4);
    }

    #[test]
    fn zoom_at_or_below_fit_returns_to_fit() {
        assert_eq!(zoom_to(None, 1.0, pos2(10.0, 10.0), fit(), view()), None);
    }

    #[test]
    fn pan_stops_at_the_photo_edge() {
        let mut zoom = zoom_to(None, 2.0, view().center(), fit(), view()).unwrap();
        zoom.pan(vec2(10_000.0, 0.0), fit(), view());
        let rect = zoom.rect(fit(), view());
        assert!((rect.left() - view().left()).abs() < 1e-3);
    }

    #[test]
    fn photo_narrower_than_view_stays_centred() {
        // A portrait photo fitted in a landscape view.
        let fit = Rect::from_center_size(view().center(), vec2(100.0, 200.0));
        let zoom = zoom_to(None, 1.2, pos2(150.0, 20.0), fit, view()).unwrap();
        assert_eq!(zoom.center.x, 0.5);
    }
}
