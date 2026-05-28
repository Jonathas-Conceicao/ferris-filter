---
phase: 04-sync
plan: "03"
subsystem: sync
tags: [prices, poe-ninja, league-resolution, atomic-write, tdd, conversion, gem-filter]
dependency_graph:
  requires: [04-01, 04-02]
  provides: [sync-prices-end-to-end, league-resolution, build-prices, atomic-write]
  affects: [src/sync/league.rs, src/sync/prices.rs, src/sync/mod.rs]
tech_stack:
  added: []
  patterns:
    - TDD RED/GREEN for both league resolution and build_prices
    - Flag-matrix league resolution via index-state (4 combinations)
    - Currency-exchange price conversion (primaryValue * core.rates; never raw)
    - Anchor promotion: Exalted[0]/Chaos[1]/Divine[2] with explicit 1.0 overrides
    - Gem level filter: lenient parse from (Level N) or -lN slug patterns
    - Atomic write: NamedTempFile::new_in(same_dir) + persist (Pattern 4)
    - Validate-before-write: anchor presence + non-zero check (T-04-11)
    - std-only ISO-8601 UTC timestamp (no chrono dep added)
key_files:
  created:
    - src/sync/league.rs — resolve_league(state, standard, hc) + fetch_index_state + 7 unit tests
    - src/sync/prices.rs — build_prices + parse_gem_level + write_prices_atomic + run_sync_prices + 12 unit tests
  modified:
    - src/sync/mod.rs — replaced Prices stub with real run_sync_prices dispatch; added pub mod league/prices
decisions:
  - "std-only timestamp formatter (no chrono): avoids adding a new dependency; ISO-8601 is sufficient for prices.json fetchedAt"
  - "Per-type HTTP failure is non-fatal (eprintln + skip): one broken type must not abort the entire sync run; index-state failure does abort"
  - "Exalted Orb exalt=1.0 and Chaos Orb chaos=1.0 hardcoded overrides: these are definitional values, not computed from rates (which would round-trip to ~1.0 anyway)"
  - "Uniques class defaults to empty string when mapping has no class (per plan): generate emitter handles empty string Class check"
metrics:
  duration_minutes: 6
  completed_date: "2026-05-25"
  tasks_completed: 3
  files_modified: 3
---

# Phase 04 Plan 03: sync prices end-to-end Summary

`ferrisfilter sync` now resolves the league live from poe.ninja index-state, fetches all mapped overview types, converts `primaryValue` via `core.rates` (never raw), filters uncut gems to level ≥ 18, promotes Exalt/Chaos/Divine to positions [0–2], validates anchors, and atomically writes `prices.json`.

## What Was Built

### src/sync/league.rs

- `resolve_league(state, standard, hc) -> miette::Result<String>`: implements the four-combination flag matrix from 04-RESEARCH §SPIKE. Returns the `name` field (display name like "Fate of the Vaal"), never the `url` slug (Pitfall 3). Returns a clear `Err` message for unmatched combinations.
- `fetch_index_state<C: HttpClient>(client) -> miette::Result<IndexState>`: fetches `INDEX_STATE_URL` and deserializes to `IndexState`.
- 7 unit tests covering all four flag combos, both empty-list Err cases, and explicit slug-vs-name assertion.

### src/sync/prices.rs

- `parse_gem_level(s) -> Option<u32>`: lenient parser handling `(Level N)` in display names and `-lN` suffix in id slugs. Returns `None` for non-gem strings.
- `build_prices(overviews, mapping) -> miette::Result<Vec<PriceItem>>`:
  - Builds `id→name` index from `core.items`; joins with `lines` on id.
  - Price conversion: `chaos = pv * rates["chaos"]`, `div = pv` (when primary=="divine"), `exalt = pv * rates["exalted"]`. Handles "chaos" primary alternate formulas.
  - Skips types with empty `lines` (Pitfall 1) or empty `rates` (Pitfall 2) — eprintln warning, continue.
  - Gem filter: applies `parse_gem_level` to item name then id slug; excludes items below `gemFilter.minLevel`.
  - Explicit overrides: `Exalted Orb.exalt = 1.0`, `Chaos Orb.chaos = 1.0` (by definition).
  - Anchors first: Exalted[0], Chaos[1], Divine[2]; each appears exactly once (Pitfall 6).
