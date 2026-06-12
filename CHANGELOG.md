# Changelog

All notable changes to **FerrisFilter** are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Maintenance:** keep `[Unreleased]` current as work lands, and **whenever a milestone is
> completed** (e.g. via `/gsd-complete-milestone`) move its entries into a new dated version
> section below with a one-line milestone summary.

## [Unreleased]

### Added

- **`generate --config <path>`** — point the `generate` subcommand at a config file other than
  the default `config.json`. Defaults to `config.json`, so existing invocations are unchanged.

## [0.2.0] - 2026-06-12

**Declarative Rule Model** — the generator's hardcoded gear/flask band logic is replaced by a
general, declarative rule schema in `config.json`, so any class + level-range + rarity (+
defence / quality / sockets / unidentified-tier / base-type) policy is expressible in config.
The full in-game filter is now reproducible from `config.json` alone. Tagged `v0.2`.

### Added

- **Declarative rule schema** (`config.json` `rules` array) — an ordered, first-match-wins list
  of show/hide rules. Each rule references a named style and an optional `when` block whose
  condition vocabulary covers class lists, a compact rarity comparator (`">=Magic"` etc.),
  `areaLevel` min/max, base-defence/`Quality`/`Sockets` numeric conditions, `BaseType` lists,
  and `UnidentifiedItemTier`. Optional `classGroups` aliases. Serde-validated with field-path
  errors; unknown style names are a generate-time hard error.
- **New capabilities now expressible as config** — build-weapon Magic+Rare up to a configurable
  area-level cap, build-armour upper cap, flask/charm mid-band Magic+Rare with a rarity-bounded
  Hide fallback, Normal charms surfaced at high area level, low-rarity jewellery hide, and a
  high-tier unidentified-item highlight (new `unidentified` style).

### Changed

- **Emitter rewritten** as a generic rule compiler (`emit_rules`) — the hardcoded
  `emit_gear_bands` / `emit_flask_charm_bands` / `emit_recycle` special-casing is gone; all
  policy blocks are produced by iterating the rule list. Price-driven currency/unique sections
  are unchanged. The `seed-universal.filter` artifact baseline was deliberately re-locked.
- **`config.json` migrated** to the rule schema; the dead `areaLevelBands` model
  (`AreaLevelBand`/`BandShow`/`ArmourShow`/`RarityRule`) was removed.
- **CLAUDE.md realigned** to the declarative rule model; `UnidentifiedItemTier` and the
  base-defence filters are documented as confirmed working in-game.

### Fixed

- Dropped the **Tablets** category and **Vaal Infuser** from the price data — they emitted an
  invalid `Class "Tablet"` block (tablets have no usable PoE2 filter class; they match by
  BaseType only), which caused the filter to fail to load in-game. (A BaseType-matched,
  economy-tiered tablet category is deferred — see PROJECT.md.)

## [0.1.0] - 2026-05-28

**Initial Release** — a Rust CLI that compiles a human-editable policy model into a Path of
Exile 2 loot filter and prices currency from poe.ninja. Tagged `v0.1`.

### Added

- **`generate` command** — compiles `filter-model.json` + `config.json` into `universal.filter`
  deterministically (byte-for-byte reproducible, guarded by a snapshot/`artifact_guard` test).
  Implements the Dracula tier palette, the single-file `AreaLevel` progression, build-gear
  taper, and the "hide what we don't want, style the highlights" philosophy.
- **`sync` command** — fetches live PoE2 economy data from poe.ninja and writes `prices.json`;
  resolves the current league and honors `--standard`/`--hc`. `sync metadata` rebuilds the
  committed poe.ninja-type → filter-class `mapping.json`.
- **`install` command** — copies a generated `.filter` into the in-game config folder
  (`--dest`/`--input` overrides); replaces the old `install.bat`.
- **Pricing & tier system** — `tiers.json` cross-rate math normalizes prices to chaos and
  assigns each currency a tier (S–F); price-tiered unique highlights; graceful handling of a
  missing `prices.json`.
- **Early-league fallback** — committed `prices-fallback.json` + `tiers-fallback.json` pair,
  used together when `prices.json` is absent/empty (with a warning) or when `generate
  --early-game` is passed. Cutoffs are exalt-denominated (`Unit::Exalt`) so currency tiers like
  the poe2filter.com export; cross-rates are derived from the anchor orbs (no stored rates).

[Unreleased]: https://github.com/Jonathas-Conceicao/poe2-myfilter/compare/v0.1...HEAD
[0.1.0]: https://github.com/Jonathas-Conceicao/poe2-myfilter/releases/tag/v0.1
