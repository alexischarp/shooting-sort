//! The panels and dialogs of the window, drawn from the app state.

mod dialogs;
mod empty_state;
mod filmstrip;
mod gallery;
mod photo_view;
mod sidebar;
mod thumbnail;
mod top_bar;

use eframe::egui::{RichText, Ui};

use crate::ui::theme;

/// Shown in place of the photos when the folder or the list has none.
fn empty_list(ui: &mut Ui, folder_is_empty: bool) {
    let message = if folder_is_empty {
        "No JPEG or RAW files in this folder."
    } else {
        "No photos in this list."
    };
    ui.centered_and_justified(|ui| {
        ui.label(RichText::new(message).size(16.0).color(theme::TEXT_MUTED));
    });
}
