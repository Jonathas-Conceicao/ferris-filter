---
phase: 18-filter-as-store-economy-section
plan: 02
subsystem: emitter-compile
tags: [emitter, compile, economy, refactor]
requires: []
provides:
  - "emitter::generate_economy(&StyleSet, &PricedData)"
  - "compile::{EconomyInputs, load_economy, compile_economy}"
affects: [18-03, 18-04, 18-05]
key-files:
  created: []
  modified: [src/emitter.rs, src/compile.rs]
key-decisions:
  - "Economy emitters take &StyleSet; generate_as passes &config.styles, so generate bytes are unchanged"
  - "load_priced holds the single copy of the price/tier source picker, shared by load_inputs and load_economy"
  - "load_economy returns PriceSource::Absent as a value; the refusal is the caller's job (18-04, D-17)"
requirements-completed: []
duration: 15min
completed: 2026-10-12
---

# Phase 18 Plan 02: Economy rendering split Summary

The economy body can now be rendered from a `StyleSet` plus priced data alone (`emitter::generate_economy`, `compile::load_economy`/`compile_economy`), with `generate` byte-identical.

## Tasks
1. `emitter::generate_economy` over `&StyleSet`, commit 351b2c1. Four new unit tests: parity with the tail of `generate` (seed data and with a unique block present), banner-only output for empty priced data, and a typed `GenerateError::Styles` for an invalid `StyleSet`.
2. `compile::load_economy` / `compile_economy` / `EconomyInputs`, with `load_inputs` recomposed from a private `load_priced`, commit 8afb6ff. Six new unit tests covering Live, Fallback, Absent, forced-fallback-missing, malformed styles.json, and compile parity.

## Verification
Full `cargo test`: 492 lib tests plus all integration suites pass, 0 failures. `git diff --exit-code tests/fixtures tests/snapshots` is clean (artifact_guard and generate_snapshot pass).

## Requirements
ECON-01 and ECON-03 are not marked complete: they only complete once 18-03 to 18-05 wire the section into Apply.

## Deviations from Plan
None. Unit parity covers the unique-block path too, since `UniqueBlock` fields are public.

## Known Stubs
None.

## Self-Check: PASSED
