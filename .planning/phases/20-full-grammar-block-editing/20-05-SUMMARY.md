---
phase: 20-full-grammar-block-editing
plan: 05
subsystem: overlay-model
tags: [rust, overlay, working-model]

requires:
  - phase: 20-01
    provides: LinePatch, patch_line, insert_body_line, remove_body_line, EditError
provides:
  - rules::{LineKind, EntryKind, BodyEntry, STALE_LINE_STATUS, NOT_EDITABLE_STATUS, coalesce_edits}
  - WorkingFilter::{body_view, body_editable}
  - RowAction::{EditLine, AddLine, RemoveLine}, ActionOutcome.line
affects: [20-06, 20-07, 20-08]

key-files:
  modified: [src/overlay/rules.rs, tests/overlay_rules.rs]

key-decisions:
  - "body_view is computed on demand for the one expanded row; RuleRow is not widened"
  - "Guard order for line actions: id is a row and body_editable (NOT_EDITABLE_STATUS), then expect-text match (STALE_LINE_STATUS)"
  - "AddLine(Condition) goes after the last condition, else after the header; AddLine(Action) after the last action, else the last body line, else the header"
  - "Opaque entries are keyword None, op None, values empty; they stay editable through LinePatch::Whole and RemoveLine"

requirements-completed: []
requirements-progress: [EDIT-01, EDIT-02]

completed: 2026-10-13
---

# Phase 20 Plan 05: Line-level working model Summary

**`WorkingFilter` now exposes every body line as a typed `BodyEntry` and accepts `EditLine`/`AddLine`/`RemoveLine` actions guarded by an expect-text stale check, with `coalesce_edits` collapsing same-line edits per frame.**

## Accomplishments
- `body_view` yields condition, action, `Continue` and opaque lines in file order with raw (quoted) value tokens, operator and per-keyword ordinal; disabled-block offsets handled as in `extract`.
- Line actions route through the 20-01 API; refusals (stale, not editable, library `EditError`) leave text unchanged and surface a status.
- Fixture tests: 100 edits on the 511-block FilterBlade file within budget, `revert()` restores bytes, and `body_view` count equals each block's body length.
- Full `cargo test` green, `cargo fmt --check` clean, bins warning-free, fixtures untouched.

## Task Commits
1. Tasks 1 and 2 (body view + line actions): 80231d5
2. Task 3 (fixture tests): c28d68a

## Deviations from Plan
- Tasks 1 and 2 share one commit: the new `RowAction` variants must be matched in `apply_action`, so the view alone could not compile as a separate commit. Unit tests for both live in `rules.rs`.
- `src/overlay/rules.rs` is stored with CRLF in the working tree; it was rewritten LF-only (git normalizes either way).

## Known Stubs
None.

## Self-Check: PASSED
