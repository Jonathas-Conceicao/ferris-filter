# FerrisFilter

## What This Is

FerrisFilter is a Rust CLI tool that compiles a human-editable configuration into a working Path of Exile 2 loot filter. The author edits a compact policy file describing what they want shown, highlighted, or hidden; FerrisFilter deterministically generates the `.filter` the game consumes. It also fetches live item prices from poe.ninja so highlight tiers can be driven by real economy values rather than hand-curated lists.

## Core Value

`generate` turns the config into a correct, deterministic PoE2 loot filter — the same translation currently done by hand, done by the tool instead. If nothing else works, this must.

## Current Milestone: none — v0.3 shipped, v0.4 not yet defined

Run `/gsd-new-milestone` to scope the next milestone.

<details>
<summary>✅ v0.3 Economy Items & Exceptional Gear — SHIPPED 2026-06-27</summary>

**Goal:** Track three new/broken economy categories with correct styling, and highlight "exceptional" gear.

**Delivered features:**
- **Verisium** — new synced category (poe.ninja exchange `type=Verisium`), emitted with the regular tiered currency style.
- **Uncut Gems fix** — emitted `BaseType` includes the level (`Uncut Skill Gem (Level N)`) so the rule actually matches; drop the `minLevel: 18` floor; price-tiered with a fixed background `80 70 55`.
- **Lineage Support Gems** — new synced category (`type=LineageSupportGems`), price-tiered with a fixed background `10 150 150`.
- **Exceptional gear highlight** — gear with extra sockets OR extra quality and NOT corrupted (filter mechanism unknown → resolved by research).

**Key context:** League is "Runes of Aldur" (Fate of the Vaal ended). Gem styling = tier color (border+text, by value) + a per-category background override — the same mechanism as priced uniques' bronze background, **not** a new named style (so the config-driven-styles backlog item was not pulled in). Tablets remain in the backlog (out of scope for v0.3).

**Audit:** `passed` — 9/9 requirements, 3/3 phases, 8/8 integration, 5/5 E2E flows. Archives:
[v0.3-ROADMAP.md](milestones/v0.3-ROADMAP.md) · [v0.3-REQUIREMENTS.md](milestones/v0.3-REQUIREMENTS.md) · [v0.3-MILESTONE-AUDIT.md](milestones/v0.3-MILESTONE-AUDIT.md)

</details>

## Current State

**Shipped:** v0.3 Economy Items & Exceptional Gear (2026-06-27) — `sync` now tracks Verisium and
Lineage Support Gems, Uncut Gems finally match in-game (level kept in `BaseType`, per-subtype plural
`Class`, no `minLevel` floor), priced gem blocks carry a per-category background over the value-tier
text/border, and the rule vocabulary gained `corrupted` — driving three exceptional-gear rules
(sockets >= 3 / >= 2, quality >= 24, all uncorrupted) in the new pink `exceptional` style.
`generate` + `sync` + `install` all live. ~7,534 LOC Rust; 133 lib tests + integration suite green.

**Prior:** v0.2 Declarative Rule Model (2026-06-12) replaced the generator's hardcoded gear/flask
band logic with a declarative `rules` schema, making the full in-game filter reproducible from config.

**Next milestone:** not yet defined — run `/gsd-new-milestone`. Leading candidates are the deferred
items below: Tablets as a class-less priced category (ECONX-01), generate QoL (`--dry-run`,
`--diff`, verbose rule logging), and the config-driven style vocabulary (backlog Phase 999.1).

**Open at close:** nothing blocking. Full suite green on `main` at tag `v0.3` (166 tests, 0
failures).

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
- [x] **Verisium sync + tiering** — `sync` fetches Verisium from poe.ninja and `generate` tiers it as regular stackable currency, no special background. *(Validated in Phase 8 — ECON-01, v0.3)*
- [x] **Uncut Gems base-type fix** — emitted base type carries the level (`Uncut Skill Gem (Level N)`), per-subtype plural `Class`; `minLevel` floor removed; subtypes merge into one multi-class block. *(Validated in Phase 8 — ECON-03, v0.3)*
- [x] **Lineage Support Gems sync + tiering** — new synced category (`Class "Support Gems"`, spike-confirmed), price-tiered. *(Validated in Phase 8 — ECON-02, v0.3)*
- [x] **Gem per-category background styling** — uncut (`80 70 55`) / lineage (`10 150 150`) gems show value-tier text+border over a fixed per-category background, via the priced-unique clone-null-append pattern. *(Validated in Phase 9 — STYLE-01/02, v0.3)*
- [x] **`corrupted` rule condition** — `when.corrupted` emits the bare boolean `Corrupted True`/`Corrupted False`, no operator. *(Validated in Phase 10 — EXC-01, v0.3)*
- [x] **Exceptional gear highlight** — three rules ahead of recycle surface uncorrupted gear with extra sockets (>= 3 2h/body, >= 2 1h) or `Quality >= 24`, in the dedicated pink `exceptional` style. *(Validated in Phase 10 — EXC-02/03/04, v0.3)*

