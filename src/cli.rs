use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ferrisfilter", about = "Compile a PoE2 loot filter from your config")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compile config.json (+ tiers.json + prices.json) into a .filter file
    #[command(alias = "gen")]
    Generate {
        /// Output file path [default: universal.filter]
        #[arg(long, default_value = "universal.filter")]
        output: String,
    },
    /// Fetch live PoE2 prices from poe.ninja (not yet implemented)
    Sync,
}
