---
phase: 44-auto-detect-styles
fixed_at: 2026-10-23T00:00:00Z
review_path: .planning/phases/44-auto-detect-styles/44-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 44: Code Review Fix Report

**Fixed at:** 2026-10-23
**Source review:** .planning/phases/44-auto-detect-styles/44-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3
- Fixed: 3
- Skipped: 0

`cargo test --locked` is green (1739 lib tests plus the integration suites).

## Fixed Issues

### WR-01: Economy blocks get Auto Detect styles and tags when the economy markers are damaged

**Files modified:** `src/overlay/rules.rs`, `src/overlay/app.rs`, `src/overlay/config_view.rs`, `CLAUDE.md`
**Commits:** 9f490a2, 0c27b4c (CLAUDE.md note)
**Applied fix:**
- `detect_eligible` now refuses every row while `base_economy` is `EconomyView::Damaged`. As a result `detect_inputs` returns nothing and `tag_styles` tags nothing.
- The `AutoDetect` handler does nothing while `working.economy()` is damaged.
- `can_auto_detect` now also requires `!footer.filter_damaged`. The button is greyed with the tooltip `Economy markers damaged.`.
- New tests:
  - `detect_inputs_empty_when_economy_markers_are_damaged` (rules model): a lone begin line above a bronze unique block gives no candidates, tags nothing and leaves the text unchanged.
  - `auto_detect_is_a_no_op_while_economy_markers_are_damaged` (app): no new styles, no tags, nothing dirty.
- The CLAUDE.md Auto Detect note now mentions the greyed state.

### WR-02: The Reload test covers a path users cannot reach, and the recorded "finding" about Reload is wrong

**Files modified:** `src/overlay/app.rs`, `.planning/phases/44-auto-detect-styles/44-03-SUMMARY.md`, `.planning/phases/AUTONOMOUS-V072-DEFERRED.md`
**Commit:** eec6b9a
**Applied fix:**
- Replaced `auto_detect_reload_keeps_rule_edits_and_reloads_config_only` with `auto_detect_bar_reload_drops_tags_keeps_new_styles`. The new test:
  - writes the filter into the test game folder;
  - asserts that `bar_reload_target` gives `BarReload::Shared`;
  - calls `reload_pressed()`;
  - asserts that the tags are gone (text equals the original, rules not dirty) and that exactly `n + 3` styles stay pending, with CONFIG still dirty.
- Corrected the summary's "Reload finding" and the deferred note. Both now describe the actual behaviour: the tags are dropped and the new styles stay pending.
- The deferred note also says that the next Apply would write those styles to `styles.json` with no rule using them, unless Auto Detect is clicked again or Config is reverted.
- CLAUDE.md made no Reload claim, so it needed no correction for this finding.

### WR-03: The second-Apply idempotence test and the naming test cannot fail

**Files modified:** `src/overlay/app.rs`, `src/overlay/style_detect.rs`
**Commit:** 58fa931
**Applied fix:**
- The second Apply's outcome is now asserted to be `AllOutcome::Applied`. It passes: an Apply whose result equals the file on disk still counts as applied.
- The naming test now unwraps `insert_new("custom style 1", ...)` and asserts `names == ["Custom Style 2", "Custom Style 4"]`. The circular `next_name` expectation is gone.

---

_Fixed: 2026-10-23_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
