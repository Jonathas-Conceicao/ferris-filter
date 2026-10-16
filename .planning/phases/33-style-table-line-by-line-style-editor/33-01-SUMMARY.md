---
phase: 33-style-table-line-by-line-style-editor
plan: 01
subsystem: ui
tags: [egui, skin, colour-picker, popup]
requires: []
provides:
  - "skin::colour_picker: PickerMode, PickerResult, PickerEdit, ff_colour_picker, colour_popup, request_open (debug)"
  - "skin::minimap::DRACULA quick-pick table"
  - "skin::popup::paint_box_behind shared popup painter"
affects: [33-02, 33-03, 33-05]
tech-stack:
  added: []
  patterns:
    - "gamma-space HSV edited behind a linear Hsva interface, remembered in egui temp memory"
    - "test-only thread-local probe for popup widget rects"
key-files:
  created:
    - src/skin/colour_picker.rs
  modified:
    - src/skin/mod.rs
    - src/skin/minimap.rs
    - src/skin/popup.rs
    - src/overlay/body.rs
key-decisions:
  - "The picker keeps the Hsva interface (WR-04) but draws and edits in gamma-space HSV, so the brightness axis looks even; the gamma triple is remembered per picker and used only while it still matches the RGB (hue survives greys)."
  - "Hex field is 72pt with no visible 'Hex' label (accessible name and tooltip only) so Hex + R/G/B fit one 268pt line."
requirements-completed: [STY-03]
duration: 40min
completed: 2026-10-16
---

# Phase 33 Plan 01: Skinned colour picker Summary

A game-framed, live-editing colour picker (square, hue bar, Hex and R/G/B, optional alpha, nine Dracula quick picks, optional None, Use colour) is in `src/skin/colour_picker.rs`, ready for Plan 03 to wire.

## Tasks

1. **Pure helpers, Dracula table, shared painter** (c33a8b9): `hsva_of`, `parse_hex`, `hex_of`, `parse_channel`, `sv_at`, `hue_at` with unit tests; `minimap::DRACULA`; `paint_box_behind` moved to `skin::popup` and called from `picker_popup`.
2. **Picker widget and popup host** (fa84593): `ff_colour_picker` and `colour_popup`. Live edits; Escape returns the opening value; click outside, the trigger or Use colour keep the value; no edit unless the unmultiplied result differs from `current`; opening a picker closes any other; `request_open` (debug only) for Plan 05 screenshots. 11 headless tests cover idle/open-idle for alpha 255/128/0, quick pick keeping alpha, None, Escape (set and unset slot), mode visibility, square, Use colour.

## Deviations from Plan

**1. [Rule 1 - Layout arithmetic] Hex label dropped, Hex field 72pt.** The plan's 80pt Hex + label + three labelled 44pt fields cannot fit 268pt on one line (about 299pt). Hex now has no visible caption (the `#` makes it self-evident; tooltip and accessible name `Hex`), 72pt wide; the line is 264pt. Plan 05's visual walk should confirm.

**2. [Rule 2 - Correctness] Gamma-space HSV.** egui's `Hsva` is linear, which makes the brightness axis lopsided. Interaction and painting use gamma HSV; `Hsva` stays the working value, so callers see no difference.

No other deviations. `src/overlay/widgets.rs` was not touched (Plan 03).

## Gates

`cargo test` 1267 passed, `cargo fmt --all --check` clean, clippy warnings 122 (<= 122), `tests/fixtures` unchanged.

## Known Stubs

None.

## Threat Flags

None. T-33-01 mitigated (strict `parse_hex`/`parse_channel`, tested); T-33-03 `request_open` is `cfg(debug_assertions)`.

## Self-Check: PASSED

src/skin/colour_picker.rs exists; commits c33a8b9 and fa84593 present.
