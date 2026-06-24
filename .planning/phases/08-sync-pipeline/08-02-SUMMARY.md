---
phase: 08-sync-pipeline
plan: "02"
subsystem: sync/prices + pricer + emitter
tags: [rust, gem-dispatch, multi-class, fixture-rebaseline, emitter, pricer]
dependency_graph:
  requires: [mapping-v2-schema, GemFilter-vec, Verisium-overviewType]
  provides: [per-level-gem-PriceItems, multi-class-emit, Verisium-blocks, background-override-carry]
  affects:
    - src/sync/prices.rs
    - src/pricer.rs
    - src/emitter.rs
    - tests/sync_prices.rs
    - tests/cli_generate.rs
    - tests/fixtures/seed-prices.json
    - tests/fixtures/seed-universal.filter
    - tests/snapshots/cli_generate__generate_snapshot.snap
tech_stack:
  added: []
  patterns:
    - prefix-match class derivation (derive_uncut_class)
    - pre-quoted multi-class string merged at pricer boundary (UNCUT_GEM_MULTICLASS)
    - background_override carry field on CategoryBlock (Phase 9 consumes)
key_files:
  created: []
  modified:
    - src/sync/prices.rs
    - src/pricer.rs
    - src/emitter.rs
    - tests/sync_prices.rs
    - tests/cli_generate.rs
    - tests/fixtures/seed-prices.json
    - tests/fixtures/seed-universal.filter
    - tests/snapshots/cli_generate__generate_snapshot.snap
decisions:
  - "derive_uncut_class uses prefix-match (not lookup table) for Skill/Spirit/Support gem class derivation"
  - "Pre-quoted multi-class string stored as UNCUT_GEM_MULTICLASS const and emitted verbatim via contains(chr) check"
  - "category_background() derives background_override from category name (no mapping.json access in pricer)"
  - "Pricer merges all Uncut gem subtypes per-tier into one TierGroup at the pricer boundary (not emitter)"
  - "Fixed 3 pre-existing pricer test failures from tiers.json cutoff change (Divine: A→S at 30c floor)"
  - "generate_block_counts updated to 74 Show (from 84) due to tiers.json + seed data change"
metrics:
  duration_minutes: 12
  completed_date: "2026-06-24"
  tasks_completed: 3
  files_changed: 8
---

# Phase 8 Plan 2: Sync Gem Dispatch Rewrite + Emitter Multi-class Support Summary

**One-liner:** Rewrote the build_prices gem branch for per-level uncut gem PriceItems with prefix-derived plural class, merged subtypes in pricer into one multi-class TierGroup, added emitter multi-class emit support, and re-baselined all fixtures to make the full suite green.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Rewrite build_prices gem dispatch (per-level, prefix-class, no strip/dedup/minLevel) | c9faa50 | src/sync/prices.rs |
| 2 | Multi-class emitter support + pricer background-field carry | a237bec | src/pricer.rs, src/emitter.rs, src/sync/prices.rs |
| 3 | Re-baseline fixtures (seed-prices, seed-universal, insta snapshot, block counts) | e9bcfe6 | tests/fixtures/seed-prices.json, tests/fixtures/seed-universal.filter, tests/snapshots/cli_generate__generate_snapshot.snap, tests/sync_prices.rs, tests/cli_generate.rs, src/emitter.rs, src/pricer.rs |

## Verification

- `cargo test` (full): PASSED (123 lib tests + 25 integration tests, 0 failures)
- `grep -c "strip_gem_level" src/sync/prices.rs`: 0 (deleted)
- `grep -c "gem_best" src/sync/prices.rs`: 0 (deleted)
- `grep -c "min_level" src/sync/prices.rs`: 0 (removed)
- `grep -n "gem_filters" src/sync/prices.rs`: matches build_prices + run_sync_prices loop
- `grep -n "Uncut Skill Gems" src/sync/prices.rs`: matches derive_uncut_class
- `grep -n "background_override" src/pricer.rs`: CategoryBlock field present
- `grep -n "Uncut Skill Gems" src/pricer.rs`: UNCUT_GEM_MULTICLASS const + merge logic
- `grep -n 'contains.*"' src/emitter.rs`: multi-class check at line 406
- `grep -n "Class == \"Uncut Skill Gems\"" tests/fixtures/seed-universal.filter`: matches (4 times)
- `grep -n "Uncut Skill Gem (Level 20)" tests/fixtures/seed-universal.filter`: matches
- `grep -n "Show # Verisium" tests/fixtures/seed-universal.filter`: matches (S and A)
- `grep -n "Uncut Skill Gem (Level 20)" tests/fixtures/seed-prices.json`: matches
- `grep -n "Verisium" tests/fixtures/seed-prices.json`: matches (2 entries)
- `grep -n "Lineage Support Gems" tests/fixtures/seed-prices.json`: matches (2 entries)

## What Was Built

### Task 1 — build_prices gem dispatch rewrite (src/sync/prices.rs)

- **Deleted** `strip_gem_level` fn (was stripping `(Level N)` — the bug)
- **Deleted** `gem_best` HashMap (was collapsing levels to highest-value — the bug)
- **Added** `derive_uncut_class(base_type: &str) -> Option<String>` — prefix-match on `"Uncut Skill/Spirit/Support Gem"` stem → plural class
- **Replaced** single-entry `is_gem_type` check with `mapping.gem_filters.iter().find(...)` lookup
- **Rewrote** gem branch: emits ONE PriceItem per level, keeps full `(Level N)` in `base_type`, derives per-item class via `derive_uncut_class()` (fallback to `gem_filter.classes[0]` for LineageSupportGems)
- **Updated** `run_sync_prices`: replaced single-entry fetch with `for gf in &mapping.gem_filters { ... }` loop
- Updated `minimal_mapping()` / `gem_mapping()` fixtures to `gemFilters` array shape (v2)
- Replaced `gem_level_filter_keeps_only_min_level_and_above` with `gem_per_level_entries_keep_level_and_derive_class`
- Added `derive_uncut_class_from_base_type` unit test

