//! The callable compile pipeline.
//!
//! This module is the layer that was missing before v0.4: `emitter::generate` has always been a
//! pure function, but the orchestration *above* it — resolving input paths, choosing between live
//! and fallback prices, deriving cross-rates, pricing tiers — lived inside a private CLI function
//! that hardcoded working-directory-relative paths and printed its own warnings. That made it
//! unusable from any front end other than the CLI.
//!
//! The pipeline is split in two (11-CONTEXT.md D-01):
//!
//! 1. [`load_inputs`] — all I/O. Every expected failure surfaces here, and it reports which price
//!    source it actually used so the caller can present that however it likes.
//! 2. [`compile`] — pure. Turns loaded inputs into filter text.
//!
//! The split exists because a GUI compiles repeatedly (once per Apply) but only needs to read
//! `prices.json` once — it can hold a [`CompileInputs`] and call [`compile`] as often as it wants.
//!
//! Two rules this module must keep:
//!
//! - **No `println!` / `eprintln!`.** The library reports; callers present (D-09).
//! - **No implicit working-directory reads.** Every path comes from [`InputPaths`] (D-03).

use std::path::{Path, PathBuf};

use miette::Diagnostic;
use thiserror::Error;

use crate::defaults::{self, Builtin};
use crate::emitter::{self, FilterIdentity, GenerateError};
use crate::loader::{self, LoadError};
use crate::models::config::{Config, StyleSet};
use crate::models::prices::PricesFile;
use crate::models::tiers::{TierEntry, ValidatedTiers};
use crate::pricer::{self, PriceRateError, PricedData};

/// Conventional file name for the policy config.
const CONFIG_FILE: &str = "config.json";
/// Conventional file name for the named styles.
const STYLES_FILE: &str = "styles.json";
/// Committed default read when the live `styles.json` is absent (DEF-03). Never written.
const STYLES_DEFAULT_FILE: &str = "styles-default.json";
/// Conventional file name for live synced prices.
const PRICES_FILE: &str = "prices.json";
/// Conventional file name for the tier cutoffs that pair with live prices.
const TIERS_FILE: &str = "tiers.json";
/// Committed default read when the live `tiers.json` is absent (DEF-03). Never written.
const TIERS_DEFAULT_FILE: &str = "tiers-default.json";
/// Conventional file name for the committed early-league price fallback.
const PRICES_FALLBACK_FILE: &str = "prices-fallback.json";
/// Conventional file name for the tier cutoffs calibrated to the fallback prices.
const TIERS_FALLBACK_FILE: &str = "tiers-fallback.json";

/// Where the compiler reads its inputs from.
///
/// Every path is supplied by the caller — the library never resolves a bare relative name against
/// the process working directory. The CLI builds these with `InputPaths::relative_to(".")`, which
/// reproduces its historical CWD-relative behavior exactly; an overlay launched from a shortcut
/// passes whatever root it resolved for itself.
///
/// This struct is the **only** place the conventional file names appear in the library.
///
/// `styles` / `tiers` are the **live** paths: every write targets them. Reads go through
/// [`InputPaths::styles_source`] / [`InputPaths::tiers_source`], which fall back to the committed
/// `-default.json` file when the live one is absent (DEF-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputPaths {
    pub config: PathBuf,
    pub styles: PathBuf,
    /// The committed default read when the live `styles` file is absent; `None` for an explicit
    /// override (`--styles`), which never falls back.
    pub styles_default: Option<DefaultSource>,
    pub prices: PathBuf,
    pub tiers: PathBuf,
    /// The committed default read when the live `tiers` file is absent; `None` for an explicit
    /// override (`--tiers`), which never falls back.
    pub tiers_default: Option<DefaultSource>,
    pub prices_fallback: DefaultSource,
    pub tiers_fallback: DefaultSource,
}

/// Where a default (the `-default` / `-fallback` copy of a shipped file) is read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultSource {
    /// A file on disk (`generate`, and the overlay until it moves to [`InputPaths::in_home`]).
    File(PathBuf),
    /// The copy compiled into the program.
    Embedded(&'static Builtin),
}

impl DefaultSource {
    /// The file path, when this default is a file.
    pub fn as_file(&self) -> Option<&Path> {
        match self {
            DefaultSource::File(p) => Some(p),
            DefaultSource::Embedded(_) => None,
        }
    }
}

impl From<&DefaultSource> for Source {
    fn from(d: &DefaultSource) -> Self {
        match d {
            DefaultSource::File(p) => Source::File(p.clone()),
            DefaultSource::Embedded(b) => Source::Embedded(b),
        }
    }
}

/// What a read actually uses: a file on disk or an embedded built-in copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    File(PathBuf),
    Embedded(&'static Builtin),
}

impl Source {
    /// Name for messages: the full path of a file, or `<file> (built in)`.
    pub fn label(&self) -> String {
        match self {
            Source::File(p) => p.display().to_string(),
            Source::Embedded(b) => b.label(),
        }
    }

    /// The raw bytes of the source.
    pub fn bytes(&self) -> std::io::Result<Vec<u8>> {
        match self {
            Source::File(p) => std::fs::read(p),
            Source::Embedded(b) => Ok(b.text.as_bytes().to_vec()),
        }
    }

    /// The file path, when this source is a file.
    pub fn as_file(&self) -> Option<&Path> {
        match self {
            Source::File(p) => Some(p),
            Source::Embedded(_) => None,
        }
    }
}

impl InputPaths {
    /// The conventional file names as bare relative paths, resolved by the caller's process
    /// against its own working directory.
    ///
    /// This is what the CLI uses: it yields `config.json`, `prices.json`, … with no directory
    /// prefix, exactly the strings `cmd_generate` passed before v0.4. Note this is deliberately
    /// **not** `relative_to(".")`, which would produce `./config.json` (`.\config.json` on
    /// Windows) and change every rendered error message.
    pub fn conventional() -> Self {
        Self::relative_to("")
    }

    /// The conventional file names, resolved against `root`.
    ///
    /// An empty `root` yields bare relative names — see [`InputPaths::conventional`].
    pub fn relative_to(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        InputPaths {
            config: root.join(CONFIG_FILE),
            styles: root.join(STYLES_FILE),
            styles_default: Some(DefaultSource::File(root.join(STYLES_DEFAULT_FILE))),
            prices: root.join(PRICES_FILE),
            tiers: root.join(TIERS_FILE),
            tiers_default: Some(DefaultSource::File(root.join(TIERS_DEFAULT_FILE))),
            prices_fallback: DefaultSource::File(root.join(PRICES_FALLBACK_FILE)),
            tiers_fallback: DefaultSource::File(root.join(TIERS_FALLBACK_FILE)),
        }
    }

