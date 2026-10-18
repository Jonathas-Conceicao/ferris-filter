---
phase: 38-built-in-styles-strictness-reset
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/38-built-in-styles-strictness-reset/38-REVIEW.md
iteration: 2
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 38: Code Review Fix Report

**Fixed at:** 2026-10-18
**Source review:** .planning/phases/38-built-in-styles-strictness-reset/38-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (WR-01..WR-04; Info findings out of scope)
- Fixed: 4
- Skipped: 0

Fixes were committed directly in the milestone-closeout worktree as instructed (no nested
review-fix worktree or merge). Tests ran with `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` set
to an empty temp dir.

## Fixed Issues

### WR-01: "Already at the defaults." tooltip on both Reset buttons never shows

**Files modified:** `src/overlay/config_view.rs`, `src/overlay/shell.rs`
**Commit:** efc9cb6
**Applied fix:** Both Reset buttons now pick the tip text by state and attach it with
`on_hover_text(tip).on_disabled_hover_text(tip)`, since `ff_button_ex` greys a disabled button
inside an enabled Ui (the response still counts as enabled). Verified: build, `overlay::shell` and
`overlay::config_view` tests (101 passed). The tooltip itself is a UI behaviour: confirm by hovering
a greyed Reset button.

### WR-02: `reset_tiers` clears every tier error without re-validating

**Files modified:** `src/overlay/config_state.rs`
**Commit:** 85cc7ed
**Applied fix:** `reset_tiers` copies the defaults as before, keeps the file order when the
defaults validate in it (so the existing `reset_tiers_keeps_order_and_clears_errors` test still
holds), and otherwise sorts the entries into the default order; it then calls
`recompute_tier_errors()` after `resync_buffers()`. `tiers_at_defaults` also requires no edge and no
general error. New test `reset_tiers_takes_default_order_when_file_order_would_not_validate` uses
the review's mixed-unit example. Status: fixed: requires human verification (logic change: the
order fallback is a judgement call between the two options the review offered).

### WR-03: `old_twelve_style_file_loads_and_keeps_its_bytes` asserts nothing on one branch

**Files modified:** `src/overlay/config_state.rs`
**Commit:** 78ddf24
**Applied fix:** Replaced the `if !w.styles_changed()` guard with an unconditional
`assert!(!w.styles_changed(), ...)` followed by the byte comparison. Test passes.

### WR-04: `shipped_styles_round_trip_through_render_styles` fails on a CRLF checkout

**Files modified:** `src/defaults.rs`
**Commit:** 59f57ee
**Applied fix:** Chose to normalise in the test (`STYLES_DEFAULT.text.replace("\r\n", "\n")`)
rather than add a `.gitattributes` `eol=lf` entry: `.gitattributes` has no existing per-file
eol pin for the JSON defaults (its `-text` entries are for parser fixtures), and the test change
touches no tracked file bytes. `defaults` tests pass (24).

---

_Fixed: 2026-10-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

_Iteration 2 re-review (38-REVIEW.md): clean, WR-01..WR-04 confirmed fixed; Info IN-01..IN-05 left as advisory._
