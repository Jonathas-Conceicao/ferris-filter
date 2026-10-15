---
phase: 26-uat-feedback-fixes-3
plan: 06
subsystem: overlay
tags: [egui, rule-editor, defences, picker]
requires: [26-02, 26-04]
provides:
  - "+ condition picker on ConditionEntry with one Defences entry"
  - "present entries greyed but clickable; click focuses the existing card"
  - "one Defences card per rule with per-defence toggles"
affects: [26-08]
key-files:
  modified:
    - src/overlay/body.rs
    - src/overlay/rule_list.rs
    - src/overlay/grammar.rs
requirements: [UAT3-05, UAT3-06]
completed: 2026-10-14
---

# Phase 26 Plan 06: Defences card and duplicate-free condition picker

The picker offers a single Defences entry, entries already on the rule are greyed yet clickable (tooltip "Already on this rule") and focus the existing card instead of adding another, and a rule shows at most one Defences card.

## What was built

- `picker_popup` now works on `ConditionEntry` with a `present` input (greyed, clickable) next to `disabled` (greyed, not clickable). The action picker is wrapped with `keyword_entries` and behaves as before.
- `pick_entry` (pure): present -> sets `list.focus_line` (line-backed keyword), or `list.focus_card` (pending row / Defences) and emits nothing; absent keyword -> `pick_condition`; absent Defences -> `list.pending_defences` and writes nothing.
- `ListUi` gained `pending_defences: HashSet<BlockId>` and `focus_card: Option<(BlockId, FocusTarget)>` (`Pending(kw)` / `Defences`), cleared with the other per-row state.
- Focus choice (discretion): `scroll_to_me(Center)`, keyboard focus on the card's primary control, plus a 1 s accent outline (start time in egui temp data, `focus_flash`).
- `conditions_group` lays cards out from `conditions::card_plan`. `defences_card`: Armour / Evasion / Energy Shield circular toggles; each line of an on keyword shows its own operator dropdown and value control (slider) writing only its own line. Toggle on -> `AddLine` of `default_line`; off -> `RemoveLine`s for that keyword only (highest line first). Header X removes all defence lines (or discards the pending card). Pending Defences renders three off toggles and writes nothing.
- Between parts render as two single cards until 26-08.
- `grammar::condition_groups` is `#[cfg(test)]` (still used by its tests); `picker_label`/`picker_entries` likewise.

## Deviations from Plan

- Both tasks are one commit: the picker change and the Defences card edit the same functions in `conditions_group`, so splitting would have left a non-building intermediate state.
- Slider/operator edits on a defence line are tested at the pure-edit level (`int_edit`/`op_edit` on the line's own entry plus apply), because a headless click scan could not drive the slider. Toggle on/off, pending, idle byte-identity and one-card-per-rule are driven through the real UI.
- The "greyed but clickable" picker drawing is asserted by a source check; the popup itself is not simulated headless (same limitation as 26-04). The behaviour behind it is covered by `pick_entry` tests.

## Verification

Full `cargo test` passes (928 lib tests plus integration), `cargo fmt --check` clean, fixtures untouched.

## Known Stubs

None.

## Self-Check: PASSED
