---
phase: 26-uat-feedback-fixes-3
plan: 02
subsystem: overlay
tags: [grammar, conditions, defences, between]
requires: []
provides:
  - "grammar::ConditionEntry, condition_picker_groups (one Defences entry after Rarity)"
  - "overlay::conditions: card_plan, present_entries, numeric_op_choices, between_switch, between_ok, DEFENCES"
affects: [26-06, 26-08]
key-files:
  created: [src/overlay/conditions.rs]
  modified: [src/filter/keywords.rs, src/overlay/grammar.rs, src/overlay/mod.rs, src/dynamic.rs]
decisions:
  - "condition_groups() kept as a projection (Defences expands in place) so body.rs compiles unchanged until 26-06"
  - "Between only for ValueShape::Int keywords with exactly one lower and one upper single-integer line"
requirements: [UAT3-05, UAT3-06]
metrics:
  completed: 2026-10-14
---

# Phase 26 Plan 02: Defences/Between pure logic Summary

BaseDefencePercentile is removed (foreign lines stay lossless raw rows), the condition picker model has a single Defences entry after Rarity, and a new GUI-free `overlay::conditions` module supplies card grouping, presence checks and Between helpers.

## Commits
- ab7c347: drop BaseDefencePercentile; picker model with one Defences entry
- c07839b: conditions.rs card plan, presence and Between helpers

## Deviations from Plan
None. `cargo fmt` reformatted two test snippets before the second commit.

## Notes
- between_switch inserts the slider limit as the other bound, clamped so the pair is satisfiable (e.g. lower 250 gives upper 250).
- Full `cargo test` passes (883 lib tests plus integration), `cargo fmt --check` clean, fixtures untouched.
- A pre-existing `unused_mut` warning in rule_list.rs is out of scope.

## Known Stubs
None.

## Self-Check: PASSED
