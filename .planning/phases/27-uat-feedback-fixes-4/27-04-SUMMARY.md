---
phase: 27-uat-feedback-fixes-4
plan: 04
subsystem: overlay
tags: [overlay, rules-editor, conditions, defences, simple-defences, picker]
requires:
  - "27-02: conditions::defences_kind / SimpleSet / seg_write / lit_combos / DEFENCE_COMBOS / DEFENCE_SHORT"
provides:
  - "grammar::ConditionEntry::SimpleDefences (Item group, right after Defences)"
  - "ListUi.pending_simple_defences and ListUi.advanced_defences (cleared by clear_ids)"
  - "body::simple_defences_card, any_has_no, combo_strip, simple_seg_actions, combo_chip_states"
affects: [27-05]
tech-stack:
  added: []
  patterns:
    - "per-rule advanced marker keeps the card kind stable until a reload"
    - "test-only rect/card probes in egui temp data (#[cfg(test)]) to click controls headless"
key-files:
  created: []
  modified:
    - src/overlay/grammar.rs
    - src/overlay/conditions.rs
    - src/overlay/rule_list.rs
    - src/overlay/body.rs
decisions:
  - "Defences and Simple Defences are one slot: any defence line or either pending card greys both picker entries; either click focuses the shown card (FocusTarget::Defences)"
  - "advanced_defences is set when Defences is picked or the Defences card renders, cleared by clear_ids (Revert, RELOAD and the Apply re-parse count as a reload), either card's X, and a Simple Defences pick"
  - "Setting the last non-Any row of a Simple card back to Any keeps the card open as a pending card instead of letting it vanish"
metrics:
  duration: "~35 min"
  completed: 2026-10-15
  tasks: 3
  files: 4
requirements: [UAT4-03]
---

# Phase 27 Plan 04: Simple Defences card Summary

The Simple Defences card in the RULES editor (UAT4-03): a picker entry right after Defences, mutual
exclusivity with the Defences card, AR / EV / ES rows of Any | Has | No that write one zero-based
line each (`X > 0`, `X 0`, or remove), a non-clickable seven-chip combination strip, byte-preserved
loaded lines, and a per-rule marker so a Defences card never flips to Simple while editing.

## What was built

- **grammar.rs:** `ConditionEntry::SimpleDefences` (label "Simple Defences", search text includes
  simple / armour / evasion / energy shield / ar / ev / es), `SIMPLE_DEFENCES_MARKER` after
  `DEFENCES_MARKER`; the legacy keyword projection expands it to nothing.
- **conditions.rs:** `present_entries` pushes both `Defences` and `SimpleDefences` for any defence
  line or pending card (callers pass "either pending card").
- **rule_list.rs:** `pending_simple_defences` (retained like `pending_defences` on collapse/switch)
  and `advanced_defences`, both cleared by `clear_ids`.
- **body.rs:**
  - `pick_entry`: present -> focus for either entry; absent Defences -> pending + advanced; absent
    Simple Defences -> pending simple, advanced removed.
  - `conditions_group` dispatches the `CardPlan::Defences` position on
    `conditions::defences_kind(&defence_lines, advanced)`; the Defences card marks the rule advanced
    whenever it renders (before its toggles run, so the first `BaseArmour >= 1` is already covered).
  - `simple_defences_card`: header "Simple Defences" with X (removes every defence line
    highest-first, clears pending simple + advanced), rows labelled AR / EV / ES with tooltips
    Armour / Evasion / Energy Shield, `any_has_no` segments (tooltips "With or without",
    "More than 0", "Exactly 0"), one write via `simple_seg_actions` -> `conditions::seg_write`
    (Add -> `AddLine`, Replace -> `EditLine(LinePatch::Whole)` on that type's line, Remove ->
    `RemoveLine`), focus/flash like the Defences card.
  - `combo_strip`: seven chips painted with `Sense::hover()` only, lit = accent fill with dark text,
    dimmed = outline with disabled text, one tooltip on the strip; lit flags from
    `combo_chip_states` (pure, tested).
  - The Defences card X also clears `advanced_defences`. No other change to the Defences card.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | dd16bd3 | feat(27-04): Simple Defences picker entry, exclusivity and per-rule state |
| 2 | 2ca14cd | feat(27-04): Simple Defences card with Any/Has/No rows and combination strip |
| 3 | 5c93e84 | test(27-04): card stability and defences exclusivity through the UI |

## Verification

- `cargo test --lib overlay::`: 493 passed, 5 failed. The 5 failures are the pre-existing
  styles.json/tiers.json baseline (app::clean_config_follows_outside_edits..., 
  app::apply_working_style_change..., config_state::load_reads_styles_and_tiers,
  config_state::loading_again_discards..., config_state::save_new_style_writes_it_last).
- `cargo test --lib overlay::body`: 114 passed (19 new tests across tasks 1-3, plus updated grammar,
  conditions and rule_list tests).
- `cargo test --test cli_generate`: 16 passed (fixtures byte-identical; no generator change).
- `cargo fmt --check` clean; `cargo build` no warnings; clippy reports nothing new in the touched code.
- Mutation check: removing the render-time `advanced_defences.insert` makes
  `loaded_defences_card_edited_to_zero_based_stays_defences` fail.
- styles.json / tiers.json still modified and uncommitted.

## Deviations from Plan

**1. [Rule 2 - UX correctness] Last row back to Any keeps the Simple card open**
- **Found during:** Task 2
- **Issue:** setting the only Has/No row back to Any removes the last defence line, so the card would
  disappear under the user's pointer.
- **Fix:** that write re-inserts `pending_simple_defences`, so the all-Any pending card stays
  (writes nothing; collapse or X discards it). Test `setting_the_last_simple_row_to_any_keeps_the_card_open`.
- **Commit:** 2ca14cd

**2. Headless click location via test-only probes**
- Segments, the strip, both X buttons and the Defences toggles record their rects (and which card a
  rule rendered) in egui temp data under `#[cfg(test)]`, so tests click real controls instead of
  scanning coordinates. All segment writes are covered both through real clicks and through the
  pure `simple_seg_actions` applied via `WorkingFilter::apply_action`. No production behaviour change.

**3. TDD commit shape:** tests and implementation landed together per task (Task 3 is test-only, as
Task 2 already set the marker before the first toggle); no separate RED commits.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: src/overlay/body.rs (fn simple_defences_card, fn any_has_no, fn combo_strip; defences_kind, seg_write, lit_combos used in non-test code)
- FOUND: src/overlay/grammar.rs SimpleDefences; src/overlay/rule_list.rs pending_simple_defences / advanced_defences
- FOUND commits: dd16bd3, 2ca14cd, 5c93e84
