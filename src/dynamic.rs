//! The dynamic filter lifecycle.
//!
//! # Filter-as-store (Phase 18)
//!
//! The `.filter` file itself is the store. [`LoadedFilter::load`] reads it (an absent file is an
//! empty filter; a v0.4 file is migrated in memory only), [`economy_body`] prices the economy
//! section, [`apply_text`] computes the next text for an explicit [`ApplyOp`], and [`apply_file`]
//! is the single writer: guarded against external edits, backed up once, written atomically.
//! Loading never writes. [`apply_edited`] is the overlay's entry to the same single writer: it
//! writes caller-edited text after the [`blocking_errors`] grammar gate.
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
    #[error(
        "`{path}` is not valid UTF-8; FerrisFilter only edits UTF-8 filters and will not convert it"
    )]
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
    #[error(
        "No price data: neither prices.json nor prices-fallback.json was found, so the economy section was left as it is"
    )]
    #[diagnostic(help("Run `ff-cli sync` (or SYNC ECONOMY in the overlay) and apply again."))]
    EconomyUnavailable,

    /// An enabled rule block has a grammar error; Apply was refused before anything was written.
    #[error(
        "Apply refused: {count} enabled rule(s) have grammar errors (first: rule {first_rule}, line {first_line}); nothing was written"
    )]
    #[diagnostic(help("Fix those rules or turn them off, then apply again."))]
    InvalidBlocks {
        path: String,
        count: usize,
        first_rule: usize,
        first_line: usize,
    },

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
                });
            }
        };
        crate::filter::parse(&bytes).map_err(|_| DynamicError::NotUtf8 { path: shown() })?;
        let mut text = String::from_utf8(bytes.clone())
            .map_err(|_| DynamicError::NotUtf8 { path: shown() })?;
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
                });
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
    /// Adopt the file's current bytes as the guard baseline, so the next guarded write replaces an
    /// outside change instead of refusing it (the overlay's OVERWRITE). Only the baseline moves:
    /// the text, which the caller is about to replace, is left as loaded. Nothing is written.
    pub fn rebase_to_disk(&mut self) -> Result<(), DynamicError> {
        self.disk = match std::fs::read(&self.path) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                return Err(DynamicError::FilterUnreadable {
                    path: self.path.display().to_string(),
                    source: e,
                });
            }
        };
        Ok(())
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

/// The CLI writer (no grammar gate; see [`apply_edited`] for the overlay). Never writes on open; the residual TOCTOU window is accepted (see
/// [`crate::safe_write`]). On success the loaded baseline becomes the written bytes; on error
/// `loaded` is unchanged.
pub fn apply_file(loaded: &mut LoadedFilter, op: ApplyOp<'_>) -> Result<ApplyReport, DynamicError> {
    let shown = loaded.path.display().to_string();
    if !loaded.existed() && matches!(op, ApplyOp::Refresh(_) | ApplyOp::RemoveEconomy) {
        return Err(DynamicError::FilterNotFound { path: shown });
    }
    let new_text = apply_text(&loaded.text, op).map_err(|e| match e {
        crate::filter::section::SectionError::NoSection => DynamicError::NoEconomySection {
            path: shown.clone(),
        },
        other => DynamicError::Section {
            path: shown.clone(),
            source: other,
        },
    })?;
    write_new_text(loaded, new_text)
}

/// The one guarded write, shared by [`apply_file`] and [`apply_edited`]. Legacy-spelled `#ff:`
/// metadata and marker lines are rewritten to the current `# ff:` prefix first; nothing else
/// changes. On success the loaded baseline becomes the written bytes; on error `loaded` is
/// unchanged.
fn write_new_text(
    loaded: &mut LoadedFilter,
    new_text: String,
) -> Result<ApplyReport, DynamicError> {
    let new_text = crate::filter::meta::normalize_ff_prefix(&new_text);
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

/// An enabled rule block that blocks Apply because of an Error-severity diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockingError {
    pub block: crate::filter::BlockId,
    /// 1-based position among non-economy blocks, in file order.
    pub rule_index: usize,
    pub diagnostic: crate::filter::FilterDiagnostic,
}

