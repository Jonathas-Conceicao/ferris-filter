---
phase: 38-built-in-styles-strictness-reset
plan: 02
subsystem: skin-widgets
tags: [skin, levels-widget, tooltip]
requires: []
provides:
  - "skin::editor::{Level, ff_levels, levels_layout, LEVELS_HEIGHT}"
  - "one tooltip per minimap shape tile"
affects: [38-04]
key-files:
  modified:
    - src/skin/editor.rs
    - src/overlay/style_lines.rs
key-decisions:
  - "FIX-01 verified with a real headless hover test (tooltip text counted in frame output), not a source guard"
  - "levels_layout falls back to proportional shrink if even 8pt padding does not fit (widths never negative)"
requirements-completed: [STRICT-01, STRICT-03, FIX-01]
metrics:
  tasks: 2
  completed: 2026-10-18
---

# Phase 38 Plan 02: Levels widget and tooltip fix Summary

A seven-segment, one-line `ff_levels` switch (content-proportional segments, stacked two-word captions, display-only Custom segment) now exists in `skin::editor`, and each minimap shape tile shows a single tooltip.

## Tasks

| Task | Commit | Notes |
|------|--------|-------|
| 1 ff_levels + levels_layout | 5486859 | `ff_segmented` untouched; tests for layout at 517/660/460, caption fit at 517, 40pt single row, click contract |
| 2 FIX-01 | bd98161 | removed the second `on_hover_text` in the shapes loop of `style_lines.rs` |

## Verification

`cargo test --lib -- skin::editor overlay::style_lines`: 11 passed. The FIX-01 hover test counted the shape name twice before the fix and once after (RESEARCH A2 harness worked, so no source-guard fallback was needed).
`skin::editor::ff_icon_button` has one caller (the shapes loop). The rule-row chips and Config Icon column were not changed.

## Deviations from Plan

None. `cargo fmt` reformatted `tests/isolation_guard.rs` again; reverted as unrelated.

## Known Stubs

None (the widget is wired by plan 04).

## Self-Check: PASSED
