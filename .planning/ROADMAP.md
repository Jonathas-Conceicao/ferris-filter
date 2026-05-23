# Roadmap: FerrisFilter

## Overview

FerrisFilter goes from zero to a working Rust CLI in four phases. Phase 1 lays the Cargo
skeleton and serde models for all input files. Phase 2 wires those models into a complete
static generator that reproduces the existing hand-built `universal.filter` byte-for-byte —
establishing the regression baseline with an insta snapshot test. Phase 3 adds the pricing
layer: cross-rate math, tier assignment, unique-base-collision resolution, and graceful
handling of a missing `prices.json`. Phase 4 resolves the unresolved poe.ninja PoE2 API
surface (spike), then builds the `sync` subcommand on top of that confirmed interface.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Foundation** - Cargo crate + CLI shell + serde models for all JSON inputs
- [ ] **Phase 2: Static Generate** - Deterministic filter emitter that reproduces universal.filter byte-for-byte
- [ ] **Phase 3: Pricing Layer** - Cross-rate math, tier assignment, price-tiered unique highlights
- [ ] **Phase 4: Sync** - poe.ninja API spike + sync subcommand that fetches and writes prices.json

## Phase Details

### Phase 1: Foundation
**Goal**: The Rust crate exists, compiles, and can be invoked as a CLI with both subcommands visible; all JSON input shapes are modeled and round-trip through serde
**Depends on**: Nothing (first phase)
**Requirements**: CLI-01, CLI-03, CLI-05, CFG-01, CFG-02, CFG-03
**Success Criteria** (what must be TRUE):
  1. `cargo build` succeeds and `ferrisfilter --help` lists `generate` and `sync` subcommands
  2. `ferrisfilter generate --help` shows `--output <path>` flag documentation
  3. `ferrisfilter generate` with a missing or malformed `config.json` exits non-zero and prints a message containing the bad field path and expected type
  4. `tiers.json` is validated on load and produces an actionable error when a cutoff is missing or out of order
  5. `ferrisfilter nonexistent` exits non-zero with a clear unknown-command message
**Plans**: TBD

### Phase 2: Static Generate
**Goal**: `ferrisfilter generate` compiles the existing `config.json` and curated currency lists into a `universal.filter` that is byte-identical to the hand-built original — confirmed by an insta snapshot test
**Depends on**: Phase 1
**Requirements**: GEN-01, GEN-02, GEN-03, GEN-04
**Success Criteria** (what must be TRUE):
  1. Running `ferrisfilter generate` produces a `universal.filter` whose SHA-256 matches the existing hand-built file (snapshot test passes)
  2. Each block in the output carries a comment tag linking it to its source in the model (e.g. `[areaLevelBands "6-25".weapons -> styles.unchanged]`)
  3. Items with no rules (uniques, gems, jewels, jewellery, quest, waystones, trials, unidentified) produce no blocks in the output — they fall through to game default
  4. Running `ferrisfilter generate` twice in succession with identical inputs produces identical output (determinism test)
**Plans**: TBD

### Phase 3: Pricing Layer
**Goal**: `generate` can consume a `prices.json` to assign price tiers to currency and uniques, aborts loudly on corrupt reference rates, and degrades gracefully when `prices.json` is absent
**Depends on**: Phase 2
**Requirements**: GEN-05, TIER-01, TIER-02, TIER-03, TIER-04
**Success Criteria** (what must be TRUE):
  1. With a fixture `prices.json` present, currency and unique blocks in the generated filter reflect the price-derived tier assignments from `tiers.json` cutoffs
  2. When a unique base type is shared by multiple priced uniques, the generated block uses the highest tier among them
  3. Running `generate` without `prices.json` present emits a warning to stderr and produces a valid filter without price-tiered blocks (no crash)
  4. Running `generate` when Exalt, Chaos, or Divine has a null or zero price in `prices.json` aborts with a clear error naming the missing currency
**Plans**: TBD

### Phase 4: Sync
**Goal**: `ferrisfilter sync` fetches live PoE2 economy data from poe.ninja and writes a well-formed `prices.json`; the poe.ninja PoE2 API surface is confirmed via a spike before any HTTP code is written
**Depends on**: Phase 3
**Requirements**: CLI-02, CLI-04, SYNC-01, SYNC-02, SYNC-03, SYNC-04, SYNC-05
**Success Criteria** (what must be TRUE):
  1. The poe.ninja PoE2 API spike is resolved: base URL, league-resolution endpoint, per-category `type`/`overviewName` values, response field names (`primaryValue`/`chaosValue`/etc.), and `baseType` field are all confirmed and documented before any HTTP client code is written
  2. `ferrisfilter sync` without flags writes a `prices.json` with Exalt, Chaos, and Divine as the first three entries, sourced from the current softcore trade league
  3. `ferrisfilter sync --hc` and `ferrisfilter sync --standard` (combinable) target the correct league; the fetched league name and a timestamp appear in `prices.json`
  4. Uncut gems are included in `prices.json` only at gem level >= 18
  5. A network failure or empty poe.ninja response does not overwrite or corrupt an existing `prices.json`
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 0/TBD | Not started | - |
| 2. Static Generate | 0/TBD | Not started | - |
| 3. Pricing Layer | 0/TBD | Not started | - |
| 4. Sync | 0/TBD | Not started | - |
