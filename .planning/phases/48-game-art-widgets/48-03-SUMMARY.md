---
phase: 48-game-art-widgets
plan: 03
subsystem: overlay
tags: [art, egui, search-field]
requires: [48-01]
provides:
  - search_layout pure geometry and ff_search_field skin widget
affects: [49]
key-files:
  modified:
    - src/skin/widgets.rs
    - src/overlay/rule_list.rs
    - src/overlay/body.rs
key-decisions:
  - "ff_search_field takes &Skin (like the other skin widgets) rather than reading it from the context"
metrics:
  tasks: 2
  completed: 2026-10-23
requirements: [ART-02, ART-03]
---

# Phase 48 Plan 03: Search field game art Summary

The RULES toolbar search and the + condition picker search now show the game's magnifier at the left and a round clear X at the right while they hold text; the X empties the field and keeps it focused.

## Commits

- 83ebdee: feat(48-03): add ff_search_field with game magnifier and clear X
- 9e6944b: feat(48-03): use ff_search_field in RULES toolbar and condition picker

## What changed

- `ff_text_field_ex` now delegates to a private `text_field_inner` (left/right edit insets, returns edit response and field rect); existing callers keep 10/10 insets, so they are unchanged.
- `search_layout` (glyph 16x16 at +10, edit inset 32, X disc 18 / hit 24 centred 17 from the right) and `ff_search_field(ui, skin, id, text, hint, width)`. The X uses non-focusable `Sense::CLICK`, is hidden for empty text, and requests focus for the edit id after clearing.
- Tests: layout geometry, X absent when empty, X click empties and the edit has focus on the next frame.
- RULES toolbar uses id `rules-search`; the old painted `Clear` link is deleted; Escape-clear untouched. Picker uses `popup_id.with("query-edit")`; its test now checks for `ff_search_field(`.
- `cargo test --locked` green (1714 lib tests); tests/fixtures untouched; no new dependencies.

## Deviations from Plan

- `ff_search_field` takes `&Skin` as an argument (the plan allowed either).
- Task 1 tests and implementation share one commit (no separate RED commit).
- No harness test for the picker staying open on an X click (the X is inside the popup area, so CloseOnClickOutside does not trigger); left as a manual check, as is any screenshot check of the glyph/X art.
- `cargo fmt` also reformatted `src/skin/editor.rs` (pre-existing drift); that change was reverted, out of scope.

## Known Stubs

None.

## Self-Check: PASSED

Commits 83ebdee and 9e6944b exist; full test suite passes.
