---
phase: 39-league-picker-price-source
plan: 01
subsystem: sync, settings
tags: [league, poe.ninja, settings, price-source]
requires: []
provides:
  - sync::league::list_leagues, default_league
  - sync::fetch_leagues_blocking
  - Settings.league, Settings.price_source, saved_league(), price_policy()
affects: [39-02, 39-03, 39-04]
key-files:
  created: [tests/fixtures/index-state-2026-10-18.json]
  modified: [src/sync/models.rs, src/sync/league.rs, src/sync/mod.rs, src/settings.rs]
key-decisions:
  - "Default league = first non-hardcore entry not standard/hardcore; indexed ignored (OD-4)"
  - "Unknown price_source string reads Live without a warning; wrong JSON type warns via salvage (OD-5)"
requirements-completed: [LEAGUE-01, LEAGUE-04]
duration: short
completed: 2026-10-18
---

# Phase 39 Plan 01: League list and price-source data layer Summary

Network-free data layer: poe.ninja league list and default pick, a blocking fetch for the overlay worker, and `league` / `price_source` keys in `ff-settings.json`.

## Tasks

1. Fixture, `list_leagues`, `default_league`, `fetch_leagues_blocking` (commit 97b8894). `IndexState` / `EconomyLeague` now derive `Debug, Clone, PartialEq`.
2. `league` and `price_source` settings keys with accessors and salvage arms (commit 2d5b126).

## Verification

- `cargo test --lib -- sync::league settings::`: 63 passed, 0 failed.
- `cargo build --all-targets`: clean (every `Settings` literal compiles).
- Old `tests/fixtures/index-state.json` untouched; `resolve_league*`, `OVERLAY_LEAGUE` and `sync_economy_blocking` unchanged.
- Clippy not run to completion for new code beyond the build (repo already has ~125 pre-existing clippy errors); no new patterns of concern were introduced.

## Deviations from Plan

None. The plan's literal-update note was moot: all existing literals use `..Default::default()` or the salvage literal I extended.

## For the Release Review

- New `ff-settings.json` keys `league` and `price_source` (`"live"` / `"fallback"`), overlay-only. Not user-visible until plans 03/04 wire the UI.

## Known Stubs

None. `fetch_leagues_blocking` has no caller yet by design (plan 03).

## Self-Check: PASSED

Fixture, commits 97b8894 and 2d5b126 exist.
