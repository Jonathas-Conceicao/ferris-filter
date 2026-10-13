---
phase: 19-overlay-rule-list-over-parsed-blocks
plan: 06
subsystem: overlay
tags: [egui, overlay, apply, revert, rule-list]
requires:
  - phase: 19-03
    provides: dynamic::apply_edited, blocking_errors, EconomyOp
  - phase: 19-05
    provides: rule_list::show_list/search_bar/economy_footer
provides:
  - Stripe wired to the searchable virtualized rule list
  - APPLY/REVERT on the edited document, rules-only Apply, staged economy Add/Remove
affects: [phase-19 verification]
tech-stack:
  patterns: [deferred actions applied once after render, controller helpers as free functions]
key-files:
  modified:
    - src/overlay/apply.rs
    - src/overlay/app.rs
key-decisions:
  - "APPLY needs no economy section (OD-2); an Absent filter needs a dirty working copy"
  - "Expanded row survives Apply by position (ids reset on re-parse)"
requirements-completed: [EDIT-03, EDIT-04, EDIT-05, EDIT-07, EDIT-08, ECON-02]
duration: 40min
completed: 2026-10-13
---

# Phase 19 Plan 06: Wire the rule list into the stripe Summary

The overlay now shows the loaded filter as a searchable, virtualized rule list with a rule count, an economy footer row, and Apply/Revert operating on the in-memory edited document through `dynamic::apply_edited`.

## Tasks

1. **Apply/Revert rules and controller** (d920ac5): `FooterActions { sync, apply, revert, add_rule, reload }`, `footer_actions(root_ok, state, needs_reload, dirty, blocking)`, new copy constants and `invalid_blocks_message`/`added_hidden_message`; `OverlayApp` owns `WorkingFilter` + `ListUi` + query; `apply_working` and `apply_row_actions` helpers with tempdir tests; `revert`, `stage_add_economy`, `stage_remove_economy`, extended `reload`.
2. **Stripe layout** (17890a3): FILTER header with right-aligned count (`N rules` / `M of N rules`), search bar, empty/no-match/load-failed states, footer economy row, `+ ADD RULE`, `REVERT`, `APPLY` with disabled tooltip, clickable grammar-gate line that clears a hiding search and expands/scrolls to the first offending rule. Removed the Phase 18 "Rule editing returns" notice, the `ADD ECONOMY SECTION` button and the separate changed-on-disk line (its copy now flows through `apply_message`). 511-block stripe-scale test added.

## Deviations from Plan

None of substance. Minor choices: the count beside the header uses the search query from the start of the frame (the search bar renders below it); the `Apply` failure path preserves `needs_reload` unless a changed-on-disk refusal sets it.

## Verification

Full `cargo test` green (498 lib tests plus integration), `cargo build --bins` warning-free, `cargo fmt --check` clean, `tests/fixtures` and `tests/snapshots` unchanged. Not verified visually: the live overlay was not launched (constraint: no real game-folder access).

## Known Stubs

None.

## Self-Check: PASSED
