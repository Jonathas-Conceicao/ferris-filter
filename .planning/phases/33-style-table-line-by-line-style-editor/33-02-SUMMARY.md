---
phase: 33-style-table-line-by-line-style-editor
plan: 02
subsystem: ui
tags: [egui, skin, style-table, minimap-icon]
requires: []
provides:
  - "config_view: table_columns geometry, five-column header, skin style rows (Icon, Preview, greyed built-in Order arrows)"
  - "config_view::label_look: Style -> skin LabelLook"
affects: [33-03, 33-04, 33-05]
tech-stack:
  added: []
  patterns:
    - "pure column geometry shared by header and rows, asserted through test probes"
key-files:
  created: []
  modified:
    - src/overlay/config_view.rs
key-decisions:
  - "Below the 110pt Name minimum the Preview column shrinks (down to 100pt) so every column still ends at width - 10 in narrow windows."
  - "The row click target starts at the Icon column for custom rows (the Name cell is a text field) and at Name for built-ins, and stops before Order."
requirements-completed: [STY-01, STY-04]
duration: 50min
completed: 2026-10-16
---

# Phase 33 Plan 02: Style table on skin widgets Summary

Both CONFIG style groups now share a Visibility / Name / Icon / Preview / Order header and row geometry (`table_columns`), with the minimap Icon column, a skin item-label Preview, no Show/Hide caption, and built-in Order arrows drawn disabled.

## Tasks

1. **Column geometry, header, headings, draft row** (da40c88): `table_columns(width)`, `styles_table_header`, small-caps group headings, `+ Add style` skin button on the Custom heading, draft row on `ff_text_field_id` / `ff_button`. Old `COL_*` constants removed.
2. **Style rows on skin widgets** (3ae7867): `style_row` rebuilt on `row_background`; Icon via `paint_minimap_icon`; Preview via `ff_item_label` + pure `label_look`; `ff_arrow` Order cell (disabled with tooltip `Built-in styles keep their order` on built-ins); rename field on `ff_text_field_id`; gold outline around an expanded row plus its editor. `style_preview_chip` no longer used here (Plan 03 removes it from `widgets.rs`).

## Deviations from Plan

**1. [Rule 1 - Layout arithmetic] Preview column yields below the Name minimum.** The plan's `table_columns(460.0)` requires Name >= 110 and all columns ending by `width - 10`, which cannot hold with a fixed 160pt Preview (474pt needed). Preview shrinks (min 100pt) for narrow widths; at 517pt the values match the plan exactly (Name 153).

**2. [Rule 3 - Test fixture] Rename-test click position.** The inset skin text field puts the caret mid-text when clicked at its centre; the test helper now clicks near the field's right end. No behaviour change.

`caption_text` was kept (the TIERS table still uses it). The task split is by file state: Task 1's commit already contains the row skeleton on the new columns (with the old chip and arrows), Task 2 swaps the cell contents.

## Gates

`cargo test` 1270 lib tests plus integration green, `cargo fmt --all --check` clean, clippy warnings 122 (<= 122), `tests/fixtures` unchanged.

## Known Stubs

None.

## Threat Flags

None. T-33-04 unchanged (`rename_hint` still blocks invalid names; tests kept); T-33-05 covered by the disabled built-in arrow click test.

## Self-Check: PASSED

src/overlay/config_view.rs modified; commits da40c88 and 3ae7867 present.
