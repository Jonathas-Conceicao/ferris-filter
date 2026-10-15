---
phase: 27-uat-feedback-fixes-4
plan: 01
subsystem: overlay-ui
tags: [egui, layout, config-tab, combo-box, uat]
requires: []
provides:
  - right-aligned Order column in the CONFIG Custom styles table (UAT4-01)
  - colour dropdowns that keep their scroll bar off the colour names (UAT4-02)
affects: [src/overlay/config_view.rs, src/overlay/body.rs]
tech-stack:
  added: []
  patterns:
    - "right_to_left strip for trailing table cells (same as RULES row arrows)"
    - "headless popup test that reads the painted scroll bar and text shapes"
key-files:
  created: []
  modified:
    - src/overlay/config_view.rs
    - src/overlay/body.rs
decisions:
  - "Colour rows fill the popup width (label in a justified layout) instead of reserving a padding constant: the bar is floating and allocates nothing, so the fix is to stop the ScrollArea shrinking to the row content"
metrics:
  duration: ~45m
  completed: 2026-10-15
requirements: [UAT4-01, UAT4-02]
---

# Phase 27 Plan 01: Order column and colour dropdown scroll bar Summary

The Custom styles Order header and arrows now sit at the table's right edge in a right_to_left strip, and colour dropdown rows fill the popup width so the scroll bar no longer covers the longest colour name.

## Tasks

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1 | Right-aligned Order column in the Custom styles table | 265bf93 | src/overlay/config_view.rs |
| 2 | Colour dropdown leaves room for its scroll bar | a5ae27f | src/overlay/body.rs |

### Task 1 (UAT4-01)

- New `cell_with(ui, salt, width, layout, add)` with `cell` (left to right) and `cell_right`
  (right to left) on top of it.
- `styles_table_header`: when `with_order`, the Order head cell is drawn inside
  `ui.with_layout(Layout::right_to_left(..))`, so it sits at the header frame's right content edge,
  with the "Order" label right-aligned in the cell.
- `style_row`: the order cell is drawn the same way. Inside it Down is added first (rightmost) and Up
  second, so they read Up then Down like the RULES rows. The `row-up` / `row-down` / `row-order`
  probes and `arrows_left` (which clips the row click target) are kept.
- Header frame and `widgets::zebra_row` both use an 8px horizontal inset, so header and arrows share
  the same right edge.
- Built-in table unchanged (no Order column).
- `styles_table_header_and_columns` now checks: the `row-order` right edge equals the `head-order`
  right edge and is the same on every custom row; each preview cell ends before its order cell; the
  Order column is well to the right of the shared columns; Up is left of Down, both inside the
  order cell, with Down at its right edge. The Visibility / Name / Preview alignment checks are
  unchanged. `custom_arrows_reorder_and_builtins_have_none` still passes, clicking the probed
  rects (T-27-01).

### Task 2 (UAT4-02)

- `colour_combo` rows are still `ui.horizontal` (dot, then label). The `selectable_label` now sits
  in a `with_layout(Layout::top_down_justified(Align::Min))` child, so it fills the rest of the row.
  The whole row width is now the hover/click target, as in the shape combo. Signature unchanged. All
  callers get the fix: minimap colour, PlayEffect colour, and the CONFIG `minimap_editor` /
  `beam_editor`.
- New test module `colour_combo_tests` (real fonts via `theme::install_fonts`). It opens the popup
  headless by clicking the button, hovers it for 16 frames with advancing time, and reads the
  painted output: the scroll bar as tall rects no wider than `spacing().scroll.bar_width`, and the
  item names as text shapes.
  - `colour_combo_rows_leave_scroll_bar_room_like_the_shape_combo`: for both the shape combo (the
    reference) and the colour combo, every painted item name ends left of
    `bar.right() - bar_width` (the bar at its full, hovered width).
  - `colour_combo_picks_and_offers_none`: clicking another colour returns `Some(name)`, clicking the
    current one returns `None`, `allow_none` adds "None" as the last entry, and without it there is
    no "None" entry.
