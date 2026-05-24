---
phase: 02-static-generate
plan: 02
subsystem: emitter
tags: [rust, filter-generation, code-generation, dracula, poe2, determinism]

# Dependency graph
requires:
  - phase: 02-static-generate/02-01
    provides: typed Styles struct (D-11), RarityRule enum (D-10), Config model with named fields

provides:
  - "src/emitter.rs: pub fn generate(&Config, &CurrencyData) -> String"
  - "Regenerated universal.filter: 85 blocks (83 Show + 2 Hide), LF-only, GEN-03 tags"
  - "Single canonical action order via emit_style_actions (D-03)"
  - "cmd_generate: load → emit → write pipeline (D-09)"
  - "D-14 diff artifacts: diffs/universal_filter.diff"

affects: [02-03-snapshot-tests, phase-3-price-driven-tiering]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure emitter function: (Config, CurrencyData) -> String — no I/O, testable in isolation"
    - "Single canonical action emitter: all 85 blocks go through emit_style_actions"
    - "Deterministic collection traversal: Vec fields only, no HashMap iteration in emission"
    - "LF-only output: writeln! on all platforms, never inline \\r\\n"

key-files:
  created:
    - src/emitter.rs
    - .planning/phases/02-static-generate/diffs/universal_filter.diff
    - .planning/phases/02-static-generate/diffs/universal_before.filter
  modified:
    - src/main.rs
    - universal.filter
    - config.json

key-decisions:
  - "D-03 canonical action order applied: PlayEffect→MinimapIcon→SetTextColor→SetBackgroundColor→SetBorderColor→SetFontSize→Sound→DisableDropSound"
  - "Re-baseline D-01: recycle block action order normalized (was SetTextColor/SetBorderColor/SetBackgroundColor/MinimapIcon/PlayEffect)"
  - "D-02 behavioral equivalence confirmed: only header source-indicator + recycle reorder in diff"
  - "Tier-C beam bug fixed in config.json (Rule 1): was null, must be Yellow to match original filter"
  - "derive_block_label handles custom headers: Expedition Logbooks B, Pinnacle Keys, Map Fragments, Trial Coins, Inscribed Ultimatum, Omen Splinter, Tablet"

patterns-established:
  - "emit_style_actions: single function owns canonical order; all blocks call it"
  - "Gear blocks use bare Class without == (substring match); currency blocks use Class == (exact)"
  - "armour_types HashMap accessed by named key only (anyNonEvasion/esHybrid/pureES) — never iterated"
  - "EOF handling: pop trailing blank line after last currency block to end with exactly one newline"

requirements-completed: [GEN-01, GEN-02, GEN-03, GEN-04]

# Metrics
duration: 45min
completed: 2026-05-24
---

# Phase 02 Plan 02: Static Filter Emitter Summary

**Deterministic Rust emitter producing all 85 filter blocks from config.json + currency.json, with a single canonical action order and regenerated universal.filter as the new golden master**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-05-24T00:00:00Z
- **Completed:** 2026-05-24T00:45:00Z
- **Tasks:** 2
- **Files modified:** 5 (src/emitter.rs created, src/main.rs, universal.filter, config.json, diffs/)

## Accomplishments

- Implemented `src/emitter.rs` with `pub fn generate(&Config, &CurrencyData) -> String` — pure, deterministic, LF-only, single canonical action order (D-03)
- Wired `cmd_generate` in main.rs: loads config + currency, calls emitter, writes output (fs error via miette per T-02-03)
- Regenerated `universal.filter` as the new golden master: 83 Show + 2 Hide = 85 blocks, all GEN-03 tags present, no Unique/Jewels blocks (GEN-04)
- 32 unit tests pass (5 new emitter tests: smoke, lf-only, trailing-newline, block-counts, determinism)
- D-14 diff artifacts saved for human review

## Task Commits

1. **Task 1: Emitter core — header, recycle, gear bands, flask/charm bands, currency** - `9de7895` (feat)
2. **Task 2: Wire cmd_generate + regenerate universal.filter + fix tier-C beam** - `89547c8` (feat)

## Files Created/Modified

- `src/emitter.rs` - Pure emitter function with all 85 blocks, canonical action order, derive_block_label, 5 unit tests
- `src/main.rs` - cmd_generate wired: load config+currency, generate, fs::write
- `universal.filter` - Regenerated as new golden master (re-baseline D-01)
- `config.json` - Fix: tier-C beam null → Yellow (Rule 1 auto-fix)
- `.planning/phases/02-static-generate/diffs/universal_filter.diff` - D-14 human-review diff
- `.planning/phases/02-static-generate/diffs/universal_before.filter` - D-14 before-state snapshot

## Decisions Made

- Applied D-03 canonical order: beam/minimap first (world effects), then colors, font, sound, disable — matches the dominant currency-block pattern (~60 of 85 blocks)
- derive_block_label encodes the 7 custom header exceptions (Expedition Logbooks, Pinnacle Keys, Map Fragments, Trial Coins, Inscribed Ultimatum, Omen Splinter, Tablet) as explicit match arms
- EOF handled by popping the trailing blank line after the last currency block entry — universal.filter ends immediately after the last `PlayAlertSound` line (RESEARCH Pitfall 6)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed tier-C beam: config.json had `"beam": null`, original filter has `PlayEffect Yellow`**
- **Found during:** Task 2 (D-02 diff review)
- **Issue:** config.json's tier-C style defined `"beam": null`, but the original universal.filter emits `PlayEffect Yellow` for all 14 tier-C blocks. This made the diff show 14 `PlayEffect Yellow` removals — a behavioral change, not permitted by D-02.
- **Fix:** Changed config.json tier-C from `"beam": null` to `"beam": { "color": "Yellow", "temp": false }`
- **Files modified:** config.json
- **Verification:** D-02 diff confirmed: only header source-indicator + recycle reorder remain (zero block behavior changes)
- **Committed in:** 89547c8 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — bug in config.json data)
**Impact on plan:** Essential for D-02 behavioral equivalence. No scope creep.

## Issues Encountered

None beyond the tier-C beam bug (documented above as a deviation).

## User Setup Required

None — no external service configuration required.

## Known Stubs

None — the emitter is fully implemented and the generate command is wired end-to-end.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes introduced beyond what was in the plan's threat model.

## Next Phase Readiness

- `emitter::generate` is ready for Plan 02-03's snapshot + artifact-guard tests (D-05/D-06/D-07)
- `universal.filter` is the new golden master to snapshot against
- Plan 02-03 adds `insta` snapshot test + determinism test + artifact-guard test
- In-game human smoke test (D-13) can be run now: copy universal.filter to PoE2 config folder and verify highlights

---
*Phase: 02-static-generate*
*Completed: 2026-05-24*
