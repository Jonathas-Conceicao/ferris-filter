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
    rt.block_on(run(standard, hc, league, sub, std::path::Path::new(".")))
}

/// Refresh `prices.json` in `base_dir`, blocking until it finishes.
///
/// The overlay's economy-sync affordance calls this from a worker thread — it owns its own tokio
/// runtime exactly as [`cmd_sync`] does, so the caller needs no async machinery of its own. League
/// selection uses the same automatic resolution the bare CLI `sync` uses; the overlay deliberately
/// exposes no `--standard`/`--hc`/`--league`/`--tiers` switches (author, 2026-09-10: the GUI syncs
/// with the tiers file already in place).
///
/// Nothing here touches `dynamic.filter`. Fresh prices reach the game on the next **Apply**, which
/// is what makes this safe to run mid-session.
pub fn sync_economy_blocking(base_dir: &std::path::Path) -> miette::Result<()> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| miette::miette!("Cannot create async runtime: {e}"))?;
    rt.block_on(async {
        let client = ReqwestClient::new()?;
        prices::run_sync_prices(&client, false, false, None, base_dir).await
    })
}

async fn run(
    standard: bool,
    hc: bool,
    league: Option<String>,
    sub: SyncSubcommand,
    base_dir: &std::path::Path,
) -> miette::Result<()> {
    match sub {
        SyncSubcommand::Prices => {
            let client = ReqwestClient::new()?;
            prices::run_sync_prices(&client, standard, hc, league.as_deref(), base_dir).await
        }
        SyncSubcommand::Metadata => {
            let client = ReqwestClient::new()?;
            metadata::cmd_sync_metadata(&client).await
        }
    }
}
