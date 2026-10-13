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
//! # Legacy JSON store (removed in 18-07)
//!
//! Everything below that mentions `dynamic.json`, `BASE_FILE`, `STORE_FILE`, `IDENTITY`,
//! [`ensure_store`], [`reset_store`], [`compile_store`] and [`write_filter`] is legacy, kept only
//! until the CLI and overlay are re-pointed.
//!
//! `dynamic.json` is a full [`Config`](crate::models::config::Config), exactly like `config.json`
//! — so compiling it needs no new emitter path, just [`crate::compile`] pointed at a different
//! file. What this module adds is the lifecycle around that:
//!
//! - **seed** — create the working copy from the committed template when it is absent
//! - **reset** — restore the working copy from the template on demand
//! - **compile** — load and compile the working copy, naming itself in the header
//! - **write** — land the result in the game folder without the game ever seeing a partial file
//!
//! Two rules inherited from [`crate::compile`]:
//!
//! - **No `println!` / `eprintln!`.** Functions here return what happened; `run.rs` presents it.
//! - **No implicit working-directory reads.** Every path is a parameter, so the overlay can point
//!   at its own root.
//!
//! Note that PoE2 never re-reads a filter on its own — after writing, the player must reload it
//! via Options → Game → Item Filter → Refresh. Nothing here can change that, so nothing here
//! watches files or writes on a timer.

use std::path::{Path, PathBuf};

use crate::compile::{self, CompileError, InputPaths, PriceSource, PriceSourcePolicy};
use crate::emitter::FilterIdentity;

/// Committed template — the starting point for a new store, and the source for a reset.
/// Never written to by the tool.
pub const BASE_FILE: &str = "dynamic_base.json";
/// The working copy the overlay edits. Git-ignored: per-user state, not committed policy.
pub const STORE_FILE: &str = "dynamic.json";
/// The generated filter's conventional name.
pub const FILTER_FILE: &str = "dynamic.filter";

/// How the filter names itself and its source in the generated header.
pub const IDENTITY: FilterIdentity<'static> = FilterIdentity {
    output_name: FILTER_FILE,
    source_name: STORE_FILE,
};

/// The in-game config folder `dynamic.filter` lands in by default.
///
/// Delegates to the same resolution `install` uses rather than re-spelling the path segments —
/// there should be exactly one place in the crate that knows where Path of Exile 2 keeps its
/// filters. Returns `None` when `USERPROFILE` is unset (non-Windows), in which case the caller
/// must require an explicit destination.
pub fn default_destination() -> Option<PathBuf> {
    crate::install::default_dest()
}

/// What [`ensure_store`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreState {
    /// The working copy already existed and was left untouched.
    Existing,
    /// The working copy was absent and has been created from the template.
    Seeded,
}

