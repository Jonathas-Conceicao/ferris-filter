---
phase: 38-built-in-styles-strictness-reset
plan: 06
subsystem: docs
tags: [claude-md, phase-gate, screenshots]
requires: [38-01, 38-02, 38-05]
provides: [CLAUDE.md accurate for the nine built-ins, seed-styles generate spec, strictness row, resets, ten popups]
affects: [CLAUDE.md]
key-files:
  modified:
    - CLAUDE.md
requirements-completed: [BSTY-01, BSTY-02, BSTY-03, STRICT-01, STRICT-02, STRICT-03, STRICT-04, RESET-01, RESET-02, FIX-01]
metrics:
  tasks: 2
  files: 1
---

# Phase 38 Plan 06: CLAUDE.md and phase gate Summary

CLAUDE.md now documents the nine shipped built-ins (new values), the frozen `tests/fixtures/seed-styles.json` as the `generate` spec, the consequence that `generate` without a live `styles.json` reports the unknown style for Recycle / kite / exceptional, the strictness row, both Reset to defaults buttons, the ten popups and the new `--popup` values.

## Tasks

| Task | Commit | Notes |
|------|--------|-------|
| 1 CLAUDE.md update | 9d15001 | intro + layout rows, Tier table, Current styles table (nine rows plus a "no custom styles ship" paragraph), Config/Economy overlay notes, popups eight to ten, screenshot flags; stale `uwu.mp3` authoring-convention bullet and the kite rename history removed |
| 2 Phase gate + screenshots | (no files) | see below |

## Gate

- `cargo test --no-fail-fast` (temp FERRISFILTER_HOME / FERRISFILTER_GAME_DIR): 1658 passed, 0 failed (lib 1535, cli_errors 12, cli_filter_copy 4, cli_generate 20, dynamic_filter 26, filter_roundtrip 25, filter_structure 18, isolation_guard 5, overlay_rules 4, sync_prices 8, doc-tests 1).
- `cargo fmt --check`: only diffs are in `tests/isolation_guard.rs` (known unrelated reformat; not committed, file untouched).
- `cargo clippy --all-targets -- -D warnings`: fails with ~125 errors, all lint categories of the current floating nightly in code this phase did not touch (large `Err` variants, `field_reassign_with_default`, collapsible `if`, `is_multiple_of`, ...). Not fixed (out of scope); recorded as deferred.

## Screenshots (screenshot mode, temp home/USERPROFILE/game folder under `%TEMP%\ff38`, fresh install with no styles.json)

All in `C:\Users\jonat\AppData\Local\Temp\ff38\shots\`:
- `cfg.png` (Config): seven captions on one row, none truncated (two-word names stacked on two lines by design); Flat Broke lit; Reset to defaults at the right of the Styles heading, dimmed at defaults. OK.
- `eco.png` (Economy): Reset to defaults at the right of the Tiers heading, dimmed. OK.
- `rs.png` (Config + reset-styles popup): title "Reset styles to defaults?", UI-SPEC body, red Reset + Keep; tab shows ` *`. OK.
- `rt.png` (Economy + reset-tiers popup): title "Reset tiers to defaults?", body, red Reset + Keep. OK.
- `ed.png` (Config, `--expand-style S`): editor opens normally under the strictness row; pink Star minimap, pale background card, font 37 shown. OK.

## Deviations / Decisions

- Documentation-only plan. Did not touch CHANGELOG.md or README.md.
- `FIX-01` etc. were delivered by earlier plans; this plan only verified them together.
- Clippy red is pre-existing toolchain lint debt (see above), not caused by Phase 38.

## For the Release Review

- Overlay: Config tab Strictness row (Custom, Flat Broke, Scrounging, Getting By, Comfy, Loaded, Div Farmer; affects every rule using tiers S-F, user rules included) and Reset to defaults on Styles and Tiers (pending edit, custom styles kept).
- Built-ins reworked (Star minimaps, built-in alert sounds, no `uwu.mp3`); Recycle / kite / exceptional no longer ship, so `ferrisfilter generate` on a checkout without a live `styles.json` fails with the unknown-style error (deferred rework).
- `tests/fixtures/seed-styles.json` is the new generate fixture source.

## Human verification

- Click Reset on both tabs in the live overlay (tab ` *`, Revert undoes, Apply writes); try each strictness level against a real filter.

## Self-Check: PASSED
