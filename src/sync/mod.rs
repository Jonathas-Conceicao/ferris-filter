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
    rt.block_on(run(standard, hc, league, sub))
}

async fn run(
    standard: bool,
    hc: bool,
    league: Option<String>,
    sub: SyncSubcommand,
) -> miette::Result<()> {
    match sub {
        SyncSubcommand::Prices => {
            let client = ReqwestClient::new()?;
            prices::run_sync_prices(&client, standard, hc, league.as_deref()).await
        }
        SyncSubcommand::Metadata => {
            let client = ReqwestClient::new()?;
            metadata::cmd_sync_metadata(&client).await
        }
    }
}
