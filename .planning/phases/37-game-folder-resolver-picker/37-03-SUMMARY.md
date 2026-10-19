---
phase: 37-game-folder-resolver-picker
plan: 03
subsystem: cli-overlay
tags: [rust, game-folder, cli, overlay]
requires: [37-01]
provides:
  - dynamic::default_destination(saved) / default_filter_path(saved) backed by locations::game_dir
  - run::saved_game_folder_in(root)
  - overlay startup_game_dir_from / startup_game_dir
affects: [37-07]
key-files:
  modified: [src/install.rs, src/dynamic.rs, src/run.rs, src/cli.rs, src/overlay/app.rs, src/overlay/screenshot.rs, tests/dynamic_filter.rs]
decisions:
  - "app.rs reads the saved folder through a small saved_game_folder(root) helper shared by the pure and wrapper startup functions; startup_game_dir_from carries cfg_attr(not(test), allow(dead_code)) since only tests call it"
  - "Reload moves the game-folder re-resolution after the settings reload (inside the existing cfg(not(test)) block)"
metrics:
  tasks: 2
  completed: 2026-10-18
---

# Phase 37 Plan 03: CLI and overlay use the game folder resolver Summary

`ff-cli apply` (no --filter) and `ff-cli install` (no --dest) now resolve the game folder with the same resolver as the overlay (FERRISFILTER_GAME_DIR, then the saved `game_folder`, then Documents\My Games\Path of Exile 2); the overlay restores the saved folder at launch and re-resolves it on Reload. The OneDrive\Documentos path and USERPROFILE derivation are gone from CLI source and help.

## Commits

- 91b51dd: feat(37-03): CLI apply and install defaults use the game folder resolver
- ded2277: feat(37-03): overlay restores the saved game folder at startup and on Reload

## Verification

- `cargo test --lib -- run:: startup_game_dir filter_target`: 9 passed (3 saved_game_folder_in, 5 startup_game_dir_from, filter_target).
- `cargo test --test dynamic_filter --test cli_filter_copy --test isolation_guard`: all passed (24 / 4 / 5), including `default_filter_path_uses_game_dir_env` and `default_filter_path_refreshes_the_pinned_game_dir`.
- `cargo build --bin ff` ok; `ff-cli apply --help` has no "OneDrive".
- All runs used FERRISFILTER_HOME and FERRISFILTER_GAME_DIR under a temp dir. No full `cargo test --lib` (per instructions).

## Deviations from Plan

None in behaviour. The first commit (91b51dd) does not compile on its own because `src/overlay/app.rs` still called the old `default_destination()` until the second commit; the pair is consistent.

## Known Stubs

None.

## Threat Flags

None. T-37-07 (every dynamic_filter spawn pins FERRISFILTER_GAME_DIR: 6 spawn sites plus the new test), T-37-09, T-37-10 (from_env still only in run.rs / app.rs) and T-37-22 (tests use injected env and a panicking documents closure) are implemented as planned.

## Human verification

- Pick a folder (once 37-05/06 land), restart `ff`, confirm the Config tab still points at it.
- With FERRISFILTER_GAME_DIR unset and a saved game_folder, `ff-cli apply` (no --filter) refreshes the file in the saved folder.

## For the Release Review

- CLAUDE.md: the `ferrisfilter apply` and `ferrisfilter install` rows still describe the `%USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2` default; they should say the game folder is resolved as FERRISFILTER_GAME_DIR, else the saved `game_folder` (ff-settings.json), else Documents\My Games\Path of Exile 2. The overlay Filter-section/missing-folder text also changed to "Cannot find the Documents folder; pick the game folder on the Config tab."
- CHANGELOG (GAME-01/GAME-02): CLI defaults no longer use USERPROFILE\OneDrive\Documentos; they honour the saved folder and FERRISFILTER_GAME_DIR.

## Self-Check: PASSED

- 91b51dd and ded2277 exist; all modified files present.
