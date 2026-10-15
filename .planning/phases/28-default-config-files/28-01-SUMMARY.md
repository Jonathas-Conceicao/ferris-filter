---
phase: 28-default-config-files
plan: 01
subsystem: config / tests
tags: [defaults, styles, tiers, tests]
requires: []
provides:
  - styles-default.json (committed default styles, HEAD bytes of styles.json)
  - tiers-default.json (committed default tier cutoffs, HEAD bytes of tiers.json)
  - test suite independent of the author's live styles.json / tiers.json
affects: [28-02, 28-03, 28-04]
tech-stack:
  added: []
  patterns: ["temp-dir rigs copy (default source -> live name) pairs"]
key-files:
  created: [styles-default.json, tiers-default.json]
  modified: [src/compile.rs, src/dynamic.rs, src/emitter.rs, src/loader.rs, src/pricer.rs, src/styled.rs, src/models/config.rs, src/models/tiers.rs, src/overlay/app.rs, src/overlay/apply_all.rs, src/overlay/body.rs, src/overlay/classes.rs, src/overlay/config_state.rs, src/overlay/config_view.rs, src/overlay/rule_list.rs, src/overlay/rules.rs, tests/cli_errors.rs, tests/cli_generate.rs, tests/dynamic_filter.rs, tests/filter_roundtrip.rs, tests/sync_prices.rs]
decisions:
  - "Test rigs keep the live names styles.json / tiers.json in their temp roots, sourced from the -default files"
  - "generate_in_repo_root_succeeds passes --styles styles-default.json --tiers tiers-default.json"
metrics:
  duration: ~15 min
  completed: 2026-10-15
requirements: [DEF-01, DEF-05]
---

# Phase 28 Plan 01: Default files and test switch Summary

Committed `styles-default.json` / `tiers-default.json` (byte-identical to the HEAD blobs of
`styles.json` / `tiers.json`, blob hashes `5b690b4` / `643f3e9` verified equal) and moved every
unit and integration test off the repo-root live files onto the defaults, with no production code
change.

## What was done

- **DEF-01:** defaults created with `git show HEAD:<file> > <file>-default.json`; never copied from
  the working tree.
- **DEF-05 (src unit tests):** `include_str!` / `concat!` / `repo_file(...)` / `format!("{dir}/...")`
  reads switched to the `-default` names; the four `for f in ["styles.json", "tiers.json"]` rigs
  (app.rs `config_root`, apply_all.rs `Fixture::new`, config_state.rs / config_view.rs `temp_root`)
  now copy `(default source, live destination)` pairs; app.rs `styles_root` likewise.
  `StyleSet::committed_for_tests` (`#[cfg(test)]`) reads `styles-default.json`.
- **DEF-05 (integration tests):** `generate_on_seed`, `generate_early_game`, the config-flag and
  cap loops, `new_style_in_styles_json_is_emitted_with_no_code_change`,
  `generate_styles_flag_reads_alternate_path`, cli_errors `valid_tiers`/`valid_styles`,
  dynamic_filter `workspace()`, filter_roundtrip `corpus_generated_dynamic`, sync_prices
  generate-loop. `generate_in_repo_root_succeeds` now passes explicit default `--styles`/`--tiers`
  and its stale plan-01-02/01-03 skip comment became a plain assertion that config.json exists.
  No other repo-root `current_dir` spawn exists in tests/.

## Verification

- `cargo test`: lib 1014 passed; integration 91 passed (12+4+16+15+23+8+4+8+1); 0 failed —
  run with the author's differing working-tree `styles.json` / `tiers.json` present.
  `artifact_guard` and `generate_snapshot` pass.
- `cargo fmt --check` clean.
- `git diff --exit-code -- tests/fixtures tests/snapshots`: clean (seed-universal.filter byte-identical).
- `styles.json` / `tiers.json` still ` M`, unstaged; sha256 equal to the parked copies.
- Every src hunk lies inside `#[cfg(test)]` code (checked per file).

## Deviations from Plan

None - plan executed exactly as written. (Both tasks land in the single commit the plan's Task 2
specifies.)

## Commits

- `d9861c8` `test(28-01): add styles-default.json/tiers-default.json and read them in every test`

## Self-Check: PASSED

- styles-default.json, tiers-default.json exist; commit present on the branch.
