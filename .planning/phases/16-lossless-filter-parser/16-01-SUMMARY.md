---
phase: 16-lossless-filter-parser
plan: 01
subsystem: filter-parser
tags: [parser, lossless, filter-grammar]
requires: []
provides:
  - "ferrisfilter::filter::{parse, Document, Line, Eol, FilterDiagnostic, Severity, LoadError}"
  - "tests/fixtures/parser/filterblade.filter (byte-exact CRLF fixture)"
affects: [16-02, 18, 19, 20]
tech-stack:
  added: []
  patterns: ["raw lines are source of truth; parsed layer is lookup only"]
key-files:
  created:
    - src/filter/mod.rs
    - src/filter/lines.rs
    - src/filter/diag.rs
    - tests/filter_roundtrip.rs
    - tests/fixtures/parser/filterblade.filter
  modified:
    - src/lib.rs
    - .gitattributes
decisions:
  - "Scoped -text rule (tests/fixtures/parser/*.filter) rather than global *.filter -text"
  - "Fixture licence (A5): committed per CONTEXT D-13"
metrics:
  tasks: 2
  completed: 2026-10-12
---

# Phase 16 Plan 01: Lossless Filter Document Foundation Summary

Raw-line `filter::Document` (text + per-line Lf/CrLf/None terminator + BOM flag) whose serializer is byte-identical by construction, with the FilterBlade fixture committed CRLF-intact via a scoped `-text` attribute.

## Tasks

| Task | Commit | Notes |
|------|--------|-------|
| 1. Fixture + .gitattributes | ff9fc06 | `git ls-files --eol`: `i/crlf attr/-text`; 258,094 bytes; seed-universal still `i/lf` |
| 2. Document model + roundtrip tests | 9c74269 | std only, Cargo.toml unchanged |

## Test results

- `cargo test --lib filter::` 8 passed
- `cargo test --test filter_roundtrip` 4 passed (filterblade, seed-universal, synthetic x12 cases, invalid UTF-8)
- Full `cargo test` green (336 lib tests, cli_generate 13, artifact_guard/generate_snapshot pass)
- Run with CARGO_TARGET_DIR in a scratch dir to avoid the ff.exe lock.

## Deviations from Plan

None in behavior. Notes:
- The acceptance check `grep -c $'\r$'` returns 0 under Git Bash (grep strips CR in text mode); CRLF was verified instead via `git ls-files --eol` (`i/crlf`) and byte length 258094.
- The grep gate for `.lines()` matches only the `Document::lines()` accessor inside a test, not `str::lines()`.

## Decisions Made

- Followed the orchestrator's scoped `-text` rule; added after `*.filter text`.
- `Document` fields private so later plans can add block/item fields.

## Known Stubs

None. `Document.diagnostics` is always empty until grammar plans populate it (intentional).

## Self-Check: PASSED
