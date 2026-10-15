---
phase: quick-261015-nbp-simple-defences-multi-select-base-types
reviewed: 2026-10-15T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - src/filter/group.rs
  - src/filter/structure.rs
  - src/overlay/rules.rs
  - src/dynamic.rs
  - src/styled.rs
  - src/overlay/body.rs
  - src/overlay/rule_list.rs
  - src/overlay/conditions.rs
  - src/overlay/app.rs
findings:
  critical: 1
  warning: 6
  info: 5
  total: 12
status: issues_found
---

# Quick 261015-nbp: Code Review Report

**Reviewed:** 2026-10-15
**Depth:** standard (some findings checked with throwaway probe tests run on a scratch copy of HEAD; no repo source was modified)
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Range 65a3c67..233964b (base 3b45445): the cover algorithm, group analysis, resync, set_selection and duplicate in `filter/group.rs`; `replace_blocks_after` and `swap_runs` in `structure.rs`; group dispatch in `WorkingFilter`; follower-aware counting in `dynamic.rs` and `styled.rs`; and the chip card.

These parts hold up:
- **Rollback on failure.** In `WorkingFilter::dispatch` and `set_selection`, every group operation clones the document first and restores it on refusal.
- **Line endings.** CRLF and a missing final newline survive grow and shrink. Probe: growing the last rule of a CRLF, no-final-newline file to 3 blocks and back gives the original bytes.
- **Economy section.** Growing the last rule before the section leaves the markers intact.
- **Disabled groups.** They re-cover, duplicate and enable correctly.
- **Counting.** Rule numbering in `blocking_errors`, `rule_index_of` and the style report counts a group once.

The issues found:
1. **The chips do not mean what they say (critical).** The cover ignores items that have no defence at all. So "Full AR", "Full ES", "No EV" and similar picks also match weapons, jewellery and currency.
2. **A group can break silently.** Adding a defence keyword line to a group leader, or acting on a follower's block id, breaks the group without any message.
3. **Stale card state.** `simple_all` outlives the pending card it belongs to.
4. **Report failures are swallowed.** In the styled rewrite, a follower's rewrite failure is ignored and can leave a broken group on disk.
5. **The `group.rs` module doc overstates rollback.** It claims more than the multi-step functions provide.

## Critical Issues

### CR-01: The chips leave out "no defence at all", so "Full AR/EV/ES" and other picks match weapons, jewellery and currency

**File:** `src/filter/group.rs:45-66` (`COMBOS`), `src/filter/group.rs:121-163` (`compute_cover`, filter at line 131)

**Issue:** `mask_of` scores a constraint only against the seven combinations that have a defence. The point "no AR, no EV, no ES" is not one of them, so the cover treats it as don't-care. It then picks the cheapest constraints that also match it:
- `cover(Full AR)` = `[Any, No, No]` → `BaseEvasion 0` + `BaseEnergyShield 0`
- `cover(Full ES)` = `[No, No, Any]`
- "no EV" (Full AR, Full ES, AR/ES) = a single `BaseEvasion 0`
- all seven = no line at all

An item without the property (weapon, ring, amulet, belt, currency, flask, jewel) reads as base value 0. So every one of these blocks also matches every non-armour item.

The UI and docs promise the opposite. The chip tooltip says "Energy Shield bases". CLAUDE.md and CHANGELOG say "The selected chips are exactly what the rule matches".

Example: a class-less `Hide` rule with only "Full ES" picked hides all currency, jewellery and weapons. The test comment at `group.rs:765` ("items without any defence are not a combination") shows this was deliberate, but nothing tells the user.

The old Any/Has/No card wrote the same lines. The difference is that the user picked "No AR / No EV" explicitly; a chip labelled with a base type now writes it for them.

**Fix:** Treat "no defence" as an eighth point that is never selected:
- add bit 7 for `[false, false, false]` in `mask_of`;
- require `m & !sel == 0` with bit 7 never in `sel`.

The cover then uses `Has` where it must. For example Full ES becomes `[No, No, Has]`, and "no EV" becomes `[Has, No, Any] + [No, No, Has]`. The minimum block count grows (all seven becomes `Has` on any type, i.e. 3 parts or a different encoding). Re-run `every_selection_is_covered_exactly_and_minimally` with the extended bfs.

If the current trade-off is kept on purpose, then at least:
- state it on the card ("also matches items without defences unless the rule restricts Class");
- correct the "exactly what the rule matches" sentence in CLAUDE.md and the CHANGELOG.

## Warnings

### WR-01: Adding a defence keyword line to a group leader breaks the group silently and drops the line from the other blocks

**File:** `src/overlay/rules.rs:1005-1015` (`dispatch`, `other` arm) with `src/filter/group.rs:603-629` (`resync`)