    /// The per-user folder `root`: live files joined onto it, every default and the fallback
    /// pair embedded in the program. Reads with nothing on disk use the built-ins and create
    /// nothing. `config` is read only by `generate`; the overlay class catalog uses
    /// `defaults::CONFIG` and `apply` never reads it.
    pub fn in_home(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        InputPaths {
            config: root.join(CONFIG_FILE),
            styles: root.join(STYLES_FILE),
            styles_default: Some(DefaultSource::Embedded(&defaults::STYLES_DEFAULT)),
            prices: root.join(PRICES_FILE),
            tiers: root.join(TIERS_FILE),
            tiers_default: Some(DefaultSource::Embedded(&defaults::TIERS_DEFAULT)),
            prices_fallback: DefaultSource::Embedded(&defaults::PRICES_FALLBACK),
            tiers_fallback: DefaultSource::Embedded(&defaults::TIERS_FALLBACK),
        }
    }

    /// [`InputPaths::in_home`] for the resolved app folder.
    pub fn from_dirs(dirs: &crate::locations::AppDirs) -> Self {
        Self::in_home(&dirs.root)
    }

    /// Override just the config path, leaving the rest untouched — this is what
    /// `generate --config <path>` needs.
    pub fn with_config(mut self, path: impl Into<PathBuf>) -> Self {
        self.config = path.into();
        self
    }

    /// Override just the styles path, leaving the rest untouched. Lets generate/apply point
    /// at a non-default styles file, e.g. for an experimental config (OD-2).
    ///
    /// An explicit path has no default: if it is absent the read fails, it never falls back.
    pub fn with_styles(mut self, path: impl Into<PathBuf>) -> Self {
        self.styles = path.into();
        self.styles_default = None;
        self
    }

    /// Override just the tier-cutoff path, leaving prices and everything else untouched.
    ///
    /// This exists to decouple *which prices* from *which cutoffs*. `--early-game` swaps both
    /// together, which is right when there is no economy data at all. But early in a live league
    /// there IS data, and it is the chaos-denominated cutoffs in `tiers.json` that misbehave:
    /// chaos is expensive in a young economy, so measuring against it compresses the whole scale
    /// downward (observed 2026-09-06 in Forbidden Rites — Divine landed in A and Exalted in F).
    /// Pointing this at the exalt-denominated `tiers-fallback.json` while keeping live prices
    /// restores sane placement without touching the live `tiers.json` (the test oracle reads
    /// `tiers-default.json`).
    ///
    /// An explicit path has no default: if it is absent the read fails, it never falls back.
    pub fn with_tiers(mut self, path: impl Into<PathBuf>) -> Self {
        self.tiers = path.into();
        self.tiers_default = None;
        self
    }

    /// [`InputPaths::with_tiers`] for a user-given name: when `given` is a bare file name of a
    /// tiers built-in (`tiers-default.json`, `tiers-fallback.json`) and that file is absent, the embedded copy is read, so
    /// `apply --tiers tiers-fallback.json` works with no file on disk. Any other missing
    /// explicit path stays a `Cannot read` error.
    pub fn with_tiers_or_builtin(self, path: impl Into<PathBuf>, given: &str) -> Self {
        let mut me = self.with_tiers(path);
        me.tiers_default =
            builtin_for_bare_name(given, &["tiers-default.json", "tiers-fallback.json"])
                .map(DefaultSource::Embedded);
        me
    }

    /// [`InputPaths::with_styles`] with the same built-in rule as
    /// [`InputPaths::with_tiers_or_builtin`], for `styles-default.json` only.
    pub fn with_styles_or_builtin(self, path: impl Into<PathBuf>, given: &str) -> Self {
        let mut me = self.with_styles(path);
        me.styles_default =
            builtin_for_bare_name(given, &["styles-default.json"]).map(DefaultSource::Embedded);
        me
    }

    /// What to read styles from: the live `styles` path if it exists, else the default.
    /// Reads only; writes always target [`InputPaths::styles`].
    pub fn styles_source(&self) -> Result<Source, LoadError> {
        read_source(&self.styles, self.styles_default.as_ref())
    }

    /// What to read tier cutoffs from: the live `tiers` path if it exists, else the default.
    /// Reads only; writes always target [`InputPaths::tiers`].
    pub fn tiers_source(&self) -> Result<Source, LoadError> {
        read_source(&self.tiers, self.tiers_default.as_ref())
    }
}

/// The built-in named by `given`, only when it is a bare file name (no separator, not absolute)
/// and one of `allowed`, so `--tiers` never reads the prices or config text as cutoffs.
fn builtin_for_bare_name(given: &str, allowed: &[&str]) -> Option<&'static Builtin> {
    if given.contains('/') || given.contains('\\') || Path::new(given).is_absolute() {
        return None;
    }
    if !allowed.contains(&given) {
        return None;
    }
    defaults::by_file_name(given)
}

/// Decide what a read uses (DEF-03): `live` if it exists; else the `default` (a file that
/// exists, or an embedded copy); else, for a missing default file, [`LoadError::NoSource`] naming
/// both. With no default (an explicit `--styles` / `--tiers`) `live` is returned as is, so the
/// caller's read reports the usual `Cannot read` error.
///
/// Only checks existence: never creates, writes or touches either file. An existence check that
/// fails (permission denied, a sharing violation) is a [`LoadError::Read`] on that file, never
/// "absent": an unreadable live file must not silently fall back to the defaults.
pub fn read_source(live: &Path, default: Option<&DefaultSource>) -> Result<Source, LoadError> {
    let exists = |p: &Path| {
        p.try_exists().map_err(|e| LoadError::Read {
            path: path_str(p),
            source: e,
        })
    };
    if exists(live)? {
        return Ok(Source::File(live.to_path_buf()));
    }
    match default {
        Some(DefaultSource::File(d)) if exists(d)? => Ok(Source::File(d.clone())),
        Some(DefaultSource::File(d)) => Err(LoadError::NoSource {
            path: path_str(live),
            default: path_str(d),
        }),
        Some(DefaultSource::Embedded(b)) => Ok(Source::Embedded(b)),
        None => Ok(Source::File(live.to_path_buf())),
    }
}

/// What the caller wants the loader to do about price sources.
///
/// Deliberately not a bare `bool`: at the call site `PriceSourcePolicy::ForceFallback` says what
/// it means, where `true` would not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceSourcePolicy {
    /// Prefer live prices; fall back to the committed pair when they are absent or empty.
    Auto,
    /// Always use the committed fallback pair. This is what `--early-game` maps to.
    ForceFallback,
}

