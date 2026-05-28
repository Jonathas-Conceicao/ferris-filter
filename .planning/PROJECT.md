# FerrisFilter

## What This Is

FerrisFilter is a Rust CLI tool that compiles a human-editable configuration into a working Path of Exile 2 loot filter. The author edits a compact policy file describing what they want shown, highlighted, or hidden; FerrisFilter deterministically generates the `.filter` the game consumes. It also fetches live item prices from poe.ninja so highlight tiers can be driven by real economy values rather than hand-curated lists.

## Core Value

`generate` turns the config into a correct, deterministic PoE2 loot filter — the same translation currently done by hand, done by the tool instead. If nothing else works, this must.

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

### Active

<!-- Candidates for the next milestone — refine via /gsd-new-milestone. -->

- [ ] Generate QoL (v2): `--dry-run`, `--diff`, `-v/--verbose` rule logging, stale-`prices.json` warning (GENX-01..04)
- [ ] Sync extensions (v2): multiple build profiles (`--config builds/<name>.json`); alternate price source fallback, e.g. poe2scout (SYNX-01..02)
- [ ] Lenient-parse guard for Standard-league unique endpoints (different/empty shape); optional no-flag `sync` → `--standard` hint when no challenge league is active

### Out of Scope

- Hand-maintaining currency/tier base-lists — replaced by `sync` + price-driven tiering
- Filter rules for gems, jewels, jewellery, quest items, waystones, trials, unidentified items — these fall through to game default (established design philosophy)
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
*Last updated: 2026-05-28 after v0.1 (Initial Release) milestone*
