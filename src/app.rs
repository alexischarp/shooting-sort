use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontFamily, FontId, Key, Margin, Rect, RichText,
    Sense, Stroke, StrokeKind, TextureFilter, TextureHandle, TextureOptions, Ui, Vec2, pos2, vec2,
};

use crate::library::{self, Decision, Photo};
use crate::loader::Loader;
use crate::session;
use crate::sorter::{self, SortReport};
use crate::theme::{self, Icon};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Unsorted,
    Keep,
    Remove,
}

impl Filter {
    fn matches(self, photo: &Photo) -> bool {
        match self {
            Filter::All => true,
            Filter::Unsorted => photo.decision.is_none(),
            Filter::Keep => photo.decision == Some(Decision::Keep),
            Filter::Remove => photo.decision == Some(Decision::Remove),
        }
    }
}

enum Dialog {
    ConfirmSort,
    SortDone(SortReport),
    Error(String),
}

pub struct ShootingSortApp {
    loader: Loader,
    /// Increases each time a folder is (re)loaded, so stale decode results are ignored.
    generation: u64,
    folder: Option<PathBuf>,
    photos: Vec<Photo>,
    filter: Filter,
    /// Index into `photos`.
    current: Option<usize>,
    thumbs: HashMap<usize, TextureHandle>,
    thumbs_requested: HashSet<usize>,
    previews: HashMap<usize, TextureHandle>,
    /// Photos whose preview is wanted (current and neighbours); others are evicted.
    previews_wanted: Vec<usize>,
    previews_requested_for: Option<usize>,
    load_errors: HashMap<usize, String>,
    scroll_filmstrip: bool,
    dialog: Option<Dialog>,
    save_error: Option<String>,
}

