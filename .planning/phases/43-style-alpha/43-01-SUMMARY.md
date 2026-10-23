---
phase: 43-style-alpha
plan: 01
subsystem: models, emitter, overlay config state
tags: [alpha, styles, serde]
requires: []
provides:
  - "StyleColour ([u8;4] newtype, read 3 or 4 values, write 4) in src/models/config.rs"
  - "style_action_lines writes the style alpha (generate, economy, styled rewrite)"
  - "4-value styles-default.json"
affects: [43-02, 43-03, 44]
tech-stack:
  added: []
  patterns: ["hand-written visit_seq serde (precedent Cancelable)", "value-based dirtiness, no migration flag"]
key-files:
  created: []
  modified:
    - src/models/config.rs
    - src/loader.rs
    - src/emitter.rs
    - src/styled.rs
    - src/defaults.rs
    - styles-default.json
    - src/overlay/config_state.rs
    - src/overlay/config_view.rs
    - src/overlay/style_lines.rs
    - src/overlay/app.rs
key-decisions:
  - "No 3-to-4 migration flag: dirtiness is derived PartialEq on Style, so a 3-value file is not an edit and keeps its bytes until another write"
  - "StyleView::from_style sets alpha: true now (see deviations)"
requirements-completed: [ALPHA-02, ALPHA-03]
duration: ~40 min
completed: 2026-10-23
---

# Phase 43 Plan 01: Style alpha data path Summary

`StyleColour` carries alpha from `styles.json` through the single emitter to every filter write; 3-value files load as alpha 255 silently and every write is 4-value.

## Tasks

1. StyleColour type, serde and compile sites: 1c0c875 (also regenerated `styles-default.json` via `render_styles` and updated the `defaults.rs` snapshot literals, since the shipped round-trip tests need them to compile and pass).
2. Emitter, economy and styled-rewrite alpha tests: 15ddc8a.
3. CONFIG state tests (3-value not an edit, alpha edit dirty / 4-value / revert / reset, strictness leaves colours): 36beb8f. No production change was needed.

## Deviations from Plan

**1. [Rule 1 - Bug] StyleView::from_style sets `alpha: true`**
The plan said to leave `alpha: false` for Plan 02. With the opaque colour branch, any colour pick on a style with alpha would be written back with 255, dropping alpha. Setting `alpha: true` routes styles through the alpha-capable picker already. Plan 02 can still remove the field and the opaque branch. Commit 1c0c875.

**2. Opaque-picker code left in place** (`widgets::colour_slot`, `colour_swatch_rgb`) as the plan directed; they are now dead for styles and left for Plan 02.

## Deferred / out of scope

- `cargo clippy --locked --all-targets -- -D warnings` fails with 127 errors on the floating nightly (large Err variants, collapsible ifs, etc.) in code this plan did not touch; `cargo fmt --check` also reports 3 pre-existing diffs (src/skin/editor.rs, src/skin/widgets.rs, others). Not fixed.
- `cargo fmt` was run once then the unrelated files were reverted.

## Verification

`cargo test --locked` green (1702 lib tests + integration, incl. artifact_guard, generate_snapshot, isolation_guard, shipped_styles_*). `tests/fixtures/`, Cargo.toml and Cargo.lock unchanged. Built with `CARGO_TARGET_DIR` outside the repo because `ff.exe` was running.

## Self-Check: PASSED

Commits 1c0c875, 15ddc8a, 36beb8f exist; files above present.
