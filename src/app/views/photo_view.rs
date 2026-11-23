use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontFamily, FontId, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2, pos2, vec2,
};

use crate::app::ShootingSortApp;
use crate::library::Decision;
use crate::platform::file_name;
use crate::ui::painting;
use crate::ui::theme;
use crate::ui::widgets::{ButtonKind, button};
use crate::zoom;

impl ShootingSortApp {
    /// The current photo with its name, formats and decision buttons.
    pub(in crate::app) fn main_view(&mut self, ui: &mut Ui) {
        let Some(cur) = self.culling.current else {
            super::empty_list(ui, self.culling.photos.is_empty());
            return;
        };

        // Header: file name, formats, decision buttons.
        let decision = self.culling.photos[cur].decision;
        let mut clicked = None;
        ui.horizontal(|ui| {
            let photo = &self.culling.photos[cur];
            ui.label(RichText::new(&photo.stem).size(24.0).color(theme::IVORY));
            ui.add_space(6.0);
            let (sep, _) = ui.allocate_exact_size(vec2(1.0, 22.0), Sense::hover());
            ui.painter().rect_filled(sep, 0.0, theme::BORDER);
            ui.add_space(6.0);
            let format_color = if photo.raw.is_some() && photo.jpeg.is_some() {
                theme::TEXT_MUTED
            } else {
                theme::BLUE
            };
            ui.label(
                RichText::new(photo.format_label())
                    .size(14.0)
                    .color(format_color),
            )
            .on_hover_text(
                [&photo.jpeg, &photo.raw]
                    .into_iter()
                    .flatten()
                    .map(|p| file_name(p))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );

            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let size = vec2(170.0, 40.0);
                let remove_on = decision == Some(Decision::Remove);
                let keep_on = decision == Some(Decision::Keep);
                if button(
                    ui,
                    "To Remove",
                    Some("X"),
                    ButtonKind::Toggle {
                        on: remove_on,
                        color: theme::REMOVE,
                    },
                    size,
                    true,
                )
                .clicked()
                {
                    clicked = Some(if remove_on {
                        None
                    } else {
                        Some(Decision::Remove)
                    });
                }
                if button(
                    ui,
                    "Keep",
                    Some("K"),
                    ButtonKind::Toggle {
                        on: keep_on,
                        color: theme::KEEP,
                    },
                    size,
                    true,
                )
                .clicked()
                {
                    clicked = Some(if keep_on { None } else { Some(Decision::Keep) });
                }
            });
        });
        if let Some(decision) = clicked {
            self.set_decision(decision, decision.is_some());
        }

        ui.add_space(14.0);

        self.photo_view(ui, cur);
    }

    /// The photo itself, fitted or zoomed.
    fn photo_view(&mut self, ui: &mut Ui, cur: usize) {
        let area = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(area, Sense::click_and_drag());
        let painter = ui.painter_at(area);
        painter.rect_filled(area, theme::RADIUS, theme::CHARCOAL);

        let Some(base) = self.images.preview(cur).or_else(|| self.images.thumb(cur)) else {
            if let Some(error) = self.images.error(cur) {
                painter.text(
                    area.center(),
                    Align2::CENTER_CENTER,
                    format!("Could not show this photo: {error}"),
                    FontId::new(14.0, FontFamily::Proportional),
                    theme::TEXT_MUTED,
                );
            } else {
                let spinner = Rect::from_center_size(area.center(), Vec2::splat(28.0));
                ui.put(spinner, egui::Spinner::new().color(theme::TEXT_FAINT));
            }
            return;
        };
        // Texture handles are cheap reference-counted clones.
        let base = base.clone();
        let base_width = base.size_vec2().x;
        let view = area.shrink(1.0);
        let fit = zoom::fit(base.size_vec2(), view);
        let ppp = ui.ctx().pixels_per_point();
        let original_width = self.images.original_width(cur).unwrap_or(base_width);
        // Zoom factor (relative to fit) that shows one photo pixel per screen pixel.
        let actual_size = (original_width / (fit.width() * ppp)).max(1.0);

        self.handle_zoom_input(ui, &response, fit, view, actual_size);

        let mut rect = self.zoom.map_or(fit, |z| z.rect(fit, view));
        // Align to screen pixels, so 100 % is sharp.
        rect = rect.translate(((rect.min.to_vec2() * ppp).round() / ppp) - rect.min.to_vec2());

        // Use the full-size image only where the preview has too few pixels.
        let physical_width = rect.width() * ppp;
        let full = self
            .images
            .full(cur)
            .filter(|_| physical_width > base_width);
        let texture = full.unwrap_or(&base);
        let zoomed = self.zoom.is_some();
        let corner = if zoomed {
            CornerRadius::ZERO
        } else {
            CornerRadius::same(theme::RADIUS - 2)
        };
        painter.add(
            egui::epaint::RectShape::filled(rect, corner, Color32::WHITE)
                .with_texture(texture.id(), zoom::FULL_UV),
        );

        let shown = rect.intersect(view);
        if let Some(decision) = self.culling.photos[cur].decision {
            let keep = decision == Decision::Keep;
            let color = if keep { theme::KEEP } else { theme::REMOVE };
            painter.rect_stroke(shown, corner, Stroke::new(3.0, color), StrokeKind::Inside);
            painting::paint_decision_badge(
                &painter,
                shown.right_top() + vec2(-26.0, 26.0),
                14.0,
                keep,
            );
        }

        if zoomed || response.hovered() {
            let percent = (physical_width / original_width * 100.0).round();
            let mut label = if zoomed {
                format!("{percent} %")
            } else {
                format!("Fit · {percent} %")
            };
            let photo = &self.culling.photos[cur];
            if photo.jpeg.is_none() {
                label.push_str(" · embedded RAW preview");
            } else if zoomed && full.is_none() && physical_width > base_width {
                label.push_str(" · loading full size…");
            }
            let font = FontId::new(12.0, FontFamily::Proportional);
            let galley = painter.layout_no_wrap(label, font, theme::IVORY);
            let pill = Rect::from_min_size(
                pos2(
                    view.left() + 12.0,
                    view.bottom() - 12.0 - galley.size().y - 8.0,
                ),
                galley.size() + vec2(16.0, 8.0),
            );
            painter.rect_filled(pill, 6.0, theme::CHARCOAL.gamma_multiply(0.8));
            painter.galley(pill.min + vec2(8.0, 4.0), galley, theme::IVORY);
        }
    }
}
