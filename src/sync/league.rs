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
                miette::miette!("No active hardcore trade league found in poe.ninja index-state")
            }),
        (true, false) => state
            .economy_leagues
            .iter()
            .find(|l| l.url == "standard")
            .map(|l| l.name.clone())
            .ok_or_else(|| miette::miette!("Standard league not found in poe.ninja index-state")),
        (true, true) => state
            .economy_leagues
            .iter()
            .find(|l| l.url == "hardcore")
            .map(|l| l.name.clone())
            .ok_or_else(|| miette::miette!("Hardcore league not found in poe.ninja index-state")),
    }
}

/// Resolve the league, honouring an explicit `--league` override when given.
///
/// The flag matrix in [`resolve_league`] assumes exactly one *indexed* softcore trade league.
/// That assumption breaks during a league transition, when poe.ninja lists the outgoing and
/// incoming leagues side by side and flags only the outgoing one as `indexed` — the new league
/// carries live data well before its `indexed` flag flips. In that window no combination of
/// `--standard`/`--hc` can reach the new softcore league, which is why the override exists.
///
/// The override matches on `name` first, then on the `url` slug, both case-insensitively, so
/// `--league "Forbidden Rites"` and `--league forbiddenrites` both work. `indexed` is
/// deliberately NOT consulted here: naming a league explicitly is the caller asserting they
/// want it.
pub fn resolve_league_with_override(
    state: &IndexState,
    standard: bool,
    hc: bool,
    league: Option<&str>,
) -> miette::Result<String> {
    let Some(wanted) = league else {
        return resolve_league(state, standard, hc);
    };

    state
        .economy_leagues
        .iter()
        .find(|l| l.name.eq_ignore_ascii_case(wanted) || l.url.eq_ignore_ascii_case(wanted))
        .map(|l| l.name.clone())
        .ok_or_else(|| {
            let available = state
                .economy_leagues
                .iter()
                .map(|l| format!("  {} ({})", l.name, l.url))
                .collect::<Vec<_>>()
                .join("\n");
            miette::miette!(
                "No league matching `{wanted}` in poe.ninja index-state.\nAvailable:\n{available}"
            )
        })
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

    // --- explicit --league override (league-transition window) ---

    /// The real poe.ninja shape observed 2026-09-06: the incoming league is listed first but
    /// is NOT flagged indexed, while the outgoing league still is.
    fn transition_state() -> IndexState {
        make_state(vec![
            league("Forbidden Rites", "forbiddenrites", false, false),
            league("Runes of Aldur", "runesofaldur", false, true),
            league("HC Forbidden Rites", "forbiddenriteshc", true, false),
            league("HC Runes of Aldur", "runesofaldurhc", true, false),
            league("Standard", "standard", false, false),
            league("Hardcore", "hardcore", true, false),
        ])
    }

    #[test]
    fn without_override_a_transition_resolves_to_the_indexed_outgoing_league() {
        // Documents the behavior the override exists to work around: the flag matrix alone
        // cannot reach a live-but-not-yet-indexed new league.
        let name = resolve_league_with_override(&transition_state(), false, false, None).unwrap();
        assert_eq!(name, "Runes of Aldur");
    }

    #[test]
    fn override_reaches_a_league_that_is_not_flagged_indexed() {
        let name = resolve_league_with_override(
            &transition_state(),
            false,
            false,
            Some("Forbidden Rites"),
        )
        .unwrap();
        assert_eq!(name, "Forbidden Rites");
    }

    #[test]
    fn override_matches_the_url_slug_and_ignores_case() {
        for wanted in ["forbiddenrites", "FORBIDDEN RITES", "fOrBiDdEnRiTeS"] {
            let name =
                resolve_league_with_override(&transition_state(), false, false, Some(wanted))
                    .unwrap_or_else(|e| panic!("`{wanted}` should resolve: {e}"));
            assert_eq!(name, "Forbidden Rites", "input was `{wanted}`");
        }
    }

    #[test]
    fn override_returns_the_display_name_not_the_slug() {
        let name =
            resolve_league_with_override(&transition_state(), false, false, Some("forbiddenrites"))
                .unwrap();
        assert_eq!(
            name, "Forbidden Rites",
            "the league= query param takes the display name (Pitfall 3)"
        );
    }

    #[test]
    fn unknown_override_errors_and_lists_what_is_available() {
        let err = resolve_league_with_override(
            &transition_state(),
            false,
            false,
            Some("Forbidden Rights"), // plausible typo
        )
        .expect_err("an unknown league must not silently fall back to another one");
        let msg = format!("{err}");
        assert!(
            msg.contains("Forbidden Rights"),
            "names what was asked for: {msg}"
        );
        assert!(
            msg.contains("Forbidden Rites"),
            "lists the real options: {msg}"
        );
        assert!(msg.contains("runesofaldur"), "lists slugs too: {msg}");
    }

    #[test]
    fn override_takes_precedence_over_the_standard_and_hc_flags() {
        let name =
            resolve_league_with_override(&transition_state(), true, true, Some("Forbidden Rites"))
                .unwrap();
        assert_eq!(
            name, "Forbidden Rites",
            "an explicit league is the caller being specific; flags must not override it"
        );
    }

    #[test]
    fn no_override_delegates_to_the_flag_matrix_unchanged() {
        let state = make_state(vec![
            league("Fate of the Vaal", "vaal", false, true),
            league("Standard", "standard", false, false),
            league("Hardcore", "hardcore", true, false),
        ]);
        assert_eq!(
            resolve_league_with_override(&state, false, false, None).unwrap(),
            resolve_league(&state, false, false).unwrap()
        );
        assert_eq!(
            resolve_league_with_override(&state, true, false, None).unwrap(),
            "Standard"
        );
    }
}
