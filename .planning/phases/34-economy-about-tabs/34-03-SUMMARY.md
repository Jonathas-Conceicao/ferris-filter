---
phase: 34-economy-about-tabs
plan: 03
subsystem: ui
tags: [egui, overlay, economy-tab, tiers]
requires: [34-02]
provides:
  - "shell::economy_tab(ui, skin, &EconomyTabInputs, tiers closure) -> EconomyClicks"
  - "config_view::show_tiers_section(ui, skin, state) -> Vec<ConfigAction>"
affects: [34-04]
key-files:
  modified: [src/overlay/shell.rs, src/overlay/config_view.rs, src/overlay/rule_list.rs, src/overlay/app.rs, src/overlay/body.rs]
key-decisions:
  - "Add economy section stages in memory; Remove opens the existing popup; both are written only by Apply"
metrics:
  tasks: 3
  completed: 2026-10-16
requirements: [ECO-01, ECO-02]
---

# Phase 34 Plan 03: ECONOMY tab Summary

The ECONOMY tab now shows League, Last sync (relative, exact time on hover), Priced items and Price source above a centred Sync economy button, then an Economy section row with one Add/Remove button and the tier cutoffs editor. RULES and CONFIG lost their interim economy and tiers controls.

## Commits
- 967f1ae: feat(34-03): ECONOMY tab with sync, section button and tiers; RULES economy divider display-only (tasks 1-3)

## What changed
- `shell.rs`: `EconomyClicks`, `EconomyTabInputs`, new `economy_tab`; the `#[allow(dead_code)]` markers from 34-02 are gone. Headless tests cover every state and that the tiers closure runs once.
- `config_view.rs`: `show_tiers_section` (skinned `ff_field_row`s, red outline plus tooltip on error, edge bar kept, captions turned into tooltips); `show_sections` is Filter and Styles only; unused `helper` removed.
- `rule_list.rs`: removed the Economy header x, `economy_edit`, the economy footer and its tests; `ECONOMY_TIP` now ends "Add or remove it in the ECONOMY tab."; `confirm_economy_remove` kept.
- `app.rs`: `rules_content` has no footer slot; the Economy arm builds the inputs and routes `EconomyClicks` (sync start body unchanged, add -> `stage_add_economy`, remove -> `confirm_economy_remove`); tier actions are folded into `config_actions`; repaint every 60 s keeps Last sync fresh. Tests: add click stages in memory with the file unchanged; remove popup choice 0 stages, choice 1 does not.
- `economy.rs` untouched.

## Deviations from Plan
- One commit instead of three: the `economy_tab` signature change, the `show_list` parameter removal (call sites also in `body.rs`) and the app wiring must build together for every commit to pass `cargo test`.
- `show_tiers_section` takes `skin` per the interface but does not need it (`ff_field_row` is skin-free); it is ignored.
- A damaged economy section no longer has a footer message on RULES; the status line already reports it and the ECONOMY button is disabled.

## Verification
`cargo test` green (1322 lib tests plus integration), `cargo fmt --all --check` clean, clippy total 122 (lib test 119 + cli_generate 2 + sync_prices 1), `tests/fixtures` unchanged, `cargo build --release --bin ff` succeeds.

## Known Stubs
None.

## Self-Check: PASSED
