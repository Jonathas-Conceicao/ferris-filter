---
phase: 20-full-grammar-block-editing
reviewed: 2026-10-13T04:44:44Z
depth: standard
iteration: 2
files_reviewed: 19
files_reviewed_list:
  - src/emitter.rs
  - src/filter/edit.rs
  - src/filter/lineedit.rs
  - src/filter/mod.rs
  - src/filter/structure.rs
  - src/lib.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/body.rs
  - src/overlay/classes.rs
  - src/overlay/grammar.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/theme.rs
  - src/overlay/widgets.rs
  - src/styled.rs
  - tests/filter_roundtrip.rs
  - tests/overlay_rules.rs
findings:
  critical: 0
  warning: 1
  info: 5
  total: 6
status: issues_found
---

# Phase 20: Code Review Report (iteration 2)

**Reviewed:** 2026-10-13T04:44:44Z
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found

## Summary

This pass re-checked the fix commits `0a84a2c..c7d9833` against every finding from iteration 1, looked for regressions they introduced, and re-ran the suite. `cargo test` passes: 644 lib tests plus every integration test, including `artifact_guard` and `generate_snapshot`. The only clippy warnings in the touched files are style lints in test code.

**Verification of previous findings**

| ID | Verdict | Notes |
|----|---------|-------|
| CR-01 | Resolved | `rarity_edit` now drops only `<` `<=` `>` `>=`, so `!=`, `!`, `=` and `==` are kept. List-form labels are "Not any of" / "Not any of (!)", and the note reads "exclude all of them". A test covers this. |
| WR-01 | Resolved | The Continue checkbox is always enabled. The model never locked `SetContinue`. A click-sweep test covers it. |
| WR-02 | Resolved, with one edge (IN-05) | A refused rule is recorded in `StyleReport::failed` and left byte-identical while the other rules are still rewritten, and the Apply message names the rule. The undo is not byte-exact in one corner case, described in IN-05. |
| WR-03 | Resolved | `AddLine` checks the declared section against `text_kind`. A `Whole` edit that would create an action line on a known-styled rule is refused. Values, Operator and Both patches cannot change the keyword, so they cannot bypass the lock. |
| WR-04 | Resolved, with one cosmetic regression (IN-06) | Colours are edited as unmultiplied HSVA built from linear RGB. The round trip is exact at every alpha (tested). |
| WR-05 | Resolved | A `CountThenNames` line from the pending card has no count, and the model accepts it. |
| WR-06 | Resolved | `chip_list` refuses unclean quotes, quoted entries and control characters, and keeps the draft. `pending_card` shows `BAD_TEXT_HINT`. |
| WR-07 | Mostly resolved (IN-07) | The epoch salts every row's `push_id`. It is bumped on `clear_ids`, on collapse or switch, and on a successful `RemoveLine`. Raw drafts only persist while focused, so stale drafts cannot leak. |
| IN-01, IN-02 | Resolved | |
| IN-03, IN-04 | Deferred | Carried below as Info. |

