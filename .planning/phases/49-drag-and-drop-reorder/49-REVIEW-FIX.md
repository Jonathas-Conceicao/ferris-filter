---
phase: 49-drag-and-drop-reorder
fixed_at: 2026-10-23T00:00:00Z
review_path: .planning/phases/49-drag-and-drop-reorder/49-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 4
skipped: 1
status: partial
---

# Phase 49: Code Review Fix Report

**Fixed at:** 2026-10-23
**Source review:** .planning/phases/49-drag-and-drop-reorder/49-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (Critical + Warning; Info findings not in scope)
- Fixed: 4
- Skipped: 1

`cargo test --locked` is green (1845 lib tests plus all integration suites). The clippy
warning count is unchanged, and the touched files are rustfmt-clean. Fixtures and
dependencies are unchanged.

## Fixed Issues

### WR-01: The gold drop marker is drawn below the Tooltip-order shadow, and always inside it

**Files modified:** `src/overlay/rule_list.rs`, `src/overlay/config_view.rs`, `src/overlay/drag_reorder.rs`
**Commit:** 95664e0
**Applied fix:**
- **Marker:** both lists paint the gold line on the shadow's own layer
  (`LayerId::new(Order::Tooltip, shadow_id)`), after the shadow, at full opacity and clipped
  to the scroll viewport. In RULES the line rect is carried out of the scroll closure.
- **Shadow ground:** the shadow row gets an opaque `pal::PANEL` ground, painted before
  `disable()` / `set_opacity(0.6)`. The row stays at 60% on top of it, so the rows below no
  longer show through its text.
- **Tests:** a test-only helper, `drag_reorder::top_fill_at`, finds the topmost painted shape at
  a point. New tests `drop_marker_is_painted_above_the_shadow` (RULES) and
  `style_drop_marker_is_painted_above_the_shadow` (CONFIG) assert that the gold rect is on top
  at the drop boundary. Both tests fail if the line goes back to a layer below Tooltip.
- **Screenshots:** regenerated with the debug ff.exe and checked; the gold line is visible
  across the shadow in both:
  - `C:\Users\jonat\AppData\Local\Temp\claude\ffshot49\rules-drag.png`
  - `C:\Users\jonat\AppData\Local\Temp\claude\ffshot49\config-drag.png`

  The CONFIG shot still uses `--expand-style` (IN-03, out of scope), so it shows an expanded
  editor during the drag. A real drag can't reach that state.

### WR-02: A RULES drag keeps running after reordering becomes disabled (search typed mid-drag)

**Files modified:** `src/overlay/rule_list.rs`
**Commits:** 36aa3a8, c54cd49
**Applied fix:**
- The finish/cancel block at the top of `show_list` now cancels a live (non-injected) drag as
  soon as `reorder_enabled` is false.
- The follow-up commit folds that check into the existing missing-source branch. The separate
  branch had an identical body and tripped clippy's `if_same_then_else`.
- `DragRig` gains a `reorder` switch. New test `reorder_switched_off_mid_drag_cancels_it`: the
  drag ends and the release emits no move.

### WR-03: CONFIG commits a style move when released anywhere in the CONFIG viewport, not just over the table

**Files modified:** `src/overlay/config_view.rs`, `src/overlay/app.rs`
**Commit:** b6de218
**Applied fix:**
- **Drop target:** `draw_style_drag` builds the drop rect from the custom rows: half a row
  above the first and below the last, at the table's width, intersected with the visible area.
  It is stored as `StyleDrag::drop_rect` (new field; also initialised in `app.rs` for the
  screenshot injection).
- **Both checks use it:** the marker/`last_gap` check and the release check. The viewport still
  drives only the auto-scroll edges.
- **Test change:** `gaps_exist_only_among_custom_rows` used to assert the old behaviour
  (release over the built-in table → move to gap 0). It now asserts that this release cancels,
  and that a release just above the first custom row still gives gap 0 (never above a
  built-in).

### WR-05: The model layer now depends on an overlay (UI) module

**Files modified:** `src/models/config.rs`, `src/overlay/drag_reorder.rs`
**Commit:** dff9585
**Applied fix:**
- `style_noop` and `style_gap_to_index` moved into `models::config` as free functions, and
  `StyleSet::move_custom_to` calls them locally.
- `drag_reorder` re-exports them (`pub use crate::models::config::{style_gap_to_index, style_noop}`),
  so the view code and the existing unit tests are unchanged.

## Skipped Issues

### WR-04: `moved across Continue` is cleared by a later non-crossing drop even though the rule still sits across the Continue rule

**File:** `src/overlay/rules.rs:1800`
**Reason:** No fix is strictly better and stays consistent with the arrows, so I left the
author's binding rule ("the arrows' rules hold") as it is. There are two candidate fixes, and
each would need an author decision.

1. **Sticky flag (never cleared until Apply / Revert)**
   - This is a small change to `move_rule` and `move_rule_to`.
   - It fixes the reviewer's case: A, B (Continue), C, D. Drag A below C, then between C and
     D, and A keeps its flag.
   - It adds a false positive. Take X, A, B (Continue). Move A below B, then move B back above
     A, then move A above X. A is on its original side of B, but stays flagged. Today that
     last non-crossing move clears the flag.
   - So it trades a missed warning for a stale one. It is safer, but not strictly better.
2. **Flag computed against the applied document** (the set of Continue rules above the rule,
   compared with the applied text)
   - This is the exact meaning of the flag.
   - It needs a stable mapping from the working rules to the applied rules. Block ids,
     duplicated rules (no baseline), `# ff:group` runs that change block count on a chip edit,
     and the `SetContinue` clear-all all get in the way.
   - It would also change the arrows' semantics (same code path).
   - That is a design change, not a review fix.

**Recommendation:** if the author accepts option 1's trade-off, it is a few lines in both move
functions: drop the `else { self.moved.remove(&id) }` branches. The test
`move_to_sets_and_clears_the_continue_flag_like_the_arrows`, which currently asserts the
clearing, would need updating.
**Original issue:** `move_rule_to` sets the flag only when this one drop passes a Continue rule
and clears it otherwise. A rule that crossed a Continue rule in an earlier drop loses its
warning after a later non-crossing drop, even though it still sits on the other side.

---

_Fixed: 2026-10-23_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
