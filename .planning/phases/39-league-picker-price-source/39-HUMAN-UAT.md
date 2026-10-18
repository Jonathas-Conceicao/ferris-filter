---
status: partial
phase: 39-league-picker-price-source
source: [39-VERIFICATION.md, 39-06-SUMMARY.md]
started: 2026-10-18T00:00:00Z
updated: 2026-10-18T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Live league list, restore and Sync outcome
expected: With poe.ninja reachable, the Economy tab's League dropdown lists the current leagues (fetched the first time the tab is opened, not at start-up). Pick one, relaunch: the choice is restored. Sync economy's outcome names that league.
result: [pending]

### 2. Fallback greying and tooltips
expected: Switch Price source to Fallback. League dropdown, refresh and Sync are greyed with the recommendation tooltip; the Tiers editor is greyed with "The fallback tiers are fixed."; Reset to defaults is dimmed (capture 7 looked slightly brighter than capture 6, though the code computes it disabled in both). Next Apply prices the economy section from the fallback pair.
result: [pending]

### 3. Ended league status line
expected: With a saved league no longer listed, the dropdown shows "<name> (ended)" in amber, Sync is greyed, and the status line reads "<name> is no longer on poe.ninja; pick a league" (cut with an ellipsis at the window width, full text on hover; judge whether that reads acceptably). Clicking it opens the Economy tab; nothing is switched.
result: [pending]

### 4. Price-source unapplied marker
expected: With a filter that has an economy section, switching Live/Fallback marks the Economy tab ` *` and the unapplied indicator; switching back clears it; Revert does not undo the switch.
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