**Issue:** Probe: `AddLine { kind: Condition, text: "BaseArmour > 50" }` on a group leader (the same happens with a raw-row `EditLine` whose new keyword is a defence keyword).
- The action succeeds (`changed=true`, no status).
- `resync` then builds each follower from the leader with *all* defence lines replaced by the follower's own. The new line exists only in the leader.
- The leader now has two `BaseArmour` lines, so `block_constraint` is `None`. On rebuild the group is `group broken` and shows as 2 rows.

There is no refusal and no message, and the rule's meaning differs between its blocks.

**Fix:** In `dispatch`, for a multi-block target, refuse any `AddLine`, `InsertLineAfter` or `EditLine` whose resulting keyword is in `group::DEFENCE_NAMES`. The message should say defences on a grouped rule are set with the chips.

More generally, after `resync` run `group::analyze` on the result. If the leader is no longer a `Leader` with the same member count, restore the snapshot and refuse. That turns every silent break into an error that rolls back.

### WR-02: Row actions aimed at a follower's block id are not refused and act on that one block

**File:** `src/overlay/rules.rs:968-982` (`dispatch`), `src/overlay/rules.rs:1023-1048` (`move_rule`), `src/overlay/rules.rs:1068+` (`block_action`)

**Issue:**
- `GroupIndex::members(follower)` returns `[follower]`, so `dispatch` sends the action to `block_action` on that block alone.
- Only `ApplyStyle` (and the line-edit arms, through their own checks) look at `self.row(id)`. `SetEnabled`, `Delete`, `Duplicate`, `SetKeyword`, `SetContinue`, `SetName`, `ClearName` and `RemoveStyle` all act on the follower alone.
- `move_rule` moves a follower out of its group.

Any of these breaks the group silently. Today the UI only issues ids taken from rows, but an id can go stale within a frame or across a rebuild. Also, `follower_ids_are_not_rows_and_refuse_edits` only covers `AddLine`, so the claim "every row action acts on the whole group" is untested for the other actions.

**Fix:** At the top of `dispatch`:
```rust
if let Some(id) = action_target(&action) && self.groups.is_follower(id) {
    return refused(NOT_EDITABLE_STATUS.to_string());
}
```
and extend the test to cover Delete, SetEnabled and Move.

### WR-03: `simple_all` outlives the pending card, so a later Simple Defences pick opens with all seven chips on

**File:** `src/overlay/rule_list.rs:357-361`, `src/overlay/body.rs:658-683`

**Issue:**
- Collapsing or switching rows prunes `pending_simple_defences`, `pending_defences` and `pending` to the expanded row, but leaves `simple_all` alone.
- Steps to reproduce: pick all seven chips (no line written, `simple_all` gets the id), collapse the row, re-expand it, choose **+ condition → Simple Defences**. `shown_selection` returns `ALL_COMBOS`, so the new card shows every chip lit.
- The first click then *removes* a chip and writes a 6-combination cover. The user expected to add one base type.
- `chip_click` and the X keep both sets in sync. Nothing else does: Revert of that rule, the advanced Defences pick, or an Apply that re-keys.

**Fix:** In the collapse branch add `list.simple_all.retain(|b| Some(*b) == keep);`. Also clear `simple_all` for the id wherever `pending_simple_defences` is inserted from the picker (`pick_entry`, body.rs:328-331). More simply, keep `simple_all ⊆ pending_simple_defences`: drop the id from `simple_all` whenever it leaves the pending set.

### WR-04: The styled rewrite swallows a follower's rewrite or tag-removal failure, which can write a broken group and shift later rule numbers

**File:** `src/styled.rs:186-196` (`Err(_) if follower => {}`), `src/styled.rs:222-226` (`let _ = doc.remove_meta(...)`)

**Issue:**
- If the leader's `rewrite_block` succeeds and a follower's fails, the error is dropped. The same goes for `drop_unknown_style_tags` when a follower's tag removal fails.
- The applied document then holds a group whose blocks differ. It loads as `group broken` with no outcome message.
- `rule_index_of` re-runs `group::analyze` on the half-mutated document. Every later rule in that pass is then reported one number too high, because the former follower now counts as a rule.

Unlikely while blocks are identical, but this pass runs on every Apply.

**Fix:**
- Report a follower failure against the leader's rule index rather than ignoring it.
- Better: rewrite a group as a unit. Rewrite the leader, then call `group::resync(doc, members)`, and on `Err` restore the leader from a pre-pass clone.
- Compute rule indices once, before the loop. That fixes both the numbering and the repeated `analyze` per report.

### WR-05: The `group.rs` module doc claims full rollback on `Err`, which the multi-step functions do not provide

**File:** `src/filter/group.rs:11-12`, `634-673` (`set_selection`), `605-629` (`resync`), `678-700` (`duplicate`)

