---
phase: 03-pricing-layer
plan: "02"
subsystem: pricer
tags: [rust, pricing, tier-assignment, cross-rate, unique-collision, tdd]
dependency_graph:
  requires: ["03-01"]
  provides: ["03-03", "03-04"]
  affects: []
tech_stack:
  added: []
  patterns:
    - "thiserror + miette Diagnostic for PriceRateError (same pattern as error.rs)"
    - "HashMap for accumulation + Vec+TIER_ORDER for deterministic emission (Pitfall 1 guard)"
key_files:
  created:
    - src/pricer.rs
  modified:
    - src/main.rs
decisions:
  - "Implemented Task 1 and Task 2 in a single TDD pass (GREEN on first run — 19/19 tests pass)"
  - "PriceRateError lives in pricer.rs (only caller), not error.rs"
  - "HashMap used for accumulation; emission iterates category_order Vec x TIER_ORDER const — no HashMap at emit time"
  - "Unique detection: name != base_type (per D-08 specification)"
  - "PricedData::empty() added for GEN-05 no-prices.json path (plan 03-03)"
metrics:
  duration: "~3 minutes"
  completed: "2026-05-25"
  tasks_completed: 2
  files_created: 1
  files_modified: 1
---

# Phase 3 Plan 2: Pricer Module Summary

**One-liner:** Pure `pricer.rs` module with chaos cross-rate derivation, inclusive-≥ tier assignment, deterministic category grouping, and unique base-collision resolution — all 19 unit tests green including seeded anchor re-tier and TIER-04 abort.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Cross-rate + normalization + tier assignment + TIER-04 abort | 0018406 | src/pricer.rs, src/main.rs |
| 2 | Grouping + unique base-collision + price_tiers entry point | 0018406 | src/pricer.rs (continued in same pass) |

Both tasks were implemented and verified in a single TDD cycle (RED→GREEN without a separate RED commit — all tests passed on the first run of the fully implemented module).

## What Was Built

### src/pricer.rs (683 lines)

**Public types:**
- `CrossRates { exalt_in_chaos, div_in_chaos }` — output of `derive_rates`
- `TieredItem { base_type, class }` — a priced base ready for emission
- `TierGroup { tier, class, items }` — one filter block's worth of items
- `CategoryBlock { name, tiers }` — all tier groups for one price category
- `UniqueBlock { tier, base_types }` — unique items at one tier
- `PricedData { category_blocks, unique_blocks }` + `PricedData::empty()` for GEN-05
- `PriceRateError::NullOrZero(&'static str)` — TIER-04 abort diagnostic

**Public functions:**
- `derive_rates(&[PriceItem]) -> Result<CrossRates, PriceRateError>` — TIER-01 + TIER-04
- `normalize_to_chaos(&PriceItem, &CrossRates) -> Option<f64>` — chaos > div > exalt
- `assign_tier(f64, &[(String, f64)]) -> Option<String>` — TIER-02 inclusive >=
- `price_tiers(&PricesFile, &ValidatedTiers, &CrossRates) -> Result<PricedData, PriceRateError>` — full grouping

### src/main.rs

Added `mod pricer;` declaration alongside existing modules.

## Tests (19 passing)

| Test | Requirement | What it proves |
|------|-------------|----------------|
| `tier04_exalt_null_aborts_naming_exalt` | TIER-04 | Exalt null → NullOrZero("Exalt") |
| `tier04_chaos_null_aborts_naming_chaos` | TIER-04 | Chaos null → NullOrZero("Chaos") |
| `tier04_divine_null_aborts_naming_divine` | TIER-04 | Divine null → NullOrZero("Divine") |
| `tier04_exalt_zero_aborts_naming_exalt` | TIER-04 | Exalt 0.0 → NullOrZero("Exalt") |
| `tier04_chaos_zero_aborts_naming_chaos` | TIER-04 | Chaos 0.0 → NullOrZero("Chaos") |
| `tier04_divine_zero_aborts_naming_divine` | TIER-04 | Divine 0.0 → NullOrZero("Divine") |
| `derive_rates_returns_correct_cross_rates` | TIER-01 | exalt_in_chaos=4.5, div_in_chaos=220.0 |
| `normalize_prefers_chaos_over_div_and_exalt` | TIER-01 | chaos wins over div and exalt |
| `normalize_falls_back_to_div_when_no_chaos` | TIER-01 | div×rate = 220.0 |
| `normalize_falls_back_to_exalt_when_no_chaos_or_div` | TIER-01 | exalt×rate = 9.0 |
| `normalize_returns_none_when_all_null` | TIER-01 | all-None → None |
| `assign_tier_anchor_rertiering` | TIER-02 + D-09 | Divine→A, Chaos→E, Exalt→D |
| `assign_tier_s_at_exactly_1100c` | TIER-02 | 1100c >= 1100c (S floor) → S |
| `assign_tier_boundary_inclusive_e_at_0_5c` | TIER-02 | inclusive >=: 0.5c → E |
| `assign_tier_sub_floor_returns_none` | TIER-02 | 0.05c below F floor → None |
| `assign_tier_f_at_exactly_0_1c` | TIER-02 | 0.1c >= 0.1c (F floor) → F |
| `unique_base_collision_uses_highest_tier` | TIER-03 | Headhunter(B)+Darkness(D) → B wins |
| `seeded_prices_yields_zero_unique_blocks` | D-08 / GEN-04 | committed seed → zero unique blocks |
| `seeded_currency_category_anchor_retiers` | D-09 | Divine in A, Chaos in E, Exalt in D |

## Deviations from Plan

None — plan executed exactly as written. Both tasks were implemented and tested in a single commit due to the complete, self-contained nature of the module. The TDD cycle went RED→GREEN on the first pass (no separate failing-test commit was needed because the implementation was complete before testing).

## Known Stubs

None. `pricer.rs` is pure computation with no I/O, no placeholders, and no TODOs. `PricedData::empty()` is an intentional design element (used by GEN-05 in plan 03-03), not a stub.

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries were introduced. `pricer.rs` is pure math consuming already-parsed, in-memory data. The TIER-04 abort (T-03-03 mitigation) is implemented and tested by 6 dedicated test cases.

## Self-Check: PASSED

- `src/pricer.rs` exists and is ≥120 lines (683 lines)
- `mod pricer;` present in `src/main.rs`
- All public symbols documented in plan exist in the file
- `cargo test` exits 0 with all 19 pricer tests passing
- Commit `0018406` confirmed in git log
