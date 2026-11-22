//! Decodes photos on background threads so the UI never waits on disk or JPEG decoding.
//!
//! Large previews (for the main view) take priority over filmstrip thumbnails.
//! Every decode produces a thumbnail, so loading a preview also fills the filmstrip.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};

use eframe::egui::{self, ColorImage};
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader};

/// Longest side of the main view image, in pixels. Large enough for a Retina display.
const PREVIEW_MAX: u32 = 2560;
/// Longest side of filmstrip thumbnails, in pixels.
const THUMB_MAX: u32 = 320;

struct Job {
    generation: u64,
    index: usize,
    path: PathBuf,
    want_preview: bool,
}

pub struct Loaded {
    pub generation: u64,
    pub index: usize,
    pub result: Result<Images, String>,
}

pub struct Images {
    pub thumb: ColorImage,
    pub preview: Option<ColorImage>,
}

#[derive(Default)]
struct Queues {
    generation: u64,
    /// Most important first.
    previews: VecDeque<Job>,
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
        q.previews.clear();
        q.thumbs.clear();
    }

    pub fn request_thumb(&self, index: usize, path: &Path) {
        let mut q = self.shared.queues.lock().unwrap();
        let generation = q.generation;
        q.thumbs.push_back(Job {
            generation,
            index,
            path: path.to_owned(),
            want_preview: false,
        });
        self.shared.wake.notify_one();
    }

    /// Replaces the pending preview requests, most important first.
    /// Requests that are no longer wanted (the user moved on) are dropped.
    pub fn set_previews(&self, wanted: Vec<(usize, PathBuf)>) {
        let mut q = self.shared.queues.lock().unwrap();
        let generation = q.generation;
        q.previews = wanted
            .into_iter()
            .map(|(index, path)| Job {
                generation,
                index,
                path,
                want_preview: true,
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
                if let Some(job) = q.previews.pop_front().or_else(|| q.thumbs.pop_front()) {
                    break job;
                }
                q = shared.wake.wait(q).unwrap();
            }
        };
        let result = decode(&job.path, job.want_preview);
        let loaded = Loaded {
            generation: job.generation,
            index: job.index,
            result,
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

fn decode(path: &Path, want_preview: bool) -> Result<Images, String> {
    let (image, orientation) = if is_jpeg(path) {
        decode_jpeg(path)
    } else {
        decode_raw_preview(path)
    }?;

    let preview = shrink(image, PREVIEW_MAX);
    let thumb = shrink(preview.clone(), THUMB_MAX);
    Ok(Images {
        thumb: to_color_image(thumb, orientation),
        preview: want_preview.then(|| to_color_image(preview, orientation)),
    })
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
