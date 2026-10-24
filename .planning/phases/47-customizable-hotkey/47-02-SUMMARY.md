---
phase: 47-customizable-hotkey
plan: 02
subsystem: skin / ff-art
tags: [art, skin, widget, keybind, KEY-01]
requires: []
provides:
  - "assets/poe2/options/keybind_border.png (trimmed 60x60 gold frame, transparent interior)"
  - "assets/poe2/options/keybind_plate.png (trimmed 60x60 greyscale plate)"
  - "Skin::keybind_border, Skin::keybind_plate"
  - "skin::widgets::ff_keybind_button + KEYBIND_HEIGHT (30.0)"
  - "ff-art ImageEntry.key_white manifest option"
affects: [47-04 (Overlay row on the CONFIG tab)]
tech-stack:
  added: []
  patterns: ["extract-time colour-to-alpha keying of a game mask region (ff-art key_white)"]
key-files:
  created:
    - assets/poe2/options/keybind_border.png
    - assets/poe2/options/keybind_plate.png
  modified:
    - tools/ff-art/src/main.rs
    - assets/poe2/manifest.toml
    - src/skin/textures.rs
    - src/skin/widgets.rs
decisions:
  - "Border interior cleared at extract time (ff-art key_white), not at draw time: egui only alpha-blends and the irregular white interior reaches the nine-slice edges"
  - "key_white threshold 32 (not 96): also keys the 32-96 grey fringe, which would otherwise stay opaque grey"
  - "Nine-slice corner margin 20 texels (clusters reach ~14 across, ~22 down; vertically nothing stretches at 30pt)"
metrics:
  duration: "~25 min"
  completed: 2026-10-23
  tasks: 2
  files: 6
---

# Phase 47 Plan 02: Keybind button art and skin widget Summary

The game's own Options keybind button art (`optionsbuttonborder` + `optionsbutton`) is now
extracted by ff-art. A new `key_white` manifest option flood-fills the border's white mask
interior and makes it transparent. `ff_keybind_button` draws the gold-tinted plate under that
frame with a centred SmallCaps caption, in rest, lit, pressed and disabled states.

## What was built

- **ff-art `key_white = [seed_x, seed_y, threshold]`** (`tools/ff-art/src/main.rs`): one field,
  one function and one call after the `clear` loop. It runs a 4-connected flood fill with an
  explicit stack from the seed through pixels with `min(r,g,b) >= threshold` and alpha > 0, and
  applies colour-to-alpha against white (`a' = a(1-w)`, colour un-premultiplied toward black).
  A seed outside the image is skipped with a warning (T-47-05). No new dependency.
- **Manifest**: the `key_white` line is added to the field comment block. Two `[[image]]`
  entries are appended at the very end, after `[ferris_icon]`: the border with `trim` and
  `key_white = [30, 30, 32]`, the plate with `trim`. There is no `prefer_4k` (the game has no 4K
  sibling).
- **Art check**: both PNGs are 60x60. Dumped pixel values show the border interior fully
  transparent at the centre, with a black, partly transparent vignette toward the gold line.
  There is no pale ring, and the frame and corner pixels are untouched (alpha 254/255). The full
  extract changed no other committed asset.
- **Skin**: `keybind_border` / `keybind_plate` textures, plus `ff_keybind_button(ui, skin, label,
  caption, caption_color, width, lit, enabled)` and `KEYBIND_HEIGHT = 30.0`:
  - The plate is drawn first (`paint_nine`, centre included), then the border (centre skipped).
    Both use `KEYBIND_CORNER_TEXELS = 20` margins at scale `30 / texture height`.
  - Plate tint is `pal::GOLD` at rest and `#E6C882` when hovered, focused, pressed or `lit`.
    Disabled, it is GOLD at alpha 90 and the border is drawn at white alpha 90.
  - The caption is SmallCaps 17, centred, cut with `…` past `width - 24`, and moves down 1pt
    while pressed.
  - `ff_reload_button` is untouched.

## Commits

- 1150931 feat(47-02): extract the Options keybind button art with ff-art
- 9147b53 test(47-02): add failing tests for ff_keybind_button (RED)
- 4c02e6f feat(47-02): add ff_keybind_button and the keybind skin textures (GREEN)

## Verification

- `cargo test --locked`: green (lib 1745 passed, including 3 new keybind tests); `tests/fixtures`
  unchanged.
- `cargo build --locked` (root) builds without ff-art.
- `git status` after the full `cargo ff-art extract` showed only the two new PNGs plus the
  manifest and ff-art source.

## Deviations from Plan

- **Threshold tuned downward (32) rather than the suggested 96/128/160 range.** Higher thresholds
  leave the darker grey fringe opaque (a lighter band inside the frame). Gold has a low blue
  channel, so `min(r,g,b)` stays small on the frame and the lower threshold does not bleed into
  it. This is recorded in AUTONOMOUS-V072-DEFERRED.md.
- **Tests use the module's existing `setup()` (full `Skin::load`)** rather than `load_light`, to
  match the neighbouring widget tests. The behaviour asserted is the same.
- I added a temporary integration test to dump pixel values, because Python is not installed
  and PowerShell is blocked. It was deleted before the commit and was never committed.

## TDD Gate Compliance

RED (`test(47-02)` 9147b53) precedes GREEN (`feat(47-02)` 4c02e6f). No refactor commit was needed.

## Notes for Plan 04

- The widget contract matches the plan exactly. `label` is the accessible name (for example
  `Show / hide overlay: Ctrl+Shift+Z`).
- `cargo fmt --check` already reports diffs in `src/skin/editor.rs` and in the `ff_reload_button`
  tests in `widgets.rs`. They come from earlier work, are out of scope, and were left alone.

## Self-Check: PASSED

- FOUND: assets/poe2/options/keybind_border.png, assets/poe2/options/keybind_plate.png
- FOUND commits: 1150931, 9147b53, 4c02e6f
