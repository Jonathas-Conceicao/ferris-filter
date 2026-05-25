pub mod client;
pub mod models;

use crate::cli::SyncSubcommand;

pub fn cmd_sync(sub: Option<SyncSubcommand>) -> miette::Result<()> {
    let sub = sub.unwrap_or(SyncSubcommand::Prices { standard: false, hc: false });
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| miette::miette!("Cannot create async runtime: {e}"))?;
    rt.block_on(run(sub))
}

async fn run(sub: SyncSubcommand) -> miette::Result<()> {
    match sub {
        SyncSubcommand::Prices { standard, hc } => {
            // Implemented in plan 04-03.
            let _ = (standard, hc);
            Err(miette::miette!("sync prices not yet implemented"))
        }
        SyncSubcommand::Metadata => {
            // Implemented in plan 04-02.
            Err(miette::miette!("sync metadata not yet implemented"))
        }
    }
}
