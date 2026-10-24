---
phase: 49-drag-and-drop-reorder
plan: 04
subsystem: overlay
tags: [egui, drag-and-drop, rule-list]
requires: [49-01, 49-02, 49-03]
provides:
  - "RULES list drag-and-drop: handle on user rule rows, list-level DragState, wheel/auto-scroll, marker, source wash, shadow, one MoveTo per drop"
affects: [49-05]
key-files:
  modified: [src/overlay/rule_list.rs, src/overlay/app.rs]
key-decisions:
  - "Nearest slot is chosen among real places first, then a no-op result means no slot (not 'nearest non-no-op')"
  - "Every expanded row collapses at drag start"
  - "A drop on a collapsed header does not expand the section"
  - "Escape is consumed during a drag"
requirements-completed: [DND-01, DND-02, DND-03, DND-04]
completed: 2026-10-23
---

# Phase 49 Plan 04: RULES list drag and drop Summary

User rule rows now carry `ff_drag_handle` where the arrows were, and dragging it scrolls, marks, shadows and drops through one `RowAction::MoveTo`; section headers keep their arrows and economy rows have no handle.

## What was built

- Task 1 (0e6c038): `DragState { src, handle, had_focus, injected, last_slot }` and `ListUi.drag`, `ListUi::cancel_drag(ctx)`, `clear_ids` clears the drag. In `draw_row` the two arrows are replaced by the handle (disabled with the search tooltip while searching). `drag_started` captures `handle.id`, collapses every expanded row and requests a repaint; a tap does nothing. `app.rs`: `MoveTo` counts as a move (`pending_scroll`), and `cancel_drag` runs where `Effect::Conceal` fires.
- Task 2 (3476bca): in `show_list`, before rows: Escape (consumed with `consume_key`), vanished source, or focus lost after starting focused cancel; otherwise when egui no longer drags the captured handle, a real release over `list_rect` on the last valid, non-no-op slot pushes one `MoveTo` (BeforeRule -> `Before(row id)`, AfterBanner, BeforeBanner, End). While live, inside the scroll closure: `scroll_with_delta_animation(wheel - autoscroll * dt, none)` per the 49-01 spike, slot computed from the shadow centre, gold 2pt marker only when the pointer is in the list, source row washed (`GROUND` 55%, text dimmed), and an `Order::Tooltip` Area with `ui.disable()` + `set_opacity(0.6)` drawing the row via a throwaway `ListUi` at the list's left edge. `ListFrame.scroll_y` added for tests. Pure helpers `slot_entries_of` and `move_dest`.
- Tests: rewrote the arrows test into `handle_disabled_while_searching_and_starts_a_drag_when_enabled`; tap, collapse-all, `clear_ids`, flag budget with the handle width; a `DragRig` covering wheel scroll, edge auto-scroll with the source virtualized out then drop, release outside / pointer gone, Escape consumed, focus loss, no-op, economy boundary, collapsed header (stays collapsed after apply), grouped leader, shadow geometry.

## Judgement calls

- Collapse-all at drag start (not only the dragged row), as planned.
- A drop on a collapsed section header does not expand it; `pending_scroll` is then a harmless no-op.
- Escape is consumed during a drag (nothing else in the overlay reads it while the list is dragging).
- Slot choice deviates from the plan's formula: `nearest_slot` filters only on `slot_target.is_some()` and then a no-op result yields no slot. With the plan's `valid = target && !noop`, hovering over the source's own place would snap to the neighbouring boundary and commit a real move; mine shows no marker and cancels, matching "release on the source's own boundary emits nothing".
- Focus is read from `viewport().focused` (as `app.rs` does), not `InputState::focused`, which headless events never change; unknown focus counts as never focused.
- The shadow's handle is drawn disabled (dim) because the Area is a disabled `Ui`; accepted, not separately dimmed.
- `arrows_w` is now measured after the 4pt gap (handle 28 + 4).
- `clicking_the_summary_text_expands_the_row` control click moved from x=500 to x=420, since 500 is now the handle.
- Not done: the debug screenshot mode's `injected` pointer is plumbed in `DragState` but no `--drag` flag was added (not in this plan).
- rustfmt on `rule_list.rs` rewrote its working-copy line endings to LF (index is LF, `.gitattributes` text=auto), no content effect.

## Deviations from Plan

None beyond the judgement calls above.

## Verification

`cargo test --locked` green (1835 lib tests plus integration suites). `tests/fixtures` unchanged, no dependency changes. STATE.md and ROADMAP.md untouched.

## Commits

- 0e6c038: feat(49-04): drag handle replaces rule-row arrows, DragState and MoveTo wiring
- 3476bca: feat(49-04): rule drag lifecycle: scroll, marker, shadow, drop and cancel

## Self-Check: PASSED
