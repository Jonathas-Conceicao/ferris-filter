//! The combined Apply (UAT2-10): CONFIG edits and RULES edits saved and applied as one step.
//!
//! Validate everything, then write all-or-none. [`prepare`] is pure with respect to the files it
//! will replace: it validates the CONFIG edits, renders the filter from the in-memory (pending)
//! styles and tier cutoffs, runs the grammar and economy-marker gates, and compares every
//! participating file with its load baseline. Any refusal happens there, before a single byte or
//! backup is written. [`commit`] then writes `styles.json`, `tiers.json` and the filter, in that
//! order, through [`crate::safe_write::write_all_or_none`]: a failed later rename restores the
//! earlier files from the in-memory baselines. Baselines in [`ConfigState`] and [`LoadedFilter`]
//! move only after the write succeeded.
//!
//! Layer rule (same as `config_state.rs` / `apply.rs`): no GUI-framework types here, so every
//! rule is provable with temp roots.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::compile::{InputPaths, PriceSource, PriceSourcePolicy};
use crate::dynamic::{self, DynamicError, EconomyOp, LoadedFilter};
use crate::overlay::apply::invalid_blocks_message;
use crate::overlay::config_state::{ConfigState, ConfigWrite};
use crate::overlay::rules::EconomyPlan;
use crate::safe_write::{self, AllOrNoneError, Target, WriteOutcome};

/// One of the (up to) three files an Apply writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskFile {
    Styles,
    Tiers,
    Filter,
}

impl DiskFile {
    /// The name the user sees: the two config file names, or the filter's own file name.
    pub fn label(&self, filter_path: &Path) -> String {
        match self {
            DiskFile::Styles => "styles.json".to_string(),
            DiskFile::Tiers => "tiers.json".to_string(),
            DiskFile::Filter => filter_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| filter_path.display().to_string()),
        }
    }
}

/// The RULES half of an Apply.
pub struct FilterInput<'a> {
    pub loaded: &'a LoadedFilter,
    /// The working document's text rendered with the pending styles, or why that failed.
    pub styled: Result<String, String>,
    pub plan: EconomyPlan,
}

/// Why an Apply did not happen. Nothing was changed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyAllError {
    /// A finished sentence: invalid config, grammar errors, damaged markers, no prices, ...
    Invalid(String),
    /// These files changed since they were loaded.
    ChangedOnDisk(Vec<DiskFile>),
    /// The write itself failed (and was rolled back where needed). A finished sentence.
    Write(String),
}

impl ApplyAllError {
    /// The refusal as one sentence. `ChangedOnDisk` names the files by their conventional
    /// names; the caller may use [`DiskFile::label`] for the real filter name.
    pub fn message(&self) -> String {
        match self {
            ApplyAllError::Invalid(text) | ApplyAllError::Write(text) => text.clone(),
            ApplyAllError::ChangedOnDisk(files) => {
                let names: Vec<&str> = files
                    .iter()
                    .map(|f| match f {
                        DiskFile::Styles => "styles.json",
                        DiskFile::Tiers => "tiers.json",
                        DiskFile::Filter => "The filter",
                    })
                    .collect();
                format!("Changed on disk since it was loaded: {}.", names.join(", "))
            }
        }
    }
}

struct ConfigPart {
    write: ConfigWrite,
    styles_path: PathBuf,
    tiers_path: PathBuf,
    styles_guard: Option<Vec<u8>>,
    tiers_guard: Option<Vec<u8>>,
}

struct FilterPart {
    path: PathBuf,
    text: String,
    migrated: bool,
    guard: Option<Vec<u8>>,
}

/// A fully validated Apply, ready for [`commit`].
pub struct Prepared {
    config: Option<ConfigPart>,
    filter: Option<FilterPart>,
    used_fallback: bool,
}

impl Prepared {
    /// The economy section was priced from the early-league fallback pair.
    pub fn used_fallback(&self) -> bool {
        self.used_fallback
    }
}

