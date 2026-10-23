---
phase: 43-style-alpha
plan: 02
subsystem: overlay style editor, skin colour card
tags: [alpha, picker, previews]
requires: [43-01]
provides:
  - "CONFIG style colour cards open the alpha picker (Alpha slider + None tile) on Text, Border, Background"
  - "ff_colour_card_tip: card painted over INPUT_FILL, caption contrast from the composited colour, optional tooltip"
affects: [43-03]
tech-stack:
  added: []
  patterns: ["one colour-slot path (colour_slot_rgba) for rules and styles"]
key-files:
  modified:
    - src/overlay/style_lines.rs
    - src/overlay/widgets.rs
    - src/overlay/body.rs
    - src/overlay/config_view.rs
    - src/skin/colour_picker.rs
    - src/skin/editor.rs
key-decisions:
  - "R G B A card tooltip lives in colour_slot_rgba, so RULES cards get it too; shown only when alpha != 255"
  - "Opaque helpers (colour_slot, SlotResult, colour_swatch_rgb, rgb_caption) and StyleView.alpha removed; tests ported"
requirements-completed: [ALPHA-01, ALPHA-04]
duration: ~35 min
completed: 2026-10-23
---

# Phase 43 Plan 02: Style alpha in the overlay Summary

Style colour cards now go through the same alpha-capable slot as the RULES colour actions, and cards and previews show translucent colours over their dark ground.

## Tasks

1. Style slots on the alpha picker path, opaque path removed: 1e41e93. Ported the two slot tests to `colour_slot_rgba` (quick pick keeps alpha 128; None tile clears; Alpha slider and None tile both present). Added `style_edit_for_keeps_the_alpha`, `from_style_carries_the_style_alpha`, `label_look_paints_translucent_colours_unmultiplied`.
2. Translucent colour cards: 3e488e2. `ff_colour_card_tip` fills `INPUT_FILL` first, then the colour; caption colour from `composite_over(c, INPUT_FILL)` (`on_color` unchanged); tip replaces the cut-caption tooltip (joined `"{caption}: {tip}"` when both). Test for composited caption contrast.

## Decisions

- Tooltip in `colour_slot_rgba` (applies to RULES cards at alpha < 255; alpha 255 tooltips unchanged).
- Opaque path deleted outright; hex field stays 6-digit RGB.
- The label preview and table preview needed no code change: `style_lines::label_look` already used `from_rgba_unmultiplied`, and `ff_item_label` paints over the ground (no checkerboard added). Pinned by tests.

## Deviations from Plan

None beyond plan 01's `alpha: true` note. (`style_edit_for` already kept alpha from plan 01; only its test was added.)

## Deferred / out of scope

- `cargo clippy -D warnings` and `cargo fmt --check` were not run as gates: clippy has pre-existing nightly errors and fmt has pre-existing diffs in files I touched (editor.rs). No new warnings were knowingly added.

## Verification

`cargo test --locked --tests` green (1704 lib tests plus all integration suites, incl. artifact_guard, isolation_guard). `tests/fixtures/` unchanged. Built with `CARGO_TARGET_DIR` outside the repo. Grep gates (SlotResult, colour_swatch_rgb, rgb_caption, "styles are opaque", `pub alpha: bool` in style_lines) return nothing.

## Self-Check: PASSED

Commits 1e41e93 and 3e488e2 exist; modified files present.