impl ShootingSortApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut app = Self {
            loader: Loader::new(cc.egui_ctx.clone()),
            generation: 0,
            folder: None,
            photos: Vec::new(),
            filter: Filter::All,
            current: None,
            thumbs: HashMap::new(),
            thumbs_requested: HashSet::new(),
            previews: HashMap::new(),
            previews_wanted: Vec::new(),
            previews_requested_for: None,
            load_errors: HashMap::new(),
            scroll_filmstrip: false,
            dialog: None,
            save_error: None,
        };
        // `shooting-sort <folder>` opens that folder directly.
        if let Some(folder) = std::env::args_os().nth(1) {
            app.load_folder(PathBuf::from(folder));
        }
        app
    }

    fn open_folder_dialog(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Open a shoot folder");
        if let Some(folder) = &self.folder {
            dialog = dialog.set_directory(folder);
        }
        if let Some(folder) = dialog.pick_folder() {
            self.load_folder(folder);
        }
    }

    fn load_folder(&mut self, folder: PathBuf) {
        let mut photos = match library::scan(&folder) {
            Ok(photos) => photos,
            Err(e) => {
                self.dialog = Some(Dialog::Error(format!(
                    "Could not read {}:\n{e}",
                    folder.display()
                )));
                return;
            }
        };
        session::load(&folder, &mut photos);

        self.generation += 1;
        self.loader.reset(self.generation);
        self.thumbs.clear();
        self.thumbs_requested.clear();
        self.previews.clear();
        self.previews_wanted.clear();
        self.previews_requested_for = None;
        self.load_errors.clear();
        self.save_error = None;
        self.filter = Filter::All;
        // Resume at the first photo not handled yet.
        self.current = photos
            .iter()
            .position(|p| p.decision.is_none())
            .or(if photos.is_empty() { None } else { Some(0) });
        self.photos = photos;
        self.folder = Some(folder);
        self.scroll_filmstrip = true;
    }

    fn visible(&self) -> Vec<usize> {
        (0..self.photos.len())
            .filter(|&i| self.filter.matches(&self.photos[i]))
            .collect()
    }

    fn set_filter(&mut self, filter: Filter) {
        self.filter = filter;
        let visible = self.visible();
        if !self.current.is_some_and(|c| visible.contains(&c)) {
            self.current = visible.first().copied();
        }
        self.scroll_filmstrip = true;
    }

    fn step(&mut self, forward: bool) {
        let visible = self.visible();
        let Some(cur) = self.current else {
            self.current = visible.first().copied();
            return;
        };
        let next = if forward {
            visible.iter().copied().find(|&i| i > cur)
        } else {
            visible.iter().rev().copied().find(|&i| i < cur)
        };
        if let Some(next) = next {
            self.current = Some(next);
            self.scroll_filmstrip = true;
        }
    }

    fn set_decision(&mut self, decision: Option<Decision>, advance: bool) {
        let Some(cur) = self.current else { return };
        self.photos[cur].decision = decision;
        self.save_session();

        let visible = self.visible();
        if advance || !visible.contains(&cur) {
            // Move to the next photo in the list, or the previous one at the end.
            self.current = visible
                .iter()
                .copied()
                .find(|&i| i > cur)
                .or_else(|| visible.iter().rev().copied().find(|&i| i < cur))
                .or(visible.contains(&cur).then_some(cur));
            self.scroll_filmstrip = true;
        }
    }

    fn save_session(&mut self) {
        if let Some(folder) = &self.folder {
            self.save_error = session::save(folder, &self.photos)
                .err()
                .map(|e| format!("Could not save selections: {e}"));
        }
    }

    fn run_sort(&mut self) {
        let Some(folder) = self.folder.clone() else { return };
        let report = sorter::sort(&folder, &self.photos);
        self.load_folder(folder);
        // Forget decisions for photos that were moved away.
        self.save_session();
        self.dialog = Some(Dialog::SortDone(report));
    }

    fn receive_images(&mut self, ctx: &egui::Context) {
        let preview_options = TextureOptions {
            mipmap_mode: Some(TextureFilter::Linear),
            ..TextureOptions::LINEAR
        };
        while let Some(loaded) = self.loader.try_recv() {
            if loaded.generation != self.generation {
                continue;
            }
            let i = loaded.index;
            match loaded.result {
                Ok(images) => {
                    self.thumbs.entry(i).or_insert_with(|| {
                        ctx.load_texture(format!("thumb-{i}"), images.thumb, TextureOptions::LINEAR)
                    });
                    if let Some(preview) = images.preview
                        && self.previews_wanted.contains(&i)
                    {
                        self.previews.insert(
                            i,
                            ctx.load_texture(format!("preview-{i}"), preview, preview_options),
                        );
                    }
                }
                Err(e) => {
                    self.load_errors.insert(i, e);
                }
            }
        }
    }

    /// Keeps the current photo and its neighbours decoded at full size.
    fn update_preview_requests(&mut self) {
        if self.previews_requested_for == self.current {
            return;
        }
        self.previews_requested_for = self.current;
        let Some(cur) = self.current else {
            self.previews_wanted.clear();
            self.previews.clear();
            return;
        };
        let visible = self.visible();
        let pos = visible.iter().position(|&i| i == cur).unwrap_or(0);
        let mut wanted = vec![cur];
        for offset in [1isize, -1, 2] {
            if let Some(&i) = visible.get(pos.wrapping_add_signed(offset)) {
                wanted.push(i);
            }
        }
        self.previews.retain(|i, _| wanted.contains(i));
        let jobs = wanted
            .iter()
            .filter(|i| !self.previews.contains_key(i) && !self.load_errors.contains_key(i))
            .map(|&i| (i, self.photos[i].display_source().to_owned()))
            .collect();
        self.loader.set_previews(jobs);
        self.previews_wanted = wanted;
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        let open = ctx.input(|i| i.modifiers.command && i.key_pressed(Key::O));
        if open {
            self.open_folder_dialog();
            return;
        }
        if self.current.is_none() {
            return;
        }
        let (left, right, keep, remove, clear) = ctx.input(|i| {
            (
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::K),
                i.key_pressed(Key::X),
                i.key_pressed(Key::U),
            )
        });
        if left {
            self.step(false);
        }
        if right {
            self.step(true);
        }
        if keep {
            self.set_decision(Some(Decision::Keep), true);
        }
        if remove {
            self.set_decision(Some(Decision::Remove), true);
        }
        if clear {
            self.set_decision(None, false);
        }
    }

    fn handle_dropped_folder(&mut self, ctx: &egui::Context) {
        let dropped: Option<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_owned())
                .find(|p| p.is_dir())
        });
        if let Some(folder) = dropped {
            self.load_folder(folder);
        }
    }
}

