---
phase: quick-261017-2rf
plan: 01
status: complete
subsystem: overlay RULES list, ECONOMY tab, style editor
tags: [economy, read-only, tiers, style-editor, uat]
key-files:
  modified:
    - src/overlay/rule_list.rs
    - src/overlay/rules.rs
    - src/overlay/app.rs
    - src/overlay/screenshot.rs
    - src/overlay/widgets.rs
    - src/overlay/shell.rs
    - src/overlay/config_view.rs
    - src/overlay/style_lines.rs
    - src/overlay/body.rs
    - src/skin/widgets.rs
    - src/skin/editor.rs
    - tests/ui/test_ff.filter
    - tests/ui/README.md
    - CLAUDE.md
    - CHANGELOG.md
decisions:
  - Economy rows share ListUi::expanded with user rules (one open row); the body runs in a disabled Ui (egui gives disabled widgets no clicks, focus, keys or scrolling) against a ListUi clone and a dropped action list; the model guard T-32-13 stays
  - Disabled body fade 0.6 (egui default 0.5 read too faint); skin buttons honour ui.is_enabled()
  - string_list shows every entry when disabled (a disabled scroll area cannot scroll)
  - Display name const ECONOMY_DISPLAY_NAME = "FF Economy Section" (RULES header, ECONOMY tab heading, locked status); BEGIN_LINE and the settings economy key unchanged
  - Tier box 88pt at the value column (CONTROL_WIDTH from the right), 10pt gap measured from the field frame (ff_text_field leaves the cursor at its inner edit)
  - None colour card falls back to the bare dim label when "<Label> (none)" does not fit, full text on hover (Border and Background at 660pt)
  - MinimapIcon -1 hint order - value, hint, remembered, White Circle
metrics:
  completed: 2026-10-17
  commits: 6
---

# Quick 261017-2rf: browsable read-only economy rules, narrower tier boxes, style-editor nits

Run inline by the orchestrating agent (no separate planner/executor spawn): plan, code, tests,
screenshots, review and one fix pass.

## Commits

| Commit | Message |
|--------|---------|
| 95145bf | feat(overlay): economy rules expand read-only; header reads FF Economy Section |
| 5c4a478 | fix(overlay): narrower tier boxes at the value column with a gap before the unit |
| c3fbf3e | fix(style-editor): framed / tiles, fitting none captions, MinimapIcon -1 hint |
| 1211470 | docs(quick-261017-2rf): read-only economy rules, tier boxes, style editor nits |
| d0b381e | fix(quick-261017-2rf): WR-01..03 read-only tooltips and the MinimapIcon -1 hint |
| ef0e534 | test(quick-261017-2rf): WR-04/05 economy browse tests that can fail |

## What was built

1. **Economy rules browsable, read-only.** A click on an economy row expands it into the user-rule
   body (Name when present, Conditions, Style) in a disabled `Ui`; no check, arrows or Section
   above / Duplicate / Delete; quoted lists uncapped; search still finds them. Screenshot flag
   `--expand-economy n`.
2. **FF Economy Section** in the RULES header, the ECONOMY tab heading and the locked status; file
   banner unchanged.
3. **Tier boxes** 88pt, aligned with the value column, 10pt gap before the unit.
4. **Style-editor nits:** framed `/` tiles; fitting none captions; `MinimapIcon -1 <c> <s>` hint
   (IN-04 of 0wl).

## Tests

New: `browsing_economy_rules_changes_nothing` (every interactive widget in both expanded bodies is
disabled, clicks on each and keys change no list state, open no popup, take no body focus, emit no
action; mutation-checked by removing the disable), `economy_rule_header_click_collapses_and_search_still_finds_it`,
`economy_header_is_locked_and_its_rows_only_expand`, `browsing_economy_rules_then_apply_writes_the_same_bytes`
(through `show_list`, Apply bytes equal with and without browsing), display-name/banner assertions,
`tier_boxes_are_narrow_and_sit_at_the_value_column`, `an_unselected_none_tile_has_a_visible_frame`,
`a_none_card_caption_drops_none_when_it_does_not_fit`, `leaving_a_hidden_icon_keeps_the_colour_and_shape_it_names`,
hint decoding cases, screenshot flag parsing.

## Verification

`cargo test` 1497 passed; `cargo fmt --check` clean; clippy 126 (baseline 126); `tests/fixtures`
untouched; committed blobs LF.

## Review

0 critical, 5 warnings (all fixed), 7 info (one fixed); see `261017-2rf-REVIEW.md` and
`261017-2rf-REVIEW-FIX.md`.

## Screenshots (`shots/`)

`rules-econ1.png` (FF Economy Section open, rule 1 expanded), `rules-econ1-full-list.png` (all ten
bases, disabled controls), `rules-econ2.png` (rule 2 whole body, no buttons), `economy-tiers.png`,
`config-kite.png` (framed `/` tiles, Background caption), `rule18-hint.png` (shapes red under `/`).

## Observations

- `ff_text_field_ex` leaves the layout cursor after its inner edit, not its frame, so a widget
  placed right after any text field touches it (the tier unit had 0pt gap; the list "+ Add" button
  touches its field). Fixed locally for tiers only.
- Screenshot mode scrolls an expanded row near the bottom of the view; `--search` brings it up.
