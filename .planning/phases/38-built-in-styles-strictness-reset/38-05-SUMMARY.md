---
phase: 38-built-in-styles-strictness-reset
plan: 05
subsystem: overlay
tags: [reset, popups, config-tab, economy-tab]
requires: [38-03, 38-04]
provides: [reset-to-defaults buttons, reset-styles and reset-tiers popups]
affects: [src/overlay/config_view.rs, src/overlay/shell.rs, src/overlay/app.rs, src/overlay/screenshot.rs]
key-files:
  modified:
    - src/overlay/config_view.rs
    - src/overlay/shell.rs
    - src/overlay/app.rs
    - src/overlay/screenshot.rs
requirements-completed: [RESET-01, RESET-02]
metrics:
  tasks: 2
  files: 4
---

# Phase 38 Plan 05: Reset to defaults buttons and popups Summary

Reset to defaults on the Styles heading (Config tab) and the Tiers heading (Economy tab), each greyed with `Already at the defaults.` when nothing differs, each opening a skinned confirm popup whose red Reset applies the plan 03 model reset as one revertible pending edit.

## What was built
- `ConfigAction::ResetStyles/ResetTiers`, `ConfigUi.reset_armed` (cleared by `disarm_all`), `section_title` now returns the heading `Response`.
- Styles button in `show_styles`; Tiers button in `economy_tab` (`EconomyClicks.reset_tiers`, `EconomyTabInputs.tiers_reset_enabled`).
- `Popup::ResetStyles/ResetTiers`, `PopupFlags`, `RESET` constant, `popup_spec` arms with the UI-SPEC texts; stacking sits right after Remove economy.
- App: `popup_flags`, `popup_spec_for`, `dispatch_popup`, `handle_config_actions` arms, economy click consumer, `open_shot_popup`; screenshot `POPUPS` grows to 10.
- Tests: button greyed/armed, popup specs, stacking, economy click gating, dispatch Keep/Reset for both.

## Commits
- 9e9abdf feat(38-05): Reset to defaults buttons and confirm popups for styles and tiers

## Test results
`cargo test --lib -- overlay` : 807 passed, 0 failed.

## Deviations from Plan
1. **[Rule 1] Expanded row not collapsed by `reset_for_new_styles`.** It only bumps epochs, so the handler also sets `config_ui.expanded = None` to meet the "collapses the expanded row" requirement.
2. **Button rect.** The button spans from the heading's left edge to the ui's right edge minus 8pt rather than `heading.rect.shrink2`, so it also works when the heading is the no-skin fallback label (headless tests).
3. **One commit instead of two.** The shell popup types and the app dispatch reference each other, so the two tasks did not compile separately; they were committed together.
4. Test fixtures use `seed-styles.json`, which differs from the defaults, so tests call `reset_styles()` first to reach the at-defaults state.

## Decisions
- The Tiers button reads enabled from `!tiers_at_defaults()` and is false when no config state is loaded.

## For the Release Review
New user-facing controls: Reset to defaults on Styles (Config tab) and Tiers (Economy tab), each with a confirm popup; the result is a pending edit that Revert undoes and Apply writes. Custom styles are kept.

## Human verification
- Look at `--screenshot ... --popup reset-styles` / `reset-tiers` and the two headings on a real window (button placement beside the gold heading, 150pt width), not run here.
- Click Reset on both tabs in the live overlay: tab ` *` appears, Revert undoes, Apply writes.

## Self-Check: PASSED
