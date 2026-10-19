---
phase: 38-built-in-styles-strictness-reset
plan: 01
subsystem: styles-defaults
tags: [styles, defaults, test-fixtures]
requires: []
provides:
  - "styles-default.json = the nine reworked built-ins (S-F, unchanged, hidden)"
  - "tests/fixtures/seed-styles.json = frozen pre-rework default (the generate spec)"
affects: [38-02, 38-03, 38-04, 38-05, 38-06]
tech-stack:
  added: []
  patterns: ["frozen fixture for old-content tests; STYLES_DEFAULT.text for shipped-default assertions"]
key-files:
  created: [tests/fixtures/seed-styles.json]
  modified:
    - styles-default.json
    - src/defaults.rs
    - src/compile.rs
    - src/overlay/config_state.rs
    - tests/dynamic_filter.rs
    - tests/ui/test_ff.filter
    - tests/ui/README.md
key-decisions:
  - "Fixture named tests/fixtures/seed-styles.json (matches seed-prices/seed-universal)"
  - "Fixture-only readers: generate tests, committed_for_tests and every lib test that loads config.json against styles; apply-path tests that assert the embedded default use the shipped file"
metrics:
  tasks: 3
  completed: 2026-10-18
---

# Phase 38 Plan 01: Built-in styles swap Summary

The shipped `styles-default.json` is now exactly the author's nine built-ins (no custom styles, no custom sound file), while today's file is frozen as `tests/fixtures/seed-styles.json` so `generate`, `artifact_guard` and `seed-universal.filter` are byte-identical.

## Tasks

| Task | Commit | Notes |
|------|--------|-------|
| 1 Freeze fixture, repoint lib readers | f2011a7 | `committed_for_tests` and all old-content lib readers use the fixture |
| 2 Repoint integration readers | 9ec0c27 | cli_generate (incl. `--styles tests/fixtures/seed-styles.json` run), cli_errors, dynamic_filter, filter_roundtrip, sync_prices |
| 3 Swap defaults + pin with tests | 7427f3d | four new `defaults::tests` (nine built-ins, no custom sound, snapshot values inline, render round-trip); UI sample case 3 moved from `kite` to `C` |

## Verification

`cargo test` (whole workspace default members): lib 1506 passed, all integration binaries pass, including cli_generate, cli_errors, dynamic_filter, filter_roundtrip, sync_prices, isolation_guard. `config.json`, `tiers-default.json`, `seed-universal.filter` unmodified.

## Deviations from Plan

Stragglers the swap exposed (fixed in this plan, Rule 1/3):
- `compile::tests::economy_from_built_ins_equals_economy_from_the_committed_files`: compares the embedded default with the shipped file, so it reads the real `styles-default.json` (reverted from the fixture).
- `overlay::config_state` tests `load_falls_back_to_defaults_and_creates_nothing` and `default_files_in_the_folder_are_ignored`: expect 9 styles (embedded default); tests on fixture-backed live files keep 12.
- `tests/dynamic_filter.rs`: the `workspace()` live copy is the shipped file (apply-path default comparisons); `economy_section_matches_generate` copies the fixture over it (generate is not reworked); the `"font": 37` edit in `app_root_from_ferrisfilter_home_wins_over_cwd` matches the compact `"font":37` format.
- `cargo fmt` reflowed the lines I touched (and `tests/isolation_guard.rs`, which I reverted as unrelated).

None required architectural changes.

## Decisions

- `generate` on a checkout with no live `styles.json` now reports unknown styles (Recycle/kite/exceptional) — the CONTEXT-documented consequence; CLAUDE.md update belongs to the docs plan of this phase.

## For the Release Review

- Fresh installs get only S-F/unchanged/hidden; `generate` without a live `styles.json` fails with the unknown-style error until the generate rework.

## Self-Check: PASSED

Fixture, new defaults and commits f2011a7, 9ec0c27, 7427f3d exist.
