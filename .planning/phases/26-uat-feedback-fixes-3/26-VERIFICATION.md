---
phase: 26-uat-feedback-fixes-3
verified: 2026-10-14T00:00:00Z
status: human_needed
score: 8/8 must-haves verified (automated); GUI behaviour pending human UAT
overrides_applied: 0
human_verification:
  - test: "UAT3-01 CONFIG Styles tables (Built-in / Custom with Order column)"
    expected: "Heading -> column header -> rows per group; arrows in Order column"
    why_human: "Real egui layout"
  - test: "UAT3-02 rename a custom style in CONFIG, Apply"
    expected: "Rules stay linked; tag becomes '<id> <newname>'; legacy tag rewritten; built-ins not renamable"
    why_human: "Interactive flow across CONFIG/RULES tabs and disk write"
  - test: "UAT3-03 add style in CONFIG without Apply, open RULES"
    expected: "Style appears in the rule style picker"
    why_human: "Interactive"
  - test: "UAT3-04 expand rule, add Class, pick first class"
    expected: "Row does not collapse; operator shown before any value"
    why_human: "First-click collapse is an egui interaction bug; headless tests cannot reproduce a real click"
  - test: "UAT3-05/06/07 Defences card, picker greyed entries, Between, collapsed picker sections"
    expected: "As in 26-MANUAL-VERIFICATION.md items 5-7"
    why_human: "Popup picks and focus behaviour"
  - test: "UAT3-08 tray Quit with and without unapplied edits"
    expected: "Apply & quit / Quit without applying / Cancel; immediate exit when nothing pending"
    why_human: "Modal dialog and tray icon"
---

# Phase 26: UAT Feedback Fixes 3 Verification Report

**Phase Goal:** Third v0.5 UAT round applied: grouped CONFIG styles table with renamable id-linked custom styles, steadier rule editor, Quit that confirms before discarding edits.
**Status:** human_needed (no automated gaps found)
**Re-verification:** No, initial verification

## Evidence

- `cargo test` (CARGO_TARGET_DIR in temp): 976 lib tests pass, every integration suite passes with 0 failures, including `artifact_guard` and `generate_snapshot` (SC2 byte-identical fixtures).
- `styles.json` carries `id` fields (12 entries, `builtin:<name>` for built-ins).
- `rename_custom` exists in `src/models/config.rs` (line 614) with tests for position/id retention and refusals.
- `# ff:style <id> <name>` tag handling exists and is tested in `src/overlay/app.rs` (legacy `# ff:style kite` rewritten to `<id> locator` on Apply).
- `BaseDefencePercentile` is gone from the keyword table (only appears in an "unknown keyword" test and in raw-row/lossless tests); `ConditionEntry::Defences` exists with focus-existing-card logic (`src/overlay/body.rs`).
- Between: `between_switch`, `RowAction::InsertLineAfter`, `sole_line`/`can_between` exist and are tested (CR-02/WR-03 fixes).
- Picker sections: `PICKER_SECTION_DEFAULT_OPEN` used, with a source-guard test.
- Quit: pure `apply::quit_step` / `quit_step_shows_confirm` wired in `app.rs` at the tray Quit handler, the Choice handler and ApplyFinished.

## Requirements Coverage

| Req | Plans | Status |
|-----|-------|--------|
| UAT3-01 | 07, 09 | Code present, layout needs human check |
| UAT3-02 | 01, 05, 07, 09 | SATISFIED in code and tests; human recheck of rename flow |
| UAT3-03 | 05, 07, 09 | SATISFIED in code/tests (sync_working_styles); human recheck |
| UAT3-04 | 04, 09 | Regression tests pass; real click needs human |
| UAT3-05 | 02, 06, 09 | SATISFIED in code/tests |
| UAT3-06 | 02, 03, 06, 08, 09 | SATISFIED in code/tests |
| UAT3-07 | 04, 09 | SATISFIED in code, guard test; visual human check |
| UAT3-08 | 03, 09 | Pure logic tested; modal needs human |

All 8 IDs appear in plan frontmatter and in REQUIREMENTS.md; no orphans. REQUIREMENTS.md still marks them Pending (expected until UAT).

## Review Findings

26-REVIEW-FIX.md reports 9 of 10 findings fixed (status partial), 1 skipped; fixes marked "requires human verification" (CR-01 revert/migration, CR-02 Between strict bounds). Review the skipped finding before closing the phase.

## Anti-Patterns

No blockers seen. Untracked scratch files in the repo root are not phase artifacts.

## Gaps Summary

None found by automated checks. Status is human_needed because 26-MANUAL-VERIFICATION.md items 1-8 (modal, popup, real-window behaviour) remain unexecuted.

_Verifier: Claude (gsd-verifier)_
