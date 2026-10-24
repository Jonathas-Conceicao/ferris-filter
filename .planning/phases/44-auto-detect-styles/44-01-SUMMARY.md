---
phase: 44-auto-detect-styles
plan: 01
subsystem: ui
tags: [rust, overlay, styles, auto-detect]
requires:
  - phase: 43-style-alpha
    provides: style colours with alpha
provides:
  - StyleView::to_style (inverse of from_style, in-game values)
  - style_detect planner (plan, same_style, next_name, status_text, DetectPlan, NOTHING_NEW)
affects: [44-02, 44-03]
tech-stack:
  added: []
  patterns: [pure egui-free planner over (BlockId, Vec<BodyEntry>, BlockKeyword) inputs]
key-files:
  created: [src/overlay/style_detect.rs]
  modified: [src/overlay/style_lines.rs, src/overlay/mod.rs]
key-decisions:
  - "Exactness is style_leftovers empty and to_style Some; trailing comments never reach the view so need no handling"
  - "Match order: built-ins, customs, styles planned earlier in the run"
requirements-completed: [DETECT-01, DETECT-02, DETECT-03, DETECT-04, DETECT-07]
duration: 25min
completed: 2026-10-23
---

# Phase 44 Plan 01: Auto Detect planner Summary

Pure Auto Detect core: `StyleView::to_style` (3-value background = alpha 240, Minimal and quote/control-char sound names give None) and `style_detect::plan` matching rules to built-ins, customs, then newly planned `Custom Style N` styles, with `status_text`.

## Tasks
1. `StyleView::to_style` + 7 `to_style_*` tests incl. built-in round trip - 60849c0
2. `style_detect.rs` planner + table-driven tests (equality table, pairs, edited built-in, match order, naming, status text) - 0f5ee56

## Deviations from Plan
None in behavior. Notes:
- `cargo fmt` also reformatted `src/skin/editor.rs` and `src/skin/widgets.rs` (pre-existing drift); I reverted those, out of scope.
- No python on this machine; edits were done with the Edit tool.

## Verification
`cargo test --locked` (temp target dir): all green, 1720 lib tests plus integration tests.

## Self-Check: PASSED
Files exist, both commits present, no egui in style_detect.rs.
