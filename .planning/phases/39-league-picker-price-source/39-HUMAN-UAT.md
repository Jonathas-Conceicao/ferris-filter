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

### 5. Tier edits pending under Fallback (review fixes CR-01, WR-01)
expected: Type a tier cutoff (even an out-of-order one), then switch to Fallback. Apply is not blocked by it and does not write tiers.json; the typed text and its inline error stay; the Economy ` *` stays; with only those edits pending Apply is not live. Switch back to Live: the edits count again and the next Apply writes them (or refuses on a bad value).
result: [pending]

### 6. Quit with tier edits pending under Fallback (review fixes WR-01, IN-08)
expected: With only those tier edits pending, tray Quit's popup reads "Tier edits apply only with Live prices. Quit and lose them?" with Quit without applying / Cancel. With other work pending too, Apply & quit applies it, stays open, and shows the "Not quit: ..." warning; with the game running, the in-game reload result still lands (amber) and carries that warning after the overlay is summoned again.
result: [pending]

### 7. Pending price-source switch across filter switch / Reload (review fix WR-02)
expected: Switch the price source on filter A (marker shows), switch to filter B (or Reload onto B after editing ff-settings.json), then back to A: A's pending marker is still there.
result: [pending]

### 8. Offline refresh and broken ff-settings.json (review fixes WR-03, WR-04)
expected: After the list has loaded, go offline and press refresh: the list and default league stay usable, only the dropdown tooltip says the refresh failed. With an ff-settings.json made invalid (trailing comma), picking a league applies for the session, the status reads "Could not save ff-settings.json: ..." and the file is left as it was.
result: [pending]

## Summary

total: 8
passed: 0
issues: 0
pending: 8
skipped: 0
blocked: 0

## Gaps
