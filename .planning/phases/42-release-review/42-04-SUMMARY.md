---
phase: 42-release-review
plan: 04
subsystem: docs
tags: [claude-md, roadmap, release-review]
requirements: [REV-01]
key-files:
  modified: [CLAUDE.md, .planning/ROADMAP.md]
metrics:
  completed: 2026-10-18
---

# Phase 42 Plan 04: CLAUDE.md v0.7 sweep and ROADMAP Phase 40 wording Summary

CLAUDE.md is free of v0.7-internal ids and agrees with the code; ROADMAP Phase 40 criterion 1 now describes the draft-then-publish workflow.

## Tasks

1. CLAUDE.md id sweep and consistency pass: commit 8c6ad48
2. ROADMAP Phase 40 criterion 1: commit aac1e3e

## Sweep

The id/regression/iteration grep over lines added since v0.6 found one hit: the heading `Check for updates (Phase 41):`, now `Check for updates:`. No added sentence narrated an intra-v0.7 bug.

## Checklist

- a. Per-user folder: already correct (`%APPDATA%\FerrisFilter` / `FERRISFILTER_HOME`, no walk-up, built-in defaults).
- b. Game folder: fixed: the "Source folder for filters in-game" line hard-coded the author's OneDrive path; it now reads `<Documents>\My Games\Path of Exile 2` and names `FERRISFILTER_GAME_DIR` / saved `game_folder`. The apply / install rows were already correct.
- c. Built-in styles: already correct (nine built-ins; Recycle, kite, exceptional marked as the author's own, not shipped).
- d. League: already correct (`OVERLAY_LEAGUE` absent from src and from CLAUDE.md; league picker described).
- e. prices.json: fixed: the row now says `sync` writes into the FerrisFilter folder while `generate` reads `prices.json` from the working directory.
- f. uwu.mp3: already correct (only the grammar example remains; no `assets/` claim).
- g. Version: already correct (the only `0.7.0-rc.1` mention is the rc ordering example).
- h. Screenshot flags: already correct (ff row matches the header of `src/overlay/screenshot.rs`).

## ROADMAP Phase 40 criterion 1

"A `v0.7.0-rc` tag runs the tests and release build, creates a draft Release, uploads and verifies the installer, portable zip and `SHA256SUMS.txt` with the CHANGELOG section as notes, then publishes it automatically (an `-rc.N` tag as a pre-release)". The goal line ("draft-first") was accurate and left alone.

## Deviations

None. Python was unavailable, so edits were made with the Edit tool.

## Self-Check: PASSED