impl eframe::App for ShootingSortApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.receive_images(&ctx);
        self.handle_dropped_folder(&ctx);
        if self.dialog.is_none() {
            self.handle_keys(&ctx);
        }
        self.update_preview_requests();

        egui::Panel::top("top_bar")
            .frame(
                egui::Frame::new()
                    .fill(CHARCOAL_TOP)
                    .inner_margin(Margin::symmetric(20, 14))
                    .stroke(Stroke::new(1.0, theme::BORDER)),
            )
            .show(ui, |ui| self.top_bar(ui));

        if self.folder.is_none() {
            egui::CentralPanel::no_frame().show(ui, |ui| self.empty_state(ui));
        } else {
            egui::Panel::left("sidebar")
                .resizable(false)
                .exact_size(232.0)
                .frame(
                    egui::Frame::new()
                        .fill(theme::CHARCOAL)
                        .inner_margin(Margin::same(14))
                        .stroke(Stroke::new(1.0, theme::BORDER)),
                )
                .show(ui, |ui| self.sidebar(ui));

            egui::Panel::bottom("filmstrip")
                .exact_size(132.0)
                .frame(
                    egui::Frame::new()
                        .fill(theme::CHARCOAL)
                        .inner_margin(Margin::symmetric(16, 14))
                        .stroke(Stroke::new(1.0, theme::BORDER)),
                )
                .show(ui, |ui| self.filmstrip(ui));

            egui::CentralPanel::no_frame()
                .frame(
                    egui::Frame::new()
                        .fill(theme::SURFACE)
                        .inner_margin(Margin::same(20)),
                )
                .show(ui, |ui| self.main_view(ui));
        }

        self.dialogs(&ctx);
    }
}

const CHARCOAL_TOP: Color32 = Color32::from_rgb(0x1A, 0x1C, 0x1F);

// ---- Widgets ---------------------------------------------------------------

#[derive(Clone, Copy)]
enum ButtonKind {
    Primary,
    Secondary,
    Toggle { on: bool, color: Color32 },
}

