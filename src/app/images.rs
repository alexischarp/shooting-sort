//! Textures of the photos: thumbnails, previews and full-size images, decoded in the
//! background by the `Loader`.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use eframe::egui::{self, TextureFilter, TextureHandle, TextureOptions};

use crate::app::culling::Culling;
use crate::loader::{Loader, Request, Size};

pub struct ImageCache {
    loader: Loader,
    /// Increases each time a folder is (re)loaded, so stale decode results are ignored.
    generation: u64,
    thumbs: HashMap<usize, TextureHandle>,
    thumbs_requested: HashSet<usize>,
    previews: HashMap<usize, TextureHandle>,
    /// Photos whose preview is wanted (current and neighbours); others are evicted.
    previews_wanted: Vec<usize>,
    /// Full-resolution images, only while zoomed (current and next photo).
    fulls: HashMap<usize, TextureHandle>,
    fulls_wanted: Vec<usize>,
    /// The (current photo, zoomed) state the preview and full-size requests were made for.
    requested_for: Option<(Option<usize>, bool)>,
    /// Pixel size of each photo's largest image, used to compute 100 %.
    original_sizes: HashMap<usize, [u32; 2]>,
    errors: HashMap<usize, String>,
}

impl ImageCache {
    pub fn new(ctx: egui::Context) -> Self {
        Self {
            loader: Loader::new(ctx),
            generation: 0,
            thumbs: HashMap::new(),
            thumbs_requested: HashSet::new(),
            previews: HashMap::new(),
            previews_wanted: Vec::new(),
            fulls: HashMap::new(),
            fulls_wanted: Vec::new(),
            requested_for: None,
            original_sizes: HashMap::new(),
            errors: HashMap::new(),
        }
    }

    /// Forgets every image, for a new folder. Decodes still running are ignored.
    pub fn reset(&mut self) {
        self.generation += 1;
        self.loader.reset(self.generation);
        self.thumbs.clear();
        self.thumbs_requested.clear();
        self.previews.clear();
        self.previews_wanted.clear();
        self.fulls.clear();
        self.fulls_wanted.clear();
        self.requested_for = None;
        self.original_sizes.clear();
        self.errors.clear();
    }

    pub fn thumb(&self, index: usize) -> Option<&TextureHandle> {
        self.thumbs.get(&index)
    }

    pub fn preview(&self, index: usize) -> Option<&TextureHandle> {
        self.previews.get(&index)
    }

    pub fn full(&self, index: usize) -> Option<&TextureHandle> {
        self.fulls.get(&index)
    }

    pub fn error(&self, index: usize) -> Option<&str> {
        self.errors.get(&index).map(String::as_str)
    }

    /// Width in pixels of the photo's largest image, once one has been decoded.
    pub fn original_width(&self, index: usize) -> Option<f32> {
        self.original_sizes.get(&index).map(|s| s[0] as f32)
    }

    /// Asks for the thumbnail of a photo, once.
    pub fn request_thumb(&mut self, index: usize, path: &Path) {
        if self.thumbs_requested.insert(index) {
            self.loader.request_thumb(index, path);
        }
    }

    /// Turns the decoded images into textures.
    pub fn receive(&mut self, ctx: &egui::Context) {
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
                    let known = self.original_sizes.entry(i).or_insert(images.original_size);
                    if images.original_size[0] > known[0] {
                        *known = images.original_size;
                    }
                    self.thumbs.entry(i).or_insert_with(|| {
                        ctx.load_texture(format!("thumb-{i}"), images.thumb, TextureOptions::LINEAR)
                    });
                    match (loaded.size, images.large) {
                        (Size::Preview, Some(preview)) if self.previews_wanted.contains(&i) => {
                            self.previews.insert(
                                i,
                                ctx.load_texture(format!("preview-{i}"), preview, preview_options),
                            );
                        }
                        (Size::Full { .. }, Some(full)) if self.fulls_wanted.contains(&i) => {
                            // Full-size images are shown at 100 % or more: no mipmaps needed.
                            self.fulls.insert(
                                i,
                                ctx.load_texture(format!("full-{i}"), full, TextureOptions::LINEAR),
                            );
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    self.errors.insert(i, e);
                }
            }
        }
    }

    /// Keeps the current photo and its neighbours decoded at preview size and, while
    /// zoomed, the current and next photos at full size, at most `max_texture_side` pixels.
    pub fn update_requests(&mut self, culling: &Culling, zoomed: bool, max_texture_side: u32) {
        let key = (culling.current, zoomed);
        if self.requested_for == Some(key) {
            return;
        }
        self.requested_for = Some(key);
        let Some(cur) = culling.current else {
            self.previews_wanted.clear();
            self.previews.clear();
            self.fulls_wanted.clear();
            self.fulls.clear();
            return;
        };
        let photos = &culling.photos;
        let visible = culling.visible();
        let pos = visible.iter().position(|&i| i == cur).unwrap_or(0);
        let neighbour = |offset: isize| visible.get(pos.wrapping_add_signed(offset)).copied();

        let mut previews = vec![cur];
        previews.extend([1, -1, 2].into_iter().filter_map(neighbour));
        // A RAW-only photo has nothing larger than its embedded preview.
        let fulls: Vec<usize> = if zoomed {
            [Some(cur), neighbour(1)]
                .into_iter()
                .flatten()
                .filter(|&i| photos[i].jpeg.is_some())
                .collect()
        } else {
            Vec::new()
        };
        self.previews.retain(|i, _| previews.contains(i));
        self.fulls.retain(|i, _| fulls.contains(i));

        let full_size = Size::Full {
            max_side: max_texture_side,
        };
        let mut order = Vec::new();
        if fulls.contains(&cur) {
            order.push((cur, full_size));
        }
        order.extend(previews.iter().map(|&i| (i, Size::Preview)));
        order.extend(fulls.iter().filter(|&&i| i != cur).map(|&i| (i, full_size)));

        let requests = order
            .into_iter()
            .filter(|(i, size)| {
                let cached = match size {
                    Size::Full { .. } => self.fulls.contains_key(i),
                    _ => self.previews.contains_key(i),
                };
                !cached && !self.errors.contains_key(i)
            })
            .map(|(index, size)| Request {
                index,
                path: photos[index].display_source().to_owned(),
                size,
            })
            .collect();
        self.loader.set_large(requests);
        self.previews_wanted = previews;
        self.fulls_wanted = fulls;
    }
}
