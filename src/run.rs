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
use crate::{install, sync};

/// Parse CLI args and dispatch to the matching subcommand.
pub fn run() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate { config, output, early_game } => cmd_generate(&config, &output, early_game)?,
        Commands::Sync { standard, hc, sub } => sync::cmd_sync(standard, hc, sub)?,
        Commands::Install { input, dest } => install::cmd_install(&input, dest)?,
    }
    Ok(())
}

fn cmd_generate(config_path: &str, output: &str, early_game: bool) -> miette::Result<()> {
    // Bare relative names, resolved against the process working directory — exactly the paths
    // this function passed before the compile API existed. `conventional()` rather than
    // `relative_to(".")` because the latter would render as `./config.json` and change every
    // error message that names a file.
    let paths = InputPaths::conventional().with_config(config_path);

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
