---
phase: 33-style-table-line-by-line-style-editor
plan: 03
subsystem: ui
tags: [egui, skin, colour-picker, rules-editor]
requires: [33-01, 33-02]
provides:
  - "widgets::colour_swatch / colour_swatch_rgb as a 34x22 chip over the skinned picker (caption is the tooltip)"
  - "widgets::colour_slot + SlotResult: opaque picker with a None tile for the CONFIG style cards"
  - "skin::editor::ff_colour_card_ex with the 2pt HEADING open ring"
affects: [33-04, 33-05]
tech-stack:
  added: []
  patterns:
    - "open-state remembered in egui temp data so the trigger can draw its ring before the popup is shown"
key-files:
  created: []
  modified:
    - src/overlay/widgets.rs
    - src/skin/editor.rs
    - src/skin/colour_picker.rs
key-decisions:
  - "colour_swatch_rgb stays (config_view still uses it until Plan 04 replaces the old style editor)."
  - "Test-only probe accessors (probes_clear, probe_rect) added to colour_picker so widget tests can find picker pieces."
requirements-completed: [STY-03]
duration: 40min
completed: 2026-10-16
---

# Phase 33 Plan 03: Picker wiring Summary

Rule colour actions now open the game-skinned picker (with Alpha, no None tile) from a caption-less 34x22 chip; `colour_slot` gives the style cards an opaque, None-capable picker entry point for Plan 04.

## Tasks

1. **RULES colour chip** (685fb5a): `colour_swatch` / `colour_swatch_rgb` rebuilt over `colour_popup`; egui's `color_edit_button_hsva`, the local `hsva_of`, `rgb_or_unset`, `chip_font_size` and `style_preview_chip` (plus tests) removed. `body.rs` untouched; `rgba_edit` keeps the 3-token form.
2. **colour_slot and open ring** (32f78ec): `ff_colour_card_ex(.., open)`; `ff_colour_card` delegates (ff-proto unchanged and builds); `colour_slot` with accessible name `<label> colour`.

## Deviations from Plan

None - plan executed as written. `colour_swatch_rgb` was kept because `config_view.rs` still calls it.

## Gates

`cargo test` 1274 lib tests plus integration green; `cargo fmt --all --check` clean; clippy warnings 122 (<= 122); `tests/fixtures` unchanged; `ff-proto` builds; no `color_edit_button_hsva` left.

## Known Stubs

None. `colour_slot` has no caller until Plan 04 (intentional).

## Threat Flags

None. T-33-07 covered by tests (open and idle frames emit nothing, 3-token lines unchanged); T-33-08 channels are `u8` end to end.

## Self-Check: PASSED

Files modified present; commits 685fb5a and 32f78ec exist.
