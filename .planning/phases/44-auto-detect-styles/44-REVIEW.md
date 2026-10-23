---
phase: 44-auto-detect-styles
reviewed: 2026-10-23T00:00:00Z
depth: standard
iteration: 2
files_reviewed: 4
files_reviewed_list:
  - src/overlay/style_detect.rs
  - src/overlay/rules.rs
  - src/overlay/config_view.rs
  - src/overlay/app.rs
findings:
  critical: 0
  warning: 0
  info: 4
  total: 4
status: clean
---

# Phase 44: Code Review Report (re-review, iteration 2)

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 4
**Status:** clean

## Narrative Findings (AI reviewer)

## Summary

This pass reviewed `git diff f340174..HEAD -- src`, the fix commits
9f490a2, 0c27b4c, eec6b9a and 58fa931. It checked them against the
iteration-1 findings WR-01..WR-03. The targeted tests ran in a temp
target dir (`cargo test --locked --lib detect`): 23 passed, 0 failed.

**WR-01 (economy blocks touched when markers are damaged): resolved.**
- The model now has a guard. `detect_eligible` (`rules.rs:1439-1445`)
  refuses every row while `base_economy` is `EconomyView::Damaged`. This
  empties `detect_inputs`, and `tag_styles` then tags nothing, because
  it goes through the same eligibility check. The new test
  `detect_inputs_empty_when_economy_markers_are_damaged` confirms it:
  `tag_styles` returns 0 and the text is unchanged.
- The `AutoDetect` handler (`app.rs:3063-3065`) has the same guard,
  through the projected `working.economy()`. Projected and base differ
  only when an op is staged, and an op cannot be staged on a damaged
  base, because `stage_add` works only from Absent. So the two guards
  agree.
- The button: `can_auto_detect` (`app.rs:3319-3321`) now requires
  `!footer.filter_damaged`. That flag is
  `filter_state() == FilterState::Damaged`, which comes from the same
  `EconomyView::Damaged`, so the button and the model use one source of
  truth. The tooltip branch (`config_view.rs:867`) is reachable only
  when a filter is loaded, so the text `Economy markers damaged.` is
  accurate.
- The app-level no-op test checks every effect: no styles, no tags,
  rules clean, config clean.

**WR-02 (Reload tested on an unreachable path): resolved.**
- The new test `auto_detect_bar_reload_drops_tags_keeps_new_styles`
  writes the filter into the temp game folder. It asserts
  `bar_reload_target(Config, ∅) == BarReload::Shared` and drives the
  real `reload_pressed()`.
- Its assertions are now exact: text equals the original, rules are not
  dirty, `names().len() == n + 3`, and config is dirty. The weak `<= n`
  assertion is gone.

**WR-03 (tests that cannot fail): resolved.**
- The second Apply's outcome is now asserted to be
  `AllOutcome::Applied`.
- The naming test unwraps `insert_new("custom style 1", ...)` and
  asserts exactly `["Custom Style 2", "Custom Style 4"]`. This would
  fail on a skipped N=2 or on a reused case-variant name. The circular
  `next_name` expectation was removed.

**No new Critical or Warning issues were introduced.** One small point,
not worth a finding: the handler's damaged guard returns silently after
`clear_events_on_edit()` with no status message. The button is greyed
in that state, so users cannot reach this path.

The four Info items from iteration 1 were out of scope for the fix pass
and are still open. They are listed below unchanged, except that IN-03
now also covers the Reload behaviour documented by the WR-02 fix.

## Info

### IN-01: Rules with grammar errors are skipped silently, a gap in the CONTEXT scope

**File:** `src/overlay/rules.rs:1439-1445`
**Issue:** `detect_eligible` skips enabled rules with any grammar
error, including errors in conditions only. CONTEXT lists only tagged,
`can't enable` and economy rules as skipped. Neither the status line
nor the count says these rules were left out. The impact is small: the
error blocks Apply anyway, and after fixing it a second click tags the
rule.
**Fix:** Record the skip in CONTEXT/CLAUDE.md, or skip only rules
whose errors fall on action lines.

### IN-02: Auto Detect reads a 3-token background as alpha 240, but the Custom rule editor shows it as 255

**File:** `src/overlay/style_lines.rs:107-113` vs `src/overlay/body.rs`
(`rule_style_view` -> `grammar::decode` sets `rgba[3] = 255`)
**Issue:** After Auto Detect, the new style's editor shows alpha 240
for a background that the rule's Custom editor showed at 255 a moment
earlier.
**Fix:** Make the Custom rule view display a 3-token background as 240,
or document the difference.

### IN-03: Pending styles can end up with no rule using them, and the status still reports Ok

**File:** `src/overlay/app.rs:3063-3090`, `src/overlay/rules.rs:1450-1480`
**Issue:** Styles are added before tagging. There are two ways to end
up with pending styles that no rule uses:
- `tag_styles` skips an entry. The status still says
  `Auto Detect: N styles, M rules tagged` at `StatusKind::Ok`, even
  when M is 0.
- The bottom-bar Reload drops the tags but keeps the new styles. This
  is the per-tab behaviour, now documented in the deferred note and
  tested by the WR-02 fix. The next Apply writes those styles into
  `styles.json` with no rule using them.

Both are consistent with the decisions and unlikely in practice.
**Fix:** Optionally, after tagging, delete any added style that no
tagged rule uses, and report Info `NOTHING_NEW` when the tag count is 0.

### IN-04: Tagging couples ordinary user rules to the strictness control and to visibility rewrites

**File:** `src/overlay/style_detect.rs:53-90`
**Issue:** A user rule tagged with a built-in from S to F then follows
the Economy Visibility strictness control. For a compact header such as
`Hide Continue`, the keyword switch fails at Apply
(`StyleReport.failed`). Neither the tooltip nor the CLAUDE.md note
mentions either behaviour.
**Fix:** Mention it in the Auto Detect CLAUDE.md note, or skip
compact-header rows in `detect_inputs`.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
