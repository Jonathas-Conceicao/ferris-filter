---
phase: 18-filter-as-store-economy-section
plan: 07
subsystem: cleanup
tags: [cleanup, removal, docs, changelog]
requires: [18-05, 18-06]
provides:
  - "src/dynamic.rs without the JSON-store API"
  - "tests/fixtures/v04-dynamic-base.json (former dynamic_base.json)"
  - "CLAUDE.md, CHANGELOG.md, .gitignore describing filter-as-store"
affects: [19]
key-files:
  modified: [src/dynamic.rs, src/loader.rs, src/models/config.rs, src/overlay/edit.rs, src/overlay/rule_editor.rs, src/overlay/view_model.rs, .gitignore, CLAUDE.md, CHANGELOG.md]
  renamed: [dynamic_base.json -> tests/fixtures/v04-dynamic-base.json]
key-decisions:
  - "FILTER_FILE, default_destination and default_filter_path kept; everything else of the JSON store removed"
requirements-completed: [STORE-01, STORE-03, STORE-04, STORE-05, ECON-01, ECON-03, ECON-04, EDIT-09, EDIT-10]
duration: 25min
completed: 2026-10-13
---

# Phase 18 Plan 07: Remove the JSON store; document filter-as-store Summary

The legacy JSON-store API (BASE_FILE, STORE_FILE, IDENTITY, StoreState, ensure_store, reset_store, copy_template, CompiledFilter, compile_store, write_filter, and the TemplateUnreadable / StoreUnwritable / DestinationMissing / FilterUnwritable variants plus their tests) is deleted from `src/dynamic.rs`. `dynamic_base.json` moved to `tests/fixtures/v04-dynamic-base.json`, and the four `include_str!` sites in the legacy overlay modules point at it. CLAUDE.md, CHANGELOG `[Unreleased]` (with a `**BREAKING:**` entry) and `.gitignore` describe the new design.

## Commits
- 439bcc9: refactor(18-07): remove the legacy JSON-store API; template becomes a test fixture
- 7130680: docs(18-07): document filter-as-store; record the breaking change

## Verification
- `cargo build --bins` clean. Full `cargo test`: 511 lib tests plus all integration suites pass, 0 failures. `tests/fixtures/seed-universal.filter` and `tests/snapshots` unchanged.
- Grep gates: no `compile_store|ensure_store|reset_store|write_filter(|StoreState` in src/tests; no `/dynamic_base.json` include path; 4 `v04-dynamic-base.json` references.
- Remaining `dynamic_base.json` / `dynamic.json` text hits are only doc comments in the untouched overlay modules (edit.rs, rule_editor.rs, view_model.rs), `.expect` messages, comments in `src/compile.rs` / `src/models/config.rs`, and test names, all describing the template's rule numbers.

## Deviations from Plan
- None functionally. A `loader.rs` doc comment referencing `overlay::store::save` was reworded (comment only, as planned).
- `.gitignore` no longer lists `/dynamic.json`, as the plan requires, so the user's real repo-root `dynamic.json` (left on disk untouched) now shows as untracked in `git status`. It must not be staged.

## Deferred / noted
- A pre-existing `unused import: super::*` test-build warning in `src/overlay/app.rs` (`mod tests`, from 18-06) was not touched; it does not affect `cargo build --bins`.

## Human verification (not run)
- Copy the game-folder filter to a scratch folder, run `ff-cli apply --filter <copy>` twice (first converts and keeps `.ffbak`, second is a no-op). Launch `ff.exe` and check status and SYNC ECONOMY target. Apply to the real file only with the game closed.

## Known Stubs
None.

## Self-Check: PASSED
