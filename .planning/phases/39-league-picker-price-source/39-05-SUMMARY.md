---
phase: 39-league-picker-price-source
plan: 05
subsystem: overlay
tags: [price-source, unapplied-marker, status-line, ended-league]
requires: [39-04]
provides:
  - App.applied_policy, App::source_pending, App::has_economy_section
  - StatusInputs.ended_league and its status_for branch
affects: [39-06]
key-files:
  modified: [src/overlay/app.rs, src/overlay/shell.rs, src/overlay/leagues.rs]
key-decisions:
  - "OD-1 adopted: marker = leagues::source_pending(applied_policy, now, economy_info.live, has_section), OR-ed into every style_pending slot and the Economy tab asterisk; never into rules_revertible / rules_revert_live / economy_revert_pressed"
  - "A2 accepted: applied_policy starts as the saved policy; moves on build, every successful Apply/Overwrite (run_apply) and adopt_settings (filter/folder switch); Reload and Revert do not move it"
requirements-completed: [LEAGUE-03, LEAGUE-04, LEAGUE-05]
completed: 2026-10-18
---

# Phase 39 Plan 05: Source marker, tier gate, ended league Summary

A price-source switch now counts as an unapplied change exactly when Apply would write different economy blocks, Fallback no longer lets greyed tier cutoffs block Apply, and an ended league is reported on the status line of every tab.

## Tasks

Both tasks are in one commit, 4edd500 (the `StatusInputs` literal in `app.rs` ties task 2 to task 1's edits in the same file, and the file cannot be split without interactive staging).

1. applied_policy / source_pending / marker / re-priced preview / per-frame `sync_tiers_in_use`.
2. `ended_league` status branch (after sync running, before unknown styles; click opens the Economy tab through the existing generic `open_tab` handling).

## Verification

- `cargo test --lib -- overlay::` : 857 passed, 0 failed. `cargo build --all-targets` : no warnings. `cargo fmt` touched `src/sync/league.rs` and `tests/isolation_guard.rs` (pre-existing); reverted, not committed.
- Removed the item-level `#[allow(dead_code)]` on `ended_status`, `effective_fallback` and `source_pending`; `LeagueList::inject` keeps its allow (screenshot mode, plan 06).

## Deviations from Plan

- [Rule 1] The Rules Revert, Config Revert / RevertScoped and the changed-file Reload also add `source_pending()` to the "other dirt" they pass to `state_after_revert`. Without it the indicator showed Applied for a frame after a Revert while a switch was pending (caught by the new revert test). They still do not undo the switch.
- One commit instead of two (see Tasks).

## Decisions

- `has_economy_section` uses the projected working view (`EconomyView::Present`), so a staged add also counts; harmless because a staged add is already unapplied.

## For the Release Review

- Economy tab: switching Live/Fallback with an economy section in the filter and usable prices.json now shows `Economy *` and `Unapplied changes`; switching back clears it. Revert does not undo the switch.
- Status line: `<league> is no longer on poe.ninja; pick a league` when the saved league is missing from the loaded list under Live.

## Self-Check: PASSED

Commit 4edd500 exists; changed files present.
