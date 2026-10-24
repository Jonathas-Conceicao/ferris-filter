---
phase: 49-drag-and-drop-reorder
plan: 02
subsystem: skin
tags: [egui, skin, drag-handle, art]
requires: []
provides:
  - "Skin.drag_handle: [TextureHandle; 3] embedded from assets/custom/icon_reoder_*.png"
  - "skin::widgets::ff_drag_handle, DRAG_HANDLE_OPAQUE, DRAG_HANDLE_SIZE, DRAG_HANDLE_HIT"
affects: [49-04, 49-05]
key-files:
  created: []
  modified: [src/skin/textures.rs, src/skin/widgets.rs]
key-decisions:
  - "Handle senses click_and_drag, not drag only (UI-SPEC said Sense::drag())"
requirements-completed: [DND-01, DND-05]
completed: 2026-10-23
---

# Phase 49 Plan 02: Handle art and ff_drag_handle Summary

The author's nine art files are tracked under their original names, the three handle PNGs are embedded in the Skin, and `ff_drag_handle` paints the visible plate into a 28pt square with a 32pt grab target.

## What was built

- Task 1 (16638d1): nine `assets/custom/` files committed unrenamed and unmodified; `Skin.drag_handle` (0 normal, 1 hover, 2 pressed); tests for the light skin and for the three embedded PNGs being 92x92.
- Task 2 (a109d6e): `ff_drag_handle(ui, skin, enabled, dragging) -> Response`. Painted with `slice::paint_visible(.., DRAG_HANDLE_OPAQUE, ..)`. Enabled: `Sense::click_and_drag()` on a 32pt rect centred on the plate, Grab / Grabbing cursor, tooltip `Drag to reorder` (none while any drag runs). Disabled: `Sense::hover()`, art 0, tint alpha 90, tooltip `Clear the search to reorder`. Accessible name `Reorder`. No keyboard handling.
- Tests: plate geometry and UV (`box_uv` of [0,0,84,84]), hit rect centring, disabled sense, tap on the handle leaving a `Sense::click` row behind unclicked, accessible name.

## Deviations from Plan

None in scope. Judgement calls:
- `click_and_drag` instead of the UI-SPEC's `Sense::drag()`, as the plan directs (research Pitfall 3, confirmed by the 49-01 S3 spike): a tap is consumed by the handle and does not toggle the row. Callers must ignore `clicked()`.
- The returned `Response` is the 32pt grab target's (enabled) or the 28pt plate's (disabled).
- Hover art is suppressed while another drag is in progress (`ctx.dragged_id().is_some()`).
- Test detail: the tap test needs a prior frame on the same context so egui has the widget rects; fixed in the test, not the widget.
- `cargo fmt` reformatted five unrelated files; I reverted those and kept only `src/skin/widgets.rs`.

## Verification

`cargo test --locked` green (1805 lib tests plus integration suites). No dependency changes. STATE.md and ROADMAP.md untouched.

## Commits

- 16638d1: feat(49-02): commit reorder handle art and embed it in the skin
- a109d6e: feat(49-02): add ff_drag_handle widget

## Self-Check: PASSED
