---
phase: 36-per-user-locations-embedded-defaults
plan: 07
subsystem: infra
tags: [locations, test-isolation, docs]
requires: [36-01, 36-02, 36-03, 36-04, 36-05, 36-06]
provides:
  - "src/paths.rs (walk-up resolver) deleted; nothing references it"
  - "tests/isolation_guard.rs source-scan guard (5 tests)"
  - "CLAUDE.md describes the FerrisFilter folder rule and built-in defaults"
key-files:
  created:
    - tests/isolation_guard.rs
  modified:
    - src/lib.rs
    - src/compile.rs
    - tests/cli_generate.rs
    - CLAUDE.md
  deleted:
    - src/paths.rs
decisions:
  - "Guard checks the real resolver name (locations::from_env, a free function), not AppDirs::from_env as the plan wrote"
  - "CHANGELOG.md and README.md not edited (milestone close-out rule); README findings listed below"
metrics:
  tasks: 2
  completed: 2026-10-18
requirements: [LOC-01, LOC-02, LOC-03, LOC-04, LOC-05]
---

# Phase 36 Plan 07: Remove old resolver, isolation guard, docs Summary

The walk-up resolver is gone, a source-scan test now fails the suite on any unpinned apply/sync/ff spawn, and CLAUDE.md describes the per-user folder rule.

## Tasks

1. **Remove old resolver, add guard** (b8077a3): deleted `src/paths.rs`, `pub mod paths;`, the transitional `styles_source_path` / `tiers_source_path` and their `file_only` helper. `tests/isolation_guard.rs` has five tests: `spawns_that_resolve_the_folder_set_home`, `dirs_only_in_locations`, `no_env_mutation`, `from_env_call_sites`, `no_old_resolver`. Needles are built with `concat!` so the file does not match itself. Negative check done once: removing `.env("FERRISFILTER_HOME", dir)` from `tests/dynamic_filter.rs`'s `apply` helper made `spawns_that_resolve_the_folder_set_home` fail naming `tests/dynamic_filter.rs:73`; restored afterwards.
2. **Docs** (b4b260d): CLAUDE.md rows for the defaults pair, `styles.json`, `prices.json`, `ff-settings.json`, `ferrisfilter apply` (new FerrisFilter folder paragraph, test-isolation sentence), the status-line priority list, the Development warning and the economy-sync note. `grep "walking up from the exe" CLAUDE.md` is empty; `APPDATA` appears twice. CHANGELOG.md and README.md were skipped by instruction.

## Deviations from Plan

- **[Rule 1 - Bug] Unpinned spawn found by the guard:** `tests/cli_generate.rs` `sync_help_shows_league_flags` spawned `sync --help` without the env pins. Added `FERRISFILTER_HOME` and `USERPROFILE` (a temp dir) to it.
- **Resolver name:** the plan's `from_env_call_sites` check names `AppDirs::from_env`; the real function is `locations::from_env`, so the guard searches for that text (allowed in `src/locations.rs`, `src/run.rs`, `src/sync/mod.rs`, `src/overlay/app.rs`).
- **CHANGELOG/README edits skipped** per project rules; see the Release Review section.
- **Not done:** `cargo clippy --all-targets -- -D warnings` cannot pass (many pre-existing lib-test warnings, out of scope). Clippy reports no warning in the files this plan touched.

## Verification

Full `cargo test --no-fail-fast` with `FERRISFILTER_HOME` set to an empty temp dir, 0 failures: lib 1428 (down from 1439 because the deleted `paths.rs` carried its own unit tests), cli_errors 12, cli_filter_copy 4, cli_generate 20, dynamic_filter 23, filter_roundtrip 25, filter_structure 18, isolation_guard 5, overlay_rules 4, sync_prices 8, doctests 1. `cargo build --bins` succeeds. The byte-for-byte fixture test and `artifact_guard` (in cli_generate) pass.

Real-folder check: `%APPDATA%\FerrisFilter` did not exist before the run (`ls: cannot access ... No such file or directory`) and still did not exist after it, so the suite created nothing there.

## For the Release Review

README.md was not edited. Sentences that are now wrong:

1. README.md lines 66-69, "Your settings live in the FerrisFilter folder (the one holding `styles-default.json`): `styles.json`, `tiers.json`, `prices.json` and `ff-settings.json`. Delete `styles.json` or `tiers.json` to go back to the defaults."
   Now true: the folder is `%APPDATA%\FerrisFilter` (or the directory `FERRISFILTER_HOME` names). It is not located by `styles-default.json`, which is built into the exe and no longer a marker. The folder is created by the first Apply that writes styles/tiers, the first settings save or the first Sync; a fresh install has no files there and shows the built-in defaults. The last sentence is still true.

No other README line mentions the folder, the walk-up, `prices.json` location or `mapping.json`. CHANGELOG `[Unreleased]` still needs a `### Changed` entry: settings, styles, tiers and synced prices live in `%APPDATA%\FerrisFilter` (or `FERRISFILTER_HOME`), created on first save; shipped defaults, fallback prices, class catalog and mapping are built into the exe; `ff-cli sync` writes `prices.json` there; old files are not migrated.

## Known Stubs

None.

## Self-Check: PASSED
