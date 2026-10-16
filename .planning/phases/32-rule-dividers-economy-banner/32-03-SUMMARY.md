---
phase: 32-rule-dividers-economy-banner
plan: 03
subsystem: filter-model
tags: [rust, filter, sections, lossless-edit]

requires: [32-01]
provides:
  - "Document::{insert_section_at_top, insert_section_above, rename_section, remove_section, section_can_move, move_section, rule_step}"
  - "filter::RuleStep { Swap, CrossBanner, Blocked }"
  - "section::sanitize_name, EditError::{UnknownSection, SectionLocked}"
  - "top_insertion_line stops at the first user banner (RULES-03)"
affects: [32-04, 32-05]

key-files:
  created:
    - src/filter/divider.rs
    - .planning/phases/32-rule-dividers-economy-banner/shots/03-regression.png
  modified:
    - src/filter/mod.rs
    - src/filter/section.rs
    - src/filter/edit.rs
    - src/filter/structure.rs
    - tests/filter_structure.rs

key-decisions:
  - "Every divider edit is a line splice through commit_structural with unchanged ids and span texts, so any span change is refused"
  - "A banner moves by swapping its unit (3 lines plus trailing blanks) with the nearest rule unit (whole group run); lines between stay, so a move is its own inverse"
  - "move_neighbour / move_block still refuse to swap across a banner; rules hop dividers through rule_step + move_section(k, !up)"

requirements-completed: []  # model only; SECT-02/05 and RULES-03 are user-visible with the UI in plan 05

duration: ~40min
completed: 2026-10-16
---

# Phase 32 Plan 03: Divider edits in the filter model Summary

**Dividers can be inserted (top or above a rule or group run), renamed, removed and moved one rule unit at a time as lossless Document edits, rules hop dividers via `rule_step`, and `+ Add rule` now lands above any leading divider.**

## Accomplishments
- `insert_section_at_top/above`, `rename_section`, `remove_section`: blank lines added only where missing; remove deletes the blank below (else above), so insert then remove restores the bytes (LF, CRLF, unterminated last line, no-block files).
- `sanitize_name` turns every control char into a space (T-32-09); `"x\nShow"` becomes `x Show` and adds no block.
- `section_can_move`, `move_section`, `rule_step` + `RuleStep`; economy banner/blocks are locked or `Blocked` (T-32-11); neighbour units are resolved through `group::analyze` (T-32-12).
- `top_insertion_line` returns the earlier of the first block and the first user banner.

## Task Commits
1. Insert / rename / remove + top insertion point - `948b4cf`
2. Move dividers, rule_step, regression shot - `8c6644d`

## Gates
- `cargo test`: all green (213 `filter::` lib tests, 17 `filter_structure`, full suite 1213 lib tests).
- `cargo fmt --all --check` clean; clippy `^warning` 126 (baseline); `tests/fixtures` unchanged.

## Regression screenshot
`shots/03-regression.png` (RULES tab, 32-01 sample filter): identical to before. Six rules, search hint `Search 6 rules, ...`, `+ Add rule`, footer `+ 60 locked rule blocks (economy)` with Remove, status `Filter out of date - press Apply`. No damaged status.

## Deviations from Plan
None of substance. Notes:
- `EditError` has no `PartialEq`, so unit tests use `matches!` instead of `assert_eq!`.
- Known corner: moving an unterminated last rule across a banner whose unit ends in a blank last line cannot restore "no final newline" (an empty line cannot carry it); a move and its inverse then differ by one final newline. Not reachable with the overlay (files it writes are terminated); not tested.
- `RuleStep` is re-exported as `ferrisfilter::filter::RuleStep` (the `divider` module stays private) so integration tests and Plan 04 can name it.

## Known Stubs
None. The methods are consumed by Plan 04.

## Threat Flags
None. T-32-09 to T-32-12 are covered by `rename_cannot_inject_a_rule`, the identity tests, `nothing_crosses_the_economy_banner`, `divider_steps_over_a_group_run`.

## Self-Check: PASSED
- src/filter/divider.rs and shots/03-regression.png present; commits 948b4cf and 8c6644d exist.
