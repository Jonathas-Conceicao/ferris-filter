---
phase: 27-uat-feedback-fixes-4
verified: 2026-10-15T00:00:00Z
status: passed
score: 4/4 must-haves verified (automated); GUI behaviour pending human UAT
overrides_applied: 0
human_verification:
  - test: "UAT4-01 CONFIG Custom styles Order column"
    expected: "Order header and up/down arrows are right-aligned at the table's right edge, like the RULES arrows; Built-in table unchanged"
    why_human: "Real egui layout"
  - test: "UAT4-02 open minimap colour, beam colour and CONFIG style-editor colour dropdowns and scroll"
    expected: "Scroll bar sits at the popup's right edge and never draws over a colour name"
    why_human: "Hover-expanded floating scroll bar in a real window"
  - test: "UAT4-03 Simple Defences card in a real rule"
    expected: "Any | Has | No per AR/EV/ES writes nothing / X > 0 / X 0; 7-chip strip lit correctly; picker greys both defence entries and focuses the existing card; Defences card does not flip to Simple while editing"
    why_human: "Popup picks, focus and chip appearance"
---

# Phase 27: UAT Feedback Fixes 4 Verification Report

**Phase Goal:** Fourth v0.5 UAT round applied: right-aligned Order column, colour dropdown scroll bar fix, Simple Defences condition beside Defences, UAT results of Phases 16 and 22-26 recorded.
**Status:** human_needed (no automated gaps found)
**Re-verification:** No, initial verification

## Evidence (checked in code, not from SUMMARY claims)

- SC1 / UAT4-01: `src/overlay/config_view.rs` draws the Order head cell and row order cell inside `Layout::right_to_left` (lines ~644, ~684, ~844; `cell_right` helper at ~602). Alignment test `styles_table_header_and_columns` asserts the head and row right edges match.
- SC2 / UAT4-02: `colour_combo` in `src/overlay/body.rs` (~2971) puts the label in a `top_down_justified` layout so rows fill the popup width. The root cause (floating scroll bar painted at the edge of a ScrollArea that shrank to its content) is recorded in the 27-01 SUMMARY. `colour_combo_tests` (2 tests) passed when I ran them; the plan reports the first test fails with the fix removed.
- SC3 / UAT4-03:
  - Pure logic in `src/overlay/conditions.rs`: `zero_based`, `simple_defences`, `defences_kind`, `seg_write`, `DEFENCE_COMBOS`, `lit_combos`. `cargo test --lib overlay::conditions`: 29 passed.
  - `ConditionEntry::SimpleDefences` in `src/overlay/grammar.rs`, placed in the Item group by a sentinel; `body.rs` `pick_entry` greys and focuses on either entry (`FocusTarget::Defences`) and `simple_defences_card` is rendered at ~756.
  - Card stability: `ListUi.advanced_defences` / `pending_simple_defences` set and cleared in `body.rs`, with tests.
  - `cargo test --lib overlay::body`: 114 passed.
- SC4 / UAT4-04: ROADMAP has backlog 999.12 and 999.13 (lines 677, 689). All Result fields in the 24, 25 and 26 manual verification docs read pass (grep for non-pass results returned nothing). CLAUDE.md and CHANGELOG `[Unreleased]` document Simple Defences.
- 27-MANUAL-VERIFICATION.md exists with UAT4-01..03 items.

## Requirements Coverage

| Req | Plans | Status |
|-----|-------|--------|
| UAT4-01 | 01 | Code and test present; layout needs human check |
| UAT4-02 | 01 | Code and headless test present; real window check needed |
| UAT4-03 | 02, 04, 05 | SATISFIED in code and tests; card UI needs human check |
| UAT4-04 | 03 | SATISFIED (docs and backlog verified) |

All four IDs appear in plan frontmatter and REQUIREMENTS.md; no orphans. REQUIREMENTS.md rows still Pending, expected until UAT.

## Anti-Patterns

No debt markers found in `conditions.rs`. Untracked scratch files in the repo root and the local `styles.json` / `tiers.json` edits are not phase artifacts. Five `overlay::app` / `config_state` tests fail only with the author's local `styles.json` (pre-existing); the orchestrator reports a full `cargo test` pass with those files stashed.

## Gaps Summary

None found by automated checks. Status is human_needed because 27-MANUAL-VERIFICATION.md items (Order column layout, colour dropdown scroll bar, Simple Defences card behaviour) need a real window.

_Verifier: Claude (gsd-verifier)_

## Human UAT outcome (2026-10-15)

UAT complete (v0.5 wrap-up). UAT4-01..03 pass (author's report that Phases 24-27 passed); results in 27-MANUAL-VERIFICATION.md. Status changed from `human_needed` to `passed`.