/// The Apply grammar gate: Error diagnostics inside enabled, non-economy blocks. Warnings never
/// block (real foreign filters carry them as opaque lines). With damaged section markers every
/// block counts as a rule.
pub fn blocking_errors(doc: &crate::filter::Document) -> Vec<BlockingError> {
    use crate::filter::Severity;
    let span = crate::filter::section::find(doc).ok().flatten();
    let mut out = Vec::new();
    let mut rule_index = 0usize;
    for b in doc.blocks() {
        if let Some(s) = span
            && s.begin < b.header_line()
            && b.header_line() < s.end
        {
            continue;
        }
        rule_index += 1;
        if !b.is_enabled() {
            continue;
        }
        let owned = b.owned();
        for d in doc.diagnostics() {
            if d.severity == Severity::Error && d.line > owned.start && d.line <= owned.end {
                out.push(BlockingError {
                    block: b.id(),
                    rule_index,
                    diagnostic: d.clone(),
                });
            }
        }
    }
    out
}

/// What to do with the economy section when applying edited text.
#[derive(Debug, Clone, Copy)]
pub enum EconomyOp<'a> {
    Keep,
    Refresh(&'a str),
    Add(&'a str),
    Remove,
}

/// The overlay's Apply: write `edited_text` (with `op` on the economy section) through the shared
/// guarded writer, after refusing enabled-block grammar errors. Nothing is written on any error.
pub fn apply_edited(
    loaded: &mut LoadedFilter,
    edited_text: &str,
    op: EconomyOp<'_>,
) -> Result<ApplyReport, DynamicError> {
    use crate::filter::section::{self, SectionError};
    let shown = loaded.path.display().to_string();
    if !loaded.existed() && edited_text.trim().is_empty() && !matches!(op, EconomyOp::Add(_)) {
        return Err(DynamicError::FilterNotFound { path: shown });
    }
    let doc = crate::filter::parse(edited_text.as_bytes()).map_err(|_| DynamicError::NotUtf8 {
        path: shown.clone(),
    })?;
    let blocking = blocking_errors(&doc);
    if let Some(first) = blocking.first() {
        return Err(DynamicError::InvalidBlocks {
            path: shown,
            count: blocking.len(),
            first_rule: first.rule_index,
            first_line: first.diagnostic.line,
        });
    }
    let new_text = match op {
        EconomyOp::Keep => Ok(edited_text.to_string()),
        EconomyOp::Refresh(body) => section::refresh(edited_text, body),
        EconomyOp::Add(body) => section::add(edited_text, body),
        EconomyOp::Remove => section::remove(edited_text),
    }
    .map_err(|e| match e {
        SectionError::NoSection => DynamicError::NoEconomySection {
            path: shown.clone(),
        },
        other => DynamicError::Section {
            path: shown.clone(),
            source: other,
        },
    })?;
    write_new_text(loaded, new_text)
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
        economy_body(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
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
        let (text, src) = economy_body(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .unwrap();
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
        assert!(written.contains("# ff:name"));
        assert!(!written.contains("#ff:"));
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

    fn bad_rule() -> &'static str {
        "Show # ok\n\tRarity Rare\n\nShow # bad\n\tRarity =< Rare\n"
    }

    #[test]
    fn gate_flags_enabled_error_with_rule_index_and_line() {
        let doc = crate::filter::parse(bad_rule().as_bytes()).unwrap();
        assert!(
            doc.diagnostics()
                .iter()
                .any(|d| d.severity == crate::filter::Severity::Error)
        );
        let e = blocking_errors(&doc);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].rule_index, 2);
        assert_eq!(e[0].diagnostic.line, 5);
    }

    #[test]
    fn gate_ignores_disabled_block() {
        let doc =
            crate::filter::parse(b"Show # ok\n\tRarity Rare\n\n#Show # bad\n#\tRarity =< Rare\n")
                .unwrap();
        assert!(blocking_errors(&doc).is_empty());
    }

    #[test]
    fn gate_ignores_warnings() {
        let doc = crate::filter::parse(b"Show\n\tFrobnicate 3\n").unwrap();
        assert!(
            doc.diagnostics()
                .iter()
                .all(|d| d.severity != crate::filter::Severity::Error)
        );
        assert!(blocking_errors(&doc).is_empty());
    }

    #[test]
    fn gate_numbering_skips_economy_and_counts_after_end() {
        let base = "Show # first\n\tRarity Rare\n\n";
        let with = crate::filter::section::add(base, "Show # eco\n\tRarity Rare\n").unwrap();
        let text = format!("{with}\nShow # after\n\tRarity =< Rare\n");
        let doc = crate::filter::parse(text.as_bytes()).unwrap();
        let e = blocking_errors(&doc);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].rule_index, 2);
    }

    #[test]
    fn gate_passes_filterblade_fixture() {
        let doc = crate::filter::parse(include_bytes!(
            "../tests/fixtures/parser/filterblade.filter"
        ))
        .unwrap();
        assert!(blocking_errors(&doc).is_empty());
    }

    #[test]
    fn gate_works_with_damaged_markers() {
        let text = format!("{}\n#ff:economy-begin only begin\n", bad_rule());
        let doc = crate::filter::parse(text.as_bytes()).unwrap();
        assert_eq!(blocking_errors(&doc).len(), 1);
    }

    #[test]
    fn edited_keep_writes_verbatim_then_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("dynamic.filter");
        let mut l = LoadedFilter::load(&p).unwrap();
        let text = "Show # a\n\tRarity Rare\n";
        let r = apply_edited(&mut l, text, EconomyOp::Keep).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), text);
        assert_eq!(r.bytes, text.len());
        let r2 = apply_edited(&mut l, text, EconomyOp::Keep).unwrap();
        assert_eq!(r2.outcome, crate::safe_write::WriteOutcome::Unchanged);
    }

    #[test]
    fn apply_normalizes_legacy_meta_lines_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("dynamic.filter");
        let body = seed_body();
        let legacy = "# my notes\r\n#ff:name Rings\r\n#ff:style S\r\nShow # a\r\n\tRarity Rare\r\n\r\n#Show\r\n#\tClass \"Bows\"\r\n\r\n#ff:economy-begin old text\r\n\r\nShow # old\r\n\r\n# ff:economy-end\r\n";
        fs::write(&p, legacy).unwrap();
        let mut l = LoadedFilter::load(&p).unwrap();
        // Reading the legacy spelling works: the section is found.
        assert!(l.has_section().unwrap());
        apply_file(&mut l, ApplyOp::Refresh(&body)).unwrap();
        let written = fs::read_to_string(&p).unwrap();
        assert!(!written.contains("#ff:"), "{written}");
        assert!(written.starts_with(
            "# my notes\r\n# ff:name Rings\r\n# ff:style S\r\nShow # a\r\n\tRarity Rare\r\n\r\n#Show\r\n#\tClass \"Bows\"\r\n\r\n# ff:economy-begin old text\r\n"
        ));
        assert!(written.ends_with("# ff:economy-end\r\n"));

        // The overlay's path normalizes too; an all-current file goes out byte-identical.
        let p2 = dir.path().join("other.filter");
        let mut l2 = LoadedFilter::load(&p2).unwrap();
        apply_edited(&mut l2, "#ff:name x\nShow\n", EconomyOp::Keep).unwrap();
        assert_eq!(fs::read_to_string(&p2).unwrap(), "# ff:name x\nShow\n");
        assert_eq!(l2.text(), "# ff:name x\nShow\n");
        let current = "# a\n# ff:name x\nShow # #ff:not-meta\n#Show\n#\tRarity Rare\n";
        let r = apply_edited(&mut l2, current, EconomyOp::Keep).unwrap();
        assert_eq!(fs::read_to_string(&p2).unwrap(), current);
        assert_eq!(r.bytes, current.len());
    }

    #[test]
    fn edited_economy_ops() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("dynamic.filter");
        let body = seed_body();
        let mut l = LoadedFilter::load(&p).unwrap();
        let text = "Show # a\n\tRarity Rare\n";
        apply_edited(&mut l, text, EconomyOp::Add(&body)).unwrap();
        let with = fs::read_to_string(&p).unwrap();
        assert!(crate::filter::section::has_section(&with).unwrap());
        apply_edited(&mut l, &with, EconomyOp::Refresh(&body)).unwrap();
        apply_edited(&mut l, &with, EconomyOp::Remove).unwrap();
        assert!(!fs::read_to_string(&p).unwrap().contains("ff:economy"));
        assert!(matches!(
            apply_edited(&mut l, text, EconomyOp::Refresh(&body)),
            Err(DynamicError::NoEconomySection { .. })
        ));
    }

    #[test]
    fn edited_invalid_blocks_write_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("dynamic.filter");
        fs::write(&p, "Show\n").unwrap();
        let mut l = LoadedFilter::load(&p).unwrap();
        let err = apply_edited(&mut l, bad_rule(), EconomyOp::Keep).expect_err("gate");
        assert!(matches!(
            err,
            DynamicError::InvalidBlocks {
                count: 1,
                first_rule: 2,
                first_line: 5,
                ..
            }
        ));
        assert_eq!(fs::read_to_string(&p).unwrap(), "Show\n");
        assert_eq!(l.text(), "Show\n");
        assert!(!crate::safe_write::backup_path(&p).exists());
    }

    #[test]
    fn edited_absent_file_rules() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("dynamic.filter");
        let mut l = LoadedFilter::load(&p).unwrap();
        for op in [EconomyOp::Keep, EconomyOp::Refresh("x"), EconomyOp::Remove] {
            assert!(matches!(
                apply_edited(&mut l, "  \n", op),
                Err(DynamicError::FilterNotFound { .. })
            ));
        }
        assert!(!p.exists());
        apply_edited(&mut l, "Show\n", EconomyOp::Keep).unwrap();
        assert!(p.exists());
    }

    #[test]
    fn edited_refuses_file_changed_since_load() {
        let dir = tempfile::tempdir().unwrap();
        let p = copy_fixture(dir.path(), "seed-universal.filter");
        let mut l = LoadedFilter::load(&p).unwrap();
        fs::write(&p, b"Show # elsewhere\n").unwrap();
        let err = apply_edited(&mut l, "Show # mine\n", EconomyOp::Keep).expect_err("refuse");
        assert!(matches!(
            err,
            DynamicError::Write(crate::safe_write::WriteError::ChangedOnDisk { .. })
        ));
        assert_eq!(fs::read(&p).unwrap(), b"Show # elsewhere\n");
        assert!(!l.text().contains("mine"));
    }

    #[test]
    fn rebased_baseline_overwrites_an_outside_change() {
        let dir = tempfile::tempdir().unwrap();
        let p = copy_fixture(dir.path(), "seed-universal.filter");
        let mut l = LoadedFilter::load(&p).unwrap();
        fs::write(&p, b"Show # elsewhere\n").unwrap();
        l.rebase_to_disk().unwrap();
        assert_eq!(l.disk(), Some(&b"Show # elsewhere\n"[..]));
        let r = apply_edited(&mut l, "Show # mine\n", EconomyOp::Keep).unwrap();
        // The one-time backup holds the bytes that were overwritten.
        assert!(matches!(
            r.outcome,
            crate::safe_write::WriteOutcome::Written { backup: Some(_) }
        ));
        assert_eq!(fs::read(&p).unwrap(), b"Show # mine\n");
        assert_eq!(
            fs::read(crate::safe_write::backup_path(&p)).unwrap(),
            b"Show # elsewhere\n"
        );
        assert_eq!(l.disk(), Some(&b"Show # mine\n"[..]));

        // A file deleted outside rebases to "absent" and is recreated.
        fs::remove_file(&p).unwrap();
        l.rebase_to_disk().unwrap();
        assert!(!l.existed());
        apply_edited(&mut l, "Show # again\n", EconomyOp::Keep).unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"Show # again\n");
    }

    #[test]
    fn edited_backs_up_once_and_reports_migration() {
        let dir = tempfile::tempdir().unwrap();
        let p = copy_fixture(dir.path(), "v04-dynamic.filter");
        let orig = fs::read(&p).unwrap();
        let mut l = LoadedFilter::load(&p).unwrap();
        assert!(l.migrated());
        let text = l.text().to_string();
        let r = apply_edited(&mut l, &text, EconomyOp::Keep).unwrap();
        assert!(r.migrated);
        assert_eq!(fs::read(crate::safe_write::backup_path(&p)).unwrap(), orig);
        let r2 = apply_edited(&mut l, &format!("{text}\n#x\n"), EconomyOp::Keep).unwrap();
        assert!(!r2.migrated);
        assert_eq!(fs::read(crate::safe_write::backup_path(&p)).unwrap(), orig);
    }
}
