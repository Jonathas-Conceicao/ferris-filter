---
phase: 21-overlay-configuration-tab
plan: 02
subsystem: persistence
tags: [safe-write, settings, rollback]
requires: []
provides:
  - "safe_write::{write_all_or_none, Target, AllOrNoneError (changed_on_disk_path)}, private persist_with_retry"
  - "settings::{SETTINGS_FILE, DEFAULT_FILTER, Settings, load, save, valid_filter_name, new_filter_name, list_filters, contains_name, NameProblem}"
affects: [21-04]
tech-stack:
  added: []
  patterns: ["guard all, stage all, preflight all, persist in order, roll back from in-memory bytes"]
key-files:
  created: [src/settings.rs]
  modified: [src/safe_write.rs, src/lib.rs, .gitignore]
key-decisions:
  - "No .ffbak for multi-file writes; in-memory loaded bytes are the rollback source"
  - "Rollback reuses the injected persist seam so a failing rollback is testable"
requirements-completed: [CONF-04, STORE-02]
duration: ~20min
completed: 2026-10-13
---

# Phase 21 Plan 02: All-or-none write and filter settings Summary

Two-file writes either replace every changed file or restore the earlier ones byte-for-byte, and the selected game-folder filter is remembered in a git-ignored, validated `ff-settings.json`.

## Accomplishments
- `write_all_or_none` guards, stages and preflights (read-only / locked) every target before any rename; a later persist failure restores replaced files (or deletes ones that did not exist) and reports `RolledBack` or `RollbackFailed` naming unrestored paths.
- The single-file retry loop was extracted to `persist_with_retry`, shared by `write_guarded`; existing tests unchanged.
- `settings` module: load never writes, corrupt or traversal-name files fall back to `dynamic.filter` with a warning, unknown keys survive saves; names are bare `*.filter` only (separators, device names, bad chars, length refused).

## Commits
- 1e34bb5: write_all_or_none
- ae21cba: settings module

## Deviations from Plan
None. Verification: lib tests 675 pass, `cargo fmt --check` clean, `git check-ignore ff-settings.json` confirms the ignore.

## Self-Check: PASSED
