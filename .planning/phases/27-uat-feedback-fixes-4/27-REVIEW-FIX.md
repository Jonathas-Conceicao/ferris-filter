---
phase: 27-uat-feedback-fixes-4
fixed_at: 2026-10-15T00:00:00Z
review_path: .planning/phases/27-uat-feedback-fixes-4/27-REVIEW.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 27: Code Review Fix Report

**Fixed at:** 2026-10-15
**Source review:** .planning/phases/27-uat-feedback-fixes-4/27-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 2 (WR-01, WR-02; fix scope critical_warning, so IN-01..IN-05 were not attempted)
- Fixed: 2
- Skipped: 0

Verification: `cargo fmt --check` is clean. `cargo test --offline --lib overlay::` gave 502 passed and 0 failed. This ran in an isolated worktree without the author's local `styles.json`/`tiers.json` edits, so the 5 baseline failures do not appear there. No new clippy warnings in the touched files.

## Fixed Issues

### WR-01: A Defences card edited to zero-based lines flips to Simple Defences right after APPLY

**Files modified:** `src/overlay/rule_list.rs`, `src/overlay/app.rs`, `src/overlay/body.rs`, `CLAUDE.md`, `CHANGELOG.md`, `.planning/phases/27-uat-feedback-fixes-4/27-04-SUMMARY.md`
**Commit:** ffbb3a9
**Status:** fixed: requires human verification (state-handling change)
**Applied fix:**
- Added `ListUi::kept_positions` and `ListUi::rekey_after_apply`, plus a `KeptPositions` struct. Together they carry the expanded row and the `advanced_defences` markers across the Apply re-parse by 1-based row position.
- `run_apply` now uses them instead of its inline expanded-row remap. `rekey_after_apply` still calls `clear_ids` first, so everything else is forgotten as before. Revert and RELOAD still call `clear_ids` alone, so they classify from the file again.
- Tests:
  - `loaded_defences_card_edited_to_zero_based_stays_defences` now asserts that the card stays Defences after a simulated Apply re-parse, then flips to Simple only after `clear_ids` (reload).
  - Added `a_defences_card_toggled_on_stays_defences_after_apply` (the reviewer's scenario: Defences, toggle Armour on, Apply, still `"defences"`).
  - Added `a_simple_classified_rule_stays_simple_after_apply`.
  - Added `rekey_after_apply_carries_expanded_and_defences_markers_by_position`, which covers positions past the end and no document.
- Docs:
  - CLAUDE.md overlay notes: Apply keeps the card; only Revert, RELOAD or removing the card re-classify.
  - CHANGELOG: "does not flip while editing or on Apply".
  - 27-04-SUMMARY: notes the WR-01 change.
- Note: the tests simulate the re-parse with `WorkingFilter::from_text` because `run_apply` needs a full app (window, root). Please confirm in the overlay: Defences, toggle Armour on, APPLY, and the open card stays Defences.

### WR-02: Simple Defences Has<->No rewrites through `LinePatch::Whole`, dropping trailing comments

**Files modified:** `src/overlay/body.rs`, `src/overlay/conditions.rs`
**Commit:** cbcdedc
**Applied fix:**
- In `simple_seg_actions`, the `SegWrite::Replace` arm now emits `LinePatch::Both(Some(Operator::Gt), "0")` for Has and `LinePatch::Both(None, "0")` for No on the existing line. Before, it emitted `LinePatch::Whole`. As a result, the keyword's spelling and any trailing comment are kept, the commit is strict, and the keyword-casing ordinal-shift path in `app.rs` is no longer hit.
- Updated the `SegWrite::Replace` doc comment.
- Tests:
  - Updated `simple_segment_writes_touch_only_their_own_line` to expect the `Both` patch.
  - Added `simple_has_no_switch_keeps_trailing_comment_and_keyword_spelling`: `BaseArmour >= 1 # keep` switched to No gives `BaseArmour 0 # keep`, and `baseevasion == 0 # $tier->ev` switched to Has gives `baseevasion > 0 # $tier->ev`. The untouched line stays byte-identical.

---

_Fixed: 2026-10-15_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

## Author decision (2026-10-15)

Recorded at the v0.5 wrap-up.

Remaining Info findings in 27-REVIEW.md (IN-01 to IN-05) are accepted as tech debt.
