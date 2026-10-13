---
phase: 21-overlay-configuration-tab
plan: 04
subsystem: overlay
tags: [config-state, validation, apply, outside-edit-guard]
requires: ["21-01", "21-02"]
provides:
  - "overlay::config_state::{ConfigState, StyleEdit, ColourSlot, StyleBadges, TierRow, OutsideEdit, ConfigApplyError, ConfigApplyReport}"
  - "config_style_refs, delete_confirm_text, config_warning, dirty_status, applied_message, not_applied_message, outside_edit_message, OUT_OF_DATE_MESSAGE, APPLY_BLOCKED_TOOLTIP"
affects: [21-05, 21-06]
key-files:
  created: [src/overlay/config_state.rs]
  modified: [src/overlay/mod.rs]
key-decisions:
  - "A file whose working value equals its applied snapshot keeps its loaded bytes on Apply (no reformat); Overwrite rewrites both files from memory"
  - "ConfigApplyError::Write carries the finished sentence, so a RollbackFailed message never claims nothing changed"
  - "Tier errors come from ValidatedTiers::try_from only; parse failures and order failures are cached per row on every edit"
requirements-completed: [CONF-02, CONF-03, CONF-04]
duration: ~25min
completed: 2026-10-13
---

# Phase 21 Plan 04: ConfigState model Summary

A window-free `ConfigState` holds styles and tier cutoffs in memory, counts edits by value-compare, validates, and applies both files all-or-none with an outside-edit guard.

## Accomplishments
- Style add/delete/edit with built-in protection, trimmed unique names, font refused when it does not fit u8, range errors from `StyleSet::validate`.
- Tier text buffers separate from the model: `Enter a number` keeps the last good value; order errors `Must be higher than A (20).` with edge rows.
- Apply goes through `write_all_or_none` over exactly styles.json and tiers.json; bytes baseline guard (Apply), mtime check (tab activation), Reload, Overwrite, Revert.
- Pure copy helpers for delete confirm, config warning and status lines.

## Commits
- a62d670: model, style edits, tier buffers, validation, dirty count
- a14e4d9: Apply, outside-edit guard, Reload/Overwrite, message helpers

## Deviations from Plan
- Commit trailer names Claude Sonnet 5.5 (the model that ran this), per the session attribution instruction, not the Opus 5.5 line in the task prompt.
- `ConfigApplyError::Write(String)` holds the finished message rather than a bare reason (see decisions); `message()` returns it verbatim.
- Apply reuses loaded bytes for an unchanged file; the plan only said to render both. Behavior asserted by `apply_writes_only_the_two_files`.

Verification: lib tests 700 pass, `cargo fmt --check` clean, committed config/fallback/styles/tiers files unmodified.

## Self-Check: PASSED
