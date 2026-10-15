---
phase: 19-overlay-rule-list-over-parsed-blocks
plan: 03
subsystem: dynamic-filter-writer
tags: [rust, apply, grammar-gate, safe-write]
requires: [19-01]
provides: [dynamic::apply_edited, dynamic::EconomyOp, dynamic::blocking_errors, dynamic::BlockingError, DynamicError::InvalidBlocks]
affects: [overlay Apply (later plans)]
tech-stack:
  added: []
  patterns: [shared private write helper behind one guarded writer]
key-files:
  modified: [src/dynamic.rs]
decisions:
  - "OD-1: only Error diagnostics in enabled non-economy blocks block Apply; Warnings never do"
  - "OD-2: EconomyOp::Keep allows rules-only filters (no section)"
  - "OD-3: blocks after economy-end are ordinary rules"
  - "Gate uses Block::owned() range (same test as edit.rs) rather than span()"
metrics:
  tasks: 2
  files: 1
  completed: 2026-10-12
requirements: [EDIT-08]
---

# Phase 19 Plan 03: apply_edited and grammar gate Summary

Overlay-facing Apply: `apply_edited` writes caller-edited text through the same `write_guarded` single writer as the CLI, behind an Error-only `blocking_errors` gate, with an explicit `EconomyOp` (Keep/Refresh/Add/Remove).

## What was built
- `blocking_errors(&Document)`: Error diagnostics in enabled, non-economy blocks, 1-based `rule_index` over non-economy blocks, falls back to "all blocks are rules" when markers are damaged.
- `DynamicError::InvalidBlocks { path, count, first_rule, first_line }`.
- `write_new_text` private helper extracted from `apply_file` (sole `write_guarded` call site); `apply_file` unchanged in behavior and still ungated.
- `apply_edited` + `EconomyOp`.
- 13 new hermetic tempdir tests (6 gate, 7 apply_edited).

## Verification
Full `cargo test` green (454 lib tests plus all integration suites), `cargo fmt --check` clean, fixtures untouched.

## Deviations from Plan
Both tasks touch only `src/dynamic.rs` and were interdependent (tests share helpers), so they were committed together in one commit (ef3e332) rather than two. No other deviations.

## Commits
- ef3e332: feat(19-03): apply_edited and blocking_errors grammar gate over the shared writer

## Known Stubs
None.

## Self-Check: PASSED
