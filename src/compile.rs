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

use crate::emitter::{self, FilterIdentity, GenerateError};
use crate::loader::{self, LoadError};
use crate::models::config::Config;
use crate::pricer::{self, PriceRateError, PricedData};

/// Conventional file name for the policy config.
const CONFIG_FILE: &str = "config.json";
/// Conventional file name for live synced prices.
const PRICES_FILE: &str = "prices.json";
/// Conventional file name for the tier cutoffs that pair with live prices.
const TIERS_FILE: &str = "tiers.json";
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputPaths {
    pub config: PathBuf,
    pub prices: PathBuf,
    pub tiers: PathBuf,
    pub prices_fallback: PathBuf,
    pub tiers_fallback: PathBuf,
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
            prices: root.join(PRICES_FILE),
            tiers: root.join(TIERS_FILE),
            prices_fallback: root.join(PRICES_FALLBACK_FILE),
            tiers_fallback: root.join(TIERS_FALLBACK_FILE),
        }
    }

    /// Override just the config path, leaving the rest untouched — this is what
    /// `generate --config <path>` needs.
    pub fn with_config(mut self, path: impl Into<PathBuf>) -> Self {
        self.config = path.into();
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
    /// restores sane placement without touching the committed `tiers.json` the test oracle
    /// depends on.
    pub fn with_tiers(mut self, path: impl Into<PathBuf>) -> Self {
        self.tiers = path.into();
        self
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
    let config: Config = loader::load_json(&path_str(&paths.config))?;

    // Pick the (prices, tiers) source pair. The early-league fallback always pairs
    // prices-fallback.json with tiers-fallback.json — their values are calibrated to each other
    // (exalt-denominated cutoffs), so they must travel together.
    let (prices_opt, tiers, price_source) = match policy {
        PriceSourcePolicy::ForceFallback => {
            let prices = loader::load_prices_optional(&path_str(&paths.prices_fallback))?
                .ok_or_else(|| CompileError::ForcedFallbackMissing {
                    path: path_str(&paths.prices_fallback),
                })?;
            let tiers = loader::load_tiers(&path_str(&paths.tiers_fallback))?;
            (Some(prices), tiers, PriceSource::Fallback(FallbackReason::Forced))
        }
        PriceSourcePolicy::Auto => {
            match loader::load_prices_optional(&path_str(&paths.prices))? {
                // Live prices present and non-empty → normal path.
                Some(prices) if !prices.items.is_empty() => {
                    let tiers = loader::load_tiers(&path_str(&paths.tiers))?;
                    (Some(prices), tiers, PriceSource::Live)
                }
                // Absent or empty → fall back to the committed early-league pair.
                _ => match loader::load_prices_optional(&path_str(&paths.prices_fallback))? {
                    Some(prices) => {
                        let tiers = loader::load_tiers(&path_str(&paths.tiers_fallback))?;
                        (
                            Some(prices),
                            tiers,
                            PriceSource::Fallback(FallbackReason::LiveMissingOrEmpty),
                        )
                    }
                    // No prices anywhere → currency/category blocks are omitted (GEN-05).
                    None => {
                        let tiers = loader::load_tiers(&path_str(&paths.tiers))?;
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

    Ok(CompileInputs {
        config,
        priced,
        price_source,
    })
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
    Ok(emitter::generate_as(&inputs.config, &inputs.priced, identity)?)
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
    fn relative_to_joins_the_five_conventional_names() {
        let p = InputPaths::relative_to("/tmp/root");
        assert_eq!(p.config, Path::new("/tmp/root").join("config.json"));
        assert_eq!(p.prices, Path::new("/tmp/root").join("prices.json"));
        assert_eq!(p.tiers, Path::new("/tmp/root").join("tiers.json"));
        assert_eq!(
            p.prices_fallback,
            Path::new("/tmp/root").join("prices-fallback.json")
        );
        assert_eq!(
            p.tiers_fallback,
            Path::new("/tmp/root").join("tiers-fallback.json")
        );
    }

    /// The CLI's paths must render with no directory prefix, or every error message that names
    /// a file changes (`./config.json` vs `config.json`). Guards the byte-identical-CLI bar.
    #[test]
    fn conventional_yields_bare_relative_names() {
        let p = InputPaths::conventional();
        assert_eq!(p.config, Path::new("config.json"));
        assert_eq!(p.prices, Path::new("prices.json"));
        assert_eq!(p.tiers, Path::new("tiers.json"));
        assert_eq!(p.prices_fallback, Path::new("prices-fallback.json"));
        assert_eq!(p.tiers_fallback, Path::new("tiers-fallback.json"));
        assert_eq!(p.prices_fallback.to_string_lossy(), "prices-fallback.json");
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
    fn load_inputs_reports_live_when_prices_present() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            ("tiers.json", &repo_file("tiers.json")),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        let inputs = load_inputs(&InputPaths::relative_to(dir.path()), PriceSourcePolicy::Auto)
            .expect("load");
        assert_eq!(inputs.price_source, PriceSource::Live);
        assert!(!inputs.priced.category_blocks.is_empty());
    }

    #[test]
    fn load_inputs_falls_back_when_live_prices_absent() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
            ("tiers.json", &repo_file("tiers.json")),
            ("prices-fallback.json", &repo_file("prices-fallback.json")),
            ("tiers-fallback.json", &repo_file("tiers-fallback.json")),
        ]);
        let inputs = load_inputs(&InputPaths::relative_to(dir.path()), PriceSourcePolicy::Auto)
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
            ("tiers.json", &repo_file("tiers.json")),
        ]);
        let inputs = load_inputs(&InputPaths::relative_to(dir.path()), PriceSourcePolicy::Auto)
            .expect("load");
        assert_eq!(inputs.price_source, PriceSource::Absent);
        assert!(inputs.priced.category_blocks.is_empty());
        assert!(inputs.priced.unique_blocks.is_empty());
    }

    #[test]
    fn force_fallback_reports_forced() {
        let dir = temp_inputs(&[
            ("config.json", &repo_file("config.json")),
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
            ("tiers.json", &repo_file("tiers.json")),
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

    #[test]
    fn missing_config_surfaces_as_a_load_read_error() {
        let dir = temp_inputs(&[("tiers.json", &repo_file("tiers.json"))]);
        let err = load_inputs(&InputPaths::relative_to(dir.path()), PriceSourcePolicy::Auto)
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
            ("tiers.json", &repo_file("tiers.json")),
            ("prices.json", &fixture("seed-prices.json")),
        ]);
        let inputs = load_inputs(&InputPaths::relative_to(dir.path()), PriceSourcePolicy::Auto)
            .expect("load");
        let text = compile(&inputs).expect("compile");
        assert!(text.starts_with('#'), "filter should open with a comment header");
        assert!(text.ends_with('\n'));
        assert!(text.contains("Show # "));
    }
}
