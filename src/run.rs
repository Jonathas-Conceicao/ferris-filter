//! Shared CLI entry point for both binary targets (`ferrisfilter` and `ff`).
//!
//! The dispatch logic lives here in the library so the two `fn main()` shims
//! (`src/main.rs` and `src/bin/ff.rs`) each have their own source file. Previously both
//! `[[bin]]` targets pointed at `src/main.rs`, which made Cargo warn that one file was
//! present in multiple build targets.

use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::models::config::Config;
use crate::{emitter, install, loader, pricer, sync};

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
    // Load and validate the config into the typed Config model.
    // serde_path_to_error + miette surface the dotted field path and expected type
    // for any malformed field.
    let config: Config = loader::load_json(config_path).map_err(miette::Error::from)?;

    // Pick the (prices, tiers) source pair. The early-league fallback always pairs
    // prices-fallback.json with tiers-fallback.json — their values are calibrated to each
    // other (exalt-denominated cutoffs), so they must travel together.
    let (prices_opt, tiers) = if early_game {
        eprintln!("note: --early-game — using prices-fallback.json + tiers-fallback.json");
        let prices = loader::load_prices_optional("prices-fallback.json").map_err(miette::Error::from)?
            .ok_or_else(|| miette::miette!("--early-game requires prices-fallback.json, which is missing"))?;
        (Some(prices), loader::load_tiers("tiers-fallback.json").map_err(miette::Error::from)?)
    } else {
        match loader::load_prices_optional("prices.json").map_err(miette::Error::from)? {
            // Live prices present and non-empty → normal path.
            Some(prices) if !prices.items.is_empty() => {
                (Some(prices), loader::load_tiers("tiers.json").map_err(miette::Error::from)?)
            }
            // Absent or empty → fall back to the committed early-league pair (with a warning).
            _ => {
                eprintln!(
                    "warning: prices.json absent or empty — falling back to \
                     prices-fallback.json + tiers-fallback.json"
                );
                match loader::load_prices_optional("prices-fallback.json").map_err(miette::Error::from)? {
                    Some(prices) => (Some(prices), loader::load_tiers("tiers-fallback.json").map_err(miette::Error::from)?),
                    None => {
                        eprintln!(
                            "warning: prices-fallback.json not found — \
                             currency/category blocks will be omitted"
                        );
                        (None, loader::load_tiers("tiers.json").map_err(miette::Error::from)?)
                    }
                }
            }
        }
    };

    let priced = match prices_opt {
        None => pricer::PricedData::empty(),
        Some(prices) => {
            // TIER-04: abort on bad reference rates (names the offending currency).
            let rates = pricer::derive_rates(&prices.items)?;
            pricer::price_tiers(&prices, &tiers, &rates)?
        }
    };

    // Generate the filter text (pure function — no I/O inside).
    let generated = emitter::generate(&config, &priced)
        .map_err(miette::Error::from)?;

    // Write LF-only content to the output path.
    // T-02-03: write exactly the user-supplied --output path (no traversal, no shell);
    // surface fs errors via miette rather than panicking.
    std::fs::write(output, &generated)
        .map_err(|e| miette::miette!("Cannot write `{output}`: {e}"))?;

    println!("wrote {} bytes to {output}", generated.len());
    Ok(())
}
