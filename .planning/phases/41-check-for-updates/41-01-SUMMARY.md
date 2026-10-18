---
phase: 41-check-for-updates
plan: 01
subsystem: update
tags: [semver, ring, sha256, github-releases]
requires: []
provides:
  - "src/update.rs pure core: version rule, date text, SHA256SUMS parse, file hash, Release JSON, UpdateHttp seam, classify_latest, message_for"
affects: [41-02]
tech-stack:
  added: [semver (direct), ring (direct)]
  patterns: [pure message producer, injectable HTTP seam]
key-files:
  created: [src/update.rs]
  modified: [Cargo.toml, Cargo.lock, src/lib.rs]
key-decisions:
  - "Compare on CARGO_PKG_VERSION reduced to major.minor.patch; installer name derived from the validated tag"
  - "Download URL accepted only when equal (ASCII case-insensitive) to the exact project releases/download URL"
  - "Ready with no published_at reads '<version> available'"
requirements-completed: [UPD-01, UPD-02, UPD-03]
duration: 20min
completed: 2026-10-18
---

# Phase 41 Plan 01: Pure update-check core Summary

Framework-free `src/update.rs` holding every update decision as a tested pure function, with `semver` and `ring` as direct dependencies (no new package in Cargo.lock).

## Tasks

1. Dependencies, module, version rule, dates, sums, file hash: f0ad768
2. Release types, UpdateHttp seam, classification, message table: 031acb1

## Verification

`cargo test --locked --lib update::` passes (11 tests). `Cargo.lock` gained only two dependency edges on the `ferrisfilter` package, no `[[package]]` entry. No clippy warnings in `src/update.rs`.

## Deviations from Plan

- [Rule 3] `compare_release` returns `Result<Compare, ()>` as specified; added `#[allow(clippy::result_unit_err)]` to keep it clippy-clean.
- Plan wrote tests and code in one commit per task rather than separate RED/GREEN commits.

## Decisions

- Ready/UpToDate messages use the offset-adjusted local date through `release_date_text`.
- `…` is written as the literal U+2026 character in source.

## For the Release Review

Nothing for README/CHANGELOG from this plan (core only; the user-visible button comes later in the phase).

## Self-Check: PASSED

src/update.rs exists; commits f0ad768 and 031acb1 exist.
