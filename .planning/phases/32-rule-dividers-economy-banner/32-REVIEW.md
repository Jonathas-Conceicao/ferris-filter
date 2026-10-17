---
phase: 32-rule-dividers-economy-banner
reviewed: 2026-10-16T00:00:00Z
depth: standard
iteration: 2
files_reviewed: 22
files_reviewed_list:
  - src/cli.rs
  - src/dynamic.rs
  - src/filter/divider.rs
  - src/filter/edit.rs
  - src/filter/mod.rs
  - src/filter/parse.rs
  - src/filter/section.rs
  - src/filter/structure.rs
  - src/overlay/app.rs
  - src/overlay/apply_all.rs
  - src/overlay/body.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/screenshot.rs
  - src/overlay/virtual_list.rs
  - src/overlay/widgets.rs
  - src/settings.rs
  - src/skin/icons.rs
  - src/styled.rs
  - tests/dynamic_filter.rs
  - tests/filter_roundtrip.rs
  - tests/filter_structure.rs
findings:
  critical: 0
  warning: 2
  info: 1
  total: 3
status: issues_found
---

# Phase 32: Code Review Report (iteration 2)

**Reviewed:** 2026-10-16
**Depth:** standard
**Files Reviewed:** 22
**Status:** issues_found

## Summary

I read `git diff b8f50f8..HEAD -- src tests` (5 fix commits) and traced each change against its finding. I did not run cargo. The fixer reports a green test run.

All six iteration-1 findings are fixed. No fix regressed anything I could find. I found two new low-severity warnings and one info item, all in the settings and fold-state code. No data-loss or blocker issues remain.

## Verification of iteration-1 findings

| ID | Verdict | Evidence |
|----|---------|----------|
| CR-01 remove divider below economy | Fixed | `remove_section` returns `SectionLocked` when `follows_economy(ordinal)` (`divider.rs`). `section_can_remove` feeds `SectionRow.can_remove`, and the x button is not drawn when it is false (`rule_list.rs:662`). Tests: `divider.rs` unit test and `tests/filter_structure.rs::divider_below_economy_stays_put`. |
| CR-02 move divider below economy down | Fixed | `hop` returns `Ok(None)` for `!up && follows_economy`, so `section_can_move` is false and `move_section` gives `NoNeighbour`. The ordinal used by `follows_economy` is the same one `user_banner` uses (`banners()[ordinal-1]`). The same tests cover it. The up direction was already blocked by the economy-neighbour check. |
| WR-01 stale ordinal on batched rename | Fixed | `coalesce_edits` stable-partitions `RenameSection` to the front. Test `rename_runs_before_a_structural_action_in_the_same_batch`. |
| WR-02 jump into a collapsed section | Fixed | `section_to_reveal` and `reveal_rule` are called on the status-line jump and after row actions. `pending_scroll` is `take()`n by each list draw, so a stale id cannot re-open a section the user later folds. |
| WR-03 fold overwrites unusable settings | Fixed | `settings_unusable` is set at start and on Reload. `save_view_settings` skips the write while it is set. An explicit Config save clears it. See WR-05 for a side effect. |
| WR-04 name-keyed fold state leaks | Fixed | `section_key` prefixes `ff:` and `user:` names with `user:`. The rename, remove and add paths now go through `section_state_after`, which drops or moves the flag. Duplicate-named sections still share one flag by design. |

## Warnings

### WR-05: Any `ff-settings.json` load warning now stops fold clicks from persisting, with no message

**File:** `src/overlay/app.rs:1107` (init), `:1829` (reload), `:1280-1284` (`store_settings`)
**Issue:** `settings_unusable` is set from `settings_warning.is_some()`. `settings::load` also returns a warning for a still-valid JSON object, for example one wrong-typed key such as `reload_after_apply: "yes"`, or a malformed `collapsed_sections`. In that case unknown keys were salvaged, so a save would have been safe. Fold clicks are now kept in memory only, and they are lost on restart. The user sees nothing after the first status line is replaced. The 32-REVIEW-FIX note says a fold stays in memory, but no message tells the user. The behaviour only matches the intent for the "not a JSON object / unreadable" case.
**Fix:** Distinguish the two cases. Either have `load` return a flag (or an enum) saying the file was entirely unusable, or set `settings_unusable` only when `load` took the `unusable(...)` branch. Also post a one-time `list_status` or `config_message` ("Fold state not saved: ff-settings.json could not be used") when a fold is skipped.

### WR-06: `section_state_after` assumes every section action succeeds and addresses pre-batch ordinals

**File:** `src/overlay/app.rs:557-610` (`section_state_after`), `:1484` (`run_row_actions`)
**Issue:** The stored fold state is updated before `apply_row_actions` runs, and it is based on the pre-batch section list. Two consequences:
1. A refused `RemoveSection` or `RenameSection` still drops or moves the stored flag. A folded section then re-opens although it still exists. The new `can_remove` guard makes this unlikely for removal, but the code does not depend on it.
2. Two structural actions in one frame, for example `[RemoveSection(0), RemoveSection(1)]`, apply sequentially against shifted ordinals, but the state update uses the pre-batch ordinals for both. The flag that is dropped may belong to a different section than the one removed.

Both need several clicks in a single frame or a refused action, so the impact is cosmetic (a wrong fold state).
**Fix:** Compute the state change from the result. Compare `working.sections()` before and after `apply_row_actions` and reconcile the stored flags against the section names that still exist. Or move `carry_section_state` after `apply_row_actions` and only act when the document changed.

## Info

### IN-01: Doc comment of `apply_row_actions` is now attached to the wrong function

**File:** `src/overlay/app.rs:555-559`
**Issue:** The two fix commits inserted `section_state_after`, `save_view_settings` and `section_to_reveal` between the doc comment "Applies the frame's collected row actions once, after the list rendered..." and `apply_row_actions`. That comment is now glued to the `section_state_after` docs, and `apply_row_actions` has no doc comment.
**Fix:** Move the three helper functions above that doc comment, or move the comment back onto `apply_row_actions`.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
_Iteration: 2_
