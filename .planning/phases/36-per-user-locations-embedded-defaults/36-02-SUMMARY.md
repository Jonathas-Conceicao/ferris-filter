---
phase: 36-per-user-locations-embedded-defaults
plan: 02
subsystem: infra
tags: [rust, loaders, settings, embedded-defaults]
requires: []
provides:
  - "loader.rs: load_json_str, load_styles_str, load_tiers_str, load_prices_str, attach_styles, load_config_str_with_live"
  - "sync/mapping.rs: load_mapping_str"
  - "settings::save creates its folder on a real write"
affects: [36-04, 36-05, 36-06]
key-files:
  modified: [src/loader.rs, src/sync/mapping.rs, src/settings.rs]
key-decisions:
  - "Path loaders read the file then delegate to the text loaders, so messages are unchanged"
  - "Fold clicks may create the settings folder (via save's NotFound branch); load never creates"
requirements-completed: [LOC-01, LOC-02]
duration: ~15min
completed: 2026-10-18
---

# Phase 36 Plan 02: Text Loaders and Settings Folder Creation Summary

Every JSON loader now has an in-memory twin that takes a label for errors, so embedded defaults can be parsed with the same validation and miette shape; `settings::save` creates its folder.

## Tasks
1. `b26af9f` text loaders in `src/loader.rs` and `src/sync/mapping.rs`. One `serde_path_to_error::deserialize` body (`load_json_str`); `load_config_with_live` keeps its failure order (config parse, then styles, then `attach_styles`). 7 new tests (text vs file equality for styles, both tiers files, fallback prices, config; label in parse errors; mapping).
2. `0d6f3e3` `create_dir_all(root)` in `settings::save` before the temp file. 3 new tests (nested folders, fold on missing folder, load from missing folder creates nothing).

## Verification
`cargo test --lib loader::` 21, `sync::mapping::` 15, `settings::` 28, `--test cli_errors` 12: all pass with `FERRISFILTER_HOME` on a temp dir. `cargo fmt --check` clean on the touched files.

## Deviations from Plan
None in behaviour. The config equality test compares `serde_json::to_value` rather than Debug, because Config holds HashMaps whose Debug order is not stable.

## Decisions
- `save_section_state` needs no second `create_dir_all` (an existing file implies its folder); a comment says so.

## Known Stubs
None.

## Self-Check: PASSED
