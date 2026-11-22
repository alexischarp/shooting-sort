//! Saves Keep / To Remove decisions in the shoot folder, so culling can resume
//! after the app is closed.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::library::{Decision, Photo};

pub const SESSION_FILE_NAME: &str = ".shooting-sort.json";

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct SessionFile {
    version: u32,
    /// Decisions keyed by photo base name (e.g. `DSC04218`).
    decisions: BTreeMap<String, Decision>,
}

fn session_path(folder: &Path) -> PathBuf {
    folder.join(SESSION_FILE_NAME)
}

/// Applies the saved decisions to `photos`. A missing or unreadable file is ignored.
pub fn load(folder: &Path, photos: &mut [Photo]) {
    let Ok(text) = std::fs::read_to_string(session_path(folder)) else {
        return;
    };
    let Ok(session) = serde_json::from_str::<SessionFile>(&text) else {
        return;
    };
    for photo in photos {
        photo.decision = session.decisions.get(&photo.stem).copied();
    }
}

/// Writes the current decisions. Deletes the file when there are none left.
pub fn save(folder: &Path, photos: &[Photo]) -> io::Result<()> {
    let decisions: BTreeMap<String, Decision> = photos
        .iter()
        .filter_map(|p| p.decision.map(|d| (p.stem.clone(), d)))
        .collect();

    let path = session_path(folder);
    if decisions.is_empty() {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }

    let json = serde_json::to_string_pretty(&SessionFile {
        version: 1,
        decisions,
    })
    .map_err(io::Error::other)?;
    // Write to a temporary file first so a crash never leaves a half-written session.
    let tmp = folder.join(format!("{SESSION_FILE_NAME}.tmp"));
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, path)
}
