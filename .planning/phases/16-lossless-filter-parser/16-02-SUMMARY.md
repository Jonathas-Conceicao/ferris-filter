---
phase: 16-lossless-filter-parser
plan: 02
subsystem: filter-parser
tags: [parser, keyword-table, tokenizer]
requires: ["16-01"]
provides:
  - "filter::keywords::{Kind, Ops, ValueShape, Keyword, KEYWORDS, lookup, COLOURS, SHAPES, RARITIES}"
  - "filter::tokenize::{Operator, TokenizedLine, TokenError, tokenize, validate_shape}"
affects: [16-03]
tech-stack:
  added: []
  patterns: ["byte-range tokens over borrowed text; never rebuild text"]
key-files:
  created:
    - src/filter/keywords.rs
    - src/filter/tokenize.rs
  modified:
    - src/filter/mod.rs
decisions:
  - "Keyword table has exactly the 80 RESEARCH entries; Ops is None|Compare only"
  - "Lenient validation: no range or name checks"
metrics:
  tasks: 2
  completed: 2026-10-12
requirements-completed: [PARSE-02, PARSE-01]
---

# Phase 16 Plan 02: Keyword Table and Tokenizer Summary

One static 80-entry keyword table (block headers, Continue, Import, all documented conditions and actions) plus a quote-aware byte-range tokenizer and lenient shape validator, std only.

## Tasks

| Task | Commit | Notes |
|------|--------|-------|
| 1. Keyword table | ccaf993 | 6 tests; fixture keyword set is a subset of the table |
| 2. Tokenizer + validator | e70e7ce | 14 tests incl. multibyte no-panic |

## Test results

- `cargo test --lib filter::` 27 passed (6 keywords, 14 tokenize, 7 from plan 01)
- Full `cargo test` green: 355 lib tests, filter_roundtrip 4, cli_generate 13, artifact_guard/generate_snapshot pass
- Built with `CARGO_TARGET_DIR` in a scratch dir (ff.exe lock).

## Deviations from Plan

- The plan asked to re-fetch the official grammar page and add missing keywords. No web fetch tool was available, so per the plan's fallback the RESEARCH list (fetched 2026-10-12) was kept unchanged; no additions.
- No other deviations. Cargo.toml untouched; no unwrap/expect before the test module in tokenize.rs.

## Decisions Made

- Comment span starts at the whitespace run preceding the first unquoted `#` (or at the `#` when glued).
- `MinimapIcon -1` may be followed by up to a colour and shape; each, if present, is validated.
- `Import` is `Ops::None`, so an operator on it is a shape error.

## Known Stubs

None.

## Self-Check: PASSED
