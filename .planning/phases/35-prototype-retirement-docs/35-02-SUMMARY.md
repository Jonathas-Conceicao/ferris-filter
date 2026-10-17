---
phase: 35-prototype-retirement-docs
plan: 02
subsystem: docs
tags: [readme, changelog, screenshots, version]
requires: ["35-01"]
provides:
  - "ferrisfilter 0.6.0 (Cargo.toml, Cargo.lock)"
  - "five overlay screenshots in docs/screenshots/"
  - "plain general README; CLAUDE.md and CHANGELOG corrected"
affects: []
key-files:
  created:
    - docs/screenshots/rules.png
    - docs/screenshots/rule-expanded.png
    - docs/screenshots/config-style.png
    - docs/screenshots/economy.png
    - docs/screenshots/about.png
  modified:
    - Cargo.toml
    - Cargo.lock
    - README.md
    - CLAUDE.md
    - CHANGELOG.md
requirements-completed: [CLEAN-02]
completed: 2026-10-17
---

# Phase 35 Plan 02: Docs and 0.6.0 Summary

**Package bumped to 0.6.0, README rewritten as a plain general page around five fresh screenshots, CLAUDE.md and CHANGELOG corrected.**

## Commits

- `c0d5730` chore(35-02): bump to 0.6.0 and add README screenshots
- `40cc952` docs(35-02): README, CLAUDE.md, CHANGELOG

## What changed

- Cargo.toml / Cargo.lock: `ferrisfilter` 0.6.0; the lock diff is that one line. About tab reads `Version 0.6.0` (checked in about.png).
- Screenshots (35-01 recipe: temp home, built `ff.exe` run directly) of a temp copy of `tests/ui/test_ff.filter` where the deliberate grammar-error rule was made valid (`Sockets >= 3`, renamed), so the status line is a benign amber "unknown style" notice rather than a red "Apply refused". `rule-expanded.png` uses `--expand 12`. All five were opened and checked; none shows a user path. `config-style.png` never settled byte-identically in the settle loop (scroll-animation timing); the kept capture was viewed and shows the kite editor fine.
- README: sections overlay (four tabs, hotkey, tray, windowed mode, Apply), command line, files, building, references, ff-art (unchanged), credits/disclaimer; `%USERPROFILE%` path, no `ff-proto`.
- CLAUDE.md: removed the `ff-proto` row, the `ff-proto`/`overlay::theme`/Cinzel mentions in the `src/skin/` row and the "until Phase 33" clause.
- CHANGELOG `[Unreleased]`: the two prototype entries merged into one (added, then removed), version entry says 0.6.0, Phase 35 `### Removed` and README `### Changed` lines added; no `[0.6.0]` heading.

## Gates

fmt clean; cargo test all green (1389 lib + integration, 0 failed); clippy count 121 (<= 121); `git diff tests/fixtures` empty; greps for `ff-proto|cinzel|overlay::theme` in README/CLAUDE.md/assets/src/tests/tools/Cargo.toml and `C:\Users\jonat` in README return nothing.

## Deviations from Plan

None beyond the screenshot filter copy described above (kept out of the repo, in the temp folder).

## Self-Check: PASSED
