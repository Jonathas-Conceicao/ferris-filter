---
phase: 25-uat-feedback-fixes-2
verified: 2026-10-14T00:00:00Z
status: passed
score: 10/10 must-haves verified (code level)
overrides_applied: 0
gaps: []
human_verification:
  - test: "Run 25-MANUAL-VERIFICATION.md in the live overlay (with FERRISFILTER_HOME and USERPROFILE pointed at a temp copy)"
    expected: "Continue is the last body line; '+ style' button; Show/Hide only under Custom; name box without toggle; Sockets slider 0-6 with no '+ socket'; Exceptional line is a raw row and Apply works; no config status line and no 'new' tag; Add style on the Custom heading; Visibility | Name | Preview header aligned and arrows reorder; one APPLY from either tab writes styles.json, tiers.json and the filter together, with correct refusal/OVERWRITE/RELOAD behaviour"
    why_human: "Visual layout, alignment and in-game reload behaviour cannot be checked by grep or unit tests"
  - test: "Review fix CR-01 (clean config follows disk, is neither guarded nor written)"
    expected: "Editing tiers.json by hand with no pending config edits: RULES Apply succeeds and leaves styles.json and tiers.json byte-identical; with pending config edits the refusal names the file"
    why_human: "State and ownership logic across tabs; the review marked it as requiring human verification"
---

# Phase 25: UAT Feedback Fixes 2 Verification Report

**Goal:** The author's second v0.5 UAT round is applied: tidier rule body and CONFIG styles table, and one all-or-nothing Apply that saves and applies config and rules together.
**Re-verification:** No (initial).

## Requirements (UAT2-01..UAT2-10), all present in PLAN frontmatter and REQUIREMENTS.md

Plan coverage: 25-01 claims UAT2-01..06, 25-02 claims UAT2-09/10, 25-03 claims UAT2-07..10, 25-04 claims all ten. REQUIREMENTS.md maps all ten to Phase 25. No orphans.

| ID | Evidence in code | Status |
|----|------------------|--------|
| UAT2-01 | `src/overlay/body.rs` ~1663-1714: "Continue to later rules" checkbox is pushed after the style section and is editable on a styled rule. A layout test asserts Continue sits below Show/Hide and below the action rows. | VERIFIED (look: human) |
| UAT2-02 | body.rs:1680 button text is `+ style`. The remaining "+ action" text is only a doc comment in grammar.rs. | VERIFIED |
| UAT2-03 | body.rs style section orders picker, then Show/Hide, then action rows. The layout test asserts the order. | VERIFIED (look: human) |
| UAT2-04 | The name is a plain `TextEdit::singleline` (body.rs:197), and the toggle is gone. Meta-edit tests in `src/filter/edit.rs` cover set and clear of `# ff:name`. | VERIFIED |
| UAT2-05 | grammar.rs:292 gives Sockets a `(0, 6)` slider. `ValueShape::Sockets` emits `{n} >= 1`. The colour entry is not offered (a `5GGG` colour suffix is preserved on write-back per the manual-verification step). | VERIFIED (slider look: human) |
| UAT2-06 | `Exceptional` is asserted unknown in the `src/filter/keywords.rs` test (the only mention outside tests of its removal). The body.rs:2283 test shows it loads as a raw row. | VERIFIED |
| UAT2-07 | config_view.rs test (~1698) asserts no "unapplied change" line and no "new"/"edited" badge text. | VERIFIED |
| UAT2-08 | config_view.rs:622 and :819: Add style is always drawn on the Custom heading, even before the first custom style exists. | VERIFIED (position: human) |
| UAT2-09 | config_view.rs: `COL_VISIBILITY`/name/`COL_PREVIEW` header (~575-592). `ConfigAction::MoveStyle` for customs only. A test (~1891) asserts `move_style("S", true)` is false, so built-ins never move. | VERIFIED (alignment: human) |
| UAT2-10 | `apply_all::prepare` validates before any write, and `commit` writes all targets with rollback. `app.rs` routes APPLY and OVERWRITE from either tab through `apply_everything`. I re-ran `cargo test --lib apply_all`: 10 passed, including `third_rename_failure_rolls_back_all`, `commit_time_refusal_leaves_no_ffbak`, `changed_on_disk_names_files_and_overwrite_writes_all`, `clean_config_is_neither_guarded_nor_written`. | VERIFIED (in-game reload and tab UX: human) |

## Review fix (CR-01)

An unedited styles.json/tiers.json now follows disk (`ConfigState::follow_disk`) and is neither guarded nor written. Treated as intended, per the orchestrator. It is covered by the test above and is listed for human confirmation.

## Tests

I re-ran `apply_all` (10/10 pass). The orchestrator reported the full suite (958 tests) passing and fmt clean. `tests/` fixtures were not touched by the review fixes.

## Anti-patterns

No unreferenced TBD/FIXME/XXX was found in the grepped areas. The known test-build `unused mut` warning at rule_list.rs:942 is pre-existing and minor.

## Human verification required

Run `25-MANUAL-VERIFICATION.md` (10 steps, one per requirement). Items cover visual layout, column alignment, slider appearance, the in-game reload after Apply, and the CR-01 cross-tab state behaviour.

## Gaps Summary

None at code level.

_Verifier: Claude (gsd-verifier)_

## Human UAT outcome (2026-10-15)

UAT complete (v0.5 wrap-up). All items pass; results in 25-MANUAL-VERIFICATION.md. Status changed from `human_needed` to `passed`.