**Issue:** The module doc says "Every mutation goes through the validated Document edit API, so an `Err` leaves the document as it was". That holds per `Document` call, but these functions make up to three separate commits:
- `set_selection`: `set_constraint`, then `set_meta`/`remove_meta`, then `replace_blocks_after`. If the last one fails, the leader is already rewritten and tagged.
- `resync` and `duplicate` are similar.

Only `WorkingFilter` is safe, because it snapshots first. These are `pub` and are called directly by the module's own tests, so a future caller (CLI, styled pass) that trusts the doc comment will get a half-written group.

**Fix:** Either clone and restore inside the functions:
```rust
let snap = doc.clone();
let r = (|| { ... })();
if r.is_err() { *doc = snap; }
r
```
or correct the module doc and each function's doc to say "may leave a partial edit on `Err`; callers must snapshot".

### WR-06: A rule moved past a run that ends the file loses its blank separator and the file's final-newline state

**File:** `src/filter/structure.rs:455-503` (`swap_runs`, reached by every `move_rule`)

**Issue:** Probes (multi-block `swap_runs`):
- A group that is the last thing in a file with no final newline, moved up past a single rule: the result is `...SetFontSize 30\nShow\n  Rarity Normal\n\n`. There is no blank line between the group and the moved rule, and the file now ends with an extra empty line instead of no newline.
- A group moved down past a final unterminated rule gives `Show\n  Rarity Magic\n# ff:name Pure\n...`. The moved rule's last line sits directly against the group's `# ff:name`.

The parse is still correct, but the bytes change and the layout degrades. This is pre-existing behaviour of `move_block`'s region logic, which `swap_runs` generalises. It now applies to whole groups and to every move past one.

**Fix:**
- When `b_region` ends at EOF with no trailing blank, insert one blank line between the swapped-in `b` and `a`.
- Drop the trailing blank that `a` carries to the end.
- Re-apply `Eol::None` to the new last non-empty line.

Add a test with a group last in a file that has no final newline, moved up and back down. It should round-trip exactly.

## Info

### IN-01: Free-form leading comments (section headers) are copied into every block of a group

**File:** `src/filter/group.rs:561-595` (`part_span` copies the whole leader span)

**Issue:** Probe: `# === Section ===\nShow\n  Rarity Rare` with Full AR + Full EV picked writes `# === Section ===` above both blocks. A NeverSink/FilterBlade section header directly above a rule is repeated up to four times. It is removed again on shrink.

**Fix:** Copy only `# ff:` metadata lines from the leader's leading comments into later parts, and leave other comments out of the skeleton comparison. If the duplication is intended, document it.

### IN-02: The first chip click drops trailing comments and keyword spelling on zero-based lines

**File:** `src/filter/group.rs:543-549` (`set_constraint` → `canonical_texts`)

**Issue:** Before this change, Has↔No patched only the operator and value, which kept `# keep` and `baseevasion` spelling (review WR-02 of phase 27). Now any chip change that alters a block's constraint rewrites all its defence lines in canonical form. CLAUDE.md does say "bytes kept until a chip is clicked", so this is a deliberate change of an earlier guarantee, but no test pins it.

**Fix:** Add a test that pins the new behaviour. Or, when a type's segment is unchanged, keep that line's bytes: rewrite only the types whose segment changes.

### IN-03: `chip_click` updates the card state before it knows whether the write succeeded

**File:** `src/overlay/body.rs:670-683`

**Issue:** `pending_simple_defences` and `simple_all` are updated when the click is drawn. If `SetDefenceSelection` is then refused (for example, the line-index check fails after another edit in the same frame), the card state no longer matches the document.

**Fix:** Apply the state change in `apply_row_actions` after `outcome.changed`, or return it alongside the action.

### IN-04: A chip change shifts the absolute line numbers of every later rule within the same frame

**File:** `src/overlay/app.rs:575-615`

**Issue:** `SetDefenceSelection` inserts or removes whole blocks. A later `EditLine`, `RemoveLine` or `InsertLineAfter` in the same frame, on a rule below or on the same rule's lines after the defence lines, carries a stale `line`. The `expect` check refuses it, which is safe, but it shows a spurious `line_error`. The problem already existed for `AddLine`; it is wider now.

**Fix:** Run `SetDefenceSelection` last in a frame, or stop after it (defer the rest to the next frame).

### IN-05: A group whose blocks differ only in `#ff:` vs `# ff:` spelling is flagged broken

**File:** `src/filter/group.rs:363-372` (`skeleton` compares raw text), `480-509`

**Issue:** Probe: changing one member's `# ff:name` to the legacy `#ff:name` makes the group `group broken`. Apply normalises the spelling afterwards, but the group stays split until it is edited. This only happens to hand-edited files, so it is low risk.

**Fix:** In `skeleton`, compare normalised `ff_marker()` (key, value) pairs instead of raw text for `# ff:` lines.

---

_Reviewed: 2026-10-15_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
