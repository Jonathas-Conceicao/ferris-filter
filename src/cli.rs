use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "ferrisfilter",
    about = "Compile a PoE2 loot filter from your config"
)]
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
        /// Tier-cutoff file to use instead of tiers.json.
        ///
        /// Use --tiers tiers-fallback.json early in a live league: its cutoffs are
        /// exalt-denominated, so Divine is not pinned to the A floor by an expensive chaos.
        /// Unlike --early-game this keeps your live prices.json.
        #[arg(long)]
        tiers: Option<String>,
        /// Styles file to use instead of styles.json.
        ///
        /// Named styles live in styles.json beside the inputs; point this at another file to
        /// compile against a different style set (for example one carrying the styles of an
        /// experimental config) without editing the committed one.
        #[arg(long)]
        styles: Option<String>,
    },
    /// Fetch live PoE2 economy data from poe.ninja (default: prices)
    ///
    /// Writes prices.json into the FerrisFilter folder (FERRISFILTER_HOME, else
    /// %APPDATA%\FerrisFilter; ~/.config/ferrisfilter on Linux), creating it if missing.
    Sync {
        /// Target the permanent Standard league
        #[arg(long)]
        standard: bool,
        /// Target the Hardcore variant (combine with --standard for permanent Hardcore)
        #[arg(long)]
        hc: bool,
        /// Fetch a specific league by display name or url slug, e.g.
        /// --league "Forbidden Rites" (or --league forbiddenrites).
        ///
        /// Overrides --standard/--hc. Needed during a league transition: poe.ninja lists the
        /// new league before flagging it as indexed, so the automatic softcore pick still
        /// resolves to the outgoing league even though the new one already has live data.
        #[arg(long)]
        league: Option<String>,
        #[command(subcommand)]
        sub: Option<SyncSubcommand>,
    },
    /// Refresh the economy section of a filter in place
    ///
    /// The filter itself is the store: only the blocks under the `# ff:economy-begin` banner (up
    /// to the next banner or the end of the file) are regenerated from prices.json, tiers.json
    /// and styles.json found in the FerrisFilter folder (FERRISFILTER_HOME, else
    /// %APPDATA%\FerrisFilter; ~/.config/ferrisfilter on Linux). A missing styles.json,
    /// tiers.json or prices.json falls back to the built-in defaults and fallback prices. The
    /// first write keeps `<name>.filter.ffbak`; a v0.4 dynamic.filter is
    /// converted on its first apply. PoE2 still needs the filter reloaded in-game afterwards.
    Apply {
        /// Filter file to update
        /// [default: dynamic.filter in the game folder: FERRISFILTER_GAME_DIR, else the folder picked in ff (ff-settings.json game_folder), else Documents\My Games\Path of Exile 2]
        #[arg(long)]
        filter: Option<String>,
        /// Add an economy section at the end of the filter (creates the file if it does not
        /// exist; its folder must exist)
        #[arg(long, conflicts_with = "remove_economy")]
        add_economy: bool,
        /// Remove the economy section from the filter
        #[arg(long)]
        remove_economy: bool,
        /// Tier-cutoff file to use instead of tiers.json.
        ///
        /// Use --tiers tiers-fallback.json early in a live league: its cutoffs are
        /// exalt-denominated, so Divine is not pinned to the A floor by an expensive chaos.
        /// Unlike --early-game this keeps your live prices.json. A relative path is resolved
        /// against the FerrisFilter folder, like tiers.json itself. A bare built-in name
        /// (tiers-fallback.json, tiers-default.json) that is not there uses the built-in copy.
        #[arg(long)]
        tiers: Option<String>,
        /// Styles file to use instead of styles.json.
        ///
        /// Named styles live in styles.json beside the inputs; point this at another file to
        /// compile against a different style set (for example one carrying the styles of an
        /// experimental config) without editing the committed one. A relative path is resolved
        /// against the FerrisFilter folder, like styles.json itself. The bare name
        /// styles-default.json uses the built-in copy when it is not there.
        #[arg(long)]
        styles: Option<String>,
    },
    /// Copy a generated .filter into the PoE2 config folder so it takes effect in-game
    Install {
        /// Filter file to install [default: universal.filter]
        #[arg(long, default_value = "universal.filter")]
        input: String,
        /// Destination PoE2 config folder
        /// [default: the game folder: FERRISFILTER_GAME_DIR, else the folder picked in ff (ff-settings.json game_folder), else Documents\My Games\Path of Exile 2]
        #[arg(long)]
        dest: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum SyncSubcommand {
    /// Fetch prices from poe.ninja and write prices.json into the FerrisFilter folder
    /// (default; honors --standard/--hc)
    Prices,
    /// Rebuild the repo mapping.json in the working directory from the RePoE-fork exports
    /// (the binary embeds that file, so rebuild afterwards)
    Metadata,
}
