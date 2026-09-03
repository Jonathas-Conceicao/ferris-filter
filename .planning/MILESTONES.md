# Milestones

## v0.3 — Economy Items & Exceptional Gear

**Shipped:** 2026-06-27
**Phases:** 8-10 (3) · **Plans:** 5 · **Tasks:** 13 · **Commits:** 54 · **Tests:** 133 lib + integration green
**Timeline:** 2026-06-12 → 2026-06-27 · **Code:** ~7,534 lines Rust (+1,453/-726 in src/tests)

**Delivered:** Tracked three new/broken poe.ninja economy categories with correct in-game
matching and per-category gem styling, and added a `corrupted` rule condition plus
exceptional-gear highlighting for uncorrupted extra-socket / high-quality drops.

**Key accomplishments:**

1. **`mapping.json` schema v2** — a `gemFilters` array (UncutGems + LineageSupportGems, each
   with `backgroundOverride`) plus the `Verisium` overview type; `Mapping`/`GemFilter` Rust
   structs reshaped to match, `minLevel` dropped.
2. **Uncut Gems actually match in-game** — the `build_prices` gem branch was rewritten to emit
   one `PriceItem` per gem level (`Uncut Skill Gem (Level N)`) with a prefix-derived plural
   class; subtypes merge in the pricer into one multi-class `TierGroup`, and the emitter gained
   multi-class emit support. The old `strip_gem_level` + `gem_best` dedup — the root bug — is gone.
3. **Per-category gem backgrounds** — `emit_currency_sections` carries
   `CategoryBlock.background_override` through the priced-unique clone-null-append pattern:
   Uncut Gem blocks emit `SetBackgroundColor 80 70 55 255`, Lineage Support Gems
   `10 150 150 255`, with no category-name string checks in the emitter and a guard test
   against duplicate background lines.
4. **`corrupted` when-condition** — `When.corrupted: Option<bool>` emitting the bare boolean
   `Corrupted True` / `Corrupted False` (no `==`), slotted between `sockets` and `baseType` in
   the deterministic emit order, `serde(default)` for backward compatibility.
5. **Exceptional-gear highlighting** — three rules at the top of `config.json` `rules` (ahead of
   recycle): sockets >= 3 for 2h/body, sockets >= 2 for 1h/smaller, quality >= 24 — all
   `corrupted: false`, all using the new pink `exceptional` style (Pink Cross minimap, no sound),
   with a guard test asserting both the styling and the ordering vs recycle.
6. **One consolidated fixture re-baseline** — deferred to Phase 10 (the last emitter-output
   phase): `seed-universal.filter` + the insta snapshot re-baselined, `generate_block_counts`
   bumped 74 → 77 Show / 3 Hide.

**Audit:** `passed` — requirements 9/9, phases 3/3, integration 8/8, E2E flows 5/5.

**Known deferred items at close:** 0 open GSD artifacts (audit clear). Carried tech debt:
`parity_ingame` test failing (pre-existing, predates Phase 10); two test-only compiler warnings
(pre-existing); `10-01-SUMMARY.md` prose stale on `exceptional` being a required vs optional
`Styles` field. Backlog Phase 999.1 (config-driven style vocabulary) deliberately not pulled in.

**Archives:** [v0.3-ROADMAP.md](milestones/v0.3-ROADMAP.md) · [v0.3-REQUIREMENTS.md](milestones/v0.3-REQUIREMENTS.md) · [v0.3-MILESTONE-AUDIT.md](milestones/v0.3-MILESTONE-AUDIT.md)

## v0.2 — Declarative Rule Model

**Shipped:** 2026-06-12
**Phases:** 5-7 (3) · **Plans:** 7 · **Commits:** ~38 · **Tests:** 154 green
**Timeline:** 2026-06-05 → 2026-06-12

**Delivered:** Replaced the generator's hardcoded gear/flask band logic with a general,
declarative rule schema in `config.json`, making the fully-functional in-game filter
reproducible from config alone.

