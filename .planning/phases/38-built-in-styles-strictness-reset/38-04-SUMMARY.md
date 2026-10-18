---
phase: 38-built-in-styles-strictness-reset
plan: 04
subsystem: overlay-config-tab
tags: [strictness, config-view, app-wiring]
requires: [38-02, 38-03]
provides:
  - "ConfigAction::SetStrictness(Strictness)"
  - "config_view::strictness_row (ff_levels) drawn at the top of the Styles section"
affects: [38-05]
key-files:
  modified:
    - src/overlay/config_view.rs
    - src/overlay/app.rs
key-decisions:
  - "The selection is state.strictness() of the pending set every frame; no new ConfigViewCtx field"
  - "SetStrictness is not in the clear-events list; the arm clears stale messages only when set_strictness changed something, so a no-op keeps the last message"
requirements-completed: [STRICT-01, STRICT-02, STRICT-03, STRICT-04]
metrics:
  tasks: 2
  completed: 2026-10-18
---

# Phase 38 Plan 04: Strictness row on the Config tab Summary

The Styles section now opens with the one-line `Custom | Flat Broke | Scrounging | Getting By | Comfy | Loaded | Div Farmer` switch. A click is an ordinary pending style edit (tab ` *`, Revert popup, Apply writes `styles.json`).

## Tasks

| Task | Commit | Notes |
|------|--------|-------|
| 1 Strictness row in show_styles | 3375aac | `strictness_row` between the heading and Built-in table, skipped on `No styles loaded`; includes the app match arm needed to compile |
| 2 App handler + tests | f05f15b | revert, apply (temp home, no filter loaded) and no-op tests |

## Verification

`cargo test --lib -- overlay::app overlay::config_view overlay::config_state`: 273 passed; `cargo test --test isolation_guard`: 5 passed. Tests use temp dirs only (FERRISFILTER_HOME / FERRISFILTER_GAME_DIR set to empty temp dirs). Row tests click real segment centres in a headless egui frame: Comfy emits exactly one `SetStrictness(Comfy)`, Custom and the selected level emit nothing, a manual Hide of E reads as Custom, the row sits above the `builtin-heading` probe.

## Deviations from Plan

**1. [Process] The handler arm landed in the Task 1 commit.** `handle_config_actions` matches `ConfigAction` exhaustively, so the new variant would not compile without it; Task 2 holds the tests and the message-clearing refinement.

**2. [Behaviour] SetStrictness left out of the clear-events `matches!` list.** The plan asked for it there, but that list clears messages for any action in the batch, contradicting the "current level changes nothing and keeps the last config message" behaviour. The arm calls `clear_events_on_edit()` only on a real change instead (same effect for real edits).

The Apply test sets `app.filter = Err(..)` / `working = None` so only the config files are written (the rig's game folder does not exist).

## Known Stubs

None.

## Human-verification items

- Look at the Config tab at 660pt wide: the seven captions fit on one line, stacked two-word captions are legible, the selected segment highlight and the Custom hover tooltip read well (layout was only exercised headless, no screenshot taken).
- Click Loaded then check the Config tab label shows ` *`, the Visibility switches for C..F read Hide, Revert asks "Discard config changes?" and restores Flat Broke.

## For the Release Review

Config tab gains a Strictness row (six presets plus a display-only Custom) that hides tier styles C..F/B..F in one click; it affects every rule using those tier styles, user rules included (stated in each tooltip). Not yet in CHANGELOG/README.

## Self-Check: PASSED
