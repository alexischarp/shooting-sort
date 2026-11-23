//! Decodes photos on background threads so the UI never waits on disk or JPEG decoding.
//!
//! Large images (full size for zoom, then previews for the main view) take priority over
//! filmstrip thumbnails. Every decode also produces a thumbnail, so loading a large image
//! fills the filmstrip too.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};

use eframe::egui::{self, ColorImage};
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader};

/// Longest side of the main view image, in pixels. Large enough for a Retina display.
pub const PREVIEW_MAX: u32 = 2560;
/// Longest side of filmstrip thumbnails, in pixels.
const THUMB_MAX: u32 = 320;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Thumb,
    Preview,
    /// The full resolution, limited to `max_side` (the GPU's largest texture).
    Full {
        max_side: u32,
    },
}

pub struct Request {
    pub index: usize,
    pub path: PathBuf,
    pub size: Size,
}

struct Job {
    generation: u64,
    request: Request,
}

pub struct Loaded {
    pub generation: u64,
    pub index: usize,
    pub size: Size,
    pub result: Result<Images, String>,
}

pub struct Images {
    /// Pixel size of the decoded image, after rotation, before any downscaling.
    pub original_size: [u32; 2],
    pub thumb: ColorImage,
    /// The preview or full-size image, when one was requested.
    pub large: Option<ColorImage>,
}

#[derive(Default)]
struct Queues {
    generation: u64,
    /// Most important first.
    large: VecDeque<Job>,
    thumbs: VecDeque<Job>,
}

struct Shared {
    queues: Mutex<Queues>,
    wake: Condvar,
}

pub struct Loader {
    shared: Arc<Shared>,
    results: Receiver<Loaded>,
}

impl Loader {
    pub fn new(ctx: egui::Context) -> Self {
        let shared = Arc::new(Shared {
            queues: Mutex::new(Queues::default()),
            wake: Condvar::new(),
        });
        let (tx, results) = mpsc::channel();
        let workers = std::thread::available_parallelism()
            .map(|n| n.get().clamp(2, 6))
            .unwrap_or(4);
        for i in 0..workers {
            let shared = Arc::clone(&shared);
            let tx = tx.clone();
            let ctx = ctx.clone();
            std::thread::Builder::new()
                .name(format!("image-loader-{i}"))
                .spawn(move || worker(&shared, &tx, &ctx))
                .expect("failed to start image loader thread");
        }
        Self { shared, results }
    }

    /// Drops all pending work. Results from earlier generations should be ignored.
    pub fn reset(&self, generation: u64) {
        let mut q = self.shared.queues.lock().unwrap();
        q.generation = generation;
        q.large.clear();
        q.thumbs.clear();
    }

    pub fn request_thumb(&self, index: usize, path: &Path) {
        let mut q = self.shared.queues.lock().unwrap();
        let generation = q.generation;
        q.thumbs.push_back(Job {
            generation,
            request: Request {
                index,
                path: path.to_owned(),
                size: Size::Thumb,
            },
        });
        self.shared.wake.notify_one();
    }

    /// Replaces the pending preview and full-size requests, most important first.
    /// Requests that are no longer wanted (the user moved on) are dropped.
    pub fn set_large(&self, wanted: Vec<Request>) {
        let mut q = self.shared.queues.lock().unwrap();
        let generation = q.generation;
        q.large = wanted
            .into_iter()
            .map(|request| Job {
                generation,
                request,
            })
            .collect();
        self.shared.wake.notify_all();
    }

    pub fn try_recv(&self) -> Option<Loaded> {
        self.results.try_recv().ok()
    }
}

fn worker(shared: &Shared, tx: &Sender<Loaded>, ctx: &egui::Context) {
    loop {
        let job = {
            let mut q = shared.queues.lock().unwrap();
            loop {
                if let Some(job) = q.large.pop_front().or_else(|| q.thumbs.pop_front()) {
                    break job;
                }
                q = shared.wake.wait(q).unwrap();
            }
        };
        let Request { index, path, size } = job.request;
        let loaded = Loaded {
            generation: job.generation,
            index,
            size,
            result: decode(&path, size),
        };
        if tx.send(loaded).is_err() {
            return; // The app is closing.
        }
        ctx.request_repaint();
    }
}

fn is_jpeg(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("jpeg"))
}

fn decode(path: &Path, size: Size) -> Result<Images, String> {
    let (image, orientation) = if is_jpeg(path) {
        decode_jpeg(path)
    } else {
        decode_raw_preview(path)
    }?;

    let (w, h) = (image.width(), image.height());
    let original_size = if swaps_sides(orientation) {
        [h, w]
    } else {
        [w, h]
    };

    let (thumb, large) = match size {
        Size::Thumb => (shrink(image, THUMB_MAX), None),
        Size::Preview => {
            let preview = shrink(image, PREVIEW_MAX);
            (shrink(preview.clone(), THUMB_MAX), Some(preview))
        }
        Size::Full { max_side } => {
            let thumb = image.thumbnail(THUMB_MAX, THUMB_MAX);
            (thumb, Some(shrink(image, max_side)))
        }
    };
    Ok(Images {
        original_size,
        thumb: to_color_image(thumb, orientation),
        large: large.map(|image| to_color_image(image, orientation)),
    })
}

fn swaps_sides(orientation: Orientation) -> bool {
    matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    )
}

fn decode_jpeg(path: &Path) -> Result<(DynamicImage, Orientation), String> {
    let mut decoder = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .into_decoder()
        .map_err(|e| e.to_string())?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let image = DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    Ok((image, orientation))
}

/// Reads the JPEG preview that cameras embed in RAW files, instead of developing the RAW.
/// This preview is also the largest image available for a RAW-only photo.
fn decode_raw_preview(path: &Path) -> Result<(DynamicImage, Orientation), String> {
    let source = rawler::rawsource::RawSource::new(path).map_err(|e| e.to_string())?;
    let decoder = rawler::get_decoder(&source).map_err(|e| e.to_string())?;
    let params = rawler::decoders::RawDecodeParams::default();
    let image = match decoder.preview_image(&source, &params) {
        Ok(Some(image)) => image,
        _ => decoder
            .thumbnail_image(&source, &params)
            .map_err(|e| e.to_string())?
            .ok_or("this RAW file has no embedded preview")?,
    };
    let orientation = decoder
        .raw_metadata(&source, &params)
        .ok()
        .and_then(|m| m.exif.orientation)
        .and_then(|o| u8::try_from(o).ok())
        .and_then(Orientation::from_exif)
        .unwrap_or(Orientation::NoTransforms);
    Ok((image, orientation))
}

fn shrink(image: DynamicImage, max_side: u32) -> DynamicImage {
    if image.width().max(image.height()) <= max_side {
        image
    } else {
        image.thumbnail(max_side, max_side)
    }
}

fn to_color_image(mut image: DynamicImage, orientation: Orientation) -> ColorImage {
    image.apply_orientation(orientation);
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    ColorImage::from_rgba_unmultiplied(size, rgba.as_raw())
}