/// Which price source [`load_inputs`] actually used.
///
/// Returned rather than printed, so a GUI can render it as a status line while the CLI turns it
/// into the same stderr warnings it has always emitted (D-09).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PriceSource {
    /// Live `prices.json`, present and non-empty.
    Live,
    /// The committed `prices-fallback.json` + `tiers-fallback.json` pair.
    Fallback(FallbackReason),
    /// No price data at all — economy sections are omitted from the output (GEN-05).
    Absent,
}

/// Why the fallback pair was used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    /// The caller asked for it explicitly ([`PriceSourcePolicy::ForceFallback`]).
    Forced,
    /// Live prices were missing, or present but contained no items.
    LiveMissingOrEmpty,
}

/// Everything loaded and priced, ready to compile. Stage one's output.
#[derive(Debug)]
pub struct CompileInputs {
    pub config: Config,
    pub priced: PricedData,
    pub price_source: PriceSource,
}

/// Everything that can go wrong compiling a filter.
///
/// Each variant a caller might want to distinguish is its own arm, and every wrapped error keeps
/// its original miette rendering via `#[diagnostic(transparent)]` (D-06/D-07).
#[derive(Debug, Error, Diagnostic)]
pub enum CompileError {
    /// An input file could not be read, parsed, or validated.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Load(#[from] LoadError),

    /// The reference orbs could not yield cross-rates (TIER-04).
    #[error(transparent)]
    #[diagnostic(transparent)]
    Rates(#[from] PriceRateError),

    /// A rule referenced a style name that does not exist.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Generate(#[from] GenerateError),

    /// [`PriceSourcePolicy::ForceFallback`] was requested but the fallback prices are missing.
    ///
    /// Deliberately carries **no** `#[diagnostic(code(...))]`: the CLI previously raised this
    /// with the bare `miette::miette!` macro, which renders no code line. Adding one changes the
    /// rendered stderr, which the byte-identical-CLI bar forbids.
    #[error("--early-game requires {path}, which is missing")]
    ForcedFallbackMissing { path: String },
}

/// Stage one: resolve and load everything the compiler needs.
///
/// Performs all I/O and surfaces every expected failure. Reports which price source it used in
/// [`CompileInputs::price_source`] rather than printing anything.
pub fn load_inputs(
    paths: &InputPaths,
    policy: PriceSourcePolicy,
) -> Result<CompileInputs, CompileError> {
    let styles_src = match paths.styles_source() {
        Ok(p) => p,
        // Keep the historical order: a missing config.json is reported before the styles.
        // `load_config` reads the config first and fails on it, so this styles path is never read.
        Err(_) if !paths.config.exists() => Source::File(PathBuf::new()),
        Err(e) => return Err(e.into()),
    };
    let styles = styles_src.label();
    let config: Config = match &styles_src {
        Source::File(_) => loader::load_config_with_live(
            &path_str(&paths.config),
            &styles,
            &path_str(&paths.styles),
        )?,
        Source::Embedded(_) => {
            let config: Config = loader::load_json(&path_str(&paths.config))?;
            let set = load_styles_from(&styles_src)?;
            loader::attach_styles(
                config,
                set,
                &path_str(&paths.config),
                &styles,
                &path_str(&paths.styles),
            )?
        }
    };
    config
        .styles
        .check_rules(config.rules.as_deref().unwrap_or(&[]), &styles)
        .map_err(LoadError::from)?;

    let (priced, price_source) = load_priced(paths, policy)?;

    Ok(CompileInputs {
        config,
        priced,
        price_source,
    })
}

/// Everything the economy section needs, and nothing it does not: no `Config`, no rules.
///
/// Stage one's counterpart for the filter-as-store Apply, which regenerates only the fenced
/// economy section from `styles.json` plus priced data.
#[derive(Debug)]
pub struct EconomyInputs {
    pub styles: StyleSet,
    pub priced: PricedData,
    pub price_source: PriceSource,
}

/// Load only what the economy section needs: `styles.json` and the price/tier pair.
///
/// Never reads `config.json`. Returns [`PriceSource::Absent`] as a value (with empty priced
/// data); callers that would write a section must refuse it rather than replace real blocks
/// with nothing (D-17).
pub fn load_economy(
    paths: &InputPaths,
    policy: PriceSourcePolicy,
) -> Result<EconomyInputs, CompileError> {
    let styles = load_styles_from(&paths.styles_source()?)?;
    let (priced, price_source) = load_priced(paths, policy)?;
    Ok(EconomyInputs {
        styles,
        priced,
        price_source,
    })
}

/// [`load_economy`] with in-memory inputs: the caller's `styles` replace `styles.json` and
/// `live_tiers` replace `tiers.json` (the overlay's pending, not yet applied CONFIG edits). The
/// early-league fallback pair is untouched: its tiers are calibrated to its own prices.
pub fn load_economy_with(
    paths: &InputPaths,
    policy: PriceSourcePolicy,
    styles: StyleSet,
    live_tiers: &[TierEntry],
) -> Result<EconomyInputs, CompileError> {
    let (priced, price_source) = load_priced_with(paths, policy, Some(live_tiers))?;
    Ok(EconomyInputs {
        styles,
        priced,
        price_source,
    })
}

/// Pure counterpart of [`load_economy`]: render the economy body text.
pub fn compile_economy(inputs: &EconomyInputs) -> Result<String, CompileError> {
    Ok(emitter::generate_economy(&inputs.styles, &inputs.priced)?)
}

/// Load styles from a file (path loader, messages unchanged) or an embedded copy.
fn load_styles_from(src: &Source) -> Result<StyleSet, LoadError> {
    match src {
        Source::File(p) => loader::load_styles(&path_str(p)),
        Source::Embedded(b) => loader::load_styles_str(b.text, &b.label()),
    }
}

/// Load tier cutoffs from a file or an embedded copy.
fn load_tiers_from(src: &Source) -> Result<ValidatedTiers, CompileError> {
    Ok(match src {
        Source::File(p) => loader::load_tiers(&path_str(p))?,
        Source::Embedded(b) => loader::load_tiers_str(b.text, &b.label())?,
    })
}

/// Load prices from a default: a file may be absent (`None`); an embedded copy never is.
fn load_prices_from(src: &DefaultSource) -> Result<Option<PricesFile>, LoadError> {
    match src {
        DefaultSource::File(p) => loader::load_prices_optional(&path_str(p)),
        DefaultSource::Embedded(b) => loader::load_prices_str(b.text, &b.label()).map(Some),
    }
}

/// Pick the (prices, tiers) source for `policy` and price the tiers. Shared by [`load_inputs`]
/// and [`load_economy`] so the source-selection arms exist exactly once.
fn load_priced(
    paths: &InputPaths,
    policy: PriceSourcePolicy,
) -> Result<(PricedData, PriceSource), CompileError> {
    load_priced_with(paths, policy, None)
}

/// [`load_priced`] where `Some(live_tiers)` replaces the read of the live tiers source.
fn load_priced_with(
    paths: &InputPaths,
    policy: PriceSourcePolicy,
    live_tiers: Option<&[TierEntry]>,
) -> Result<(PricedData, PriceSource), CompileError> {
    let live = |paths: &InputPaths| -> Result<ValidatedTiers, CompileError> {
        match live_tiers {
            Some(t) => Ok(ValidatedTiers::try_from(t.to_vec()).map_err(LoadError::from)?),
            None => load_tiers_from(&paths.tiers_source()?),
        }
    };
    // Pick the (prices, tiers) source pair. The early-league fallback always pairs
    // prices-fallback.json with tiers-fallback.json — their values are calibrated to each other
    // (exalt-denominated cutoffs), so they must travel together.
    let (prices_opt, tiers, price_source) = match policy {
        PriceSourcePolicy::ForceFallback => {
            let prices = load_prices_from(&paths.prices_fallback)?.ok_or_else(|| {
                CompileError::ForcedFallbackMissing {
                    path: Source::from(&paths.prices_fallback).label(),
                }
            })?;
            let tiers = load_tiers_from(&Source::from(&paths.tiers_fallback))?;
            (
                Some(prices),
                tiers,
                PriceSource::Fallback(FallbackReason::Forced),
            )
        }
        PriceSourcePolicy::Auto => {
            match loader::load_prices_optional(&path_str(&paths.prices))? {
                // Live prices present and non-empty → normal path.
                Some(prices) if !prices.items.is_empty() => {
                    let tiers = live(paths)?;
                    (Some(prices), tiers, PriceSource::Live)
                }
                // Absent or empty → fall back to the committed early-league pair.
                _ => match load_prices_from(&paths.prices_fallback)? {
                    Some(prices) => {
                        let tiers = load_tiers_from(&Source::from(&paths.tiers_fallback))?;
                        (
                            Some(prices),
                            tiers,
                            PriceSource::Fallback(FallbackReason::LiveMissingOrEmpty),
                        )
                    }
                    // No prices anywhere → currency/category blocks are omitted (GEN-05).
                    None => {
                        let tiers = live(paths)?;
                        (None, tiers, PriceSource::Absent)
                    }
                },
            }
        }
    };

    let priced = match prices_opt {
        None => PricedData::empty(),
        Some(prices) => {
            // TIER-04: abort on bad reference rates (names the offending currency).
            let rates = pricer::derive_rates(&prices.items)?;
            pricer::price_tiers(&prices, &tiers, &rates)?
        }
    };

    Ok((priced, price_source))
}

/// Stage two: turn loaded inputs into filter text. Pure — no I/O, no side effects.
///
/// Returns LF-only text ending in exactly one newline. Writing it is the caller's job (D-05).
///
/// Uses the default [`FilterIdentity`] (`universal.filter` from `config.json`); use
/// [`compile_as`] to name a different pair in the header.
pub fn compile(inputs: &CompileInputs) -> Result<String, CompileError> {
    compile_as(inputs, &FilterIdentity::default())
}

/// Stage two, naming the filter and its source in the generated header.
///
/// The dynamic path passes `output_name: "dynamic.filter"`, `source_name: "dynamic.json"`.
pub fn compile_as(
    inputs: &CompileInputs,
    identity: &FilterIdentity,
) -> Result<String, CompileError> {
    Ok(emitter::generate_as(
        &inputs.config,
        &inputs.priced,
        identity,
    )?)
}

/// `Path` → `&str` for the loader's string-based API.
///
/// `to_string_lossy` rather than `to_str().unwrap()`: a non-UTF-8 path should surface as a
/// "cannot read" error naming the mangled path, not a panic inside the library.
fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Absolute path to a committed fixture. Tests must never depend on the repo-root
    /// `prices.json`, which is git-ignored and reflects whatever the last `sync` fetched.
    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    fn repo_file(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
    }

