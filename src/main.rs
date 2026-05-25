mod cli;
mod emitter;
mod error;
mod loader;
mod models;
mod pricer;
mod sync;

use cli::{Cli, Commands};
use clap::Parser;
use models::config::Config;

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate { output } => cmd_generate(&output)?,
        Commands::Sync { sub } => sync::cmd_sync(sub)?,
    }
    Ok(())
}

fn cmd_generate(output: &str) -> miette::Result<()> {
    // Load and validate config.json into the typed Config model.
    // serde_path_to_error + miette surface the dotted field path and expected type
    // for any malformed field.
    let config: Config = loader::load_json("config.json")?;

    // Load and validate tiers.json (completeness of S-F + within-unit ascending check).
    let tiers = loader::load_tiers("tiers.json")?;

    // GEN-05: load prices.json if present, warn + skip currency/category blocks if absent.
    let prices_opt = loader::load_prices_optional("prices.json")?;

    let priced = match prices_opt {
        None => {
            eprintln!("warning: prices.json not found — currency/category blocks will be omitted");
            pricer::PricedData::empty()
        }
        Some(prices) => {
            // TIER-04: abort on bad reference rates (names the offending currency).
            let rates = pricer::derive_rates(&prices.items)?;
            pricer::price_tiers(&prices, &tiers, &rates)?
        }
    };

    // Generate the filter text (pure function — no I/O inside).
    let generated = emitter::generate(&config, &priced);

    // Write LF-only content to the output path.
    // T-02-03: write exactly the user-supplied --output path (no traversal, no shell);
    // surface fs errors via miette rather than panicking.
    std::fs::write(output, &generated)
        .map_err(|e| miette::miette!("Cannot write `{output}`: {e}"))?;

    println!("wrote {} bytes to {output}", generated.len());
    Ok(())
}
