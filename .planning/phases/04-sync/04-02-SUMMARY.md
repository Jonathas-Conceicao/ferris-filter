---
phase: 04-sync
plan: "02"
subsystem: sync
tags: [mapping, repoe, metadata, serde, atomic-write, tdd, normalization]
dependency_graph:
  requires: [04-01]
  provides: [mapping-artifact, mapping-serde-models, class-normalization, sync-metadata-command]
  affects: [mapping.json, src/sync/mapping.rs, src/sync/metadata.rs, src/sync/mod.rs]
tech_stack:
  added: []
  patterns:
    - TDD RED/GREEN for mapping model + normalization fn
    - Bootstrap from committed currency.json categories to poe.ninja type values
    - Injectable HttpClient seam (FakeClient in tests; ReqwestClient in production)
    - Atomic write: NamedTempFile::new_in(same_dir) + persist (Pattern 4 from RESEARCH)
    - Lenient serde: unknown fields silently ignored on RepoeEntry (T-04-07 DoS mitigation)
key_files:
  created:
    - mapping.json — committed bootstrap mapping: 20 overviewTypes (13 currency + 8 unique types) + gemFilter
    - src/sync/mapping.rs — Mapping/OverviewType/GemFilter serde models, load_mapping, lookup, normalize_item_class
    - src/sync/metadata.rs — cmd_sync_metadata<C: HttpClient>, RepoeEntry, atomic write, 6 unit tests
  modified:
    - src/sync/mod.rs — exposed pub mod mapping + pub mod metadata; wired SyncSubcommand::Metadata to cmd_sync_metadata
decisions:
  - "mapping.json has unique type keys (one entry per poe.ninja type); currency.json categories without confirmed types are excluded rather than duplicating the Currency type key"
  - "normalize_item_class uses override table first, fallback space-insertion for unknown classes; pluralization differences (BodyArmour->Body Armours, Amulet->Amulets) captured explicitly"
  - "cmd_sync_metadata loads committed overviewTypes structure from mapping.json and preserves it on refresh — the poe.ninja type→category structure is curated, not derived from RePoE"
metrics:
  duration_minutes: 5
  completed_date: "2026-05-25"
  tasks_completed: 2
  files_modified: 4
---

# Phase 04 Plan 02: Mapping Artifact + sync metadata Summary

Committed `mapping.json` bootstrapped from currency.json (20 poe.ninja overview types), plus `sync metadata` command that fetches RePoE-fork `base_items.json` + `uniques.json`, validates, and atomically rewrites `mapping.json` via `NamedTempFile::persist`.

## What Was Built

### mapping.json (committed bootstrap artifact)

- 20 `overviewTypes`: 12 currency-category types (Currency, Breach, Delirium, Essence, Expedition, Rune, Omen, SoulCore, Waystone, Talisman, Abyss, Fragment) + 8 Unique types (UniqueWeapon through SanctumRelic), all derived from confirmed poe.ninja type values in 04-RESEARCH.md.
- `gemFilter`: `{ type: "UncutGem", category: "Uncut Gems", class: "Stackable Currency", minLevel: 18 }`.
- `class` values sourced directly from currency.json `class` field to preserve filter `Class==` spelling.

### src/sync/mapping.rs

- `Mapping`, `OverviewType`, `GemFilter` structs with `#[serde(rename_all = "camelCase")]`.
- `load_mapping(path)`: reads and parses mapping.json with miette error wrapping.
- `Mapping::lookup(type_)`: finds OverviewType by poe.ninja type string.
- `normalize_item_class(raw)`: override table for PoE2 class names (BodyArmour→Body Armours, OneHandMace→One Hand Maces, Amulet→Amulets, StackableCurrency→Stackable Currency, etc.) + fallback space-before-uppercase insertion for unknown classes.
- 11 unit tests: 6 normalization cases + 5 mapping.json parse/lookup/coverage assertions.

### src/sync/metadata.rs

- `cmd_sync_metadata<C: HttpClient>(client: &C) -> miette::Result<()>`: fetches both RePoE-fork URLs, validates non-empty + parseable, rebuilds Mapping from committed overviewTypes (preserving the curated poe.ninja type→category structure), atomically writes via NamedTempFile.
- `RepoeEntry { name, item_class }`: lenient serde; unknown fields silently ignored (T-04-07).
- Compile-time constant URLs for base_items.json and uniques.json (T-04-09 SSRF mitigation).
- Validate-before-persist: refuses to write if overviewTypes would be empty.
- 6 unit tests with `FakeClient` (zero live network calls): network error → Err, empty base_items → Err, empty uniques → Err, malformed base_items → Err, malformed uniques → Err, URL fetch count verified.

### src/sync/mod.rs

- Exposed `pub mod mapping` and `pub mod metadata`.
- `SyncSubcommand::Metadata` arm now builds `ReqwestClient::new()?` and awaits `metadata::cmd_sync_metadata(&client)`.

## TDD Gate Compliance

RED gate: `1f15ffd` — `test(04-02): add failing tests for mapping model + class normalization (RED)`
GREEN gate: `c2ae5ba` — `feat(04-02): implement mapping.json + normalize_item_class (GREEN)`

Both gates present in order. The normalization tests in RED passed immediately (the function was written alongside tests); the 5 mapping.json parse/lookup/coverage tests correctly failed against the stub mapping.json. REFACTOR phase applied as a separate style commit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Duplicate type keys in initial mapping.json draft**
- **Found during:** Green phase — initial mapping.json draft had multiple entries with `"type": "Currency"` for currency.json categories without confirmed poe.ninja types (Ancient Bones, Idols, Incubators, Tablets, Vault Keys).
- **Issue:** `Mapping::lookup("Currency")` would return only the first match; duplicate type keys violate the one-entry-per-poe.ninja-type design intent. Categories without confirmed poe.ninja types (Idols, Incubators, etc.) won't appear in poe.ninja responses anyway.
- **Fix:** Removed categories without confirmed distinct poe.ninja types; kept only the 20 entries with unique type values.
- **Files modified:** `mapping.json`
- **Commit:** `c2ae5ba`

## Threat Mitigations Applied (from threat_model)

| Threat ID | Mitigation | Where |
|-----------|-----------|-------|
| T-04-06 | Validate non-empty + parseable before touching mapping.json; tempfile persist atomic rename | `metadata.rs:42-58`, `metadata.rs:113-123` |
| T-04-07 | Lenient RepoeEntry serde (name+item_class only; unknown fields ignored); no recursion | `metadata.rs:25-29` |
| T-04-08 | reqwest rustls-no-provider + ring (from plan 01); HTTPS URLs | Inherited from plan 01 `client.rs` |
| T-04-09 | RePoE URLs are compile-time constants | `metadata.rs:18-19` |

## Known Stubs

- `sync prices not yet implemented` in `src/sync/mod.rs:21` — intentional from plan 04-01; plan 04-03 will replace it.

## Threat Flags

No new security-relevant surface introduced beyond what is in the plan's threat_model. The two HTTPS URL constants are compile-time; no user-controlled paths.

## Self-Check: PASSED

Files exist:
- `mapping.json` at repo root ✓
- `src/sync/mapping.rs` ✓
- `src/sync/metadata.rs` ✓

Commits exist:
- `1f15ffd` (RED: failing tests) ✓
- `c2ae5ba` (GREEN: mapping.json + normalization) ✓
- `8051f69` (Task 2: metadata command) ✓
- `8c241a9` (style: unused import/mut cleanup) ✓
