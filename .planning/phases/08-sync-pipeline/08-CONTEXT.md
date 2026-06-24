# Phase 8: Sync Pipeline - Context

**Gathered:** 2026-06-23
**Status:** Ready for planning

<domain>
## Phase Boundary

`sync` correctly fetches all three new economy categories (Verisium, Uncut Gems at per-level
BaseType, Lineage Support Gems) and writes them to `prices.json` in a form `generate` can emit
as valid, in-game-matching filter blocks. Scope is the **sync → prices.json → generate emit**
data path and `mapping.json` schema. Gem *background* styling is Phase 9; the `corrupted`
condition and exceptional-gear rules are Phase 10.
</domain>

<decisions>
## Implementation Decisions

### Mapping schema & gem dispatch
- Replace the singular `gemFilter` with a `gemFilters` **array** (bump `mapping.json` `version` 1 → 2). Two entries:
  - **A — Uncut Gems:** type `UncutGems`. One block grouping all three subtypes via a multi-class line: `Class == "Uncut Skill Gems" "Uncut Spirit Gems" "Uncut Support Gems"`. The per-subtype Class is derived from the base-type prefix at build time (`Uncut Skill Gem…` → `Uncut Skill Gems`, etc.). Support gems show alongside Skill + Spirit.
  - **B — Lineage Support Gems:** type `LineageSupportGems`, **separate** category. `Class == "Support Gems"` (author-confirmed — the v0.3 blocking spike is RESOLVED, no poe2db lookup needed). Each gem's name is its `BaseType` (`baseTypeIsName`). Per-category background is deferred to Phase 9.
- **Uncut gem BaseType keeps the level:** `Uncut Skill/Spirit/Support Gem (Level N)`, one priced entry per level. STOP stripping the `(Level N)` suffix and STOP the highest-value-per-base dedup (`strip_gem_level` + `gem_best` are the bug).
- **Remove the `minLevel` floor entirely** — every level is priced; tier cutoffs gate relevance (ECON-03).
- **Verisium:** regular `overviewTypes` entry, `Class "Stackable Currency"`. All items returned by the Verisium API are Stackable Currency (incl. "Exceptional Verisium") — no special handling, regular tiering, no background.
- **Guard test:** extend `overview_types_use_correct_plural_spellings` to assert `Verisium` and `LineageSupportGems` are present/correctly spelled (silent-empty-fetch pitfall).
- **Re-baseline cadence:** re-baseline whatever fixtures this phase's output changes (`seed-prices.json`, `seed-universal.filter`, insta snapshot, `generate_block_counts`) so the test suite stays green at this phase's verification.

### Claude's Discretion
- Exact `Mapping`/`GemFilter` Rust struct field shapes for the `gemFilters` array (and where the per-category background field lives, ready for Phase 9 to consume).
- Mechanism for deriving the per-subtype uncut Class (prefix match vs lookup table).
- Structure of the new/updated unit + guard tests.
</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/sync/prices.rs::build_prices` — the join/convert pipeline. The gem branch (`is_gem_type`) currently: `parse_gem_level` → filter `< min_level` → `strip_gem_level` → `gem_best` dedup by base keeping highest `div`. This whole branch is rewritten: keep full name as `base_type`, one `PriceItem` per level, derive Class from prefix, no minLevel, no dedup.
- `convert_primary` — reuse unchanged for gem/Verisium price conversion.
- The id→name join (`item_index` from top-level `items` + `core.items`) — reuse unchanged.
- `src/sync/mapping.rs` — `Mapping` struct + `GemFilter` (singular, with `min_level`). Generalize to `gem_filters: Vec<GemFilter>`; `GemFilter` gains a per-category background field (Option) and supports multi-class output. `run_sync_prices` loops `overview_types` then the single `gem_filter` — change to loop `gem_filters`.

### Established Patterns
- poe.ninja exchange overview schema is shared across all three new types (`core` + `lines[].id` + top-level `items[]` dictionary). No new model needed (`ExchangeOverview`).
- Warnings (not errors) on empty lines/rates/unmapped types — preserve.
- Atomic write via `write_prices_atomic` (refuses empty) — unchanged.

### Integration Points
- `mapping.json` (repo root) — schema change consumed by `mapping.rs` + `prices.rs`.
- `prices.json` output feeds `pricer.rs` → `emitter.rs` (Phase 9 adds the background; Phase 8 just gets correct data + class flowing).
- Guard tests in `src/sync/mapping.rs`; fixture tests in `tests/` (`artifact_guard`, `generate_snapshot`, `generate_block_counts`) + `tests/fixtures/seed-prices.json` / `seed-universal.filter`.
</code_context>

<specifics>
## Specific Ideas

- Author reference filters confirm the exact uncut form (`reference-old-filters/chronomancer_farm.filter`): per-subtype `Class == "Uncut Skill Gems"` + `BaseType == "Uncut Skill Gem (Level 20)"`.
- Endpoints (league "Runes of Aldur"), all exchange overview:
  - `…/exchange/current/overview?type=UncutGems`
  - `…/exchange/current/overview?type=LineageSupportGems`
  - `…/exchange/current/overview?type=Verisium`
- Phase 9 will need the per-category background: Uncut `80 70 55`, Lineage `10 150 150` — leave the `gemFilters` background field in place (Phase 8 may store it; Phase 9 emits it).
</specifics>

<deferred>
## Deferred Ideas

- Gem per-category background emission → Phase 9.
- `corrupted` `when` condition + exceptional-gear rules + `exceptional` style → Phase 10.
- "Exceptional Verisium" special treatment → none planned (standard tiering as Stackable Currency).
</deferred>
