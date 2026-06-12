---
gsd_state_version: 1.0
milestone: v0.2
milestone_name: Declarative Rule Model
status: milestone_complete
stopped_at: "Completed 07-03-PLAN.md: CLAUDE.md declarative rule model realignment (DOC-01)"
last_updated: "2026-06-05T11:56:50.222Z"
last_activity: 2026-06-05
progress:
  total_phases: 3
  completed_phases: 3
  total_plans: 7
  completed_plans: 6
  percent: 100
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-05)

**Core value:** `generate` turns the config into a correct, deterministic PoE2 loot filter.
**Current focus:** Phase 7 — New Capabilities + Docs

## Current Position

Phase: 7
Plan: Not started
Status: Milestone complete
Last activity: 2026-06-12

Progress: [█████████░] 86%

## Performance Metrics

**Velocity:**

- Total plans completed: 17
- Average duration: —
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 3 | - | - |
| 02 | 3 | - | - |
| 03 | 4 | - | - |
| 5 | 2 | - | - |
| 6 | 2 | - | - |
| 7 | 3 | - | - |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

*Updated after each plan completion*
| Phase 02-static-generate P01 | 15 | 3 tasks | 3 files |
| Phase 02-static-generate P02 | 45 | 2 tasks | 5 files |
| Phase 02-static-generate P03 | 20 | 2 tasks | 3 files |
| Phase 03-pricing-layer P01 | 200 | 3 tasks | 4 files |
| Phase 03-pricing-layer P02 | 155 | 2 tasks | 2 files |
| Phase 03-pricing-layer P03 | 326 | 2 tasks | 3 files |
| Phase 03 P04 | 30 | 2 tasks | 4 files |
| Phase 05-rule-schema P01 | 6 | 3 tasks | 1 files |
| Phase 05 P02 | 12 | 3 tasks | 2 files |
| Phase 06-emitter-rewrite-migration-parity P01 | 516 | 3 tasks | 4 files |
| Phase 07-new-capabilities-docs P01 | 4 | 2 tasks | 3 files |
| Phase 07-new-capabilities-docs P03 | 3 | 2 tasks | 1 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- v0.2 finding: the emitter is FULLY HARDCODED — `emit_gear_bands`/`emit_flask_charm_bands` write all band ranges + rarities as literal strings; `config.area_level_bands` and the `RarityRule` enum are parsed but NEVER read (zero consumers outside config.rs). v0.2 deletes this special-casing in favor of a general declarative rule model.
- v0.2 acceptance bar: `generate` on the migrated config must be SEMANTICALLY EQUIVALENT to the filter currently installed in the game folder (block reordering OK; other behavioral changes surfaced for author review). The in-game `universal.filter` (game folder, fully functional) is the parity oracle and should be captured into the repo as a fixture.
- v0.2 confirmed (author tested live, in-game filter fully functional): `UnidentifiedItemTier` and base-defence filters (`BaseArmour`/`BaseEvasion`/`BaseEnergyShield`) work in PoE2 — update CLAUDE.md's "unverified" caveats.
- v0.2 scope: config-flexibility only. CLI surface (`generate`/`sync`/`install`) unchanged; price-driven currency/category + unique sections untouched. Generate QoL + sync extensions deferred.
- v0.2 build order: (1) schema/rule-model design (Phase 5), (2) emitter rewrite + migrate config.json + re-baseline fixture (Phase 6), (3) new capabilities + docs (Phase 7). The schema is the linchpin.
- Planning: poe.ninja PoE2 API surface is UNRESOLVED — spike is the first task of Phase 4 (sync), not a separate phase (granularity = coarse).
- Planning: insta snapshot of generated universal.filter is the Phase 2 acceptance criterion; byte-identical reproduction is non-negotiable.
- D-11: Styles named-fields struct replaces HashMap<String, Style> for deterministic tier access
- D-10: RarityRule closed enum replaces freeform rarity strings; serde alias covers otherGear bare 'Rare' value
- D-03 canonical action order: PlayEffect→MinimapIcon→SetTextColor→SetBackgroundColor→SetBorderColor→SetFontSize→Sound→DisableDropSound — applied to all 85 blocks via single emit_style_actions function
- Re-baseline D-01 complete: universal.filter regenerated as golden master; only header source-indicator + recycle action reorder changed (D-02 behavioral equivalence confirmed)
- CRLF normalization: s.replace(\"\r\n\", \"\n\") on both sides of every string comparison in tests — working-tree *.filter is CRLF on Windows, generator emits LF-only
- Approach A for snapshot tests: drive binary via assert_cmd --output to temp path; consistent with existing test conventions
- D-03 extended: PriceItem has required class+category+baseType (String, not Option)
- D-09 seed: non-reference bases at tier lower-bound; Chaos=1c, Exalt=4.5c, Divine=220c committed in prices.json
- seed_prices duplicates minimal currency structs locally (no lib target in crate)
- pricer.rs: HashMap for O(1) accumulation + Vec+TIER_ORDER const for deterministic emission (no HashMap iteration at output time)
- PriceRateError lives in pricer.rs (only caller); PricedData::empty() for GEN-05 no-prices.json path
- emitter::generate now takes &PricedData instead of &CurrencyData
- load_prices_optional: Path::exists() check first; absent=Ok(None), malformed=Err
- currency.json removed from generate path; src/models/currency.rs kept for tests
- RarityComparator uses custom Deserialize for compact string format (>=Magic, <=Rare, bare==Eq) per 05-CONTEXT spec
- New rule-schema types (Rule/When/RarityComparator/NumericCondition/AreaLevelRange/UnidentifiedTier) are additive; Config top-level struct unchanged until Plan 02
- Config.rules (Option<Vec<Rule>>) and class_groups (Option<HashMap<String,Vec<String>>>) are additive optional fields with serde(default); existing config.json parses with both as None until Phase 6 populates them
- Dead areaLevelBands scaffolding (RarityRule/AreaLevelBand/BandShow/ArmourShow + Config field + config.json block) removed; emitter confirmed non-consumer (comment strings only); Phase 6 emitter rewrite starts clean
- classGroups approach (a): inline full class lists in each rule when.class; classGroups is documentation/Phase-7 aliases only, not consumed by emit_rules
- base-defence min:1 renders as '>0' (not '>=1') in emit_numeric to match oracle filter syntax
- CAP-06 weapon+armour class list omits Quarterstaves (matches foo.filter oracle exactly — author's in-game list)
- unidentified style uses no text/border/background/font to preserve native rarity color
- generate_block_counts test updated to 86 Show / 3 Hide after adding 3 CAP Show blocks + 1 Hide block
- DOC-01: CLAUDE.md realigned to v0.2 declarative rule model; config.json named source of truth; UnidentifiedItemTier + base-defence filters confirmed working in-game

### Pending Todos

- Phase 5 first: capture the current in-game `universal.filter` from `C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2` into `tests/fixtures/parity-ingame.filter` as the parity oracle (MIG-03 prerequisite).

### Blockers/Concerns

- None. (The poe.ninja PoE2 API spike — the project's only external unknown — was resolved in Phase 4. UnidentifiedItemTier + base-defence filters confirmed working in-game.)

## Deferred Items

Items acknowledged and deferred at milestone close on 2026-05-28:

| Category | Item | Status |
|----------|------|--------|
| uat | Live softcore no-flag `sync` happy path | Blocked on PoE2 league cycle ("Fate of the Vaal" ended); tool fails safe. Live integration confirmed via `sync --standard` (318 items). Re-confirm when a new league launches. |
| verification | Live network sign-off for `sync` | Signed off working 2026-05-28; full live re-check pending an active softcore league. |
| sync | Lenient-parse guard for Standard unique endpoints (`UniqueSanctumRelics` missing `core`) | Non-blocking; wrote 0 uniques on Standard. |
| sync | Optional no-flag `sync` → `--standard` fallback/hint when no challenge league active | Enhancement idea. |

## Session Continuity

Last session: 2026-06-05T11:56:50.216Z
Stopped at: Completed 07-03-PLAN.md: CLAUDE.md declarative rule model realignment (DOC-01)
Resume file: None

**Next step:** `/gsd-plan-phase 5`

**Planned Phase:** 7 (New Capabilities + Docs) — 3 plans — 2026-06-05T11:47:27.370Z
