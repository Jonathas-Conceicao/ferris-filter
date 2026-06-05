---
phase: 06-emitter-rewrite-migration-parity
plan: "02"
subsystem: emitter-parity
tags: [rust, parity-test, re-baseline, semantic-diff]
dependency_graph:
  requires: [06-01]
  provides: [semantic-parity-test, rebaselined-seed-fixture, pricer-guard]
  affects: [tests/cli_generate.rs, tests/fixtures/seed-universal.filter, tests/snapshots/cli_generate__generate_snapshot.snap]
tech_stack:
  added: []
  patterns: [split_blocks, normalize_block, policy_blocks, BTreeSet order-insensitive comparison]
key_files:
  created: []
  modified:
    - tests/cli_generate.rs
    - tests/fixtures/seed-universal.filter
    - tests/snapshots/cli_generate__generate_snapshot.snap
decisions:
  - Cut marker for policy/pricer boundary: "# --- Currency ---" (confirmed present in both oracle and generated output)
  - Block normalization: Show/Hide keyword kept first, remaining condition/action lines sorted alphabetically for within-block order insensitivity
  - parity_policy_blocks_match_oracle: BTreeSet comparison means cross-block order is also insensitive
  - pricer_section_present_and_unchanged: asserts Class == "Stackable Currency" present and policy section has no Class == (bare Class only)
metrics:
  duration_seconds: ~180
  completed: "2026-06-05T12:00:00Z"
  tasks: 3
  files: 3
---

# Phase 6 Plan 02: Parity Test + Re-baseline Summary

Semantic parity test (MIG-02) confirms zero non-reorder differences between new rule-driven generator and frozen oracle; seed-universal.filter re-baselined (MIG-03); pricer section confirmed byte-identical (MIG-04). Full suite: 152 tests, 0 failures.

## What Was Built

### Task 1: Semantic parity test (MIG-02) + pricer guard (MIG-04)

Added to `tests/cli_generate.rs`:

- `generate_early_game()` — hermetic helper copying `config.json`, `currency.json`, `prices-fallback.json`, `tiers-fallback.json` into a temp dir and running `ferrisfilter generate --early-game --output <out>`. Returns CRLF-normalized output.
- `split_blocks(filter: &str) -> Vec<Vec<String>>` — splits filter text into blocks (Show/Hide to next blank line). Each block collected as keyword + trimmed non-comment lines. Comment lines and block header comment text are dropped.
- `normalize_block(block: &[String]) -> Vec<String>` — returns the block with Show/Hide keyword first, then all other lines sorted. Makes within-block condition/action order insensitive.
- `policy_blocks(filter: &str) -> BTreeSet<Vec<String>>` — truncates at `# --- Currency ---`, splits, normalizes, collects into BTreeSet (also makes cross-block order insensitive).
- `parity_policy_blocks_match_oracle` — compares `generate_early_game()` policy set against frozen oracle `parity-baseline-earlygame.filter` policy set. **Zero non-reorder differences found.** Test passes.
- `pricer_section_present_and_unchanged` — asserts `# --- Currency ---` marker present, Currency S-F blocks present, `Class == "Stackable Currency"` used in pricer blocks, `BaseType ==` present, and policy section uses no `Class ==` (bare Class only).

Commit: `fcdd05f`

### Task 2: Re-baseline seed-universal.filter + insta snapshot (MIG-03)

Re-baselined `tests/fixtures/seed-universal.filter` using the exact hermetic procedure:
1. `cargo build` (binary already up to date)
2. Created `target/rebaseline/` with `config.json`, `currency.json`, `tiers.json`, `seed-prices.json` → `prices.json`
3. Ran `ferrisfilter.exe generate --output out.filter` from that dir
4. Copied output to `tests/fixtures/seed-universal.filter`

Then:
- `artifact_guard`: PASSED (byte-identical, CRLF-normalized)
- `INSTA_UPDATE=always cargo test generate_snapshot`: snapshot updated and passes
- `generate_block_counts`: PASSED — still 83 Show + 2 Hide (unchanged)
- `generate_is_deterministic`: PASSED

Commit: `91e512d`

### Task 3: Confirm pricer currency/unique sections unchanged (MIG-04)

- `currency_retiering_present`: PASSED (Divine in A, Exalted in D, Chaos in E)
- `pricer_section_present_and_unchanged`: PASSED (new test, MIG-04)
- `git diff` on `seed-universal.filter` currency section: **zero changes from `# --- Currency ---` onward** — the entire diff is confined to the policy section (13 lines of block header/tag comments removed, condition order normalized)

## Parity Result: ZERO Non-Reorder Differences

`parity_policy_blocks_match_oracle` passes with empty diff sets. The only differences between old and new generator output are:

1. **Section header comments removed** — `#====...gear...====` banners gone (comment-only, not semantic)
2. **Block comment tags stripped** — `[areaLevelBands "1-5".allArmour -> styles.unchanged]` suffixes gone from block headers (comment-only)
3. **Within-block condition order normalized** — `Quality/Sockets` order, `BaseEvasion/BaseEnergyShield` order differ from oracle but are order-insensitive (same set of conditions)
4. **`flasks/charms: quality always` block** — oracle has `Quality >= 1` then `Class ...`; new generator has `Class ...` then `Quality >= 1` (emit_rules outputs class before quality)

All four are pure reordering/comments. The parity test normalizes them all. The condition/action sets are identical.

## Currency Section Git Diff

`git diff --no-color tests/fixtures/seed-universal.filter | grep Currency` shows only the file header comment line `# Currency base lists are curated data kept outside the model` (line 7, unchanged). The pricer section (`# --- Currency ---` onwards, lines 122+) is **byte-identical** to the prior committed fixture. No pricer output was altered by the emitter rewrite.

## Block Counts (Confirmed Unchanged)

| Section | Show | Hide |
|---------|------|------|
| Policy (rules 1-13) | 11 | 2 |
| Pricer currency/categories | 72 | 0 |
| **Total** | **83** | **2** |

`generate_block_counts` asserts 83/2 — PASSED.

## Deviations from Plan

None — plan executed exactly as written. All tests passed on first attempt.

## Self-Check: PASSED

Files confirmed present:
- `tests/cli_generate.rs` — contains `parity_policy_blocks_match_oracle`, `pricer_section_present_and_unchanged`, `generate_early_game`, `policy_blocks`, `split_blocks`, `normalize_block`
- `tests/fixtures/seed-universal.filter` — re-baselined
- `tests/snapshots/cli_generate__generate_snapshot.snap` — updated

Commits verified:
- `fcdd05f` — Task 1 (parity test + pricer guard)
- `91e512d` — Task 2 (re-baseline)

Full test suite: `cargo test` → 152 passed, 0 failed.
