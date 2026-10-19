pub mod client;
pub mod league;
pub mod mapping;
pub mod metadata;
pub mod models;
pub mod prices;

use crate::cli::SyncSubcommand;
use client::ReqwestClient;

/// `--standard`/`--hc` live on the top-level `sync` command (not the `prices` subcommand)
/// so `ferrisfilter sync --standard` parses. They apply to the default/`prices` run and are
/// ignored by `metadata` (which is league-agnostic).
pub fn cmd_sync(
    standard: bool,
    hc: bool,
    league: Option<String>,
    sub: Option<SyncSubcommand>,
) -> miette::Result<()> {
    let sub = sub.unwrap_or(SyncSubcommand::Prices);
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| miette::miette!("Cannot create async runtime: {e}"))?;
    let base_dir = match sub {
        SyncSubcommand::Prices => {
            crate::locations::from_env()
                .map_err(miette::Error::from)?
                .root
        }
        SyncSubcommand::Metadata => std::path::PathBuf::from("."),
    };
    rt.block_on(run(standard, hc, league, sub, &base_dir))?;
    Ok(())
}

/// Refresh `prices.json` in `base_dir` for `league`, blocking until it finishes.
///
/// The overlay's economy-sync affordance calls this from a worker thread; it owns its own tokio
/// runtime exactly as [`cmd_sync`] does, so the caller needs no async machinery of its own. The
/// overlay syncs the league the user picked on the Economy tab (saved in ff-settings.json as
/// `league`, else the default from [`league::default_league`]); it exposes no
/// `--standard`/`--hc`/`--tiers` switches (the GUI syncs with the tiers file already in place).
///
/// `run_sync_prices` re-resolves the name against poe.ninja, so a league that ended between
/// listing and syncing fails loudly with the list of available leagues.
///
/// `prices.json` lands in `base_dir`; the mapping is built into the binary. Nothing here touches
/// `dynamic.filter`: fresh prices reach the game on the next **Apply**, which is what makes this
/// safe to run mid-session.
///
/// Returns the league the prices were actually fetched for, so the caller can show it.
pub fn sync_economy_blocking(base_dir: &std::path::Path, league: &str) -> miette::Result<String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| miette::miette!("Cannot create async runtime: {e}"))?;
    rt.block_on(async {
        let client = ReqwestClient::new()?;
        prices::run_sync_prices(&client, false, false, Some(league), base_dir).await
    })
}

/// Fetch poe.ninja's economy league list, blocking until it finishes.
///
/// Called only from the overlay's league-list worker thread; never from tests.
pub fn fetch_leagues_blocking() -> miette::Result<Vec<models::EconomyLeague>> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| miette::miette!("Cannot create async runtime: {e}"))?;
    rt.block_on(async {
        let client = ReqwestClient::new()?;
        league::list_leagues(&client).await
    })
}

async fn run(
    standard: bool,
    hc: bool,
    league: Option<String>,
    sub: SyncSubcommand,
    base_dir: &std::path::Path,
) -> miette::Result<String> {
    match sub {
        SyncSubcommand::Prices => {
            let client = ReqwestClient::new()?;
            prices::run_sync_prices(&client, standard, hc, league.as_deref(), base_dir).await
        }
        SyncSubcommand::Metadata => {
            let client = ReqwestClient::new()?;
            metadata::cmd_sync_metadata(&client).await?;
            Ok(String::new())
        }
    }
}
