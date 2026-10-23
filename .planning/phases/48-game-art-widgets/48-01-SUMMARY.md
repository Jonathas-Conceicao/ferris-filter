---
phase: 48-game-art-widgets
plan: 01
subsystem: skin
tags: [art, egui, ff-art, skin]
requires: []
provides:
  - 14 extracted game-art PNGs (toggle plates, stone bar, clear X, magnifier)
  - Skin fields toggle_collapse, toggle_expand, bar, clear, search
  - slice::box_uv and slice::paint_visible
affects: [48-02, 48-03]
tech-stack:
  added: []
  patterns: ["paint art by its visible plate (paint_visible), never by the texture rect"]
key-files:
  created:
    - assets/poe2/common/toggle_collapse_{normal,hover,pressed}.png
    - assets/poe2/common/toggle_expand_{normal,hover,pressed}.png
    - assets/poe2/common/bar_{left,middle,right}.png
    - assets/poe2/common/clear_{normal,hover,pressed}.png
    - assets/poe2/common/search_{normal,hover}.png
  modified:
    - assets/poe2/manifest.toml
    - src/skin/textures.rs
    - src/skin/slice.rs
    - src/skin/widgets.rs
key-decisions:
  - "box_uv takes an exclusive texel box [x0, y0, x1, y1), matching the plan's [0,0,84,84] / [0,0,54,54] constants"
metrics:
  tasks: 3
  completed: 2026-10-23
requirements: [ART-04, ART-01, ART-02, ART-03]
---

# Phase 48 Plan 01: Game art foundation and ART-04 offset fix Summary

Extracted the four game-art pieces with ff-art via 14 cropped manifest entries, embedded them in `Skin` with decode-size tests, and fixed the reload-button and round-check offsets with a new `paint_visible` helper.

## Commits

- 0f7fc75: feat(48-01): extract toggle, bar, clear and search art
- 2e907b7: feat(48-01): embed new art in Skin, add box_uv and paint_visible
- fix commit (HEAD): fix(48-01): paint reload button and check by visible plate

## What changed

- ff-art run changed no existing asset; git status listed only manifest.toml and the 14 new PNGs.
- `Skin` gained the five fields; `every_embedded_texture_decodes` asserts all new sizes (54x64, 132/111/151x75, 36x36, 48x49).
- `ff_reload_button` paints the opaque 84x84 box of the icon-button texture into the full ROW_HEIGHT rect; glyph placement moved into `reload_glyph_rect` (tested). `PLATE_VISIBLE` removed. The Filter refresh and Economy league refresh share this widget, so both are fixed with no call-site edits.
- `ff_check` paints its visible 54x54 plate into the 22x22 rect.
- `cargo test --locked` is green (1708 lib tests plus integration suites); `tests/fixtures` untouched; no new dependencies.

## Deviations from Plan

None. The plan's tdd ordering for the two TDD tasks was followed loosely: tests and code were committed together per task rather than as separate RED/GREEN commits.

## Known Stubs

None. The new Skin fields are not yet painted anywhere; plans 02 and 03 use them.

## Self-Check: PASSED

All 14 PNGs exist, the three commits exist, and the full test suite passes.
