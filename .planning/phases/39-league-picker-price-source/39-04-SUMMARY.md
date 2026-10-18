---
phase: 39-league-picker-price-source
plan: 04
subsystem: overlay
tags: [league, price-source, economy-tab]
requires: [39-02, 39-03]
provides:
  - sync_economy_blocking(base_dir, league); OVERLAY_LEAGUE removed
  - App league worker wiring, set_league / set_price_source, league_view
  - Economy tab rows Price source / League / Last sync / Priced items, Fallback greying
affects: [39-05, 39-06]
key-files:
  modified: [src/sync/mod.rs, src/overlay/app.rs, src/overlay/shell.rs, src/overlay/leagues.rs]
key-decisions:
  - "Tiers editor locked under Fallback with an explicit hover area (R1 fallback path), Reset button gets the same tip"
requirements-completed: [LEAGUE-01, LEAGUE-02, LEAGUE-03, LEAGUE-04, LEAGUE-05]
completed: 2026-10-18
---

# Phase 39 Plan 04: Picker and price-source switch Summary

The Economy tab now has a Live | Fallback switch and a live league dropdown with a refresh button; Sync prices the chosen league and Fallback greys everything it makes irrelevant.

## Tasks

1. App wiring: league worker, settings setters, sync with the chosen league (commit db7fa93).
2. Economy tab rows and greying (commit a549548).

## Precondition checks

- `ff_icon_button` present at `src/skin/widgets.rs`; `pub refresh: [TextureHandle; 3]` in `src/skin/textures.rs`.
- Phase 38's `tiers_reset_enabled` / `reset_tiers` present in shell.rs; greyed under Fallback with `The fallback tiers are fixed.`.
- `adopt_settings` exists but also reloads the filter state; the new setters use a small `save_view_setting` (the `set_reload_after_apply` pattern) instead.

## Verification

- Full `cargo test` (temp FERRISFILTER_HOME / FERRISFILTER_GAME_DIR): 1588 lib tests plus integration tests pass.
- `cargo build --all-targets`: no warnings. Clippy shows no new findings in shell.rs, leagues.rs, sync/mod.rs (pre-existing app.rs findings untouched).
- `grep -rn OVERLAY_LEAGUE src` is empty.

## Deviations from Plan

- Removed leagues.rs's module-level `#![allow(dead_code)]`; four item-level allows remain for `ended_status`, `effective_fallback`, `source_pending` (plan 05) and `LeagueList::inject` (screenshot mode, plan 06).
- Tests: the Economy-tab fetch-once behaviour and the Sync-refused-under-Fallback handler are covered through `league_view` / the cfg(test) fetch twin and the disabled-button tests, not a full frame drive.
- `cargo fmt` also reformatted `src/sync/league.rs` and `tests/isolation_guard.rs` (pre-existing); those were reverted and not committed.
- Tiers tooltip uses an explicit `ui.interact` hover area over the disabled editor (the plan's R1 fallback), not verified visually (plan 06 screenshot).

## Decisions

- Clicking the already-selected default league does nothing (the computed default is never saved).
- The synthetic `(ended)` / `(no league)` entries cannot be picked (`pick_name` is None).

## For the Release Review

- Economy tab: the pinned league is gone; pick the league from the poe.ninja list (refresh button), and choose Live or Fallback prices. Fallback greys the league, Sync and the Tiers editor. `league` and `price_source` are saved in ff-settings.json.

## Known Stubs

None.

## Self-Check: PASSED

Commits db7fa93 and a549548 exist.
