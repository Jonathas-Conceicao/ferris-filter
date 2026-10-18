---
phase: 39-league-picker-price-source
plan: 03
subsystem: overlay
tags: [league, price-source, state-machine]
requires: [39-01]
provides:
  - overlay::leagues (ListState, LeagueList, ViewInputs, LeagueView, CaptionKind, view, effective_fallback, source_pending, copy consts)
affects: [39-04, 39-05]
key-files:
  created: [src/overlay/leagues.rs]
  modified: [src/overlay/mod.rs]
key-decisions:
  - "OD-1/OD-2/OD-4 implemented as specified; Fallback overrides everything last"
requirements-completed: [LEAGUE-01, LEAGUE-02, LEAGUE-03, LEAGUE-04]
completed: 2026-10-18
---

# Phase 39 Plan 03: League model Summary

Framework-free `overlay::leagues`: a worker-backed `LeagueList` state machine (NotStarted / Loading / Loaded / Failed, last good list kept during refresh), the pure `view()` decision table for the league UI, `pick_name`, `effective_fallback` and `source_pending`.

## Tasks

1. `LeagueList` worker and `ListState` (commit ff4be39, together with task 2: both live in one new file).
2. `view()`, copy consts, `source_pending` (commit ff4be39); `#![allow(dead_code)]` follow-up (commit 662fc4e).

## Verification

- `cargo test --lib -- overlay::leagues` (temp FERRISFILTER_HOME / FERRISFILTER_GAME_DIR): 24 passed, 0 failed. No network; the list comes from the frozen 2026-10-18 fixture.
- `cargo clippy --all-targets`: no output for `leagues.rs`.

## Deviations from Plan

- Both tasks in one commit (single new file written at once).
- Added a module-level `#![allow(dead_code)]` because nothing consumes the model until plans 04/05; remove it when the view lands.

## Decisions

- Failed with nothing saved and nothing in prices.json: dropdown disabled, refresh enabled, dropdown tip is the failure text.
- `fetching` only disables refresh in the Loaded state (other states are already Loading); `syncing` disables dropdown and refresh.

## For the Release Review

None (no user-visible change yet).

## Known Stubs

None; the model is unused until plan 04 draws it.

## Self-Check: PASSED

Commits ff4be39 and 662fc4e and `src/overlay/leagues.rs` exist.
