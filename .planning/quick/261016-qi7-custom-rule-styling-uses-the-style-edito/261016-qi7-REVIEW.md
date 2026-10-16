---
phase: quick-261016-qi7
reviewed: 2026-10-16T00:00:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - src/overlay/style_lines.rs
  - src/overlay/body.rs
  - src/overlay/config_view.rs
  - src/overlay/widgets.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/mod.rs
  - src/overlay/shell.rs
  - src/overlay/app.rs
  - src/overlay/screenshot.rs
findings:
  critical: 1
  warning: 5
  info: 4
  total: 10
status: issues_found
---

# Quick 261016-qi7: Code Review Report

**Reviewed:** 2026-10-16
**Depth:** standard
**Files Reviewed:** 10 (plus CLAUDE.md / CHANGELOG.md for doc accuracy)
**Status:** issues_found

## Summary

Reviewed `git diff dfd5204..HEAD -- src` (5916af2, f54235d, 9a0b71e). The shared `style_lines`
component is a clean move: CONFIG parity holds (same widgets, probes, memo semantics, file
draft, volume logic). The intent-to-line mapping is lossless for single-line slots: untouched
rules stay idle, edits go through `EditLine` with `expect` text, 3-token colours keep their form
while alpha is 255, and volume-less sound lines stay volume-less. Popup stacking, the
Escape/outside default and the auto-disarm logic are correct. Screenshot mode stays memory-only.

The main problem is the binding model when a rule has more than one line of the same action
keyword. The editor can show and edit a line the game does not use, and it can add a second
line next to one it cannot represent. Whichever line the game honours, one of those two paths is
wrong (CR-01). There are also two RULES Revert predicate mismatches and one comment-dropping
rewrite. The docs still describe the colour picker the old way, and the docs and CHANGELOG do not
say which actions can no longer be added.

## Critical Issues

### CR-01: With duplicate same-keyword action lines, the style lines show and edit a line the game may ignore, and the absent-slot add can create the duplicate

**File:** `src/overlay/body.rs:2611-2699` (binding), `src/overlay/body.rs:2772-2775, 2781, 2787-2793, 2799-2805, 2827-2833` (absent-slot `AddLine`), `src/overlay/rules.rs:711-725` (chip uses the same binding)

**Issue:** Each slot binds the FIRST representable line. Every other line of the same keyword
becomes a leftover. A slot whose only line of that keyword is unrepresentable reads as absent.
That gives two contradictory paths:

1. *Binding path.* With `MinimapIcon 0 Red Circle` followed by `MinimapIcon -1`, or
   `SetTextColor A` followed by `SetTextColor B`, the editor, the label preview and the collapsed
   chip all show and edit the first line.
2. *Add path.* With an unrepresentable line of the keyword (`MinimapIcon -1`, `PlayEffect None`,
   `SetFontSize 50`, a named `PlayAlertSound ShAlchemy` or `PlayAlertSound 17`) the slot shows
   **None**, even though the item has that look or sound in game. Picking a value runs
   `AddLine` and appends a second line of the same keyword after the unrepresentable one. The
   sound slot has a wider version of this: a `PlayAlertSoundPositional` or
   `CustomAlertSoundOptional` leftover plus a newly added `PlayAlertSound` leaves two alert
   sounds on one block. `EnableDropSound` plus an added `DisableDropSound` gives contradictory
   drop-sound lines.

If the game applies the last line of a keyword, path 1 is wrong: the edit and the chip have no
in-game effect. If it applies the first, path 2 is wrong: the user's new minimap, beam, font or
sound is shadowed by the line already there, while the editor shows it as active. Either way the
editor shows something the game does not do. That breaks "correct intent to line mapping", and
the user is not warned.

**Fix:** First confirm the game's duplicate-line rule. Then bind the line the game actually uses
(`entries.iter().rev()` if the last one wins) and never `AddLine` while a line of the same
keyword exists. In that case, rewrite the effective line in place, or disable the slot with a
tooltip naming the leftover line. For example, in `rule_intent_actions`:
```rust
// absent slot, but a same-keyword leftover exists: rewrite it instead of adding a duplicate
if let Some(e) = same_keyword_leftover(binding_entries, "MinimapIcon") {
    return one(raw_edit(id, e, &format!("MinimapIcon {}", tokens)));
}
```
Keep the chip (`rules.rs:719`) on the same "effective line" rule. Add tests for
`MinimapIcon -1` followed by a pick, and for two `SetTextColor` lines followed by an edit.

## Warnings

### WR-01: Removing `+ style` drops the only way to add several action keywords, and no doc says so

**File:** `src/overlay/body.rs:3030-3073` (`style_section`), `src/overlay/grammar.rs:115-140, 213` (`ACTION_GROUPS` / `action_groups()`, now only used by the test at body.rs:5451)

**Issue:** `PlayAlertSoundPositional`, `CustomAlertSoundOptional`, `EnableDropSound`,
`DisableDropSoundIfAlertSound`, `EnableDropSoundIfAlertSound`, `MinimapIcon -1` (Hide icon) and
`PlayEffect None` can no longer be added from the overlay. They can only be kept if already in
the file. CLAUDE.md:93 still says "Every condition and action is editable with structured
controls". CHANGELOG has no `Removed` entry. `action_groups()` is now dead production code.

**Fix:** Either keep a small "+ action" picker limited to the keywords the style lines cannot
represent, or add a CHANGELOG `Removed` entry plus a CLAUDE.md sentence listing what can no
longer be added, and make `action_groups` `#[cfg(test)]` or delete it.

### WR-02: Switching the sound kind rewrites the whole line and drops a trailing inline comment

**File:** `src/overlay/body.rs:2842-2851`

