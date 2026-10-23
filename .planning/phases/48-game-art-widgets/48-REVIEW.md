---
phase: 48-game-art-widgets
reviewed: 2026-10-23T00:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - src/skin/widgets.rs
  - src/skin/slice.rs
  - src/skin/textures.rs
  - src/overlay/rule_list.rs
  - src/overlay/body.rs
  - assets/poe2/manifest.toml
findings:
  critical: 0
  warning: 3
  info: 5
  total: 8
status: issues_found
---

# Phase 48: Code Review Report

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

The diff 9a56894..HEAD was read in full, and the art files named in the manifest were checked to exist. No crash, data-loss or security defect was found. I checked these areas specifically:

- **Egui ids:** the edit id (`id`), `id.with("glyph")` and `id.with("clear")` are distinct. The rules search id comes from `ui.id()` of the toolbar child, so it is stable. The picker id `popup_id.with("query-edit")` does not collide with the `query` and `focused` data keys.
- **Clear X geometry:** its hit zone (right edge minus 29 to minus 5) does not overlap the edit rect (ends at right edge minus 32). The clear click re-requests focus on the release frame.
- **Escape:** the rules-search Escape handler still works. The text edit surrenders focus on Escape, and `lost_focus()` is covered by the handler.
- **Section header hit-testing order:** `resp` is allocated first and the right cluster and plate are interacted afterwards. Later widgets win in egui, so plate and right controls take precedence over the whole-bar toggle. `resp.clicked()` and `chev.clicked()` are mutually exclusive, so there is no double toggle.
- **`box_uv` and `paint_visible`:** the half-texel inset maths is right. `ICON_BUTTON_OPAQUE` and `CHECK_OPAQUE` are consistent with the doc comments.
- **Reload glyph:** `reload_glyph_rect` is consistent with the old maths once the plate fills the rect.
- **DPI:** the header rects are snapped, and the layout test covers 1.0 to 2.0 ppp.

Remaining issues are keyboard and accessibility regressions and some dead code.

## Warnings

### WR-01: Search glyph is an unlabeled, focusable Tab stop

**File:** `src/skin/widgets.rs:1458-1462`
**Issue:** `ui.interact(lay.glyph, id.with("glyph"), Sense::CLICK)` creates a focusable widget with no `widget_info`. Tab walks to a nameless button with no visible focus ring that does nothing visible. Screen readers see an unlabeled button. This is a second stop in front of the real edit, in the RULES toolbar and in the + condition popup.
**Fix:** Make it non-focusable, since the edit itself is the keyboard target:
```rust
let g = ui.interact(lay.glyph, id.with("glyph"), Sense::click().with_focusable(false));
// or Sense { focusable: false, ..Sense::CLICK }
```
If it stays focusable, add `g.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, "Search"))`.

### WR-02: Section toggle plate keeps the accessible name "chevron" and stays a second Tab stop

**File:** `src/overlay/rule_list.rs:769-770`
**Issue:** The plate is now the game's expand/collapse art, with no chevron to name. Its accessible name is still the literal `"chevron"`, which says nothing about its function. The whole-bar `resp` is a second focusable toggle for the same action, so every header costs two Tab stops. The old code had the same duplicate; the art redraw was the moment to fix it.
**Fix:** Name it after the action, for example `if collapsed { "Expand section" } else { "Collapse section" }`. Make the plate non-focusable, or fold it into `resp`.

### WR-03: Test for "clear X keeps focus" cannot fail on the property it names

**File:** `src/skin/widgets.rs:1946-1969` (test `search_field_clear_x_empties_and_keeps_focus`)
**Issue:** No frame gives the edit focus before the click. The final `has_focus(id)` assertion therefore passes purely because the click handler calls `request_focus(id)`. The test cannot catch the real regression, which is the edit surrendering focus on the press frame while the clear zone is pressed. There is also no test for the Escape path, the picker integration, or the header hit-testing order (plate vs bar vs right controls). Only pure layout is tested for the header.
**Fix:** Focus the edit first with `ctx.memory_mut(|m| m.request_focus(id))` and run a frame. Then click the X and assert the edit still has focus and the text is empty. Add a header test that clicks the plate, then the bar, then the remove X, and asserts exactly one toggle or action each.

## Info

### IN-01: Stale comment after the rule line was removed

**File:** `src/overlay/rule_list.rs:809`
**Issue:** "The name (or its edit box) and the rule line after it." No rule line is drawn any more.
**Fix:** Drop "and the rule line after it".

### IN-02: `FlagIcon::ChevronRight` / `ChevronDown` are now unused outside tests

**File:** `src/skin/icons.rs:36-38, 139-148, 185-186`
**Issue:** The header was the only production user. The variants and their paint code are now dead (the enum is `pub`, so there is no compiler warning).
**Fix:** Remove them and their test list entries, or note why they are kept.

### IN-03: `ff_text_field` has no production caller left

**File:** `src/skin/widgets.rs:1352`
**Issue:** Both production uses moved to `ff_search_field`. Only a unit test (`widgets.rs:1912`) still calls it.
**Fix:** Delete it, or keep it deliberately and say so.

### IN-04: Magic numbers in geometry

**File:** `src/skin/widgets.rs:1429` (`field.max.x - 17.0`, `+ 3.0` / `- 3.0`), `src/overlay/rule_list.rs:~697` (`- 12.0`), `src/overlay/rule_list.rs:~800` (`- 8.0`)
**Issue:** The clear X centre inset, the vertical edit inset, and the right padding of the header are bare literals, unlike their sibling constants. The `3.0` is also duplicated in `text_field_inner`.
**Fix:** Name them (`CLEAR_CENTRE_INSET`, `SEARCH_EDIT_VPAD`, `HEADER_RIGHT_PAD`).

### IN-05: Narrow-width title and control overlap

**File:** `src/overlay/rule_list.rs:~795` (`slot_w = (...).max(20.0)`)
**Issue:** The title starts 60pt into the bar. If the right cluster and the count leave less than 20pt, the 20pt floor makes the title draw under the controls. The docked window is a fixed 660pt, so this is not reachable today.
**Fix:** None needed now; clamp the title to the space left, or hide the count first, if the width ever becomes variable.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
