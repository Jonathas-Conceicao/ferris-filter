---
phase: 37-game-folder-resolver-picker
plan: 02
subsystem: skin
tags: [rust, egui, skin, ff-art, refresh-button]
requires: []
provides:
  - Skin.refresh ([TextureHandle; 3])
  - widgets::ff_icon_button
  - assets/poe2/common/refresh_{normal,hover,down}.png
affects: [37-06]
key-files:
  created: [assets/poe2/common/refresh_normal.png, assets/poe2/common/refresh_hover.png, assets/poe2/common/refresh_down.png]
  modified: [assets/poe2/manifest.toml, src/skin/textures.rs, src/skin/widgets.rs]
decisions:
  - "Refresh art used as the whole button (not a glyph on skin.button), per orchestrator decision 2"
  - "Manifest entries have no trim: with trim the three states came out 60x60 / 60x59 / 59x60; untrimmed all are 68x68"
metrics:
  tasks: 2
  files: 6
  completed: 2026-10-18
---

# Phase 37 Plan 02: Refresh button art and icon button Summary

The game's web-toolbar refresh art (round arrow on a square button) is extracted by ff-art, embedded in the skin, and drawn by a new `ff_icon_button` widget.

## Art vs fallback

**ART USED (no fallback).** `uiimages/common/webtoolbarrefresh{normal,hover,down}.dds` were found in the Steam install and extracted (4K variants, 68x68 each, square). Viewed `refresh_normal.png`: a round arrow on a square bronze button. Plan 06 should use `ff_icon_button(ui, &skin.refresh, "Refresh", enabled)`, not the R text button.

## Commits

- 20b8e84: feat(37-02): extract the game's refresh button art
- 1cffa12: feat(37-02): Skin.refresh textures and ff_icon_button

## What was built

- `manifest.toml`: three `[[image]]` entries after the close button. `trim` was left off because trimming made the states differ by a pixel (60x60, 60x59, 59x60).
- `ff-art` re-extracted everything; `git status assets` showed only the manifest and the three new PNGs (T-37-04 held).
- `Skin.refresh` loaded with `tex!` (headless `load_light` follows automatically through `load`).
- `ff_icon_button(ui, textures, label, enabled)`: ROW_HEIGHT x ROW_HEIGHT, click sense only when enabled (also honours a disabled `Ui`), state 2 pressed, 1 hover or focus, disabled is the dimmed normal art (alpha 90 tint), `WidgetInfo::labeled(Button, enabled, label)`.
- Tests: `icon_button_is_row_height_square`, `icon_button_clicks_only_when_enabled`, `icon_button_is_labelled_and_reports_enabled`.

## Verification

- `cargo test --lib skin::`: 78 passed (includes `every_embedded_texture_decodes`, so the new PNGs decode). Run with FERRISFILTER_HOME / FERRISFILTER_GAME_DIR under a temp dir.
- No separate `cargo build --bin ff` run; the lib and widgets compile in the test build, and no bin code changed.

## Deviations from Plan

- Dropped `trim = true` on the three entries (plan allowed removing trim when the result is not square). No other deviations.

## Human verification

- Visually check the hover and pressed states of the refresh art once plan 06 places the button (the assets were only inspected in the normal state).

## For the Release Review

- New committed art `assets/poe2/common/refresh_*.png` (GGG art; the README Credits already cover `assets/poe2/`).
- CLAUDE.md `src/skin/` row could mention `ff_icon_button` and the `assets/poe2/common/` refresh art; the CHANGELOG entry belongs with the phase 37 refresh button.

## Self-Check: PASSED

Files and both commits exist.
