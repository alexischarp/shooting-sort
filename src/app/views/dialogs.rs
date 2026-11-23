use eframe::egui::{self, Align, Key, Margin, RichText, Stroke, vec2};

use crate::app::{Dialog, ShootingSortApp};
use crate::library::{Decision, Photo};
use crate::platform::{file_name, reveal};
use crate::sorter;
use crate::ui::theme;
use crate::ui::widgets::{ButtonKind, button};

impl ShootingSortApp {
    /// The open dialog, if any: sort confirmation, sort report or error.
    pub(in crate::app) fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &self.dialog else { return };
        let mut close = false;
        let mut sort = false;

        let modal_frame = egui::Frame::new()
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(14)
            .inner_margin(Margin::same(24));

        egui::Modal::new(egui::Id::unique("dialog")).frame(modal_frame).show(ctx, |ui| {
            ui.set_width(440.0);
            match dialog {
                Dialog::ConfirmSort => {
                    let photos = &self.culling.photos;
                    let kept: Vec<&Photo> = photos.iter().filter(|p| p.decision == Some(Decision::Keep)).collect();
                    let removed: Vec<&Photo> = photos.iter().filter(|p| p.decision == Some(Decision::Remove)).collect();
                    let unsorted = photos.len() - kept.len() - removed.len();
                    let raws = kept.iter().filter(|p| p.raw.is_some()).count();
                    let jpegs = kept.iter().filter(|p| p.jpeg.is_some()).count();

                    ui.label(RichText::new("Sort files?").size(20.0));
                    ui.add_space(10.0);
                    ui.label(format!(
                        "{} kept photos → sorted/raw ({raws} files) and sorted/jpeg ({jpegs} files)",
                        kept.len()
                    ));
                    ui.label(format!("{} photos → to-remove", removed.len()));
                    if unsorted > 0 {
                        ui.label(
                            RichText::new(format!("{unsorted} unsorted photos stay in the folder"))
                                .color(theme::TEXT_MUTED),
                        );
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Files are moved, never deleted or overwritten.")
                            .size(12.0)
                            .color(theme::TEXT_FAINT),
                    );
                    ui.add_space(18.0);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if button(ui, "Sort Files", None, ButtonKind::Primary, vec2(118.0, 38.0), true).clicked() {
                            sort = true;
                        }
                        if button(ui, "Cancel", None, ButtonKind::Secondary, vec2(96.0, 38.0), true).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::SortDone(report) => {
                    ui.label(RichText::new("Sorting done").size(20.0));
                    ui.add_space(10.0);
                    ui.label(format!("{} files moved.", report.moved));
                    if !report.failures.is_empty() {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "{} files could not be moved and stay in the folder:",
                                report.failures.len()
                            ))
                            .color(theme::REMOVE),
                        );
                        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                            for (path, error) in &report.failures {
                                ui.label(
                                    RichText::new(format!("{}: {error}", file_name(path)))
                                        .size(12.0)
                                        .color(theme::TEXT_MUTED),
                                );
                            }
                        });
                    }
                    ui.add_space(18.0);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if button(ui, "Done", None, ButtonKind::Primary, vec2(96.0, 38.0), true).clicked() {
                            close = true;
                        }
                        if let Some(folder) = &self.folder {
                            let sorted = folder.join(sorter::SORTED_DIR);
                            if sorted.is_dir()
                                && button(ui, "Show in Finder", None, ButtonKind::Secondary, vec2(140.0, 38.0), true)
                                    .clicked()
                            {
                                reveal(&sorted);
                            }
                        }
                    });
                }
                Dialog::Error(message) => {
                    ui.label(RichText::new("Something went wrong").size(20.0));
                    ui.add_space(10.0);
                    ui.label(message);
                    ui.add_space(18.0);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if button(ui, "OK", None, ButtonKind::Primary, vec2(96.0, 38.0), true).clicked() {
                            close = true;
                        }
                    });
                }
            }
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                close = true;
            }
        });

        if close {
            self.dialog = None;
        }
        if sort {
            self.dialog = None;
            self.run_sort();
        }
    }
}
