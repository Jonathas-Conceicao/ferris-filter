//! Shared CLI entry point for both binary targets (`ferrisfilter` and `ff`).
//!
//! The dispatch logic lives here in the library so the two `fn main()` shims
//! (`src/main.rs` and `src/bin/ff.rs`) each have their own source file. Previously both
//! `[[bin]]` targets pointed at `src/main.rs`, which made Cargo warn that one file was
//! present in multiple build targets.
//!
//! This module owns *presentation* only. The compile pipeline itself lives in
//! [`crate::compile`], which does no printing and takes all its paths from the caller — so the
//! overlay front end can drive the same pipeline without inheriting the CLI's stdout or its
//! working-directory assumptions.

use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::compile::{self, FallbackReason, InputPaths, PriceSource, PriceSourcePolicy};
use crate::dynamic;
use crate::safe_write::WriteOutcome;
use crate::{install, sync};
use std::path::{Path, PathBuf};

/// `p` as given when absolute, else joined onto `root`. `apply` reads its inputs from the
/// FerrisFilter folder, so a relative `--tiers`/`--styles` override is relative to it rather than
/// the process working directory.
fn under_root(root: &Path, p: &str) -> PathBuf {
    let p = Path::new(p);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}

/// Parse CLI args and dispatch to the matching subcommand.
pub fn run() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate {
            config,
            output,
            early_game,
            tiers,
            styles,
        } => cmd_generate(
            &config,
            &output,
            early_game,
            tiers.as_deref(),
            styles.as_deref(),
        )?,
        Commands::Apply {
            filter,
            add_economy,
            remove_economy,
            tiers,
            styles,
        } => cmd_apply(
            filter.as_deref(),
            add_economy,
            remove_economy,
            tiers.as_deref(),
            styles.as_deref(),
        )?,
        Commands::Sync {
            standard,
            hc,
            league,
            sub,
        } => sync::cmd_sync(standard, hc, league, sub)?,
        Commands::Install { input, dest } => {
            // The saved folder is read only when --dest is omitted.
            let saved = match dest {
                Some(_) => None,
                None => saved_choice()?.game_folder,
            };
            install::cmd_install(&input, dest, saved.as_deref())?
        }
    }
    Ok(())
}

fn cmd_generate(
    config_path: &str,
    output: &str,
    early_game: bool,
    tiers: Option<&str>,
    styles: Option<&str>,
) -> miette::Result<()> {
    // Bare relative names, resolved against the process working directory — exactly the paths
    // this function passed before the compile API existed. `conventional()` rather than
    // `relative_to(".")` because the latter would render as `./config.json` and change every
    // error message that names a file.
    let mut paths = InputPaths::conventional().with_config(config_path);
    if let Some(t) = tiers {
        eprintln!("note: --tiers {t} — using its cutoffs instead of tiers.json");
        paths = paths.with_tiers(t);
    }
    if let Some(s) = styles {
        eprintln!("note: --styles {s} — using it instead of styles.json");
        paths = paths.with_styles(s);
    }

    let policy = if early_game {
        // Emitted BEFORE loading, gated on the flag rather than derived from the returned
        // PriceSource — because this note must still appear when prices-fallback.json turns out
        // to be missing and the load fails. Deriving it after the fact would silently drop it on
        // that path.
        eprintln!("note: --early-game — using prices-fallback.json + tiers-fallback.json");
        PriceSourcePolicy::ForceFallback
    } else {
        PriceSourcePolicy::Auto
    };

    let inputs = compile::load_inputs(&paths, policy).map_err(miette::Error::from)?;

    // Render the price-source decision the loader reported. The library returns this as a value
    // so a GUI can present it its own way; here it becomes the warnings this CLI has always
    // printed.
    match inputs.price_source {
        PriceSource::Fallback(FallbackReason::LiveMissingOrEmpty) => {
            eprintln!(
                "warning: prices.json absent or empty — falling back to \
                 prices-fallback.json + tiers-fallback.json"
            );
        }
        PriceSource::Absent => {
            eprintln!(
                "warning: prices.json absent or empty — falling back to \
                 prices-fallback.json + tiers-fallback.json"
            );
            eprintln!(
                "warning: prices-fallback.json not found — \
                 currency/category blocks will be omitted"
            );
        }
        // Live prices, or a fallback the caller explicitly asked for — both already reported.
        PriceSource::Live | PriceSource::Fallback(FallbackReason::Forced) => {}
    }

    let generated = compile::compile(&inputs).map_err(miette::Error::from)?;

    // Write LF-only content to the output path.
    // T-02-03: write exactly the user-supplied --output path (no traversal, no shell);
    // surface fs errors via miette rather than panicking.
    std::fs::write(output, &generated)
        .map_err(|e| miette::miette!("Cannot write `{output}`: {e}"))?;

    println!("wrote {} bytes to {output}", generated.len());
    Ok(())
}

