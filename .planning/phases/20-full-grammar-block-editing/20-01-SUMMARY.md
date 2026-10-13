---
phase: 20-full-grammar-block-editing
plan: 01
subsystem: filter-document
tags: [rust, lossless-edit, filter, parse-04]

requires:
  - phase: 16-filter-document
    provides: lossless filter::Document with block-scoped edit API
provides:
  - LinePatch + Document::patch_line / insert_body_line / remove_body_line / splice_body
  - commit_owned shared validation helper (new-diagnostics-only)
  - EditError::NotABodyLine / EmptyValue / CannotEditInvalidDisabled
affects: [20-04, 20-05, overlay rule editor]

tech-stack:
  added: []
  patterns:
    - "Candidate Vec<Line> keeps each untouched line's own Eol; only the touched line is rebuilt"
    - "Validation compares candidate diagnostics to the block's existing ones as a multiset"

key-files:
  created: [src/filter/lineedit.rs]
  modified: [src/filter/edit.rs, src/filter/mod.rs, src/filter/structure.rs, tests/filter_roundtrip.rs]

key-decisions:
  - "Whole patch refuses only new Errors; every other op refuses new Errors and Warnings"
  - "set_continue/set_block_keyword stay lenient on invalid disabled blocks (previous behaviour) via splice_impl(lenient)"
  - "Whitespace-only text counts as EmptyValue"
  - "A disabled-block edit that leaves it non-enableable returns InvalidBlock with the disabled reasons"

requirements-completed: [EDIT-01, EDIT-02]

duration: ~25min
completed: 2026-10-13
---

# Phase 20 Plan 01: Single-line body edit API Summary

**Lossless per-line operator/value patching and body-line insert/remove/splice on `filter::Document`, validated against only the diagnostics the edit introduces.**

## Accomplishments
- `commit_owned` in edit.rs: isolated candidate check, multiset diff vs. current block diagnostics; `replace_block_lines` now goes through it (pre-existing warnings no longer lock a block). `set_enabled` unchanged and strict.
- `src/filter/lineedit.rs`: `LinePatch` (Operator/Values/Both/Whole), `patch_line`, `insert_body_line`, `remove_body_line`, `splice_body`; prefix-aware for valid disabled blocks, per-line EOLs and no-final-newline preserved.
- `set_block_keyword` and `set_continue` no longer re-terminate every line (mixed EOLs survive).
- FilterBlade sweeps: >1000 no-op value patches and operator patches leave the fixture byte-identical; bare `Class "x"` never becomes `==`.

## Task Commits
1. Task 1 - shared commit helper: 5f7c4ea
2. Task 2 - LinePatch and line ops: 4276f22
3. Task 3 - fixture sweeps: d660bf9

## Deviations from Plan
None of substance. Additions: `commit_owned` returns `InvalidBlock` (with the disabled reasons) rather than the generic `NotOneBlock` when an edit would leave a valid disabled block non-enableable.

## Issues Encountered
None. `cargo fmt --check` clean, `cargo build --bins` warning-free, full `cargo test` green, fixtures/snapshots unchanged.

## Self-Check: PASSED
