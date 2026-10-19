---
status: passed
phase: 36-per-user-locations-embedded-defaults
source: [36-VERIFICATION.md]
started: 2026-10-18T00:00:00Z
updated: 2026-10-18T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Fresh launch against an empty per-user folder
Launch the release `ff` on a machine with a truly empty %APPDATA%\FerrisFilter (folder absent) and no config.json / mapping.json / *-default / *-fallback files beside the exe. Open each tab, then change a style and a tier cutoff and press Apply.
expected: Default styles, tiers, class picker groups and fallback prices all appear; Apply succeeds; %APPDATA%\FerrisFilter is created and holds styles.json, tiers.json and ff-settings.json; nothing is written beside the exe or in the working directory.
result: [pending]

### 2. Real Sync economy writes into the matching folder
Press Sync economy on the Economy tab against the live network (with and without FERRISFILTER_HOME set).
expected: prices.json is written into %APPDATA%\FerrisFilter (or into FERRISFILTER_HOME when set, with nothing written to %APPDATA%); the status line names the league.
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
