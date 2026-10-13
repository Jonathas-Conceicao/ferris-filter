---
status: partial
phase: 23-auto-reload-filter-in-game-on-apply
source: [23-VERIFICATION.md]
started: 2026-10-13T20:13:05Z
updated: 2026-10-13T20:13:05Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. 23-MANUAL-VERIFICATION.md check 0 - type /reloaditemfilter by hand in game chat
expected: The game reloads the filter; no unknown-command message (gates the whole feature)
result: [pending]

### 2. Checks 1, 2, 3 - RULES Apply with the game running and the setting on
expected: Game takes focus, command sent exactly once, edit visible in game, stripe later reads "Wrote <path>. Reloaded in game", clipboard restored afterwards
result: [pending]

### 3. Checks 4, 5, 6, 8, 9, 10 - not-sent cases (game closed, browser tab titled "Path of Exile 2", setting off, unchanged Apply, refused Apply, CONFIG Apply)
expected: Nothing typed anywhere; Refresh reminder with the reason shown where applicable
result: [pending]

### 4. Checks 7, 7b, 12, 12b, 16 - chat already open, pre-filled prefix, held modifiers, elevated game
expected: Documented limitation (CR-02): text in an open chat box is sent first; a pre-filled channel prefix is replaced by Ctrl+A; a held modifier aborts with its reason; an elevated game reports the failure and the Refresh reminder
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
