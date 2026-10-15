---
phase: 27-uat-feedback-fixes-4
plan: 05
subsystem: docs
tags: [docs, changelog, uat]
requires: [27-01, 27-02, 27-04]
provides: [CLAUDE.md Simple Defences notes, CHANGELOG Phase 27 entry, 27-MANUAL-VERIFICATION.md]
key-files:
  modified: [CLAUDE.md, CHANGELOG.md]
  created: [.planning/phases/27-uat-feedback-fixes-4/27-MANUAL-VERIFICATION.md]
metrics:
  completed: 2026-10-15
requirements: [UAT4-01, UAT4-02, UAT4-03]
---

# Phase 27 Plan 05: Docs and manual verification Summary

CLAUDE.md, CHANGELOG `[Unreleased]` and a three-item UAT4 checklist now describe Simple Defences, the right-aligned Order column and the colour dropdown fix.

## Tasks

| Task | Commit | Files |
| ---- | ------ | ----- |
| 1 + 2 | 84e729c | CLAUDE.md, CHANGELOG.md, 27-MANUAL-VERIFICATION.md |

- CLAUDE.md: Simple Defences (Any | Has | No, `X > 0` / `X 0`, 7-combination strip, shared slot with Defences in the picker, zero-based loading with bytes kept, card stability until Apply/Revert/RELOAD/removal, last row back to Any keeps the card open, no class check); Order column right-aligned.
- CHANGELOG: Phase 27 entry with the colour dropdown cause from 27-01.
- Manual checklist in the Phase 26 format with the FERRISFILTER_HOME/USERPROFILE safety note.

## Deviations from Plan

None. Both tasks went into one docs commit. The deviations recorded in 27-04 (card stays open at all Any; Apply re-parse, Revert and RELOAD reset the hold) are documented.

## Self-Check: PASSED

Files exist; commit 84e729c exists.
