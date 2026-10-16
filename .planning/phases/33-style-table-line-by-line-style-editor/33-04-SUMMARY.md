---
phase: 33-style-table-line-by-line-style-editor
plan: 04
subsystem: ui
tags: [egui, skin, style-editor, config-tab]
requires: [33-02, 33-03]
provides:
  - "Frame-less line-by-line style editor on ff_field_row (Visibility, Label, colour cards, Font size, Minimap, Beam, Sound, Volume, Drop sound, Delete style)"
  - "Switch-back memory for Minimap / Beam / Sound None, keyed by the ConfigUi epoch"
  - "ConfigUi::reset_for_new_styles bumps the epoch (Revert orphans open pickers and memory)"
affects: [33-05]
tech-stack:
  added: []
  patterns:
    - "one id scope per editor line (push_id) so auto ids never clash across lines"
    - "pure helpers (sound_for_choice, safe_file_name) for logic behind widgets that are hard to drive headless"
key-files:
  created: []
  modified:
    - src/overlay/config_view.rs
    - src/overlay/app.rs
key-decisions:
  - "Tile sizes adapt to the control column (14-24pt) so 12 shapes sit in one row even at the 296pt headless width."
  - "Font slider is given the remaining row width explicitly (ff_slider reads available_width, which a wrapping field row does not shrink)."
requirements-completed: [STY-02, STY-03, STY-04, STY-05]
duration: 75min
completed: 2026-10-16
---

# Phase 33 Plan 04: Line-by-line style editor Summary

The expanded style row is now a stack of zebra `ff_field_row` lines with no frame or caption text, using Plan 03's skinned `colour_slot` for Text / Border / Background; every control still emits only `ConfigAction::EditStyle`.

## Tasks

1. **Editor lines** (5017ed1): `style_editor` rewritten; Frame wrapper removed from `style_row`; Visibility segmented, Label preview, three colour cards, Font size check + 1-45 slider, Drop sound check, load-error line, and `Delete style` (custom only, last line, arms the existing popup). The Minimap / Beam / Sound / Volume lines were written in the same pass, so this commit already contains them.
2. **Minimap, Beam, Sound, Volume** (057bbda): tests for pills, 11 swatches, 12 shape tiles in one row, beam none + colours + Temp (disabled without a beam), sound choices, volume and custom-file lines; `sound_for_choice` / `safe_file_name` extracted and tested; session memory proven for Minimap None and back.
3. **Revert orphans pickers; STY-05** (dba1dd8): `reset_for_new_styles` bumps `epoch`; app test `style_editor_edits_stay_in_memory_until_apply_and_revert_restores` shows edits make CONFIG dirty, leave `styles.json` bytes unchanged, and Revert restores the style.

## Deviations from Plan

**1. [Process] Tasks 1 and 2 code landed in one commit.** The editor was rewritten as one unit, so commit 1 holds all helper code and commit 2 adds the Task 2 tests plus the pure-helper extraction.

**2. [Rule 1 - Bug] Font slider wrapped onto a second row.** `ff_slider` sizes itself from `available_width()`, which stays the full control width in a wrapping field row. Fixed by allocating the remaining width explicitly.

**3. [Rule 1 - Bug] Beam row overflowed the control column.** Minimum tile size lowered from 16 to 14pt so 12 tiles plus Temp fit.

**4. Helper named `ff_field_row_line`** (wraps `ff_field_row` plus a per-line `push_id`) so the plan's grep for `ff_field_row` in `style_editor` holds.

**5. Frame-fill test** uses a height threshold (two rows) because `theme::ROW_BASE` equals `pal::ROW_EVEN`, the zebra ground colour. The test renders `kite`, since `S`'s Sound line legitimately wraps (dropdown plus file field) at the 296pt headless width.

## Gates

`cargo test` green (1290 lib tests plus integration); `cargo fmt --all --check` clean; clippy warnings 122 (<= 122); `tests/fixtures` unchanged. `config_state.rs` untouched; `NOT_SET`, `WHOLE_NUMBER` and the old `segmented` helper removed.

## Known Stubs

None.

## Threat Flags

None. T-33-09: a custom file name with quotes or control characters (or an empty one) is never emitted, shows a red outline and tooltip, and `StyleSet::validate` still guards Apply. T-33-10: the view only emits `EditStyle`; the app test checks no file changes before Apply. T-33-11: font via the 1-45 slider and the `StyleEdit::Font` u8 check, volume via `u16::try_from`. T-33-12: Delete only arms the existing popup.

## Self-Check: PASSED

Modified files present; commits 5017ed1, 057bbda and dba1dd8 exist.
