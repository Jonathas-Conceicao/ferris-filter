# FerrisFilter

## What This Is

FerrisFilter is a Rust tool for Path of Exile 2 loot filters. Its CLI compiles a human-editable configuration into a working `.filter` and fetches live item prices from poe.ninja so highlight tiers are driven by real economy values. Its overlay, `ff`, draws over the running game and edits any working PoE2 filter in place — its own output or a FilterBlade / NeverSink / hand-written one — losslessly, with named styles, tier cutoffs and a regenerable economy section configurable from the overlay.

## Core Value

`generate` turns the config into a correct, deterministic PoE2 loot filter — the same translation currently done by hand, done by the tool instead. If nothing else works, this must.

## Current Milestone: v0.6 New PoE2-based Tool UI

**Goal:** `ff` is rebuilt in Path of Exile 2's own Options-window style, on the shared skin proven
by the `ff-proto` prototype (quick tasks 261015-tz2 / 261016-1de), and the prototype binary is
retired.

**Target features:**
- One Cargo workspace: `ff` stays the default package; `tools/ff-art` is a member runnable from the
  repo root (`cargo run -p ff-art`)
- New overlay shell: left-docked 660px full-height window on the game's monitor, DPI-correct, game
  header, RULES / CONFIG / ECONOMY / ABOUT tabs, REVERT / RELOAD / APPLY bar with a one-line,
  truncating, colour-coded status (absorbs backlog 999.11)
- RULES tab on the skin, every existing editor capability kept; new rules are added at the top
- Rule dividers: an explicit `#`-banner comment in the filter renders as a named, collapsible
  section; the economy section becomes the default (locked) divider
- CONFIG style table with a column header and aligned (greyed) order arrows; a line-by-line style
  editor with a skinned colour-picker popup and Delete-with-confirm at the bottom
- ECONOMY tab (league, last sync, priced items, SYNC ECONOMY) and ABOUT tab (identity, version,
  repo, ownership disclaimer, third-party credits)

**Key context:** Branch `milestone/v0.6-new-poe2-based-tool-ui` was cut from `topic/dev`, so it
also carries the Simple Defences rework (quick 261015-nbp) and both prototype rounds; they ship to
`main` with v0.6. Game art is GGG's (README credits); Ferris is public-domain community art.
Research skipped (the prototype answered the look). Sound preview and long-distance rule
reordering are backlog. The Cargo version now follows tags (0.5.0, bumped to 0.6.0 at close).

<details>
<summary>✅ v0.5 Generic Filter Editor & Initial App Configuration — SHIPPED 2026-10-15</summary>

**Goal:** FerrisFilter can open, edit and save any working PoE2 filter, not just its own output,
with the style palette and economy tier cutoffs configurable from the overlay.

Delivered: lossless parser, `styles.json` with stable ids and committed defaults, the filter file as
the store with a fenced economy section and guarded writes, a rule list + full-grammar editor, a
CONFIG tab with one Apply, single-instance `ff`, in-game reload after Apply.

**Audit:** `tech_debt` — 73/73 requirements, 13/13 phases, no gaps. Archives:
[v0.5-ROADMAP.md](milestones/v0.5-ROADMAP.md) · [v0.5-REQUIREMENTS.md](milestones/v0.5-REQUIREMENTS.md) · [v0.5-MILESTONE-AUDIT.md](milestones/v0.5-MILESTONE-AUDIT.md)

</details>

<details>
<summary>✅ v0.4 Overlay Filter Editor Rework — SHIPPED 2026-09-17</summary>

See Current State below and [v0.4-ROADMAP.md](milestones/v0.4-ROADMAP.md).

</details>

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

**Shipped:** v0.5 Generic Filter Editor & Initial App Configuration (2026-10-15) — `ff` now edits
any working PoE2 filter, not just FerrisFilter's own output. A lossless parser round-trips foreign
filters byte-for-byte; `dynamic.json` is gone and the game-folder `.filter` is the store, with a
fenced economy section regenerated on Apply and guarded writes (changed-on-disk refusal, atomic
write, one-time `.ffbak`). Every condition and action is editable; named styles live in
`styles.json` (stable ids, `# ff:style` links rewritten on Apply) with committed
`styles-default.json` / `tiers-default.json` defaults; a CONFIG tab edits styles, tier cutoffs and
the target filter under one all-or-none Apply. `ff` runs as a single instance, launches from
Explorer (app root resolved from the exe), and sends `/reloaditemfilter` after Apply. ~50,244 LOC
Rust; 1142 tests green. This closes v0.4's "launch by double-click" gap.

