---
phase: 05-rule-schema
plan: "02"
subsystem: data-model
tags: [rust, serde, rule-schema, config, cleanup]
dependency_graph:
  requires: [rule-schema-types]
  provides: [config-rules-field, config-class-groups-field, dead-band-types-removed]
  affects: [src/models/config.rs, config.json]
tech_stack:
  added: []
  patterns: [optional-additive-fields, serde-default, deny_unknown_fields, round-trip-tests]
key_files:
  modified: [src/models/config.rs, config.json]
decisions:
  - rules and class_groups are Optional with serde(default) so the existing config.json parses with both as None (additive, no migration required until Phase 6)
  - areaLevelBands field removed from Config and config.json — deny_unknown_fields was already enforcing this; the emitter only referenced the camelCase name in writeln! comment literals, not as a type/field consumer
  - Four rarity_rule_* tests deleted along with the RarityRule enum; no replacement needed (RarityComparator from Plan 01 supersedes the old RarityRule for the new schema)
  - bad_rarity_in_rule_reports_error uses plain serde_json::from_str assertion rather than serde_path_to_error in the test, with a comment noting loader.rs provides field-path context at runtime
metrics:
  duration_minutes: 12
  completed_date: "2026-06-05"
  tasks_completed: 3
  files_modified: 2
---

# Phase 5 Plan 02: Config Rules Field + Cleanup — Summary

**One-liner:** Wired optional `rules: Option<Vec<Rule>>` and `class_groups: Option<HashMap<String, Vec<String>>>` into `Config`, deleted the dead `areaLevelBands`/`RarityRule`/`BandShow`/`ArmourShow`/`AreaLevelBand` scaffolding from both `config.rs` and `config.json`, and added round-trip + order-preservation + bad-rarity tests proving MIG-01.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add rules + classGroups optional fields to Config | 3f6392f | src/models/config.rs |
| 2 | Round-trip + order-preservation + parity tests | 4826cac | src/models/config.rs |
| 3 | Remove dead areaLevelBands field + types | ae241cb | src/models/config.rs, config.json |

## What Was Built

**Task 1:** Added two optional fields to `Config`:

- `pub rules: Option<Vec<Rule>>` — ordered declarative policy rules (RULE-01); `serde(default, skip_serializing_if = "Option::is_none")` so existing `config.json` (no `rules` key) parses with `None`.
- `pub class_groups: Option<HashMap<String, Vec<String>>>` — class-alias map for Phase 6 `when.class` reuse; same serde attributes. Maps to `classGroups` in JSON via the struct's existing `rename_all = "camelCase"`.

New test `config_with_rules_and_class_groups_parses` confirms the injection path (string-replace into real `config.json`) and the baseline `None` case.

**Task 2:** Three new tests proving MIG-01:

- `REPRESENTATIVE_RULES_JSON` const — exercises all condition types: class list, rarity comparator (>=), areaLevel min+max, baseArmour/baseEvasion eq, baseEnergyShield min, quality min, sockets min, baseType list, unidentifiedTier min, plus classGroups.
- `rules_preserve_authored_order` — authored order F → S → unchanged is NOT sorted; deserialized order matches exactly.
- `config_with_rules_round_trips` — full `Config` with representative rules+classGroups deserializes, serializes, re-deserializes to an equal value (lossless round-trip via `PartialEq`).
- `bad_rarity_in_rule_reports_error` — `"Legendary"` in `when.rarity` returns `Err` from plain `serde_json::from_str`; loader.rs provides field-path context at runtime.

**Task 3:** Removed dead band scaffolding:

- Deleted `RarityRule` enum (D-10 — parsed but never consumed outside `config.rs`)
- Deleted `ArmourShow`, `BandShow`, `AreaLevelBand` structs (entire `AreaLevelBands section` comment block)
- Removed `pub area_level_bands: Vec<AreaLevelBand>` from `Config`
- Removed `"areaLevelBands": [...]` block (lines 34-40) from `config.json` (required by `deny_unknown_fields`)
- Deleted four `rarity_rule_*` tests (`rarity_rule_variants_roundtrip`, `rare_alias_maps_to_rare_only`, `rarity_rule_rejects_unknown`, `rarity_rule_all_variants_parse`)
- Removed `"areaLevelBands": []` from `config_rejects_unknown_key` test JSON
- `src/emitter.rs` not touched: all its `areaLevelBands` references are inside `writeln!` comment string literals — confirmed by grep before deletion

## Tests Added / Removed

Added (4): `config_with_rules_and_class_groups_parses`, `rules_preserve_authored_order`, `config_with_rules_round_trips`, `bad_rarity_in_rule_reports_error`

Removed (4): `rarity_rule_variants_roundtrip`, `rare_alias_maps_to_rare_only`, `rarity_rule_rejects_unknown`, `rarity_rule_all_variants_parse`

Net change: 0 tests added/removed (119 total lib tests after Plan 01's additions; 123 lib tests remain — 4 added, 4 removed = net 0 change in count).

Wait — Plan 01 ended at 123 lib tests. After Task 3 removes 4 and adds 4 the total stays at 123. Confirmed by final `cargo test` run.

## Verification

- `cargo build`: exits 0, no errors
- `cargo test`: 123 lib tests + 4 cli_errors + 4 cli_filter_copy + 9 cli_generate + 8 sync_prices + 1 doc-test — all pass (0 failures)
- `artifact_guard` and `generate_snapshot`: pass with byte-identical filter output (emitter unchanged)
- `git diff src/emitter.rs`: empty (no changes)
- grep `pub enum RarityRule` in src/: no matches
- grep `pub struct AreaLevelBand|BandShow|ArmourShow` in src/: no matches
- grep `area_level_bands` in src/: no matches
- grep `areaLevelBands` in config.json: no matches

## Deviations from Plan

None — plan executed exactly as written. One minor auto-fix applied: the initial `config_with_rules_and_class_groups_parses` test used `trim_end_matches('}')` string surgery which failed due to trailing whitespace in `config.json`; replaced with `replacen()` insertion before `"fallthrough":` (same functional outcome, more robust). This is an implementation detail of the test, not a plan deviation.

## Known Stubs

None — pure data-model changes with no UI rendering or placeholder values.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries introduced. T-05-04 (deny_unknown_fields on Config/Rule/When rejects mistyped keys) is fully mitigated: `rules` and `class_groups` are optional so partial migration cannot silently drop policy; all three deny_unknown_fields guards (Config, Rule, When) remain in place after the dead-type removal.

## Self-Check: PASSED

- `src/models/config.rs` exists and contains `pub rules: Option<Vec<Rule>>` and `pub class_groups: Option<HashMap<String, Vec<String>>>`
- `config.json` no longer contains `"areaLevelBands"`
- Commits `3f6392f`, `4826cac`, `ae241cb` all exist in git log
- No files deleted in any commit (only content changes within tracked files)
- `cargo test` exits 0 with all tests passing
