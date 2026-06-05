---
phase: 07-new-capabilities-docs
plan: "01"
subsystem: emitter/config
tags: [cap-rules, unidentified-style, config-json, rule-model]
dependency_graph:
  requires: [06-01-SUMMARY.md]
  provides: [unidentified-style, CAP-01..06-rules]
  affects: [config.json, src/models/config.rs, src/emitter.rs]
tech_stack:
  added: []
  patterns: [declarative-rule-model, style-resolution]
key_files:
  created: []
  modified:
    - config.json
    - src/models/config.rs
    - src/emitter.rs
decisions:
  - "CAP-06 weapon+armour class list omits Quarterstaves (matches foo.filter oracle exactly)"
  - "generate_block_counts test updated inline (83 Show → 86, 2 Hide → 3) as part of CAP rules commit"
  - "unidentified style uses no text/border/background/font — preserves native rarity color"
metrics:
  duration: "~4 minutes"
  completed: "2026-06-05T11:51:14Z"
  tasks_completed: 2
  files_modified: 3
---

# Phase 7 Plan 01: Encode CAP Rules + unidentified Style Summary

**One-liner:** 7 in-game filter edits (CAP-01..06) encoded as declarative rules in config.json, plus the new `unidentified` style plumbing (Styles struct + resolve_style + inline test JSON).

## What Was Built

### Task 1: Add `unidentified` style (commit ff99220)

Three coordinated changes to introduce the new style name:

- **config.json**: added `"unidentified"` key to `styles` after `"hidden"` — beam Orange, minimap size 2 Orange Kite, sound id 2 vol 200, Show action, no text/border/background/font (preserves native rarity color).
- **src/models/config.rs**: added `pub unidentified: Style,` field to the `Styles` struct (after `hidden`); updated the inline minimal-styles JSON literal in `config_with_rules_and_class_groups_parses` test to include the `"unidentified"` key.
- **src/emitter.rs**: added `"unidentified" => &styles.unidentified,` arm to `resolve_style`; updated the `GenerateError::UnknownStyle` error message to include `unidentified` in the valid styles list.

Verified: `cargo test --lib config_ --quiet` — 7 tests passed.

### Task 2: Encode 7 CAP edits as rules in config.json (commit 9410f9a)

All 7 edits encoded per `foo.filter` oracle:

| Edit | Rule change | Key condition |
|------|-------------|---------------|
| CAP-01 | Rename + modify `bands 26+: build weapons` | `areaLevel.max=75`, `rarity: ">=Magic"` |
| CAP-02 | Add `areaLevel.max=75` to `bands 46+: build armour (pure ES)` | `areaLevel: { min: 46, max: 75 }` |
| CAP-03a | Change flask/charm mid-band rarity | `"Magic"` → `">=Magic"` |
| CAP-03b | Add rarity to flask/charm fallback Hide | `rarity: "<=Magic"` (Rare flasks fall through) |
| CAP-04 | NEW rule: `bands 78+: normal charms` | `areaLevel.min=78`, `rarity: "==Normal"` |
| CAP-05 | NEW rule: jewellery fallback Hide | `class: [Rings, Amulets, Belts]`, `rarity: "<=Rare"` |
| CAP-06 | TWO NEW rules: unidentified high-tier | `unidentifiedTier.min=4`, style `unidentified` — one for weapons+armour, one for jewellery |

CAP-06 weapon+armour class list per oracle (foo.filter line 79): `Bows, Crossbows, Foci, One Hand Maces, Quivers, Sceptres, Spears, Talismans, Staves, Two Hand Maces, Wands, Body Armours, Helmets, Gloves, Boots, Shields, Bucklers` — NOTE: "Quarterstaves" deliberately omitted (matches author's in-game list).

Rule placement follows foo.filter ordering:
- CAP-06 rules inserted after `bands 46+: build armour (pure ES)`, before `bands <=45: any other Rare gear`
- CAP-05 inserted after `gear fallback: hide unmatched equipment`
- CAP-04 inserted before `flasks/charms fallback`

Verified:
- `cargo run -- generate --early-game --output target/cap-check.filter` succeeds
- `UnidentifiedItemTier >= 4` appears twice in output
- `AreaLevel <= 75` appears twice (CAP-01 weapons, CAP-02 armour)
- `Rarity == Normal` present (CAP-04)
- `Rarity <= Rare` present (CAP-05 jewellery Hide)
- `Rarity >= Magic` present (CAP-03a flask/charm mid-band)
- `Rarity <= Magic` present (CAP-03b flask/charm fallback)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Updated generate_block_counts test to match new block counts**
- **Found during:** Task 2 verification (`cargo test --lib`)
- **Issue:** The `emitter::tests::generate_block_counts` test hardcoded `83 Show / 2 Hide` block counts from Phase 6. Adding 3 new Show blocks (CAP-06 ×2, CAP-04) and 1 new Hide block (CAP-05) changed counts to `86 Show / 3 Hide`.
- **Fix:** Updated the assertions in the test to reflect the new counts.
- **Files modified:** `src/emitter.rs`
- **Commit:** 9410f9a

## Known Stubs

None — all 7 CAP rules produce real filter output with real conditions; no placeholder data.

## Integration Test Failures (Expected — deferred to Plan 07-02)

The following CLI integration tests fail as expected and are deferred to Plan 07-02 for re-baselining:

| Test | Reason |
|------|--------|
| `artifact_guard` | `tests/fixtures/seed-universal.filter` is stale — new policy blocks added (CAP rules) |
| `generate_snapshot` | insta snapshot needs updating for same reason |
| `parity_policy_blocks_match_oracle` | Parity oracle needs to be switched from Phase 6 baseline to `parity-ingame.filter` |

All 125 lib tests (`cargo test --lib`) pass. `cargo build` passes with no errors.

## Self-Check: PASSED

- config.json exists and parses: confirmed (cargo test --lib config_parses_real_file passes)
- src/models/config.rs Styles struct has `unidentified` field: confirmed
- src/emitter.rs resolve_style has `"unidentified"` arm: confirmed
- Commit ff99220 exists: confirmed (git log)
- Commit 9410f9a exists: confirmed (git log)
- generate --early-game succeeds: confirmed (18714 bytes written)
- All 6 CAP behaviors present in output: confirmed (grep checks all passed)
