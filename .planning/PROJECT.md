# FerrisFilter

## What This Is

FerrisFilter is a Rust CLI tool that compiles a human-editable configuration into a working Path of Exile 2 loot filter. The author edits a compact policy file describing what they want shown, highlighted, or hidden; FerrisFilter deterministically generates the `.filter` the game consumes. It also fetches live item prices from poe.ninja so highlight tiers can be driven by real economy values rather than hand-curated lists.

## Core Value

`generate` turns the config into a correct, deterministic PoE2 loot filter — the same translation currently done by hand, done by the tool instead. If nothing else works, this must.

## Current Milestone: v0.3 Economy Items & Exceptional Gear

**Goal:** Track three new/broken economy categories with correct styling, and highlight "exceptional" gear.

**Target features:**
- **Verisium** — new synced category (poe.ninja exchange `type=Verisium`), emitted with the regular tiered currency style.
- **Uncut Gems fix** — emitted `BaseType` includes the level (`Uncut Skill Gem (Level N)`) so the rule actually matches; drop the `minLevel: 18` floor; price-tiered with a fixed background `80 70 55`.
- **Lineage Support Gems** — new synced category (`type=LineageSupportGems`), price-tiered with a fixed background `10 150 150`.
- **Exceptional gear highlight** — gear with extra sockets OR extra quality and NOT corrupted (filter mechanism unknown → resolved by research).

**Key context:** League is now "Runes of Aldur" (Fate of the Vaal ended). Gem styling = tier color (border+text, by value) + a per-category background override — the same mechanism as priced uniques' bronze background, **not** a new named style (so the config-driven-styles backlog item is not pulled in). Tablets remain in the backlog (out of scope this milestone).

## Current State

**Shipped:** v0.2 Declarative Rule Model (2026-06-12) — the generator's hardcoded gear/flask band
logic is replaced by a general, declarative `rules` schema in `config.json`; the full in-game
filter is now reproducible from config. `generate` + `sync` + `install` all live. ~154 tests green.

**Next milestone:** v0.3 Economy Items & Exceptional Gear — started 2026-06-23 (defining requirements).
Adds Verisium + Lineage Support Gems sync, fixes Uncut Gems base-type matching, and highlights
exceptional (extra-socket/quality, uncorrupted) gear.

## Requirements

### Validated

- [x] **CLI shell** — `ferrisfilter` crate builds; `generate` and `sync` subcommands are visible with `--help`; `generate --output` documented; unknown commands exit non-zero. *(Validated in Phase 1: Foundation)*
- [x] **Static filter generation** — `ferrisfilter generate` deterministically compiles `config.json` + `currency.json` into `universal.filter`, reproducing the established block structure (model-path tags, AreaLevel bands, flask/charm ladder, recycle, currency tiers), locked by an insta snapshot + artifact-guard test; uniques/gems/jewels/etc. fall through. *(Validated in Phase 2: Static Generate)*
- [x] **Price-driven tiering** — `generate` derives currency/category (and, dormant, unique) tiers from `prices.json` via cross-rate normalization + `tiers.json` cutoffs, aborts on null/zero reference rates (TIER-04), and degrades gracefully without prices.json (GEN-05); `currency.json` is dropped as a generate input (reference/seed only). Re-baseline limited to the 3 reference orbs, guardrail-tested. *(Validated in Phase 3: Pricing Layer)*
- [x] **Input models** — `config.json` (renamed from `filter-model.json`), `tiers.json`, and `prices.json` are modeled in serde, round-trip, and validate with actionable field-path + expected-type errors; `tiers.json` completeness/ordering checked on load. *(Validated in Phase 1: Foundation)*
- [x] **Currency data extraction** — curated currency base-lists pulled out of `universal.filter` into a tracked `currency.json` and modeled. *(Validated in Phase 1: Foundation)*
- [x] **Live price sync** — `ferrisfilter sync` fetches PoE2 economy data from poe.ninja (currency, uniques, tracked drops; uncut gems ≥ 18), resolves the league per `--standard`/`--hc` (none = softcore trade), records league + timestamp, and writes `prices.json` atomically (failure-safe; never corrupts an existing file). *(Validated in Phase 4: Sync — v0.1)*
- [x] **Install** — `ferrisfilter install` copies a generated filter into the game config folder (`--dest`/`--input` overrides), replacing the old `install.bat`. *(Shipped in v0.1)*
- [x] **Early-league fallback** — committed `prices-fallback.json` + `tiers-fallback.json` (exalt-denominated) drive `generate` when `prices.json` is absent/empty (with a warning) or `--early-game` is passed; cross-rates are derived from the anchor orbs. *(Shipped in v0.1)*
- [x] **Declarative rule engine** — `config.json` `rules` array (ordered, first-match-wins) compiled by `emit_rules`; the hardcoded gear/armour/otherGear/flask special-casing is gone. *(Validated in Phase 6 — v0.2)*
- [x] **Condition vocabulary** — rules combine class list, compact rarity comparator, areaLevel min/max, base-defence/Quality/Sockets, BaseType list, and UnidentifiedItemTier; serde-validated with field-path errors. *(Validated in Phase 5 — v0.2)*
- [x] **The 7 in-game edits expressible as config** — weapon/armour caps, flask/charm Magic+Rare + rarity-bounded Hide, Normal charms ≥78, jewellery hide, unidentified-tier highlight (new `unidentified` style). *(Validated in Phase 7 — v0.2)*
- [x] **Config migration + semantic parity** — `config.json` migrated to rules; `generate --early-game` reproduces the in-game filter with zero non-reorder policy differences; `seed-universal.filter` re-baselined; price-driven sections untouched. *(Validated in Phase 6/7 — v0.2)*

