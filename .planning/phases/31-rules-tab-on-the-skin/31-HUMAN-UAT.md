---
status: partial
phase: 31-rules-tab-on-the-skin
source: [31-VERIFICATION.md]
started: 2026-10-16T13:59:58.439Z
updated: 2026-10-16T13:59:58.439Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Rule rows: one row per rule, round check enables/disables (disabled dims), arrows reorder …
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Rule rows: one row per rule, round check enables/disables (disabled dims), arrows reorder (disabled while searching), click expands one row at a time.
result: [pending]

### 2. Conditions: **+ condition** picker is grouped and searchable, sections start collapsed, an…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Conditions: **+ condition** picker is grouped and searchable, sections start collapsed, an added condition greys out and focuses the existing card.
result: [pending]

### 3. Actions: **+ style** picker adds an action card; X removes it; each action editable (font …
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Actions: **+ style** picker adds an action card; X removes it; each action editable (font size, colours, sound, minimap, beam).
result: [pending]

### 4. Operators: dropdown with plain-English labels; an existing operator (e.g. `!=`) is kept on…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Operators: dropdown with plain-English labels; an existing operator (e.g. `!=`) is kept on edit.
result: [pending]

### 5. Numeric: slider and exact number stay paired (ItemLevel).
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Numeric: slider and exact number stay paired (ItemLevel).
result: [pending]

### 6. Between: choose Between on a numeric; Min never exceeds Max; a loaded `>=`/`<=` pair shows…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Between: choose Between on a numeric; Min never exceeds Max; a loaded `>=`/`<=` pair shows as one card with bytes kept until edited.
result: [pending]

### 7. Defences: Armour / Evasion / Energy Shield toggles each with operator and slider; BaseDefe…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Defences: Armour / Evasion / Energy Shield toggles each with operator and slider; BaseDefencePercentile loads as raw row.
result: [pending]

### 8. Simple Defences: seven chips; selection writes the minimum number of blocks; groups carry …
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Simple Defences: seven chips; selection writes the minimum number of blocks; groups carry `# ff:group`; X removes all defence lines.
result: [pending]

### 9. Group behaviour: group edits (enable, delete, duplicate with a new id, move) act on the wh…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Group behaviour: group edits (enable, delete, duplicate with a new id, move) act on the whole group; a broken run is flagged `group broken` and left as written.
result: [pending]

### 10. Class picker: grouped, counts per group, classes outside config groups listed on one line.
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Class picker: grouped, counts per group, classes outside config groups listed on one line.
result: [pending]

### 11. Quoted lists: type + Enter and + ADD, a pasted quoted list splits into entries, per-entry …
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Quoted lists: type + Enter and + ADD, a pasted quoted list splits into entries, per-entry X, list scrolls at about 7 rows.
result: [pending]

### 12. Raw rows: unknown keyword line shown in a text field; Enter commits; an edit that adds a g…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Raw rows: unknown keyword line shown in a text field; Enter commits; an edit that adds a grammar Error shows an inline hint and changes nothing.
result: [pending]

### 13. Sockets: numeric 0-6 slider; a hand-written `Sockets >= 5GGG` keeps its letters when the c…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Sockets: numeric 0-6 slider; a hand-written `Sockets >= 5GGG` keeps its letters when the count changes.
result: [pending]

### 14. Booleans: Yes | No switch, no operator; remove the condition to match either.
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Booleans: Yes | No switch, no operator; remove the condition to match either.
result: [pending]

### 15. Pickers: colour picker (old look, alpha), minimap, beam and sound controls work.
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Pickers: colour picker (old look, alpha), minimap, beam and sound controls work.
result: [pending]

### 16. Style picker: Custom + presets; preset writes `# ff:style <id> <name>` and hides actions; …
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Style picker: Custom + presets; preset writes `# ff:style <id> <name>` and hides actions; unknown style flagged `unknown style` with tooltip.
result: [pending]

### 17. Custom keeps actions: switching a preset back to Custom keeps the current actions and drop…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Custom keeps actions: switching a preset back to Custom keeps the current actions and drops the tag.
result: [pending]

### 18. Show/Hide on Custom only: switch on its own line below the picker; Minimal blocks shown as…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Show/Hide on Custom only: switch on its own line below the picker; Minimal blocks shown as Minimal and left alone.
result: [pending]

### 19. Continue: round check, last line of the body; the Continue word appears in the collapsed r…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Continue: round check, last line of the body; the Continue word appears in the collapsed row.
result: [pending]

