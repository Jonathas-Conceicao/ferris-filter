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
    /// Fetch live PoE2 economy data from poe.ninja (default: prices)
    Sync {
        /// Target the permanent Standard league
        #[arg(long)]
        standard: bool,
        /// Target the Hardcore variant (combine with --standard for permanent Hardcore)
        #[arg(long)]
        hc: bool,
        #[command(subcommand)]
        sub: Option<SyncSubcommand>,
    },
    /// Copy a generated .filter into the PoE2 config folder so it takes effect in-game
    Install {
        /// Filter file to install [default: universal.filter]
        #[arg(long, default_value = "universal.filter")]
        input: String,
        /// Destination PoE2 config folder
        /// [default: %USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2]
        #[arg(long)]
        dest: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum SyncSubcommand {
    /// Fetch prices from poe.ninja and write prices.json (default; honors --standard/--hc)
    Prices,
    /// Rebuild the committed item-class mapping from the RePoE-fork exports
    Metadata,
}
