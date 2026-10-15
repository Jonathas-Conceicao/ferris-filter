---
phase: 16-lossless-filter-parser
plan: 03
subsystem: filter-parser
tags: [parser, blocks, disabled-blocks, metadata]
requires: ["16-01", "16-02"]
provides:
  - "filter::{Block, BlockId, BlockKeyword, BodyLine, DisabledState, Item}"
  - "Document::{items, blocks, block, block_meta, meta_get}"
  - "Line::ff_marker, filter::meta::parse_ff_line"
affects: [18, 19, 20]
tech-stack:
  added: []
  patterns: ["read-only index of line ranges over raw lines; parse never writes lines"]
key-files:
  created:
    - src/filter/block.rs
    - src/filter/parse.rs
    - src/filter/disabled.rs
    - src/filter/meta.rs
  modified:
    - src/filter/mod.rs
    - src/filter/lines.rs
    - src/filter/keywords.rs
    - src/filter/tokenize.rs
    - tests/filter_roundtrip.rs
decisions:
  - "Metadata accessors live on Document (block_meta(id), meta_get(id, key)), not on Block"
  - "Disabled blocks: reasons stored as Warning on DisabledState::Invalid, mirrored as Info document diagnostics (never Error)"
  - "Malformed line in a disabled body only counts if its first word is a known condition/action/modifier; otherwise it is prose"
metrics:
  tasks: 2
  completed: 2026-10-12
requirements-completed: [PARSE-01, PARSE-02, PARSE-03]
---

# Phase 16 Plan 03: Block Index, Disabled Blocks and Corpus Gate Summary

Read-only block index over the raw lines (stable BlockId, attached comments, #ff: metadata, Import/opaque items, valid/invalid disabled blocks, line-tied diagnostics) with a corpus gate proving zero Error diagnostics and byte identity on every real filter.

## Tasks

| Task | Commit | Notes |
|------|--------|-------|
| 1. Block model, live grouping, leading comments, #ff:, diagnostics | 790a9d5 | 15 parse/meta tests |
| 2. Disabled blocks and corpus tests | 8b66b7c | 10 disabled tests, 5 corpus tests |

## Test results

- Full `cargo test`: 385 lib tests pass; filter_roundtrip 9 (4 roundtrip + 5 corpus); artifact_guard, generate_snapshot, cli_generate (13), dynamic_filter and the rest green.
- Corpus: FilterBlade 447 enabled Show / 13 Hide / 0 Minimal / 40 Continue lines, disabled-block count equals the independently computed `^#(Show|Hide|Minimal)` count, no block has a `name`; 10 `reference-old-filters/*.filter` files checked locally with zero Errors.
- Built with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`.

## Deviations from Plan

**1. [Rule 1 - Bug] Keyword table too strict for DisableDropSound**
- **Found during:** Task 2 corpus gate (11 Error diagnostics in the FilterBlade fixture)
- **Issue:** Real filters write `DisableDropSound True`; plan 02 table was `NoValue`.
- **Fix:** New `ValueShape::OptionalBool` (zero values or True/False) for DisableDropSound, EnableDropSound and the two IfAlertSound variants; tokenizer unit test added.
- **Files:** src/filter/keywords.rs, src/filter/tokenize.rs

**2. [Rule 3] Gate hygiene**
- The plan's grep gate bans `.lines()`; a plan 02 test used `src.lines()` and a plan 01 test used `d.lines()`. Changed to `split('\n')` / the private field. Test-only helpers `line_at/line_text/line_count` (cfg(test)) were added on Document for the new tests.

## Decisions Made

- Metadata API on `Document` (`block_meta`, `meta_get`), first matching `#ff:` key wins.
- `next_id` is kept on Document (marked `allow(dead_code)`) for block insertion in later phases.
- Disabled-body scanning: a line is part of the block if it is a known condition/action/modifier line, or malformed but starting with such a word (makes the block Invalid); comments/blank `#` lines are interior; owned ends at the last keyword line.
- Import path range is relative to the line text, quotes excluded.
- Disabled body `tokens` ranges are relative to the text after the line's first `#`.

## Known Stubs

None.

## Threat Flags

None. No fs access in src/filter/; Import path stored as a range only.

## Self-Check: PASSED