/// What the filter half of a commit did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterCommitted {
    pub path: PathBuf,
    /// False when the new bytes equalled the file's bytes.
    pub written: bool,
    /// Set when this commit created the one-time `.ffbak`.
    pub backup: Option<PathBuf>,
    /// The loaded filter was a v0.4 file migrated in memory before this write.
    pub migrated: bool,
}

/// What a commit did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    pub styles_written: bool,
    pub tiers_written: bool,
    /// `None` when there was no filter part (none loaded, or nothing to write).
    pub filter: Option<FilterCommitted>,
    pub used_fallback: bool,
}

fn read_now(path: &Path) -> Result<Option<Vec<u8>>, ApplyAllError> {
    match fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(ApplyAllError::Write(format!(
            "Nothing was applied: cannot read {}: {e}.",
            path.display()
        ))),
    }
}

/// Validate everything an Apply will write, without writing or backing up anything.
///
/// `config` is the CONFIG state (`None` = no config part); `filter` is the RULES half (`None` =
/// no filter loaded). With `overwrite`, files that changed on disk are not a refusal: their
/// current bytes become the guard baseline for the write (the states themselves move only on a
/// successful [`commit`]).
pub fn prepare(
    root: &Path,
    config: Option<&ConfigState>,
    filter: Option<FilterInput<'_>>,
    overwrite: bool,
) -> Result<Prepared, ApplyAllError> {
    // 1. CONFIG: validity and rendering.
    let config_write = match config {
        Some(c) => Some(
            c.prepare_write()
                .map_err(|e| ApplyAllError::Invalid(e.message()))?,
        ),
        None => None,
    };

    // 2. RULES: styled text, economy pricing from the pending config, grammar and markers.
    let mut used_fallback = false;
    let mut filter_part: Option<FilterPart> = None;
    if let Some(input) = filter {
        let edited = input.styled.map_err(ApplyAllError::Invalid)?;
        let body = match input.plan {
            EconomyPlan::Refresh | EconomyPlan::Add => {
                let paths = InputPaths::relative_to(root);
                let priced = match config {
                    Some(c) => dynamic::economy_body_with(
                        &paths,
                        PriceSourcePolicy::Auto,
                        c.pending_styles(),
                        c.pending_tiers(),
                    ),
                    None => dynamic::economy_body(&paths, PriceSourcePolicy::Auto),
                };
                let (text, source) = priced.map_err(|e| ApplyAllError::Invalid(e.to_string()))?;
                used_fallback = matches!(source, PriceSource::Fallback(_));
                Some(text)
            }
            EconomyPlan::Keep | EconomyPlan::Remove => None,
        };
        let op = match (input.plan, body.as_deref()) {
            (EconomyPlan::Refresh, Some(b)) => EconomyOp::Refresh(b),
            (EconomyPlan::Add, Some(b)) => EconomyOp::Add(b),
            (EconomyPlan::Remove, _) => EconomyOp::Remove,
            _ => EconomyOp::Keep,
        };
        match dynamic::prepare_edited(input.loaded, &edited, op) {
            Ok(text) => {
                filter_part = Some(FilterPart {
                    path: input.loaded.path().to_path_buf(),
                    text,
                    migrated: input.loaded.migrated(),
                    guard: input.loaded.disk().map(<[u8]>::to_vec),
                });
            }
            // Nothing to write to a filter that does not exist and has no rules: skip the filter
            // part, the config part still applies.
            Err(DynamicError::FilterNotFound { .. }) if !input.loaded.existed() => {}
            Err(DynamicError::InvalidBlocks {
                count,
                first_rule,
                first_line,
                ..
            }) => {
                return Err(ApplyAllError::Invalid(invalid_blocks_message(
                    count, first_rule, first_line,
                )));
            }
            Err(other) => return Err(ApplyAllError::Invalid(other.to_string())),
        }
    }

    // 3. Changed on disk: every participating file against its baseline. A config file without a
    // pending change does not participate: it is neither guarded nor written (CR-01).
    let mut changed = Vec::new();
    let mut config_part = None;
    if let (Some(c), Some(write)) = (config, config_write) {
        let styles_path = c.styles_path();
        let tiers_path = c.tiers_path();
        let (styles_base, tiers_base) = c.baselines();
        let styles_now = read_now(&styles_path)?;
        let tiers_now = read_now(&tiers_path)?;
        if write.styles_changed() && styles_now.as_deref() != Some(styles_base) {
            changed.push(DiskFile::Styles);
        }
        if write.tiers_changed() && tiers_now.as_deref() != Some(tiers_base) {
            changed.push(DiskFile::Tiers);
        }
        let (styles_guard, tiers_guard) = if overwrite {
            (styles_now, tiers_now)
        } else {
            (Some(styles_base.to_vec()), Some(tiers_base.to_vec()))
        };
        config_part = Some(ConfigPart {
            write,
            styles_path,
            tiers_path,
            styles_guard,
            tiers_guard,
        });
    }
    if let Some(f) = filter_part.as_mut() {
        let now = read_now(&f.path)?;
        if now != f.guard {
            changed.push(DiskFile::Filter);
        }
        if overwrite {
            f.guard = now;
        }
    }
    if !overwrite && !changed.is_empty() {
        return Err(ApplyAllError::ChangedOnDisk(changed));
    }

    Ok(Prepared {
        config: config_part,
        filter: filter_part,
        used_fallback,
    })
}

