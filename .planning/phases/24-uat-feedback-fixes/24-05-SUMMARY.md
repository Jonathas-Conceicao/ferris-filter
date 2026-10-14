---
phase: 24-uat-feedback-fixes
plan: 05
subsystem: overlay
tags: [egui, rule-editor, styles]
requires: [24-03, 24-04]
provides: [body::style_section, DISPOSITION_CHOICES, style_pick_action]
affects: [src/overlay/body.rs, src/overlay/rule_list.rs]
key-files:
  modified: [src/overlay/body.rs, src/overlay/rule_list.rs]
decisions:
  - "Custom picker entry maps to RemoveStyle (tag only, actions kept); preset maps to ApplyStyle"
  - "Minimal shown as a non-clickable label when the block is Minimal; never offered"
metrics:
  completed: 2026-10-13
---

# Phase 24 Plan 05: Style section Summary

One style section at the end of the rule body: a Custom/preset picker with Show/Hide (no Minimal) and Continue, Custom editing actions inline, presets hiding them, and no style captions.

## Tasks

1. `body::style_section` replaces `style_group` and `actions_group`. Added `DISPOSITION_CHOICES`, `StylePick`, `style_pick_action`, `shows_action_rows`. Unknown/unchecked/known warnings are tooltips on the picker (warning colour for unknown/unchecked). Commit f89f58f.
2. `expanded_body` order is NAME, CONDITIONS, STYLE, then Duplicate/Delete (same commit). Test `expanded_body_orders_name_conditions_style` added.
3. Follow-up eabe485: gated the now test-only `style_owned` and its import with `cfg(test)` to keep the build warning-free.

## Deviations from Plan

- The plan's Task 2 changes landed in the same commit as Task 1 because removing `style_group`/`actions_group` would not compile without the `rule_list.rs` call-site change.
- `known_style_row_offers_remove_style` was replaced by pure-function tests (`style_pick_action_maps_custom_and_presets`, `preset_to_custom_keeps_actions_and_drops_tag`) since the picker is a popup combo.
- Style-owned action locking inside `action_card` is no longer reachable (preset hides rows); the `lock` argument is always `None`.

## Verification

Full `cargo test` passes (832 lib tests plus integration), `cargo fmt --check` clean, fixtures untouched.

## Self-Check: PASSED
