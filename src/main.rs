mod cli;
mod error;
mod loader;
mod models;

use cli::{Cli, Commands};
use clap::Parser;

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate { output } => cmd_generate(&output)?,
        Commands::Sync => cmd_sync(),
    }
    Ok(())
}

fn cmd_generate(output: &str) -> miette::Result<()> {
    // Load and validate config.json (field-path errors via serde_path_to_error + miette).
    // NOTE: In Phase 1 the config is loaded as a generic JSON value so the loader/error
    // path is exercisable end-to-end. Phase 2 will replace serde_json::Value with the
    // full typed Config model once plan 01-02 models are merged.
    let config: serde_json::Value = loader::load_json("config.json")?;

    // Load and validate tiers.json (completeness + within-unit ascending check).
    let tiers = loader::load_tiers("tiers.json")?;

    let n_styles = config
        .get("styles")
        .and_then(|s| s.as_object())
        .map(|m| m.len())
        .unwrap_or(0);

    println!(
        "generate not yet implemented (output: {output}, {n_styles} styles, {} tiers)",
        tiers.0.len()
    );
    Ok(())
}

fn cmd_sync() {
    println!("sync not yet implemented");
}
