---
phase: 31-rules-tab-on-the-skin
plan: 02
subsystem: ui
tags: [egui, skin, widgets, icons]
requires: []
provides:
  - skin::remember / skin::of (Skin reachable from any egui context)
  - ff_scroll_area_viewport, ff_field_row, ff_text_field_id/_ex, ff_x_button, ff_dropdown_ex, ff_chip
  - flag icons at 16pt glyph, 1.75 stroke
affects: [31-01, 31-03, 31-04, 31-05, 31-06]
tech-stack:
  added: []
  patterns: [skin stored in ctx data, shared private scrollbar painter]
key-files:
  created: []
  modified: [src/skin/mod.rs, src/skin/icons.rs, src/skin/widgets.rs, src/skin/editor.rs]
key-decisions:
  - "ff_field_row lays the control out in a 34pt-high wrapping child and grows only when the control is taller"
requirements-completed: [RULES-01, RULES-02]
duration: 25min
completed: 2026-10-16
---

# Phase 31 Plan 02: Skin pieces for the RULES tab Summary

Additive skin widgets (viewport game-scrollbar area, variable-height label/control row, chip, X button, id-aware text field, coloured dropdown) plus `skin::of(ctx)` and the flag icon fixes; nothing in the overlay uses them yet.

## Tasks

1. Skin reachable from the context; icon fixes: commit 6cec7c3
2. + 3. Viewport scroll area, field row, small controls: commit 76764e5 (tasks 2 and 3 share `widgets.rs`, so they were committed together)

## Deviations from Plan

- `ff_field_row` uses a finite 34pt-high child (an unbounded height made Align::Center produce an infinite rect); row height is `ROW_HEIGHT`, or `used + 4` when the control is taller.
- Tasks 2 and 3 in one commit (see above).

## Verification

Full `cargo test` green (1153 lib tests), `cargo build --bin ff-proto` ok, fmt clean, clippy warnings 126 (limit 126), fixtures untouched.

## Known Stubs

None.

## Self-Check: PASSED
