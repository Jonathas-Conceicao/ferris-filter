---
phase: 24-uat-feedback-fixes
verified: 2026-10-13T00:00:00Z
status: human_needed
score: 13/13 must-haves verified (code level)
overrides_applied: 0
gaps: []
accepted_gaps:
  - id: WR-02
    note: "A styles.json with action Minimal is rejected (StyleAction never had Minimal). Pre-existing, skipped to keep the generator byte-identical. Accepted by the author; not a failure."
human_verification:
  - test: "Run 24-MANUAL-VERIFICATION.md in the live overlay"
    expected: "Slide switch look and feel, right-aligned X columns, 6-8 row scroll cap on string lists, Yes/No control, collapsed summary line, no reload message when the game is closed, Custom/preset switching"
    why_human: "Visual layout and in-game behaviour cannot be checked by grep"
---

# Phase 24: UAT Feedback Fixes Verification Report

**Goal:** The author's v0.5 UAT feedback applied to the overlay: less text, tighter controls, PoE2-only vocabulary.
**Re-verification:** No (initial).

## Requirements (UAT-01..UAT-13), all present in PLAN frontmatter and REQUIREMENTS.md

| ID | Evidence in code | Status |
|----|------------------|--------|
| UAT-01 | `skipped_reload_message` in src/overlay/app.rs: `GameNotRunning` yields only the written path; other reasons use `not_reloaded_message`. Test `skipped_reload_message_keeps_the_refresh_reminder` passes. | VERIFIED |
| UAT-02 | `style_groups` in src/overlay/config_view.rs partitions built-in and custom using the existing `badges().builtin`. No "built-in" badge on the row (only a reserved-name hint). | VERIFIED (divider and headings are visual: human) |
| UAT-03 | `widgets::slide_switch` (with tests) is used in `style_row`, and emits `StyleEdit::Action(Show/Hide)`. | VERIFIED |
| UAT-04 | `DISPOSITION_CHOICES = [Show, Hide]`. A foreign Minimal block is shown as a "Minimal" label. `Minimal` stays in the keyword table (test). No Minimal in config_view. | VERIFIED |
| UAT-05 | Condition card X uses `right_to_left` layout (body.rs ~787). | VERIFIED (alignment is visual) |
| UAT-06 | `widgets::string_list` vertical list with tests (one row per entry plus Remove, commit). | VERIFIED (scroll cap is visual) |
| UAT-07 | Chip-free class picker (commit beb8ff6). A test preserves the unknown class "Custom Thing" on write-back. | VERIFIED |
| UAT-08 | No style caption. The only `caption(...)` calls are section headings and field labels. | VERIFIED |
| UAT-09 | All 25 keywords are absent from `KEYWORDS`. `poe1_only_conditions_are_unknown` passes (the 25 names appear only in that test, and `MirageMap`, `Exceptional` and `Minimal` are kept). Round-trip, raw-row and Apply-gate tests pass: `gate_ignores_removed_poe1_keyword`, `removed_poe1_keyword_is_a_raw_row`, `removed_poe1_keywords_are_not_offered`. The `INFLUENCES` and `GEM_QUALITY` lists are gone. | VERIFIED |
| UAT-10 | GemLevel is `(1,21)` with default 20. `every_numeric_default_is_inside_its_slider` passes. | VERIFIED |
| UAT-11 | `style_section` is called last in the body. It has one picker (Custom plus presets), Show/Hide, and action rows only when `shows_action_rows`. Custom drops the tag. | VERIFIED |
| UAT-12 | `bool_yes_no` Yes/No control. The operator is kept, and only the value word is rewritten. Negating operators are read as the flipped value. | VERIFIED |
| UAT-13 | Summary strip in rule_list.rs (`ff-summary-click`). Review fixes WR-01 (click expands) and WR-03 (painted height equals the virtual list height) are applied with tests. | VERIFIED (performance at ~460 blocks: human) |

## Tests

I re-ran 8 targeted tests: all passed. The orchestrator reported the full suite (834 lib plus integration) and fmt clean at HEAD. The generator fixtures were untouched (no `tests/` changes in the review fixes).

## Anti-patterns

No unreferenced TBD/FIXME/XXX was found in the verified areas.

## Accepted gap

WR-02: a styles.json carrying `"action": "Minimal"` is rejected. This is pre-existing and was accepted, so it is recorded here and does not affect the status. It is a mild deviation from the UAT-04 context note "keep loading it".

## Human verification required

Run `24-MANUAL-VERIFICATION.md`. It covers the visual and in-game items marked above: group headings and divider, switch rendering, X alignment, list scroll cap, summary line and scrolling smoothness, and no reload message with the game closed.

_Verifier: Claude (gsd-verifier)_
