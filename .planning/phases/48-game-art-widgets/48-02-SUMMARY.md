---
phase: 48-game-art-widgets
plan: 02
subsystem: overlay
tags: [art, egui, rules-tab, section-header]
requires: [48-01]
provides:
  - header_layout pure geometry for section headers
  - section headers (user and economy) painted with toggle plate and stone bar
affects: [49]
key-files:
  modified:
    - src/overlay/rule_list.rs
key-decisions:
  - "Dropped the decorative rule line between title and count; the bar art carries the frame (research Open Question 1)"
metrics:
  tasks: 2
  completed: 2026-10-23
requirements: [ART-01]
---

# Phase 48 Plan 02: Section header game art Summary

RULES section headers now draw the game's toggle plate (chevron up over dot when expanded, dot over chevron down when collapsed, hover and pressed art) and the three-slice stone bar, with the title at bar.min.x + 60 and the right-hand controls and padlock unchanged.

## Commits

- b691a2a: feat(48-02): redraw section headers with game toggle plate and stone bar

## What changed

- `header_layout` (plate 27x32 at +4,+2; bar from plate.max.x + 4 to the row's right edge; title_x = bar.min.x + 60) with three tests: spec geometry, snapping at ppp 1.0/1.25/1.5/2.0, cap aspect via `three_h_rects` with the real `BAR_LEFT_PX`/`BAR_RIGHT_PX` constants.
- `draw_section_header` paints the bar first, then a ROW_HOVER wash and focus stroke on the bar, then the right cluster inside `bar.max.x - 12`, then the plate (state 2 pressed, 1 hover/focus, 0 normal). Toggle logic, rename, whole-row click and `SECTION_HEADER_HEIGHT` (36) untouched.
- Removed the flat PANEL ground, the two 1pt border lines, the painted chevron and the rule line (and the dead `name_end`).
- `cargo test --locked` green (1711 lib tests); tests/fixtures untouched; no new dependencies.

## Deviations from Plan

- Tests and implementation were committed together in one commit (not separate RED/GREEN), and both tasks share that commit since they edit the same function.
- Rule line removal is the judgement call the plan anticipated.
- No screenshot check was done (autonomous run); visual verification of the plate/bar is left to the author.

## Known Stubs

None.

## Self-Check: PASSED

Commit b691a2a exists; full test suite passes.
