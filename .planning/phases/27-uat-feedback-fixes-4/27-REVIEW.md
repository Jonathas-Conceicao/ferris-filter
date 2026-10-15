---
phase: 27-uat-feedback-fixes-4
reviewed: 2026-10-15T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src/overlay/body.rs
  - src/overlay/conditions.rs
  - src/overlay/config_view.rs
  - src/overlay/grammar.rs
  - src/overlay/rule_list.rs
findings:
  critical: 0
  warning: 2
  info: 5
  total: 7
status: issues_found
---

# Phase 27: Code Review Report

**Reviewed:** 2026-10-15
**Depth:** standard (changes since `314fd72`)
**Files Reviewed:** 5
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

I reviewed the Phase 27 diff against the locked decisions in 27-CONTEXT.md:

- **UAT4-01:** the right-aligned Order column.
- **UAT4-02:** the colour-combo scroll bar. The cause analysis in 27-01-SUMMARY matches egui 0.36.2 `combo_box.rs`.
- **UAT4-03:** Simple Defences. This covers zero-based classification, Any/Has/No writes, picker exclusivity, card stability and the 27-combination strip.

The pure logic in `conditions.rs` is correct:
- `zero_based`, `simple_defences`, `seg_write` and `combo_matches`/`lit_combos` are right.
- The 27-setting test checks the strip against an independent formulation.
- Exclusivity is sound. `present_entries` always pushes both entries together, and both picks route to `FocusTarget::Defences`.

`cargo test --lib overlay::` gives 493 passed and 5 failed. All 5 failures are the known baseline caused by the author's local `styles.json`/`tiers.json` edits (`config_state::*`, `app::tests::apply_working_style_change...`, `app::tests::clean_config_follows...`).

I found two defects that should be fixed:
- **WR-01:** card stability does not hold across APPLY.
- **WR-02:** a Has/No switch drops the line's trailing comment, because it goes through the raw `LinePatch::Whole` path.

No blockers.

## Warnings

### WR-01: A Defences card edited to zero-based lines flips to Simple Defences right after APPLY, on the still-expanded row

**File:** `src/overlay/app.rs:1142-1147` (the `clear_ids` + `expanded` remap after a successful Apply), `src/overlay/rule_list.rs:113-123`, `src/overlay/body.rs:480`

**Issue:** the locked decision says the card kind "must not flip under the user while they edit". Classification may come from the file again only after a reload. But a successful Apply calls `self.list.clear_ids()`, which empties `advanced_defences`, and then re-expands the same row by index (`self.list.expanded = expanded_index.and_then(...)`). Here is the sequence:

1. Pick Defences.
2. Toggle Armour on, which writes `BaseArmour >= 1`.
3. Press APPLY.

The open card immediately becomes a Simple Defences card under the user. The same happens to a loaded `BaseArmour >= 100` that was dragged to `1` with the slider. 27-04-SUMMARY treats "the Apply re-parse counts as a reload" as a deliberate choice. But Apply is not a reload in the user's terms: the row stays open, the edit session continues, and the user never asked for the file to be re-read. The test `loaded_defences_card_edited_to_zero_based_stays_defences` (body.rs ~4452) even names "the Apply re-parse" as a flip point, so the gap is encoded rather than caught.

**Fix:** carry the marker across the re-parse the same way `expanded` is carried, by row index. Before `clear_ids`, collect the indices of rows in `advanced_defences`. Afterwards, map them back to the new ids:

```rust
let advanced_idx: Vec<usize> = self.list.advanced_defences.iter()
    .filter_map(|id| working.row(*id).map(|r| r.index))
    .collect();
self.list.clear_ids();
for i in advanced_idx {
    if let Some(r) = self.working.as_ref().and_then(|w| w.rows().get(i - 1)) {
        self.list.advanced_defences.insert(r.id);
    }
}
```

Keep clearing it on RELOAD, Revert and filter switches. Add an app-level test: Defences card, toggle on, Apply, and the card is still `"defences"`.

### WR-02: Simple Defences Has<->No rewrites through `LinePatch::Whole`, dropping trailing comments and skipping strict validation

**File:** `src/overlay/body.rs:669-674` (`simple_seg_actions`, `SegWrite::Replace`)