    /// Build a self-contained input directory so no test reads the repo root.
    fn temp_inputs(with: &[(&str, &Path)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for (name, src) in with {
            fs::copy(src, dir.path().join(name))
                .unwrap_or_else(|e| panic!("copy {} -> temp: {e}", src.display()));
        }
        dir
    }

    #[test]
    fn relative_to_joins_the_six_conventional_names() {
        let p = InputPaths::relative_to("/tmp/root");
        assert_eq!(p.config, Path::new("/tmp/root").join("config.json"));
        assert_eq!(p.styles, Path::new("/tmp/root").join("styles.json"));
        assert_eq!(p.prices, Path::new("/tmp/root").join("prices.json"));
        assert_eq!(p.tiers, Path::new("/tmp/root").join("tiers.json"));
        assert_eq!(
            p.prices_fallback.as_file().unwrap(),
            Path::new("/tmp/root").join("prices-fallback.json")
        );
        assert_eq!(
            p.tiers_fallback.as_file().unwrap(),
            Path::new("/tmp/root").join("tiers-fallback.json")
        );
    }

    /// The CLI's paths must render with no directory prefix, or every error message that names
    /// a file changes (`./config.json` vs `config.json`). Guards the byte-identical-CLI bar.
    #[test]
    fn conventional_yields_bare_relative_names() {
        let p = InputPaths::conventional();
        assert_eq!(p.config, Path::new("config.json"));
        assert_eq!(p.styles, Path::new("styles.json"));
        assert_eq!(p.prices, Path::new("prices.json"));
        assert_eq!(p.tiers, Path::new("tiers.json"));
        assert_eq!(
            p.prices_fallback.as_file().unwrap(),
            Path::new("prices-fallback.json")
        );
        assert_eq!(
            p.tiers_fallback.as_file().unwrap(),
            Path::new("tiers-fallback.json")
        );
        assert_eq!(
            p.prices_fallback.as_file().unwrap().to_string_lossy(),
            "prices-fallback.json"
        );
    }

    #[test]
    fn with_tiers_overrides_only_the_tiers_path() {
        let base = InputPaths::relative_to("/tmp/root");
        let overridden = base.clone().with_tiers("tiers-fallback.json");
        assert_eq!(overridden.tiers, Path::new("tiers-fallback.json"));
        // Prices in particular must be untouched — the whole point of this override is to keep
        // live prices while changing only the cutoffs. If it moved prices too it would be
        // `--early-game` under a different name.
        assert_eq!(overridden.prices, base.prices);
        assert_eq!(overridden.config, base.config);
        assert_eq!(overridden.prices_fallback, base.prices_fallback);
        assert_eq!(overridden.tiers_fallback, base.tiers_fallback);
    }

    #[test]
    fn with_config_and_with_tiers_compose() {
        let paths = InputPaths::conventional()
            .with_config("dynamic.json")
            .with_tiers("tiers-fallback.json");
        assert_eq!(paths.config, Path::new("dynamic.json"));
        assert_eq!(paths.tiers, Path::new("tiers-fallback.json"));
        assert_eq!(paths.prices, Path::new("prices.json"));
    }

    #[test]
    fn with_styles_overrides_only_the_styles_path() {
        let base = InputPaths::relative_to("/tmp/root");
        let overridden = base.clone().with_styles("/elsewhere/my-styles.json");
        assert_eq!(overridden.styles, Path::new("/elsewhere/my-styles.json"));
        assert_eq!(overridden.config, base.config);
        assert_eq!(overridden.prices, base.prices);
        assert_eq!(overridden.tiers, base.tiers);
        assert_eq!(overridden.prices_fallback, base.prices_fallback);
        assert_eq!(overridden.tiers_fallback, base.tiers_fallback);
    }

    #[test]
    fn with_config_overrides_only_the_config_path() {
        let base = InputPaths::relative_to("/tmp/root");
        let overridden = base.clone().with_config("/elsewhere/ssf.json");
        assert_eq!(overridden.config, Path::new("/elsewhere/ssf.json"));
        assert_eq!(overridden.prices, base.prices);
        assert_eq!(overridden.tiers, base.tiers);
        assert_eq!(overridden.prices_fallback, base.prices_fallback);
        assert_eq!(overridden.tiers_fallback, base.tiers_fallback);
    }

    #[test]
    fn relative_to_sets_the_default_sources() {
        let p = InputPaths::relative_to("/tmp/root");
        let file =
            |d: &Option<DefaultSource>| d.as_ref().and_then(|d| d.as_file()).map(PathBuf::from);
        assert_eq!(
            file(&p.styles_default),
            Some(Path::new("/tmp/root").join("styles-default.json"))
        );
        assert_eq!(
            file(&p.tiers_default),
            Some(Path::new("/tmp/root").join("tiers-default.json"))
        );
        let c = InputPaths::conventional();
        assert_eq!(
            file(&c.styles_default),
            Some(PathBuf::from("styles-default.json"))
        );
        assert_eq!(
            file(&c.tiers_default),
            Some(PathBuf::from("tiers-default.json"))
        );
    }

    /// An explicit `--styles` / `--tiers` keeps its meaning: no default to fall back to.
    #[test]
    fn with_styles_and_with_tiers_clear_only_their_own_default() {
        let base = InputPaths::relative_to("/tmp/root");
        let s = base.clone().with_styles("my-styles.json");
        assert_eq!(s.styles_default, None);
        assert_eq!(s.tiers_default, base.tiers_default);
        let t = base.clone().with_tiers("my-tiers.json");
        assert_eq!(t.tiers_default, None);
        assert_eq!(t.styles_default, base.styles_default);
    }

    #[test]
    fn read_source_prefers_the_live_file() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("styles.json");
        let default = dir.path().join("styles-default.json");
        fs::write(&live, "{}").unwrap();
        fs::write(&default, "{}").unwrap();
        let d = DefaultSource::File(default);
        assert_eq!(read_source(&live, Some(&d)).unwrap(), Source::File(live));
    }

