---
phase: 16-lossless-filter-parser
plan: 04
subsystem: filter-parser
tags: [parser, edit-api, lossless]
requires: ["16-03"]
provides:
  - "filter::EditError"
  - "Document::{set_enabled, replace_block_lines, set_meta}"
affects: [18, 19, 20]
tech-stack:
  added: []
  patterns: ["validate-then-commit edits; full index rebuild with BlockId remap by position"]
key-files:
  created:
    - src/filter/edit.rs
  modified:
    - src/filter/mod.rs
    - tests/filter_roundtrip.rs
    - CHANGELOG.md
decisions:
  - "Edits rebuild the whole block index over the new lines (the plan's accepted alternative) and remap BlockIds by block position; an edit that changes the block count, header position, owned length or enabled state is refused as NotOneBlock before anything is mutated"
  - "Enabling also refuses (CannotEnable) if the uncommented block would carry an Error diagnostic, not only when classified Invalid"
  - "Lines we create use the header's Eol, falling back to the nearest terminated line above (or LF) when the header is an unterminated last line"
metrics:
  tasks: 2
  completed: 2026-10-12
---

# Phase 16 Plan 04: Edit API and PARSE-04 Proof Summary

Block-scoped `set_enabled` / `replace_block_lines` / `set_meta` that change only the edited block's lines, refuse anything that would yield an invalid enabled block, and are proven byte-exact on the real 258 KB FilterBlade fixture.

## Tasks

| Task | Commit | Notes |
|------|--------|-------|
| 1. Edit API with validity guards | 7a29056 | 16 unit tests in edit.rs |
| 2. Fixture edit tests, no-panic sweep, CHANGELOG | 6910912 | 7 new integration tests |

## Test results

- Full `cargo test`: 401 lib tests pass; filter_roundtrip 16 (4 roundtrip, 5 corpus, 6 edit_*, parse_never_panics_on_prefixes); artifact_guard, generate_snapshot, cli_* and the rest green.
- Fixture still `i/crlf attr/-text`. No changes to fixtures, emitter, compile, dynamic, overlay, config.json, CLAUDE.md.
- Built with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`.

## Deviations from Plan

None functionally. Choice recorded: the full-rebuild alternative was used instead of partial re-derivation (cost is a re-parse per edit, fine at this scale). The `Fail`/structure check means disabling a block whose header has trailing tokens (e.g. `Hide ItemLevel < 85`) is refused as NotOneBlock, because such a header is not recognizable as a disabled header.

## Known Stubs

None.

## Threat Flags

None. T-16-12..16 mitigated as planned (embedded newline rejection, one-block check, validate-before-mutate, key/value validation, prefix sweep, byte prefix/suffix tests).

## Self-Check: PASSED
