---
phase: 49-drag-and-drop-reorder
plan: 05
subsystem: overlay
tags: [egui, drag-and-drop, config-styles]
requires: [49-01, 49-02, 49-04]
provides:
  - "CONFIG style table drag-and-drop: StyleSet::move_custom_to, ConfigState::move_style_to, ConfigAction::MoveStyleTo, Order-column handle, style drag lifecycle"
affects: []
key-files:
  modified: [src/models/config.rs, src/overlay/config_state.rs, src/overlay/config_view.rs, src/overlay/app.rs]
key-decisions:
  - "Built-in rows keep an empty 28pt Order cell: no handle, no tooltip"
  - "Gap choice mirrors 49-04: nearest gap first, a no-op result means no gap and no marker"
  - "Every expanded style editor collapses at drag start"
  - "Escape is consumed during a style drag"
requirements-completed: [DND-01, DND-02, DND-03, DND-04]
completed: 2026-10-23
---

# Phase 49 Plan 05: CONFIG style table drag and drop Summary

Custom style rows now carry the drag handle at the right edge of the Order column; dragging scrolls, marks the nearest custom gap, washes the source and shows a 60% shadow, and a drop emits one `ConfigAction::MoveStyleTo` through the existing pending-edit path.

## What was built

- Task 1 (7b07223): `StyleSet::move_custom_to(name, gap)` rotates only the custom index slots (built-ins never move, so a custom can never land above them), reusing `drag_reorder::{style_noop, style_gap_to_index}`; false for built-ins, unknown names and no-op gaps, gap clamped. `ConfigState::move_style_to`, `ConfigAction::MoveStyleTo { name, to }`, and the `app.rs` dirty-list entry and handler (clear config message, Applied becomes Unapplied), identical to `MoveStyle`. `move_custom`, `move_style` and `MoveStyle` kept.
- Task 2 (c26a04d): in `config_view.rs`, `StyleDrag`, `ConfigUi.drag`, `ConfigUi.row_rects`, `ConfigUi::cancel_drag`. `style_row` takes `custom: bool`: custom rows draw `ff_drag_handle` (probe `row-handle`), built-ins an empty handle-sized cell (probe `row-order` kept); the arrows and `ORDER_TIP` are gone. `show_styles` finishes or cancels a drag before drawing (Escape consumed, vanished source, focus loss, release outside the area), forwards wheel and edge auto-scroll with `scroll_with_delta_animation`, and after the rows `draw_style_drag` computes the gap from custom row tops plus last bottom (`style_gap_for_y`, no-op filtered), paints the gold marker, washes the source row and draws the shadow Area (`Order::Tooltip`, disabled, opacity 0.6) via a throwaway `ConfigUi`. `reset_for_new_styles` clears the drag (Reload, Apply, disk follow); `app.rs` calls `config_ui.cancel_drag` on Conceal.
- Tests: `move_custom_to` (swap equivalence, built-in slots, refusals, clamp), `reorder_to_is_dirty_and_reverts`, Order-cell handle geometry, `custom_handle_drags_and_builtin_rows_have_none`, tap, collapse at drag start, cancel/no-op/outside/focus loss/Escape, gaps only among customs, wheel scroll (a `StyleRig` over a scroll area).

## Judgement calls

- Blank built-in Order cell with no tooltip, as directed.
- Collapse-all at drag start, Escape consumed, same as 49-04.
- The CONFIG "area" for the release check and auto-scroll edges is the scroll area's visible viewport (`ui.clip_rect()` inside `show_styles`), since the plan did not name a rect.
- The shadow's handle draws disabled (dim), as in 49-04.
- The fixture already has three custom styles (Recycle, kite, exceptional) and file order is not built-ins-first, so tests assert that built-in index slots are unchanged rather than that built-ins come first.
- `cargo fmt` reformatted unrelated files (dynamic.rs, emitter.rs, skin/editor.rs, tests/isolation_guard.rs, app.rs); these were reverted and only my changes committed.
- `Reset to defaults` / `Revert` clear the drag through `reset_for_new_styles` without `ctx.stop_dragging` (harmless: the finish logic ignores a missing drag).

## Deviations from Plan

None beyond the judgement calls above.

## Verification

`cargo test --locked` green (1842 lib tests plus integration suites). `tests/fixtures` unchanged, no dependency changes. STATE.md and ROADMAP.md untouched.

## Commits

- 7b07223: feat(49-05): StyleSet::move_custom_to and ConfigAction::MoveStyleTo
- c26a04d: feat(49-05): style table drag handle and drag lifecycle

## Self-Check: PASSED