    #[test]
    fn read_source_falls_back_to_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("styles.json");
        let default = dir.path().join("styles-default.json");
        fs::write(&default, "{}").unwrap();
        let d = DefaultSource::File(default.clone());
        assert_eq!(read_source(&live, Some(&d)).unwrap(), Source::File(default));
        assert!(
            !live.exists(),
            "read_source must never create the live file"
        );
    }

    #[test]
    fn read_source_with_neither_file_names_both() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("styles.json");
        let default = dir.path().join("styles-default.json");
        let err = read_source(&live, Some(&DefaultSource::File(default))).unwrap_err();
        assert!(matches!(err, LoadError::NoSource { .. }), "got {err:?}");
        let msg = err.to_string();
        assert!(msg.starts_with("Cannot read `"), "{msg}");
        assert!(
            msg.contains("styles.json") && msg.contains("styles-default.json"),
            "{msg}"
        );
    }

    /// A live file whose existence cannot be checked is an error naming it, not "absent": the
    /// defaults must not silently replace a live file that is there but unreadable (WR-04). A NUL
    /// byte in the name makes the check itself fail on every platform.
    #[test]
    fn read_source_failed_existence_check_is_an_error_not_a_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let default = dir.path().join("styles-default.json");
        fs::write(&default, "{}").unwrap();
        let live = dir.path().join("sty\0les.json");
        let err = read_source(&live, Some(&DefaultSource::File(default))).unwrap_err();
        assert!(matches!(err, LoadError::Read { .. }), "got {err:?}");
        assert!(err.to_string().starts_with("Cannot read `"), "{err}");
    }

    /// An explicit path never falls back, even with a default file beside it: the caller's read
    /// then fails with the unchanged `Cannot read` error.
    #[test]
    fn read_source_explicit_path_never_falls_back() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("styles-default.json"), "{}").unwrap();
        let live = dir.path().join("styles.json");
        assert_eq!(
            read_source(&live, None).unwrap(),
            Source::File(live.clone())
        );
        let paths = InputPaths::relative_to(dir.path()).with_styles(&live);
        assert_eq!(paths.styles_source().unwrap(), Source::File(live));
    }

    #[test]
    fn in_home_reads_the_embedded_defaults_from_an_empty_folder() {
        let dir = tempfile::tempdir().unwrap();
        let p = InputPaths::in_home(dir.path());
        assert_eq!(
            p.styles_source().unwrap(),
            Source::Embedded(&defaults::STYLES_DEFAULT)
        );
        assert_eq!(
            p.tiers_source().unwrap(),
            Source::Embedded(&defaults::TIERS_DEFAULT)
        );
        assert_eq!(p.styles, dir.path().join("styles.json"));
        assert_eq!(p.tiers, dir.path().join("tiers.json"));
        assert_eq!(p.prices, dir.path().join("prices.json"));
        assert_eq!(
            p.prices_fallback,
            DefaultSource::Embedded(&defaults::PRICES_FALLBACK)
        );
        assert_eq!(
            p.tiers_fallback,
            DefaultSource::Embedded(&defaults::TIERS_FALLBACK)
        );
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);

        fs::write(dir.path().join("styles.json"), "{}").unwrap();
        fs::write(dir.path().join("tiers.json"), "[]").unwrap();
        assert_eq!(
            p.styles_source().unwrap(),
            Source::File(dir.path().join("styles.json"))
        );
        assert_eq!(
            p.tiers_source().unwrap(),
            Source::File(dir.path().join("tiers.json"))
        );
    }

    #[test]
    fn economy_from_an_empty_home_uses_the_built_ins_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let inputs = load_economy(&InputPaths::in_home(dir.path()), PriceSourcePolicy::Auto)
            .expect("embedded defaults");
        assert_eq!(
            inputs.price_source,
            PriceSource::Fallback(FallbackReason::LiveMissingOrEmpty)
        );
        assert!(!inputs.priced.category_blocks.is_empty());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);

        let forced = load_economy(
            &InputPaths::in_home(dir.path()),
            PriceSourcePolicy::ForceFallback,
        )
        .expect("embedded pair is never missing");
        assert_eq!(
            forced.price_source,
            PriceSource::Fallback(FallbackReason::Forced)
        );
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn economy_from_built_ins_equals_economy_from_the_committed_files() {
        let home = tempfile::tempdir().unwrap();
        let from_embedded =
            load_economy(&InputPaths::in_home(home.path()), PriceSourcePolicy::Auto).unwrap();
        let files = temp_inputs(&[
            (
                "styles-default.json",
                &repo_file("tests/fixtures/seed-styles.json"),
            ),
            ("tiers-default.json", &repo_file("tiers-default.json")),
            ("prices-fallback.json", &repo_file("prices-fallback.json")),
            ("tiers-fallback.json", &repo_file("tiers-fallback.json")),
        ]);
        let from_files = load_economy(
            &InputPaths::relative_to(files.path()),
            PriceSourcePolicy::Auto,
        )
        .unwrap();
        assert_eq!(
            compile_economy(&from_embedded).unwrap(),
            compile_economy(&from_files).unwrap()
        );
    }

    #[test]
    fn a_broken_live_file_in_home_names_its_full_path() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("styles.json");
        fs::write(&live, "{").unwrap();
        let err = load_economy(&InputPaths::in_home(dir.path()), PriceSourcePolicy::Auto)
            .unwrap_err()
            .to_string();
        assert!(err.contains(&live.display().to_string()), "{err}");
    }

    #[test]
    fn a_broken_built_in_names_itself_as_built_in() {
        static BAD: Builtin = Builtin {
            file_name: "styles-default.json",
            text: "{",
        };
        let err = load_styles_from(&Source::Embedded(&BAD)).unwrap_err();
        assert!(
            err.to_string().contains("styles-default.json (built in)"),
            "{err}"
        );
    }

    #[test]
    fn from_dirs_equals_in_home() {
        let dir = tempfile::tempdir().unwrap();
        let dirs = crate::locations::AppDirs::at(dir.path());
        assert_eq!(
            InputPaths::from_dirs(&dirs),
            InputPaths::in_home(dir.path())
        );
    }

    #[test]
    fn read_source_embedded_default_never_replaces_an_unreadable_live_file() {
        let live = Path::new("sty\0les.json");
        let d = DefaultSource::Embedded(&defaults::STYLES_DEFAULT);
        let err = read_source(live, Some(&d)).unwrap_err();
        assert!(matches!(err, LoadError::Read { .. }), "got {err:?}");
    }

    #[test]
    fn source_labels_name_the_full_path_or_the_built_in() {
        let p = PathBuf::from("/a/b/styles.json");
        assert_eq!(Source::File(p.clone()).label(), p.display().to_string());
        assert_eq!(
            Source::Embedded(&defaults::STYLES_DEFAULT).label(),
            "styles-default.json (built in)"
        );
    }

    #[test]
    fn with_tiers_or_builtin_falls_back_only_for_a_bare_built_in_name() {
        let dir = tempfile::tempdir().unwrap();
        let base = InputPaths::in_home(dir.path());
        let bare = base.clone().with_tiers_or_builtin(
            dir.path().join("tiers-fallback.json"),
            "tiers-fallback.json",
        );
        assert_eq!(
            bare.tiers_source().unwrap(),
            Source::Embedded(&defaults::TIERS_FALLBACK)
        );
        let default = base
            .clone()
            .with_tiers_or_builtin(dir.path().join("tiers-default.json"), "tiers-default.json");
        assert_eq!(
            default.tiers_source().unwrap(),
            Source::Embedded(&defaults::TIERS_DEFAULT)
        );
        for given in [
            "sub/tiers-fallback.json",
            "alt-tiers.json",
            "config.json",
            "prices-fallback.json",
            "styles-default.json",
        ] {
            let p = base
                .clone()
                .with_tiers_or_builtin(dir.path().join(given), given);
            assert_eq!(p.tiers_default, None, "{given}");
        }
        let abs = dir.path().join("tiers-fallback.json");
        let p = base
            .clone()
            .with_tiers_or_builtin(&abs, &abs.display().to_string());
        assert_eq!(p.tiers_default, None);
        // A present file of that name wins over the built-in.
        fs::write(&abs, "[]").unwrap();
        let p = base.with_tiers_or_builtin(&abs, "tiers-fallback.json");
        assert_eq!(p.tiers_source().unwrap(), Source::File(abs));
    }

    #[test]
    fn with_styles_or_builtin_falls_back_for_styles_default() {
        let dir = tempfile::tempdir().unwrap();
        let p = InputPaths::in_home(dir.path()).with_styles_or_builtin(
            dir.path().join("styles-default.json"),
            "styles-default.json",
        );
        assert_eq!(
            p.styles_source().unwrap(),
            Source::Embedded(&defaults::STYLES_DEFAULT)
        );
        let p = InputPaths::in_home(dir.path())
            .with_styles_or_builtin(dir.path().join("mine.json"), "mine.json");
        assert_eq!(p.styles_default, None);
        for given in ["mapping.json", "tiers-default.json", "config.json"] {
            let p = InputPaths::in_home(dir.path())
                .with_styles_or_builtin(dir.path().join(given), given);
            assert_eq!(p.styles_default, None, "{given}");
        }
    }

    /// A root holding only the committed defaults (no live styles.json / tiers.json).
    fn defaults_only_root(with_config: bool) -> tempfile::TempDir {
        let mut files: Vec<(&str, PathBuf)> = vec![
            (
                "styles-default.json",
                repo_file("tests/fixtures/seed-styles.json"),
            ),
            ("tiers-default.json", repo_file("tiers-default.json")),
            ("prices.json", fixture("seed-prices.json")),
        ];
        if with_config {
            files.push(("config.json", repo_file("config.json")));
        }
        let refs: Vec<(&str, &Path)> = files.iter().map(|(n, p)| (*n, p.as_path())).collect();
        temp_inputs(&refs)
    }

    fn assert_no_live_files(dir: &Path) {
        assert!(!dir.join("styles.json").exists(), "styles.json was created");
        assert!(!dir.join("tiers.json").exists(), "tiers.json was created");
    }

    #[test]
    fn load_inputs_falls_back_to_both_defaults_and_creates_no_live_file() {
        let dir = defaults_only_root(true);
        let from_defaults = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load from defaults");
        assert_no_live_files(dir.path());

        let live = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("tiers.json", &repo_file("tiers-default.json")),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        let from_live = load_inputs(
            &InputPaths::relative_to(live.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load from live names");
        assert_eq!(
            compile(&from_defaults).unwrap(),
            compile(&from_live).unwrap()
        );
    }

    #[test]
    fn load_inputs_falls_back_to_default_styles_only() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            (
                "styles-default.json",
                &repo_file("tests/fixtures/seed-styles.json"),
            ),
            ("tiers.json", &repo_file("tiers-default.json")),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("styles from the default");
        assert!(!dir.path().join("styles.json").exists());
    }

    /// WR-03: a differing inline `styles` section read against the committed default tells the
    /// user to move their styles into the live `styles.json`, never into `styles-default.json`.
    #[test]
    fn inline_styles_advice_names_the_live_file_when_reading_the_default() {
        let dir = defaults_only_root(true);
        let cfg_p = dir.path().join("config.json");
        let mut v: serde_json::Value = serde_json::from_slice(&fs::read(&cfg_p).unwrap()).unwrap();
        v["styles"] = serde_json::json!({ "S": { "font": 44, "action": "Show" } });
        fs::write(&cfg_p, v.to_string()).unwrap();

        let err = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .unwrap_err();
        let msg = err.to_string();
        let live = path_str(&dir.path().join("styles.json"));
        let default = path_str(&dir.path().join("styles-default.json"));
        assert!(
            msg.contains(&format!(
                "Move any styles you changed or added into {live} "
            )),
            "{msg}"
        );
        assert!(
            msg.contains(&format!("create it from a copy of {default}")),
            "{msg}"
        );
        assert!(
            !msg.contains(&format!("into {default}")),
            "advice must not name the default: {msg}"
        );
        assert_no_live_files(dir.path());

        // With a live file the advice is unchanged: it names the file that was read.
        fs::copy(
            dir.path().join("styles-default.json"),
            dir.path().join("styles.json"),
        )
        .unwrap();
        let msg = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .unwrap_err()
        .to_string();
        assert!(
            msg.contains(&format!(
                "Move any styles you changed or added into {live}, then"
            )),
            "{msg}"
        );
    }

    #[test]
    fn load_inputs_falls_back_to_default_tiers_only() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("tiers-default.json", &repo_file("tiers-default.json")),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        let inputs = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("tiers from the default");
        assert_eq!(inputs.price_source, PriceSource::Live);
        assert!(!dir.path().join("tiers.json").exists());
    }

    /// A present live file wins: a broken live styles.json is reported even though a valid
    /// default sits beside it.
    #[test]
    fn load_inputs_prefers_the_live_styles_file() {
        let dir = defaults_only_root(true);
        fs::write(dir.path().join("styles.json"), "{ not json").unwrap();
        let err = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect_err("the broken live file must be read, not the default");
        assert!(
            matches!(err, CompileError::Load(LoadError::Parse(_))),
            "{err:?}"
        );
    }

    #[test]
    fn load_economy_falls_back_to_defaults_and_creates_no_live_file() {
        let dir = defaults_only_root(false);
        let inputs = load_economy(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load_economy from defaults");
        assert_eq!(inputs.price_source, PriceSource::Live);
        assert_no_live_files(dir.path());
    }

    #[test]
    fn load_economy_with_neither_tiers_file_names_both() {
        let dir = temp_inputs(&[
            (
                "styles-default.json",
                &repo_file("tests/fixtures/seed-styles.json"),
            ),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        let err = load_economy(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("tiers.json") && msg.contains("tiers-default.json"),
            "{msg}"
        );
    }

    #[test]
    fn load_economy_needs_no_config_json() {
        let dir = temp_inputs(&[
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("tiers.json", &repo_file("tiers-default.json")),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        let inputs = load_economy(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load_economy");
        assert_eq!(inputs.price_source, PriceSource::Live);
        assert!(!inputs.priced.category_blocks.is_empty());
    }

    #[test]
    fn load_economy_falls_back_to_the_committed_pair() {
        let dir = temp_inputs(&[
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("prices-fallback.json", &repo_file("prices-fallback.json")),
            ("tiers-fallback.json", &repo_file("tiers-fallback.json")),
        ]);
        let inputs = load_economy(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load_economy");
        assert_eq!(
            inputs.price_source,
            PriceSource::Fallback(FallbackReason::LiveMissingOrEmpty)
        );
    }

    #[test]
    fn load_economy_reports_absent_as_a_value() {
        let dir = temp_inputs(&[
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("tiers.json", &repo_file("tiers-default.json")),
        ]);
        let inputs = load_economy(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load_economy");
        assert_eq!(inputs.price_source, PriceSource::Absent);
        assert!(inputs.priced.category_blocks.is_empty());
        assert!(inputs.priced.unique_blocks.is_empty());
    }

    #[test]
    fn load_economy_force_fallback_errors_when_fallback_missing() {
        let dir = temp_inputs(&[("styles.json", &repo_file("tests/fixtures/seed-styles.json"))]);
        let err = load_economy(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::ForceFallback,
        )
        .unwrap_err();
        assert!(
            matches!(err, CompileError::ForcedFallbackMissing { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn load_economy_malformed_styles_is_a_load_error_naming_the_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("styles.json"), "{ not json").unwrap();
        let err = load_economy(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Load(_)), "got {err:?}");
        assert!(err.to_string().contains("styles.json"), "got {err}");
    }

    #[test]
    fn compile_economy_matches_generate_economy() {
        let dir = temp_inputs(&[
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("tiers.json", &repo_file("tiers-default.json")),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        let inputs = load_economy(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load_economy");
        let via_compile = compile_economy(&inputs).expect("compile_economy");
        let direct = emitter::generate_economy(&inputs.styles, &inputs.priced).unwrap();
        assert_eq!(via_compile, direct);
    }

    #[test]
    fn load_inputs_reports_live_when_prices_present() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("tiers.json", &repo_file("tiers-default.json")),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        let inputs = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load");
        assert_eq!(inputs.price_source, PriceSource::Live);
        assert!(!inputs.priced.category_blocks.is_empty());
    }

    #[test]
    fn load_inputs_falls_back_when_live_prices_absent() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("tiers.json", &repo_file("tiers-default.json")),
            ("prices-fallback.json", &repo_file("prices-fallback.json")),
            ("tiers-fallback.json", &repo_file("tiers-fallback.json")),
        ]);
        let inputs = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load");
        assert_eq!(
            inputs.price_source,
            PriceSource::Fallback(FallbackReason::LiveMissingOrEmpty)
        );
    }

    #[test]
    fn load_inputs_reports_absent_when_no_prices_anywhere() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("tiers.json", &repo_file("tiers-default.json")),
        ]);
        let inputs = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load");
        assert_eq!(inputs.price_source, PriceSource::Absent);
        assert!(inputs.priced.category_blocks.is_empty());
        assert!(inputs.priced.unique_blocks.is_empty());
    }

    #[test]
    fn force_fallback_reports_forced() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("prices-fallback.json", &repo_file("prices-fallback.json")),
            ("tiers-fallback.json", &repo_file("tiers-fallback.json")),
            // A live prices.json is present and must be ignored under ForceFallback.
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        let inputs = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::ForceFallback,
        )
        .expect("load");
        assert_eq!(
            inputs.price_source,
            PriceSource::Fallback(FallbackReason::Forced)
        );
    }

    #[test]
    fn force_fallback_errors_when_fallback_prices_missing() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("tiers.json", &repo_file("tiers-default.json")),
        ]);
        let err = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::ForceFallback,
        )
        .expect_err("must fail without prices-fallback.json");
        assert!(
            matches!(err, CompileError::ForcedFallbackMissing { .. }),
            "expected ForcedFallbackMissing, got {err:?}"
        );
        assert!(err.to_string().contains("--early-game requires"));
    }

    /// Temp dir with the committed styles/tiers/prices and a caller-supplied config text.
    fn inputs_with_config(config_text: &str) -> tempfile::TempDir {
        let dir = temp_inputs(&[
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("tiers.json", &repo_file("tiers-default.json")),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        fs::write(dir.path().join("config.json"), config_text).unwrap();
        dir
    }

    fn config_with_rule_style(style: &str, enabled: Option<bool>) -> String {
        let mut v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(repo_file("config.json")).unwrap()).unwrap();
        let rule = &mut v["rules"][1];
        rule["style"] = serde_json::json!(style);
        if let Some(e) = enabled {
            rule["enabled"] = serde_json::json!(e);
        }
        serde_json::to_string(&v).unwrap()
    }

    #[test]
    fn load_inputs_attaches_the_twelve_committed_styles() {
        let dir = inputs_with_config(&fs::read_to_string(repo_file("config.json")).unwrap());
        let inputs = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load");
        assert_eq!(inputs.config.styles.len(), 12);
    }

    #[test]
    fn load_inputs_rejects_an_enabled_rule_with_an_unknown_style() {
        let dir = inputs_with_config(&config_with_rule_style("nope", None));
        let err = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect_err("unknown style must fail at load");
        let msg = err.to_string();
        assert!(msg.contains("nope"), "{msg}");
        assert!(msg.contains("rule 1") || msg.contains("(\""), "{msg}");
        assert!(msg.contains("exceptional"), "{msg}");
        assert!(msg.contains("styles.json"), "{msg}");
    }

    #[test]
    fn load_inputs_ignores_a_disabled_rule_with_an_unknown_style() {
        let dir = inputs_with_config(&config_with_rule_style("nope", Some(false)));
        load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("a disabled rule is inert");
    }

    #[test]
    fn load_inputs_without_styles_json_is_a_read_error_naming_it() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            ("tiers.json", &repo_file("tiers-default.json")),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        let err = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect_err("must fail without styles.json");
        let msg = err.to_string();
        assert!(msg.starts_with("Cannot read `"), "{msg}");
        assert!(msg.contains("styles.json"), "{msg}");
        assert!(msg.contains("styles-default.json"), "{msg}");
    }

    #[test]
    fn load_inputs_rejects_a_config_with_an_inline_styles_section() {
        let mut v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(repo_file("config.json")).unwrap()).unwrap();
        // `E` differs from styles.json (font 30, not 29), so dropping the section would lose it.
        let styles: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(repo_file("tests/fixtures/seed-styles.json")).unwrap(),
        )
        .unwrap();
        let mut e = styles["E"].clone();
        e["font"] = serde_json::json!(30);
        v["styles"] = serde_json::json!({ "S": styles["S"].clone(), "E": e });
        let dir = inputs_with_config(&serde_json::to_string(&v).unwrap());
        let err = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect_err("a differing inline styles section must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("config.json") && msg.contains("styles.json"),
            "{msg}"
        );
        assert!(
            msg.contains(": E."),
            "names only the differing style: {msg}"
        );
        assert!(!msg.contains("--styles"), "{msg}");
    }

    /// WR-03: an inline section repeating styles.json entries (including an empty one) is dropped.
    #[test]
    fn load_inputs_accepts_an_inline_styles_section_matching_styles_json() {
        let mut v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(repo_file("config.json")).unwrap()).unwrap();
        let styles: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(repo_file("tests/fixtures/seed-styles.json")).unwrap(),
        )
        .unwrap();
        for inline in [
            serde_json::json!({}),
            serde_json::json!({ "hidden": styles["hidden"].clone() }),
        ] {
            v["styles"] = inline;
            let dir = inputs_with_config(&serde_json::to_string(&v).unwrap());
            let inputs = load_inputs(
                &InputPaths::relative_to(dir.path()),
                PriceSourcePolicy::Auto,
            )
            .expect("a matching inline section loses nothing and is dropped");
            assert!(inputs.config.legacy_styles.is_none());
        }
    }

    #[test]
    fn missing_config_surfaces_as_a_load_read_error() {
        let dir = temp_inputs(&[("tiers.json", &repo_file("tiers-default.json"))]);
        let err = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect_err("must fail without config.json");
        assert!(
            matches!(err, CompileError::Load(LoadError::Read { .. })),
            "expected Load(Read), got {err:?}"
        );
        assert!(err.to_string().contains("Cannot read"));
    }

    #[test]
    fn compile_produces_filter_text_from_loaded_inputs() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            ("styles.json", &repo_file("tests/fixtures/seed-styles.json")),
            ("tiers.json", &repo_file("tiers-default.json")),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        let inputs = load_inputs(
            &InputPaths::relative_to(dir.path()),
            PriceSourcePolicy::Auto,
        )
        .expect("load");
        let text = compile(&inputs).expect("compile");
        assert!(
            text.starts_with('#'),
            "filter should open with a comment header"
        );
        assert!(text.ends_with('\n'));
        assert!(text.contains("Show # "));
    }
}
