---
phase: 36-per-user-locations-embedded-defaults
plan: 06
subsystem: overlay
tags: [locations, embedded-defaults, first-run, test-isolation]
requires: [36-01, 36-02, 36-03, 36-05]
provides:
  - "Overlay resolves its folder via locations::from_env (FERRISFILTER_HOME, else per-user folder), no walk-up"
  - "Class catalog always parsed from the built-in config.json"
  - "StylesStamp enum so the styles watcher follows an embedded source"
  - "Unit-test apps built on a per-test temp home"
key-files:
  modified:
    - src/overlay/app.rs
    - src/overlay/screenshot.rs
decisions:
  - "resolve_root returns Err under cfg(test); reload keeps an Ok root and re-resolves only from Err"
  - "reload re-resolves game_dir only outside cfg(test)"
  - "Test app holds its TempDir in a cfg(test) test_home field"
metrics:
  tasks: 2
  completed: 2026-10-18
requirements: [LOC-01, LOC-02, LOC-03, LOC-05]
---

# Phase 36 Plan 06: Overlay on AppDirs and Embedded Defaults Summary

The overlay now takes its folder from the new resolver and shows built-in styles, tiers, class catalog and fallback prices from an empty folder, writing nothing at launch.

## Tasks

1. **Root resolution and test-safe construction** (496e5c9): `build` takes `root` and `game_dir`; `new` / `new_screenshot` pass `resolve_root()` and `dynamic::default_destination()`. `popup_app` uses a per-test temp home and a non-existent game dir. `reload` keeps an Ok root. Screenshot guard reads `locations::HOME_ENV` (guard order in `mod.rs` unchanged).
2. **Embedded class catalog, Source-aware styles, stamp, in_home preview** (bb3803f): `load_classes_at` parses `defaults::CONFIG` (a `config.json` in the folder is never read); `load_styles_at` handles `Source::File` / `Source::Embedded`; `StylesStamp { Embedded, File(path, mtime) }`; economy preview uses `InputPaths::in_home`. Tests: `first_run_overlay_inputs`, stamp create/delete switching, broken live file names its full path, folder `config.json` ignored, removing `styles-default.json` from the root changes nothing.

## Deviations from Plan

- **[Rule 1 - Bug]** Task 1's commit called `AppDirs::from_env`, which does not exist (the resolver is the free function `locations::from_env()`). It only compiled under `cfg(test)` (where `resolve_root` is a stub), so the non-test build was broken at 496e5c9 and fixed in bb3803f.
- The `no_styles_file_at_all_names_both` test was replaced by `removing_the_default_files_from_the_root_changes_nothing`, as the plan directed.
- `apply_working_adds_the_economy_section_from_the_repo_inputs` still uses `InputPaths::relative_to` on the repo root, read-only, in test code only.

## Verification

Full `cargo test --no-fail-fast` with `FERRISFILTER_HOME` set to a temp dir: lib 1439 passed, 0 failed. Integration: cli_errors 12, cli_filter_copy 4, cli_generate 20, dynamic_filter 23, filter_roundtrip 25, filter_structure 18, overlay_rules 4, sync_prices 8, doctests 1; all passed. `cargo build --bins` succeeds; `cargo clippy --all-targets` reports no warnings in `src/overlay/app.rs`. Screenshot mode was not run.

## Known Stubs

None.

## Self-Check: PASSED

Commits 496e5c9 and bb3803f exist; both modified files exist.