**Issue:** `LinePatch::Whole` replaces everything after the indent (`lineedit.rs:142-147`). For example, `BaseArmour > 0 # $tier->ar` switched to No becomes `BaseArmour 0`, and the comment bytes are lost. FerrisFilter preserves foreign comment tags everywhere else (the tokenizer keeps them as a comment span, and NeverSink/FilterBlade filters carry them on condition lines). Every other structured card edits through `Operator`/`Values`/`Both` patches, which keep the comment.

`Whole` is the raw-row path, and it has two more side effects:
- It commits with `strict_warnings = false` (`lineedit.rs:149`), so a new warning would not be refused.
- `app.rs:589-595` treats any `Whole` edit whose keyword text differs (`baseenergyshield` vs `BaseEnergyShield`) as an ordinal shift and calls `forget_card_state()`.

The spec's "rewrites only that line (to the canonical `X > 0` / `X 0`)" is about the condition, not about deleting the user's annotation.

**Fix:** write the segment as an operator+value patch on the existing line:

```rust
SegWrite::Replace(_) => entry
    .map(|e| {
        let patch = match want {
            DefenceSeg::Has => LinePatch::Both(Some(Operator::Gt), "0".into()),
            _ => LinePatch::Both(None, "0".into()),
        };
        edit(id, e, patch)
    })
    .into_iter()
    .collect(),
```

The keyword's original casing and any trailing comment then survive, and the commit is strict. Add a test: `BaseArmour >= 1 # keep` switched to No gives `BaseArmour 0 # keep`.

## Info

### IN-01: `zero_based` accepts `+0`, `-0`, `00`, `01` as zero based

**File:** `src/overlay/conditions.rs:339-348`

**Issue:** the value goes through `str::parse::<i64>`, which accepts a leading sign and leading zeros. So `BaseArmour -0`, `BaseArmour > +0` and `BaseArmour >= 01` all classify as Simple. The spec defines zero based as "value `0`" or `>= 1`. The semantics match and the bytes are kept, so the impact is low, but it is wider than the locked definition.

**Fix:** compare the raw token (`v == "0"` / `v == "1"`), or add these spellings to `zero_based_rejects_everything_else` if accepting them is intended.

### IN-02: Pending flag dropped before the write is known to succeed

**File:** `src/overlay/body.rs:830-843` (`simple_defences_card`, `list.pending_simple_defences.remove(&id)`)

**Issue:** the pending card is closed as soon as a write is emitted. If `apply_action` refuses the `AddLine` (for example, a strict commit that adds a new diagnostic), the card disappears and nothing was written. This is unlikely with the current grammar, and the Defences card does the same.

**Fix:** clear the pending flag in the action handler once `outcome.changed` is true, or accept the edge and document it.

### IN-03: Custom-style arrows now tab Down before Up

**File:** `src/overlay/config_view.rs:844-848`

**Issue:** the right-to-left cell draws Down first, so keyboard Tab focus visits Down, then Up. Visually the order is Up, Down. The RULES rows have the same quirk, so this is consistent, but it is a small accessibility regression in this table.

**Fix:** none required. Optionally allocate the cell left-to-right inside a right-aligned fixed-width cell (a `cell` of `COL_ORDER` placed by `right_to_left`).

### IN-04: The `advanced_defences` marker is not copied by Duplicate and never pruned

**File:** `src/overlay/rule_list.rs:57-64`

**Issue:** duplicating a rule whose Defences card was edited to zero-based lines shows the copy as Simple Defences and the source as Defences. Markers for deleted blocks stay until the next `clear_ids`. This is harmless but slightly surprising.

**Fix:** copy the marker to the new id in the Duplicate handler, or document that classification of a new block comes from its lines.

### IN-05: Test probes interleaved in production render code

**File:** `src/overlay/body.rs:520, 545, 569, 782, 801, 826, 852-853`

**Issue:** `#[cfg(test)] probe_*` calls and the `let _ = strip;` exist only to support headless tests. They clutter the card functions, and `let _ = strip` is a dead binding in release builds.

**Fix:** have `combo_strip` and the card functions return the rects the tests need, or gather the probes into one `#[cfg(test)]` helper per card.

---

_Reviewed: 2026-10-15_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