### Active

<!-- v0.3 Economy Items & Exceptional Gear — REQ-IDs assigned in REQUIREMENTS.md. -->

- [x] **Verisium sync + tiering** — `sync` fetches Verisium from poe.ninja and `generate` tiers it as regular stackable currency. *(Validated in Phase 8: Sync Pipeline — ECON-01)*
- [x] **Uncut Gems base-type fix** — emitted base type carries the level (`Uncut Skill Gem (Level N)`), per-subtype plural Class; `minLevel` floor removed. *(Validated in Phase 8 — ECON-03)*
- [x] **Lineage Support Gems sync + tiering** — new synced category (`Class "Support Gems"`), price-tiered. *(Validated in Phase 8 — ECON-02)*
- [ ] **Gem per-category background styling** — uncut/lineage gems show tier color (border+text) over a fixed per-category background. *(Phase 9)*
- [ ] **Exceptional gear highlight** — rules surface uncorrupted gear with extra sockets or extra quality, dedicated `exceptional` style. *(Phase 10)*

### Deferred (next-milestone candidates)

- [ ] **BaseType-matched, economy-tiered priced category** (e.g. Tablets): support a priced category that emits `BaseType ==` with **no `Class` line** (tablets have no usable PoE2 filter class). Needs a `Tablets` type in `mapping.json` (spike: confirm poe.ninja serves tablet prices), pricer/emitter support for class-less category blocks, and corrected base names (`Ritual Tablet`/`Abyss Tablet`/…, no "Precursor"). Tablets currently fall through to game default after the v0.2 removal of the invalid `Class "Tablet"` rows.
- [ ] Generate QoL: `--dry-run`, `--diff`, `-v/--verbose` rule logging, stale-`prices.json` warning (GENX-01..04)
- [ ] Sync extensions: multiple build profiles (`--config builds/<name>.json`); alternate price source fallback, e.g. poe2scout (SYNX-01..02)
- [ ] Lenient-parse guard for Standard-league unique endpoints (different/empty shape); optional no-flag `sync` → `--standard` hint when no challenge league is active

### Out of Scope

- Hand-maintaining currency/tier base-lists — replaced by `sync` + price-driven tiering
- Filter rules for gems, jewels, quest items, waystones, trials — these fall through to game default (established design philosophy). *(Jewellery hide and unidentified-tier highlight move IN-scope for v0.2 — the in-game filter already curates them.)*
- Blanket unique styling — only price-tracked uniques get rules; the rest fall through
- GUI / web interface — CLI only for now
- ~~Auto-installing into the game folder~~ — **shipped in v0.1** as the `install` subcommand (copy-only; `install.bat` removed)

## Context

