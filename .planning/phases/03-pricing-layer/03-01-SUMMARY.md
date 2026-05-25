---
phase: 03-pricing-layer
plan: "01"
subsystem: pricing-data
tags: [prices, schema, seed, migration, tdd]
dependency_graph:
  requires: []
  provides: [seeded-prices-json, extended-price-item-schema, seed-prices-binary]
  affects: [src/models/prices.rs, prices.json, Cargo.toml]
tech_stack:
  added: []
  patterns: [tdd-red-green, one-time-migration-binary, de-dup-anchor-prepend]
key_files:
  created:
    - src/bin/seed_prices.rs
  modified:
    - src/models/prices.rs
    - prices.json
    - Cargo.toml
decisions:
  - "D-03: PriceItem now has required class+category+baseType (String, not Option)"
  - "D-09 seed: non-reference bases at tier lower-bound; Chaos=1c, Exalt=4.5c, Divine=220c"
  - "Anchor de-dup: Exalt/Chaos/Divine prepended as items[0..2], skipped in currency walk"
  - "seed_prices duplicates minimal currency structs locally (no lib target in crate)"
metrics:
  duration_seconds: 200
  completed_date: "2026-05-25"
  tasks_completed: 3
  files_changed: 4
---

# Phase 03 Plan 01: prices.json Schema Extension and Seed Summary

Extended `PriceItem` with required `class` and `category` fields (D-03), wrote a one-time
`seed_prices` migration binary, and committed the full 358-item `prices.json` seeded from
`currency.json` with anchor orbs first and all non-reference bases at tier lower-bound values.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Extend PriceItem schema (TDD RED+GREEN) | 18a8a76 | src/models/prices.rs |
| 2 | Write seed_prices migration binary | ef64db9 | Cargo.toml, src/bin/seed_prices.rs |
| 3 | Verify + commit seeded prices.json | 9dfc5d6 | prices.json |

## What Was Built

### PriceItem schema extension (src/models/prices.rs)

`PriceItem` now has:
- `base_type: String` — required (was `Option<String>` with `#[serde(default)]`)
- `class: String` — new required field, maps to `Class ==` in filter blocks
- `category: String` — new required field, groups items into block sections
- `exalt/chaos/div: Option<f64>` — unchanged (nullable price values)

`#[serde(deny_unknown_fields, rename_all = "camelCase")]` is preserved.

### seed_prices binary (src/bin/seed_prices.rs)

One-time migration tool. Reads `currency.json`, builds `prices.json` with:
1. Items[0..2]: Exalted Orb (4.5c), Chaos Orb (1.0c), Divine Orb (220.0c) — positional anchors per D-03/SYNC-02
2. Items[3..357]: all other bases in currency.json file order (category order, S→F within, bases in file order)
3. De-dup: Chaos Orb, Exalted Orb, Divine Orb are skipped during the currency walk (appear only as anchors)
4. D-09 tier lower-bounds: S=1100c, A=220c, B=30c, C=10c, D=2c, E=0.5c, F=0.1c
5. Reference orb overrides: Chaos=1.0c, Exalt=4.5c, Divine=220.0c (realistic values that re-tier them per D-09)
6. All items: `exalt=null`, `div=null`, `chaos=Some(value)` — consistent chaos field usage

Note: The binary uses locally-duplicated minimal structs (not from crate modules) because the crate
has no `[lib]` target. The canonical models in `src/models/currency.rs` and `src/models/prices.rs`
are unchanged for the migration path.

### Seeded prices.json (committed artifact)

- 358 items total (3 anchors + 355 others; exactly 0 duplicates)
- 15 categories in currency.json file order
- Every item: `name == baseType` (zero uniques — D-08 preserved)
- Every item: `class` and `category` populated
- First 3 items: Exalted Orb / Chaos Orb / Divine Orb with correct anchor chaos values

## Tests

6 unit tests in `models::prices::tests`, all passing:
- `load_real_prices_json_parses` — parses committed prices.json under new schema
- `round_trip_prices_file` — serialize/deserialize round-trip
- `item_with_null_exalt_parses` — updated to include class/category/baseType
- `item_with_base_type_parses` — updated to include class/category
- `seeded_prices_anchors_first` — asserts items.len()>300, first 3 are Exalt/Chaos/Divine
- `seeded_prices_no_uniques` — asserts every item has name == baseType

Full suite: 34 unit tests + 7 CLI integration tests — all pass.

## Deviations from Plan

None — plan executed exactly as written.

The TDD sequence was: schema change + updated inline tests committed as RED (`test(03-01)` commit
18a8a76), then the seed binary + committed prices.json as consecutive GREEN commits. The 4 tests
that depend on prices.json were in the expected FAILED state during RED and pass in GREEN because
the schema change and seed generation are coupled (prices.json must match the new schema).

## Known Stubs

None — all items have fully populated class/category/baseType/chaos values. The `exalt` and `div`
fields are intentionally `null` in the seed (all values expressed in chaos per D-09/A3).

## Threat Flags

None — seed_prices reads/writes only in-repo trusted files. No new network endpoints, auth paths,
or schema changes at external trust boundaries.

## Self-Check: PASSED

- FOUND: src/models/prices.rs
- FOUND: src/bin/seed_prices.rs
- FOUND: prices.json
- FOUND: Cargo.toml
- FOUND commit: 18a8a76 (test RED)
- FOUND commit: ef64db9 (feat seed_prices binary)
- FOUND commit: 9dfc5d6 (feat seeded prices.json)
- cargo test: 34+34 unit + 7 CLI = all pass
- prices.json: 358 items, 358 "category" fields, 0 name!=baseType
