//! The dynamic filter lifecycle.
//!
//! # Filter-as-store (Phase 18)
//!
//! The `.filter` file itself is the store. [`LoadedFilter::load`] reads it (an absent file is an
//! empty filter; a v0.4 file is migrated in memory only), [`economy_body`] prices the economy
//! section, [`apply_text`] computes the next text for an explicit [`ApplyOp`], and [`apply_file`]
//! is the single writer: guarded against external edits, backed up once, written atomically.
//! Loading never writes.
//!
//! Two rules inherited from [`crate::compile`]:
//!
//! - **No `println!` / `eprintln!`.** Functions here return what happened; `run.rs` presents it.
//! - **No implicit working-directory reads.** Every path is a parameter, so the overlay can point
//!   at its own root.
//!
//! PoE2 never re-reads a filter on its own — after writing, the player must reload it via
//! Options → Game → Item Filter → Refresh. Nothing here watches files or writes on a timer.

use std::path::{Path, PathBuf};

use crate::compile::{self, CompileError, InputPaths, PriceSource, PriceSourcePolicy};

/// The generated filter's conventional name.
pub const FILTER_FILE: &str = "dynamic.filter";

/// The in-game config folder `dynamic.filter` lands in by default.
///
/// Delegates to the same resolution `install` uses rather than re-spelling the path segments —
/// there should be exactly one place in the crate that knows where Path of Exile 2 keeps its
/// filters. Returns `None` when `USERPROFILE` is unset (non-Windows), in which case the caller
/// must require an explicit destination.
pub fn default_destination() -> Option<PathBuf> {
    crate::install::default_dest()
}

/// Everything that can go wrong in the dynamic lifecycle.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum DynamicError {
    /// Loading or compiling the store failed.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Compile(#[from] CompileError),

    /// The filter file exists but could not be read.
    #[error("Cannot read `{path}`: {source}")]
    FilterUnreadable {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// The filter is not UTF-8; never lossy-converted.
    #[error("`{path}` is not valid UTF-8; FerrisFilter only edits UTF-8 filters and will not convert it")]
    NotUtf8 { path: String },

    /// Refresh/remove was asked of a file that does not exist.
    #[error("`{path}` does not exist; nothing was written")]
    #[diagnostic(help("Use --add-economy to create it with an economy section."))]
    FilterNotFound { path: String },

    /// Refresh/remove was asked of a filter with no economy section.
    #[error("`{path}` has no economy section; nothing was written")]
    #[diagnostic(help("Use --add-economy to add one at the end of the file."))]
    NoEconomySection { path: String },

    /// The section markers are damaged, or the filter could not be processed.
    #[error("`{path}`: {source}")]
    Section {
        path: String,
        #[source]
        source: crate::filter::section::SectionError,
    },

    /// Neither live nor fallback prices exist.
    #[error("No price data: neither prices.json nor prices-fallback.json was found, so the economy section was left as it is")]
    #[diagnostic(help("Run `ff-cli sync` (or SYNC ECONOMY in the overlay) and apply again."))]
    EconomyUnavailable,

    /// The guarded write failed.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Write(#[from] crate::safe_write::WriteError),
}

/// The default `dynamic.filter` location inside the PoE2 folder.
pub fn default_filter_path() -> Option<PathBuf> {
    default_destination().map(|d| d.join(FILTER_FILE))
}

