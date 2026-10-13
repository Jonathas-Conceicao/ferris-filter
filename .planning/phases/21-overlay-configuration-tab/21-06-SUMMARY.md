---
phase: 21-overlay-configuration-tab
plan: 06
subsystem: overlay
tags: [egui, tabs, settings, config-tab, docs]
requires: ["21-02", "21-03", "21-05"]
provides:
  - "RULES | CONFIG tab strip, CONFIG body with pinned ff-config-footer, ConfigAction dispatch in OverlayApp"
  - "Filter selection remembered in ff-settings.json with a switch guard"
  - "apply::{config_newer_than_filter, tab_label, OTHER_TAB_DIRTY_TIP}, WorkingFilter::style_users"
  - "21-MANUAL-VERIFICATION.md UAT script"
affects: []
key-files:
  created: [.planning/phases/21-overlay-configuration-tab/21-MANUAL-VERIFICATION.md]
  modified: [src/overlay/app.rs, src/overlay/apply.rs, src/overlay/rules.rs, CHANGELOG.md, CLAUDE.md, .planning/REQUIREMENTS.md]
key-decisions:
  - "CONFIG Apply marks the filter out of date only when a file was actually written"
  - "RELOAD and filter switching share reset_filter_state; RELOAD keeps config edits unless the root changed"
  - "Filter list and config mtimes are read on activation/switch/reload only, never per frame"
requirements-completed: [CONF-01, CONF-02, CONF-03, CONF-04, STORE-02]
duration: ~45min
completed: 2026-10-13
---

# Phase 21 Plan 06: App wiring, docs and UAT Summary

The stripe now has RULES | CONFIG tabs; CONFIG edits styles, tiers and the active game-folder filter (remembered in `ff-settings.json`) with its own Apply/Revert, an out-of-date notice shared by both tabs, and a switch guard for unapplied rule edits.

## Accomplishments
- Tab strip with ` *` cross-tab markers; switching never applies or discards.
- CONFIG body: `ff-config-footer` bottom panel (outcome area `config-outcome` plus `show_footer`) claimed before `show_sections`, which owns its ScrollArea; `style_users()` rebuilt every frame for the ctx.
- Dispatch for Apply/Overwrite/Revert/Reload and style/tier edits; after Apply the RULES styles refresh synchronously; `config_changed_pending` clears on RULES Apply (Written and Unchanged) and is computed from mtimes at load.
- Filter selection: `filter_target` (re-validates names), `switch_decision`, `switch_to`, create-new (file written by first RULES Apply), pending-switch confirm.
- CHANGELOG, CLAUDE.md (layout row, CONFIG note, FERRISFILTER_HOME warning), REQUIREMENTS ("Complete (UAT pending)"), UAT script.

## Commits
- 1afe213: tab strip, config panel, dispatch, helpers
- aabbc78: ff-settings.json filter selection and switch guard
- da9cea9: docs and UAT script

## Deviations from Plan
- Task 1 already needed the settings/filters_cache/pending_switch/game_dir fields and `refresh_filters` (the CONFIG ctx and activation hook use them), so that scaffolding landed in the Task 1 commit; Task 2 added the behaviour.
- [Rule 1] CONFIG Apply sets `config_changed_pending` and appends the out-of-date sentence only when a file was written (a no-op Apply does not make the filter stale).
- `apply.rs`/`rules.rs` are CRLF in the working tree; edits preserved their endings.

## Deferred Issues
- Pre-existing clippy warnings in `app.rs`/other files (redundant guard, collapsible if) are untouched; `cargo clippy -- -D warnings` was already failing before this plan.

## Verification
`cargo test` (722 lib tests plus integration suites incl. artifact_guard/generate_snapshot) passes, `cargo fmt --check` clean, fixtures and fallback files untouched. Manual UAT pending (`21-MANUAL-VERIFICATION.md`).

## Self-Check: PASSED
