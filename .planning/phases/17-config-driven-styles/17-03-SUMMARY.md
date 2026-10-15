---
phase: 17-config-driven-styles
plan: 03
subsystem: styles
tags: [styles, loader, compile, migration]

requires: [17-01, 17-02]
provides:
  - styles.json is the only source of named styles for generate, apply and the overlay
  - InputPaths.styles + with_styles
  - loader::load_config rejecting a leftover inline styles section
  - StyleSet::check_rules (load-time rule-to-style reference check)
affects: [17-04]

tech-stack:
  added: []
  patterns: [serde(skip) runtime-attached field plus a raw legacy field used only to reject old files]

key-files:
  modified: [src/models/config.rs, src/loader.rs, src/compile.rs, src/overlay/store.rs, config.json, dynamic_base.json, tests/cli_errors.rs]

key-decisions:
  - "OD-1: inline `styles` in a config/store file fails at load naming the file (legacy_styles raw field, skip_serializing)"
  - "store::load does not run check_rules; Apply (compile_store -> load_inputs) enforces it so a bad rule never blocks overlay startup"

requirements-completed: [STYLE-01, STYLE-02, STYLE-04]

duration: 30min
completed: 2026-10-12
---

# Phase 17 Plan 03: styles.json becomes the single source Summary

**config.json and dynamic_base.json no longer carry styles; every load path reads styles.json, unknown rule styles and leftover inline sections fail at load, and generate output is byte-identical.**

## Accomplishments
- `Config.styles` is `#[serde(skip)]`; `legacy_styles` (renamed `styles`, never serialized) lets old files parse far enough to be rejected by `loader::load_config` with an error naming the file and pointing to styles.json / `--styles`.
- `compile::load_inputs` loads config, attaches `load_styles(paths.styles)`, then `check_rules` (enabled rules only; disabled rules are inert). Error names rule index/label, the style, and the defined styles.
- `InputPaths` gained `styles` (conventional `styles.json`) and `with_styles`.
- `overlay::store::load` reads styles.json from the store's directory via `InputPaths::relative_to`; `save` never writes styles.
- Removed the styles object (lines 4-18) from config.json and dynamic_base.json; all other top-level keys verified deep-equal to HEAD.
- `tests/cli_errors.rs`: the malformed-config test became `malformed_styles_json_reports_field_path_and_expected_type` (asserts `styles.json`, `S.font`, `expected u8`).
- New tests: path tests, load_inputs (attach, unknown style, disabled unknown style, missing styles.json, inline section), config tests, and four store tests.
- Full `cargo test` green; `tests/fixtures/seed-universal.filter` has no diff.

## Task Commits
1. Task 1: styles loading, check, JSON migration - 51748a7
2. Task 2: store styles tests - 83e0719

## Deviations from Plan
None of substance. Task 1's store::load change already satisfied the Task 2 tests, so Task 2 was tests only.

## Consequences for the user
- The user's git-ignored `dynamic.json` (inline styles section, including `craftable`) will fail to load until its `styles` section is deleted or `apply --reset` is run. If a rule uses `craftable`, add that style to styles.json first. I did not edit dynamic.json or any ssf_* config (those also need their `styles` section moved).

## Known Stubs
None.

## Self-Check: PASSED
