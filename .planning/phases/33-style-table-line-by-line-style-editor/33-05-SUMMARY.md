---
phase: 33-style-table-line-by-line-style-editor
plan: 05
subsystem: ui
tags: [screenshot, docs, verification, colour-picker]
requires: [33-04]
provides:
  - "Debug-only --expand-style <name> and --picker text|border|background|rule screenshot flags"
  - "Eight viewed verification shots under shots/"
  - "CLAUDE.md and CHANGELOG [Unreleased] describing the Phase 33 table, editor and picker"
affects: []
tech-stack:
  added: []
  patterns:
    - "picker opened in shots via request_open on frames 3-5, before the save at frame 20"
key-files:
  created:
    - .planning/phases/33-style-table-line-by-line-style-editor/shots/*.png
  modified:
    - src/overlay/screenshot.rs
    - src/overlay/app.rs
    - src/skin/colour_picker.rs
    - CLAUDE.md
    - CHANGELOG.md
key-decisions:
  - "Hex field widened 72 -> 88 and R/G/B fields narrowed 44 -> 38 so #RRGGBB is not clipped (found in the first shot)."
requirements-completed: [STY-01, STY-02, STY-03, STY-04, STY-05]
duration: ~25 min
completed: 2026-10-16
---

# Phase 33 Plan 05: Evidence and docs Summary

Debug screenshot flags `--expand-style` / `--picker`, eight viewed shots of the style table, editor and colour picker, one picker rendering fix, and CLAUDE.md / CHANGELOG updated.

## Commits
- c1c0129 feat: `--expand-style` and `--picker` (parse, PICKERS, seeding, request_open; release build verified)
- a965639 fix: colour picker hex field widened
- 6618878 docs: style shots
- f67af17 refactor: collapse picker trigger (clippy back to 122)
- 5e4a8f7 docs: CLAUDE.md and CHANGELOG

## Shots (all viewed with Read; temp home `%TEMP%/ff-shot-33`)
- `styles-table.png` (`--tab config`, no scroll needed): both tables have a header row; labels sit over their columns; Icon column shows game icons (S pink, A purple, B red, C gold, D grey, blank for E/F/unchanged/hidden); built-in arrows greyed and aligned with the custom Order column; no Show/Hide caption. Nit: built-in names are centred in the column while the "Name" header is left-aligned (custom rows use a left text field), so the header sits over the left of the column. Left as is (observation).
- `editor-builtin.png` (S): lines Visibility, Label (preview), Text/Border/Background cards, Font size, Minimap (None/Large/Medium/Small, 11 colours, 12 shapes on one row), Beam (none + 11 colours + Temp), Sound (Custom file + uwu.mp3), Volume 300, Drop sound; no frames. Pass.
- `editor-custom.png` (Loot): same lines with Alert 2 / volume 120; Delete style falls below the window (scrolled list ends at Volume), so it is evidenced by the Plan 04 tests as in Phase 32. Custom names are text fields, Order arrows live.
- `editor-minimap-none.png` (E): Minimap None selected, colour and shape rows absent, Beam none (Temp greyed), Sound None. Pass.
- `picker-style-text.png`: popup inside the window under the Text card; square, hue bar, Hex/R/G/B, nine quick picks plus a None tile, Use colour; no Alpha. First render clipped the hex text (`#FF79C(`) with the R label touching it: FIXED (a965639), re-rendered and viewed: `#FF79C6` fully visible.
- `picker-style-background.png`: popup right-aligned inside the 660pt window, Hex `#282A36`, no Alpha, None tile. Pass (re-rendered after the fix).
- `picker-rule.png` (RULES, rule 1 expanded): Alpha slider with swatch and 255, no None tile, chip with no caption under Set Text Color; popup inside the window and covering the Name/Class rows while open (expected for a popup). Observation: the label "Set Background C" is truncated (pre-existing, not Phase 33).
- `delete-style.png`: Delete style? popup with rule-count and config.json warning (Recycle), red Delete style, Keep style. Pass.

## Deviations
- [Rule 1 - Bug] Hex field clipped in the picker; fixed as above.
- [Rule 3] clippy `collapsible_if` on my picker trigger pushed the count to 123; collapsed to a let-chain (122).

## Gates
`cargo test` 1290 lib tests + integration green; `cargo fmt --all --check` clean; clippy warnings 122; `tests/fixtures` unchanged; release build of `ff` succeeds.
Threat T-33-14: new flags exist only in `#[cfg(debug_assertions)]` code (release build verified). T-33-13: shots ran with FERRISFILTER_HOME/USERPROFILE under `%TEMP%`; nothing outside temp was written.

## 33-VERIFICATION-CHECKLIST
(Use an overlay run with `FERRISFILTER_HOME` and `USERPROFILE` on a temp copy.)
- [ ] STY-01: header and rows line up in both tables at the real 660pt window
- [ ] STY-01: greyed built-in arrows do nothing
- [ ] STY-01: no Show/Hide caption beside the switch
- [ ] STY-02: expand S and a custom style; every line present, no frames
- [ ] STY-02: the 12 shape tiles fit one row
- [ ] STY-02: Minimap None hides colours/shapes; switching back restores them
- [ ] STY-03: picker on Text / Border / Background stays inside the window
- [ ] STY-03: picker for a rule colour near the bottom of the list stays inside the window
- [ ] STY-03: hex `#FF79C6` equals 255 121 198 (both directions)
- [ ] STY-03: Escape reverts a live edit
- [ ] STY-03: opening and closing a picker without changes leaves Apply disabled
- [ ] STY-03: a 3-token rule colour stays 3-token after a change with alpha 255
- [ ] STY-04: renaming a custom style keeps its rules styled
- [ ] STY-04: reorder custom styles (order saved by Apply)
- [ ] STY-04: Delete style at the bottom of a custom editor asks with the rule count and the config.json warning
- [ ] STY-05: Apply writes styles.json and the filter together
- [ ] STY-05: Revert restores styles and closes an open picker
- [ ] STY-05: tier edits behave as before

### Less-texty changes for review
- Show/Hide caption removed from style rows (the switch alone).
- NAME caption removed from the editor; the rename tip moved to the built-in Name tooltip.
- `Not set (game default)` caption replaced by the card's `(none)` state and the picker's None tooltip.
- Rule colour `R G B A` caption moved to the chip tooltip.
- Font size / Volume number fields replaced by slider value text.
- Font size and Drop sound explanations are tooltips.

## Self-Check: PASSED
Commits c1c0129, a965639, 6618878, f67af17, 5e4a8f7 exist; eight PNGs present under shots/.
