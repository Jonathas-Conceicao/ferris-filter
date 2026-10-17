---
quick_id: 261016-1de
status: complete
date: 2026-10-16
commits: f64cd42..(this docs commit)
---

# Summary: PoE2 prototype round 2

## Built
1. **Tabs** (`skin::widgets::tab_layout` + `ff_tabs`): each stone is left cap + flat middle +
   right cap; the caption is centred on the flat top (middle of the left curve to just past the
   start of the slanted tail), stones overlap by 30% of the tail like the game's Options tabs,
   the selected stone is drawn last, clicks land on flat bodies only. Four tabs fit easily.
   Unit test: centred row, constant overlap, captions left of the stone centre.
2. **Style editor** (CONFIG): styles come from `styles-default.json` (`include_str!`, parsed in
   memory in file order, never written), grouped Built-in / Custom (`+ Add style` on the Custom
   heading). A row (show check, name, real minimap icon, label chip, arrows for customs) expands
   in place, one at a time, into cards: Visibility (Show/Hide segmented switch on button art),
   Label (preview in Fontin SmallCaps at the style's font size, Text/Border/Background cards --
   click toggles none / last colour, no picker), font size (check + slider), Minimap icon (current
   icon, None/Large/Medium/Small pills, 11 colour swatches, 12 shape tiles of the real game icons
   in the chosen colour), Beam (none + 11 swatches + Temp), Sound (None / Alert sound 1-16 /
   custom file + volume). Customs: name field, reorder arrows, Delete with inline confirm.
   New skin module `skin::editor` holds the pieces; `skin::minimap` the colours/shapes/UVs.
3. **ABOUT tab**: name, `CARGO_PKG_VERSION`, one-line description, repo link
   (github.com/Jonathas-Conceicao/poe2-myfilter), three credit rows, and the disclaimer pinned to
   the bottom.
4. **Status line** (`ff_status` + `StatusKind`, palette `OK` / `WARN` / `ERROR`): one row, cut
   with an ellipsis to the space left of the buttons, full text in a tooltip only when cut.
   REVERT sets a long ok, RELOAD a long error, APPLY a long warning; `--status` presets one.
5. **Minimap atlas**: see below.

## Minimap sheet mapping
`art/2dart/minimap/player.dds` is 896x4928 = 14 columns of 64px cells (cell n at
`(n % 14, n / 14)`). Filter icons come as Large/Medium/Small triplets (n, n+1, n+2):
- Circle, Diamond, Hexagon, Square, Star, Triangle: Blue, Green, Brown, Red, White, Yellow from
  cell 46 (18 cells per shape); Cyan, Grey, Orange, Pink, Purple from cell 212 (15 per shape).
- Cross, Moon, Raindrop, Kite, Pentagon, UpsideDownHouse: all 11 colours from cell 302 (33 per
  shape; Blue, Green, Brown, Red, White, Yellow, Cyan, Grey, Orange, Pink, Purple).

A new `[[atlas]]` manifest entry lists the source cell of every output cell; `ff-art extract`
packs the Large cells into `assets/poe2/minimap/icons.png`: 768x704 (12 shapes x 11 colours),
**513,764 bytes** (was 4,734,324 for the whole sheet, now `git rm`'d). Medium/Small are drawn
by scaling (0.78 / 0.58). `ff-art dump minimap/player <scratch>` still dumps the whole sheet.

## Screenshots (this folder)
`ff-proto-rules.png`, `ff-proto-config.png`, `ff-proto-economy.png`, `ff-proto-about.png`,
`ff-proto-style-editor-S.png` (built-in, colours + Medium pink circle),
`ff-proto-style-editor-kite-status-warning.png` (custom style, Small orange kite selected, long
warning cut with an ellipsis), `ff-proto-status-error.png`, `ff-proto-tabs-zoom.png` (2x).

## Verification
- `cargo test`: all pass (skin: 20 unit tests, 6 new). `cargo fmt --check` clean (main crate and
  ff-art); no clippy warnings in `src/skin`, `src/bin/ff-proto.rs` or ff-art.
- `src/overlay/`, `src/bin/ff.rs`, `tests/` and fixtures untouched.

## Follow-up: tab strip seating (user feedback)
The tab strip was a hard-coded 26pt band starting 13pt below the frame's top. In frame
texels (scale 0.5) that is rows 26-78: the stones started above the frame's outer gold line
(rows 41-47), running into the header's lower edge, and ended in the middle of the dark band,
floating above the light stone strip (row 103) instead of resting on it. New
`skin::widgets::tab_strip(&FrameLayout)` derives the strip from the frame art
(`TAB_STRIP_TEXELS = (48, 103)`): stones are drawn at their full height (27.5pt) between the
gold line and the light strip, as in the game's Options window (`poe2_print009.png`), with no
clip rect on them, selected or not. Unit-tested in `frame_layout_keeps_content_inside_the_gold_line`.
All screenshots in this folder were re-taken after the fix.

## Known gaps
- No colour picker (by request); swatch cards only toggle none / colour.
- The status tooltip and hover states were checked in code and headless tests, not in a
  screenshot (screenshots have no pointer).
- Item-label preview is an approximation of the game's label (font-size scale 0.5 pt per unit,
  default text colour = rare yellow).