**Invariants re-checked**
- **PARSE-04, EOL and empty lines.** No edits normalize operators unless the user picks one. Each line keeps its own EOL (splice and keyword edits reuse the line's `eol()`). There are no empty keyword lines.
- **Style rewrite.** It runs on a clone, skips economy and disabled blocks, is idempotent, reports unknown styles, and now also reports refused rewrites.
- **Locks and inputs.** The style lock is enforced in both the model and the view. Class groups come from `config.json`. Idle frames emit no `RowAction` (the colour idle test now includes alpha 0).
- **Game folder.** Nothing writes to the game folder.

One new Warning: removing the last rarity chip and then picking another one silently drops a negated or ordering `Rarity` operator. It is the same kind of meaning flip as CR-01, reached through a different click sequence.

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: Unchecking the last rarity chip and picking another loses a negated or ordering operator

**File:** `src/overlay/body.rs:911-921` (`rarity_control`), `src/overlay/body.rs:524-552` (`pending_first_entry`)
**Issue:** When the chip selection becomes empty, `rarity_control` removes the line and queues a replacement `PendingRow` with `op: None`:
```rust
RarityEdit::Remove(a) => {
    actions.push(a);
    let p = PendingRow { block: card.id, keyword: card.kw, op: None, kind: LineKind::Condition };
```
The pending card shows only rarity chips, with no operator control. `pending_first_entry` then writes `Rarity <name>` without the old operator.

Take `Rarity != Normal`, meaning "anything but Normal". A user who wants to exclude Magic instead will often uncheck Normal first and then check Magic. That sequence writes `Rarity Magic`, which shows only Magic items, the opposite of what they intended. Ordering operators are lost the same way: `Rarity >= Rare`, uncheck Rare, check Magic gives `Rarity Magic` instead of `Rarity >= Magic`.

Only the order of the clicks differs from CR-01. Checking Magic first and then unchecking Normal correctly gives `Rarity != Magic`. The pending card never shows the operator it is about to drop, so nothing warns the user.
**Fix:** Carry the removed line's operator into the pending row, and apply the same rule as `rarity_edit`: drop only an ordering operator when two or more rarities are picked.
```rust
// rarity_control
let p = PendingRow { block: card.id, keyword: card.kw, op: card.entry.op, kind: LineKind::Condition };

// pending_first_entry, Rarity arm
let op = match p.op {
    Some(Operator::Lt | Operator::Le | Operator::Gt | Operator::Ge) if items.len() >= 2 => None,
    o => o,
};
```
Also show the operator label (`grammar::operator_label`) on the pending card when `p.op` is set. Add a test: `Rarity != Normal`, uncheck Normal, then pick Magic must give `Rarity != Magic`.

## Info

### IN-03 (carried): `body_view` re-implements the disabled-line offset

**File:** `src/overlay/rules.rs:675-683`
**Issue:** `body_view` uses `text.find('#')`, while `lineedit::grammar_offset` and `structure::header_parts` use `disabled::uncomment_once`. They agree today, but there are two definitions of the token base.
**Fix:** Expose one shared helper (for example `Document::grammar_offset`) and use it in both places.

### IN-04 (carried): An integration test asserts a wall-clock limit

**File:** `tests/overlay_rules.rs` (`fixture_line_edits_stay_fast_and_revert_restores_bytes`)
**Issue:** `started.elapsed() < Duration::from_secs(15)` depends on machine load and on the build profile. In this run the suite took 23 s, so the margin is not large on a busy runner.
**Fix:** Move the timing assertion into an `#[ignore]` benchmark and keep the revert/bytes assertion.

### IN-05: The undo in `rewrite_block` restores a canonical keyword, not the original bytes

**File:** `src/styled.rs:117-130`, `src/styled.rs:154-158`
**Issue:** If `splice_body` fails after the keyword switch, the undo calls `set_block_keyword(id, old_kw)`. That writes the literal `Show`/`Hide`/`Minimal`. Keyword lookup is case-insensitive (`keywords.rs:184`), so a header written `hide` comes back as `Hide`. `rewrite_styled` then sees changed bytes and returns `Err`, and Apply is refused for the whole filter with the generic message. That is exactly the behaviour WR-02 removed. Reaching it needs a refused splice on a lowercase header, so it is rare.
**Fix:** Make the rule atomic by snapshotting instead of reversing the edit:
```rust
let snapshot = doc.clone();
match rewrite_block(doc, id, style) {
    Err(e) => { *doc = snapshot; report.failed.push((i, e.to_string())); }
    ...
}
```
This also removes the "could not undo" branch.

### IN-06: The colour picker loses hue on greys, black and white

**File:** `src/overlay/widgets.rs:445-460`
**Issue:** `colour_swatch` rebuilds the `Hsva` from the line's `u8` RGBA every frame, and drops the colour cache that `color_edit_button_srgba` kept. On an achromatic colour (`s == 0` or `v == 0`, such as `0 0 0` or `40 40 40`), dragging the hue slider leaves the RGB unchanged. Nothing is committed, and the next frame resets the hue to 0. The user has to raise saturation first, which starts from red, and only then pick a hue. No data is corrupted. The fixer already flagged this for manual verification.
**Fix:** Keep the in-progress `Hsva` in egui temp memory, keyed by the swatch id, while the popup is open. Reuse it when its `to_srgba_unmultiplied()` still equals `rgba`, and rebuild it otherwise.

### IN-07: A raw edit that changes a line's keyword still shifts card ordinals without bumping the epoch

**File:** `src/overlay/app.rs:305-309`, `src/overlay/rules.rs:695-698`
**Issue:** The epoch is bumped only for `RemoveLine`. Two other edits also shift keyword ordinals:
- A `LinePatch::Whole` raw edit that changes the keyword, for example fixing `BaseTyp "A"` to `BaseType "A"` when another `BaseType` line comes after it.
- An `AddLine` of an opaque or unknown line inserted right after the header when the rule has no conditions yet.

In both cases a later card's `bad-text`, `bad-int` or `Named` temp state moves to its neighbour. Drafts are focus-scoped, so they do not leak. The impact is a stale hint or toggle, nothing more.
**Fix:** Bump the epoch after any successful `EditLine` with `LinePatch::Whole` (or whenever the keyword at `line` changed), and after any successful `AddLine`. Alternatively, key card state by `(block, keyword, line text hash)` rather than by ordinal.

---

_Reviewed: 2026-10-13T04:44:44Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
