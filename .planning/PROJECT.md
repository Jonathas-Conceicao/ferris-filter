# FerrisFilter

## What This Is

FerrisFilter is a Rust CLI tool that compiles a human-editable configuration into a working Path of Exile 2 loot filter. The author edits a compact policy file describing what they want shown, highlighted, or hidden; FerrisFilter deterministically generates the `.filter` the game consumes. It also fetches live item prices from poe.ninja so highlight tiers can be driven by real economy values rather than hand-curated lists.

## Core Value

`generate` turns the config into a correct, deterministic PoE2 loot filter — the same translation currently done by hand, done by the tool instead. If nothing else works, this must.

## Requirements

### Validated

- [x] **CLI shell** — `ferrisfilter` crate builds; `generate` and `sync` subcommands are visible with `--help`; `generate --output` documented; unknown commands exit non-zero. *(Validated in Phase 1: Foundation)*
- [x] **Static filter generation** — `ferrisfilter generate` deterministically compiles `config.json` + `currency.json` into `universal.filter`, reproducing the established block structure (model-path tags, AreaLevel bands, flask/charm ladder, recycle, currency tiers), locked by an insta snapshot + artifact-guard test; uniques/gems/jewels/etc. fall through. *(Validated in Phase 2: Static Generate)*
- [x] **Input models** — `config.json` (renamed from `filter-model.json`), `tiers.json`, and `prices.json` are modeled in serde, round-trip, and validate with actionable field-path + expected-type errors; `tiers.json` completeness/ordering checked on load. *(Validated in Phase 1: Foundation)*
- [x] **Currency data extraction** — curated currency base-lists pulled out of `universal.filter` into a tracked `currency.json` and modeled. *(Validated in Phase 1: Foundation)*

### Active

<!-- Hypotheses until shipped and validated. -->

- [ ] CLI crate `FerrisFilter` with two subcommands: `generate` and `sync`
- [ ] `config.json` (renamed from `filter-model.json`) is the human-editable policy: `styles`, `build`, class lists, `areaLevelBands`, `alwaysShow`
- [ ] `generate` reads `config.json` + `tiers.json` + `prices.json` and emits `universal.filter` deterministically
- [ ] `generate` reproduces the existing hand-built filter logic: style tiers, build-weapon/armour bands, area-level progression, flask/charm ladder, recycle, currency tiers, fallthrough
- [ ] `generate` derives currency cross-rates from the first three `prices.json` entries (Exalt, Chaos, Divine), converts every item to chaos, and assigns tiers per `tiers.json`
- [ ] `generate` tier-highlights valuable uniques (those present in `prices.json`); untracked uniques fall through
- [ ] `tiers.json` holds configurable price cutoffs (chaos as reference unit): `F <0.1c · E 0.1–<0.5c · D 0.5–<2c · C 2–<5c · B 5c–<1div · A 1div–<5div · S ≥5div`
- [ ] `sync` fetches PoE2 economy data from poe.ninja and writes `prices.json`
- [ ] `prices.json` is a list of `item → {exalt, chaos, div}` (any currency may be null when untraded/missing); first three entries are always Exalt, Chaos, Divine
- [ ] `sync` includes currency, uniques, and other poe.ninja-tracked drops; uncut gems only at level ≥ 18
- [ ] League selection via flags: none = current trade league; `--hc` = current hardcore; `--standard` = standard; `--standard --hc` = standard hardcore (both default off)

### Out of Scope

- Hand-maintaining currency/tier base-lists — replaced by `sync` + price-driven tiering
- Filter rules for gems, jewels, jewellery, quest items, waystones, trials, unidentified items — these fall through to game default (established design philosophy)
- Blanket unique styling — only price-tracked uniques get rules; the rest fall through
- GUI / web interface — CLI only for now
- Auto-installing into the game folder from the tool — `install.bat` already covers copying; revisit later

## Context

- **Prior art lives in this repo.** `filter-model.json` (the policy model) and `universal.filter` (the hand-generated output) already exist and encode the exact logic `generate` must reproduce. `CLAUDE.md` documents the filter grammar, the Dracula style-tier system, and the design philosophy (hide unwanted, style highlights, fall through the rest). These are the executable spec for the generator.
- **The model was built and refined across a long session:** `styles` (S–F + `unchanged`/`hidden`), `build.weapons` (Staves/Wands/Foci) and `build.armour` (Body Armours/Helmets/Gloves/Boots) with defence-type taper, `areaLevelBands` (1-5 / 6-25 / 26-45 / 46-65 / 66+), and `alwaysShow` (currency tiers, recycle). Weapons are build-only across all bands; rares of any class show ≤45 via the otherGear rule.
- **Currency tiers are currently curated base-lists inside the filter.** `sync` + `generate` will replace that curation with price-driven tiering from poe.ninja.
- **Platform:** Windows 11, PowerShell primary shell. Author prefers Rust.
- **poe.ninja** is the chosen price source; its PoE2 API surface (endpoints, categories, auth, rate limits) needs research before `sync` is built.

## Constraints

- **Tech stack**: Rust — author preference; crate named `FerrisFilter`.
- **Output format**: Must emit valid PoE2 filter grammar (same grammar as PoE1, different valid classes/base types — verify against poe2db.tw / live game).
- **Determinism**: `generate` must be deterministic — same inputs always produce the same `universal.filter`.
- **Dependency**: `sync` depends on poe.ninja's PoE2 economy API (external, may change, per-league).
- **Reference unit**: All tiering math normalizes to Chaos Orb value via cross-rates from the three reference currencies.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust CLI crate named `FerrisFilter` | Author preference; deterministic compiled tool | — Pending |
| Two subcommands: `generate` + `sync` | Separate filter compilation from price fetching | — Pending |
| `sync` fetches raw prices only; `generate` does all conversion + tiering | Keep network/IO and pure logic separate; deterministic generate | — Pending |
| Separate files: `config.json` (human) + `tiers.json` (cutoffs) + `prices.json` (synced) | Keep hand-edited policy apart from machine-generated data | — Pending |
| Chaos as the reference unit; cross-rates from first 3 `prices.json` entries | Single common unit for threshold comparisons | — Pending |
| poe.ninja as price source | Author choice | — Pending |
| Price-tier valuable uniques; untracked uniques fall through | Only relevant-value uniques deserve rules | — Pending |
| Both `generate` and `sync` in v1 | Author wants the full loop usable from the start | — Pending |
| Milestone git workflow (branch per milestone) | Clean, reviewable history per milestone | — Pending |

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
*Last updated: 2026-05-24 after Phase 2 (Static Generate) completion*
