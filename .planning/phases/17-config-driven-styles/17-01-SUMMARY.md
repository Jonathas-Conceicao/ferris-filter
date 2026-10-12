---
phase: 17-config-driven-styles
plan: 01
subsystem: styles
tags: [styles, serde, indexmap, loader]

requires: []
provides:
  - Committed repo-root styles.json (today's 12 styles, in order)
  - StyleSet ordered map with validate(), StyleError, duplicate-name rejection
  - loader::load_styles and LoadError::Styles
  - Every temp-dir test harness copies styles.json beside config.json
affects: [17-02, 17-03]

tech-stack:
  added: [indexmap 2 (serde feature)]
  patterns: [transparent map newtype with custom Deserialize for duplicate detection]

key-files:
  created: [styles.json]
  modified: [Cargo.toml, src/models/config.rs, src/loader.rs, src/compile.rs, src/dynamic.rs, src/overlay/store.rs, tests/cli_generate.rs, tests/cli_errors.rs, tests/dynamic_filter.rs, tests/rule_editor.rs, tests/filter_roundtrip.rs, tests/sync_prices.rs]

key-decisions:
  - "StyleSet::REQUIRED = S..F, unchanged, hidden; kite/Recycle/exceptional optional"
  - "Names differing only by case from a required style are rejected (PITFALLS 23)"
  - "sound.custom containing a quote or control character is rejected (filter-line injection)"

requirements-completed: [STYLE-01, STYLE-02, STYLE-04]

duration: 20min
completed: 2026-10-12
---

# Phase 17 Plan 01: Styles data model Summary

**styles.json plus an insertion-ordered, validated StyleSet and load_styles, proven equal to the hardcoded Styles struct before anything is deleted.**

## Accomplishments
- styles.json holds exactly the 12 current styles in order; a unit test proves it equals the old `Styles` from both config.json and dynamic_base.json (names, order, values).
- `StyleSet` rejects duplicate names at deserialize, missing required styles (naming all), case-shadowing names, and unsafe `sound.custom`.
- `loader::load_styles` always validates; parse errors carry paths starting at the style name (`S.font`).
- All harnesses that copy config.json (integration tests and in-source temp dirs) now also copy styles.json, so Plan 03 can make it mandatory.
- Consumers unchanged: artifact_guard/generate_snapshot pass, seed-universal.filter has no diff; full `cargo test` green.

## Task Commits
1. Task 1: styles.json, StyleSet, StyleError, equality test - 4fd2695
2. Task 2: load_styles and LoadError::Styles - a185514
3. Task 3: styles.json in test harnesses - d3b8444

## Deviations from Plan
- Cargo.lock is git-ignored in this repo, so it was not staged (the plan listed it); the lock updates locally only.
- Tasks 1 and 2 were committed as a single GREEN commit each rather than separate RED/GREEN commits, because tests cannot compile without the types.

## Known Stubs
None.

## Self-Check: PASSED
