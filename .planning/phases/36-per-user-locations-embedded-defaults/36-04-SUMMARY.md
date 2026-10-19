---
phase: 36-per-user-locations-embedded-defaults
plan: 04
subsystem: infra
tags: [rust, cli, sync, apply, embedded-defaults]
requires: [36-01, 36-02, 36-03]
provides:
  - "sync parses the built-in mapping.json; write_prices_atomic creates its folder"
  - "ff-cli sync writes prices.json into the FerrisFilter folder"
  - "ff-cli apply reads InputPaths::from_dirs; --tiers/--styles accept bare built-in names"
affects: [36-05, 36-06, 36-07]
key-files:
  modified: [src/sync/prices.rs, src/sync/mod.rs, src/sync/metadata.rs, src/run.rs, src/cli.rs, tests/dynamic_filter.rs]
key-decisions:
  - "Plan wrote AppDirs::from_env; the 36-01 API is the free function locations::from_env(), used instead"
  - "Metadata subcommand keeps the working-directory mapping.json; only Prices resolves the folder"
requirements-completed: [LOC-01, LOC-02, LOC-03, LOC-04]
completed: 2026-10-18
---

# Phase 36 Plan 04: CLI uses the FerrisFilter folder Summary

`ff-cli sync` and `ff-cli apply` now use the same folder as the overlay (FERRISFILTER_HOME, else %APPDATA%\FerrisFilter). Sync never reads mapping.json from disk; apply works against an empty folder with the built-in defaults and fallback prices.

## Tasks
1. `dfdea07` Sync: `run_sync_prices` loads `defaults::MAPPING` via `load_mapping_str`; `write_prices_atomic` does `create_dir_all` on the parent after the empty-items refusal (empty write creates no folder); `cmd_sync` passes the resolved folder for `Prices`; docs on `metadata`. Two new unit tests.
2. `14c1c64` Apply: `cmd_apply` uses `locations::from_env` + `InputPaths::from_dirs`, `with_tiers_or_builtin` / `with_styles_or_builtin`; fallback warning now names the built-in pair; Apply/Sync/Prices/Metadata help text updated; `--filter` default, `install`, `generate` untouched. Integration tests: `missing_prices_use_the_built_in_fallback` (replaces the refusal test), `workspace_defaults` no longer copies -default files, new empty-folder, built-in `--tiers`, missing-home and help tests (all spawns pin FERRISFILTER_HOME and USERPROFILE to temp dirs).

## Deviations from Plan
- **[Rule 3]** `AppDirs::from_env` does not exist; the resolver is `crate::locations::from_env()`. Used that.
- The mock-HTTP "sync into empty dir" test was not added: prices.rs has no mock `HttpClient` (the integration test file reserves one for the future). The mapping change is covered by the existing sync tests plus compile-time removal of the disk read.
- The missing `--tiers missing-tiers.json` case uses live seed prices, because with the fallback pair the override is never read (existing behaviour).

## Verification
`cargo test --no-fail-fast` with FERRISFILTER_HOME on a temp dir: lib 1430 passed / 3 failed (the known baseline overlay::app tests, unchanged); cli_errors 12, cli_filter_copy 4, cli_generate 20, dynamic_filter 23, filter_roundtrip 25, filter_structure 18, overlay_rules 4, sync_prices 8, others pass. `cargo clippy --lib --bins` shows no new warning in the added code (remaining are pre-existing); rustfmt clean on touched files.

## Known Stubs
None.

## Self-Check: PASSED
