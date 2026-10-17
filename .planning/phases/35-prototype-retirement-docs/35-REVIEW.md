---
phase: 35-prototype-retirement-docs
reviewed: 2026-10-17T00:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - CHANGELOG.md
  - CLAUDE.md
  - Cargo.lock
  - Cargo.toml
  - README.md
  - assets/ASSETS.md
  - src/overlay/apply.rs
  - src/overlay/body.rs
  - src/overlay/config_view.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/shell.rs
  - src/overlay/widgets.rs
  - src/skin/fonts.rs
  - src/skin/mod.rs
  - src/skin/palette.rs
findings:
  critical: 0
  warning: 0
  info: 2
  total: 2
status: issues_found
---

# Phase 35: Code Review Report

**Depth:** standard. Also deleted: `src/bin/ff-proto.rs`, `src/overlay/theme.rs`, `assets/Cinzel-*`.

## Summary

No functional regressions found.

- **Theme-to-skin value mapping.** Every constant matches. Checked against `theme.rs` at 64fdedc:
  - LABEL_TAN -> `pal::LABEL`, VALUE_TEXT -> `pal::VALUE`, DISABLED_TEXT -> `pal::DISABLED`.
  - WARM_ACCENT_TEXT -> `pal::WARN`, SECTION_CREAM and TAB_ACTIVE -> `pal::HEADING`.
  - ROW_BASE -> `pal::ROW_EVEN`.
  - ROW_SUMMARY_SIZE 15.0 -> `fonts::SUMMARY_SIZE` 15.0.
  - INPUT_FILL (0x09 0x09 0x09) -> new `pal::RECESS` with the same RGB.
  - CONTROL_GLYPH keeps its RGB.
  - `pal::INPUT_FILL` is 0x07, so `RECESS` was correctly added rather than reusing it.
  - A unit test pins both new constants.
- **Cinzel.** No remaining reference to the "cinzel" family or `DISPLAY_FONT` in `src/`. The only `FontFamily::Name` use outside `fonts.rs` is `SMALL_CAPS`, which is registered. The fallback chain in `skin::fonts::install` is unchanged and independent of Cinzel, so there is no egui missing-family panic.
- **Removed items.** No remaining reference to `ff-proto`, `overlay::theme`, `header_plate` or `zebra_row` in `src`, `tests`, README, CLAUDE.md or ASSETS.md. `Cargo.toml` and `Cargo.lock` agree on 0.6.0, and the `ff-proto` `[[bin]]` entry is gone.
- **Docs.** README tabs (Rules | Config | Economy | About), hotkey `Ctrl+Shift+Z`, binaries, `apply` flags and file names match CLAUDE.md and the code. All five `docs/screenshots/*.png` files referenced by README exist.
- **Screenshot check.** The before/after PNG diff is recorded as identical for the shipped views.

## Info

### IN-01: Overlay-only size constant lives in the skin module

**File:** `src/skin/fonts.rs:14`
**Issue:** `SUMMARY_SIZE` is an overlay summary/caption size, but it now sits next to the font-family names in the shared skin. Call sites in `overlay/*` use the long path `crate::skin::fonts::SUMMARY_SIZE`. This is not a defect.
**Fix:** Optional. Add `use crate::skin::fonts::SUMMARY_SIZE;` in `body.rs`, `config_view.rs` and `widgets.rs` to shorten the call sites.

### IN-02: "stripe" vocabulary survives in many doc comments

**File:** `src/overlay/app.rs:1` (and about 40 more occurrences in `app.rs`, `apply.rs:65`, `bin/ff.rs:3`)
**Issue:** Comments still call the window "the stripe", the pre-skin design. CLAUDE.md and README now describe a docked window. This is not a removed symbol, so it is outside the CLEAN-01/02 scope, but it is stale wording the phase aimed to retire ("old stripe styling").
**Fix:** Sweep "stripe" -> "window" in comments, or defer explicitly.

---

_Reviewed: 2026-10-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
