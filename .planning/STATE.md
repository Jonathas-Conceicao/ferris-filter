---
gsd_state_version: 1.0
milestone: v0.3
milestone_name: Economy Items & Exceptional Gear
status: verifying
stopped_at: Phase 08 Plan 02 complete (sync gem dispatch rewrite, multi-class emit, fixtures re-baselined)
last_updated: "2026-06-24T01:01:01.104Z"
last_activity: 2026-06-24
progress:
  total_phases: 4
  completed_phases: 1
  total_plans: 2
  completed_plans: 2
  percent: 100
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-23)

**Core value:** `generate` turns the config into a correct, deterministic PoE2 loot filter.
**Current focus:** Phase --phase — 8

## Current Position

Phase: 08-sync-pipeline — EXECUTING
Plan: 2 of 2 (08-01 complete; 08-02 next)
Status: Phase complete — ready for verification
Last activity: 2026-06-24

Progress: [██████████] 100%

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
| Phase 08-sync-pipeline P01 | 5 | 3 tasks | 4 files |
| Phase 08-sync-pipeline P02 | 12 | 3 tasks | 8 files |

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
- CRLF normalization: s.replace("\r\n", "\n") on both sides of every string comparison in tests — working-tree *.filter is CRLF on Windows, generator emits LF-only
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
- v0.3 AUTHORITATIVE: uncut gem each level is a distinct PoE2 base type; `strip_gem_level`+`gem_best` dedup is the bug; emit `Uncut Skill Gem (Level N)` with per-subtype plural Class
- v0.3 poe.ninja spellings live-verified 2026-06-23: `type=UncutGems`, `type=LineageSupportGems`, `type=Verisium`
- v0.3 background-override pattern: mirror emit_unique_sections — clone tier Style, null its background, call emit_style_actions, then writeln! the override SetBackgroundColor
- v0.3 Lineage gem Class is a blocking spike (poe2db says "Support Gems"; NeverSink dual-class — confirm before committing mapping.json)
- v0.3 fixture re-baseline: do ONE consolidated pass at Phase 10 (last phase that changes emitter output)
- 08-01: GemFilter.classes is Vec<String> to model both multi-class UncutGems and single-class LineageSupportGems uniformly (Claude's Discretion per CONTEXT)
- 08-01: prices.rs compat shim uses gem_filters.first() + hardcoded min_level=18; Plan 02 rewrites gem branch fully
- 08-02: derive_uncut_class uses prefix-match for Skill/Spirit/Support gem class derivation
- 08-02: pre-quoted multi-class string UNCUT_GEM_MULTICLASS merged at pricer boundary (not emitter)
- 08-02: category_background() derives background_override from category name for Phase 9 carry

### Pending Todos

- ~~Phase 8 first task (BLOCKING SPIKE): confirm LineageSupportGems filter Class via poe2db lookup.~~ RESOLVED in CONTEXT: Class == "Support Gems" confirmed.
- Phase 5 todo carried forward: capture current in-game `universal.filter` from game folder into `tests/fixtures/parity-ingame.filter` as parity oracle (MIG-03 prerequisite — may be moot after v0.2 re-baseline, but worth confirming).
- Phase 08-02: rewrite prices.rs gem dispatch branch (loop gem_filters, per-level BaseType, no min_level, strip_gem_level/gem_best removal).

### Blockers/Concerns

- ~~LineageSupportGems filter Class is MEDIUM confidence~~ — RESOLVED: Class == "Support Gems" locked in CONTEXT (D-CTX-1); mapping.json committed in 08-01.

## Deferred Items

Items acknowledged and deferred at milestone close on 2026-05-28:

| Category | Item | Status |
|----------|------|--------|
| uat | Live softcore no-flag `sync` happy path | Blocked on PoE2 league cycle ("Fate of the Vaal" ended); tool fails safe. Live integration confirmed via `sync --standard` (318 items). Re-confirm when a new league launches. |
| verification | Live network sign-off for `sync` | Signed off working 2026-05-28; full live re-check pending an active softcore league. |
| sync | Lenient-parse guard for Standard unique endpoints (`UniqueSanctumRelics` missing `core`) | Non-blocking; wrote 0 uniques on Standard. |
| sync | Optional no-flag `sync` → `--standard` fallback/hint when no challenge league active | Enhancement idea. |

## Session Continuity

Last session: 2026-06-24T01:01:01.091Z
Stopped at: Phase 08 Plan 02 complete (sync gem dispatch rewrite, multi-class emit, fixtures re-baselined)
Resume file: None

**Next step:** Execute Phase 08 Plan 02 (08-02-PLAN.md — prices.rs gem dispatch rewrite)

**Planned Phase:** 8 (Sync Pipeline) — 2 plans — 2026-06-24T00:36:32.476Z
