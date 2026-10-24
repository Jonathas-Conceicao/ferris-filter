---
phase: 49-drag-and-drop-reorder
reviewed: 2026-10-23T00:00:00Z
depth: standard
iteration: 2
files_reviewed: 13
files_reviewed_list:
  - src/filter/mod.rs
  - src/filter/structure.rs
  - src/models/config.rs
  - src/overlay/app.rs
  - src/overlay/config_state.rs
  - src/overlay/config_view.rs
  - src/overlay/drag_reorder.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/screenshot.rs
  - src/skin/textures.rs
  - src/skin/widgets.rs
findings:
  critical: 0
  warning: 0
  info: 7
  total: 7
deferred: 1
status: clean
---

# Phase 49: Code Review Report (iteration 2)

**Reviewed:** 2026-10-23
**Depth:** standard. Fix commits `b9f1513..HEAD` (dff9585, 36aa3a8, b6de218, 95664e0, c54cd49),
checked against the code around them.
**Files Reviewed:** 13
**Status:** clean. No new defects. WR-04 is deferred by decision (see below). Info items remain.

## Summary

All four fixed warnings are resolved, and I found no regressions. I re-ran the drag, marker
and reorder lib tests (`cargo test --locked --lib -- drag marker reorder style_noop gap`):
59 passed, 0 failed.

- **WR-01, drop marker under the shadow: resolved.**
  - Both lists now paint the gold line through
    `ctx.layer_painter(LayerId::new(Order::Tooltip, shadow_id))` after the shadow `Area` has
    been shown. Shapes on one layer paint in the order they are added, and egui's
    `GraphicsLayers::drain` also draws layers missing from the area order. So the line is on
    top even on a frame where the shadow is not shown.
  - The clip rect is the scroll viewport: `ui.clip_rect()` captured inside the scroll closure
    in RULES, and the content `ui.clip_rect()` in CONFIG. The line therefore never spills
    over the toolbar or the bottom bar.
  - The opaque `pal::PANEL` ground (opaque `from_rgb`) is painted before `disable()` /
    `set_opacity(0.6)`. Those calls only affect shapes added after them, so the ground stays
    opaque.
  - The new tests `drop_marker_is_painted_above_the_shadow` and
    `style_drop_marker_is_painted_above_the_shadow` check the topmost shape at the boundary.
    `FullOutput::shapes` is in paint order, so the check is meaningful.
- **WR-02, RULES drag continues after reorder is disabled: resolved.**
  - `src_index.is_none() || (!reorder_enabled && d.injected.is_none())` routes to
    `cancel_drag`, which calls `ctx.stop_dragging()`.
  - It runs before the release-commit branch. A release on the same frame that search turns
    on is therefore cancelled, not committed.
  - Screenshot injections are exempt. That is correct, because they never commit
    (`d.injected.is_none()` gates the commit branch).
  - The new test, `reorder_switched_off_mid_drag_cancels_it`, covers the path.
- **WR-03, CONFIG commits a release anywhere in the viewport: resolved.**
  - `cfg.row_rects` is filled only for custom rows (`if custom { ... push }`). The drop rect is
    therefore exactly the custom table, plus h/2 of slack, intersected with the viewport.
  - Both the marker gate (`last_gap`) and the release check (`drop_rect`) use it. Releasing
    over the Filter section, Economy Visibility or the built-in table now cancels, and the
    updated test asserts that.
  - Edge cases checked, none of them a defect:
    - **Table scrolled out of view:** `intersect` gives an inverted rect whose `contains` is
      always false, so the drop cancels.
    - **Release on the frame right after drag start:** `drop_rect` is still `None` and the
      drag cancels. It already did, since `last_gap` was `None` too.
    - **Rect one frame stale during auto-scroll:** it is consistent with `last_gap`, which
      comes from the same frame.
- **WR-05, `models` depends on an overlay module: resolved.**
  - `style_noop` / `style_gap_to_index` now live in `models::config`, and
    `StyleSet::move_custom_to` calls them locally.
  - `drag_reorder` re-exports them, so `models` no longer references `crate::overlay`.

## Deferred (not blocking)

### WR-04 (deferred by author decision): `moved across Continue` can be cleared by a later non-crossing drop

