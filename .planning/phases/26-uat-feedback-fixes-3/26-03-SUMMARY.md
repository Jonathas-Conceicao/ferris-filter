---
phase: 26-uat-feedback-fixes-3
plan: 03
subsystem: overlay
tags: [overlay, tray, quit, row-action]
requires: []
provides:
  - apply::quit_step pure decision (UAT3-08)
  - RowAction::InsertLineAfter (for Between, UAT3-06)
affects: [26-08]
key-files:
  modified:
    - src/overlay/apply.rs
    - src/overlay/app.rs
    - src/overlay/rules.rs
decisions:
  - Apply & quit defers the shutdown until any in-game reload worker has finished
metrics:
  tasks: 3
  completed: 2026-10-14
---

# Phase 26 Plan 03: Quit confirm and InsertLineAfter Summary

Tray Quit now asks (Apply & quit / Quit without applying / Cancel) when anything is unapplied, decided by a pure `quit_step`, and rules gained a validated `RowAction::InsertLineAfter`.

## What changed

- `apply.rs`: `QuitChoice`, `QuitEvent`, `QuitStep`, `quit_step`, label constants; table test covering all eight behavior rows.
- `app.rs`: `quit_confirm` / `quit_after_apply` fields, `quit_dirty`, `begin_shutdown` (Phase 22 order moved, not duplicated: `release_toggle` count 2 = definition use + doc mention as before, one `ViewportCommand::Close` send site), `egui::Modal` confirm (Escape/click outside = Cancel), `run_apply` now returns `bool` (true only on Applied). A second tray Quit with the confirm open keeps it open.
- Staged economy ops are already inside `WorkingFilter::is_dirty` (`refresh_projection` ORs `pending.is_some()`), so no extra term was needed in the quit dirty check.
- `rules.rs`: `InsertLineAfter { id, after, expect, text }` with anchor `expect` check, action-in-conditions refusal; inserts directly after the anchor. `app.rs apply_row_actions` classifies it as edited block, focus line, and ordinal shift.

## Deviations from Plan

- Worktree HEAD was not at the expected base; reset to 28a7677 per the startup check.
- Tasks 2 and 3 both edit `app.rs`, so they share one commit (efad0b4 holds Task 1; the second commit holds Tasks 2+3).

## Verification

Full `cargo test` passes (871 lib tests plus integration), `cargo fmt --check` clean. The modal and shutdown path are not exercised by an automated GUI test (no real tray or window in tests); covered by the pure `quit_step` test and should be confirmed in manual verification.

## Self-Check: PASSED