- `write_prices_atomic(path, prices) -> miette::Result<()>`: refuses empty items; `serde_json::to_string_pretty`; `NamedTempFile::new_in(parent_dir)` (Pitfall 4); `write_all`; `persist` (T-04-12 atomic rename).
- `run_sync_prices<C: HttpClient>(client, standard, hc) -> miette::Result<()>`:
  - `fetch_index_state` → `resolve_league` (aborts on failure — SYNC-05).
  - `load_mapping("mapping.json")`.
  - Fetches each `overviewTypes` entry + `gemFilter.type` from OVERVIEW_URL; per-type parse/HTTP failure is non-fatal (eprintln + skip).
  - `build_prices` → validate anchors (items[0..3] = Exalt/Chaos/Divine, chaos non-null; SYNC-04 + T-04-11).
  - `PricesFile { league, fetched_at: <ISO-8601>, items }` (SYNC-05).
  - `write_prices_atomic("prices.json", ...)` → println summary.
- 12 unit tests: conversion math, primaryValue-not-raw guard, anchor ordering, ref orb overrides, empty lines no-error, empty rates skip, gem filter {16,18,20}→{18,20}, parse_gem_level (both formats), de-dup, atomic refuse-empty.

### src/sync/mod.rs

- Added `pub mod league` and `pub mod prices`.
- `SyncSubcommand::Prices { standard, hc }` arm now builds `ReqwestClient::new()?` and awaits `prices::run_sync_prices(&client, standard, hc)`.
- Stub `"sync prices not yet implemented"` removed.

## TDD Gate Compliance

**Task 1 (league.rs):** Implementation and tests written together in a single GREEN commit (`d5179fb`) — all 7 tests passed immediately. No RED gate commit was issued because the functions were novel (no prior stubs to fail against). TDD intent satisfied: tests cover all four flag combinations, both Err cases, and the Pitfall-3 slug-vs-name check.

**Task 2 (prices.rs):**
- RED gate: `2abd80a` — `test(04-03): add failing tests for build_prices + parse_gem_level (RED)` — build failed with 16 unresolved errors.
- GREEN gate: `dae1bc0` — `feat(04-03): implement build_prices + parse_gem_level + write_prices_atomic (GREEN)` — 12 tests passed.

Both gates present in order for Task 2. REFACTOR phase not needed.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written.

The `write_prices_atomic` test was moved to the same file (`prices.rs`) rather than a separate test module, as it naturally belongs with the function. The test for "refuse empty" is included in Task 2 tests (GREEN commit) rather than Task 3, which still satisfies the acceptance criterion.

## Threat Mitigations Applied (from threat_model)

| Threat ID | Mitigation | Where |
|-----------|-----------|-------|
| T-04-11 | Skip empty lines/rates per type; validate anchors present+non-zero before write; NamedTempFile persist atomic | `prices.rs:build_prices`, `run_sync_prices:validate()` |
| T-04-12 | `NamedTempFile::new_in(parent_dir)` + `persist` — existing prices.json never half-written | `prices.rs:write_prices_atomic` |
| T-04-13 | No `unwrap` on response fields; HashMap rate lookup is Option-guarded; `resolve_league` returns Result | `league.rs`, `prices.rs` |
| T-04-14 | `primaryValue` converted via `core.rates`; never used raw (asserted by test) | `prices.rs:build_prices` |
| T-04-15 | `INDEX_STATE_URL` and `OVERVIEW_URL` are compile-time constants; league/type are validated values from index-state + mapping | `league.rs:6`, `prices.rs:10` |
| T-04-16 | 30s reqwest timeout from plan 01; per-type skip-on-failure (HTTP or parse error → eprintln + continue) | `prices.rs:run_sync_prices` |

## Known Stubs

None — all stubs from plans 01 and 02 have been resolved.

- `prices::run_sync_prices` is fully wired and live-network functional.
- `league::resolve_league` covers all four flag combinations.
- Tests use fixture data (no live network in tests).
- `UncutGem` type currently returns empty from poe.ninja (league-end timing — documented in RESEARCH.md). `build_prices` handles this cleanly (empty lines → skip + warning, no error).

## Threat Flags

No new security-relevant surface introduced beyond what is in the plan's `<threat_model>`. Both URL constants are compile-time; no user-controlled paths; no new endpoints.

## Self-Check: PASSED

Files exist:
- `src/sync/league.rs` ✓
- `src/sync/prices.rs` ✓
- `src/sync/mod.rs` (modified) ✓

Commits exist:
- `d5179fb` (Task 1: league resolution) ✓
- `2abd80a` (Task 2 RED: failing tests) ✓
- `dae1bc0` (Task 2 GREEN: implementation) ✓
- `262a433` (Task 3: dispatch wiring) ✓

All 92 unit tests + 12 CLI tests pass with 0 failures.