**Prior:** v0.4 Overlay Filter Editor Rework (2026-09-17) — `ff`, a GUI overlay that draws
over a running Path of Exile 2 and edits filter rules in place. A rule expands into controls for
style, classes, base types, rarity, defences, sockets, quality, corrupted and unidentified tier;
Apply is the single writer, compiling `dynamic.json` into `dynamic.filter` straight into the game
folder, with Revert to discard unsaved edits and a SYNC ECONOMY button that refreshes prices on a
worker thread. Underneath it: `compile::` as a real library API (Phase 11) and a complete headless
`dynamic.json → dynamic.filter` pipeline reachable from the CLI alone (Phase 12). ~16,093 LOC Rust;
368 tests green, build warning-free.

**Prior:** v0.3 Economy Items & Exceptional Gear (2026-06-27) — `sync` now tracks Verisium and
Lineage Support Gems, Uncut Gems finally match in-game (level kept in `BaseType`, per-subtype plural
`Class`, no `minLevel` floor), priced gem blocks carry a per-category background over the value-tier
text/border, and the rule vocabulary gained `corrupted` — driving three exceptional-gear rules
(sockets >= 3 / >= 2, quality >= 24, all uncorrupted) in the new pink `exceptional` style.
`generate` + `sync` + `install` all live. ~7,534 LOC Rust; 133 lib tests + integration suite green.

**Prior:** v0.2 Declarative Rule Model (2026-06-12) replaced the generator's hardcoded gear/flask
band logic with a declarative `rules` schema, making the full in-game filter reproducible from config.

**Next milestone:** v0.6 New PoE2-based Tool UI (in progress). Remaining candidates: the open backlog
(999.3 load message for online filters, 999.4 economy change summary, 999.7 locked rules / `Import`
layering, 999.8 rest of the configuration tab, 999.9 presets, 999.10 raw-text block editing, 999.11
UI review, 999.12 in-game keyword checks, 999.13 unmatchable-condition guidance, 999.14 `Minimal`),
the v0.5 tech debt (integration warnings W1-W4 in the audit), and the longer-standing Deferred list
below.

**Open at close:** no blockers. Tech debt accepted per the v0.5 audit; upgrade note: an existing
install folder needs `styles-default.json` to be found as the app root. Full suite green at tag
`v0.5` — 1142 tests, 0 failures, `cargo fmt --check` clean.

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

- [x] **Lossless filter parsing and editing** — any PoE2 filter loads and saves byte-identical; every condition and action editable; unknown/PoE1-only lines kept as raw rows. *(Validated in Phases 16, 19-20, 24-27 — PARSE, EDIT, v0.5)*
- [x] **Filter file as the store** — `dynamic.json` removed; fenced, regenerable economy section; guarded atomic writes with a one-time `.ffbak`. *(Validated in Phase 18 — STORE, ECON, v0.5)*
- [x] **Config-driven styles** — `styles.json` with stable ids, resolved by name in `generate` and by id in `# ff:style` tags; committed `-default.json` fallbacks. *(Validated in Phases 17, 26, 28 — STYLE, DEF, v0.5)*
- [x] **Overlay CONFIG tab** — filter choice, styles and tier cutoffs under one all-or-none Apply. *(Validated in Phases 21, 25 — CONF, v0.5)*
- [x] **Single-instance `ff` and in-game reload after Apply.** *(Validated in Phases 22-23 — INST-01, RELOAD-01, v0.5)*
- [x] **`ff` launchable from Explorer** — app root resolved via `FERRISFILTER_HOME` or by walking up from the exe (closes the v0.4 known gap). *(Validated in Phase 18 — STORE-05, v0.5)*

### Active

See `.planning/REQUIREMENTS.md` (milestone v0.6).

