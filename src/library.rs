//! Scans a shoot folder and groups RAW and JPEG files that share a base name.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

const JPEG_EXTENSIONS: &[&str] = &["jpg", "jpeg"];

const RAW_EXTENSIONS: &[&str] = &[
    "3fr", "arw", "cr2", "cr3", "crw", "dcr", "dng", "erf", "iiq", "kdc", "mef", "mos", "nef",
    "nrw", "orf", "pef", "raf", "rw2", "rwl", "sr2", "srf", "srw",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Decision {
    Keep,
    Remove,
}

/// One shot: a JPEG, a RAW, or both, sharing the same base name.
#[derive(Clone, Debug)]
pub struct Photo {
    /// File name without extension, e.g. `DSC04218`.
    pub stem: String,
    pub jpeg: Option<PathBuf>,
    pub raw: Option<PathBuf>,
    pub decision: Option<Decision>,
}

impl Photo {
    /// File used to show the photo: the JPEG when present, otherwise the RAW.
    pub fn display_source(&self) -> &Path {
        self.jpeg
            .as_deref()
            .or(self.raw.as_deref())
            .expect("a photo always has at least one file")
    }

    pub fn format_label(&self) -> &'static str {
        match (&self.jpeg, &self.raw) {
            (Some(_), Some(_)) => "RAW + JPEG",
            (Some(_), None) => "JPEG only",
            _ => "RAW only",
        }
    }
}

enum Kind {
    Jpeg,
    Raw,
}

fn classify(path: &Path) -> Option<Kind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if JPEG_EXTENSIONS.contains(&ext.as_str()) {
        Some(Kind::Jpeg)
    } else if RAW_EXTENSIONS.contains(&ext.as_str()) {
        Some(Kind::Raw)
    } else {
        None
    }
}

/// Lists the photos at the top level of `folder`, sorted by name.
///
/// Subfolders (such as `sorted/` and `to-remove/`) and hidden files (such as the
/// `._*` files macOS writes on SD cards) are ignored.
pub fn scan(folder: &Path) -> io::Result<Vec<Photo>> {
    let mut by_stem: BTreeMap<String, Photo> = BTreeMap::new();

    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let Some(kind) = classify(&path) else {
            continue;
        };
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };

        let photo = by_stem.entry(stem.to_owned()).or_insert_with(|| Photo {
            stem: stem.to_owned(),
            jpeg: None,
            raw: None,
            decision: None,
        });
        // If two files of the same kind share a stem, keep the first one found;
        // the other one is left untouched in the folder.
        let slot = match kind {
            Kind::Jpeg => &mut photo.jpeg,
            Kind::Raw => &mut photo.raw,
        };
        if slot.is_none() {
            *slot = Some(path);
        }
    }

    Ok(by_stem.into_values().collect())
}
