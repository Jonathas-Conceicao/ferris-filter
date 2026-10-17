---
phase: 31-rules-tab-on-the-skin
plan: 03
subsystem: ui
tags: [egui, skin, rule-list, virtual-list]
requires: [31-01, 31-02]
provides:
  - skinned search_row (search field and + Add rule 8pt apart, Clear inside the field)
  - 52pt two-line collapsed rows (ff_check, ff_arrow, flag cluster, flag_budget)
  - show_list inside ff_scroll_area_viewport
affects: [31-04, 31-05]
key-files:
  modified:
    - src/overlay/rule_list.rs
    - src/overlay/app.rs
    - src/overlay/body.rs
    - src/skin/mod.rs
    - src/skin/textures.rs
key-decisions:
  - "show_list keeps its signature and fetches the skin with skin::of(ctx) (body.rs tests call it)"
  - "the + Add rule button is placed from the row's right edge, because the text field leaves the cursor 2pt short of its end"
requirements-completed: []
duration: long
completed: 2026-10-16
---

# Phase 31 Plan 03: Toolbar, rows and list on the skin Summary

Search row, collapsed rule rows and the list container are drawn with the skin widgets; the 511-rule list stays virtualised.

## Tasks

1. Toolbar and collapsed row (52pt, hierarchy, flag cluster) and 2. list container: commit 2474e5d (the two tasks share one file and were committed together).
3. Test support: commit 5df517f.

## Deviations from Plan

- **[Rule 3 - Blocking] show_list takes no `skin` parameter.** Adding it would have forced edits to the `show_list` call sites in `body.rs` tests (owned by 31-04). It calls `skin::of(ui.ctx())` instead; `draw_row` takes `&Skin`. `app.rs` call is unchanged apart from `add_space(8.0)`.
- **[Rule 3 - Blocking] body.rs test scan ranges moved.** Pixel-scan tests assumed the old row header height; the group-header scans moved down 30pt (`188..212` -> `218..242`, `164..200` -> `194..230`). Test code only.
- **[Rule 3 - Blocking] light skin in `skin::of` under cfg(test).** The body click scans build a fresh `Context` per frame; loading the full skin each time (decoding the 4K PNGs) made them take tens of minutes. The fallback now uses 2x2 placeholder textures in tests; `skin::install` still loads the real art.
- **[Rule 1 - Bug] search field touched the button.** `ff_text_field` leaves the layout cursor 2pt before its end, so the 8pt item gap became -2. The button is now placed from the row's right edge and its width is measured (the caption needs 126.6pt, more than 124).
- The expanded row's body keeps the old `PANEL_GROUND` frame (redrawn in plan 05); the expanded row hides the summary line.

## Verification

Full `cargo test` green (1165 lib tests), fmt clean, clippy warnings 125 (limit 126), fixtures untouched.
New tests: `collapsed_row_is_52`, `flag_budget_keeps_label_minimum`, `continue_word_is_8pt_from_styled_chip`, `search_field_and_add_button_do_not_touch`, `pending_scroll_brings_a_far_row_into_view`.

## Known Stubs

None.

## Self-Check: PASSED