fn button(ui: &mut Ui, label: &str, hint: Option<&str>, kind: ButtonKind, size: Vec2, enabled: bool) -> egui::Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let hovered = enabled && response.hovered();
    let (fill, text, stroke) = match kind {
        ButtonKind::Primary => (
            if hovered { theme::BLUE.gamma_multiply(1.12) } else { theme::BLUE },
            theme::CHARCOAL,
            Stroke::NONE,
        ),
        ButtonKind::Secondary => (
            if hovered { theme::GRAPHITE_HOVER } else { theme::GRAPHITE },
            theme::IVORY,
            Stroke::new(1.0, theme::BORDER),
        ),
        ButtonKind::Toggle { on: true, color } => (
            color.gamma_multiply(0.22),
            color,
            Stroke::new(1.5, color),
        ),
        ButtonKind::Toggle { on: false, .. } => (
            if hovered { theme::GRAPHITE_HOVER } else { theme::GRAPHITE },
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
        let key_rect = Rect::from_center_size(pos2(rect.right() - 22.0, rect.center().y), vec2(20.0, 20.0));
        painter.rect_stroke(key_rect, 4.0, Stroke::new(1.0, text.gamma_multiply(0.5)), StrokeKind::Inside);
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

fn format_tag(painter: &egui::Painter, anchor: egui::Pos2, label: &str, color: Color32) -> Rect {
    let font = FontId::new(10.0, FontFamily::Monospace);
    let galley = painter.layout_no_wrap(label.to_owned(), font, color);
    let rect = Rect::from_min_size(anchor, galley.size() + vec2(8.0, 4.0));
    painter.rect_filled(rect, 4.0, theme::CHARCOAL.gamma_multiply(0.85));
    painter.galley(rect.min + vec2(4.0, 2.0), galley, color);
    rect
}

// ---- Layout ----------------------------------------------------------------

impl ShootingSortApp {
    fn top_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let (logo_rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
            theme::paint_logo(ui.painter(), logo_rect);
            ui.label(RichText::new("Shooting Sort").size(21.0).color(theme::IVORY));

            if let Some(folder) = &self.folder {
                ui.add_space(8.0);
                let (sep, _) = ui.allocate_exact_size(vec2(1.0, 22.0), Sense::hover());
                ui.painter().rect_filled(sep, 0.0, theme::BORDER);
                ui.add_space(8.0);
                let name = folder.file_name().map_or_else(
                    || folder.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                ui.label(RichText::new(name).color(theme::TEXT_MUTED))
                    .on_hover_text(folder.display().to_string());
            }

            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if self.folder.is_some() {
                    let any_decision = self.photos.iter().any(|p| p.decision.is_some());
                    let sort = button(ui, "Sort Files", None, ButtonKind::Primary, vec2(118.0, 38.0), any_decision);
                    if sort.clicked() {
                        self.dialog = Some(Dialog::ConfirmSort);
                    }
                    sort.on_disabled_hover_text("Mark photos as Keep or To Remove first");
                }
                if button(ui, "Open Folder", None, ButtonKind::Secondary, vec2(128.0, 38.0), true).clicked() {
                    self.open_folder_dialog();
                }
            });
        });
    }

    fn empty_state(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, theme::SURFACE);
        let center = rect.center();
        let logo = Rect::from_center_size(center - vec2(0.0, 110.0), vec2(120.0, 120.0));
        ui.painter().rect_filled(logo.expand(6.0), 28.0, theme::CHARCOAL);
        theme::paint_logo(ui.painter(), logo.shrink(10.0));

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
        if button(&mut child, "Open Folder", None, ButtonKind::Primary, button_rect.size(), true).clicked() {
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

    fn sidebar(&mut self, ui: &mut Ui) {
        let total = self.photos.len();
        let kept = self.photos.iter().filter(|p| p.decision == Some(Decision::Keep)).count();
        let removed = self.photos.iter().filter(|p| p.decision == Some(Decision::Remove)).count();
        let unsorted = total - kept - removed;

        ui.add_space(4.0);
        for (filter, icon, label, count) in [
            (Filter::All, Icon::Photos, "All Photos", total),
            (Filter::Unsorted, Icon::Unsorted, "Unsorted", unsorted),
            (Filter::Keep, Icon::Keep, "Keep", kept),
            (Filter::Remove, Icon::Remove, "To Remove", removed),
        ] {
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::click());
            let selected = self.filter == filter;
            let painter = ui.painter();
            if selected {
                painter.rect_filled(rect, 8.0, theme::GRAPHITE);
            } else if response.hovered() {
                painter.rect_filled(rect, 8.0, theme::GRAPHITE.gamma_multiply(0.5));
            }
            let color = if selected { theme::IVORY } else { theme::TEXT_MUTED };
            let icon_rect = Rect::from_center_size(pos2(rect.left() + 22.0, rect.center().y), Vec2::splat(18.0));
            theme::paint_icon(painter, icon_rect, icon, color);
            painter.text(
                pos2(rect.left() + 44.0, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::new(15.0, FontFamily::Proportional),
                color,
            );
            painter.text(
                pos2(rect.right() - 12.0, rect.center().y),
                Align2::RIGHT_CENTER,
                count.to_string(),
                FontId::new(12.0, FontFamily::Monospace),
                theme::TEXT_FAINT,
            );
            if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.set_filter(filter);
            }
            ui.add_space(2.0);
        }

        ui.add_space(20.0);
        ui.label(RichText::new("PROGRESS").size(11.0).color(theme::TEXT_FAINT));
        ui.add_space(2.0);
        ui.label(
            RichText::new(format!("{} of {total} handled", kept + removed)).color(theme::IVORY),
        );
        let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
        ui.painter().rect_filled(bar, 3.0, theme::GRAPHITE);
        if total > 0 {
            let mut filled = bar;
            filled.set_width(bar.width() * (kept + removed) as f32 / total as f32);
            ui.painter().rect_filled(filled, 3.0, theme::BLUE);
        }

        if let Some(error) = &self.save_error {
            ui.add_space(12.0);
            ui.label(RichText::new(error).size(12.0).color(theme::REMOVE));
        }

        ui.with_layout(egui::Layout::bottom_up(Align::LEFT), |ui| {
            for (key, action) in [("U", "Clear mark"), ("X", "To Remove"), ("K", "Keep"), ("← →", "Previous / next")]
            {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        vec2(40.0, 18.0),
                        egui::Label::new(RichText::new(key).monospace().size(11.0).color(theme::TEXT_MUTED)),
                    );
                    ui.label(RichText::new(action).size(12.0).color(theme::TEXT_FAINT));
                });
            }
            ui.label(RichText::new("SHORTCUTS").size(11.0).color(theme::TEXT_FAINT));
        });
    }

    fn main_view(&mut self, ui: &mut Ui) {
        let Some(cur) = self.current else {
            let message = if self.photos.is_empty() {
                "No JPEG or RAW files in this folder."
            } else {
                "No photos in this list."
            };
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(message).size(16.0).color(theme::TEXT_MUTED));
            });
            return;
        };

        // Header: file name, formats, decision buttons.
        let decision = self.photos[cur].decision;
        let mut clicked = None;
        ui.horizontal(|ui| {
            let photo = &self.photos[cur];
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
            ui.label(RichText::new(photo.format_label()).size(14.0).color(format_color))
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
                if button(ui, "To Remove", Some("X"), ButtonKind::Toggle { on: remove_on, color: theme::REMOVE }, size, true).clicked() {
                    clicked = Some(if remove_on { None } else { Some(Decision::Remove) });
                }
                if button(ui, "Keep", Some("K"), ButtonKind::Toggle { on: keep_on, color: theme::KEEP }, size, true).clicked() {
                    clicked = Some(if keep_on { None } else { Some(Decision::Keep) });
                }
            });
        });
        if let Some(decision) = clicked {
            self.set_decision(decision, decision.is_some());
        }

        ui.add_space(14.0);

        // The photo itself.
        let area = ui.available_rect_before_wrap();
        ui.allocate_rect(area, Sense::hover());
        let painter = ui.painter_at(area);
        painter.rect_filled(area, theme::RADIUS, theme::CHARCOAL);

        let texture = self.previews.get(&cur).or_else(|| self.thumbs.get(&cur));
        if let Some(texture) = texture {
            let rect = theme::fit(texture.size_vec2(), area.shrink(1.0));
            let corner = CornerRadius::same(theme::RADIUS - 2);
            painter.add(
                egui::epaint::RectShape::filled(rect, corner, Color32::WHITE)
                    .with_texture(texture.id(), theme::FULL_UV),
            );
            if let Some(decision) = self.photos.get(cur).and_then(|p| p.decision) {
                let keep = decision == Decision::Keep;
                let color = if keep { theme::KEEP } else { theme::REMOVE };
                painter.rect_stroke(rect, corner, Stroke::new(3.0, color), StrokeKind::Inside);
                theme::paint_decision_badge(&painter, rect.right_top() + vec2(-26.0, 26.0), 14.0, keep);
            }
        } else if let Some(error) = self.load_errors.get(&cur) {
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
    }

    fn filmstrip(&mut self, ui: &mut Ui) {
        let visible = self.visible();
        let height = ui.available_height();
        let cell = vec2(height * 1.5, height);
        let scroll_to_current = std::mem::take(&mut self.scroll_filmstrip);
        let mut clicked = None;

        egui::ScrollArea::horizontal()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for &i in &visible {
                        let (rect, response) = ui.allocate_exact_size(cell, Sense::click());
                        let is_current = self.current == Some(i);
                        if is_current && scroll_to_current {
                            ui.scroll_to_rect(rect, Some(Align::Center));
                        }
                        if response.clicked() {
                            clicked = Some(i);
                        }
                        if !ui.is_rect_visible(rect) {
                            continue;
                        }
                        self.paint_thumb(ui, rect, i, is_current, response.hovered());
                    }
                });
            });

        if let Some(i) = clicked {
            self.current = Some(i);
        }
    }

    fn paint_thumb(&mut self, ui: &Ui, rect: Rect, i: usize, is_current: bool, hovered: bool) {
        let photo = &self.photos[i];
        let painter = ui.painter_at(rect.expand(2.0));
        let corner = CornerRadius::same(8);
        painter.rect_filled(rect, corner, theme::GRAPHITE);

        if let Some(texture) = self.thumbs.get(&i) {
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
            painter.add(egui::epaint::RectShape::filled(rect, corner, tint).with_texture(texture.id(), uv));
        } else if self.load_errors.contains_key(&i) {
            painter.text(rect.center(), Align2::CENTER_CENTER, "!", FontId::proportional(18.0), theme::REMOVE);
        } else if self.thumbs_requested.insert(i) {
            self.loader.request_thumb(i, photo.display_source());
        }

        let tag_color = theme::IVORY.gamma_multiply(0.9);
        let tag = match (&photo.jpeg, &photo.raw) {
            (Some(_), Some(_)) => "RAW",
            (Some(_), None) => "JPEG only",
            _ => "RAW only",
        };
        let tag_color = if photo.jpeg.is_some() && photo.raw.is_some() { tag_color } else { theme::BLUE };
        format_tag(&painter, rect.left_bottom() + vec2(6.0, -22.0), tag, tag_color);

        if let Some(decision) = photo.decision {
            theme::paint_decision_badge(&painter, rect.right_top() + vec2(-14.0, 14.0), 9.0, decision == Decision::Keep);
        }

        if is_current {
            painter.rect_stroke(rect, corner, Stroke::new(3.0, theme::BLUE), StrokeKind::Outside);
        } else if hovered {
            painter.rect_stroke(rect, corner, Stroke::new(1.5, theme::IVORY.gamma_multiply(0.5)), StrokeKind::Outside);
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
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
                    let kept: Vec<&Photo> = self.photos.iter().filter(|p| p.decision == Some(Decision::Keep)).collect();
                    let removed: Vec<&Photo> = self.photos.iter().filter(|p| p.decision == Some(Decision::Remove)).collect();
                    let unsorted = self.photos.len() - kept.len() - removed.len();
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

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned())
}

/// Opens a folder in the system file manager.
fn reveal(path: &Path) {
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let program = "xdg-open";
    let _ = std::process::Command::new(program).arg(path).spawn();
}
