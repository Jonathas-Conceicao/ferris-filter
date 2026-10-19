---
phase: 40-release-pipeline-installer
plan: 08
subsystem: ci
tags: [github-actions, release, dry-run]
requirements: [REL-01, REL-02, REL-03, REL-04]
key-files:
  modified: []
metrics:
  completed: 2026-10-18
---

# Phase 40 Plan 08: Release workflow dry run (v0.7.0-rc.1) Summary

One tag push of `v0.7.0-rc.1` ran `release.yml` green end to end and published a pre-release with
the three assets; the checksums matched after download.

## Result

- The tag sat on a side commit (8b57201: version `0.7.0-rc.1`, the 0.7.0 notes copied into
  `[Unreleased]`, never merged), because Phase 42 had already emptied `[Unreleased]`.
- Run 39166688609: release-check, tests, release build, version-resource check, packaging, draft
  then publish: all steps passed. Assets: `FerrisFilter-0.7.0-rc.1-setup.exe`,
  `FerrisFilter-0.7.0-rc.1-windows-x64.zip`, `SHA256SUMS.txt`; downloaded and verified.
- One tag push in total; no rerun or `-rc.2` needed.
- At milestone close the author had the pre-release and its tag deleted (2026-10-18).
