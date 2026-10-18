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

/// Refresh `prices.json` in `base_dir`, blocking until it finishes.
///
/// The overlay's economy-sync affordance calls this from a worker thread — it owns its own tokio
/// runtime exactly as [`cmd_sync`] does, so the caller needs no async machinery of its own. League
/// selection uses the same automatic resolution the bare CLI `sync` uses; the overlay deliberately
/// exposes no `--standard`/`--hc`/`--league`/`--tiers` switches (author, 2026-09-10: the GUI syncs
/// with the tiers file already in place).
///
/// `prices.json` lands in `base_dir`; the mapping is built into the binary.
///
/// Nothing here touches `dynamic.filter`. Fresh prices reach the game on the next **Apply**, which
/// is what makes this safe to run mid-session.
/// The league the overlay syncs against.
///
/// **Pinned, because poe.ninja's `indexed` flag cannot be trusted to identify the current
/// league.** Automatic resolution takes the first non-hardcore *indexed* league, and that flag
/// flaps: observed 2026-09-10 20:56Z with `Forbidden Rites indexed=true, Runes of Aldur
/// indexed=false`, and 2026-09-11 07:00Z with exactly the reverse. A sync in the wrong half of
/// that cycle writes last league's economy into `prices.json`, and the resulting filter looks
/// completely normal — which is how the author ended up with Runes of Aldur prices while playing
/// Forbidden Rites (screenshot poe2_print007).
///
/// **This must be updated at the start of each league.** It is a deliberate literal, not an
/// oversight: a wrong pin fails loudly with the list of available leagues, whereas trusting the
/// flag fails silently. The CLI keeps its automatic resolution and its `--league` override; only
/// the GUI is pinned.
pub const OVERLAY_LEAGUE: &str = "Forbidden Rites";

/// Returns the league the prices were actually fetched for, so the caller can show it — a stale
/// [`OVERLAY_LEAGUE`] is then visible in the success line rather than silent.
pub fn sync_economy_blocking(base_dir: &std::path::Path) -> miette::Result<String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| miette::miette!("Cannot create async runtime: {e}"))?;
    rt.block_on(async {
        let client = ReqwestClient::new()?;
        prices::run_sync_prices(&client, false, false, Some(OVERLAY_LEAGUE), base_dir).await
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