**Key accomplishments:**

1. **Rule schema** — an ordered `rules` array (first-match-wins) with a full `when` condition
   vocabulary (class, compact rarity comparator, areaLevel min/max, base-defence/quality/sockets,
   BaseType, UnidentifiedItemTier) + `classGroups` aliases; serde-validated, unknown-style hard error.
2. **Emitter rewrite** — generic `emit_rules` compiler replaces the hardcoded
   `emit_gear_bands`/`emit_flask_charm_bands`/`emit_recycle`; `config.json` migrated to rules;
   the dead `areaLevelBands` model removed.
3. **Migration parity** — `generate --early-game` reproduces the pre-edit baseline and (with the
   capability rules) the author's in-game filter with zero non-reorder policy differences;
   `seed-universal.filter` re-baselined; pricer currency/unique sections untouched.
4. **New capabilities as config** — the 7 in-game edits (weapon/armour caps, flask/charm
   Magic+Rare + rarity-bounded Hide, Normal charms ≥78, jewellery hide, unidentified-tier
   highlight via a new `unidentified` style).
5. **Docs + fix** — CLAUDE.md realigned to the rule model (UnidentifiedItemTier + base-defence
   confirmed); dropped the invalid `Class "Tablet"` rows that failed to load in-game.

**Known deferred items at close:** BaseType-matched, economy-tiered Tablets category (needs
pricer/emitter support for class-less category blocks). Generate QoL + sync extensions deferred.

**Archives:** [v0.2-ROADMAP.md](milestones/v0.2-ROADMAP.md) · [v0.2-REQUIREMENTS.md](milestones/v0.2-REQUIREMENTS.md) · [v0.2-MILESTONE-AUDIT.md](milestones/v0.2-MILESTONE-AUDIT.md)

## v0.1 — Initial Release

**Shipped:** 2026-05-28
**Phases:** 1-4 (4) · **Plans:** 14 · **Commits:** 127 · **LOC:** ~4,970 Rust
**Timeline:** 2026-05-21 → 2026-05-28

**Delivered:** A Rust CLI that compiles a human-editable policy model into a deterministic PoE2
loot filter and prices currency from poe.ninja.

**Key accomplishments:**

1. **Foundation** — `ferrisfilter` crate + clap CLI (`generate`/`sync`) + serde models for `config.json`/`tiers.json`/`prices.json` with field-path-aware validation errors.
2. **Static generate** — deterministic emitter reproducing the hand-built `universal.filter` byte-for-byte, locked by an insta snapshot + `artifact_guard` test (model-path block tags, AreaLevel bands, flask/charm ladder, recycle, fallthrough).
3. **Pricing layer** — cross-rate normalization to chaos + `tiers.json` cutoffs, TIER-04 abort on bad reference rates, graceful no-`prices.json` degradation; `currency.json` dropped from the generate path.
4. **Sync** — poe.ninja PoE2 API spike resolved, then `sync` (prices/metadata) with live league resolution, gem≥18 filter, anchors-first ordering, and atomic failure-safe writes.
5. **Extras** — `install` subcommand (replaces `install.bat`); early-league fallback (`prices-fallback.json` + `tiers-fallback.json` + `--early-game`) with cross-rates derived from the anchor orbs.

**Known deferred items at close:**

- Live softcore no-flag `sync` happy path awaits a new active PoE2 league ("Fate of the Vaal" ended; tool fails safe). Verification + UAT signed off as working on 2026-05-28 via the live `--standard` run (318 items). See STATE.md → Deferred Items.
- Lenient-parse guard for Standard-league unique endpoints (non-blocking); optional no-flag `sync` → `--standard` fallback/hint.

**Archives:** [v0.1-ROADMAP.md](milestones/v0.1-ROADMAP.md) · [v0.1-REQUIREMENTS.md](milestones/v0.1-REQUIREMENTS.md)
