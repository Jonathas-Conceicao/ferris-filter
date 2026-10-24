---
phase: 49-drag-and-drop-reorder
plan: 01
subsystem: overlay
tags: [egui, drag-and-drop, spike, virtual-list]
requires: []
provides:
  - "src/overlay/drag_reorder.rs: pure slot, auto-scroll and shadow maths shared by RULES and CONFIG"
  - "proven scroll and opacity mechanisms for plans 04 and 05"
affects: [49-04, 49-05]
tech-stack:
  added: []
  patterns:
    - "forward wheel and auto-scroll with Ui::scroll_with_delta_animation(.., ScrollAnimation::none())"
    - "drag tracked at list level by the captured handle Id"
key-files:
  created: [src/overlay/drag_reorder.rs]
  modified: [src/overlay/mod.rs]
key-decisions:
  - "Scroll during a drag: scroll_with_delta_animation with ScrollAnimation::none(); the vertical_scroll_offset fallback was not needed"
  - "Shadow opacity: ui.disable() then ui.set_opacity(0.6) on an Order::Tooltip Area; per-colour gamma_multiply not needed"
requirements-completed: [DND-02, DND-04]
duration: about 25 min (mostly the first cold build)
completed: 2026-10-23
---

# Phase 49 Plan 01: Drag spike and pure drag_reorder Summary

Headless egui spikes confirm that a forwarded `scroll_with_delta_animation` scrolls both lists during a handle drag, and `drag_reorder.rs` holds the pure slot, auto-scroll and shadow maths.

## Spike results

All eight spike tests passed on the first run, so no fallback variant and no `#[ignore]`d test exist.

| Id | Question | Result |
|----|----------|--------|
| S1a | Does the ScrollArea take the wheel while a child handle is dragged? | No. With no forwarding the offset stays 0 (documents egui's `dragged_id().is_none()` gate). |
| S1b | Does forwarding `smooth_scroll_delta` via `scroll_with_delta_animation(.., none())` scroll it? | Yes, in a virtualized `show_viewport` list; offset grew past 50pt in 8 wheel frames. |
| S1c | Does edge auto-scroll move the offset by about speed*dt per frame? | Yes, within 25% over 20 frames at 5pt from the bottom, using `-autoscroll_speed * dt`. |
| S2 | Does the drag survive the handle being virtualized out and its id scope changing? | Yes. `dragged_id()` stayed `Some(captured)` on every frame (row 3 left the visible range, salt changed at frame 5). `drag_stopped_id()` equalled the captured id exactly once, on the release frame, with `primary_released` true. |
| S3 (tap) | Does a tap on a `click_and_drag` handle start a drag or reach the row click? | Neither. No `drag_started`; the row's own `Sense::click` did not fire; the handle itself reported `clicked()` (the list must ignore it). |
| S3 (Escape) | Does Escape abort the drag? | Yes. Next frame `dragged_id()` is None, `drag_stopped_id()` is the handle, `primary_released()` is false, and after `consume_key(Escape)` a later `key_pressed(Escape)` reads false. |
| S4 | Does a disabled Tooltip-order Area steal hits, and does `set_opacity(0.6)` give about 153 alpha? | No hit stealing (click reached the widget underneath). Shadow fill alpha within 3 of 153, so `set_opacity` works as the research assumed. |
| S5 | Same forwarding on a plain non-virtualized `ScrollArea::show` (CONFIG tab style)? | Yes. Offset grows and recorded row rects shift by exactly the scrolled amount. |

Not covered here (needs real widgets, so plans 04/05): `draw_row` / `style_row` inside the shadow Area (A2, the extra dimming of widgets that dim themselves when disabled) and the nested inner scroll area stealing the delta (A3, still handled by collapsing expanded rows at drag start).

## Mechanism decisions for plans 04 and 05

- **Plans 04/05 use: `ui.scroll_with_delta_animation(vec2(0.0, wheel - auto * dt), ScrollAnimation::none())`** called from inside the scroll content closure every frame while a drag is active, with `wheel = input.smooth_scroll_delta.y` and `auto = autoscroll_speed(pointer_y, top, bottom, EDGE_ZONE, AUTOSCROLL_MAX)` (signed, positive = down), `dt = DT` capped at 0.1 (the spike used the fixed `predicted_dt`; use `stable_dt.min(0.1)` live). Same call for the virtualized RULES list and the plain CONFIG `ff_scroll_area`. The `vertical_scroll_offset` fallback is not needed.
- **Plans 04/05 use for the shadow: `Area` on `Order::Tooltip`, `interactable(false)`, then `ui.disable()` followed by `ui.set_opacity(0.6)`** (in that order). Do not use per-colour `gamma_multiply`. If the real row looks too dim from widgets that dim themselves when disabled, add a `shadow: bool` to `draw_row` instead of disabling.
- **Handle sense:** `Sense::click_and_drag()`, register the row's click first and the handle after it. Ignore the handle's `clicked()`.
- **Drag lifecycle:** capture `Response::id` at `drag_started`; finish from `ctx.dragged_id()` / `ctx.drag_stopped_id()`, never from the handle's `Response`. Commit only when `drag_stopped_id == captured && primary_released`.
- **Escape:** egui aborts the drag itself; on that frame call `consume_key(Modifiers::NONE, Key::Escape)` so nothing else reacts.

## Pure API (src/overlay/drag_reorder.rs)

No egui types outside `#[cfg(test)]`. Constants `EDGE_ZONE` 40, `AUTOSCROLL_MAX` 900, `MARKER_THICKNESS` 2. Types `SlotEntry` (Rule, UserHeader{ordinal, collapsed}, EconomyHeader, EconomyRule) and `SlotTarget` (BeforeRule, AfterBanner, BeforeBanner, End). Functions `nearest_slot`, `autoscroll_speed`, `shadow_top`, `slot_target`, `is_noop`, `style_gap_for_y`, `style_noop`, `style_gap_to_index`. 9 unit tests cover the research table, collapsed header followed by header, the economy boundary (T-49-01), no-ops, custom-style gaps, short viewports and NaN.

## Deviations from Plan

None. The plan executed as written; the spikes needed no fallback.

Judgement calls:
- `is_noop` also returns true when `src` is not a `Rule` entry (nothing to move), a safe default the plan did not specify.
- `slot_target` returns `None` for an out-of-range boundary and for a boundary below an `EconomyRule`-only prefix (cannot occur in real lists).
- In a short viewport (under 2 * zone) the nearer edge wins, as the research suggested.
- A drop on a collapsed header makes the rule that section's first rule and the section stays collapsed (documented on `slot_target`).

## Verification

`cargo test --locked` green (1798 lib tests plus integration suites), no change to `Cargo.toml` / `Cargo.lock`, no new dependencies.

## Commits

- 47d1e77: test(49-01): spike drag scrolling, survival, tap/Escape and shadow opacity
- (next): feat(49-01): pure drag_reorder API (see git log)

## Self-Check: PASSED
