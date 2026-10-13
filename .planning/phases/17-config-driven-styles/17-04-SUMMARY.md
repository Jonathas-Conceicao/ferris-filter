---
phase: 17-config-driven-styles
plan: 04
subsystem: styles
tags: [styles, cli, docs, e2e]

requires: [17-03]
provides:
  - "--styles <path> on generate and apply (OD-2)"
  - end-to-end CLI proof of phase success criteria 1-3
  - CLAUDE.md and CHANGELOG documentation of styles.json
affects: []

tech-stack:
  added: []
  patterns: [mutate-committed-file-and-assert-changed e2e tests]

key-files:
  modified: [src/cli.rs, src/run.rs, tests/cli_generate.rs, tests/dynamic_filter.rs, tests/cli_errors.rs, CLAUDE.md, CHANGELOG.md]

key-decisions:
  - "OD-2: --styles mirrors --tiers, prints a stderr note, default stays styles.json"

requirements-completed: [STYLE-01, STYLE-02, STYLE-04]

duration: 20min
completed: 2026-10-12
---

# Phase 17 Plan 04: --styles flag, e2e proof, docs Summary

**`generate` and `apply` accept `--styles <path>`; adding a style to styles.json and using it from config.json is proven through the real binary, every STYLE-02 failure mode is tested end to end, and CLAUDE.md/CHANGELOG describe styles.json and the breaking move.**

## Accomplishments
- `--styles` on `Generate` and `Apply` (cli.rs), wired through `paths.with_styles` in run.rs with a stderr note.
- cli_generate: help test, `new_style_in_styles_json_is_emitted_with_no_code_change` (custom `craftable` style emits `SetTextColor 1 2 3 255` / `SetFontSize 31`), `generate_styles_flag_reads_alternate_path` (output equals default run; fails naming styles.json without the flag). dynamic_filter: `apply_styles_flag_reads_alternate_path`.
- cli_errors: six tests (unknown style in enabled rule fails, in disabled rule passes, missing required style, duplicate name, bad colour, inline styles section). Every mutation asserts the text changed.
- CLAUDE.md: new `styles.json` layout row, config.json/dynamic_base.json/apply rows updated, Named styles section rewritten, `exceptional` row added. CHANGELOG [Unreleased]: Added entries plus a Changed section with the BREAKING note.
- Full `cargo test` green (452 unit + all integration suites); `tests/fixtures/seed-universal.filter` has no diff and its last commit predates Phase 17. Clippy reports warnings only, none introduced by this plan's changes.

## Task Commits
1. Task 1: --styles flag + e2e tests - bf9d1e3
2. Task 2: load-time error tests - fea451d
3. Task 3: docs - 7c7e4ef

## Deviations from Plan
- The plan's config mutation target `"style": "Recycle"` also occurs in `alwaysShow.recycle`; tests use `"style": "Recycle",` (trailing comma), which is unique to the rule. No source messages needed changing.
- The CLAUDE.md overlay note names no fixed style list, so no picker sentence was changed there (the picker order is stated in the styles.json row and Named styles section).

## Known Stubs
None.

## Self-Check: PASSED