/// Write a [`Prepared`] Apply: the one-time filter backup first, then styles, tiers and filter
/// all-or-none. States adopt their new baselines only on success.
pub fn commit(
    p: Prepared,
    config: Option<&mut ConfigState>,
    loaded: Option<&mut LoadedFilter>,
) -> Result<Committed, ApplyAllError> {
    commit_with(p, config, loaded, safe_write::write_all_or_none)
}

pub(crate) fn commit_with(
    p: Prepared,
    config: Option<&mut ConfigState>,
    loaded: Option<&mut LoadedFilter>,
    write: impl FnOnce(&[Target]) -> Result<Vec<WriteOutcome>, AllOrNoneError>,
) -> Result<Committed, ApplyAllError> {
    let Prepared {
        config: config_part,
        filter: filter_part,
        used_fallback,
    } = p;

    // The one-time `.ffbak` is a copy of bytes already on disk, so it never needs a rollback.
    let mut backup = None;
    if let Some(f) = &filter_part
        && let Some(original) = f.guard.as_deref()
        && original != f.text.as_bytes()
    {
        backup = safe_write::ensure_backup(&f.path, original)
            .map_err(|e| ApplyAllError::Write(format!("Nothing was applied: {e}.")))?;
    }

    // Targets in write order; remember which slot is which.
    let mut targets: Vec<Target> = Vec::new();
    let (mut styles_at, mut tiers_at, mut filter_at) = (None, None, None);
    if let Some(c) = &config_part {
        if c.write.styles_changed() {
            styles_at = Some(targets.len());
            targets.push(Target {
                path: &c.styles_path,
                loaded: c.styles_guard.as_deref(),
                new: &c.write.styles_new,
            });
        }
        if c.write.tiers_changed() {
            tiers_at = Some(targets.len());
            targets.push(Target {
                path: &c.tiers_path,
                loaded: c.tiers_guard.as_deref(),
                new: &c.write.tiers_new,
            });
        }
    }
    if let Some(f) = &filter_part {
        filter_at = Some(targets.len());
        targets.push(Target {
            path: &f.path,
            loaded: f.guard.as_deref(),
            new: f.text.as_bytes(),
        });
    }

    let outcomes = match write(&targets) {
        Ok(o) => o,
        Err(e) => {
            if let Some(path) = e.changed_on_disk_path() {
                let which = [
                    (styles_at, DiskFile::Styles),
                    (tiers_at, DiskFile::Tiers),
                    (filter_at, DiskFile::Filter),
                ]
                .into_iter()
                .find(|(at, _)| at.is_some_and(|i| targets[i].path == Path::new(path)))
                .map(|(_, f)| f);
                if let Some(f) = which {
                    return Err(ApplyAllError::ChangedOnDisk(vec![f]));
                }
            }
            return Err(ApplyAllError::Write(match &e {
                AllOrNoneError::NothingWritten(w) => {
                    format!(
                        "Nothing was applied: {}.",
                        w.to_string().trim_end_matches('.')
                    )
                }
                other => other.to_string(),
            }));
        }
    };
    let written = |at: Option<usize>| {
        at.is_some_and(|i| matches!(outcomes.get(i), Some(WriteOutcome::Written { .. })))
    };
    let (styles_written, tiers_written, filter_was_written) =
        (written(styles_at), written(tiers_at), written(filter_at));

    // Success: only now do the baselines move.
    if let (Some(part), Some(state)) = (config_part, config) {
        state.adopt(part.write);
    }
    let filter = filter_part.map(|f| {
        if let Some(l) = loaded {
            l.adopt_written(f.text);
        }
        FilterCommitted {
            path: f.path,
            written: filter_was_written,
            backup,
            migrated: f.migrated,
        }
    });
    Ok(Committed {
        styles_written,
        tiers_written,
        filter,
        used_fallback,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::section;
    use crate::models::tiers::TierEntry;

    const RULES: &str = "Show # a\n\tRarity Rare\n";

    fn repo_file(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
    }

    /// Temp app root (styles, tiers, seed prices) + temp game dir holding a filter with an
    /// economy section. Never the real game folder or the repo's committed files.
    struct Fixture {
        root: tempfile::TempDir,
        game: tempfile::TempDir,
        filter: PathBuf,
    }

    impl Fixture {
        fn new() -> Fixture {
            let root = tempfile::tempdir().unwrap();
            for f in ["styles.json", "tiers.json"] {
                fs::copy(repo_file(f), root.path().join(f)).unwrap();
            }
            fs::copy(
                repo_file("tests/fixtures/seed-prices.json"),
                root.path().join("prices.json"),
            )
            .unwrap();
            let game = tempfile::tempdir().unwrap();
            let filter = game.path().join("dynamic.filter");
            let paths = InputPaths::relative_to(root.path());
            let (body, _) = dynamic::economy_body(&paths, PriceSourcePolicy::Auto).unwrap();
            fs::write(&filter, section::add(RULES, &body).unwrap()).unwrap();
            Fixture { root, game, filter }
        }

        fn config(&self) -> ConfigState {
            ConfigState::load(self.root.path()).unwrap()
        }

        fn loaded(&self) -> LoadedFilter {
            LoadedFilter::load(&self.filter).unwrap()
        }

        fn bytes(&self) -> [Vec<u8>; 3] {
            [
                fs::read(self.root.path().join("styles.json")).unwrap(),
                fs::read(self.root.path().join("tiers.json")).unwrap(),
                fs::read(&self.filter).unwrap(),
            ]
        }

        fn no_ffbak(&self) {
            for dir in [self.root.path(), self.game.path()] {
                for e in fs::read_dir(dir).unwrap() {
                    let name = e.unwrap().file_name().to_string_lossy().into_owned();
                    assert!(!name.ends_with(".ffbak"), "unexpected backup {name}");
                }
            }
        }
    }

    fn input(l: &LoadedFilter, plan: EconomyPlan) -> FilterInput<'_> {
        FilterInput {
            loaded: l,
            styled: Ok(l.text().to_string()),
            plan,
        }
    }

    fn tier(s: &ConfigState, name: &str) -> f64 {
        s.pending_tiers()
            .iter()
            .find(|e| e.tier == name)
            .unwrap()
            .min
    }

    #[test]
    fn config_only_edit_rerenders_with_pending_tiers() {
        let fx = Fixture::new();
        let mut cfg = fx.config();
        let mut lf = fx.loaded();
        let before = fx.bytes();

        cfg.set_tier_text("S", "1000000");
        let prepared = prepare(
            fx.root.path(),
            Some(&cfg),
            Some(input(&lf, EconomyPlan::Refresh)),
            false,
        )
        .unwrap();
        let done = commit(prepared, Some(&mut cfg), Some(&mut lf)).unwrap();
        assert!(done.tiers_written);
        assert!(!done.styles_written);
        assert!(done.filter.as_ref().unwrap().written);

        let after = fx.bytes();
        assert_eq!(after[0], before[0], "styles.json untouched");
        let tiers: Vec<TierEntry> = serde_json::from_slice(&after[1]).unwrap();
        assert_eq!(
            tiers.iter().find(|t| t.tier == "S").unwrap().min,
            1_000_000.0
        );
        assert_ne!(after[2], before[2]);

        // The economy section is the one priced from the pending tiers.
        let (body, _) = dynamic::economy_body_with(
            &InputPaths::relative_to(fx.root.path()),
            PriceSourcePolicy::Auto,
            cfg.pending_styles(),
            cfg.pending_tiers(),
        )
        .unwrap();
        let expect =
            section::refresh(&String::from_utf8(before[2].clone()).unwrap(), &body).unwrap();
        assert_eq!(String::from_utf8(after[2].clone()).unwrap(), expect);
        // Baselines moved.
        assert_eq!(lf.disk(), Some(&after[2][..]));
        assert_eq!(cfg.baselines().1, &after[1][..]);
        assert!(!cfg.is_dirty());
    }

    #[test]
    fn any_refusal_writes_nothing() {
        let fx = Fixture::new();
        let before = fx.bytes();
        let lf = fx.loaded();

        // (i) invalid tier text with a valid filter edit.
        let mut cfg = fx.config();
        cfg.set_tier_text("S", "abc");
        let r = prepare(
            fx.root.path(),
            Some(&cfg),
            Some(input(&lf, EconomyPlan::Keep)),
            false,
        );
        assert!(matches!(r, Err(ApplyAllError::Invalid(_))));

        // (ii) valid config edit, enabled rule with a grammar error.
        let mut cfg = fx.config();
        cfg.set_tier_text("S", "35");
        let bad = FilterInput {
            loaded: &lf,
            styled: Ok(format!("{}\nShow # bad\n\tRarity =< Rare\n", lf.text())),
            plan: EconomyPlan::Refresh,
        };
        let r = prepare(fx.root.path(), Some(&cfg), Some(bad), false);
        assert!(matches!(r, Err(ApplyAllError::Invalid(ref m)) if m.contains("grammar")));

        // (iii) styled text unavailable.
        let nostyle = FilterInput {
            loaded: &lf,
            styled: Err("styles are unreadable".to_string()),
            plan: EconomyPlan::Keep,
        };
        let r = prepare(fx.root.path(), Some(&cfg), Some(nostyle), false);
        assert_eq!(
            r.err(),
            Some(ApplyAllError::Invalid("styles are unreadable".to_string()))
        );

        // (iv) damaged economy markers with a Refresh plan.
        let damaged = FilterInput {
            loaded: &lf,
            styled: Ok(lf.text().replace("# ff:economy-end", "")),
            plan: EconomyPlan::Refresh,
        };
        let r = prepare(fx.root.path(), Some(&cfg), Some(damaged), false);
        assert!(matches!(r, Err(ApplyAllError::Invalid(_))));

        assert_eq!(fx.bytes(), before);
        fx.no_ffbak();
    }

    #[test]
    fn third_rename_failure_rolls_back_all() {
        let fx = Fixture::new();
        let mut cfg = fx.config();
        let mut lf = fx.loaded();
        let before = fx.bytes();

        cfg.set_tier_text("S", "1000000");
        assert!(cfg.edit_style("S", crate::overlay::config_state::StyleEdit::Font(Some(12))));
        let tier_before = tier(&cfg, "S");

        let prepared = prepare(
            fx.root.path(),
            Some(&cfg),
            Some(input(&lf, EconomyPlan::Refresh)),
            false,
        )
        .unwrap();
        let mut calls = 0;
        let r = commit_with(prepared, Some(&mut cfg), Some(&mut lf), |targets| {
            assert_eq!(targets.len(), 3);
            safe_write::write_all_or_none_with(
                targets,
                |file, target| {
                    calls += 1;
                    if calls == 3 {
                        Err(tempfile::PersistError {
                            error: io::Error::from_raw_os_error(2),
                            file,
                        })
                    } else {
                        file.persist(target)
                    }
                },
                |_| {},
            )
        });
        assert!(matches!(r, Err(ApplyAllError::Write(_))), "{r:?}");
        assert_eq!(
            fx.bytes(),
            before,
            "all three files hold their original bytes"
        );
        // Baselines unchanged: still dirty, still pointing at the old bytes.
        assert!(cfg.is_dirty());
        assert_eq!(tier(&cfg, "S"), tier_before);
        assert_eq!(lf.disk(), Some(&before[2][..]));
        assert_eq!(cfg.baselines().0, &before[0][..]);

        // A following normal prepare + commit succeeds.
        let prepared = prepare(
            fx.root.path(),
            Some(&cfg),
            Some(input(&lf, EconomyPlan::Refresh)),
            false,
        )
        .unwrap();
        let done = commit(prepared, Some(&mut cfg), Some(&mut lf)).unwrap();
        assert!(done.styles_written && done.tiers_written);
        assert!(done.filter.unwrap().written);
    }

    #[test]
    fn changed_on_disk_names_files_and_overwrite_writes_all() {
        let fx = Fixture::new();
        let mut cfg = fx.config();
        let mut lf = fx.loaded();
        cfg.set_tier_text("S", "35");

        // Outside edits to tiers.json (still valid) and the filter.
        let tiers_path = fx.root.path().join("tiers.json");
        let mut other: Vec<TierEntry> =
            serde_json::from_slice(&fs::read(&tiers_path).unwrap()).unwrap();
        other.iter_mut().find(|t| t.tier == "F").unwrap().min += 0.01;
        fs::write(&tiers_path, crate::loader::render_tiers(&other)).unwrap();
        let outside_filter = format!("{}# outside\n", fs::read_to_string(&fx.filter).unwrap());
        fs::write(&fx.filter, &outside_filter).unwrap();
        let before = fx.bytes();

        let r = prepare(
            fx.root.path(),
            Some(&cfg),
            Some(input(&lf, EconomyPlan::Keep)),
            false,
        );
        assert_eq!(
            r.err(),
            Some(ApplyAllError::ChangedOnDisk(vec![
                DiskFile::Tiers,
                DiskFile::Filter
            ]))
        );
        assert_eq!(fx.bytes(), before, "nothing written by a refusal");

        let prepared = prepare(
            fx.root.path(),
            Some(&cfg),
            Some(input(&lf, EconomyPlan::Keep)),
            true,
        )
        .unwrap();
        // The states have not moved yet.
        assert!(cfg.is_dirty());
        let done = commit(prepared, Some(&mut cfg), Some(&mut lf)).unwrap();
        assert!(done.tiers_written);
        let after = fx.bytes();
        let tiers: Vec<TierEntry> = serde_json::from_slice(&after[1]).unwrap();
        assert_eq!(tiers.iter().find(|t| t.tier == "S").unwrap().min, 35.0);
        // The in-memory (pre-outside-edit) filter text replaced the outside change.
        assert!(
            !String::from_utf8(after[2].clone())
                .unwrap()
                .contains("# outside")
        );
        assert_eq!(cfg.baselines().1, &after[1][..]);
        assert_eq!(lf.disk(), Some(&after[2][..]));
        // The overwritten outside edit is preserved in the one-time backup.
        assert_eq!(
            fs::read(safe_write::backup_path(&fx.filter)).unwrap(),
            outside_filter.as_bytes()
        );
    }

    #[test]
    fn clean_config_is_neither_guarded_nor_written() {
        let fx = Fixture::new();
        let mut cfg = fx.config();
        let mut lf = fx.loaded();
        // Outside edit to tiers.json while CONFIG holds no edits.
        let tiers_path = fx.root.path().join("tiers.json");
        let mut outside = fs::read(&tiers_path).unwrap();
        outside.push(b'\n');
        fs::write(&tiers_path, &outside).unwrap();
        let before = fx.bytes();

        for overwrite in [false, true] {
            let p = prepare(
                fx.root.path(),
                Some(&cfg),
                Some(input(&lf, EconomyPlan::Keep)),
                overwrite,
            )
            .unwrap();
            let done = commit(p, Some(&mut cfg), Some(&mut lf)).unwrap();
            assert!(!done.styles_written && !done.tiers_written, "{overwrite}");
            assert_eq!(fx.bytes(), before, "{overwrite}");
        }
    }

    #[test]
    fn no_filter_writes_config_only() {
        let fx = Fixture::new();
        let mut cfg = fx.config();
        let before = fx.bytes();
        cfg.set_tier_text("S", "35");
        let prepared = prepare(fx.root.path(), Some(&cfg), None, false).unwrap();
        let done = commit(prepared, Some(&mut cfg), None).unwrap();
        assert!(done.tiers_written);
        assert!(done.filter.is_none());
        let after = fx.bytes();
        assert_ne!(after[1], before[1]);
        assert_eq!(after[2], before[2]);
        fx.no_ffbak();
    }

    #[test]
    fn first_write_of_foreign_filter_keeps_ffbak() {
        let game = tempfile::tempdir().unwrap();
        let path = game.path().join("foreign.filter");
        let original = "Show # foreign\n\tRarity Rare\n";
        fs::write(&path, original).unwrap();
        let mut lf = LoadedFilter::load(&path).unwrap();
        let root = tempfile::tempdir().unwrap();

        fn edit<'a>(lf: &'a LoadedFilter, text: &str) -> FilterInput<'a> {
            FilterInput {
                loaded: lf,
                styled: Ok(text.to_string()),
                plan: EconomyPlan::Keep,
            }
        }

        let p = prepare(root.path(), None, Some(edit(&lf, "Show # one\n")), false).unwrap();
        let done = commit(p, None, Some(&mut lf)).unwrap();
        let f = done.filter.unwrap();
        assert!(f.written);
        assert_eq!(f.backup, Some(safe_write::backup_path(&path)));
        assert_eq!(
            fs::read(safe_write::backup_path(&path)).unwrap(),
            original.as_bytes()
        );

        // A second edit does not replace the backup.
        let p = prepare(root.path(), None, Some(edit(&lf, "Show # two\n")), false).unwrap();
        let done = commit(p, None, Some(&mut lf)).unwrap();
        let f = done.filter.unwrap();
        assert!(f.written);
        assert_eq!(f.backup, None);
        assert_eq!(
            fs::read(safe_write::backup_path(&path)).unwrap(),
            original.as_bytes()
        );

        // Identical bytes: Ok, but not written.
        let p = prepare(root.path(), None, Some(edit(&lf, "Show # two\n")), false).unwrap();
        let done = commit(p, None, Some(&mut lf)).unwrap();
        assert!(!done.filter.unwrap().written);
        assert_eq!(fs::read_to_string(&path).unwrap(), "Show # two\n");
    }

    #[test]
    fn absent_empty_filter_is_skipped_not_refused() {
        let fx = Fixture::new();
        let mut cfg = fx.config();
        let missing = fx.game.path().join("none.filter");
        let mut lf = LoadedFilter::load(&missing).unwrap();
        cfg.set_tier_text("S", "35");
        let filter = FilterInput {
            loaded: &lf,
            styled: Ok(String::new()),
            plan: EconomyPlan::Keep,
        };
        let p = prepare(fx.root.path(), Some(&cfg), Some(filter), false).unwrap();
        let done = commit(p, Some(&mut cfg), Some(&mut lf)).unwrap();
        assert!(done.tiers_written);
        assert!(done.filter.is_none());
        assert!(!missing.exists());
        assert!(!cfg.is_dirty());
    }
}
