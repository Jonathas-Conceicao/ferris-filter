//! The sound files the style editor's Alert sound picker offers: the `.mp3` / `.wav` files of the
//! folder the loaded filter is in (where the game looks for `CustomAlertSound` files).
//!
//! Read-only: one non-recursive `read_dir`, never a write. The listing is cached in the egui
//! context so drawing never touches the disk: [`set_dir`] (load, reload, filter switch) lists a
//! folder and stores it, [`refresh`] re-lists the stored folder (the picker calls it when its list
//! opens) and [`get`] only reads the cache.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::style_lines::safe_file_name;

/// A listed folder and its sound files, sorted case-insensitively.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SoundFolder {
    pub dir: Option<PathBuf>,
    pub files: Vec<String>,
}

/// Extensions the game plays for `CustomAlertSound`.
const EXTENSIONS: [&str; 2] = ["mp3", "wav"];

fn cache_id() -> egui::Id {
    egui::Id::new("ff-sound-folder")
}

/// The sound files directly in `dir`: regular files ending `.mp3` / `.wav` (any case) whose names
/// can be written into a filter line. A missing or unreadable folder lists nothing.
pub fn list(dir: &Path) -> Vec<String> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<String> = read
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| {
            Path::new(name)
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| EXTENSIONS.iter().any(|e| x.eq_ignore_ascii_case(e)))
        })
        .filter(|name| safe_file_name(name))
        .collect();
    files.sort_by_key(|f| f.to_lowercase());
    files
}

/// Lists `dir` and caches it (`None` caches an empty listing).
pub fn set_dir(ctx: &egui::Context, dir: Option<PathBuf>) {
    let files = dir.as_deref().map(list).unwrap_or_default();
    let folder = Arc::new(SoundFolder { dir, files });
    ctx.data_mut(|d| d.insert_temp(cache_id(), folder));
}

/// Re-lists the cached folder.
pub fn refresh(ctx: &egui::Context) {
    let dir = get(ctx).dir.clone();
    set_dir(ctx, dir);
}

/// The cached listing (empty before the first [`set_dir`]). No I/O.
pub fn get(ctx: &egui::Context) -> Arc<SoundFolder> {
    ctx.data(|d| d.get_temp::<Arc<SoundFolder>>(cache_id()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_mp3_and_wav_files_only() {
        let dir = tempfile::tempdir().unwrap();
        for f in ["a.MP3", "b.wav", "c.ogg", "d.txt", "Z.mp3"] {
            std::fs::write(dir.path().join(f), b"").unwrap();
        }
        std::fs::create_dir(dir.path().join("e.mp3")).unwrap();
        // Quotes are not allowed in Windows file names; elsewhere such a file is skipped.
        let _ = std::fs::write(dir.path().join("q\"x.mp3"), b"");
        assert_eq!(list(dir.path()), ["a.MP3", "b.wav", "Z.mp3"]);
    }

    #[test]
    fn a_missing_folder_lists_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list(&dir.path().join("nope")).is_empty());
    }

    #[test]
    fn the_cache_is_read_without_io_and_refreshed_on_request() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        assert_eq!(*get(&ctx), SoundFolder::default());
        std::fs::write(dir.path().join("a.mp3"), b"").unwrap();
        set_dir(&ctx, Some(dir.path().to_path_buf()));
        assert_eq!(get(&ctx).files, ["a.mp3"]);
        std::fs::write(dir.path().join("b.wav"), b"").unwrap();
        assert_eq!(get(&ctx).files, ["a.mp3"], "no re-read until asked");
        refresh(&ctx);
        assert_eq!(get(&ctx).files, ["a.mp3", "b.wav"]);
        set_dir(&ctx, None);
        assert!(get(&ctx).files.is_empty());
    }
}
