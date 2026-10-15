---
phase: 20-full-grammar-block-editing
plan: 03
subsystem: overlay-widgets
tags: [rust, egui, overlay, widgets]

requires:
  - phase: 20-02
    provides: grammar module (consumed later by 20-07/08)
provides:
  - widgets::remove_x_button, themed_slider, number_field/NumberField, chip_list/ChipListOutcome, colour_swatch, rgba_caption, toggle_chip, badge
  - 24x24 click targets on circular_toggle and arrow_button with accessible names
  - theme::WARM_ACCENT_TEXT
affects: [20-07, 20-08, 20-09]

key-files:
  modified: [src/overlay/widgets.rs, src/overlay/theme.rs]

key-decisions:
  - "number_field is a TextEdit with a temp-memory draft (not DragValue) so an invalid state can be exposed"
  - "number_field clears egui's CHANGED flag (response::Flags, doc-hidden but public) when the draft is unparsable or equals the value, so changed() means a real commit"
  - "Salts are hashed to a u64 (salted_id) because egui 0.36 id salts require Hash + Debug, while the plan's API takes impl Hash"
  - "Padded click targets use ui.interact on rect.expand(4.0) with an id derived from next_auto_id() so ids stay stable across frames"
  - "Padded-target test checks 14px outside, not 8px: egui adds its own 5px interact radius on top of the 4px expansion"

requirements-completed: []
requirements-progress: [EDIT-01, EDIT-02]

completed: 2026-10-13
---

# Phase 20 Plan 03: Reusable body controls Summary

**Seven new egui controls (remove X, slider, number field, chip list, colour swatch, toggle chip, badge) that write only on real user change, plus 24px click targets and the WARM_ACCENT_TEXT token.**

## Accomplishments
- Toggle and arrow keep 16px layout but click on a 24x24 target; both expose `widget_info` names.
- `themed_slider` never clamps out-of-range values; `colour_swatch` returns `Some` only for a real unmultiplied change (idle translucent colour returns `None`).
- `chip_list` restores the v0.4 list entry (quoted-paste splitting via `parse_base_type_entry`, rejects control characters, `N items` counter, scroll cap).
- 21 widget tests; full `cargo test` green (557 lib tests), `cargo fmt --check` clean, `cargo build --bins` warning-free, fixtures untouched.

## Task Commits
1. Task 1 - padded targets, remove X, toggle chip, badge, WARM_ACCENT_TEXT: 6cc6f8d
2. Task 2 - slider, number field, colour swatch: 52c568c
3. Task 3 - chip list: 3681657

## Deviations from Plan
- `remove_x_button_with_id` private helper added so chip_list can look up a specific chip's X in tests; the public signature is unchanged.
- The `+ ADD` test locates the button relative to the add field rather than by id.

## Self-Check: PASSED
