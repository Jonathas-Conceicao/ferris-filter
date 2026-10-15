---
phase: 28-default-config-files
plan: 04
subsystem: repo config / docs
tags: [gitignore, config, docs]
requires: [28-01, 28-02, 28-03]
provides: [live config untracked, docs for the default/live split, UAT5 checklist]
affects: [.gitignore, CLAUDE.md, CHANGELOG.md]
tech-stack:
  added: []
  patterns: [committed -default.json defaults + git-ignored live files]
key-files:
  created: [.planning/phases/28-default-config-files/28-MANUAL-VERIFICATION.md]
  modified: [.gitignore, CLAUDE.md, CHANGELOG.md]
  removed-from-index: [styles.json, tiers.json]
decisions:
  - "styles.json / tiers.json are untracked (index only) and git-ignored in their own commit; the author's working-tree copies are untouched"
metrics:
  duration: ~6 min
  completed: 2026-10-15
---

# Phase 28 Plan 04: Untrack live config and document the split Summary

The live `styles.json` / `tiers.json` left git (index only, now ignored) in their own commit, and
CLAUDE.md, CHANGELOG and a UAT5 checklist describe the default/live split, fallback, marker and
reset-by-delete.

## Tasks

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1 | Ignore and untrack the live config files | fe99fe5 | .gitignore, styles.json (deleted from index), tiers.json (deleted from index) |
| 2 | CLAUDE.md, CHANGELOG, manual verification | af7f456 | CLAUDE.md, CHANGELOG.md, 28-MANUAL-VERIFICATION.md |

## Verification

- `git ls-files styles.json tiers.json` prints nothing; `git check-ignore` lists both; the defaults are tracked.
- sha256 of working-tree `styles.json` (073ac46f...) and `tiers.json` (3f369798...) equals the parked
  copies before, after the untracking commit and at plan end. No restore was needed.
- Full `cargo test` with the author's live files present: 1137 passed, 0 failed (after untracking and
  again after the docs commit). `cargo fmt --check` clean. `tests/` has no changes, so
  `tests/fixtures/seed-universal.filter` is byte-identical.
- Untracked author files were not staged.

## Deviations from Plan

None - plan executed as written. The checklist's setup also copies `prices.json` into the temp app root
so the `ff-cli apply` check (UAT5-06) has prices to read.

## Self-Check: PASSED

- FOUND: .planning/phases/28-default-config-files/28-MANUAL-VERIFICATION.md
- FOUND: fe99fe5, af7f456
