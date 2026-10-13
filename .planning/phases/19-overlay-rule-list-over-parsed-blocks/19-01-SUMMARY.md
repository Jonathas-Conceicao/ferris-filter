---
phase: 19-overlay-rule-list-over-parsed-blocks
plan: 01
subsystem: overlay
tags: [rustfmt, cleanup, overlay]
requires: []
provides:
  - rustfmt-clean crate
  - overlay::text_entry::parse_base_type_entry
affects: [overlay]
tech-stack:
  added: []
  patterns: []
key-files:
  created: [src/overlay/text_entry.rs]
  modified: [src/overlay/mod.rs, src/overlay/app.rs, src/overlay/economy.rs, src/filter/block.rs]
  deleted: [src/overlay/edit.rs, src/overlay/rule_editor.rs, src/overlay/view_model.rs]
decisions:
  - "OD-5: v0.4 JSON-rule overlay modules deleted; parse_base_type_entry preserved with its 4 tests"
  - "OD-6: one standalone cargo fmt commit"
metrics:
  completed: 2026-10-13
requirements-completed: [EDIT-05]
---

# Phase 19 Plan 01: Housekeeping Summary

Standalone `cargo fmt` over the crate (45 .rs files), then removal of the v0.4 `overlay::edit`, `rule_editor` and `view_model` modules, with `parse_base_type_entry` and its four tests moved to `overlay::text_entry` for Phase 20.

## Commits
- 4f2c049: style(19-01): cargo fmt the whole crate
- 911666f: refactor(19-01): drop the v0.4 JSON-rule overlay modules; keep parse_base_type_entry

## Verification
- `cargo fmt --check` passes; full `cargo test` green (424 lib tests plus all integration suites) after both commits.
- tests/fixtures and tests/snapshots unchanged.
- No `dynamic_base`, `overlay::edit` or `RuleId` text remains in src or tests.

## Deviations from Plan
None. `tests/fixtures/v04-dynamic-base.json` is still used by `src/models/config.rs`, so it stays.

## Self-Check: PASSED
