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
use std::path::{Path, PathBuf};
use crate::{install, sync};

/// Parse CLI args and dispatch to the matching subcommand.
pub fn run() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate {
            config,
            output,
            early_game,
            tiers,
        } => cmd_generate(&config, &output, early_game, tiers.as_deref())?,
        Commands::Apply {
            store,
            base,
            dest,
            reset,
            tiers,
        } => cmd_apply(&store, &base, dest, reset, tiers.as_deref())?,
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

/// `apply` — compile the dynamic store and write the result into the game folder.
///
/// The dynamic counterpart to `generate` + `install` in one step. All presentation lives here;
/// `crate::dynamic` returns values and prints nothing.
fn cmd_apply(
    store: &str,
    base: &str,
    dest: Option<String>,
    reset: bool,
    tiers: Option<&str>,
) -> miette::Result<()> {
    let store_path = Path::new(store);
    let base_path = Path::new(base);

    if reset {
        dynamic::reset_store(base_path, store_path).map_err(miette::Error::from)?;
        println!("reset {store} from {base}");
    } else {
        match dynamic::ensure_store(base_path, store_path).map_err(miette::Error::from)? {
            dynamic::StoreState::Seeded => println!("created {store} from {base}"),
            dynamic::StoreState::Existing => println!("using existing {store}"),
        }
    }

    let mut paths = InputPaths::conventional();
    if let Some(t) = tiers {
        eprintln!("note: --tiers {t} — using its cutoffs instead of tiers.json");
        paths = paths.with_tiers(t);
    }
    let compiled = dynamic::compile_store(&paths, store_path).map_err(miette::Error::from)?;

    // Same wording cmd_generate uses for the same conditions — one message per situation, not two
    // spellings of it.
    match compiled.price_source {
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
        PriceSource::Live | PriceSource::Fallback(FallbackReason::Forced) => {}
    }

    let dest_dir = match dest {
        Some(d) => PathBuf::from(d),
        None => dynamic::default_destination().ok_or_else(|| {
            miette::miette!(
                "Cannot resolve the default PoE2 config folder (USERPROFILE is not set). \
                 Pass --dest <folder>."
            )
        })?,
    };

    let written = dynamic::write_filter(&dest_dir, &compiled.text).map_err(miette::Error::from)?;

    println!(
        "wrote {} bytes to {}",
        compiled.text.len(),
        written.display()
    );
    // PoE2 never re-reads a filter on its own, and no external process can make it. Saying so is
    // not a nicety: a tool that writes silently reads as broken when drops do not change.
    println!("Reload it in-game: Options > Game > Item Filter > Refresh");
    Ok(())
}
