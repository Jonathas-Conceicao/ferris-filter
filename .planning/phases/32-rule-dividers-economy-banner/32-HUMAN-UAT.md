---
status: partial
phase: 32-rule-dividers-economy-banner
source: [32-VERIFICATION.md]
started: 2026-10-16T16:12:30.906Z
updated: 2026-10-16T16:12:30.906Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. SECT-01: load a filter with # ff:section banners (temp home, sample filter)
expected: Each banner is a header with the right rule count; a foreign #==== comment stays a comment; rules above the first banner have no header; a filter starting with a banner has no headerless rules
result: [pending]

### 2. + Add section, Section above, rename (pencil)
expected: + Add section inserts at top, scrolls to it, focuses the name box; Section above puts the banner above the rule (above the whole run for a grouped rule); Enter commits, Escape cancels; spaces trimmed, empty becomes Section, pasted newlines stripped. The Section above row is captured in shots/section-above-row.png (added by the orchestrator)
result: [pending]

### 3. Move / remove a section; Apply / Revert / Reload for add, rename, move, remove
expected: Divider steps past one rule per click (a group counts as one); arrows dim when nothing is left to step over; x removes only the banner; Apply writes, Revert undoes, Reload re-reads
result: [pending]

### 4. SECT-03 persistence
expected: Collapse/expand survives an overlay restart (collapsed_sections in ff-settings.json), never shows the unapplied marker or changes the filter file; renaming a collapsed section keeps it collapsed
result: [pending]

### 5. SECT-04 Economy section
expected: Economy is the last header, collapsed by default, closed padlock, '{n} rules', muted read-only rows, no arrows; expanded shows the small x opening Remove economy popup; afterwards the Add economy section footer button appears and re-adds it
result: [pending]

### 6. ff-cli apply --add-economy / --remove-economy; old markers
expected: --add-economy appends the three-line banner at EOF; --remove-economy removes banner and blocks but keeps a following user section banner; plain apply never adds one; a lone old one-line ff:economy-end / economy-begin file is reported damaged and stops apply
result: [pending]

### 7. SECT-05 search and reorder
expected: Search finds a rule inside a collapsed section; header expands for display with 'm of n rules'; clearing restores collapse; rule arrows hop a divider; + Add rule on a file starting with a banner inserts above the banner
result: [pending]

### 8. Round trip
expected: Open an untouched banner filter, Apply, file byte-identical
result: [pending]

### 9. Less-texty UI changes for author review
expected: WR-02 amber triangle on a condition-less enabled non-Continue rule followed by more rules (not on the last, Continue or disabled rule); closed padlock on Economy header and on can't-enable rules; chevron and pencil icons; Section above button; interim small x on expanded Economy header
result: [pending]

## Summary

total: 9
passed: 0
issues: 0
pending: 9
skipped: 0
blocked: 0

## Gaps
