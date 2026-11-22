//! Moves files into `sorted/raw`, `sorted/jpeg` and `to-remove` according to
//! each photo's decision. Photos without a decision stay where they are.
//! Files are only moved, never deleted, and never overwrite an existing file.

use std::io;
use std::path::{Path, PathBuf};

use crate::library::{Decision, Photo};

pub const SORTED_DIR: &str = "sorted";
pub const RAW_DIR: &str = "raw";
pub const JPEG_DIR: &str = "jpeg";
pub const TO_REMOVE_DIR: &str = "to-remove";

#[derive(Default, Debug)]
pub struct SortReport {
    pub moved: usize,
    pub failures: Vec<(PathBuf, String)>,
}

/// Where each file of `photo` goes, or nothing if the photo has no decision.
fn destinations(folder: &Path, photo: &Photo) -> Vec<(PathBuf, PathBuf)> {
    let Some(decision) = photo.decision else {
        return Vec::new();
    };
    let (jpeg_dir, raw_dir) = match decision {
        Decision::Keep => {
            let sorted = folder.join(SORTED_DIR);
            (sorted.join(JPEG_DIR), sorted.join(RAW_DIR))
        }
        Decision::Remove => {
            let dir = folder.join(TO_REMOVE_DIR);
            (dir.clone(), dir)
        }
    };
    [(&photo.jpeg, jpeg_dir), (&photo.raw, raw_dir)]
        .into_iter()
        .filter_map(|(src, dir)| {
            let src = src.as_ref()?;
            Some((src.clone(), dir.join(src.file_name()?)))
        })
        .collect()
}

fn move_file(src: &Path, dest: &Path) -> io::Result<()> {
    if dest.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "a file with this name already exists at the destination",
        ));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(src, dest)
}

pub fn sort(folder: &Path, photos: &[Photo]) -> SortReport {
    let mut report = SortReport::default();
    for photo in photos {
        for (src, dest) in destinations(folder, photo) {
            match move_file(&src, &dest) {
                Ok(()) => report.moved += 1,
                Err(e) => report.failures.push((src, e.to_string())),
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn photo(stem: &str, jpeg: bool, raw: bool, decision: Option<Decision>) -> Photo {
        Photo {
            stem: stem.into(),
            jpeg: jpeg.then(|| PathBuf::from(format!("/shoot/{stem}.JPG"))),
            raw: raw.then(|| PathBuf::from(format!("/shoot/{stem}.ARW"))),
            decision,
        }
    }

    #[test]
    fn kept_photo_is_split_into_sorted_raw_and_jpeg() {
        let dests = destinations(Path::new("/shoot"), &photo("A", true, true, Some(Decision::Keep)));
        assert_eq!(
            dests,
            vec![
                ("/shoot/A.JPG".into(), "/shoot/sorted/jpeg/A.JPG".into()),
                ("/shoot/A.ARW".into(), "/shoot/sorted/raw/A.ARW".into()),
            ]
        );
    }

    #[test]
    fn removed_photo_goes_to_to_remove() {
        let dests = destinations(Path::new("/shoot"), &photo("B", true, true, Some(Decision::Remove)));
        assert_eq!(
            dests,
            vec![
                ("/shoot/B.JPG".into(), "/shoot/to-remove/B.JPG".into()),
                ("/shoot/B.ARW".into(), "/shoot/to-remove/B.ARW".into()),
            ]
        );
    }

    #[test]
    fn undecided_photo_stays_in_place() {
        assert!(destinations(Path::new("/shoot"), &photo("C", true, true, None)).is_empty());
    }

    #[test]
    fn sort_moves_files_and_never_overwrites() {
        let folder = std::env::temp_dir().join(format!("shooting-sort-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("to-remove")).unwrap();
        for name in ["A.JPG", "A.ARW", "B.JPG", "B.ARW", "C.JPG"] {
            std::fs::write(folder.join(name), name).unwrap();
        }
        // A file already at the destination must not be overwritten.
        std::fs::write(folder.join("to-remove/B.ARW"), "existing").unwrap();

        let mut photos = crate::library::scan(&folder).unwrap();
        photos[0].decision = Some(Decision::Keep);
        photos[1].decision = Some(Decision::Remove);
        let report = sort(&folder, &photos);

        assert_eq!(report.moved, 3);
        assert_eq!(report.failures.len(), 1);
        assert!(folder.join("sorted/jpeg/A.JPG").is_file());
        assert!(folder.join("sorted/raw/A.ARW").is_file());
        assert!(folder.join("to-remove/B.JPG").is_file());
        assert_eq!(std::fs::read_to_string(folder.join("to-remove/B.ARW")).unwrap(), "existing");
        assert!(folder.join("B.ARW").is_file());
        assert!(folder.join("C.JPG").is_file());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn orphan_raw_is_sorted() {
        let dests = destinations(Path::new("/shoot"), &photo("D", false, true, Some(Decision::Keep)));
        assert_eq!(dests, vec![("/shoot/D.ARW".into(), "/shoot/sorted/raw/D.ARW".into())]);
    }
}