**File:** `src/overlay/rules.rs:1800-1851` (`move_rule_to`), same logic in `move_rule`
(`src/overlay/rules.rs:1763-1792`)

**Status:** skipped in iteration 1. The drag deliberately keeps the arrows' flag semantics,
which the author bound. Each available fix would need an author decision:

- **Sticky flag:** trades a missed warning for a stale one.
- **Flag compared with the applied document:** a design change to both the arrows and the drag
  paths.

The 49-REVIEW-FIX.md recommendation still applies: if the author accepts the sticky-flag
trade-off, drop the `else { self.moved.remove(&id) }` branches and update
`move_to_sets_and_clears_the_continue_flag_like_the_arrows`. I am not counting it as an open
finding.

## Info

These items are carried over from iteration 1. The fix commits did not touch them, and they
are still present.

### IN-01: The arrow move paths are dead in production

**File:** `src/overlay/rules.rs:1763` (`move_rule`); `ConfigAction::MoveStyle` handler in
`src/overlay/app.rs`; `ConfigState::move_style` / `StyleSet::move_custom`

**Issue:** No view emits `RowAction::Move` or `ConfigAction::MoveStyle` any more. These paths
are reachable only from tests.

**Fix:** Remove them, or mark them as test support.

### IN-02: The injected RULES pointer uses the one-shot scroll target, not the current offset

**File:** `src/overlay/rule_list.rs:563-572`

**Issue:** `list_rect.top() + y - offset.unwrap_or(0.0)` reads the pending scroll target
(usually `None`). In a scrolled screenshot, the pointer lands on the wrong boundary. This only
affects debug builds.

**Fix:** Resolve the injected position inside the scroll closure from `viewport.min.y`.

### IN-03: Screenshot injection skips the drag-start collapse

**File:** `src/overlay/app.rs:1386-1408`

**Issue:** Injected `DragState` / `StyleDrag` values don't clear `expanded` or bump the epoch.
So `--drag-style` combined with `--expand-style` renders an open editor during a drag, which a
real drag cannot reach. The iteration-1 fix report notes that config-drag.png still shows this.

**Fix:** Clear `list.expanded` / `config_ui.expanded` when injecting.

### IN-04: The source-row wash differs between the lists and uses a magic colour

**File:** `src/overlay/config_view.rs:1053` vs `src/overlay/rule_list.rs:1536`

**Issue:** CONFIG uses `Color32::from_rgba_unmultiplied(0x0A, 0x0A, 0x09, 140)`, while RULES
uses `pal::GROUND.gamma_multiply(0.55)`, which is what the UI-SPEC prescribes.

**Fix:** Use the same palette expression in both lists.

### IN-05: The focus-loss cancel is off when focus is unknown, and the reset paths don't stop egui's drag

**File:** `src/overlay/rule_list.rs:164` (`clear_ids`), `src/overlay/config_view.rs:198`
(`reset_for_new_styles`); `had_focus = focused.unwrap_or(false)` at both drag starts

**Issue:** When the platform reports no focus state at drag start, losing focus never cancels
the drag. Separately, the two reset paths drop the drag without calling `ctx.stop_dragging()`,
so handle hover art stays suppressed until the button is released. Neither affects
correctness.

**Fix:** Route both reset paths through `cancel_drag`.

### IN-06: `move_rule_to` / `move_run_to` accept a `Before(follower)` destination

**File:** `src/overlay/rules.rs:1800`, `src/filter/structure.rs:532`

**Issue:** A `Before` destination that names a non-leader block splits another `# ff:group`
run. The overlay always sends a leader, so this can't happen today, but the API allows it.

**Fix:** Map the id to its group leader, or refuse it.

### IN-07: A blank-less neighbour can make a valid drop fail closed (not verified)

**File:** `src/filter/structure.rs:603-635`

**Issue:** In a hand-written file whose previous block runs straight into the destination,
the moved run's leading comment may be re-attributed to the block above it. If so,
`commit_structural` refuses the move. No data is lost, but the user sees a refusal for a legal
drop.

**Fix:** When the line just before the insertion point is not blank, move one of the region's
trailing blanks in front of it. Add a block-into-block test.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
