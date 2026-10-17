---
phase: 31-rules-tab-on-the-skin
plan: 06
subsystem: ui
tags: [egui, skin, rule-editor]
requires: [31-05]
provides:
  - every editor dropdown is an ff_dropdown (foreign operators kept exact)
  - skinned + condition / + style picker and class picker
  - skinned quoted lists, draft and raw fields
  - test guard rules_editor_has_no_egui_default_widgets
affects: []
key-files:
  modified:
    - src/overlay/body.rs
    - src/overlay/widgets.rs
key-decisions:
  - "pick_dropdown appends a current value that is not offered as its own option, so an existing operator is never normalised"
  - "Picker and class sections reuse the existing data-stored open state with a custom section_head instead of CollapsingState"
requirements-completed: []
duration: long
completed: 2026-10-16
---

# Phase 31 Plan 06: Dropdowns, pickers and lists on the skin Summary

The rule editor now has no egui-default widget except the colour picker; all v0.5 behaviour and RowAction contracts are unchanged.

## Commits

- `feat(31-06): editor dropdowns become ff_dropdown` (Task 1)
- `feat(31-06): skinned pickers, quoted lists, raw/draft fields; no egui defaults left` (Tasks 2 and 3 together; the body.rs edits are interleaved)

## What changed

- `pick_dropdown` (body.rs) backs operators (numeric, Between, pending, list), enumerations, minimap size/shape and colour names. The style picker uses `ff_dropdown_ex` with `pal::WARN` for an unknown/unchecked tag, tooltip kept.
- `picker_popup`: box-art frame (`paint_box_behind`), `ff_text_field` search, `section_head` (small caps 17) sections collapsed by default and opened by a search, `pick_row` 28pt entries (greyed present entries stay clickable), `ff_scroll_area` capped at 320pt.
- `class_picker`: `section_head` groups, `circular_toggle` lines, unknown classes on one muted line.
- widgets.rs: `string_list` 28pt lines, right-aligned X, `ff_scroll_area` (max 7 rows); `add_field` is `ff_text_field_id` ("Type a name, press Enter") plus `ff_button("+ Add", 84)`; paste split and `CHIP_REJECT_MESSAGE` kept. `chip_list` (whole-number lists) drops its inner scroll area.
- `draft_field_ex` takes a font; raw rows use 14pt monospace. Simple Defences chips wrap with 8pt gaps.

## Deviations from Plan

- **Class picker not wrapped in `ff_card`**: it already sits inside the condition card, a second card would double-frame it.
- **`chip_list` has no scroll cap** (`max_height` is now unused): it only holds short whole-number lists.
- **`add_field` ignores the caller's hint**: the field always reads "Type a name, press Enter" as the plan specifies.
- Tasks 2 and 3 are one commit.
- Tests: colour-combo scroll-bar tests (egui ComboBox geometry) replaced by a skin-dropdown version; two source-grep picker tests retargeted. Added `foreign_operator_is_shown_and_not_normalised`, `style_picker_lists_custom_then_presets_in_file_order`, `picker_popup_uses_skin_search_and_sections`, `rules_editor_has_no_egui_default_widgets`.

## Verification

Full `cargo test` green (1177 lib tests), fmt clean, clippy warnings 126 (limit 126), fixtures untouched. No visual screenshot was taken.

## Known Stubs

None.

## Self-Check: PASSED
