---
phase: 24-uat-feedback-fixes
plan: 03
subsystem: overlay
tags: [uat, reload-message, rule-list, virtualization]
requires: []
provides:
  - apply::written_message (silent game-not-running status)
  - RuleRow.summary and two-line collapsed rows
affects: [src/overlay/apply.rs, src/overlay/app.rs, src/overlay/rules.rs, src/overlay/rule_list.rs]
key-files:
  modified:
    - src/overlay/apply.rs
    - src/overlay/app.rs
    - src/overlay/rules.rs
    - src/overlay/rule_list.rs
decisions:
  - Summary built once in WorkingFilter row rebuild (joined conditions, 160-char cap); draw_row only paints it
  - Collapsed row height = header_height + SUMMARY_LINE_H (16); expanded height = header + body
metrics:
  completed: 2026-10-13
requirements: [UAT-01, UAT-13]
---

# Phase 24 Plan 03: Silent game-closed Apply and collapsed-row summary Summary

Game-not-running Apply now shows only `Wrote <path>.`, and collapsed rules show a one-line condition summary with exact row virtualization.

## Tasks

1. No reload message when the game is not running (commit 666fcbf): `apply::written_message`, `GameNotRunning` arm in `skipped_reload_message`; other skip reasons unchanged. Tests: `written_message_has_no_reload_text`, `game_not_running_shows_no_reload_message`; the reminder test loop now covers SettingOff, Busy, NoWindow.
2. Collapsed-row condition summary (see `git log` for the second commit): `RuleRow.summary`, `SUMMARY_MAX_CHARS`, `row_summary`; `header_height`/`row_height`/`SUMMARY_LINE_H` in rule_list.rs; summary line hidden when expanded; `expanded_h` is now header + body. Tests: `row_summary_joins_conditions`, `collapsed_row_height_includes_summary_line`, `expanded_row_height_excludes_summary`; the 511-rule virtualization test passes unchanged.

## Deviations from Plan

None. Note: the shared `CARGO_TARGET_DIR` is used by parallel executors, and one full run briefly picked up another worktree's test (`style_switch_click_emits_hide`, not in this tree) which failed. A rerun was fully green (805 lib tests plus integration tests); `cargo fmt --check` clean.

## Known Stubs

None.

## Self-Check: PASSED
