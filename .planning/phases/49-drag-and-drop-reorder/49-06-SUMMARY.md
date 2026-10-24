---
phase: 49-drag-and-drop-reorder
plan: 06
subsystem: overlay
tags: [screenshot, docs, drag-and-drop]
requires: [49-04, 49-05]
provides:
  - "debug-only --drag <row>:<slot> and --drag-style <name>:<gap> screenshot flags"
  - "CLAUDE.md and assets/README.md describing drag reorder and the handle art"
  - "Phase 49 manual checks and judgement calls in AUTONOMOUS-V072-DEFERRED.md"
key-files:
  modified: [src/overlay/screenshot.rs, src/overlay/app.rs, src/overlay/rule_list.rs, src/overlay/config_view.rs, CLAUDE.md, assets/README.md, .planning/phases/AUTONOMOUS-V072-DEFERRED.md]
key-decisions:
  - "Injected pointer is a NaN-x marker (Pos2 { x: NaN, y: slot }) resolved to the boundary / gap on the first drawn frame"
requirements-completed: [DND-01, DND-02, DND-03, DND-04, DND-05]
completed: 2026-10-23
---

# Phase 49 Plan 06: Screenshot flags, docs and deferred entries Summary

Debug builds can now screenshot a RULES drag and a CONFIG style drag without a pointer, CLAUDE.md and assets/README.md describe the built drag reorder, and seven `Phase 49:` entries (ten numbered real-pointer checks, judgement calls, screenshot paths) are appended to the run's deferred list.

## What was built

- Task 1 (2a760ca): `--drag <row>:<slot>` and `--drag-style <name>:<gap>` parsed in `screenshot.rs` (malformed values are errors, parse tests added). `app.rs` sets `list.drag` / `config_ui.drag` with `injected: Some(pos2(NaN, slot))`. `show_list` resolves it to `list_rect.top() + offsets[slot] - scroll offset`; `draw_style_drag` resolves it to the gap's row boundary after the rows are drawn (the early auto-scroll ignores a NaN pointer). Injected drags never commit and never consult `dragged_id()`. Release build of `ff` succeeds.
- Task 2 (dfa46c3): CLAUDE.md edits for the flag list, `ff_drag_handle` / `Skin.drag_handle`, the nine art files under their exact names, the rules and styles drag descriptions, and the sections note (section headers keep their arrows); assets/README.md rows for the handle PNGs and the three palettes. No CHANGELOG / README.md edit.
- Task 3: four screenshots plus `rules-economy.png` under `C:\Users\jonat\AppData\Local\Temp\claude\ffshot49\` (temp home and game folder), inspected; deferred entries appended (append only).

## Screenshot review

- `rules-handles.png`: handles at the right edge of user rows; section header rows keep arrows. `rules-economy.png`: economy rows have no handle.
- `rules-drag.png`: greyed source, translucent shadow, gold marker visible.
- `config-styles.png`: built-in Order cells blank, custom handles at the right edge.
- `config-drag.png`: the gold gap line is hidden under the shadow (logged as a Phase 49 deferred note, not fixed).

## Deviations from Plan

- The CONFIG shots use `--expand-style` because the Custom table sits below the fold and no scroll flag exists.
- My scratch filter's `# ff:group` pair loaded as two ordinary rules (hand-written, not a code defect).

## Verification

`cargo test --locked` green (1842 lib tests plus integration suites); `cargo build --release --bin ff` succeeds; fixtures and dependencies unchanged; STATE.md and ROADMAP.md untouched.

## Commits

- 2a760ca: feat(49-06): debug-only --drag and --drag-style screenshot flags
- dfa46c3: docs(49-06): describe drag reorder and the handle art in CLAUDE.md and assets README
- docs(49-06): Phase 49 deferred entries and summary (see git log)

## Self-Check: PASSED
