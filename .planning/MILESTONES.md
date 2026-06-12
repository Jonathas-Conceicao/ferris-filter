# Milestones

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
