---
phase: 18-filter-as-store-economy-section
plan: 04
subsystem: dynamic
tags: [dynamic, filter-as-store, apply, headless]
requires: [18-01, 18-02, 18-03]
provides:
  - "dynamic::{LoadedFilter, ApplyOp, ApplyReport, apply_text, apply_file, economy_body, default_filter_path}"
  - "new DynamicError variants: FilterUnreadable, NotUtf8, FilterNotFound, NoEconomySection, Section, EconomyUnavailable, Write"
affects: [18-05, 18-06, 18-07]
key-files:
  modified: [src/dynamic.rs, tests/filter_roundtrip.rs]
key-decisions:
  - "apply_text returns bare SectionError (no path param); apply_file attaches the path and maps NoSection to NoEconomySection"
  - "Legacy JSON-store API left in place for run.rs/overlay until 18-07"
requirements-completed: []
duration: 15min
completed: 2026-10-12
---

# Phase 18 Plan 04: Filter-as-store apply Summary

Headless library API in `crate::dynamic`: `LoadedFilter::load` (absent -> empty, v0.4 migrated in memory, exact original bytes kept as guard baseline), `economy_body` (refuses `PriceSource::Absent`), `apply_text` (Refresh/AddEconomy/RemoveEconomy), and `apply_file` (the single writer, via `safe_write::write_guarded`, baseline updated after success so a second apply is `Unchanged`).

## Tasks
1. LoadedFilter, ApplyOp, apply_text, economy_body, new error variants.
2. apply_file + ApplyReport; `corpus_generated_dynamic` re-targeted to `economy_body` + `apply_text("", AddEconomy)`. Tasks 1 and 2 share `src/dynamic.rs`, so both landed in one feat commit; the roundtrip test is its own commit.

## Verification
Full `cargo test`: 533 lib tests plus all integration suites pass, 0 failures. 13 new `dynamic::` tests (18 total). Fixtures and snapshots unchanged. Tests use tempdirs only.

## Requirements
STORE-01/03, ECON-03/04, EDIT-09/10 are not marked complete: the CLI (18-05) and overlay (18-06) still have to expose them end-to-end.

## Deviations from Plan
None of substance. The "remove economy" test asserts the `.ffbak` exists (it holds the original from the earlier AddEconomy write, as backups are one-time) rather than pre-removal bytes.

## Known Stubs
None.

## Self-Check: PASSED
