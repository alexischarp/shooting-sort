use eframe::egui::{self, Align, RichText, Sense, Ui, vec2};

use crate::app::{Dialog, ShootingSortApp};
use crate::platform::file_name;
use crate::ui::painting;
use crate::ui::theme;
use crate::ui::widgets::{ButtonKind, button};

impl ShootingSortApp {
    pub(in crate::app) fn top_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let (logo_rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
            painting::paint_logo(ui.painter(), logo_rect);
            ui.label(
                RichText::new("Shooting Sort")
                    .size(21.0)
                    .color(theme::IVORY),
            );

            if let Some(folder) = &self.folder {
                ui.add_space(8.0);
                let (sep, _) = ui.allocate_exact_size(vec2(1.0, 22.0), Sense::hover());
                ui.painter().rect_filled(sep, 0.0, theme::BORDER);
                ui.add_space(8.0);
                ui.label(RichText::new(file_name(folder)).color(theme::TEXT_MUTED))
                    .on_hover_text(folder.display().to_string());
            }

            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if self.folder.is_some() {
                    let any_decision = self.culling.photos.iter().any(|p| p.decision.is_some());
                    let sort = button(
                        ui,
                        "Sort Files",
                        None,
                        ButtonKind::Primary,
                        vec2(118.0, 38.0),
                        any_decision,
                    );
                    if sort.clicked() {
                        self.dialog = Some(Dialog::ConfirmSort);
                    }
                    sort.on_disabled_hover_text("Mark photos as Keep or To Remove first");
                }
                if button(
                    ui,
                    "Open Folder",
                    None,
                    ButtonKind::Secondary,
                    vec2(128.0, 38.0),
                    true,
                )
                .clicked()
                {
                    self.open_folder_dialog();
                }
                if self.folder.is_some() {
                    let kind = ButtonKind::Toggle {
                        on: self.gallery,
                        color: theme::BLUE,
                    };
                    if button(ui, "Gallery", Some("G"), kind, vec2(128.0, 38.0), true).clicked() {
                        self.set_gallery(!self.gallery);
                    }
                }
            });
        });
    }
}