### Deferred (next-milestone candidates)

- [ ] **v0.5 tech debt** (audit W1-W4): CLI `sync`/`generate`/`install` resolve against the
  working directory while `apply`/`ff` use the app root; CLI `apply` does not rewrite styled rules;
  CONFIG tier edits have no effect while fallback prices are in use; class picker disabled when
  `config.json`/`styles.json` is invalid.
- [ ] **Overlay robustness** (carried from the v0.4 close; launch-by-double-click and the
  `dynamic.json` items were resolved in v0.5):
  - Surface `price_source` in the stripe, so a fallback-priced Apply is not silent
  - A destination override in the GUI, matching the CLI's `--dest`
  - A league picker, replacing the pinned `sync::OVERLAY_LEAGUE`
  - A SIGTERM/Ctrl+C console-control handler so `ff` can be stopped from a terminal

- [ ] **BaseType-matched, economy-tiered priced category** (e.g. Tablets): support a priced category that emits `BaseType ==` with **no `Class` line** (tablets have no usable PoE2 filter class). Needs a `Tablets` type in `mapping.json` (spike: confirm poe.ninja serves tablet prices), pricer/emitter support for class-less category blocks, and corrected base names (`Ritual Tablet`/`Abyss Tablet`/…, no "Precursor"). Tablets currently fall through to game default after the v0.2 removal of the invalid `Class "Tablet"` rows.
- [ ] Generate QoL: `--dry-run`, `--diff`, `-v/--verbose` rule logging, stale-`prices.json` warning (GENX-01..04)
- [ ] Sync extensions: multiple build profiles (`--config builds/<name>.json`); alternate price source fallback, e.g. poe2scout (SYNX-01..02)
- [ ] Lenient-parse guard for Standard-league unique endpoints (different/empty shape); optional no-flag `sync` → `--standard` hint when no challenge league is active

### Out of Scope

- Hand-maintaining currency/tier base-lists — replaced by `sync` + price-driven tiering
- Filter rules for jewels, quest items, waystones, trials — these fall through to game default (established design philosophy). *(Jewellery hide and unidentified-tier highlight moved IN-scope in v0.2 — the in-game filter already curates them. **Uncut and Lineage Support gems moved IN-scope in v0.3** as priced, economy-tiered categories; all other gems still fall through.)*
- Blanket unique styling — only price-tracked uniques get rules; the rest fall through
- ~~GUI / web interface — CLI only for now~~ — **invalidated in v0.4**: `ff` is a GUI overlay. The
  reasoning held right up until the CLI's real limitation showed itself — a filter cannot be edited
  while the game is running without leaving the game. A web interface remains out of scope
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
| Lossless parser; edits patch only the edited lines | A foreign filter must survive load/save byte-for-byte; no "unsupported block" class | ✓ Good (v0.5) |
| The `.filter` file is the store; `dynamic.json` dropped | One source of truth; full regeneration of the dynamic filter given up deliberately | ✓ Good (v0.5) |
| Economy blocks fenced by `# ff:economy-begin/end` markers, regenerated on Apply | Lets the economy refresh inside a hand-edited filter without touching anything else | ✓ Good (v0.5) |
| Styles linked by stable id (`# ff:style <id> <name>`), not name | Custom styles can be renamed without orphaning rules; legacy name tags still resolve | ✓ Good (v0.5) |
| One all-or-none Apply across RULES and CONFIG | Author's second UAT round (UAT2): one action, written all or none, no half-applied state | ✓ Good (v0.5) |
| In-game reload via clipboard paste, no Ctrl+A | Same method as EE2 / Awakened PoE Trade; synthetic Ctrl+A fired another tool's hotkey | ✓ Good (v0.5) |
| Committed `styles-default.json` / `tiers-default.json`; live files git-ignored | Per-user edits from the overlay must not dirty the repo or break `artifact_guard` | ✓ Good (v0.5) |
| Overlay league pinned in `sync::OVERLAY_LEAGUE` | poe.ninja's `indexed` flag flips between leagues | ⚠️ Revisit (league picker) |

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
*Last updated: 2026-10-16 at the start of milestone v0.6*
