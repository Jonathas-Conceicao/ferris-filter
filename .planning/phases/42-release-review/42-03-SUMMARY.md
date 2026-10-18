---
phase: 42-release-review
plan: 03
subsystem: release
tags: [version, changelog, release]
requires: [42-01]
provides: [version-0.7.0, changelog-0.7.0-draft]
affects: [Cargo.toml, Cargo.lock, CHANGELOG.md]
key-files:
  modified: [Cargo.toml, Cargo.lock, CHANGELOG.md]
requirements-completed: [REV-03, REV-04]
completed: 2026-10-18
---

# Phase 42 Plan 03: Version 0.7.0 and CHANGELOG draft Summary

Root crate bumped to 0.7.0 and a short 0.7.0 CHANGELOG draft written for the author's rewrite (REV-05).

## Done

- `Cargo.toml` 0.7.0-rc.1 to 0.7.0; `Cargo.lock` only the `ferrisfilter` entry changed
  (`cargo update --workspace --offline`); `tools/ff-art` untouched.
  `cargo build --locked --all-targets` passes.
- CHANGELOG: `## [Unreleased]` now empty; `## [0.7.0] - 2026-10-18` added with a bold summary line,
  Added (installer and portable zip, Game folder row, Strictness, Reset to defaults, league picker
  and Live/Fallback source, Check for updates, Ferris icon), Changed (`%APPDATA%\FerrisFilter`
  and built-in defaults, CLI follows saved folder, styles S-F/unchanged/hidden only, league chosen,
  About version and user agent, GPL licence), Removed (`uwu.mp3`), Fixed (duplicated minimap
  tooltip, FIX-01, the only v0.6 bug in REQUIREMENTS).
- Links: all `poe2-myfilter` replaced by `ferris-filter`; `[Unreleased]` compares `v0.7.0...HEAD`;
  `[0.7.0]` compares `v0.6...v0.7.0`.
- `installer/release-check.sh v0.7.0` exits 0 (notes contain the section);
  `installer/release-check.test.sh` all cases passed. Id/regression sweep of the section: 0 hits.

## Decisions

- The removed `Recycle`/`kite`/`exceptional` styles are said once, under Changed; Removed holds only `uwu.mp3`.
- The folded About entry was reworded to the v0.7 behaviour (`0.7.0` / `0.7.0-dev`, per build.rs).
- Version/CHANGELOG committed together in one commit (plan allowed it).
- Bullets I could not confirm from CLAUDE.md/deferred notes were left out (no per-bug items beyond FIX-01).

## Deviations from Plan

None.

## Commits

- 28ff433: docs(42-03): version 0.7.0 and CHANGELOG draft

## Self-Check: PASSED
