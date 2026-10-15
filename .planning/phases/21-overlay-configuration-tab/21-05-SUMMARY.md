---
phase: 21-overlay-configuration-tab
plan: 05
subsystem: overlay
tags: [egui, config-view, style-editor, tiers]
requires: ["21-03", "21-04"]
provides:
  - "overlay::config_view::{ConfigAction, ConfigUi, ConfigViewCtx, show_sections, show_status, show_footer, show_load_error, style_name_hint, filter_name_hint, footer_buttons, delete_prompt, revert_prompt, delete_visible}"
  - "widgets::{style_preview_chip, colour_swatch_rgb, rgb_caption}"
affects: [21-06]
key-files:
  created: [src/overlay/config_view.rs]
  modified: [src/overlay/widgets.rs, src/overlay/body.rs, src/overlay/mod.rs]
key-decisions:
  - "Colour picker is opaque (Alpha::Opaque): Style stores RGB and the emitter always writes alpha 255"
  - "show_sections owns its ScrollArea; the app pins the footer in a bottom panel before calling it"
  - "Keep editing on the outside-edit guard sets ConfigUi::outside_dismissed (reset by disarm_all) instead of being a dead button"
requirements-completed: [CONF-01, CONF-02, CONF-03]
duration: ~35min
completed: 2026-10-13
---

# Phase 21 Plan 05: CONFIG tab view Summary

A thin egui view over `ConfigState` renders FILTER, STYLES and TIERS plus status and footer, returning every change as a `ConfigAction` and reusing the Phase 20 action widgets.

## Accomplishments
- FILTER: file combo with `New empty filter...` draft row, name hints, switch guard.
- STYLES: file-order rows with preview chip and Hide/built-in/new/edited badges; one expanded editor (action, three optional opaque colours, font, sound None/Alert/Custom, drop sound, minimap, beam); add draft; inline delete confirm with rule count and `config.json` warning; built-ins show no Delete.
- TIERS: S..F text buffers, read-only unit, inline errors with edge stroke, footnote.
- Footer driven by pure `footer_buttons`; Apply disabled with `Fix the tier errors first.`; revert and outside-edit confirms.
- `body.rs` helpers promoted to `pub(crate)` (no behaviour change).

## Commits
- 6e4f7b1: shared widgets and body promotions
- cba8087: config_view module

## Deviations from Plan
- Commits were split by file group (widgets/body, then config_view/mod) rather than by task, since Task 1 and Task 2 both live in `config_view.rs` and were written together.
- Added `ConfigUi::outside_dismissed` so `Keep editing` has an effect (plan said "no action").
- Widget-level assertions (no Delete button for built-ins, etc.) use pure helpers (`delete_visible`, `delete_prompt`, `footer_buttons`) as the plan allowed.

## Deferred Issues
- `cargo clippy --lib -- -D warnings` reports 62 pre-existing errors in unrelated files (e.g. `src/sync/prices.rs`, `widgets.rs:89` zebra_row); none in the new code.

Verification: `cargo test` 715 lib tests pass, `cargo fmt --check` clean, fixtures and fallback files untouched.

## Self-Check: PASSED
