# Changelog

All notable changes to **FerrisFilter** are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Maintenance:** keep `[Unreleased]` current as work lands, and **whenever a milestone is
> completed** (e.g. via `/gsd-complete-milestone`) move its entries into a new dated version
> section below with a one-line milestone summary.

## [Unreleased]

_Nothing yet._

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