### 20. Duplicate creates a copy right below (new group id for a group); Delete asks via the popup…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Duplicate creates a copy right below (new group id for a group); Delete asks via the popup (Delete / Keep).
result: [pending]

### 21. + Add rule: inserts an empty `Show` as the first block of the file (above the first rule's…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: + Add rule: inserts an empty `Show` as the first block of the file (above the first rule's comments), expands it with Name focused, clears search, shows `Rule added at the top of the file. Press Apply to save it.`.
result: [pending]

### 22. Flag budget: flags beyond 120 px fold into `+N`; errors never fold away; the styled chip s…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Flag budget: flags beyond 120 px fold into `+N`; errors never fold away; the styled chip sits left of Continue with a gap; the rule name keeps at least 140 px.
result: [pending]

### 23. Empty states: `No rules yet`, `No rules match "<query>"`, `The filter could not be loaded.…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Empty states: `No rules yet`, `No rules match "<query>"`, `The filter could not be loaded.` (heading only).
result: [pending]

### 24. Apply gate: with an enabled grammar error Apply is refused; clicking the status message ju…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Apply gate: with an enabled grammar error Apply is refused; clicking the status message jumps to the first such rule.
result: [pending]

### 25. Colour actions: SetTextColor / Border / Background still use the legacy colour picker (Pha…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Colour actions: SetTextColor / Border / Background still use the legacy colour picker (Phase 33 restyles it).
result: [pending]

### 26. 500-block sample: open the FilterBlade sample (511 rules) and scroll; it stays responsive …
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: 500-block sample: open the FilterBlade sample (511 rules) and scroll; it stays responsive and the scrollbar tracks.
result: [pending]

### 27. Byte-for-byte round trip: open an untouched filter, press Apply; the file on disk is uncha…
section: A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)
expected: Byte-for-byte round trip: open an untouched filter, press Apply; the file on disk is unchanged (apart from the first-write `.ffbak`).
result: [pending]

### 28. Captions moved to tooltips: hover each control that formerly had a standing caption (e.g. …
section: B. Less-texty changes made in this phase
expected: Captions moved to tooltips: hover each control that formerly had a standing caption (e.g. unknown-style picker, Class/Defences explanations, list add fields) and confirm the explanation appears and no wall of text remains in the editor.
result: [pending]

### 29. Flag icon changes: error red disc with !, warning amber triangle, unknown style amber ring…
section: B. Less-texty changes made in this phase
expected: Flag icon changes: error red disc with !, warning amber triangle, unknown style amber ring ?, group broken two-link icon (no longer reads "0/0"), padlock for can't-enable, moved-across-Continue arrows; confirm each is legible at 100% and at 150-200% DPI and shows a tooltip.
result: [pending]

### 30. Add-rule tooltip / status text: hovering **+ Add rule** explains top placement; clicking s…
section: B. Less-texty changes made in this phase
expected: Add-rule tooltip / status text: hovering **+ Add rule** explains top placement; clicking shows `Rule added at the top of the file. Press Apply to save it.`.
result: [pending]

### 31. Add-field hints read "Type a base type, press Enter" / "Type a mod, press Enter" / "Type a…
section: B. Less-texty changes made in this phase
expected: Add-field hints read "Type a base type, press Enter" / "Type a mod, press Enter" / "Type a name, press Enter" / "Type a whole number, press Enter".
result: [pending]

### 32. Arrow click target (24x22) feels right and clicking the blank area under the arrows still …
section: B. Less-texty changes made in this phase
expected: Arrow click target (24x22) feels right and clicking the blank area under the arrows still expands the row.
result: [pending]

### 33. Number fields show 4-digit values; log sliders respond to Left/Right.
section: B. Less-texty changes made in this phase
expected: Number fields show 4-digit values; log sliders respond to Left/Right.
result: [pending]

### 34. Long dropdown option lists scroll inside the window.
section: B. Less-texty changes made in this phase
expected: Long dropdown option lists scroll inside the window.
result: [pending]

## Summary

total: 34
passed: 0
issues: 0
pending: 34
skipped: 0
blocked: 0

## Gaps

## UAT session 2026-10-17 (author)

Passed: rows, enable/disable, reorder, single expand; + Add rule at the top with the bare-Show
warning; every flag icon (error, warning, unknown style, group broken, moved across Continue,
padlock, Continue), checked with `tests/ui/test_ff.filter`; condition editing (operators, sliders,
Between, Defences, Simple Defences, class picker, quoted lists, raw rows), Duplicate / Delete;
511-rule scrolling. Style / colour items are superseded by quick tasks 261016-qi7 and the
style-groups task and are re-checked after them.
