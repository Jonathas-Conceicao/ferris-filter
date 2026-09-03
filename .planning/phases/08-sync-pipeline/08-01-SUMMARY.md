---
phase: 08-sync-pipeline
plan: "01"
subsystem: sync/mapping
tags: [schema, mapping, structs, rust, serde]
dependency_graph:
  requires: []
  provides: [mapping-v2-schema, GemFilter-vec, Verisium-overviewType]
  affects: [src/sync/prices.rs, src/sync/metadata.rs]
tech_stack:
  added: []
  patterns: [serde-camelCase-rename, Option-default-serde, Vec-of-structs]
key_files:
  created: []
  modified:
    - mapping.json
    - src/sync/mapping.rs
    - src/sync/metadata.rs
    - src/sync/prices.rs
decisions:
  - "Use classes: Vec<String> on GemFilter to model both multi-class UncutGems and single-class LineageSupportGems uniformly (Claude's Discretion per CONTEXT D-CTX-1)"
  - "Apply minimum prices.rs compat shim (first gem_filters entry, hardcoded level 18) to restore cargo build; Plan 02 rewrites the gem branch fully (Rule 3 auto-fix)"
metrics:
  duration_minutes: 5
  completed_date: "2026-06-24"
  tasks_completed: 3
  files_changed: 4
---

# Phase 8 Plan 1: Mapping Schema v2 (gemFilters Array + Verisium) Summary

**One-liner:** Migrated mapping.json to schema v2 with gemFilters array (UncutGems + LineageSupportGems + per-category backgroundOverride) and Verisium overviewType; Mapping/GemFilter Rust structs updated to match.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Reshape Mapping/GemFilter structs | b7dcf8b | src/sync/mapping.rs, src/sync/metadata.rs, src/sync/prices.rs |
| 2 | Rewrite mapping.json to schema v2 | a5a474a | mapping.json |
| 3 | Extend plural-spelling guard test | 5831fc5 | src/sync/mapping.rs |

## Verification

- `cargo build`: PASSED
- `cargo test --lib sync::mapping`: PASSED (13/13 tests)
- `grep -c "min_level" src/sync/mapping.rs`: 0 (removed)
- `grep -n "gem_filters" src/sync/mapping.rs`: Vec field on Mapping struct confirmed
- `grep -n "background_override" src/sync/mapping.rs`: Option<[u8;3]> field confirmed
- `grep -n '"version": 2' mapping.json`: matches
- `grep -n '"gemFilters"' mapping.json`: matches; singular gemFilter absent
- `grep -n '"LineageSupportGems"' mapping.json`: matches
- `grep -n '"Verisium"' mapping.json`: matches
- `grep -c 'minLevel' mapping.json`: 0

## What Was Built

### mapping.json (version 2)
- Bumped `version` 1 → 2
- Replaced singular `gemFilter` object with `gemFilters` array containing two entries:
  - **UncutGems**: `classes: ["Uncut Skill Gems", "Uncut Spirit Gems", "Uncut Support Gems"]`, `backgroundOverride: [80, 70, 55]`
  - **LineageSupportGems**: `classes: ["Support Gems"]`, `backgroundOverride: [10, 150, 150]`
- Added **Verisium** to `overviewTypes`: `class: "Stackable Currency"`, `baseTypeIsName: true`
- Removed `minLevel` entirely (D-CTX-3)

### src/sync/mapping.rs
- `Mapping.gem_filter: GemFilter` → `gem_filters: Vec<GemFilter>` (serde camelCase: `gemFilters`)
- `GemFilter` rewritten: `classes: Vec<String>` + `background_override: Option<[u8; 3]>` (serde default), no `min_level`
- Updated `mapping_json_parses` to assert `version == 2`
- Updated `mapping_contains_uncut_gem` to use `gem_filters.iter().any()`
- Extended `overview_types_use_correct_plural_spellings`:
  - Added `"Verisium"` to overviewTypes plural-spelling assert list
  - Added gem_filters assertions: LineageSupportGems and UncutGems must be present
  - Added bad-spelling guard: `"LineageSupportGem"` (singular) must NOT appear

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed metadata.rs compilation error from struct rename**
- **Found during:** Task 1 - initial cargo build after struct change
- **Issue:** `src/sync/metadata.rs:106` still referenced `gem_filter: committed.gem_filter` which no longer exists
- **Fix:** Renamed to `gem_filters: committed.gem_filters` (one-line change)
- **Files modified:** src/sync/metadata.rs
- **Commit:** b7dcf8b

**2. [Rule 3 - Blocking] Applied minimum compat shim to prices.rs to restore cargo build**
- **Found during:** Task 1 - cargo build after struct rename surfaced 5 errors in prices.rs
- **Issue:** prices.rs referenced `mapping.gem_filter` (singular), `.class` (renamed to `.classes: Vec`), and `.min_level` (removed); these are Plan 02's territory to rewrite
- **Fix:** Applied minimum changes to restore compilation:
  - `is_gem_type` now checks `gem_filters.first()` instead of `gem_filter`
  - Category/class tuple uses `first_gem.unwrap()` with `.classes.first().cloned()`
  - `min_level` reference replaced with hardcoded `18` + TODO(Plan 02) comment
  - Gem fetch block uses `gem_filters.first()` with empty-string guard
- **Behavior preserved:** Same runtime behavior (only first gem_filter entry fetched; Plan 02 loops all entries)
- **Files modified:** src/sync/prices.rs
- **Commit:** b7dcf8b

## Known Stubs

- `prices.rs` compat shim uses `gem_filters.first()` and hardcoded `min_level = 18` — intentional stub; Plan 08-02 rewrites the gem dispatch loop to iterate all `gem_filters` entries, implement per-level BaseType, and remove the min_level entirely.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundaries introduced. `mapping.json` remains a repo-controlled artifact; schema change surfaces as a parse error (guard tests enforce T-08-01).

## Self-Check: PASSED

| Check | Result |
|-------|--------|
| mapping.json exists | FOUND |
| src/sync/mapping.rs exists | FOUND |
| src/sync/metadata.rs exists | FOUND |
| src/sync/prices.rs exists | FOUND |
| 08-01-SUMMARY.md exists | FOUND |
| Commit b7dcf8b exists | FOUND |
| Commit a5a474a exists | FOUND |
| Commit 5831fc5 exists | FOUND |
| No unintended file deletions | PASSED |
