---
phase: 34-economy-about-tabs
plan: 02
subsystem: ui
tags: [egui, overlay, economy-tab, revert-scope]
requires: []
provides:
  - "config_state::RevertScope, styles_dirty, tiers_dirty, scope_dirty_count, revert_scope"
  - "WorkingFilter::economy_staged / unstage_economy"
  - "shell::relative_sync_text, thousands, SectionButton / section_button, FIX_FILTER_FIRST"
  - "shell::tab_markers(active, rules, styles, tiers); ConfigBarInputs styles_dirty/tiers_dirty/economy_staged"
  - "ConfigAction::RevertScoped, ConfigUi::revert_scope, app EconomyInfo cache"
affects: [34-03]
key-files:
  modified: [src/overlay/config_state.rs, src/overlay/rules.rs, src/overlay/shell.rs, src/overlay/config_view.rs, src/overlay/app.rs]
key-decisions:
  - "Tier edits belong to ECONOMY (marker, Revert, Apply gate); CONFIG Revert is styles-only"
  - "Economy Apply uses config.apply_enabled, same gate as CONFIG"
metrics:
  tasks: 2
  completed: 2026-10-16
---

# Phase 34 Plan 02: ECONOMY model and scoped Revert Summary

Tier cutoff edits now mark and revert on the ECONOMY tab, style edits on CONFIG, a staged economy add/remove is undone from ECONOMY alone, and the app caches prices.json info (mtime, priced items, live/fallback) for the view that Plan 34-03 draws.

## Commits
- 3143d72: feat(34-02): ECONOMY owns tier edits; scoped Revert; economy info cache (tasks 1 and 2)

## Deviations from Plan
- Tasks 1 and 2 were committed together: the `tab_markers` / `ConfigBarInputs` signature change in shell.rs and its app.rs call sites must land in one commit for every commit to build and pass `cargo test`.
- The new shell helpers (`relative_sync_text`, `thousands`, `SectionButton`, `section_button`, `FIX_FILTER_FIRST`) carry `#[allow(dead_code)]` until Plan 34-03 uses them, so the lib build adds no warnings.
- `EconomyInfo::read(root)` is a separate function so the prices.json read is unit-testable without an app.

## Verification
`cargo test` green (1318 lib tests plus integration), `cargo fmt --all --check` clean, clippy summary total 122 (lib test 119 + cli_generate 2 + sync_prices 1), `tests/fixtures` unchanged.

## Known Stubs
None. The ECONOMY tab still draws the old view (League / Last sync); Plan 34-03 replaces it.

## Self-Check: PASSED
