---
phase: 07-new-capabilities-docs
plan: "02"
subsystem: tests/fixtures
tags: [parity-gate, cap-rules, fixtures, snapshot, oracle]
dependency_graph:
  requires: [07-01-SUMMARY.md]
  provides: [parity-ingame-oracle, parity_ingame-test, cap_is_rule_driven-test, seed-universal-rebaseline]
  affects: [tests/fixtures/parity-ingame.filter, tests/cli_generate.rs, tests/fixtures/seed-universal.filter, tests/snapshots/cli_generate__generate_snapshot.snap]
tech_stack:
  added: []
  patterns: [policy-block-parity-test, hermetic-generate-test, insta-snapshot]
key_files:
  created:
    - tests/fixtures/parity-ingame.filter
  modified:
    - tests/cli_generate.rs
    - tests/fixtures/seed-universal.filter
    - tests/snapshots/cli_generate__generate_snapshot.snap
decisions:
  - "parity_policy_blocks_match_oracle (Phase 6 pre-edit oracle) removed; replaced by parity_ingame pointing at parity-ingame.filter; old baseline kept as historical artifact"
  - "cap_is_rule_driven uses string.replace(max:75, max:60) mutation — simplest robust approach that avoids json parsing complexity"
  - "seed-universal.filter diff confirms pricer section unchanged (only policy section hunks)"
metrics:
  duration: "~10 minutes"
  completed: "2026-06-05T12:01:26Z"
  tasks_completed: 2
  files_modified: 4
---

# Phase 7 Plan 02: Parity Gate + Re-baseline Summary

**One-liner:** Phase 7 CAP acceptance gate — freeze foo.filter as parity-ingame.filter oracle, add parity_ingame + cap_is_rule_driven tests, re-baseline seed-universal.filter + insta snapshot for 7 CAP edits.

## What Was Built

### Task 1: Freeze oracle + add parity_ingame and cap_is_rule_driven tests (commit 980ceb0)

**tests/fixtures/parity-ingame.filter** — verbatim copy of `foo.filter` (the author's current in-game filter with all 7 CAP edits applied), committed as the stable Phase 7 parity oracle. Contains `UnidentifiedItemTier >= 4` (twice), `AreaLevel <= 75` (CAP-01/02), `Rarity >= Magic` (CAP-03a), `Rarity == Normal` (CAP-04), jewellery `Hide` (CAP-05).

**tests/cli_generate.rs** — two new tests added, old Phase 6 pre-edit baseline test removed:

| Change | Description |
|--------|-------------|
| `parity_ingame` added | Compares `generate --early-game` policy blocks vs `parity-ingame.filter` policy blocks; semantic, order/comment-insensitive; excludes pricer section at `# --- Currency ---` |
| `cap_is_rule_driven` added | Mutates CAP-01/02 `areaLevel.max` 75→60 in a temp config; asserts `AreaLevel <= 60` present and `AreaLevel <= 75` absent in output |
| `parity_policy_blocks_match_oracle` removed | Phase 6 pre-edit oracle permanently mismatched after 7 CAP edits; replaced by `parity_ingame`; left as comment explaining rationale |

Both tests pass: `parity_ingame` exits 0 (zero non-reorder policy-block differences), `cap_is_rule_driven` exits 0.

### Task 2: Re-baseline seed-universal.filter + insta snapshot (commit 161acd9)

Re-generated `tests/fixtures/seed-universal.filter` from the seed inputs (config.json + seed-prices.json + currency.json + tiers.json). New block counts: **86 Show (+3), 3 Hide (+1)**.

Diff is confined to the **policy section** (4 hunks, all above `# --- Currency ---`):
- CAP-01: build weapons `Rarity >= Magic` + `AreaLevel <= 75`
- CAP-02: build armour pure-ES `AreaLevel <= 75`
- CAP-03a/b: flask/charm `Rarity >= Magic` + fallback `Rarity <= Magic`
- CAP-04: `bands 78+: normal charms` block added
- CAP-05: jewellery fallback Hide block added
- CAP-06: two unidentified high-tier Show blocks added

Pricer section (`# --- Currency ---` onward) is **unchanged** — confirmed by git diff.

`tests/snapshots/cli_generate__generate_snapshot.snap` updated via `INSTA_UPDATE=always cargo test`.

`generate_block_counts` assertion (86/3) was already correct in `src/emitter.rs` from Plan 07-01.

## Parity Result

`parity_ingame` test passes with **zero non-reorder differences** between `generate --early-game` policy output and `tests/fixtures/parity-ingame.filter` policy blocks.

**Known excluded differences (currency/pricer section drift):**
- `Vaal Infuser` absent from `foo.filter` (game item removed in new version)
- `Tablets` category absent from `foo.filter` (game content change)
These are pricer DATA drift — intentionally excluded from the parity comparison by the `# --- Currency ---` cut.

## Test Results

Full `cargo test` green: **125 lib tests + 12 CLI integration tests + all other integration tests pass** (0 failures).

CLI integration test count: 12 (was 11; removed `parity_policy_blocks_match_oracle`, added `parity_ingame` + `cap_is_rule_driven`).

## Deviations from Plan

### Auto-fixed Issues

None.

### Decision Deviations

**1. [Rule 4 resolved inline] parity_policy_blocks_match_oracle removal vs leave-in-place**

The PLAN.md Task 1 text says "Do not modify the existing Phase 6 `parity_policy_blocks_match_oracle` test". The critical notes (which take precedence) say to repoint or remove it to avoid a permanent false failure. Decision: **removed** (replaced by `parity_ingame`). The old pre-edit baseline fixture (`parity-baseline-earlygame.filter`) is kept as a historical artifact — unused by any test. This is documented by a comment at the removal site in `cli_generate.rs`.

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, or schema changes introduced.

## Self-Check: PASSED

- tests/fixtures/parity-ingame.filter exists: FOUND
- parity-ingame.filter contains "UnidentifiedItemTier >= 4": FOUND
- parity_ingame test function exists in cli_generate.rs: FOUND
- cap_is_rule_driven test function exists in cli_generate.rs: FOUND
- parity_policy_blocks_match_oracle removed (comment only): CONFIRMED
- Commit 980ceb0 exists: FOUND (git log)
- Commit 161acd9 exists: FOUND (git log)
- cargo test --test cli_generate: 12 passed, 0 failed
- cargo test (full suite): all passed, 0 failed
- seed-universal.filter pricer section unchanged: CONFIRMED (git diff shows 4 policy hunks only)

## Checkpoint Status

Tasks 1 and 2 complete. Task 3 (`checkpoint:human-verify`) awaits author review.

**Parity verdict for checkpoint:** `parity_ingame` passes with ZERO non-reorder differences between `generate --early-game` policy output and `tests/fixtures/parity-ingame.filter`. The only remaining differences (Vaal Infuser, Tablets absent in foo.filter) are in the pricer/currency section — intentionally excluded from the parity comparison.
