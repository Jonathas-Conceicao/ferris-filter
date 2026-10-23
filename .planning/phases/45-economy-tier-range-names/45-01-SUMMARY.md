---
phase: 45-economy-tier-range-names
plan: 01
subsystem: pricer / emitter
tags: [economy, ff-name, tiers]
requires: []
provides:
  - PricedData.tier_ranges
  - pricer::fmt_amount
  - range suffix on economy `# ff:name` lines
affects: [src/pricer.rs, src/emitter.rs, src/dynamic.rs, src/overlay/app.rs]
key-files:
  modified: [src/pricer.rs, src/emitter.rs, src/dynamic.rs, src/overlay/app.rs]
decisions:
  - Range comes from sorted cutoffs (same stable sort as assign_tier), top tier `>= floor`, others `< upper neighbour's floor`, in the tiers' own unit word.
metrics:
  tasks: 2
  files: 4
completed: 2026-10-23
---

# Phase 45 Plan 01: Economy Tier-Range Names Summary

Economy `# ff:name` lines now read `S tier Currency >= 30 chaos` / `E tier Essences < 2 chaos`, computed from tier cutoffs in the tiers' own unit, with uniques and gem background-override categories included.

## What changed
- `src/pricer.rs`: `chaos_floor` helper (factored from `normalize_cutoffs`), `pub fn fmt_amount`, private `tier_ranges`, new `PricedData.tier_ranges` (empty in `empty()`). Tests: fmt table, default (chaos) and fallback (exalt) ranges, empty map.
- `src/emitter.rs`: `economy_name` helper; both tagged writes (inside `if tags`) use it; `ranges` parameter threaded through the two section emitters. Gem categories share the category write site so they are covered. Tests: pins updated, gem-category range test, empty-map plain-name fallback.
- `src/dynamic.rs`: pin updated; pending S=1e6 renames A to `< 1000000 chaos`.
- `src/overlay/app.rs`: tests made unit-agnostic (`starts_with("S tier Currency >= ")`, `" < "`/`" >= "` check); staged preview asserts `< 100000000 chaos` after a pending tier edit.

## Verification
`cargo test --locked` green (1710 lib tests plus all integration tests incl. artifact_guard, generate_snapshot, dynamic_filter). `git diff --stat tests/fixtures/` empty. No new dependencies.

## Deviations from Plan
None. (Task 1's commit alone does not compile the emitter test literal; the struct literal fix landed in the Task 2 commit.)

## Known Stubs
None.

## Self-Check: PASSED
Commits 6ffd3bf and dc5de56 exist; files modified as listed.
