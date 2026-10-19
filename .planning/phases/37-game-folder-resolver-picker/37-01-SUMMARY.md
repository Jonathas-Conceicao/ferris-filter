---
phase: 37-game-folder-resolver-picker
plan: 01
subsystem: config
tags: [rust, settings, locations, game-folder]
requires: []
provides:
  - locations::{GAME_DIR_ENV, GAME_SUBDIRS, game_dir_from, game_dir, default_game_dir, game_dir_is_pinned}
  - Settings.game_folder, Settings::game_folder_path, settings::choose_filter
affects: [37-02, 37-03]
key-files:
  modified: [src/locations.rs, src/settings.rs]
decisions:
  - "Load-time warning keeps the old wording when only `filter` is bad; game_folder-only and both-bad get their own wording naming `game_folder` and the default game folder"
  - "default_game_dir panics under cfg(test) unconditionally (tests never need the real default)"
metrics:
  tasks: 2
  files: 2
  completed: 2026-10-18
---

# Phase 37 Plan 01: Game folder resolver and settings key Summary

Pure game-folder resolver (FERRISFILTER_GAME_DIR, then a saved absolute folder, then Documents\My Games\Path of Exile 2 via `dirs::document_dir`) plus the `game_folder` settings key and the `choose_filter` selection rule. Library only; no callers yet.

## Commits

- d711070: feat(37-01): pure game-folder resolver in locations
- b0423c4: feat(37-01): game_folder setting and filter selection rule

## What was built

- `locations.rs`: `game_dir_from` (pure, lazy Documents closure, env returned verbatim with no existence check, relative saved paths ignored), `game_dir` (cfg(test) panic unless the env var is set), `default_game_dir`, `game_dir_is_pinned`. 7 new tests.
- `settings.rs`: `game_folder: Option<String>` (skipped when None, so existing files re-serialise byte-identically), `game_folder_path()` (non-empty and absolute only), per-key salvage for a wrongly typed value, load-time reset of empty/relative values with a warning, `choose_filter`. 8 new tests (round trip, valid, empty and relative, both bad, wrong type, unusable file, unknown keys kept, choose_filter).

## Verification

- `cargo test --lib locations::` 15 passed; `cargo test --lib settings::` 36 passed; `cargo test --test isolation_guard` passed; `cargo build --bins` ok; rustfmt applied to both files.
- All runs used FERRISFILTER_HOME and FERRISFILTER_GAME_DIR under a fresh temp dir.
- A full `cargo test --lib` run did not finish within the 10 minute foreground limit in this environment (see Deferred Issues); it was not needed for the targeted verification.

## Deviations from Plan

None - plan executed as written. (`cargo fmt --check` was satisfied by running rustfmt on the two files only.)

## Deferred Issues

- Full-suite `cargo test --lib` run was slow/hung under the shared worktree (several cargo processes were active concurrently); not investigated, unrelated to these two files.

## Known Stubs

None.

## Threat Flags

None. T-37-01 and T-37-02 mitigations are implemented as planned.

## Human verification

None for this plan (library only).

## For the Release Review

- CLAUDE.md `ff-settings.json` row should mention the new `game_folder` key (absolute path, written only when the user picks a folder) and `FERRISFILTER_GAME_DIR` once the later plans land.
- CHANGELOG entry (for GAME-01/GAME-02) belongs with the phase as a whole; nothing user-visible changes in this plan alone.

## Self-Check: PASSED

- src/locations.rs, src/settings.rs modified; commits d711070 and b0423c4 exist.
