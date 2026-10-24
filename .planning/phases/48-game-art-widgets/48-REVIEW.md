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
  warning: 0
  info: 4
  total: 4
status: clean
---

# Phase 48: Code Review Report

**Reviewed:** 2026-10-23
**Depth:** standard (iteration 2 fix re-review: quick)
**Files Reviewed:** 6
**Status:** clean

## Summary

The diff 9a56894..HEAD was read in full, and the art files named in the manifest were checked to exist. No crash, data-loss or security defect was found.

**Iteration 2 (fix re-review of 9831246 and 7622346):** WR-01, WR-02 and WR-03 are resolved.

- In egui 0.36 `Sense::CLICK` has no `FOCUSABLE` flag (only `Sense::click()` adds it), so the search glyph and the header toggle plate are no longer Tab stops. The new test `search_glyph_is_not_a_tab_stop` pins this for the glyph.
- The plate is now named "Expand section" / "Collapse section" after its action, and the whole-bar `resp` is the single keyboard stop per header.
- The clear-X test now focuses the edit first and asserts that focus before the click, so the final assertion is meaningful.
- The 13 `search_` tests pass. No new Critical or Warning issues were found.
- IN-01 (stale comment) was fixed in 7622346 and is dropped from the list below.
- The header hit-order test suggested in the old WR-03 was not added. It is optional.

Earlier checks that still hold: egui ids are distinct, the clear X hit zone does not overlap the edit, Escape handling is intact, header hit-testing order (later widgets win) gives no double toggle, `box_uv` / `paint_visible` maths is right, and DPI snapping is covered by the layout test.

## Info

### IN-01: `FlagIcon::ChevronRight` / `ChevronDown` are now unused outside tests

**File:** `src/skin/icons.rs:36-38, 139-148, 185-186`
**Issue:** The header was the only production user. The variants and their paint code are now dead (the enum is `pub`, so there is no compiler warning).
**Fix:** Remove them and their test list entries, or note why they are kept.

### IN-02: `ff_text_field` has no production caller left

**File:** `src/skin/widgets.rs:1352`
**Issue:** Both production uses moved to `ff_search_field`. Only a unit test still calls it.
**Fix:** Delete it, or keep it deliberately and say so.

### IN-03: Magic numbers in geometry

**File:** `src/skin/widgets.rs:~1429` (`field.max.x - 17.0`, `+ 3.0` / `- 3.0`), `src/overlay/rule_list.rs:~697` (`- 12.0`), `src/overlay/rule_list.rs:~800` (`- 8.0`)
**Issue:** The clear X centre inset, the vertical edit inset, and the right padding of the header are bare literals, unlike their sibling constants. The `3.0` is also duplicated in `text_field_inner`.
**Fix:** Name them (`CLEAR_CENTRE_INSET`, `SEARCH_EDIT_VPAD`, `HEADER_RIGHT_PAD`).

### IN-04: Narrow-width title and control overlap

**File:** `src/overlay/rule_list.rs:~795` (`slot_w = (...).max(20.0)`)
**Issue:** The title starts 60pt into the bar. If the right cluster and the count leave less than 20pt, the 20pt floor makes the title draw under the controls. The docked window is a fixed 660pt, so this is not reachable today.
**Fix:** None needed now; clamp the title to the space left, or hide the count first, if the width ever becomes variable.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard (iteration 2 re-review: quick)_
