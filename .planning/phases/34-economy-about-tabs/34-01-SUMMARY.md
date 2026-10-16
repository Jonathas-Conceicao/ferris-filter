---
phase: 34-economy-about-tabs
plan: 01
subsystem: ui
tags: [egui, ff-art, resvg, about-tab]
requires: []
provides:
  - "assets/ui/ferris-about.png (336x176 orange Ferris, 9 KB)"
  - "Skin::ferris_about texture"
  - "skin::REPO_URL and skin::REPO_LINK_TEXT shared constants"
  - "overlay::about_view::about_tab(ui, skin) plus copy constants"
affects: [34-04]
key-files:
  created: [assets/ui/ferris-about.png, src/overlay/about_view.rs]
  modified: [tools/ff-art/src/main.rs, assets/poe2/manifest.toml, src/skin/textures.rs, src/skin/mod.rs, src/bin/ff-proto.rs, src/overlay/mod.rs]
key-decisions:
  - "ff-art renders the colour Ferris in two passes (measure bbox, rescale) so the crop is exactly 336 px wide"
metrics:
  tasks: 3
  completed: 2026-10-16
---

# Phase 34 Plan 01: ABOUT art and page module Summary

`ff-art ferris` now also renders the orange happy Ferris (original colours, tight crop, 336 px wide) and `overlay::about_view::about_tab` draws the ABOUT page in the locked order, ready for Plan 34-04 to wire.

## Commits
- f317da4: ff-art renders the orange ABOUT Ferris (main.rs, manifest `[ferris_about]`, ferris-about.png)
- 6e14256: ferris_about texture and shared REPO_URL (ff-proto now uses the library constant)
- 795c828: ABOUT page module with 4 tests (copy constants, version text, render at 535x760 and 535x300)

## Deviations from Plan

- The image is 336x176, so it draws about 59pt tall (the spec estimated about 75pt); height follows the artwork aspect as specified.
- Clippy baseline measured with the plan's `grep -c '^warning'` command is 126 (not 122) before and after; the count is unchanged by this plan. The extra lines are per-target summary lines.
- The headless tests use `ctx.run_ui` (this egui version has no `Context::run` taking a ctx closure).
- `ferris-bg.png` and `tests/fixtures` stayed byte-identical; no restore was needed.

## Verification
`cargo test` green (1307 lib tests plus integration), `cargo fmt --all --check` clean, clippy count 126 (unchanged).

## Known Stubs
None. `about_tab` is not yet wired into the app (Plan 34-04 does that).

## Self-Check: PASSED
