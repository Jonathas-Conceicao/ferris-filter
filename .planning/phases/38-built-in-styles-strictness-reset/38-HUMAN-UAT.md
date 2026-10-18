---
status: partial
phase: 38-built-in-styles-strictness-reset
source: [38-VERIFICATION.md]
started: 2026-10-18T00:00:00Z
updated: 2026-10-18T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Strictness row layout in the live overlay
expected: At 660pt the seven strictness captions fit on one line, the stacked two-word captions read well, the selected highlight shows Flat Broke on a fresh install (no styles.json), Custom looks dim and is not clickable, and each level's tooltip reads right.
result: [pending]

### 2. Strictness level as a pending edit
expected: Clicking Loaded marks the Config tab ` *`, C to F show Hide in the Styles table; Revert asks "Discard config changes?" and restores Flat Broke; Apply writes styles.json. Trying each level against a real filter hides/shows the expected tiers in game.
result: [pending]

### 3. Minimap tile tooltip shows once
expected: Hovering a minimap shape tile shows exactly one tooltip in the CONFIG style editor and in a Custom rule's STYLE lines (the unit test covers the style-editor tile).
result: [pending]

### 4. Reset to defaults buttons and popups
expected: Both headings (Config → Styles, Economy → Tiers) show Reset to defaults at the right, greyed at the defaults; the "Reset styles to defaults?" / "Reset tiers to defaults?" popups show a red Reset and Keep; after Reset the tab shows ` *`, custom styles are kept, Revert undoes it and Apply writes it.
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
