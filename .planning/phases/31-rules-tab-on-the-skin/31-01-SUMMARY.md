---
phase: 31-rules-tab-on-the-skin
plan: 01
subsystem: ui
tags: [filter-model, overlay, rules-tab]
requires: [31-02]
provides:
  - Document::insert_rule_at_top / top_insertion_line (single insertion point for new rules)
  - apply::RULE_ADDED_MESSAGE, ListUi.focus_name, new ADD_RULE_TIP
affects: [32]
tech-stack:
  added: []
  patterns: [one-shot focus request on ListUi]
key-files:
  created: []
  modified:
    - src/filter/structure.rs
    - tests/filter_structure.rs
    - src/overlay/rules.rs
    - src/overlay/apply.rs
    - src/overlay/app.rs
    - src/overlay/rule_list.rs
key-decisions:
  - "+ Add rule inserts as the first block of the file, above that block's leading comments/meta and after any free-standing preamble, never inside the economy section"
requirements-completed: []
duration: 40min
completed: 2026-10-16
---

# Phase 31 Plan 01: Add rule at the top of the file Summary

+ Add rule now inserts the new empty Show rule as the first block of the file (via `Document::insert_rule_at_top`), and the overlay clears the search, expands and scrolls to row 1, focuses its NAME field and reports "Rule added at the top of the file. Press Apply to save it."

## Tasks

1. `insert_rule_at_top` replaces `append_rule`; `top_insertion_line` is the one insertion point Phase 32 extends to stop before the first divider: commit 1f287a4
2. Overlay reaction (query cleared, expanded, scroll, `focus_name`, `RULE_ADDED_MESSAGE`, `ADD_RULE_TIP`): commit b9e697f

## Deviations from Plan

- The byte-preservation integration test compares prefix and suffix around the new block rather than a pure suffix: the fixture starts with a free-standing comment preamble, and the new block goes after it (per the plan's own preamble behaviour).
- `crlf_is_preserved` unit test reordered (insert first) because the new block is now first.
- RULES-03 is not ticked in REQUIREMENTS.md (owned by the phase's last plan).

## Verification

Full `cargo test` green (1161 lib tests), fmt clean, clippy warnings 126 (limit 126), `tests/fixtures` untouched.

## Known Stubs

None.

## Self-Check: PASSED
