---
phase: 33-style-table-line-by-line-style-editor
fixed_at: 2026-10-16T18:30:00Z
review_path: .planning/phases/33-style-table-line-by-line-style-editor/33-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 33: Code Review Fix Report

**Fixed at:** 2026-10-16T18:30:00Z
**Source review:** .planning/phases/33-style-table-line-by-line-style-editor/33-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (CR-01, CR-02, WR-01, WR-02, WR-03; Info findings out of scope)
- Fixed: 5
- Skipped: 0

Verification after all fixes: full `cargo test` green (1297 lib tests plus the integration
suites), `cargo fmt --check` clean, `cargo clippy --all-targets` 122 warnings (at the limit,
not above it), `tests/fixtures` unchanged.

## Fixed Issues

### CR-01: An unset style slot cannot take the colour the picker opens on

**Files modified:** `src/skin/colour_picker.rs`, `CLAUDE.md`
**Commit:** e7b4c66 (fix), b1993e3 (rustfmt of the new test)
**Applied fix:** `PickerResult` gained a `picked` flag, set by any quick-pick click, including
the selected tile. In `colour_popup`, `Use colour` or a quick pick on an empty slot
(`current == None`) now sends `PickerEdit::Set` of the working colour, even when that colour
is still the seed. On a set slot, Use colour still closes without an edit. The CLAUDE.md
colour picker note documents the change.
**Tests:** `use_colour_on_an_unset_slot_sets_the_seed` and
`the_selected_quick_pick_on_an_unset_slot_sets_it`.
**Status:** fixed: requires human verification (logic change)

### CR-02: Escape on a 3-token rule colour line rewrites it as 4 tokens

**Files modified:** `src/overlay/widgets.rs`, `src/overlay/body.rs`
**Commit:** 2895bee
**Applied fix:** `colour_chip` records the line's form (`had_alpha`) while the picker is closed
and keeps it while the picker is open. `colour_swatch(ui, rgba, had_alpha)` now returns
`(rgba, had_alpha_at_open)`. `rgba_edit` takes `had_alpha` as a parameter, so every live edit,
Escape included, is encoded with the form the line had when the picker opened. A 3-token line
whose alpha was edited therefore goes back to `1 2 3` on Escape.
**Tests:** `body::escape_after_an_alpha_edit_restores_a_three_token_line` checks that
`WorkingFilter::text()` equals the original source after alpha edit + restore.
`widgets::escape_after_an_alpha_edit_reports_the_opening_form` drives the real chip: open,
click the alpha slider, then Escape.
**Status:** fixed: requires human verification (logic change)

### WR-01: Adding a style collapses it instead of expanding it

**Files modified:** `src/overlay/config_view.rs`, `src/overlay/app.rs`
**Commit:** 5b6c83c
**Applied fix:** `style_draft_row` no longer sets `cfg.expanded`. The new
`ConfigUi::expand` does not toggle, and the `AddStyle` handler now calls it instead of
`set_expanded`. It also drops an armed delete and bumps the epoch, as `set_expanded` did.
**Tests:** `app::adding_a_style_expands_it` runs the handler twice, once with the row not yet
expanded and once with it already expanded. Both cases must end expanded.

### WR-02: Sound switch-back memory loses the custom file through an Alert

**Files modified:** `src/overlay/config_view.rs`, `CLAUDE.md` (the doc line landed in the WR-03 commit)
**Commit:** b98445b
**Applied fix:** The dropdown's memory moved into a pure `switch_sound(choice, current, &mut SoundMemo)`:
- It remembers the sound on a switch to None.
- It remembers the custom file on any switch away from Custom.
- `sound_for_choice` takes a `last_file` and prefers it over `uwu.mp3` when the base has no
  file.

The UI recalls and remembers both values through the existing epoch-keyed `remember`/`recall`.
**Tests:** `switching_back_to_custom_keeps_the_file` covers Custom -> Alert -> Custom,
Custom -> None -> Alert -> Custom, and the never-custom default.
**Status:** fixed: requires human verification (logic change)

### WR-03: Turning Font size off and on again replaces the font with 32

**Files modified:** `src/overlay/config_view.rs`, `CLAUDE.md`
**Commit:** 0ce58b2
**Applied fix:** Turning the font check off now remembers the size. Turning it back on
restores that size, or 32 when nothing was remembered. CLAUDE.md now says that the Font size
re-enable and the custom sound file (WR-02) keep their last values.
**Tests:** `font_off_then_on_restores_the_last_size` (S: 37 -> None -> 37).

---

_Fixed: 2026-10-16T18:30:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
