---
phase: 18-filter-as-store-economy-section
plan: 05
subsystem: cli
tags: [cli, apply, integration-tests, filter-as-store]
requires: [18-01, 18-04]
provides:
  - "Commands::Apply { filter, add_economy, remove_economy, tiers, styles }"
  - "cmd_apply over dynamic::{LoadedFilter, economy_body, apply_file} and paths::app_root"
affects: [18-06, 18-07]
key-files:
  modified: [src/cli.rs, src/run.rs, tests/dynamic_filter.rs, tests/cli_errors.rs]
key-decisions:
  - "Plain apply pre-flights FilterNotFound/NoEconomySection before loading any prices"
  - "PriceSource::Absent arm is a no-op in cmd_apply because economy_body already refuses it"
requirements-completed: []
duration: 25min
completed: 2026-10-12
---

# Phase 18 Plan 05: CLI filter-as-store apply Summary

`ff-cli apply` now edits a filter file in place: `--filter`, `--add-economy` (conflicts with `--remove-economy`), `--tiers`, `--styles`. `--store/--base/--dest/--reset` are gone (clap rejects them). Inputs resolve from `paths::app_root()` via `InputPaths::relative_to`; the work is done by `dynamic::apply_file`. Output covers: v0.4 conversion notice, "already up to date", bytes written, `.ffbak` path, add/refresh/remove line, and the in-game reload hint.

## Tasks
1. New `Commands::Apply` and `cmd_apply` (commit feat 18-05).
2. `tests/dynamic_filter.rs` rewritten (14 hermetic tests; the only spawn point sets `FERRISFILTER_HOME` and `USERPROFILE`), plus `apply_obsolete_flags_are_rejected` in `tests/cli_errors.rs`.

## Verification
Full `cargo test`: 533 lib tests plus all integration suites pass, 0 failures (dynamic_filter 14, cli_errors 12, cli_generate 16). `cargo build --bins` clean. Fixtures and snapshots unchanged.

## Requirements
STORE-01/03/04/05 and ECON-03/04 are observable through the binary here, but the overlay (18-06) and removal of the legacy JSON-store API (18-07) are outstanding, so none are marked complete in REQUIREMENTS.md.

## Deviations from Plan
None of substance. One test adjustment: the "garbage between markers" fixture in `refresh_changes_nothing_outside_the_markers` is a realistic `Show` block rather than a bare non-filter line, because a bare line directly after a block is treated by the parser as part of that block and the marker is then reported as inside a rule block (a hard stop, by design).

## Known Stubs
None.

## Self-Check: PASSED
