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
        /// Config file to compile [default: config.json]
        #[arg(long, default_value = "config.json")]
        config: String,
        /// Output file path [default: universal.filter]
        #[arg(long, default_value = "universal.filter")]
        output: String,
        /// Force the early-league fallback (prices-fallback.json + tiers-fallback.json)
        /// even when a live prices.json exists
        #[arg(long)]
        early_game: bool,
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
    /// Compile dynamic.json into dynamic.filter and write it to the PoE2 config folder
    ///
    /// Seeds dynamic.json from dynamic_base.json on first run. Unlike `generate`, this both
    /// compiles and installs — PoE2 still needs the filter reloaded in-game afterwards.
    Apply {
        /// Dynamic rule store to compile [default: dynamic.json]
        #[arg(long, default_value = "dynamic.json")]
        store: String,
        /// Template used to seed the store, and to restore it with --reset
        /// [default: dynamic_base.json]
        #[arg(long, default_value = "dynamic_base.json")]
        base: String,
        /// Destination PoE2 config folder
        /// [default: %USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2]
        #[arg(long)]
        dest: Option<String>,
        /// Discard the current store and restore it from the base before compiling
        #[arg(long)]
        reset: bool,
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
