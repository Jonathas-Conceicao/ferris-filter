---
phase: 31-rules-tab-on-the-skin
plan: 04
subsystem: ui
tags: [egui, skin, widgets, editor]
requires: [31-02, 31-03]
provides:
  - circular_toggle, toggle_chip, remove_x_button, arrow_button, themed_slider and number_field delegate to skin widgets
  - widgets::NUMBER_FIELD_W (44pt exact-number field) and slider width reservation
  - Min / Max labels on Between slider lines
affects: [31-05]
key-files:
  modified:
    - src/overlay/widgets.rs
    - src/overlay/body.rs
    - src/overlay/config_view.rs
key-decisions:
  - "Width reservation lives inside themed_slider (available width minus 52pt), not at each call site; count_names_control additionally caps its scope for the trailing label"
  - "Log sliders slide over ln(1 + v - min), continuous, mapped back by round(exp(u) - 1)"
requirements-completed: []
duration: long
completed: 2026-10-16
---

# Phase 31 Plan 04: Legacy wrappers on the skin Summary

Every editor control (conditions, actions, Defences, Between, Simple Defences chips, CONFIG toggles) now draws skin widgets through unchanged wrapper signatures.

## Commits

- c0bfd34 circular_toggle -> ff_check
- toggle_chip -> ff_chip (test contexts install the skin fonts)
- remove_x_button -> ff_x_button (explicit-id variant via push_id)
- 91df49f arrow_button -> ff_arrow
- themed_slider -> ff_slider, number_field -> 44pt ff_text_field_id
- Between `Min` / `Max` labels (regular 15, MUTED, 36pt slot)

## Deviations from Plan

- **[Rule 3] Slider reservation centralised.** `themed_slider` allocates `available - 52` itself instead of five edited call sites; the number field width is passed as `NUMBER_FIELD_W`. `count_names_control` wraps slider and field in a scope narrowed by 80pt for its trailing label.
- **[Rule 3] Test fixtures.** `overlay::widgets` and `overlay::config_view` test contexts now call `skin::fonts::install` (chips use SmallCaps). The two chip-list/string-list X tests read the X rect from a cfg(test) thread-local recorded in `remove_x_button_with_id`, because `push_id` changes the widget id.
- The old padded-hit-target circular toggle test was replaced by a 22pt check test.
- `int_control_number_field_right_of_slider` exercises the wrapper pair at 360pt rather than the full card.
- The skin slider reserves 12pt for its (empty) value text; accepted.

## Verification

Full `cargo test` green (1169 lib tests), fmt clean, clippy warnings 125 (limit 126), fixtures untouched.
New tests: `themed_slider_never_clamps_on_display`, `themed_slider_log_maps_midpoint`, `number_field_is_44_wide_and_keeps_focus_id`, `between_card_labels_min_and_max`, `int_control_number_field_right_of_slider`.

## Known Stubs

None.

## Self-Check: PASSED
