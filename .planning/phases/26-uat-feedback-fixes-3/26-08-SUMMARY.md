---
phase: 26-uat-feedback-fixes-3
plan: 08
subsystem: overlay
tags: [egui, rule-editor, between, defences]
requires: [26-02, 26-03, 26-06]
provides:
  - "body.rs between_card, Between in numeric operator dropdowns (top level and in Defences)"
  - "to_between_actions / from_between_actions / clamped_bound pure helpers"
affects: []
key-files:
  modified:
    - src/overlay/body.rs
    - src/overlay/rule_list.rs
requirements: [UAT3-06]
completed: 2026-10-14
---

# Phase 26 Plan 08: Between operator Summary

Whole-number conditions (and each defence) can use a Between operator: one card, two sliders, two lines `X >= min` / `X <= max`; loaded `>`/`<=` pairs show as one card with their original bytes.

## What was built

- `between_card` renders `Part::Between`; `defence_between` renders it inside the Defences card; both share `between_op_combo` and `between_sliders`.
- Min and max each edit only their own line (`int_edit`). `clamped_bound` keeps min <= max on edit; a loaded impossible pair shows a red "min > max" hint and is never rewritten. The X removes both lines.
- `op_controls` replaces the plain dropdown for single whole-number lines with `numeric_op_choices` (Between included). Pick Between: `to_between_actions` = `InsertLineAfter` (expect-checked, issued first) plus an op-only edit only when `==` becomes `>=`. Pick a single operator on a Between card: `from_between_actions` keeps the bound matching the direction (op edit only if the operator differs) and removes the other line.
- Other duplicates (two `>=`, `==` + `>=`, three lines) stay separate cards (card_plan, 26-02).
- `mirror_apply` test helper treats `InsertLineAfter` as a line-shifting action.
- Fixed the pre-existing `unused_mut` test warning in rule_list.rs.

## Deviations from Plan

- Both tasks are one commit (9682ac1): they share the same functions in body.rs.
- Dropdown popup selection is not simulated headless (same limitation as 26-04/06); the switch behaviour is tested through the pure action builders applied via `WorkingFilter::apply_action`, plus rendered-frame checks (idle byte identity, "Between" text, "min > max" flag).

## Verification

Full `cargo test` passes (951 lib tests plus integration), `cargo fmt --check` clean, fixtures untouched.

## Known Stubs

None.

## Self-Check: PASSED
