---
phase: 20-full-grammar-block-editing
fixed_at: 2026-10-13T07:00:00Z
review_path: .planning/phases/20-full-grammar-block-editing/20-REVIEW.md
iteration: 3
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 20: Code Review Fix Report (iteration 3, final)

**Fixed at:** 2026-10-13T07:00:00Z
**Source review:** .planning/phases/20-full-grammar-block-editing/20-REVIEW.md (iteration 3)
**Iteration:** 3 (final, no re-review follows)

**Summary:**
- Findings in scope: 1 (WR-01, critical_warning scope)
- Fixed: 1
- Skipped: 0
- Not in scope: IN-03, IN-04 and IN-06 (Info, carried and deferred)

Final state: `cargo fmt --check` passes and the full `cargo test` run is green (648 lib tests plus every integration test, including `artifact_guard` and `generate_snapshot`). No fixture changed. No edit normalizes an operator (PARSE-04).

## Fixed Issues

### WR-01: Two pending Rarity rows on one rule share an egui id, so one chip click writes two lines

**Files modified:** `src/overlay/body.rs`
**Commit:** 91da445
**Status:** fixed: requires human verification (logic fix)
**Applied fix:** A block now holds at most one pending row per keyword, whatever the operator.
- `pick_condition` checks a new `has_pending(list, block, kw)` helper, which matches on block and keyword name only. Before, it used `PendingRow` equality, which includes `op`. If the keyword is already pending, the pick changes nothing, so a carried `!=` from a refill is kept.
- `rarity_control` calls a new `queue_refill(list, p)` on the `RarityEdit::Remove` path. It removes any pending row for the same block and keyword, then pushes the refill, so the refill's operator replaces the existing one.

New regression test `one_pending_row_per_keyword_and_block` covers three sequences:
- `Rarity != Normal`: refill, then pick Rarity. Leaves one row, and it keeps `Ne`.
- A range rule (`>= Magic`, `<= Rare`): refill on both lines. Leaves one Rarity row with `Le`, and a pending Class row on the same block is not touched.
- Pick Rarity, then refill. Leaves one row with the refill's `Ge`.

The first sequence leaves two rows on the old code.

**Defence-in-depth ids, not applied:** I did not add the pending index or the operator to the `push_id`/`bad_id` salts.
- `(block, keyword)` is now unique among pending rows, so the current salts are already unique.
- The index (`real + i`) changes whenever a real line is added or removed. Using it would drop a pending card's typed list text, and its refusal hint, whenever another card changed.
- Adding the operator brings back no such problem, but it no longer protects against anything.

## Earlier iterations (brief)

- **Iteration 1:** 8 findings in scope, all fixed. The changes:
  - CR-01: adding a rarity chip kept a negated `!=`
  - WR-01: `Continue` was unlocked on styled rules
  - WR-02: a styled rule that cannot be rewritten no longer blocks Apply
  - WR-03: the style-lock bypass was closed
  - WR-04: an alpha-0 colour keeps its RGB
  - WR-05: a mod condition added from the picker has no count
  - WR-06: a refused first entry shows a hint
  - WR-07: card state is keyed per epoch
  - Optional IN-01 and IN-02 were also fixed.
- **Iteration 2:** 3 findings fixed:
  - WR-01: the rarity refill carries the removed operator (`e497f1b`)
  - IN-05: `rewrite_block` is now an all-or-nothing snapshot restore (`23e9997`)
  - IN-07: keyword-shifting edits bump the card-state epoch (`e4c2fc4`)
- **Iteration 3:** WR-01 above, a regression from the iteration-2 WR-01 fix.

---

_Fixed: 2026-10-13T07:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 3_
