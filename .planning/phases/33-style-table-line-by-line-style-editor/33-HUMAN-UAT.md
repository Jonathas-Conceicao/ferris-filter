---
status: partial
phase: 33-style-table-line-by-line-style-editor
source: [33-VERIFICATION.md]
started: 2026-10-16T19:30:00.000Z
updated: 2026-10-16T19:30:00.000Z
---

## Current Test

[awaiting human testing]

Run the overlay with `FERRISFILTER_HOME` and `USERPROFILE` on a temp copy (CLAUDE.md development warning), so Apply does not write your live `styles.json` / `tiers.json`.

## Tests

### 1. STY-01: style tables at the real 660pt window
expected: Visibility / Name / Icon / Preview / Order headers sit over their columns in both the Built-in and Custom tables; built-in names start at the same x as the custom name text; the Icon column shows the minimap icon (blank for E, F, unchanged, hidden); no Show/Hide caption beside the switch; built-in Order arrows are greyed and do nothing (tooltip "Built-in styles keep their order")
result: [pending]

### 2. STY-02: expand S and a custom style
expected: One line each, no boxed panels: Visibility Show|Hide, Label preview, Text / Border / Background cards, Font size (check + slider), Minimap size pills + 11 colours + 12 shapes on one row, Beam none + colours + Temp, Sound dropdown (+ file field for Custom file), Volume, Drop sound; custom styles end with Delete style
result: [pending]

### 3. STY-02: switch-back memory
expected: Minimap / Beam / Sound to None and back, and Font size off and on, restore the last values, also after folding and reopening the row; a custom sound file survives Custom -> Alert -> Custom; Revert / Reload / Apply forget the remembered values
result: [pending]

### 4. STY-03: colour picker on style cards
expected: Opens in the game box frame below the card and stays inside the window for Text, Border and Background; square, hue bar, Hex, R/G/B, nine Dracula quick picks and the None tile on one row, Use colour; no alpha; #FF79C6 equals 255 121 198 both ways; edits are live, Escape restores the opening colour, a click outside or Use colour keeps it; opening and closing without a change leaves Apply disabled
result: [pending]

### 5. STY-03: empty colour slot
expected: On an empty slot (e.g. a style with no background), Use colour, the highlighted quick pick, or the seed hex confirmed with Enter each set the colour shown; a click outside with no change leaves the slot empty
result: [pending]

### 6. STY-03: rule colour actions use the same picker
expected: Clicking a rule's Set ... Color chip opens the picker with an Alpha slider and no None tile, also for a colour action near the bottom of the list; the R G B A text is the chip's tooltip; a 3-token line stays 3-token after a change with alpha 255, and Escape after an alpha change restores the line byte for byte (Apply stays clean)
result: [pending]

### 7. STY-04: rename, reorder, delete
expected: Renaming a custom style keeps its rules styled; reorder arrows move custom styles (order saved by Apply); Delete style at the bottom of a custom editor opens the Delete style? popup with the rule count (when > 0) and the config.json warning when it references the style; + Add style opens the new style's editor
result: [pending]

### 8. STY-05: Apply / Revert
expected: Apply writes styles.json and the filter together (all or none); Revert restores the styles and closes an open picker; tier edits behave as before; the in-game reload still runs
result: [pending]

### 9. Less-texty UI changes for author review
expected: Accept or reject each: (1) Show/Hide caption removed from style rows (the switch alone); (2) NAME caption removed from the editor, the rename tip moved to the built-in Name tooltip; (3) "Not set (game default)" caption replaced by the card's (none) state and the picker's None tooltip; (4) rule colour "R G B A" caption moved to the chip tooltip (the chip is a bare 34x22 swatch); (5) Font size / Volume number fields replaced by the slider's value text (no exact-number field in the style editor; RULES keeps its number fields); (6) Font size and Drop sound explanations are tooltips; (7) the picker's Hex field has no visible caption (tooltip and accessible name "Hex")
result: [pending]

## Summary

total: 9
passed: 0
issues: 0
pending: 9
skipped: 0
blocked: 0

## Gaps
