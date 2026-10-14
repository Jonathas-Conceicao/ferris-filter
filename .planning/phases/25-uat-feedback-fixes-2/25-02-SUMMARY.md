---
phase: 25-uat-feedback-fixes-2
plan: 02
subsystem: overlay
tags: [apply, safe_write, config_state, styles]
requires: []
provides:
  - overlay::apply_all prepare/commit (combined Apply back end)
  - ConfigState prepare_write/adopt/move_style, StyleSet::move_custom
  - dynamic::prepare_edited, economy_body_with, LoadedFilter::adopt_written
  - safe_write::ensure_backup, three-target rollback test
affects: [25-03]
key-files:
  created: [src/overlay/apply_all.rs]
  modified: [src/safe_write.rs, src/compile.rs, src/dynamic.rs, src/models/config.rs, src/overlay/config_state.rs, src/overlay/mod.rs]
requirements-completed: [UAT2-09, UAT2-10]
completed: 2026-10-14
---

# Phase 25 Plan 02: Combined Apply back end Summary

GUI-free validate-everything-then-write-all-or-none Apply (styles.json, tiers.json, filter) with the economy priced from pending tiers/styles, plus the model half of custom-style reordering.

## Tasks

1. b2fd407 - primitives: `ensure_backup`, `pub(crate) write_all_or_none_with`, count-neutral RolledBack wording, `compile::load_economy_with`, `dynamic::{prepare_edited, adopt_written, economy_body_with}`; `apply_edited` re-implemented on `prepare_edited`.
2. 7556ef1 - `StyleSet::move_custom`; `ConfigState` `move_style`, `is_dirty`, `validate`, `prepare_write`, `adopt`, `pending_styles/tiers`, `baselines`, paths; order-aware dirty (a pure reorder counts as a CONFIG edit). `apply()`/`overwrite()` re-implemented on the new pieces.
3. (apply_all commit) - `prepare` (validate all, disk guard of all three files, no writes) and `commit`/`commit_with` (one-time `.ffbak` first, then three-target all-or-none, baselines adopted only on success).

## Verification

Full `cargo test` passes (850 lib tests plus integration, artifact_guard and generate_snapshot unchanged); `cargo fmt --check` clean. New tests cover (a)-(e), first-write `.ffbak`, absent-empty filter, three-target rollback, ensure_backup, prepare_edited parity, economy_body_with, reorder dirty/revert/persist and `move_custom`.

## Deviations from Plan

None - plan executed as written. Note: `overwrite()` in `ConfigState` still force-renders both files (legacy behaviour kept for its existing tests); `apply_all` overwrite uses `prepare_write`, which keeps the loaded bytes of an unedited file and rewrites them over the outside change.

## Known Stubs

None.

## Self-Check: PASSED
