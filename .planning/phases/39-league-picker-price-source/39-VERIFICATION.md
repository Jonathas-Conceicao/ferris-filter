---
phase: 39-league-picker-price-source
verified: 2026-10-18T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Open Economy tab with the real poe.ninja reachable"
    expected: "League dropdown lists poe.ninja's current leagues; pick one, relaunch, choice restored; Sync economy outcome names the league"
    why_human: "Real network fetch; tests use no network"
  - test: "Switch Price source to Fallback and hover League, Sync, Tiers editor and Reset to defaults"
    expected: "League and Sync greyed with the recommendation tooltip; Tiers greyed with 'The fallback tiers are fixed.'; Reset dimmed"
    why_human: "Visual dimming and hover tooltips"
  - test: "Saved league absent from the list"
    expected: "Status line shows 'X is no longer on poe.ninja; pick a league' (truncation/hover acceptable), nothing switched"
    why_human: "Visual status line rendering"
---

# Phase 39: League picker and price source Verification Report

**Phase Goal:** Users choose their league and whether prices are live or fallback, without any code change per league.
**Status:** human_needed (all automated checks pass; visual/network items deferred)

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Economy tab lists current leagues; choice restored next launch | VERIFIED | `sync::fetch_leagues_blocking` (src/sync/mod.rs:63) feeds `overlay::leagues::LeagueList`; `league`/`price_source` persisted in `settings.rs` (round-trip tests pass); `set_league_saves_at_once_and_leaves_the_rest` passes |
| 2 | Sync prices against chosen league; outcome names it | VERIFIED | `sync_economy_blocking(base_dir, league)` passes `Some(league)` to `run_sync_prices`; outcome "Wrote N items to prices.json (league: {}, ...)" (prices.rs:637); app.rs:3705-3716 uses `league_view().sync_league`; `OVERLAY_LEAGUE` no longer exists in src |
| 3 | Saved league no longer listed gives explicit message, no silent switch | VERIFIED | leagues.rs caption/status "X is no longer on poe.ninja; pick a league"; `StatusInputs.ended_league` wired (app.rs:3649, shell.rs:192); test `the_ended_league_line_shows_only_with_a_loaded_list_under_live` passes |
| 4 | Fallback greys Sync + league picker (recommendation tooltip); next Apply prices from fallback pair | VERIFIED | shell.rs price_source_tip; `sync_enabled` false under fallback (app.rs:3556) and click guarded (3706); `apply_everything(..., price_policy(), ...)` with `ForceFallback`; apply_all test asserts forced fallback pair used; `source_pending` marks unapplied |
| 5 | Under Fallback Tiers editor greyed with "fallback tiers are fixed" tooltip | VERIFIED | shell.rs:807-818 `add_enabled_ui(!fallback)` plus hover area with FALLBACK_TIERS_TIP; bad tier no longer gates Apply (test passes) |

**Score:** 5/5

## Requirements Coverage

| Requirement | Plans | Status |
|-------------|-------|--------|
| LEAGUE-01 | 39-01,03,04,06,07 | SATISFIED |
| LEAGUE-02 | 39-03,04,06,07 | SATISFIED |
| LEAGUE-03 | 39-03,05,06,07 | SATISFIED |
| LEAGUE-04 | 39-01..07 | SATISFIED |
| LEAGUE-05 | 39-02,04,05,06,07 | SATISFIED |

All five IDs appear in plan frontmatter and REQUIREMENTS.md; no orphans. (REQUIREMENTS.md line 83 has a stray line break in the LEAGUE-05 ID, cosmetic.)

## Behavioral Spot-Checks

Targeted `cargo test --lib -- league fallback price_source ended` under temp FERRISFILTER_HOME/GAME_DIR: 73 passed, 0 failed. Orchestrator full run: 1598 lib passed, 0 failed.

## Anti-Patterns

No TODO/FIXME/XXX/TBD in leagues.rs or sync/mod.rs. No stubs found.

## Gaps

None. Remaining items are visual/network checks listed in frontmatter.

_Verifier: Claude (gsd-verifier)_