### Active

<!-- Empty: v0.3 shipped. New requirements land here via /gsd-new-milestone. -->

None — awaiting the v0.4 scope.

### Deferred (next-milestone candidates)

- [ ] **BaseType-matched, economy-tiered priced category** (e.g. Tablets): support a priced category that emits `BaseType ==` with **no `Class` line** (tablets have no usable PoE2 filter class). Needs a `Tablets` type in `mapping.json` (spike: confirm poe.ninja serves tablet prices), pricer/emitter support for class-less category blocks, and corrected base names (`Ritual Tablet`/`Abyss Tablet`/…, no "Precursor"). Tablets currently fall through to game default after the v0.2 removal of the invalid `Class "Tablet"` rows.
- [ ] Generate QoL: `--dry-run`, `--diff`, `-v/--verbose` rule logging, stale-`prices.json` warning (GENX-01..04)
- [ ] Sync extensions: multiple build profiles (`--config builds/<name>.json`); alternate price source fallback, e.g. poe2scout (SYNX-01..02)
- [ ] Lenient-parse guard for Standard-league unique endpoints (different/empty shape); optional no-flag `sync` → `--standard` hint when no challenge league is active

### Out of Scope

- Hand-maintaining currency/tier base-lists — replaced by `sync` + price-driven tiering
- Filter rules for jewels, quest items, waystones, trials — these fall through to game default (established design philosophy). *(Jewellery hide and unidentified-tier highlight moved IN-scope in v0.2 — the in-game filter already curates them. **Uncut and Lineage Support gems moved IN-scope in v0.3** as priced, economy-tiered categories; all other gems still fall through.)*
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
- **Shipped state (v0.3, 2026-06-27):** ~7,534 LOC Rust; 133 lib tests plus the CLI/sync integration suites green. `mapping.json` is at schema v2 (`gemFilters` array with per-category `backgroundOverride`, `Verisium` overview type). The generated filter is 77 `Show` / 3 `Hide` blocks. Live league is "Runes of Aldur"; post-close `sync` fixes landed for the Expedition logbook/Sagas mixed-class feed and PoE1-only `UniqueMaps`.

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
| Each uncut gem *level* is a distinct base type; drop `strip_gem_level`/`gem_best` dedup | Level-stripped `BaseType` never matched in-game — that dedup was the bug | ✓ Good (v0.3) |
| Gem backgrounds via a data-driven `background_override`, not a new named style | Reuses the priced-unique clone-null-append path; kept the 999.1 style refactor out of scope | ✓ Good (v0.3) |
| `corrupted` emits a bare boolean (`Corrupted False`), no operator | PoE2 boolean conditions take no comparator | ✓ Good (v0.3) |
| OR-semantics as separate rules (sockets vs quality) | Filter blocks are AND-only; one combined rule would demand *both* | ✓ Good (v0.3) |
| `exceptional` kept as its own style, separate from `S` (same pink today) | Lets the exceptional-gear color be retuned independently later | ✓ Good (v0.3) |
| `exceptional` landed as `Option<Style>` rather than a required `Styles` field | Matches the `craftable` pattern so `--config` files without it keep parsing | ✓ Good (v0.3 — adjusted in code review) |
| One consolidated fixture re-baseline at the last emitter-output phase | Avoided re-baselining the oracle three times across Phases 8-10 | ✓ Good (v0.3) |

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
*Last updated: 2026-06-27 after the v0.3 milestone — full evolution review: v0.3 requirements moved to Validated, Active cleared for v0.4, v0.3 decisions logged, Context refreshed with shipped state; full suite verified green at tag v0.3.*
