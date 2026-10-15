---
phase: 20-full-grammar-block-editing
plan: 08
subsystem: overlay-body
tags: [rust, egui, overlay, actions, styles, editor]

requires:
  - phase: 20-07
    provides: body.rs card anatomy, raw_row, picker_popup
  - phase: 20-06
    provides: StyleState, style_names, style, ApplyStyle/RemoveStyle
provides:
  - overlay::body::{actions_group, style_group}
  - pure helpers rgba_edit, font_edit, minimap_edit, effect_edit, sound_edit, custom_sound_edit, optional_bool_edit, style_owned, added_actions, font_hint, pick_action
affects: [20-09]

key-files:
  modified: [src/overlay/body.rs, src/overlay/rule_list.rs]

key-decisions:
  - "Pure helpers take a leading BlockId (as in 20-07); custom_sound_edit returns Result<Option<RowAction>, String> with the quote/control hint"
  - "optional_bool_edit treats a bare line and `True` as on; a bare line is only written when toggled off (`False`)"
  - "Sound volume shown as 100 when the line has none, but the volume is only written once touched"
  - "On a Known-styled rule every action card is locked: style-owned ones get a `from style` badge, others a `locked` badge with an explanatory tooltip (any edit would be overwritten on Apply)"
  - "Opaque lines stay in CONDITIONS (as in 20-07); ACTIONS shows only Action entries; Continue stays on the existing checkbox, now inside ACTIONS"
  - "Locked rendering is read-only text of the line value in DISABLED_TEXT (no disabled swatch)"
  - "Locked disposition note reworded to `The style decides Show or Hide. Remove the style to choose here.` so the old `Remove the style tag...` copy is gone"

requirements-completed: []
requirements-progress: [EDIT-02, EDIT-06, STYLE-03]

completed: 2026-10-13
---

# Phase 20 Plan 08: Actions editor and STYLE group Summary

**Every action keyword is an editable card (colour swatch with alpha, font slider 1-45, minimap and beam dropdowns, sound sliders, custom sound file, drop-sound toggles) with a grouped `+ action` picker, and a STYLE group applies, shows and removes named styles with style-owned actions locked.**

## Accomplishments
- `actions_group`: ACTIONS caption + `+ action` (hidden on a Known-styled rule; already-present actions disabled with `(added)`), disposition selector and Continue first, then one card per action line, or `No actions: matching items keep the game's look.`
- Controls: `colour_swatch` (3-token form preserved while alpha stays 255), font slider/number plus `Readable in game: 18-45` hint outside 18-45, MinimapIcon size/colour/shape combos with `Hide icon` (-1) and colour dots, PlayEffect colour (plus None) and `Temporary` toggle, sound id and volume slider pairs, custom sound file (commit on Enter/blur) and volume with `The file must be in the game folder.`, `On` toggle for drop-sound lines; unrepresentable values fall back to the raw value field.
- `style_group`: unstyled dropdown (`(none)` plus every style with a text-colour swatch) and help text; Known banner `Style: <name>` with explanation, change dropdown and `Remove style`; Unknown banner `Unknown style "<name>"` in WARM_ACCENT_TEXT with `Pick a style…`; Unchecked banner when styles.json cannot be read. Actions of Unknown/Unchecked rules stay editable.
- `expanded_body` order is NAME, STYLE, CONDITIONS, ACTIONS, Duplicate/Delete; 16px before ACTIONS, 8px elsewhere; `Margin::symmetric(10, 8)` snapped to `Margin::same(8)`; old ACTION block and its style note removed.
- Edits are written only on real changes; idle frames over all action shapes and all style states emit no RowAction (colour swatch with alpha 128 included).

## Task Commits
1. Task 1 + body.rs half of Task 2 (action cards, picker, STYLE group, helpers, tests): 11f7d16
2. rule_list.rs reorder and rewritten style-tagged idle test: b43ff70

## Deviations from Plan
- Tasks 1 and 2 both live mostly in body.rs and were built together, so they were committed as one body.rs commit plus one rule_list.rs commit rather than strictly per task.
- Helper signatures carry a leading `BlockId` (consistent with 20-07; `RowAction` needs it).
- Short Phase 19 note text for locked disposition reworded (see decisions).
- The row-header `add_space(6.0)` in the collapsed row (outside the expanded body) was left unchanged.

## Known Stubs
None.

## Threat Flags
None. T-20-27 (custom_sound_edit refuses quotes/control characters via `grammar::encode`), T-20-28 (only `style_names()` offered; model re-checks), T-20-29 (locked in view, refused in model), T-20-30 (idle translucent colour test) mitigated.

## Verification
Full `cargo test` green (631 lib tests plus integration), `cargo fmt --check` clean, `cargo build --bins` warning-free, `tests/fixtures` and `tests/snapshots` unchanged.

## Self-Check: PASSED
