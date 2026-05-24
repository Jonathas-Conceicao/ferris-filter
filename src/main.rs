mod cli;
mod emitter;
mod error;
mod loader;
mod models;

use cli::{Cli, Commands};
use clap::Parser;
use models::config::Config;

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate { output } => cmd_generate(&output)?,
        Commands::Sync => cmd_sync(),
    }
    Ok(())
}

fn cmd_generate(output: &str) -> miette::Result<()> {
    // Load and validate config.json into the typed `Config` model. serde_path_to_error +
    // miette surface the dotted field path and expected type for any malformed field
    // (e.g. a non-numeric `styles.S.font` -> "styles.S.font: invalid type ... expected u8"),
    // satisfying ROADMAP success criterion 3. Actual filter generation is Phase 2.
    let config: Config = loader::load_json("config.json")?;

    // Load and validate tiers.json (completeness of S-F + within-unit ascending check).
    let tiers = loader::load_tiers("tiers.json")?;

    println!(
        "generate not yet implemented (output: {output}, {} tiers)",
        tiers.0.len()
    );
    Ok(())
}

fn cmd_sync() {
    println!("sync not yet implemented");
}
