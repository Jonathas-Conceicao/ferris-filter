---
status: partial
phase: 04-sync
source: [04-VERIFICATION.md]
started: 2026-05-25T00:00:00Z
updated: 2026-05-25T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Live softcore sync run
expected: `ferrisfilter sync` (no flags) against live poe.ninja writes `prices.json` with Exalt/Chaos/Divine as the first three entries, a non-empty item list, the correct softcore trade league name, and an ISO-8601 `fetchedAt` timestamp.
result: [pending]

### 2. League flag targeting
expected: `ferrisfilter sync --hc` and `ferrisfilter sync --standard` each write `prices.json` with the correct league name resolved from the live poe.ninja index-state.
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
