---
phase: 18-filter-as-store-economy-section
plan: 03
subsystem: filter
tags: [filter, economy-section, markers, migration]
requires: []
provides:
  - "filter::section::{find, has_section, refresh, add, remove, is_v04, migrate_v04, SectionError, SectionSpan}"
  - "tests/fixtures/v04-dynamic.filter"
affects: [18-04, 18-05]
key-files:
  created: [src/filter/section.rs, tests/fixtures/v04-dynamic.filter]
  modified: [src/filter/mod.rs, .gitattributes]
key-decisions:
  - "Markers are found by ff_marker key per line, never by substring; a marker inside any block's owned range is a hard error"
  - "Garbage after the begin marker that parses as a block body (e.g. an unknown keyword right after a rule) is absorbed into that block and rejected as MarkerInsideBlock rather than guessed at"
  - "migrate_v04 names blocks only through Document::set_meta, leaving the header text and the GENERATED header untouched"
requirements-completed: []
duration: 25min
completed: 2026-10-12
---

# Phase 18 Plan 03: Economy section text ops Summary

Pure, tested `filter::section` module: locate/validate the economy markers, refresh only between them, add and remove the section explicitly, and convert the author's v0.4 `dynamic.filter` to the marker layout in memory.

## Tasks
1. Fixture commit (chore for task 2): read-only copy of the game-folder `dynamic.filter` (893 lines, 50770 bytes, LF, sha256 `21b36116f68c36bae7d72293ae3a6188af685604a076786ac1257191b61d15ca`), `-text` in `.gitattributes`.
2. `src/filter/section.rs`: find/refresh/add/remove with typed damage errors plus `is_v04`/`migrate_v04`. 30 unit tests.

## Verification
Full `cargo test`: 522 lib tests plus all integration suites pass, 0 failures. `filter_roundtrip` 18 pass. `seed-universal.filter` and `tests/snapshots` unchanged. Game-folder file was only read.

## Requirements
ECON-01/03/04 and STORE-01 are not marked complete: they only complete once 18-04/18-05 wire these functions into Apply and the overlay.

## Deviations from Plan
- Test inputs use `Show # ...` bodies instead of bare text, because the parser absorbs unknown-keyword lines after a rule into that rule (so such "garbage" between markers is reported as MarkerInsideBlock). This is behavior, not a defect.
- Negative-fixture tests use `include_str!`/`include_bytes!` instead of `std::fs`, so the module has zero `std::fs` uses.
- Fixture committed as its own commit before the module because the module's tests `include_str!` it.

## Known Stubs
None.

## Self-Check: PASSED