/// Everything that can go wrong in the dynamic lifecycle.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum DynamicError {
    /// The template is missing, so a store cannot be seeded or reset.
    #[error("Cannot read `{path}`: {source}")]
    #[diagnostic(help("`{path}` is committed to the repository — run from the repo root, or pass an explicit path."))]
    TemplateUnreadable {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// The working copy could not be created or replaced.
    #[error("Cannot write `{path}`: {source}")]
    StoreUnwritable {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Loading or compiling the store failed.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Compile(#[from] CompileError),

    /// The destination folder does not exist. Deliberately not created — a typo'd path should
    /// fail loudly rather than scatter filter files somewhere unexpected.
    #[error("Destination folder not found: `{path}` — make sure Path of Exile 2 has been run at least once, or pass an explicit destination.")]
    DestinationMissing { path: String },

    /// The filter could not be written to the destination.
    #[error("Cannot write `{path}`: {source}")]
    FilterUnwritable {
        path: String,
        #[source]
        source: std::io::Error,
    },

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

/// Create the working copy from the template if it does not exist yet.
///
/// An existing store is **never** overwritten — it holds the user's edits. Use [`reset_store`]
/// to deliberately discard them.
pub fn ensure_store(base: &Path, store: &Path) -> Result<StoreState, DynamicError> {
    if store.exists() {
        return Ok(StoreState::Existing);
    }
    copy_template(base, store)?;
    Ok(StoreState::Seeded)
}

/// Replace the working copy with the template, discarding whatever was there.
///
/// This is the "start a new filter from scratch" path — the reason the template is a separate
/// committed file rather than something generated once.
pub fn reset_store(base: &Path, store: &Path) -> Result<(), DynamicError> {
    copy_template(base, store)
}

fn copy_template(base: &Path, store: &Path) -> Result<(), DynamicError> {
    let contents = std::fs::read(base).map_err(|e| DynamicError::TemplateUnreadable {
        path: base.display().to_string(),
        source: e,
    })?;
    std::fs::write(store, contents).map_err(|e| DynamicError::StoreUnwritable {
        path: store.display().to_string(),
        source: e,
    })
}

/// The outcome of compiling the dynamic store.
#[derive(Debug)]
pub struct CompiledFilter {
    /// The filter text, LF-only, ending in exactly one newline.
    pub text: String,
    /// Which price source backed the economy sections — returned, not printed, so the caller
    /// decides how to surface it.
    pub price_source: PriceSource,
}

/// Load the working copy and compile it into filter text.
///
/// `paths` supplies every input location; its `config` field is overridden with `store` so the
/// dynamic store is what gets compiled while prices and tiers still resolve normally. The economy
/// sections come along automatically — the same pricer runs — which is why `dynamic.filter` is a
/// complete, playable filter rather than a policy-only fragment.
pub fn compile_store(paths: &InputPaths, store: &Path) -> Result<CompiledFilter, DynamicError> {
    let paths = paths.clone().with_config(store);
    let inputs = compile::load_inputs(&paths, PriceSourcePolicy::Auto)?;
    let text = compile::compile_as(&inputs, &IDENTITY)?;
    Ok(CompiledFilter {
        text,
        price_source: inputs.price_source,
    })
}

/// Write filter text into `dest_dir` as `dynamic.filter`, atomically.
///
/// The temp file is created **in the destination directory**, not the system temp dir: the game
/// folder is typically on a different volume (OneDrive), and `rename` across volumes fails. The
/// game may read the filter at any moment the player hits Refresh, so it must never observe a
/// partially written file.
///
/// Returns the path written. Errors rather than creating a missing destination, matching
/// `install`'s guard.
pub fn write_filter(dest_dir: &Path, text: &str) -> Result<PathBuf, DynamicError> {
    if !dest_dir.is_dir() {
        return Err(DynamicError::DestinationMissing {
            path: dest_dir.display().to_string(),
        });
    }

    let target = dest_dir.join(FILTER_FILE);

    let mut temp = tempfile::NamedTempFile::new_in(dest_dir).map_err(|e| {
        DynamicError::FilterUnwritable {
            path: target.display().to_string(),
            source: e,
        }
    })?;
    std::io::Write::write_all(&mut temp, text.as_bytes()).map_err(|e| {
        DynamicError::FilterUnwritable {
            path: target.display().to_string(),
            source: e,
        }
    })?;
    temp.persist(&target)
        .map_err(|e| DynamicError::FilterUnwritable {
            path: target.display().to_string(),
            source: e.error,
        })?;

    Ok(target)
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

    #[test]
    fn ensure_store_seeds_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let base = repo_file(BASE_FILE);
        let store = dir.path().join(STORE_FILE);

        let state = ensure_store(&base, &store).expect("seed");
        assert_eq!(state, StoreState::Seeded);
        assert_eq!(
            fs::read(&store).unwrap(),
            fs::read(&base).unwrap(),
            "a seeded store must be byte-identical to the template"
        );
    }

    /// The only destructive failure mode in this module: seeding must never clobber the user's
    /// edits.
    #[test]
    fn ensure_store_never_overwrites_an_existing_store() {
        let dir = tempfile::tempdir().unwrap();
        let base = repo_file(BASE_FILE);
        let store = dir.path().join(STORE_FILE);

        fs::write(&store, b"{\"edited by the user\": true}").unwrap();
        let state = ensure_store(&base, &store).expect("ensure");
        assert_eq!(state, StoreState::Existing);
        assert_eq!(
            fs::read_to_string(&store).unwrap(),
            "{\"edited by the user\": true}",
            "an existing store must be left exactly as it was"
        );
    }

    #[test]
    fn reset_store_overwrites_an_altered_store() {
        let dir = tempfile::tempdir().unwrap();
        let base = repo_file(BASE_FILE);
        let store = dir.path().join(STORE_FILE);

        fs::write(&store, b"{\"stale\": true}").unwrap();
        reset_store(&base, &store).expect("reset");
        assert_eq!(
            fs::read(&store).unwrap(),
            fs::read(&base).unwrap(),
            "reset must restore the template exactly"
        );
    }

    #[test]
    fn write_filter_refuses_a_missing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such-folder");
        let err = write_filter(&missing, "# filter\n").expect_err("must refuse");
        assert!(matches!(err, DynamicError::DestinationMissing { .. }));
        assert!(
            !missing.exists(),
            "a missing destination must not be created"
        );
    }

    #[test]
    fn write_filter_leaves_the_previous_file_intact_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("game");
        fs::create_dir(&dest).unwrap();
        write_filter(&dest, "first\n").expect("first write");

        // A missing destination fails before any temp file is created, so the previously written
        // filter is untouched.
        let _ = write_filter(&dir.path().join("gone"), "second\n");
        assert_eq!(
            fs::read_to_string(dest.join(FILTER_FILE)).unwrap(),
            "first\n"
        );
    }

    #[test]
    fn write_filter_second_write_wins() {
        let dir = tempfile::tempdir().unwrap();
        write_filter(dir.path(), "first\n").expect("first");
        let path = write_filter(dir.path(), "second\n").expect("second");
        assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");
        assert_eq!(path.file_name().unwrap(), FILTER_FILE);
    }

    #[test]
    fn compile_store_produces_a_complete_filter_with_economy_sections() {
        let dir = tempfile::tempdir().unwrap();
        fs::copy(repo_file("tiers.json"), dir.path().join("tiers.json")).unwrap();
        fs::copy(repo_file("styles.json"), dir.path().join("styles.json")).unwrap();
        fs::copy(fixture("seed-prices.json"), dir.path().join("prices.json")).unwrap();
        let store = dir.path().join(STORE_FILE);
        ensure_store(&repo_file(BASE_FILE), &store).expect("seed");

        let compiled =
            compile_store(&InputPaths::relative_to(dir.path()), &store).expect("compile");

        assert_eq!(compiled.price_source, PriceSource::Live);
        assert!(
            compiled.text.starts_with("# dynamic.filter\n"),
            "the filter must name itself, got: {:?}",
            compiled.text.lines().next()
        );
        assert!(
            compiled.text.contains("# GENERATED FROM dynamic.json"),
            "the filter must name its source"
        );
        assert!(
            compiled.text.contains("Currency"),
            "economy sections must be present — a policy-only filter would lose every currency highlight"
        );
        assert!(compiled.text.contains("Show # rule 0"));
    }
}
