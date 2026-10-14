---
phase: 25-uat-feedback-fixes-2
plan: 01
subsystem: overlay
tags: [rule-editor, egui, keywords]
requires: []
provides:
  - STYLE section layout (Continue last, "+ style", Custom-only Show/Hide)
  - plain rule-name text box; unnamed header is "<index>  <kw>"
  - count-only Sockets slider; Exceptional removed from KEYWORDS
affects: [25-03]
key-files:
  modified:
    - src/overlay/body.rs
    - src/overlay/rule_list.rs
    - src/overlay/rules.rs
    - src/filter/keywords.rs
    - src/overlay/grammar.rs
decisions:
  - "Search haystack also gets other_names, since the derived label no longer carries them"
metrics:
  tasks: 3
  completed: 2026-10-14
---

# Phase 25 Plan 01: Rule editor UAT2 fixes Summary

UAT2-01..06: STYLE section tidy, plain name box, count-only Sockets slider, `Exceptional` dropped from the keyword table.

## Commits
- aad4909: STYLE layout, "+ style", Custom-only Show/Hide, Sockets slider (UAT2-01/02/03/05)
- 6370046: name text box, unnamed label `<index>  <kw>` (UAT2-04)
- a371797: Exceptional removal (UAT2-06)

## Deviations from Plan

**1. [Rule 3 - Blocking] Worktree base reset.** HEAD merge-base differed from the expected base, so the worktree was reset to 03b7ccf before starting, as the startup check prescribes.

**2. Name-clearing test.** Headless egui (no fonts) does not process select-all/backspace in a TextEdit, so the clear path is tested through an extracted `name_action(id, draft)` helper (used by the UI) rather than by simulated keystrokes. Typing is tested through a real frame.

**3. Haystack.** `derived_pieces` was deleted (no other caller). The haystack now also adds `other_names` so rules remain searchable by those values.

## Verification
Full `cargo test` passes (840 lib tests plus integration, including artifact_guard and generate_snapshot); `cargo fmt --check` clean. No generator change.

## Known Stubs
None.

## Self-Check: PASSED
