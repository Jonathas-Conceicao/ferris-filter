---
phase: 19-overlay-rule-list-over-parsed-blocks
plan: 04
subsystem: overlay
tags: [rust, overlay, working-model, virtual-list]
requires: [19-02, 19-03]
provides:
  - overlay::rules::WorkingFilter (row cache, actions, staged economy, Revert)
  - overlay::virtual_list (build_offsets, visible_range, scroll_offset_for)
affects: [19-05, 19-06]
tech-stack:
  added: []
  patterns: [rows rebuilt once per mutation, one-pass move flags, deferred RowAction vocabulary]
key-files:
  created:
    - src/overlay/rules.rs
    - src/overlay/virtual_list.rs
    - tests/overlay_rules.rs
  modified:
    - src/overlay/mod.rs
    - src/filter/mod.rs
key-decisions:
  - "can_move_up/down computed in one pass from the ordered block list and the economy span (W1); a test asserts agreement with Document::move_neighbour"
  - "Row name read only via meta_get(id, \"name\"); derived labels never use the header comment"
requirements-completed: [EDIT-03, EDIT-04, EDIT-05, EDIT-07, EDIT-08, ECON-02]
duration: 30min
completed: 2026-10-13
---

# Phase 19 Plan 04: Working Model and Virtual List Summary

Egui-free `WorkingFilter` (cached `RuleRow`s, `RowAction` dispatch over `filter::Document`, staged economy, Revert) plus tested virtual-list offset arithmetic, proven on the 511-block FilterBlade fixture.

## Commits
- 67f6071 feat: WorkingFilter row cache, labels, search, economy view and row actions (tasks 1 and 2 implementation)
- 335f277 test: fixture-scale behaviour and perf budgets (task 2 integration tests)
- 5ce0a33 feat: virtual list offset arithmetic (task 3)

## Results
- 20 unit tests in `overlay::rules`, 8 in `overlay::virtual_list`, 2 fixture tests; full suite green, `cargo fmt --check` clean.
- Fixture: 511 rows (460 enabled, 51 disabled), no blocking errors; load, search, 100 mixed edits and byte-exact revert all inside the debug budgets (whole test about 2 s).

## Deviations from Plan

**1. [Rule 3 - Blocking] Added `Default` to `filter::Document`'s derive.** `from_text` needs a no-unwrap empty fallback for the (impossible) parse failure; `Document` had no constructor. One-word change in `src/filter/mod.rs`.

**2. Task 1 and Task 2 library code landed in one commit.** The action dispatch lives in the same file as the row cache, so it was written together; the integration tests are their own commit.

Otherwise the plan was executed as written. A `Move` with no neighbour returns `changed=false` with the status "There is no rule to swap with in that direction."

## Known Stubs
None.

## Threat Flags
None.

## Self-Check: PASSED
