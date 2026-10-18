---
phase: 40-release-pipeline-installer
plan: 02
subsystem: release-pipeline
tags: [github-actions, bash, release]
requires: []
provides:
  - tag-triggered release workflow (validate, test, build, verify, package, publish)
  - pull_request CI workflow
  - release-check.sh / verify-exe.sh
affects: [40-04, 40-05, 40-08]
tech-stack:
  added: [GitHub Actions]
  patterns: [draft-then-publish, env-only step outputs]
key-files:
  created:
    - installer/release-check.sh
    - installer/release-check.test.sh
    - installer/verify-exe.sh
    - .github/workflows/ci.yml
    - .github/workflows/release.yml
  modified:
    - .gitattributes
    - .gitignore
decisions:
  - "Exe version check is bash (verify-exe.sh), no PowerShell anywhere"
  - "*.sh forced to LF via .gitattributes"
  - "Step outputs reach run: text only through env:"
metrics:
  completed: 2026-10-18
requirements: [REL-01, REL-02, REL-03, REL-04]
---

# Phase 40 Plan 02: Release workflows and validation scripts Summary

Tag-only release workflow and PR-only CI, with tested bash scripts that validate the tag against Cargo.toml and CHANGELOG and write the release notes.

## Tasks

| Task | Commit | Result |
|------|--------|--------|
| 1 release-check.sh + tests | bc6b187 | 13 cases pass (bad tags, mismatch, pre-release/final notes, blank section, CRLF, link-reference stop, real CHANGELOG) |
| 2 workflows, verify-exe.sh, attributes | 588ea3c | all acceptance greps pass |

## Verification run

- `bash installer/release-check.test.sh`: all cases passed.
- `release-check.sh v9.9.9` exits 1; `FF_CARGO_VERSION=0.7.0 ... v0.7.0` exits 1 (no [0.7.0] section).
- Acceptance greps: triggers, pinned SHAs, draft flags, no `--cleanup-tag`, no pwsh/workspace/ff-art, no `${{` inside any `run:` body (awk prints 0), `state=$(gh release view ...|| true)` present, three .sh files mode 100755, rust-toolchain.toml / CHANGELOG.md / README.md untouched.
- Workflows were not run or pushed. `verify-exe.sh` was only syntax-checked (`bash -n`); it fails on ff.exe until plan 40-04 adds the resource. release.yml calls `installer/package.sh`, which is plan 40-05.

## Deviations from Plan

**1. [Rule 3 - blocking] CR check in the test script.** `grep -c $'\r'` is unreliable under Git Bash here (all expected-OK cases falsely failed), so the test uses `tr -cd '\r' | wc -c`. Same intent. No other deviations.

The "exact bracket match" behavior case is covered by `final-uses-own-section` (a `## [0.7.0]` request does not match the `## [0.7.0-rc.1]` heading above it) rather than a separate case.

## Decisions

- Draft-state handling: a draft of the same tag is deleted (no `--cleanup-tag`), a published release makes the step fail.
- `--prerelease` is passed through a bash array so it is absent for final tags.

## For the Release Review

- Nothing user-visible for CHANGELOG from this plan (pipeline only). The first release notes for `v0.7.0-rc.1` come from the existing `[Unreleased]` text; the author should review it before the tag is pushed.
- Floating nightly is used in CI (accepted); `rustc -vV` is logged each run.
- A final `v0.7.0` tag needs a `## [0.7.0]` CHANGELOG section first or the workflow fails by design.

## Threat Flags

None beyond the plan's threat model.

## Self-Check: PASSED

All five created files exist; commits bc6b187 and 588ea3c exist.
