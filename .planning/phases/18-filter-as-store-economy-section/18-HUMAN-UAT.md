---
status: complete
phase: 18-filter-as-store-economy-section
source: [18-VERIFICATION.md]
started: 2026-10-13T00:00:00Z
updated: 2026-10-15
---

## Current Test

[testing complete]

## Tests

### 1. Launch ff.exe by double-click from Explorer (cwd not the repo) with PoE2 windowed
expected: Stripe opens, resolves the app root from the exe folder, loads the game-folder dynamic.filter, shows status notice; APPLY / ADD ECONOMY / RELOAD / SYNC ECONOMY buttons work
result: pass (author, 2026-10-15): Explorer double-click resolves the app root, loads the filter; APPLY, ADD ECONOMY, RELOAD and SYNC ECONOMY work

### 2. In the overlay, press APPLY, then edit dynamic.filter externally and press APPLY again
expected: First Apply writes (an identical result also counts as applied); second is refused with 'Filter changed on disk since it was loaded.' and RELOAD and OVERWRITE are offered; OVERWRITE replaces the outside edit with the overlay's rules and clears the warning
result: pass (author, 2026-10-15): second Apply refused as changed on disk; RELOAD and OVERWRITE both work

### 3. Run the real v0.4 game-folder dynamic.filter through the first Apply and reload it in PoE2 (Options > Game > Item Filter > Refresh)
expected: Filter loads without warnings; all user rules look identical in game; dynamic.filter.ffbak holds the original v0.4 bytes
result: pass (author, 2026-10-15): first Apply on the real v0.4 filter converted it; rules identical in game; .ffbak holds the original

## Summary

total: 3
passed: 3
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

None. Untested items were accepted by the author as assumed passes (v0.5 wrap-up, 2026-10-15).
