mod cli;
mod emitter;
mod error;
mod loader;
mod models;

use cli::{Cli, Commands};
use clap::Parser;
use models::config::Config;
use models::currency::CurrencyData;

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate { output } => cmd_generate(&output)?,
        Commands::Sync => cmd_sync(),
    }
    Ok(())
}

fn cmd_generate(output: &str) -> miette::Result<()> {
    // Load and validate config.json into the typed Config model.
    // serde_path_to_error + miette surface the dotted field path and expected type
    // for any malformed field (satisfies ROADMAP success criterion 3).
    let config: Config = loader::load_json("config.json")?;

    // Load and validate currency.json (curated tier base lists).
    let currency: CurrencyData = loader::load_json("currency.json")?;

    // Load and validate tiers.json (completeness of S-F + within-unit ascending check).
    // Phase 3 consumes this for price-driven tiering; we keep the load for validation.
    let _tiers = loader::load_tiers("tiers.json")?;

    // Generate the filter text (pure function — no I/O inside).
    let generated = emitter::generate(&config, &currency);

    // Write LF-only content to the output path.
    // T-02-03: write exactly the user-supplied --output path (no traversal, no shell);
    // surface fs errors via miette rather than panicking.
    std::fs::write(output, &generated)
        .map_err(|e| miette::miette!("Cannot write `{output}`: {e}"))?;

    println!("wrote {} bytes to {output}", generated.len());
    Ok(())
}

fn cmd_sync() {
    println!("sync not yet implemented");
}