- **Prior art lives in this repo.** `filter-model.json` (the policy model) and `universal.filter` (the hand-generated output) already exist and encode the exact logic `generate` must reproduce. `CLAUDE.md` documents the filter grammar, the Dracula style-tier system, and the design philosophy (hide unwanted, style highlights, fall through the rest). These are the executable spec for the generator.
- **The model was built and refined across a long session:** `styles` (S–F + `unchanged`/`hidden`), `build.weapons` (Staves/Wands/Foci) and `build.armour` (Body Armours/Helmets/Gloves/Boots) with defence-type taper, `areaLevelBands` (1-5 / 6-25 / 26-45 / 46-65 / 66+), and `alwaysShow` (currency tiers, recycle). Weapons are build-only across all bands; rares of any class show ≤45 via the otherGear rule.
- **Currency tiers are currently curated base-lists inside the filter.** `sync` + `generate` will replace that curation with price-driven tiering from poe.ninja.
- **Platform:** Windows 11, PowerShell primary shell. Author prefers Rust.
- **poe.ninja** is the price source; its PoE2 API surface was resolved via the Phase 4 spike (exchange + stash/item overview endpoints, plural `type` values, league index-state resolution) and is now consumed by `sync`.
- **Shipped state (v0.1, 2026-05-28):** ~4,970 LOC Rust across 4 phases / 14 plans / 127 commits. `generate` + `sync` + `install` all live; full test suite green (~113 tests). Live softcore no-flag `sync` happy path awaits a new active league (fails safe meanwhile).

## Constraints

- **Tech stack**: Rust — author preference; crate named `FerrisFilter`.
- **Output format**: Must emit valid PoE2 filter grammar (same grammar as PoE1, different valid classes/base types — verify against poe2db.tw / live game).
- **Determinism**: `generate` must be deterministic — same inputs always produce the same `universal.filter`.
- **Dependency**: `sync` depends on poe.ninja's PoE2 economy API (external, may change, per-league).
- **Reference unit**: All tiering math normalizes to Chaos Orb value via cross-rates from the three reference currencies.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust CLI crate named `FerrisFilter` | Author preference; deterministic compiled tool | ✓ Good (v0.1) |
| Two subcommands: `generate` + `sync` | Separate filter compilation from price fetching | ✓ Good (v0.1) |
| `sync` fetches raw prices only; `generate` does all conversion + tiering | Keep network/IO and pure logic separate; deterministic generate | ✓ Good (v0.1) |
| Separate files: `config.json` (human) + `tiers.json` (cutoffs) + `prices.json` (synced) | Keep hand-edited policy apart from machine-generated data | ✓ Good (v0.1) |
| Chaos as the reference unit; cross-rates from first 3 `prices.json` entries | Single common unit for threshold comparisons | ✓ Good (v0.1) |
| poe.ninja as price source | Author choice | ✓ Good (v0.1) |
| Price-tier valuable uniques; untracked uniques fall through | Only relevant-value uniques deserve rules | ✓ Good (v0.1) |
| Both `generate` and `sync` in v1 | Author wants the full loop usable from the start | ✓ Good (v0.1) |
| Milestone git workflow (branch per milestone) | Clean, reviewable history per milestone | ✓ Good (v0.1) |
| Declarative `rules` array replaces hardcoded emitter bands | One general model; any class/level/rarity/condition policy expressible in config | ✓ Good (v0.2) |
| Compact rarity string (`">=Magic"`) over an object | Author preference; terser config authoring | ✓ Good (v0.2) |
| Semantic (order/comment-insensitive) parity, not byte-for-byte | Block reordering is irrelevant in PoE2; lets the rule model reshape output freely | ✓ Good (v0.2) |
| Price-driven currency/unique sections stay pricer-driven (not rules) | Keep the v0.1 economy pipeline untouched; rules cover policy only | ✓ Good (v0.2) |
| Tablets have no usable filter `Class` → removed broken rows | `Class "Tablet"` failed to load in-game; BaseType-only category deferred | ⚠️ Revisit (v0.2 — deferred re-add) |

## Git Workflow

`git.branching_strategy = "milestone"` (config.json). Per-milestone flow:

1. **Start of milestone** → work happens on a dedicated branch `milestone/{milestone}-{slug}` (GSD creates it at the milestone's first `execute-phase`).
2. **On `complete-milestone`**, in this exact order:
   1. **Push the milestone branch to remote** (preserves full development history).
   2. **Squash-merge** the branch into `main` with a single clean commit message summarizing the milestone.
   3. **Tag** the resulting merge commit on `main` (annotated `vX.Y`), then push the tag.

GSD's `complete-milestone` natively does the squash-merge and tag; the branch push and tag-on-main ordering are enforced as this project convention.

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-06-24 — Phase 8 (Sync Pipeline) complete: Verisium + Lineage Support Gems synced, Uncut Gems base-type fix landed (ECON-01/02/03 validated)*
