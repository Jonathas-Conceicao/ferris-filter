---
status: complete
phase: 23-auto-reload-filter-in-game-on-apply
source: [23-VERIFICATION.md]
started: 2026-10-13T20:13:05Z
updated: 2026-10-15
---

## Current Test

[testing complete]

## Tests

### 1. 23-MANUAL-VERIFICATION.md check 0 - type /reloaditemfilter by hand in game chat
expected: The game reloads the filter; no unknown-command message (gates the whole feature)
result: pass (author, 2026-10-15): /reloaditemfilter works in game

### 2. Checks 1, 2, 3, 8 - RULES Apply with the game running and the setting on (8: identical Apply, also OVERWRITE in check 9)
expected: Game takes focus, command sent exactly once, edit visible in game, stripe later reads "Wrote <path>. Reloaded in game", clipboard restored afterwards; an identical Apply reads and reloads the same without rewriting the file
result: pass (author, 2026-10-15): Apply reloads the filter in game. Clipboard restore, image clipboard and identical Apply were not exercised separately.

### 3. Checks 4, 5, 6, 9, 10 - not-sent cases (game closed, browser tab titled "Path of Exile 2", setting off, refused Apply, CONFIG Apply)
expected: Nothing typed anywhere; Refresh reminder with the reason shown where applicable
result: assumed pass (not tested, accepted by the author 2026-10-15): browser tab titled "Path of Exile 2", setting off, refused Apply (game closed: pass, recorded as Phase 24 UAT-01; CONFIG Apply now reloads by design since Phase 25, UAT2-10 pass)

### 4. Checks 7, 7b, 12, 12b, 16 - chat already open, pre-filled prefix, held modifiers, elevated game
expected: Documented limitation (CR-02): text in an open chat box is sent first; the typed command overrides a pre-filled channel prefix; a held modifier aborts with its reason; an elevated game reports the failure and the Refresh reminder
result: accepted limitation (author, 2026-10-15): with chat open no reload is sent and no false "Reloaded" claim is shown, left as is. Held modifiers and elevated game: assumed pass (not tested, accepted by the author 2026-10-15)

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps

None. Untested items were accepted by the author as assumed passes (v0.5 wrap-up, 2026-10-15).
