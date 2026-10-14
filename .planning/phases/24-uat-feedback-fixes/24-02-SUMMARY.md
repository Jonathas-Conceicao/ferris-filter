---
phase: 24-uat-feedback-fixes
plan: 02
subsystem: overlay
tags: [config, styles, widgets]
requires: []
provides: [widgets::slide_switch, config_view::style_groups]
affects: [src/overlay/widgets.rs, src/overlay/config_view.rs]
key-files:
  modified: [src/overlay/widgets.rs, src/overlay/config_view.rs]
requirements-completed: [UAT-02, UAT-03, UAT-04]
completed: 2026-10-13
---

# Phase 24 Plan 02: Styles groups and Show/Hide slide switch Summary

CONFIG Styles now lists Built-in then Custom under headings with a divider, and each style has a Show/Hide slide switch.

## What changed
- `widgets::slide_switch`: 32x16 animated pill, same shape as `circular_toggle`; 2 headless tests.
- `config_view`: `style_groups` partitions by `badges(n).builtin`; `style_row` factored out; `built-in` and `Hide` badges removed (`new`/`edited` kept); `BUILTIN_TIP` replaced by `SWITCH_TIP`; the ACTION segmented control removed from `style_editor`. The switch emits `StyleEdit::Action`; the row's click rect stops short of the switch so a switch click never expands the row.
- Tests: `styles_split_builtin_first_then_custom`, `style_switch_click_emits_hide` (scans click positions headlessly; asserts one Hide edit and no expansion).

## UAT-04 note
`StyleAction` only has `Show` and `Hide`, and the CONFIG style editor already offered only those, so no `Minimal` exists in CONFIG and nothing was added to models/emitter. The per-rule control is plan 24-05.

## Deviations
None. Environment note: the shared `ff-target` dir made cargo skip rebuilds across worktrees (0 tests ran); `touch src/lib.rs` before testing forces a rebuild.

## Verification
Full `cargo test` passes (805 lib tests plus integration), `cargo fmt --check` clean. Commits: dea4990 (widget), config_view commit follows it.

## Self-Check: PASSED