**Issue:** An Alert to Custom switch (or the reverse) uses `LinePatch::Whole`, which replaces
everything after the indent (`lineedit.rs:142-145`) and is applied non-strictly
(`lineedit.rs:149`). `PlayAlertSound 3 200 # loud` becomes `CustomAlertSound "uwu.mp3" 200`,
so the user's comment is silently lost. Every other style-line edit uses a value patch that
keeps it.

**Fix:** Carry the comment tail over:
```rust
let tail = entry.text.find(" #").map_or("", |i| &entry.text[i..]); // or the tokenizer's comment span
LinePatch::Whole(format!("CustomAlertSound \"{f}\"{}{tail}", volume_suffix(volume)))
```
A cleaner option is a `LinePatch::KeywordAndValues` variant that keeps the comment span. Add a
test with an inline comment.

### WR-03: RULES Revert is enabled but does nothing when only a style rewrite is pending

**File:** `src/overlay/app.rs:2706` (bar predicate `rules_dirty || style_pending`), `src/overlay/app.rs:1667, 1687-1689`, `src/overlay/app.rs:3017-3019` (auto-disarm)

**Issue:** The RULES Revert button is enabled when `style_pending` is true (for example after a
CONFIG style edit or a `styles.json` change on disk). The click only sets `revert_rules_armed`.
`popup_flags` requires `working.is_dirty()`, which is false, so no popup opens and the flag
disarms on the same frame. The result is a live button that does nothing. The SUMMARY's
claim that `rules_revertible()` is "the bar's Revert predicate" is false.

**Fix:** Use one predicate for both. Either drop `style_pending` from the RULES Revert
enablement (Revert cannot undo a style-file change anyway), or have `rules_revertible()` use the
same inputs and give the popup a body for that case.

### WR-04: "Discard rule changes?" also discards the ECONOMY tab's staged add/remove, and opens when there are no rule changes

**File:** `src/overlay/app.rs:1687-1689` (`is_dirty()` includes `pending`), `src/overlay/rules.rs:1874-1879` (`revert()` clears `pending`), `src/overlay/shell.rs:913-920`

**Issue:** `WorkingFilter::is_dirty` counts a staged economy add/remove, and the tab markers
explicitly assign that to ECONOMY (`rules_edited`, app.rs:2769). So with only a staged op, RULES
Revert opens "Unapplied rule changes to X will be discarded." when there are no rule changes,
and Discard silently drops the economy op. With real rule edits the op is also dropped without
being mentioned. This contradicts CLAUDE.md:217 ("Revert stays per tab ... Economy discards ...
a staged economy add/remove"). The behaviour predates this task, but the new popup now names
what it discards and gets it wrong.

**Fix:** Gate on `rules_edited()` and make RULES Revert keep the staged op, for example with a
`WorkingFilter::revert_rules()` that re-parses `applied_text` and re-stages `pending`. If that
coupling is wanted instead, name the staged op in the popup body as ECONOMY's popup does.

### WR-05: CLAUDE.md describes the colour picker the old way for the Custom style lines

**File:** `CLAUDE.md:96-97`

**Issue:** "the skinned colour picker with an Alpha slider (no None; the colour line is a chip
whose `R G B A` text is its tooltip ...)" is now true only for leftover colour cards. The Custom
style lines use the Text / Border / Background card with a None tile
(`widgets.rs:534-537`, `allow_none: true`), and clearing removes the line. CLAUDE.md is declared
an executable spec ("keep it accurate"). The decision to keep an out-of-range `SetFontSize` /
volume as a leftover (SUMMARY decision 2) is also not documented.

**Fix:** Change the sentence to: style lines use colour cards with Alpha and None (None removes
the line); leftover colour lines keep the chip without None. Add "out-of-range font size or
volume" to the leftover list at CLAUDE.md:86-88.

## Info

### IN-01: A line added to an empty colour slot flips from 4 tokens to 3 on the next drag in the same picker session

**File:** `src/overlay/widgets.rs:529-532`, `src/overlay/body.rs:2770-2775`
**Issue:** On an empty slot, the opening form is captured as `had_alpha = false`. The first
change adds `SetBorderColor r g b 255`. Each later change in the same open picker runs
`rgba_edit(.., opened_had_alpha=false)`, which rewrites it to `r g b`. The final form depends on
how many changes were made.
**Fix:** After an absent-slot add, treat the slot as 4-token (store `true` under the popup's
`form_id`), or add 3-token lines when alpha is 255.

### IN-02: The rule memo is keyed by BlockId and outlives Revert/Reload

**File:** `src/overlay/body.rs:2872`
**Issue:** `("rule-style-last", id)` remembers font, minimap, beam, sound and file values. After
a re-parse (Revert, Reload, Apply) BlockIds can point at different blocks, so another rule can
restore a stale remembered value.
**Fix:** Add a working-filter generation (`working.generation()`) to the memo id, as CONFIG does
with `memo_epoch`.

### IN-03: `--popup discard-rules` shows no popup on an empty or non-editable first rule

**File:** `src/overlay/app.rs:1087-1098`
**Issue:** If the staged filter has no rows, or `SetContinue` on row 0 is refused (an invalid
commented block), the flag is armed but `rules_revertible()` is false. The screenshot then
silently has no popup.
**Fix:** Fall back to `RowAction::AddRule`, which is always allowed, or warn on stderr when
nothing became dirty.

### IN-04: The Drop sound check reads inverted in the rule context

**File:** `src/overlay/style_lines.rs:320-327`
**Issue:** The check labelled "Drop sound" writes `DisableDropSound` when ticked. This is CONFIG
parity, but on a rule it now sits next to leftover `EnableDropSound` rows, so the ticked check
means the opposite of what it says.
**Fix:** Label the check "Silence drop sound" (the tooltip already says so) in both editors.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
