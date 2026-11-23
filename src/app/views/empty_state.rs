use eframe::egui::{self, Align2, FontFamily, FontId, Rect, Ui, vec2};

use crate::app::ShootingSortApp;
use crate::ui::painting;
use crate::ui::theme;
use crate::ui::widgets::{ButtonKind, button};

impl ShootingSortApp {
    /// Shown before a folder is open.
    pub(in crate::app) fn empty_state(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, theme::SURFACE);
        let center = rect.center();
        let logo = Rect::from_center_size(center - vec2(0.0, 110.0), vec2(120.0, 120.0));
        ui.painter()
            .rect_filled(logo.expand(6.0), 28.0, theme::CHARCOAL);
        painting::paint_logo(ui.painter(), logo.shrink(10.0));

        let painter = ui.painter();
        painter.text(
            center + vec2(0.0, -10.0),
            Align2::CENTER_CENTER,
            "Open a shoot folder",
            FontId::new(24.0, FontFamily::Proportional),
            theme::IVORY,
        );
        painter.text(
            center + vec2(0.0, 24.0),
            Align2::CENTER_CENTER,
            "Review the JPEGs, mark Keep or To Remove, then sort RAW and JPEG files.",
            FontId::new(14.0, FontFamily::Proportional),
            theme::TEXT_MUTED,
        );

        let button_rect = Rect::from_center_size(center + vec2(0.0, 80.0), vec2(150.0, 40.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(button_rect));
        if button(
            &mut child,
            "Open Folder",
            None,
            ButtonKind::Primary,
            button_rect.size(),
            true,
        )
        .clicked()
        {
            self.open_folder_dialog();
        }
        ui.painter().text(
            center + vec2(0.0, 124.0),
            Align2::CENTER_CENTER,
            "or drop a folder on this window  ·  ⌘O",
            FontId::new(12.0, FontFamily::Proportional),
            theme::TEXT_FAINT,
        );
    }
}
