# Milestones

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
