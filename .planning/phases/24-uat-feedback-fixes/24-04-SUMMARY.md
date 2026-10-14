---
phase: 24-uat-feedback-fixes
plan: 04
subsystem: overlay
tags: [uat, conditions, lists, booleans]
requires: [24-01, 24-02]
provides:
  - widgets::string_list (vertical capped quoted list)
  - body::condition_header, bool_effective, bool_edit, quoted_list, unknown_classes
affects: [src/overlay/widgets.rs, src/overlay/body.rs]
key-files:
  modified:
    - src/overlay/widgets.rs
    - src/overlay/body.rs
decisions:
  - Boolean Yes/No edit keeps the line's operator and flips only the value word (negated lines write the inverse word), so `Corrupted != True` reads No and Yes writes `!= False`
  - Booleans with ordering operators keep the operator combo and raw value control
  - Unknown classes are shown on one compact line with a remove X each and a tooltip
metrics:
  completed: 2026-10-13
requirements: [UAT-05, UAT-06, UAT-07, UAT-12]
---

# Phase 24 Plan 04: Condition editor tightening Summary

Condition remove X is flush right, quoted lists are a vertical 7-row-capped list, the Class card is the grouped picker only, and booleans are a compact Yes | No.

## Tasks

1. `widgets::string_list` (commit 3892630): add field factored into `add_field`, shared with `chip_list`; four new tests plus existing chip tests pass.
2. and 3. (commit beb8ff6, one commit because both tasks edit the same body.rs regions): `quoted_list` replaces `list_chips` for BaseType, count-names and pending rows; `class_picker` lost its chip block and gains `unknown_classes` line; `condition_header` right-aligns the X in condition, pending and raw rows; `bool_control` replaced by `bool_yes_no` + `bool_edit` + `bool_effective`. Tests for every behavior bullet added.

## Deviations from Plan

- Tasks 2 and 3 were committed together rather than individually (shared file, interleaved hunks).
- Removed the now-unused `field_id` local in `chip_list` (compiler warning).

## Verification

Full `cargo test` green (827 lib tests plus integration), `cargo fmt --check` clean, no generator change.

## Known Stubs

None.

## Self-Check: PASSED
