---
status: partial
phase: 22-single-instance-overlay
source: [22-VERIFICATION.md]
started: 2026-10-13T07:02:10Z
updated: 2026-10-13T07:02:10Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. 22-MANUAL-VERIFICATION.md check 1 - launch ff again while the stripe is visible
expected: Stripe stays shown (reveal, never toggle), second process exits within about 2 s, one tray icon, one ff.exe
result: [pending]

### 2. Check 2 and 4 - stripe hidden to tray, launch again (and Explorer double-click)
expected: Stripe reappears within about a second; second process exits; no error dialog
result: [pending]

### 3. Check 3 - PoE2 focused, stripe hidden, launch ff
expected: Stripe appears over the game (bypassing the hotkey foreground gate) without taking keyboard focus
result: [pending]

### 4. Checks 5, 8, 9 - repeated, during-startup and simultaneous launches
expected: Exactly one stripe, one tray icon, one ff.exe; Ctrl+Shift+Z still toggles; no spurious not-responding box
result: [pending]

### 5. Checks 6, 7, 10 - crash recovery, debug beside release, not-responding box
expected: No stale lock after a kill; separate debug name; message box and no second stripe
result: [pending]

### 6. Check 11 - relaunch immediately after tray Quit
expected: Fresh stripe, old tray icon gone, new tooltip names Ctrl+Shift+Z with no failure line, hotkey works
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps
