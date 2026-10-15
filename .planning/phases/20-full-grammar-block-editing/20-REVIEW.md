---
phase: 20-full-grammar-block-editing
reviewed: 2026-10-13T04:54:53Z
depth: standard
iteration: 3
files_reviewed: 15
files_reviewed_list:
  - src/emitter.rs
  - src/filter/edit.rs
  - src/filter/lineedit.rs
  - src/filter/structure.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/body.rs
  - src/overlay/classes.rs
  - src/overlay/grammar.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/widgets.rs
  - src/styled.rs
  - tests/filter_roundtrip.rs
  - tests/overlay_rules.rs
findings:
  critical: 0
  warning: 1
  info: 3
  total: 4
status: issues_found
---

# Phase 20: Code Review Report (iteration 3)

**Reviewed:** 2026-10-13T04:54:53Z
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

This pass checked the iteration-2 fix commits `e497f1b` (WR-01), `23e9997` (IN-05) and `e4c2fc4` (IN-07), looked for regressions they introduced, and re-ran the suite. `cargo test` is green: 647 lib tests plus every integration test, including `artifact_guard` and `generate_snapshot`. Clippy reports only pre-existing style lints.

**Verification of previous findings**

| ID | Verdict | Notes |
|----|---------|-------|
| WR-01 | Resolved, with one regression (WR-01 below) | `rarity_refill` carries the removed operator, and `pending_first_entry` drops only an ordering operator on a multi-rarity line, which matches `rarity_edit`. The pending card shows the operator label. The six-case test passes end to end through `WorkingFilter`. Because the carried operator is part of `PendingRow` equality, the existing dedupe no longer stops two pending Rarity rows on the same block (see below). |
| IN-05 | Resolved | `rewrite_block` snapshots the document before the keyword switch and restores the snapshot if the splice is refused. `splice_impl` runs `check_text` and the structural checks, and only then calls `commit_owned`, so the keyword switch is the only change that can need undoing. `rewrite_styled` iterates block ids computed before the loop, and the restored snapshot keeps those ids, so later rules are unaffected. The new lowercase-`hide` test is correct. |
| IN-07 | Resolved | `shifts_ordinals` covers `RemoveLine`, every `AddLine`, and a `Whole` edit whose keyword changes (case-insensitive, `#`/indent-aware). The epoch is bumped only when the outcome changed. `focus_line` is still set for an added line and is read after the new ids render, so focus on the new card still works. |
| IN-03, IN-04, IN-06 | Deferred | Carried below as Info. |

**Invariants re-checked**
- **PARSE-04.** No edit normalizes an operator unless the user picks one. The rarity refill keeps `!=`, `!`, `=` and `==`.
- **Style rewrite.** It runs on a clone, is all-or-nothing per rule and byte-exact on refusal, skips economy and disabled blocks, and reports unknown and refused rules.
- **EOL and empty lines.** Spliced lines take the EOL of their anchor line, and no empty keyword line is produced.
- **Game folder.** Nothing writes to the game folder.

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: Two pending Rarity rows on one rule share an egui id, so one chip click writes two lines

**File:** `src/overlay/body.rs:526-533` (`rarity_refill`), `src/overlay/body.rs:511-519` (`pick_condition`), `src/overlay/body.rs:589` (`pending_card` push_id), `src/overlay/body.rs:942-945` (`rarity_control`)
**Issue:** Before `e497f1b`, every pending Rarity row had `op: None`, so the `list.pending.contains(&p)` checks in `pick_condition` and `rarity_control` kept at most one pending Rarity row per block. `PendingRow` derives `PartialEq` over `op`, and the refill now carries `entry.op`. Two pending Rarity rows with different operators can therefore exist on the same block:
- A range rule such as `Rarity >= Magic` and `Rarity <= Rare`. Unchecking the last chip on both lines queues pending rows with `Some(Ge)` and `Some(Le)`.
- `Rarity != Normal` with its last chip unchecked (pending `Some(Ne)`), followed by picking **Rarity** again from `+ condition`. The condition picker passes `&|_| None` (line 296), so nothing disables it, and the new row has `op: None`.

Every pending card is wrapped in `ui.push_id((p.block, p.keyword.name, "pending"), ..)`, and the `bad_id` is built the same way. Two such rows get identical ids for the card and for every chip inside it, because `toggle_chip` uses `allocate_exact_size` auto-ids under that parent. In egui 0.36, `Response::clicked` is computed as `Some(id) == interact_widgets.clicked` (`context.rs:1512`). A click on "Magic" in one card therefore reports `clicked` on the matching chip in both cards, and each card pushes its own `AddLine`. The second example writes `Rarity != Magic` and `Rarity Magic` into the same block, a contradictory pair that matches nothing, and the user gets no warning. Debug builds also paint egui's id-clash warning over the cards.
**Fix:** Keep at most one pending row per `(block, keyword)`, whatever the operator. The refill's operator should take over from any row already pending:
```rust
// rarity_control, RarityEdit::Remove arm
let p = rarity_refill(card.id, card.kw, card.entry);
list.pending.retain(|q| !(q.block == p.block && q.keyword.name == p.keyword.name));
list.pending.push(p);

// pick_condition
if !list.pending.iter().any(|q| q.block == id && q.keyword.name == kw.name) {
    list.pending.push(p);
}
```
Another option is to add the pending index or the operator to the `push_id` and `bad_id` salts. Deduplicating is still the better fix, because two identical-looking "Rarity" pending cards would confuse the user even with distinct ids. Add a test: after `rarity_refill` for `Rarity != Normal`, `pick_condition(Rarity)` must leave `list.pending.len() == 1`.

## Info

### IN-03 (carried): `body_view` re-implements the disabled-line offset

**File:** `src/overlay/rules.rs` (`body_view`, the `text.find('#')` offset)
**Issue:** `body_view` finds the offset with `text.find('#')`, while `lineedit::grammar_offset` and `structure::header_parts` use `disabled::uncomment_once`. The two agree today, but the token base is defined in two places.
**Fix:** Expose one shared helper, for example `Document::grammar_offset`, and call it from both places.

### IN-04 (carried): An integration test asserts a wall-clock limit

**File:** `tests/overlay_rules.rs` (`fixture_line_edits_stay_fast_and_revert_restores_bytes`)
**Issue:** The 15 s bound depends on machine load and on the build profile. The `filter_roundtrip`/overlay integration binaries took about 23 s in this run, so the margin on a busy runner is small.
**Fix:** Move the timing assertion into an `#[ignore]` benchmark and keep the revert/bytes assertion.

### IN-06 (carried): The colour picker loses hue on greys, black and white

**File:** `src/overlay/widgets.rs` (`colour_swatch`)
**Issue:** `colour_swatch` rebuilds the `Hsva` from `u8` RGBA every frame. On an achromatic colour, a hue drag changes no RGB value, so nothing is committed and the hue resets on the next frame. No data is corrupted.
**Fix:** While the popup is open, keep the in-progress `Hsva` in egui temp memory keyed by the swatch id. Reuse it while its `to_srgba_unmultiplied()` still equals the line's RGBA.

---

_Reviewed: 2026-10-13T04:54:53Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
