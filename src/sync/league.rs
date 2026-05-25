// League resolution from poe.ninja index-state.
// Maps the (standard, hc) flag combination to the correct league display name.

use crate::sync::models::IndexState;

pub const INDEX_STATE_URL: &str = "https://poe.ninja/poe2/api/data/index-state";

/// Resolve the league display name from the poe.ninja index-state per the flag matrix.
///
/// # Flag → League Matrix
/// | standard | hc    | League chosen                                             |
/// |----------|-------|-----------------------------------------------------------|
/// | false    | false | First economyLeague where !hardcore && indexed (SC trade) |
/// | false    | true  | First economyLeague where hardcore && url != "hardcore"   |
/// | true     | false | economyLeague where url == "standard"                     |
/// | true     | true  | economyLeague where url == "hardcore"                     |
///
/// Returns the `name` field (display name), NOT the `url` slug (Pitfall 3).
pub fn resolve_league(state: &IndexState, standard: bool, hc: bool) -> miette::Result<String> {
    match (standard, hc) {
        (false, false) => state
            .economy_leagues
            .iter()
            .find(|l| !l.hardcore && l.indexed)
            .map(|l| l.name.clone())
            .ok_or_else(|| {
                miette::miette!("No active softcore trade league found in poe.ninja index-state")
            }),
        (false, true) => state
            .economy_leagues
            .iter()
            .find(|l| l.hardcore && l.url != "hardcore")
            .map(|l| l.name.clone())
            .ok_or_else(|| {
                miette::miette!(
                    "No active hardcore trade league found in poe.ninja index-state"
                )
            }),
        (true, false) => state
            .economy_leagues
            .iter()
            .find(|l| l.url == "standard")
            .map(|l| l.name.clone())
            .ok_or_else(|| {
                miette::miette!("Standard league not found in poe.ninja index-state")
            }),
        (true, true) => state
            .economy_leagues
            .iter()
            .find(|l| l.url == "hardcore")
            .map(|l| l.name.clone())
            .ok_or_else(|| {
                miette::miette!("Hardcore league not found in poe.ninja index-state")
            }),
    }
}

/// Fetch the poe.ninja index-state and return the parsed IndexState.
pub async fn fetch_index_state<C: crate::sync::client::HttpClient>(
    client: &C,
) -> miette::Result<IndexState> {
    let body = client.get_json(INDEX_STATE_URL, &[]).await?;
    serde_json::from_str(&body)
        .map_err(|e| miette::miette!("Cannot parse index-state response: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::models::{EconomyLeague, IndexState};

    fn make_state(leagues: Vec<EconomyLeague>) -> IndexState {
        IndexState {
            economy_leagues: leagues,
        }
    }

    fn league(name: &str, url: &str, hardcore: bool, indexed: bool) -> EconomyLeague {
        EconomyLeague {
            name: name.to_string(),
            url: url.to_string(),
            hardcore,
            indexed,
        }
    }

    /// Fixture matching 04-RESEARCH §SPIKE index-state response:
    /// softcore indexed trade + HC trade + Standard + Hardcore
    fn fixture_state() -> IndexState {
        make_state(vec![
            league("Fate of the Vaal", "vaal", false, true),
            league("HC Fate of the Vaal", "vaalhc", true, false),
            league("Standard", "standard", false, false),
            league("Hardcore", "hardcore", true, false),
        ])
    }

    #[test]
    fn softcore_trade_resolves_to_current_indexed_league() {
        let state = fixture_state();
        let name = resolve_league(&state, false, false).expect("should resolve softcore league");
        // Must return the display name, NOT the url slug "vaal"
        assert_eq!(name, "Fate of the Vaal");
    }

    #[test]
    fn hc_trade_resolves_to_hc_non_permanent() {
        let state = fixture_state();
        let name = resolve_league(&state, false, true).expect("should resolve HC trade league");
        // Must return the display name "HC Fate of the Vaal", NOT the slug "vaalhc"
        assert_eq!(name, "HC Fate of the Vaal");
    }

    #[test]
    fn standard_resolves_to_standard_league() {
        let state = fixture_state();
        let name = resolve_league(&state, true, false).expect("should resolve Standard league");
        assert_eq!(name, "Standard");
    }

    #[test]
    fn standard_hc_resolves_to_hardcore_league() {
        let state = fixture_state();
        let name = resolve_league(&state, true, true).expect("should resolve Hardcore league");
        assert_eq!(name, "Hardcore");
    }

    #[test]
    fn empty_league_list_returns_err_softcore() {
        let state = make_state(vec![]);
        let result = resolve_league(&state, false, false);
        assert!(result.is_err(), "empty list should return Err for softcore");
    }

    #[test]
    fn empty_league_list_returns_err_hc() {
        let state = make_state(vec![]);
        let result = resolve_league(&state, false, true);
        assert!(result.is_err(), "empty list should return Err for HC trade");
    }

    #[test]
    fn display_name_not_url_slug() {
        // Explicitly verify that the returned value is the 'name' field, not 'url'.
        // This guards against Pitfall 3 (passing the url slug as league= query param).
        let state = fixture_state();
        let name = resolve_league(&state, false, false).unwrap();
        assert_ne!(name, "vaal", "should return display name, not url slug");
        assert_eq!(name, "Fate of the Vaal");
    }
}
