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
use crate::{install, paths, sync};
use std::path::PathBuf;

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
        } => cmd_generate(&config, &output, early_game, tiers.as_deref(), styles.as_deref())?,
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
        Commands::Install { input, dest } => install::cmd_install(&input, dest)?,
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
        None => dynamic::default_filter_path().ok_or_else(|| {
            miette::miette!(
                "Cannot resolve the default PoE2 config folder (USERPROFILE is not set). \
                 Pass --filter <path>."
            )
        })?,
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
        let root = paths::app_root().map_err(miette::Error::from)?;
        let mut inputs = InputPaths::relative_to(&root);
        if let Some(t) = tiers {
            eprintln!("note: --tiers {t} — using its cutoffs instead of tiers.json");
            inputs = inputs.with_tiers(t);
        }
        if let Some(s) = styles {
            eprintln!("note: --styles {s} — using it instead of styles.json");
            inputs = inputs.with_styles(s);
        }
        let (body, source) = dynamic::economy_body(&inputs, PriceSourcePolicy::Auto)
            .map_err(miette::Error::from)?;
        match source {
            PriceSource::Fallback(FallbackReason::LiveMissingOrEmpty) => {
                eprintln!(
                    "warning: prices.json absent or empty — falling back to \
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
             #ff:economy markers and rule names are kept as #ff:name lines"
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
