---
phase: 31-rules-tab-on-the-skin
plan: 05
subsystem: ui
tags: [egui, skin, rule-editor]
requires: [31-03, 31-04]
provides:
  - two-column expanded rule editor (ff_field_row headers, row-ground cards)
  - segmented Yes|No, Any|Named, Show|Hide; ff_check Continue
  - skin NAME field, Duplicate / Delete buttons
affects: [31-06]
key-files:
  modified:
    - src/overlay/body.rs
    - src/overlay/rule_list.rs
key-decisions:
  - "Card inset is two nested frames (egui::Margin is i8, cannot hold 154pt); the header reaches back over the inset so its label sits at the row edge"
  - "Delete button is always enabled (not tied to editable) so non-editable rules stay deletable"
requirements-completed: []
duration: long
completed: 2026-10-16
---

# Phase 31 Plan 05: Expanded rule editor on the skin Summary

The expanded rule is a two-column skin editor: small-caps labels in a 128pt column, controls from label end + 16, cards on alternating ROW_EVEN/ROW_ODD ground, every control still pushing the same RowActions.

## Commit

- Single code commit for all three tasks (the seams are interdependent: header, section heads and expanded_body share the column geometry). See `git log` for `feat(31-05)`.

## What changed

- `card_header` / `condition_header` / `action_card` headers go through `ff_field_row`; X buttons stay right-aligned.
- `card_frame` returns a `CardFrame` (alternating ground, 154pt left inset so value lines sit in the control column); `raw_card_frame` for full-width raw lines.
- Conditions / Style heads: label column plus 132pt `+ condition` / `+ style` skin buttons feeding the unchanged picker popups.
- Booleans, Any|Named and Show|Hide use `ff_segmented`; a foreign `Minimal` shows a muted word with neither segment lit. Continue is an `ff_check` line (tooltip), still last.
- Defences toggles live in an `ff_card`.
- `expanded_body`: NAME `ff_text_field_id` (focus_name honoured), Duplicate / Delete 112pt skin buttons (Delete danger red, arms `confirm_delete` only); body ground `ROW_EVEN`, margin 0x4.

## Deviations from Plan

- **[Rule 3] Margin type.** `egui::Margin` is `i8`; 154pt does not fit, so `CardFrame` nests two frames.
- Body frame margin is `symmetric(0, 4)` rather than `(10, 4)` so `ff_field_row` (which adds its own 10pt pad) puts labels at row left + 10.
- Delete is always enabled (plan said `ctx.editable`), to keep delete available for non-editable rules (capability preservation).
- Tests: scan-based click tests retuned to the new geometry (toggle x 170-186, class header x 170, wider y ranges, `+ condition` x 150-300). Added `condition_control_starts_after_label_column`, `defences_card_is_an_ff_card_with_three_lines`, `section_heads_use_skin_buttons`, `bool_segment_keeps_not_equal_operator`, `duplicate_and_delete_are_skin_buttons`. `minimal_header_shows_muted_word_until_changed` not added; `minimal_block_renders_idle_and_untouched` already covers no-emit.
- Defences: the "three sub-lines" are the existing toggle rows inside the `ff_card`; the card's title is empty.

## Verification

Full `cargo test` green (1174 lib tests), fmt clean, clippy warnings 126 (limit 126), fixtures untouched.

## Known Stubs

None.

## Self-Check: PASSED
