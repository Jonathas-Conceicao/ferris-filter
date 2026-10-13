---
status: partial
phase: 20-full-grammar-block-editing
source: [20-VERIFICATION.md]
started: 2026-10-13T05:00:58Z
updated: 2026-10-13T05:00:58Z
---

## Current Test

[awaiting human testing]

## Tests

Full step-by-step script: 20-MANUAL-VERIFICATION.md (13 steps; work on a copy of the game folder).

### 1. Condition controls at 460px stripe width (script item 1)
expected: Class, BaseType, Rarity, Sockets, Quality cards lay out without clipping; operator dropdowns show plain-English labels
result: [pending]

### 2. + condition popup, colour picker with alpha, minimap/beam/sound controls, nested scrolling (items 2-5)
expected: Popups open/close with Esc, search works, picker keeps RGBA, long chip lists scroll inside the rule list
result: [pending]

### 3. Apply style / locked rows / Remove style / live styles.json edit / unknown style tag (items 6-9)
expected: Style actions appear as locked 'from style' rows; editing styles.json while the overlay runs and pressing Apply reports 'Updated N styled rules.'; unknown tag shows 'unknown style' flag and does not block Apply
result: [pending]

### 4. Raw unknown-keyword line, accepted-exception commented-out block, crowded row/outcome area (items 10-12)
expected: Raw row editable; invalid commented-out block lines read-only (accepted exception); flags collapse to +N; outcome area capped at 96px with buttons visible
result: [pending]

### 5. In-game reload of an applied copy (item 13)
expected: PoE2 loads the filter after Options > Game > Item Filter > Refresh; styled rules show the new style
result: [pending]

### 6. Code-review logic fixes (from 20-REVIEW-FIX.md)
expected: On a 'Rarity != Normal' rule, uncheck Normal, pick Rarity again, click Magic -> a single 'Rarity != Magic' line; adding a chip to a negated rarity keeps the negation; a mod condition picked from + condition writes no exact count; raising alpha on a transparent colour keeps its RGB
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps
