---
phase: 19-overlay-rule-list-over-parsed-blocks
plan: 05
subsystem: overlay
tags: [egui, virtual-list, rule-list]
requires: [19-04]
provides: [overlay::rule_list (ListUi, ListFrame, EconomyFooterAction, row_height, show_list, search_bar, economy_footer, economy_footer_text)]
affects: [19-06]
key-files:
  created: [src/overlay/rule_list.rs]
  modified: [src/overlay/mod.rs]
requirements-completed: [EDIT-03, EDIT-04, EDIT-05, EDIT-07, ECON-02]
duration: ~15min
completed: 2026-10-12
---

# Phase 19 Plan 05: Rule list view Summary

Virtualized egui rule list (show_viewport + cached offsets) with collapsed-row flags, in-place expanded body, search bar and pinned economy footer, emitting only deferred `RowAction`s.

## Accomplishments
- `show_list` draws only viewport rows (511 rules: drawn <= ceil(600/row_h)+2, tested), keyed by `push_id(("rule", BlockId))`, with `skip_ahead_auto_ids`.
- Collapsed row: circular toggle, truncated label with hover, six flags in fixed order, up/down arrows (disabled with hint when search active).
- Expanded body: NAME toggle/field (no write until typed), ACTION selector + Continue (read-only with note under a style tag), read-only CONDITIONS, Duplicate / Delete with inline confirm.
- `search_bar`, `economy_footer` (Present/Absent/Damaged, inline Remove confirm).
- Six headless tests pass; full suite 488 + integration pass; 0 build warnings; fmt clean.

## Deviations from Plan
- Tasks 1 and 2 were implemented and committed together in one commit (single new file; the split would have required a stub body) - commit 2fa112a.
- The arrow test sweeps click positions across the row (positive control for enabled, none for disabled) rather than computing arrow rects.

## Issues Encountered
None.

## Self-Check: PASSED
