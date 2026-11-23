//! Colours, text styles and panel frames from the Shooting Sort visual identity.

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, vec2,
};

pub const CHARCOAL: Color32 = Color32::from_rgb(0x17, 0x19, 0x1C);
pub const GRAPHITE: Color32 = Color32::from_rgb(0x2B, 0x2E, 0x33);
pub const IVORY: Color32 = Color32::from_rgb(0xF3, 0xF1, 0xEB);
pub const BLUE: Color32 = Color32::from_rgb(0x7E, 0xA8, 0xFF);

/// Slightly lighter than charcoal, for panels sitting on the window background.
pub const SURFACE: Color32 = Color32::from_rgb(0x1E, 0x20, 0x24);
/// Background of the top bar.
pub const CHARCOAL_TOP: Color32 = Color32::from_rgb(0x1A, 0x1C, 0x1F);
pub const BORDER: Color32 = Color32::from_rgb(0x33, 0x36, 0x3C);
pub const GRAPHITE_HOVER: Color32 = Color32::from_rgb(0x36, 0x3A, 0x40);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0xA3, 0xA2, 0x9D);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x6C, 0x6D, 0x6E);
/// Status colours: not part of the brand palette, used only for decisions.
pub const KEEP: Color32 = Color32::from_rgb(0x8C, 0xCF, 0x9F);
pub const REMOVE: Color32 = Color32::from_rgb(0xE8, 0x7D, 0x6E);

pub const RADIUS: u8 = 10;

pub fn apply(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::dark());
    ctx.global_style_mut(|style| {
        style.text_styles = [
            (
                TextStyle::Small,
                FontId::new(11.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Heading,
                FontId::new(22.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(12.0, FontFamily::Monospace),
            ),
        ]
        .into();

        let v = &mut style.visuals;
        v.panel_fill = CHARCOAL;
        v.window_fill = SURFACE;
        v.window_stroke = Stroke::new(1.0, BORDER);
        v.window_corner_radius = CornerRadius::same(RADIUS + 2);
        v.extreme_bg_color = CHARCOAL;
        v.override_text_color = Some(IVORY);
        v.selection.bg_fill = BLUE;
        v.selection.stroke = Stroke::new(1.0, CHARCOAL);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        for w in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
        ] {
            w.corner_radius = CornerRadius::same(8);
            w.fg_stroke = Stroke::new(1.0, IVORY);
            w.bg_stroke = Stroke::NONE;
        }
        v.widgets.inactive.weak_bg_fill = GRAPHITE;
        v.widgets.inactive.bg_fill = GRAPHITE;
        v.widgets.hovered.weak_bg_fill = GRAPHITE_HOVER;
        v.widgets.hovered.bg_fill = GRAPHITE_HOVER;
        v.widgets.active.weak_bg_fill = BORDER;
        v.widgets.active.bg_fill = BORDER;

        style.spacing.button_padding = vec2(16.0, 8.0);
        style.spacing.item_spacing = vec2(10.0, 8.0);
    });
}

/// Frame of the top bar, the sidebar and the filmstrip: a fill and a thin border.
pub fn panel_frame(fill: Color32, margin: Margin) -> egui::Frame {
    egui::Frame::new()
        .fill(fill)
        .inner_margin(margin)
        .stroke(Stroke::new(1.0, BORDER))
}
