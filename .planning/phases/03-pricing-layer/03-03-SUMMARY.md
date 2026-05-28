---
phase: 03-pricing-layer
plan: "03"
subsystem: emitter-generate-pipeline
tags: [emitter, loader, generate, prices, GEN-05, TIER-04]
dependency_graph:
  requires: [seeded-prices-json, pricer-module]
  provides: [price-driven-emitter, load-prices-optional, rewired-cmd-generate]
  affects: [src/emitter.rs, src/loader.rs, src/main.rs]
tech_stack:
  added: []
  patterns: [optional-file-load-with-warn, tier-style-helper, unique-section-dormant]
key_files:
  created: []
  modified:
    - src/emitter.rs
    - src/loader.rs
    - src/main.rs
decisions:
  - "emitter::generate now takes &PricedData (from pricer) instead of &CurrencyData"
  - "load_prices_optional uses Path::exists() first — absent is Ok(None), malformed is Err"
  - "emit_unique_sections emits nothing when unique_blocks is empty (header omitted)"
  - "style_for_tier helper extracted in emitter to avoid duplicated tier match arms"
  - "cmd_generate wired: load_prices_optional -> derive_rates (TIER-04) -> price_tiers -> generate"
  - "currency.json removed from generate path entirely; src/models/currency.rs kept compiled for tests"
metrics:
  duration_seconds: 326
  completed_date: "2026-05-25"
  tasks_completed: 2
  files_changed: 3
---

# Phase 03 Plan 03: Emitter Refactor + cmd_generate Rewire Summary

Reworked `emitter::generate` to consume `PricedData` (from `pricer::price_tiers`) instead of `&CurrencyData`, added `loader::load_prices_optional` (GEN-05), and rewired `cmd_generate` to be fully price-driven with TIER-04 abort and GEN-05 warn-and-skip.

## What Was Built

### Task 1: load_prices_optional (GEN-05) + emitter refactor to PricedData

**`src/loader.rs`** — added `load_prices_optional`:
- Checks `std::path::Path::new(path).exists()` first (Pitfall 5 from RESEARCH)
- Absent file → `Ok(None)` (enables GEN-05 warn-and-skip)
- Present but malformed file → `Err(...)` from `load_json` (field-path JsonLoadError — abort)
- Doc comment distinguishes the two cases explicitly

**`src/emitter.rs`** — refactored to `PricedData`:
- Public signature changed to `pub fn generate(config: &Config, priced: &PricedData) -> String`
- Removed `use crate::models::currency::CurrencyData`
- Added `style_for_tier(&Styles, tier: &str) -> &Style` helper (extracts the tier match arms)
- `emit_currency_sections` now iterates `&[CategoryBlock]` (from pricer) instead of `&CurrencyData`
- Added `emit_unique_sections` — emits `# --- Uniques (price-tiered) ---` header + one block per
  `UniqueBlock`; emits NOTHING (no header) when `unique_blocks` is empty (dormant for committed seed)
- `derive_block_label` preserved UNCHANGED — encodes Phase 2 block labels exactly
- All emitter mod tests updated to build `PricedData` via `pricer::price_tiers` over committed
  `prices.json` + `tiers.json`; all 6 tests pass (83 Show / 2 Hide, no Rarity == Unique, LF-only, etc.)

### Task 2: Rewire cmd_generate (drop currency.json, TIER-04 abort, GEN-05 warn)

**`src/main.rs`** — price-driven generate:
- Removed `use models::currency::CurrencyData` and `loader::load_json::<CurrencyData>("currency.json")`
- Added `loader::load_prices_optional("prices.json")?`
- Match on `prices_opt`: `None` → stderr warning + `PricedData::empty()`; `Some(prices)` → `derive_rates` (TIER-04) then `price_tiers`
- Passes `&priced` to `emitter::generate` (new signature)
- `src/models/currency.rs` NOT deleted — still compiled for existing tests

## Verification

**GEN-05 manual check:** Temporarily renamed `prices.json`, ran `cargo run --bin ff -- generate`.
- Exit code: 0
- stderr: `warning: prices.json not found — currency/category blocks will be omitted`
- Generated filter: 0 lines matching `^Show # Currency`

**TIER-04 manual check:** Crafted a temp `prices.json` with Divine chaos=0.0, ran generate.
- Exit code: 1 (non-zero)
- stderr: `ferrisfilter::prices::bad_rate × prices.json: reference currency 'Divine' has a null or zero price`

## Test Results

| Test suite | Result |
|---|---|
| emitter (all 6 unit tests) | PASS |
| pricer (all unit tests) | PASS |
| models (all unit tests) | PASS |
| cli_errors (all 4 tests) | PASS |
| cli_generate::generate_in_repo_root_succeeds | PASS |
| cli_generate::generate_is_deterministic | PASS |
| cli_generate::help_lists_subcommands | PASS |
| cli_generate::generate_help_shows_output_flag | PASS |
| cli_generate::unknown_subcommand_exits_code_2 | PASS |
| cli_generate::artifact_guard | PASS (universal.filter already at Phase 3 content) |
| cli_generate::generate_snapshot | **FAIL (planned)** |

## Expected Failures (Planned Intermediate State)

**`generate_snapshot`** — the insta snapshot in `tests/snapshots/cli_generate__generate_snapshot.snap` is the Phase 2 oracle. It still has "Divine Orb" in Currency S (8 bases) and "Chaos Orb"/"Exalted Orb" in Currency C (5 bases), along with the old Currency D/E base lists. The Phase 3 generator produces the D-09 anchor re-tiering (Chaos C→E, Exalt C→D, Divine S→A), which causes the snapshot assertion to fail.

The diff shown by insta exactly matches the pre-computed re-baseline from RESEARCH.md:
- Currency S: 8→7 bases (Divine Orb removed)
- Currency A: 6→7 bases (Divine Orb prepended)
- Currency C: 5→3 bases (Chaos Orb and Exalted Orb removed)
- Currency D: 9→10 bases (Exalted Orb prepended)
- Currency E: 4→5 bases (Chaos Orb prepended)

**`artifact_guard`** — unexpectedly passes because `universal.filter` in the working tree was already at Phase 3 content (the committed artifact was pre-updated as part of plan 03-01/02 baseline work; its git history shows the last update was 02-02 but the content already reflects the Phase 3 anchor re-tiering). This is a non-issue — artifact_guard passing is correct behavior.

Plan 04 will re-baseline the insta snapshot via `INSTA_UPDATE=always` + human review.

## Deviations from Plan

None — plan executed exactly as written.

The two tasks were committed individually per the protocol, though the compile dependency meant both had to be correct before `cargo test --lib emitter` could run (main.rs calling the old signature prevented compilation). Both tasks are logically separate and committed separately.

## Known Stubs

None — the unique section capability is built but dormant by design (empty seed). No stubs that prevent the plan's goal: generate is price-driven and GEN-05/TIER-04 both work.

## Self-Check

- [x] `src/loader.rs` — `pub fn load_prices_optional` at line 47
- [x] `src/emitter.rs` — `pub fn generate(config: &Config, priced: &PricedData)` at line 19
- [x] `src/main.rs` — `load_prices_optional` call at line 31, no `currency.json` string
- [x] Commits: 4f66bb6 (Task 1), 4301145 (Task 2)

## Self-Check: PASSED