### Task 2 — Pricer multi-class merge + emitter multi-class emit (src/pricer.rs, src/emitter.rs)

- **Added** `pub background_override: Option<[u8; 3]>` to `CategoryBlock`
- **Added** `category_background(name: &str) -> Option<[u8;3]>`: Uncut Gems → `[80, 70, 55]`, Lineage Support Gems → `[10, 150, 150]`, others → None
- **Added** `UNCUT_GEM_MULTICLASS` const: `"Uncut Skill Gems" "Uncut Spirit Gems" "Uncut Support Gems"` (pre-quoted, canonical order)
- **Added** Uncut Gems merge logic in `price_tiers`: for `cat == "Uncut Gems"`, union items across all three subtype classes per tier into one TierGroup with `class = UNCUT_GEM_MULTICLASS`
- **Updated** emitter `emit_currency_sections`: `if tier_group.class.contains('"')` → emit verbatim, else wrap in quotes

### Task 3 — Re-baseline fixtures + fix stale tests

- `tests/fixtures/seed-prices.json`: +9 entries (Verisium S/A, Uncut Skill/Spirit Gem L20/L19/L18, Lineage Support Gems B/D)
- `tests/fixtures/seed-universal.filter`: regenerated oracle with Verisium (2 blocks), Uncut Gems (4 blocks with multi-class Class line), Lineage Support Gems (2 blocks)
- `tests/snapshots/cli_generate__generate_snapshot.snap`: updated via `INSTA_UPDATE=always`
- `generate_block_counts`: updated 84 → 74 Show (tiers.json cutoff change removed mid-range tiers + new blocks added)
- **Fixed 3 pre-existing pricer test failures** (tiers.json was updated without updating tests):
  - `assign_tier_anchor_rertiering`: Divine (220c) → S (not A) with 30c S floor
  - `unique_base_collision_uses_highest_tier`: Headhunter (50c) → S (not B) with 30c S floor
  - `seeded_currency_category_anchor_retiers`: Divine lands S not A
- `tests/sync_prices.rs`: updated `minimal_mapping_str()` to `gemFilters` v2 shape; updated type key `"UncutGem"` → `"UncutGems"`; rewrote `gem_filter_excludes_level_16` → `gem_all_levels_survive_with_level_in_base_type`
- `tests/cli_generate.rs`: updated `currency_retiering_present` (Divine now S), `pricer_section_present_and_unchanged` (no Currency A/B with current seed)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed 3 pre-existing pricer test failures from stale tier expectations**
- **Found during:** Task 3 — full `cargo test` run
- **Issue:** `tiers.json` was updated (commit 7428f09: S=30c, A=20c) without updating pricer tests that expected old cutoffs (S=1100c, A=220c). `assign_tier_anchor_rertiering`, `unique_base_collision_uses_highest_tier`, and `seeded_currency_category_anchor_retiers` all failed.
- **Fix:** Updated test assertions to match current tiers.json values. Also fixed `generate_block_counts` (84→74 blocks) and `pricer_section_present_and_unchanged` (no Currency A/B in seed output).
- **Files modified:** src/pricer.rs, src/emitter.rs, tests/cli_generate.rs
- **Commit:** e9bcfe6

**2. [Rule 1 - Bug] Fixed UncutGem type key mismatch in sync_prices.rs integration tests**
- **Found during:** Task 3 — `happy_path_anchors_first_and_correct_values` failure
- **Issue:** Test passed gem overview as `"UncutGem"` but minimal_mapping_str now uses `"UncutGems"` (new schema v2 type name)
- **Fix:** Updated `"UncutGem".to_string()` → `"UncutGems".to_string()` in two places
- **Files modified:** tests/sync_prices.rs
- **Commit:** e9bcfe6

## Known Stubs

None — all new blocks are wired from seed-prices.json through pricer to emitter.

The `background_override` field on `CategoryBlock` is intentionally unread by the emitter this phase — Phase 9 will consume it. This is not a stub; it is by design and documented in the field's doc comment.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundaries introduced. The gem dispatch reads from the same poe.ninja exchange overview endpoint (existing T-08-03/T-08-04 mitigations apply); the per-level split only changes how many PriceItems are produced per API response, not the API surface.

## Self-Check: PASSED

| Check | Result |
|-------|--------|
| src/sync/prices.rs exists | FOUND |
| src/pricer.rs exists | FOUND |
| src/emitter.rs exists | FOUND |
| tests/fixtures/seed-prices.json exists | FOUND |
| tests/fixtures/seed-universal.filter exists | FOUND |
| tests/snapshots/cli_generate__generate_snapshot.snap exists | FOUND |
| Commit c9faa50 exists | FOUND |
| Commit a237bec exists | FOUND |
| Commit e9bcfe6 exists | FOUND |
| No unintended file deletions | PASSED |
| Full `cargo test` passes | PASSED (123+25 tests green) |
| seed-universal.filter contains multi-class uncut line | FOUND (line 655) |
| seed-universal.filter contains Uncut Skill Gem (Level 20) | FOUND (line 667) |
| seed-universal.filter contains Show # Verisium | FOUND (lines 631, 642) |
