pub mod client;
pub mod league;
pub mod mapping;
pub mod metadata;
pub mod models;
pub mod prices;

use crate::cli::SyncSubcommand;
use client::ReqwestClient;

pub fn cmd_sync(sub: Option<SyncSubcommand>) -> miette::Result<()> {
    let sub = sub.unwrap_or(SyncSubcommand::Prices { standard: false, hc: false });
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| miette::miette!("Cannot create async runtime: {e}"))?;
    rt.block_on(run(sub))
}

async fn run(sub: SyncSubcommand) -> miette::Result<()> {
    match sub {
        SyncSubcommand::Prices { standard, hc } => {
            let client = ReqwestClient::new()?;
            prices::run_sync_prices(&client, standard, hc).await
        }
        SyncSubcommand::Metadata => {
            let client = ReqwestClient::new()?;
            metadata::cmd_sync_metadata(&client).await
        }
    }
}
