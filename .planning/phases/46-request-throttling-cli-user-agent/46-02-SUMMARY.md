---
phase: 46-request-throttling-cli-user-agent
plan: 02
subsystem: overlay
tags: [throttle, overlay, ff-state, docs]
requires: [46-01]
provides:
  - "src/overlay/app/throttle_glue.rs: start_sync_gated, start_leagues_gated, start_update_gated_with, record_after_polls"
  - "UpdateJob::take_done"
affects: []
tech-stack:
  added: []
  patterns: ["child module of app.rs holding gate/record logic", "fake refresh through the existing worker seams"]
key-files:
  created: [src/overlay/app/throttle_glue.rs]
  modified: [src/overlay/app.rs, src/overlay/update_job.rs, CLAUDE.md, .planning/phases/AUTONOMOUS-V072-DEFERRED.md]
key-decisions:
  - "A fake sync replays the league name the real sync returned (shared Arc slot), so the message is byte-identical"
  - "Starting a fake sync drops any pending real-sync record, so a stale one is never recorded under another league"
requirements-completed: [THR-01, THR-02, THR-03, THR-04, THR-05]
duration: ~40min
completed: 2026-10-23
---

# Phase 46 Plan 02: Overlay throttle wiring Summary

Sync economy (per league), the league list and Check for updates now go through `throttle_glue`: inside the hour of a real success a click replays the last outcome through the normal worker seam (no request, `prices.json` and Last sync untouched), only real successes are recorded and saved to `ff-state.json`, and the state survives a restart.

## Commits
- 905c859: glue module, `UpdateJob::take_done`, contained app.rs edits (fields, `mod`, three start sites, one per-frame `record_after_polls` call; about 27 changed lines)
- a1e337e: CLAUDE.md (`ff-state.json` row, folder list, hourly throttle and user agent notes) and the Phase 46 deferred-check entries

## Deviations from Plan
- Tasks 1 and 2 are one commit: both edit app.rs and the plan's task-1 state (new fields only) was not worth a separate intermediate commit.
- `start_sync_gated_with` takes `(league, now, real)` (no `root`), since the real closure captures it.

## Verification
`cargo test --locked` green (1725 lib tests, 10 new in `throttle_glue`, 1 in `update_job`); no fixture, Cargo.toml or Cargo.lock change. `cargo clippy --locked --all-targets -- -D warnings` fails with 127 pre-existing errors in untouched files on the current nightly; none point at Phase 46 files (logged in the deferred file).

## Known Stubs
None.

## Self-Check: PASSED
