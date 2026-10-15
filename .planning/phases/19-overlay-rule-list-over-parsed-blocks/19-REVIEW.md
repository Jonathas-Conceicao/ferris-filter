---
phase: 19-overlay-rule-list-over-parsed-blocks
reviewed: 2026-10-12T00:00:00Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - src/dynamic.rs
  - src/filter/block.rs
  - src/filter/edit.rs
  - src/filter/mod.rs
  - src/filter/structure.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/economy.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/text_entry.rs
  - src/overlay/virtual_list.rs
  - tests/filter_structure.rs
  - tests/overlay_rules.rs
findings:
  critical: 0
  warning: 0
  info: 1
  total: 1
status: clean
---

# Phase 19: Code Review Report (iteration 2)

**Reviewed:** 2026-10-12
**Depth:** standard
**Files Reviewed:** 15
**Status:** clean (one Info item)

## Summary

I read the four iteration-1 fix commits (3aa18ae, 35ff098, 963ffb4, 91bacc2) against the code they touch. I didn't compile or run the tests. I also re-read the `apply_edited` and `blocking_errors` path in `src/dynamic.rs`, and the Apply, Revert and row-action flow in `src/overlay/app.rs`. All four fixes are correct, and I found no new Critical or Warning issues.

- **WR-01:** `name_open.insert(id)` runs whenever the name field is drawn, so emptying a loaded name no longer collapses the editor. The toggle-off path still removes the id and the draft. Revert, Apply and Reload all call `clear_ids`, so there are no stale drafts.
- **WR-02:** the hidden-add branch now sets only the status message. It no longer sets `expanded` or `pending_scroll`, which the filtered slice would have reset anyway. The test was updated to match.
- **IN-01:** `Unapplied` is now set only when `working.is_dirty()`. `is_dirty` compares the serialized document with `applied_text` and also covers a staged economy operation, so a no-op action can't flip the indicator, and a staged economy op can't be missed.
- **IN-04:** `SetContinue` clears `moved`. Delete, Revert and `on_applied` already handled it.
- **WR-04:** the timing budgets are loose enough to be non-flaky and still catch quadratic behaviour. The count pins were replaced by derived invariants.

The skipped items (WR-03, WR-05, IN-03) stay accepted per the stated rationale.

## Info

### IN-01: SetContinue clears every moved-across-Continue flag, not just the affected one

**File:** `src/overlay/rules.rs:478-483`
**Issue:** `self.moved.clear()` runs before `set_continue` and runs even when `set_continue` fails. A Continue toggle on one rule therefore also drops the warning flag on unrelated rules that were moved across a Continue. The effect is harmless because the flag is advisory. Dropping flags is the safe direction, since the alternative was a stale flag that could be false.
**Fix:** Optionally clear the set only on `Ok`. Otherwise accept the behaviour as is.

---

_Reviewed: 2026-10-12_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
