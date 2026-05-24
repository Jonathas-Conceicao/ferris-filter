---
phase: "01-foundation"
plan: "02"
subsystem: "data-models"
tags: ["serde", "config", "currency", "json", "tdd"]
dependency_graph:
  requires: ["01-01"]
  provides: ["config.json typed model", "currency.json typed model", "FilterColor", "StyleAction", "MinimapShape"]
  affects: ["01-03", "phase-02-generator"]
tech_stack:
  added: []
  patterns:
    - "deny_unknown_fields on all config + currency structs"
    - "Option<[u8;3]> RGB triples (alpha at generate-time)"
    - "HashMap<String, Style> for tier map (BTreeMap avoided — deny_unknown_fields+flatten pitfall N/A for HashMap)"
    - "Sound struct with two Option fields (id + custom) per RESEARCH Open Question 1"
    - "PartialEq-based semantic round-trip (not string comparison) to avoid HashMap ordering non-determinism"
key_files:
  created:
    - path: "config.json"
      note: "Renamed from filter-model.json via git mv (history preserved)"
    - path: "currency.json"
      note: "15 curated currency categories extracted from universal.filter"
  modified:
    - path: "src/models/config.rs"
      note: "Full Config model with FilterColor/MinimapShape/StyleAction enums, all sub-structs, 6 inline tests"
    - path: "src/models/currency.rs"
      note: "CurrencyData/CurrencyCategory/CurrencyEntry model, 5 inline tests"
decisions:
  - "Tier String (not enum) in currency.json entries — parallel wave ownership prohibits importing Tier from plan 01-03 (RESEARCH Open Question 2)"
  - "PartialEq on deserialized structs for round-trip assertion instead of re-serialized string comparison (HashMap order non-determinism)"
  - "BandShow models areaLevelBands.show with all optional fields — rarity DSL stays String per D-07"
metrics:
  duration: "4 minutes"
  completed_date: "2026-05-24"
  tasks_completed: 2
  files_modified: 4
---

# Phase 1 Plan 2: Config + Currency Data Models Summary

JSON policy data migrated into the crate: filter-model.json renamed to config.json via `git mv` (history preserved), 15 curated currency categories extracted from universal.filter into currency.json, and both files modeled with strict `deny_unknown_fields` serde structs that pass 11 inline TDD tests.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Rename filter-model.json and extract currency.json | b015c9a | config.json (renamed), currency.json (new) |
| 2 (RED) | Add failing tests for Config + CurrencyData | f2f2d94 | src/models/config.rs, src/models/currency.rs |
| 2 (GREEN) | Implement Config and CurrencyData serde models | be98578 | src/models/config.rs, src/models/currency.rs |

## What Was Built

### config.json
`filter-model.json` renamed to `config.json` via `git mv` — rename history fully preserved in git log.

### currency.json
15 currency categories extracted verbatim from `universal.filter`:
Currency, Abyssal Eyes, Ancient Bones, Breach, Delirium, Essences, Expedition, Idols, Incubators, Map Fragments & Pinnacle Keys, Omens, Runes, Soul Cores, Tablets, Vault Keys.

Schema: `{ "categories": [{ "name", "entries": [{ "tier", "class", "bases": [...] }] }] }`.

### src/models/config.rs
Complete typed model for `config.json`:
- **Closed-vocabulary enums (D-07):** `FilterColor` (11 variants), `MinimapShape` (12 variants), `StyleAction` (Show/Hide)
- **Sub-structs:** `Minimap`, `Beam`, `Sound` (struct-with-two-optionals per RESEARCH Open Question 1)
- **`Style`:** `deny_unknown_fields` + `rename_all = "camelCase"`, `Option<[u8;3]>` RGB triples, all visual fields optional
- **`Build`, `ArmourShow`, `BandShow`, `AreaLevelBand`:** full areaLevelBands coverage, rarity strings kept as `String` (D-07)
- **`AlwaysShow`, `Meta`, `Config`:** top-level with `deny_unknown_fields`
- **No `#[serde(flatten)]` anywhere** (Pitfall 1 avoided)
- 6 inline `#[cfg(test)]` tests covering: parse real file, round-trip, deny_unknown_fields rejection, bad-enum rejection, FilterColor round-trip, StyleAction variants

### src/models/currency.rs
- `CurrencyEntry`, `CurrencyCategory`, `CurrencyData` all with `deny_unknown_fields`
- `tier` field stays `String` for Phase 1 (cross-wave dependency constraint — see Decisions)
- 5 inline tests: parse real file, round-trip, unknown key rejection, 15-category count, Mirror of Kalandra sanity check

## Test Results

```
cargo build  → exit 0 (2 expected warnings: dead_code on JsonLoadError/load_json, benign ff alias warning)
cargo test   → 11 passed; 0 failed
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Round-trip test compared serialized HashMap strings**
- **Found during:** Task 2 GREEN phase — `config_round_trips` test failed
- **Issue:** `serde_json::to_string` of a `HashMap<String, Style>` produces non-deterministic key ordering. First serialize → second serialize produced different key orders even though values were semantically identical.
- **Fix:** Changed round-trip assertion from `assert_eq!(serialized_string_1, serialized_string_2)` to `assert_eq!(first_config, second_config)` using `PartialEq` on the deserialized structs. This correctly validates D-06 semantic equivalence without fragile string comparison.
- **Files modified:** `src/models/config.rs`
- **Commit:** be98578

## TDD Gate Compliance

- RED gate: f2f2d94 `test(01-02): add failing tests for Config and CurrencyData models` — compile error confirmed RED state (17 undeclared type errors)
- GREEN gate: be98578 `feat(01-02): implement Config and CurrencyData serde models` — all 11 tests pass

## Known Stubs

None — all data is wired. `config.json` is the renamed real policy file; `currency.json` contains the actual curated base lists from `universal.filter`. Both deserialize fully.

## Threat Surface Scan

No new network endpoints, auth paths, or file access patterns introduced. Files `config.json` and `currency.json` are local, hand-edited policy files parsed at CLI invocation time — this was already in the plan's threat model (T-01-04, T-01-05, T-01-06). All three mitigations are in place:
- T-01-04 (typo'd key): `deny_unknown_fields` on all structs — verified by `config_rejects_unknown_key` and `currency_rejects_unknown_key` tests
- T-01-05 (panic on malformed input): models are pure data; loading goes through `load_json` (Plan 01) which returns `Err` not panic
- T-01-06 (invalid closed-vocabulary): `FilterColor`, `MinimapShape`, `StyleAction` enums reject bad values — verified by `config_rejects_bad_enum_color` test

## Self-Check: PASSED

All created files verified on disk. All task commits confirmed in git history.

| Item | Status |
|------|--------|
| config.json | FOUND |
| currency.json | FOUND |
| src/models/config.rs | FOUND |
| src/models/currency.rs | FOUND |
| 01-02-SUMMARY.md | FOUND |
| Commit b015c9a (Task 1) | FOUND |
| Commit f2f2d94 (Task 2 RED) | FOUND |
| Commit be98578 (Task 2 GREEN) | FOUND |