- Checked that the test catches the bug: with the fix temporarily removed, the first test fails
  with `"Brown" ends at 85.09 under the bar [[92.5 25.0] - [94.5 225.0]]`.

## Cause

The cause is in egui 0.36.2, `src/containers/combo_box.rs`, lines ~395-415. The popup is a
`Popup::menu` (layout `top_down_justified`, width = button width). It calls
`ui.set_min_width(ui.available_width())` on the popup ui, then wraps the items in
`ScrollArea::vertical()`. In `src/containers/scroll_area.rs`:

1. `Prepared::end` (~L1179-1195) sizes the area's inner rect. The x axis has scrolling off and
   `auto_shrink.x` on, so its width follows the content: `(false, true) => content_size[d]`.
   `outer_rect` is that width plus `current_bar_use`.
2. The default `ScrollStyle` is `ScrollStyle::floating()` (`style.rs` ~L643):
   `floating_allocated_width: 0.0`. So `current_bar_use` is 0 (~L757) and the bar gets no room of
   its own. It is painted inside `outer_rect`, right-aligned at `max_cross - width` (~L1301-1354).
   That is 2px dormant and up to `bar_width` (10px) on hover.
3. So the bar always sits on the right edge of the ScrollArea's content. Whether it covers text
   depends on how wide the content is:
   - Shape combo: plain `selectable_label`s in the inherited justified layout fill the full
     available width. The content is as wide as the popup, the bar sits at the popup edge, and the
     label text (left-aligned, with padding) ends before it.
   - Colour combo: each row was a `ui.horizontal` (`ui.rs` `horizontal_with_main_wrap_dyn`, a
     `left_to_right` child). Its min_rect covers only the dot plus a text-sized selectable label;
     the parent's justification only widens the child's max_rect. The content was therefore as
     wide as the longest row, the ScrollArea shrank to it (measured: content right 94.5 in a
     popup whose content area ends at 117), and the floating bar was painted at x 92.5-94.5
     (84.5-94.5 when hovered), on top of the end of "Brown" / "Purple".

The fix makes the rows fill the justified width like the shape combo's rows, so the content spans
the popup and the bar lands at the popup edge. No padding constant is needed: with floating bars
egui reserves nothing anyway, and the shape combo works the same way.

## Verification

- `cargo test --lib overlay::config_view`: 26 passed, 0 failed.
- `cargo test --lib colour_combo_tests`: 2 passed.
- `cargo test --lib overlay::`: 461 passed, 5 failed. All 5 failures are styles.json-dependent
  tests in files this plan did not touch: `app::tests::clean_config_follows_outside_edits_and_never_writes_them_back`,
  `app::tests::apply_working_style_change_reaches_the_next_apply`,
  `config_state::tests::load_reads_styles_and_tiers`,
  `config_state::tests::loading_again_discards_edits_and_takes_outside_change` and
  `config_state::tests::save_new_style_writes_it_last`. They read the repo's `styles.json`, which
  has the author's uncommitted edits. This is part of the known baseline; the orchestrator runs
  the full suite with those files stashed.
- `cargo fmt --check` is clean. No generator change, so the fixtures are untouched. `styles.json`
  and `tiers.json` are still modified and uncommitted.

## Deviations from Plan

The plan was carried out as written. Small points:
- The Up/Down probes record different rect sizes. An enabled arrow's response rect is its 4px-padded
  click target; a disabled arrow's is the 16px glyph. So the arrow assertions compare centres (and
  the Down glyph's right edge = centre + 8) instead of containing rects.
- The test for the "None" entry sets `spacing.combo_height = 600` so the 12th entry is painted
  without scrolling. With the default 200px height it is culled.
- An earlier assertion that the shape and colour bars share a right edge was dropped. The shape popup
  is wider ("UpsideDownHouse" widens it past the button width), so the edges differ by design.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: src/overlay/config_view.rs (right_to_left in styles_table_header and style_row)
- FOUND: src/overlay/body.rs (fn colour_combo, same parameters; colour_combo_tests)
- FOUND: 265bf93, a5ae27f