/// `apply` — refresh, add or remove the economy section of a filter file in place.
///
/// All presentation lives here; `crate::dynamic` returns values and prints nothing.
fn cmd_apply(
    filter: Option<&str>,
    add_economy: bool,
    remove_economy: bool,
    tiers: Option<&str>,
    styles: Option<&str>,
) -> miette::Result<()> {
    let path: PathBuf = match filter {
        Some(f) => PathBuf::from(f),
        None => {
            let choice = saved_choice()?;
            dynamic::default_filter_path(choice.game_folder.as_deref(), &choice.filter)
                .ok_or_else(|| {
                    miette::miette!(
                        "Cannot find the Documents folder; set FERRISFILTER_GAME_DIR or pass --filter <path>."
                    )
                })?
        }
    };
    let shown = path.display().to_string();
    let mut loaded = dynamic::LoadedFilter::load(&path).map_err(miette::Error::from)?;

    // Pre-flight for a plain refresh, so a missing file or section is reported before any price
    // loading happens.
    if !add_economy && !remove_economy {
        if !loaded.existed() {
            return Err(dynamic::DynamicError::FilterNotFound { path: shown }.into());
        }
        if !loaded.has_section().map_err(miette::Error::from)? {
            return Err(dynamic::DynamicError::NoEconomySection { path: shown }.into());
        }
    }

    let body = if remove_economy {
        None
    } else {
        let dirs = crate::locations::from_env().map_err(miette::Error::from)?;
        let mut inputs = InputPaths::from_dirs(&dirs);
        // A relative override resolves against the FerrisFilter folder, like the default input
        // it replaces, so `apply --tiers tiers-fallback.json` works from any working directory.
        if let Some(t) = tiers {
            eprintln!("note: --tiers {t} — using its cutoffs instead of tiers.json");
            inputs = inputs.with_tiers_or_builtin(under_root(&dirs.root, t), t);
        }
        if let Some(s) = styles {
            eprintln!("note: --styles {s} — using it instead of styles.json");
            inputs = inputs.with_styles_or_builtin(under_root(&dirs.root, s), s);
        }
        let (body, source) =
            dynamic::economy_body(&inputs, PriceSourcePolicy::Auto).map_err(miette::Error::from)?;
        match source {
            PriceSource::Fallback(FallbackReason::LiveMissingOrEmpty) => {
                eprintln!(
                    "warning: prices.json absent or empty — falling back to the built-in \
                     prices-fallback.json + tiers-fallback.json"
                );
            }
            // Absent is refused by economy_body (no price data); nothing to warn about here.
            PriceSource::Absent
            | PriceSource::Live
            | PriceSource::Fallback(FallbackReason::Forced) => {}
        }
        Some(body)
    };

    let op = match (&body, add_economy) {
        (None, _) => dynamic::ApplyOp::RemoveEconomy,
        (Some(b), true) => dynamic::ApplyOp::AddEconomy(b),
        (Some(b), false) => dynamic::ApplyOp::Refresh(b),
    };

    if loaded.migrated() {
        println!(
            "converting {shown} from the v0.4 layout: the economy section is wrapped in \
             # ff:economy markers and rule names are kept as # ff:name lines"
        );
    }

    let report = dynamic::apply_file(&mut loaded, op).map_err(miette::Error::from)?;
    match report.outcome {
        WriteOutcome::Unchanged => {
            println!("{shown} is already up to date; nothing written");
        }
        WriteOutcome::Written { backup } => {
            println!("wrote {} bytes to {shown}", report.bytes);
            if let Some(b) = backup {
                println!("kept the original as {}", b.display());
            }
            println!(
                "{}",
                if remove_economy {
                    "removed the economy section"
                } else if add_economy {
                    "added the economy section"
                } else {
                    "refreshed the economy section"
                }
            );
        }
    }
    // PoE2 never re-reads a filter on its own, and no external process can make it.
    println!("Reload it in-game: Options > Game > Item Filter > Refresh");
    Ok(())
}

/// The choices ff saved in `ff-settings.json` that the CLI defaults follow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SavedChoice {
    /// The picked game folder (absolute), or `None` for the default.
    pub game_folder: Option<PathBuf>,
    /// The selected filter's file name (`dynamic.filter` when none is saved).
    pub filter: String,
}

impl Default for SavedChoice {
    fn default() -> Self {
        Self {
            game_folder: None,
            filter: dynamic::FILTER_FILE.to_string(),
        }
    }
}

