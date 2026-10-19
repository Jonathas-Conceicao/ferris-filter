---
phase: 38-built-in-styles-strictness-reset
plan: 03
subsystem: overlay-model
tags: [strictness, reset, config-state]
requires: [38-01]
provides:
  - "overlay::strictness::{Strictness, strictness_of, TIER_LETTERS}"
  - "ConfigState::{strictness, set_strictness, reset_styles, styles_at_defaults, reset_tiers, tiers_at_defaults}"
  - "StyleSet::shipped_defaults()"
affects: [38-04, 38-05]
key-files:
  created:
    - src/overlay/strictness.rs
  modified:
    - src/overlay/config_state.rs
    - src/overlay/mod.rs
    - src/models/config.rs
key-decisions:
  - "Level is derived, never stored: hidden S..F letters (found by builtin id) matched against the six presets, else Custom"
  - "set_strictness skips styles already at the target action, so a repeat call returns false"
  - "reset_tiers writes min and unit into existing entries by tier name (file order kept)"
requirements-completed: [STRICT-02, STRICT-03, STRICT-04, RESET-01, RESET-02, BSTY-03]
metrics:
  tasks: 2
  completed: 2026-10-18
---

# Phase 38 Plan 03: Strictness and reset model Summary

Pure, egui-free model for the six strictness levels and for resetting styles and tiers to the embedded defaults, all as pending edits on the existing `ConfigState`.

## Tasks

| Task | Commit | Notes |
|------|--------|-------|
| 1 Strictness model + shipped_defaults | 2555946 | `strictness.rs`, `set_strictness`, `StyleSet::shipped_defaults()` (OnceLock) |
| 2 Reset styles/tiers + unit-aware dirty count | 2555946 | `reset_styles`, `reset_tiers`, `*_at_defaults`, `tiers_dirty_count` compares unit |

## Verification

`cargo test --lib -- overlay::strictness overlay::config_state models::config`: 130 passed (temp dirs only, no real home or game folder). Tests prove: every level round-trips, only S..F change (unchanged, hidden, Recycle, kite, exceptional untouched), Revert restores, resets keep custom styles (ids, order, edits) and tier file order, resets write no file, the old twelve-style `styles.json` loads and an untouched Apply keeps its bytes, shipped defaults carry `builtin:<name>` ids.

## Deviations from Plan

**1. [Process] RED and GREEN committed together.** Tests and implementation share files (`config_state.rs`, `strictness.rs` tests need `shipped_defaults`), and the two tasks touch the same module, so a compiling RED commit was not practical; both tasks landed in one commit (2555946), with the tests written alongside the code. No functional impact.

No other deviations. `cargo fmt` reformatting of `tests/isolation_guard.rs` was reverted.

## Decisions

- Presets other than the current ones never match when a tier id is missing (Custom), and `set_strictness` skips an absent id rather than failing.
- Tooltip and caption strings are verbatim from the UI-SPEC.

## Known Stubs

None (views are wired by plans 04 and 05).

## For the Release Review

Nothing for CHANGELOG/README yet: the model has no user-visible surface until plans 04/05 wire the Strictness row and Reset buttons.

## Self-Check: PASSED
