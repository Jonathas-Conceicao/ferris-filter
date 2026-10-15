---
phase: 24-uat-feedback-fixes
fixed_at: 2026-10-13T00:00:00Z
review_path: .planning/phases/24-uat-feedback-fixes/24-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 2
skipped: 1
status: partial
---

# Phase 24: Code Review Fix Report

**Fixed at:** 2026-10-13
**Source review:** .planning/phases/24-uat-feedback-fixes/24-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (Critical + Warning; the 6 Info findings are out of scope)
- Fixed: 2
- Skipped: 1 (by decision, see below)

Verification: `cargo test` passes (834 lib tests plus all integration targets, including
`artifact_guard` and `generate_snapshot`). `cargo fmt --check` is clean. Nothing under `tests/`
changed, so the fixtures stay byte-identical.

## Fixed Issues

### WR-01: Clicking the summary text does not expand the row (the selectable label takes the click)

**Files modified:** `src/overlay/rule_list.rs`
**Commit:** 2162a94
**Applied fix:**
- The summary strip is now allocated with `Sense::hover()`.
- The summary label is `.selectable(false)`.
- The click target (`ui.interact(srect, "ff-summary-click", Sense::click())`) is registered
  *after* the label, so it is the topmost hit. This is the same pattern the header uses with
  `row_click`.
- The summary tooltip moved onto that response.
- New headless test `clicking_the_summary_text_expands_the_row`:
  - It uses real fonts, so the label actually covers the point being clicked.
  - It sends hover, press and release on separate frames.
  - A click on the blank area past the text is a control case.
  - A click on the summary text must set `list.expanded`.
  - Checked against the unfixed code: the text click fails there and the control passes.

### WR-03: The row-height tests are tautological and never measure painted geometry

**Files modified:** `src/overlay/rule_list.rs`
**Commit:** f5583c8
**Applied fix:**
- `draw_row` now sets `item_spacing.y = 0.0` itself, so the height invariant no longer depends
  on the caller. `show_list` still sets it as well, and since the style is inherited the
  behaviour does not change.
- New test `painted_row_heights_match_the_virtual_list`:
  - It calls `draw_row` in a plain UI with the default (non-zero) item spacing.
  - It measures the rect of the per-row `push_id` scope.
  - It asserts that a collapsed row spans `row_height(ui)`, and that an expanded row (once
    `expanded_h` settles) spans `list.expanded_h`.
  - Checked by removing the guard: the test fails (41 vs 38, the 4px gap the review describes).
- The older arithmetic tests are left as they are.

## Skipped Issues

### WR-02: A style JSON carrying `Minimal` still fails to load (UAT-04 requirement not met)

**File:** `src/models/config.rs:124-127`
**Reason:** Skipped by decision. `StyleAction` has never had a `Minimal` variant, so this is a
gap from before this phase, not a phase-24 regression. Nothing in the app writes `"Minimal"` into
a style. The fix would mean adding a `StyleAction::Minimal` variant and mapping it through
`rewrite_block`/`styled.rs`. That changes the `config.json`/`styles.json` model and the
generator path, which phase 24 must leave byte-identical (its boundary rules out generator
changes), so `src/models/config.rs` was not touched. The UAT-04 clause "if a style JSON somehow
carries `Minimal`, keep loading it" is still unmet. A hand-edited `styles.json` with
`"action": "Minimal"` is rejected as a whole. This needs to be recorded as a known gap, or
scheduled as a model change in a later phase.
**Original issue:** `"action": "Minimal"` fails serde in `loader::load_styles`, which rejects the
whole `styles.json`. Every styled rule then becomes `Unchecked` and the CONFIG Styles tab is
empty.

---

_Fixed: 2026-10-13_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

## Author decision (2026-10-15)

Recorded at the v0.5 wrap-up.

- **WR-02** (a style with `"action": "Minimal"` fails to load): deferred to backlog 999.14 (Minimal support review for PoE2).
- Remaining Info findings in 24-REVIEW.md (IN-01 to IN-06) are accepted as tech debt.
