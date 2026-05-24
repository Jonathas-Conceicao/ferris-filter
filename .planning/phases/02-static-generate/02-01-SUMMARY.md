---
phase: 02-static-generate
plan: "01"
subsystem: models/config
tags: [refactor, serde, type-safety, determinism, tdd]
dependency_graph:
  requires: [01-03]
  provides: [typed-Styles-struct, RarityRule-enum, insta-dev-dep]
  affects: [src/models/config.rs, src/main.rs, Cargo.toml]
tech_stack:
  added: [insta 1.47.2]
  patterns: [serde-rename, serde-alias, closed-enum-dispatch]
key_files:
  created: []
  modified:
    - src/models/config.rs
    - src/main.rs
    - Cargo.toml
decisions:
  - "D-11: HashMap<String, Style> replaced with Styles named-fields struct — serde rename handles JSON uppercase keys"
  - "D-10: RarityRule closed enum replaces Option<String> — serde alias covers otherGear bare 'Rare' value"
  - "RareOnly uses serde(alias = 'Rare') so both 'Rare only' and 'Rare' deserialize to the same variant"
  - "Styles struct uses deny_unknown_fields — unknown tier key fails deserialization"
  - "main.rs stub minimal-fix: drop styles count entirely (Plan 02-02 replaces full body)"
metrics:
  duration_minutes: 15
  completed: "2026-05-24"
  tasks_completed: 3
  tasks_total: 3
  files_modified: 3
---

# Phase 02 Plan 01: Typed Models Refactor Summary

**One-liner:** Eliminated `HashMap<String, Style>` styles footgun (D-11) and freeform rarity strings (D-10) via `Styles` named-fields struct and `RarityRule` closed enum, with `insta` dev-dep for downstream snapshot tests.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1+2 | Styles struct (D-11) + RarityRule enum (D-10) | 8247748 | src/models/config.rs |
| 3 | Fix main.rs stub + add insta dev-dep | 7482a6f | src/main.rs, Cargo.toml |

## What Was Built

### Task 1: Styles Named-Fields Struct (D-11)

`src/models/config.rs` — new `Styles` struct replaces `HashMap<String, Style>` in `Config.styles`:

- 10 named fields: `s`, `a`, `b`, `c`, `d`, `e`, `f`, `recycle`, `unchanged`, `hidden`
- `#[serde(rename = "S")]` etc. maps JSON uppercase keys to snake_case Rust fields
- `unchanged` and `hidden` are already lowercase — no rename needed
- `deny_unknown_fields` — a typo tier key (e.g. `"SS"`) fails deserialization
- No change to `config.json` — serde handles the mapping transparently

Emitter access is now `config.styles.s`, `config.styles.recycle` — fully deterministic, no HashMap lookup nondeterminism.

### Task 2: RarityRule Closed Enum (D-10)

`src/models/config.rs` — new `RarityRule` enum replaces `Option<String>` / `String` rarity fields:

| Variant | JSON value(s) | Filter emission |
|---------|---------------|-----------------|
| `Any` | `"any rarity"` | (no Rarity condition) |
| `RareOnly` | `"Rare only"`, `"Rare"` (alias) | `Rarity == Rare` |
| `MagicPlusRare` | `"Magic + Rare"` | `Rarity >= Magic` |
| `Magic` | `"Magic"` | `Rarity == Magic` |
| `QualityOnly` | `"Quality only"` | `Quality >= 1` |

Key decision: `otherGear` in `config.json` uses the bare string `"Rare"` (a sixth rarity-rule value). The `serde(alias = "Rare")` on `RarityRule::RareOnly` handles this without any JSON change. Both `"Rare only"` and `"Rare"` deserialize to `RarityRule::RareOnly` and will emit `Rarity == Rare` identically.

Fields changed:
- `BandShow.weapons`: `Option<String>` → `Option<RarityRule>`
- `BandShow.all_armour`: `Option<String>` → `Option<RarityRule>`
- `BandShow.other_gear`: `Option<String>` → `Option<RarityRule>`
- `BandShow.flasks_charms`: `Option<String>` → `Option<RarityRule>`
- `ArmourShow.rarity`: `String` → `RarityRule`

### Task 3: main.rs Stub Fix + insta Dev-Dep

- `src/main.rs`: Removed `config.styles.len()` call (Styles is a struct, not a map). Replaced with minimal println keeping the load intact.
- `Cargo.toml`: Added `insta = "1.47.2"` to `[dev-dependencies]` — ready for Plan 02-02/02-03 snapshot tests.

## Tests Added

| Test | Purpose |
|------|---------|
| `config_styles_named_fields` | Verifies all 10 tier fields accessible by name; hidden.action == Hide |
| `rarity_rule_variants_roundtrip` | "Magic + Rare" ↔ MagicPlusRare round-trip |
| `rare_alias_maps_to_rare_only` | "Rare" (otherGear) deserializes to RarityRule::RareOnly |
| `rarity_rule_rejects_unknown` | "Legendary" fails deserialization |
| `rarity_rule_all_variants_parse` | All five rarity-rule strings parse correctly |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Applied main.rs stub fix in Task 1/2 instead of waiting for Task 3**

- **Found during:** Task 1 — after writing the new config.rs with Styles struct, `cargo build` failed because `config.styles.len()` no longer compiled
- **Issue:** The tasks are in a single binary crate; the binary compilation failure prevented running ANY tests (including the lib-unit-style config tests compiled into the binary)
- **Fix:** Applied the `main.rs` minimal fix (removing `.len()` call) immediately to unblock testing, then formally committed it as part of Task 3
- **Files modified:** src/main.rs
- **Commit:** 7482a6f (grouped with Task 3)

**Note:** Tasks 1 and 2 were committed together (single `config.rs` file, changes are inextricably linked in the same refactor). Both D-10 and D-11 are addressed in commit 8247748.

## Known Stubs

None — this plan delivers typed models only, no rendering/emission logic.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries introduced. The `Styles` struct and `RarityRule` enum **strengthen** T-02-01 mitigation: unknown tier names and invalid rarity strings now fail deserialization at load time (as required by the threat register).

## Self-Check

- [x] `src/models/config.rs` exists and contains `pub struct Styles` and `pub enum RarityRule`
- [x] `src/main.rs` exists and does not contain `config.styles.len()`
- [x] `Cargo.toml` contains `insta = "1.47.2"` under `[dev-dependencies]`
- [x] Commit 8247748 exists (config.rs refactor)
- [x] Commit 7482a6f exists (main.rs + Cargo.toml)
- [x] `cargo build` exits 0
- [x] `cargo test` exits 0 — 26 unit + 4 cli_errors + 4 cli_generate = 34 tests pass
- [x] `config_parses_real_file` passes (including otherGear "Rare" alias end-to-end)
- [x] `rare_alias_maps_to_rare_only` passes
- [x] No ROADMAP.md phase-checkbox changes made