/// The game folder and filter saved in `<root>/ff-settings.json`, read in one load: an absent or
/// unusable key gives the default (Documents folder, `dynamic.filter`), and the load's warning
/// says which. Reads only; creates nothing.
pub(crate) fn saved_choice_in(root: &Path) -> (SavedChoice, Option<String>) {
    let (settings, warning) = crate::settings::load(root);
    let choice = SavedChoice {
        game_folder: settings.game_folder_path().map(Path::to_path_buf),
        filter: settings.filter_name().to_string(),
    };
    (choice, warning)
}

/// The saved choice for a resolved (or failed) app root. A root that cannot be resolved is an
/// error, so a default never silently stands in for the folder the user picked, unless
/// `FERRISFILTER_GAME_DIR` pins the folder (then only the filter name falls back, with a warning).
fn saved_choice_from(
    pinned: bool,
    root: Result<crate::locations::AppDirs, crate::locations::RootError>,
) -> Result<(SavedChoice, Option<String>), crate::locations::RootError> {
    match root {
        Ok(dirs) => Ok(saved_choice_in(&dirs.root)),
        Err(e) if pinned => Ok((
            SavedChoice::default(),
            Some(format!("{e}; using {}.", dynamic::FILTER_FILE)),
        )),
        Err(e) => Err(e),
    }
}

/// The saved choice of the app root this process resolves; a settings warning goes to stderr.
fn saved_choice() -> miette::Result<SavedChoice> {
    let (choice, warning) = saved_choice_from(
        crate::locations::game_dir_is_pinned(),
        crate::locations::from_env(),
    )
    .map_err(miette::Error::from)?;
    if let Some(w) = warning {
        eprintln!("warning: {w}");
    }
    Ok(choice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_game_folder_absent_file_is_none_and_creates_nothing() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(saved_choice_in(t.path()).0.game_folder, None);
        assert_eq!(std::fs::read_dir(t.path()).unwrap().count(), 0);
    }

    #[test]
    fn saved_game_folder_returns_the_absolute_key() {
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("game");
        let json = serde_json::json!({ "game_folder": game.to_string_lossy() });
        std::fs::write(t.path().join("ff-settings.json"), json.to_string()).unwrap();
        assert_eq!(saved_choice_in(t.path()).0.game_folder, Some(game));
    }

    #[test]
    fn saved_game_folder_relative_key_is_none() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(
            t.path().join("ff-settings.json"),
            r#"{"game_folder":"relative/dir"}"#,
        )
        .unwrap();
        assert_eq!(saved_choice_in(t.path()).0.game_folder, None);
    }

    #[test]
    fn saved_choice_reads_the_selected_filter_with_the_folder() {
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("game");
        let json =
            serde_json::json!({ "game_folder": game.to_string_lossy(), "filter": "ssf.filter" });
        std::fs::write(t.path().join("ff-settings.json"), json.to_string()).unwrap();
        assert_eq!(
            saved_choice_in(t.path()).0,
            SavedChoice {
                game_folder: Some(game),
                filter: "ssf.filter".to_string(),
            }
        );
    }

    #[test]
    fn saved_choice_without_a_usable_filter_is_dynamic_filter() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(saved_choice_in(t.path()).0.filter, "dynamic.filter");
        std::fs::write(
            t.path().join("ff-settings.json"),
            r#"{"filter":"../evil.filter"}"#,
        )
        .unwrap();
        assert_eq!(saved_choice_in(t.path()).0.filter, "dynamic.filter");
    }

    #[test]
    fn saved_choice_reports_an_unusable_key() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(
            t.path().join("ff-settings.json"),
            r#"{"game_folder":"relative/dir"}"#,
        )
        .unwrap();
        let (_, warning) = saved_choice_in(t.path());
        assert!(warning.is_some_and(|w| w.contains("relative/dir")));
    }

    #[test]
    fn an_unresolved_root_is_an_error_unless_the_game_folder_is_pinned() {
        let bad = || {
            Err(crate::locations::RootError::HomeNotADirectory {
                path: "C:/nope".to_string(),
            })
        };
        assert!(saved_choice_from(false, bad()).is_err());
        let (choice, warning) = saved_choice_from(true, bad()).unwrap();
        assert_eq!(choice, SavedChoice::default());
        assert!(warning.is_some_and(|w| w.contains("C:/nope") && w.contains("dynamic.filter")));
    }

    #[test]
    fn a_resolved_root_reads_its_settings() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(
            t.path().join("ff-settings.json"),
            r#"{"filter":"a.filter"}"#,
        )
        .unwrap();
        let dirs = crate::locations::AppDirs::at(t.path());
        let (choice, warning) = saved_choice_from(false, Ok(dirs)).unwrap();
        assert_eq!(choice.filter, "a.filter");
        assert_eq!(warning, None);
    }
}