/// An explicit operation on the economy section (D-05): only `AddEconomy` creates a section and
/// only `RemoveEconomy` deletes one. The `&str` is the body from [`economy_body`].
#[derive(Debug, Clone, Copy)]
pub enum ApplyOp<'a> {
    Refresh(&'a str),
    AddEconomy(&'a str),
    RemoveEconomy,
}

/// A filter file loaded as the store. Loading never writes (D-10); `disk` is the exact bytes read
/// and is the guard baseline for [`apply_file`]. To reload, call [`LoadedFilter::load`] again.
#[derive(Debug, Clone)]
pub struct LoadedFilter {
    path: PathBuf,
    disk: Option<Vec<u8>>,
    text: String,
    migrated: bool,
}

impl LoadedFilter {
    pub fn load(path: &Path) -> Result<Self, DynamicError> {
        let shown = || path.display().to_string();
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    path: path.to_path_buf(),
                    disk: None,
                    text: String::new(),
                    migrated: false,
                });
            }
            Err(e) => {
                return Err(DynamicError::FilterUnreadable {
                    path: shown(),
                    source: e,
                })
            }
        };
        crate::filter::parse(&bytes).map_err(|_| DynamicError::NotUtf8 { path: shown() })?;
        let mut text =
            String::from_utf8(bytes.clone()).map_err(|_| DynamicError::NotUtf8 { path: shown() })?;
        let mut migrated = false;
        match crate::filter::section::migrate_v04(&text) {
            Ok(Some(m)) => {
                text = m;
                migrated = true;
            }
            Ok(None) => {}
            Err(e) => {
                return Err(DynamicError::Section {
                    path: shown(),
                    source: e,
                })
            }
        }
        Ok(Self {
            path: path.to_path_buf(),
            disk: Some(bytes),
            text,
            migrated,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn existed(&self) -> bool {
        self.disk.is_some()
    }
    pub fn disk(&self) -> Option<&[u8]> {
        self.disk.as_deref()
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn migrated(&self) -> bool {
        self.migrated
    }
    pub fn has_section(&self) -> Result<bool, DynamicError> {
        crate::filter::section::has_section(&self.text).map_err(|e| DynamicError::Section {
            path: self.path.display().to_string(),
            source: e,
        })
    }
}

/// Compute the next filter text for `op`. Pure; errors are the bare section errors and
/// [`apply_file`] attaches the path (`NoSection` becomes [`DynamicError::NoEconomySection`]).
pub fn apply_text(
    text: &str,
    op: ApplyOp<'_>,
) -> Result<String, crate::filter::section::SectionError> {
    use crate::filter::section;
    match op {
        ApplyOp::Refresh(body) => section::refresh(text, body),
        ApplyOp::AddEconomy(body) => section::add(text, body),
        ApplyOp::RemoveEconomy => section::remove(text),
    }
}

/// Price the economy section from `paths`. Refuses to produce an empty section when no price
/// source exists (D-17). Callers present `Fallback` warnings.
pub fn economy_body(
    paths: &InputPaths,
    policy: PriceSourcePolicy,
) -> Result<(String, PriceSource), DynamicError> {
    let inputs = compile::load_economy(paths, policy)?;
    if inputs.price_source == PriceSource::Absent {
        return Err(DynamicError::EconomyUnavailable);
    }
    let text = compile::compile_economy(&inputs)?;
    Ok((text, inputs.price_source))
}

/// What [`apply_file`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyReport {
    pub path: PathBuf,
    pub outcome: crate::safe_write::WriteOutcome,
    pub migrated: bool,
    pub bytes: usize,
}

/// The only writer. Never writes on open; the residual TOCTOU window is accepted (see
/// [`crate::safe_write`]). On success the loaded baseline becomes the written bytes; on error
/// `loaded` is unchanged.
pub fn apply_file(
    loaded: &mut LoadedFilter,
    op: ApplyOp<'_>,
) -> Result<ApplyReport, DynamicError> {
    let shown = loaded.path.display().to_string();
    if !loaded.existed() && matches!(op, ApplyOp::Refresh(_) | ApplyOp::RemoveEconomy) {
        return Err(DynamicError::FilterNotFound { path: shown });
    }
    let new_text = apply_text(&loaded.text, op).map_err(|e| match e {
        crate::filter::section::SectionError::NoSection => {
            DynamicError::NoEconomySection { path: shown.clone() }
        }
        other => DynamicError::Section {
            path: shown.clone(),
            source: other,
        },
    })?;
    let outcome = crate::safe_write::write_guarded(
        &loaded.path,
        loaded.disk.as_deref(),
        new_text.as_bytes(),
    )?;
    let migrated = loaded.migrated;
    let bytes = new_text.len();
    loaded.disk = Some(new_text.clone().into_bytes());
    loaded.text = new_text;
    loaded.migrated = false;
    Ok(ApplyReport {
        path: loaded.path.clone(),
        outcome,
        migrated,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn repo_file(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    fn seed_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::copy(repo_file("tiers.json"), dir.path().join("tiers.json")).unwrap();
        fs::copy(repo_file("styles.json"), dir.path().join("styles.json")).unwrap();
        fs::copy(fixture("seed-prices.json"), dir.path().join("prices.json")).unwrap();
        dir
    }

    fn seed_body() -> String {
        let dir = seed_dir();
        economy_body(&InputPaths::relative_to(dir.path()), PriceSourcePolicy::Auto)
            .expect("body")
            .0
    }

    fn copy_fixture(dir: &Path, name: &str) -> PathBuf {
        let dest = dir.join("dynamic.filter");
        fs::copy(fixture(name), &dest).unwrap();
        dest
    }

    #[test]
    fn load_absent_is_empty_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("dynamic.filter");
        let l = LoadedFilter::load(&p).unwrap();
        assert!(!l.existed());
        assert_eq!(l.text(), "");
        assert!(l.disk().is_none());
        assert!(!l.migrated());
        assert!(!p.exists());
    }

    #[test]
    fn load_v04_migrates_in_memory_only() {
        let dir = tempfile::tempdir().unwrap();
        let p = copy_fixture(dir.path(), "v04-dynamic.filter");
        let orig = fs::read(&p).unwrap();
        let l = LoadedFilter::load(&p).unwrap();
        assert!(l.migrated());
        assert!(l.has_section().unwrap());
        assert_eq!(l.disk().unwrap(), &orig[..]);
        assert_eq!(fs::read(&p).unwrap(), orig);
    }

    #[test]
    fn load_foreign_filter_is_verbatim() {
        let dir = tempfile::tempdir().unwrap();
        let p = copy_fixture(dir.path(), "seed-universal.filter");
        let l = LoadedFilter::load(&p).unwrap();
        assert!(!l.migrated());
        assert_eq!(l.text().as_bytes(), fs::read(&p).unwrap());
        assert!(!l.has_section().unwrap());
    }

    #[test]
    fn load_rejects_non_utf8_and_directories() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bad.filter");
        fs::write(&p, [0xffu8, 0xfe, 0x41]).unwrap();
        assert!(matches!(
            LoadedFilter::load(&p),
            Err(DynamicError::NotUtf8 { .. })
        ));
        assert!(matches!(
            LoadedFilter::load(dir.path()),
            Err(DynamicError::FilterUnreadable { .. })
        ));
    }

    #[test]
    fn apply_text_ops() {
        let body = seed_body();
        let with = apply_text("", ApplyOp::AddEconomy(&body)).unwrap();
        assert!(crate::filter::section::has_section(&with).unwrap());
        assert_eq!(
            apply_text(&with, ApplyOp::Refresh(&body)).unwrap(),
            crate::filter::section::refresh(&with, &body).unwrap()
        );
        let none = "Show # x\n";
        assert!(matches!(
            apply_text(none, ApplyOp::Refresh(&body)),
            Err(crate::filter::section::SectionError::NoSection)
        ));
        assert!(matches!(
            apply_text(none, ApplyOp::RemoveEconomy),
            Err(crate::filter::section::SectionError::NoSection)
        ));
        let damaged = format!("{}\n", crate::filter::section::BEGIN_LINE);
        assert!(apply_text(&damaged, ApplyOp::Refresh(&body)).is_err());
    }

    #[test]
    fn economy_body_live_and_absent() {
        let dir = seed_dir();
        let (text, src) =
            economy_body(&InputPaths::relative_to(dir.path()), PriceSourcePolicy::Auto).unwrap();
        assert_eq!(src, PriceSource::Live);
        assert!(!text.is_empty());
        let empty = tempfile::tempdir().unwrap();
        fs::copy(repo_file("tiers.json"), empty.path().join("tiers.json")).unwrap();
        fs::copy(repo_file("styles.json"), empty.path().join("styles.json")).unwrap();
        let err = economy_body(
            &InputPaths::relative_to(empty.path()),
            PriceSourcePolicy::Auto,
        )
        .expect_err("absent");
        assert!(matches!(err, DynamicError::EconomyUnavailable));
    }

    #[test]
    fn apply_file_migrates_then_second_apply_is_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let p = copy_fixture(dir.path(), "v04-dynamic.filter");
        let orig = fs::read(&p).unwrap();
        let body = seed_body();
        let mut l = LoadedFilter::load(&p).unwrap();
        let r = apply_file(&mut l, ApplyOp::Refresh(&body)).unwrap();
        assert!(r.migrated);
        let bak = match &r.outcome {
            crate::safe_write::WriteOutcome::Written { backup: Some(b) } => b.clone(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(fs::read(&bak).unwrap(), orig);
        let written = fs::read_to_string(&p).unwrap();
        assert!(crate::filter::section::has_section(&written).unwrap());
        assert!(written.contains("#ff:name"));
        assert_eq!(written.len(), r.bytes);

        let bak_bytes = fs::read(&bak).unwrap();
        let mtime = fs::metadata(&p).unwrap().modified().unwrap();
        let r2 = apply_file(&mut l, ApplyOp::Refresh(&body)).unwrap();
        assert_eq!(r2.outcome, crate::safe_write::WriteOutcome::Unchanged);
        assert!(!r2.migrated);
        assert_eq!(fs::metadata(&p).unwrap().modified().unwrap(), mtime);
        assert_eq!(fs::read(&bak).unwrap(), bak_bytes);

        assert!(!LoadedFilter::load(&p).unwrap().migrated());
    }

    #[test]
    fn apply_file_refuses_a_file_changed_since_load() {
        let dir = tempfile::tempdir().unwrap();
        let p = copy_fixture(dir.path(), "v04-dynamic.filter");
        let body = seed_body();
        let mut l = LoadedFilter::load(&p).unwrap();
        fs::write(&p, b"Show # edited elsewhere\n").unwrap();
        let err = apply_file(&mut l, ApplyOp::Refresh(&body)).expect_err("refuse");
        assert!(matches!(
            err,
            DynamicError::Write(crate::safe_write::WriteError::ChangedOnDisk { .. })
        ));
        assert_eq!(fs::read(&p).unwrap(), b"Show # edited elsewhere\n");
        assert!(!crate::safe_write::backup_path(&p).exists());
    }

    #[test]
    fn apply_file_absent_path_rules() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("dynamic.filter");
        let body = seed_body();
        let mut l = LoadedFilter::load(&p).unwrap();
        assert!(matches!(
            apply_file(&mut l, ApplyOp::Refresh(&body)),
            Err(DynamicError::FilterNotFound { .. })
        ));
        assert!(matches!(
            apply_file(&mut l, ApplyOp::RemoveEconomy),
            Err(DynamicError::FilterNotFound { .. })
        ));
        assert!(!p.exists());

        let r = apply_file(&mut l, ApplyOp::AddEconomy(&body)).unwrap();
        assert_eq!(
            r.outcome,
            crate::safe_write::WriteOutcome::Written { backup: None }
        );
        assert!(crate::filter::section::has_section(&fs::read_to_string(&p).unwrap()).unwrap());

        let missing_parent = dir.path().join("nope").join("dynamic.filter");
        let mut l2 = LoadedFilter::load(&missing_parent).unwrap();
        let err = apply_file(&mut l2, ApplyOp::AddEconomy(&body)).expect_err("no dir");
        assert!(matches!(
            err,
            DynamicError::Write(crate::safe_write::WriteError::DestinationMissing { .. })
        ));
        assert!(!dir.path().join("nope").exists());
    }

    #[test]
    fn apply_file_refresh_without_section_leaves_file_alone() {
        let dir = tempfile::tempdir().unwrap();
        let p = copy_fixture(dir.path(), "seed-universal.filter");
        let orig = fs::read(&p).unwrap();
        let body = seed_body();
        let mut l = LoadedFilter::load(&p).unwrap();
        assert!(matches!(
            apply_file(&mut l, ApplyOp::Refresh(&body)),
            Err(DynamicError::NoEconomySection { .. })
        ));
        assert_eq!(fs::read(&p).unwrap(), orig);
        assert!(!crate::safe_write::backup_path(&p).exists());
    }

    #[test]
    fn apply_file_remove_economy_backs_up() {
        let dir = tempfile::tempdir().unwrap();
        let p = copy_fixture(dir.path(), "seed-universal.filter");
        let body = seed_body();
        let mut l = LoadedFilter::load(&p).unwrap();
        apply_file(&mut l, ApplyOp::AddEconomy(&body)).unwrap();
        let with = fs::read(&p).unwrap();
        let mut l = LoadedFilter::load(&p).unwrap();
        // The first write already made the .ffbak (original); removal keeps it.
        apply_file(&mut l, ApplyOp::RemoveEconomy).unwrap();
        let after = fs::read_to_string(&p).unwrap();
        assert!(!after.contains("ff:economy"));
        assert!(crate::safe_write::backup_path(&p).exists());
        assert_ne!(after.as_bytes(), &with[..]);
    }

}
