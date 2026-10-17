---
phase: 35-prototype-retirement-docs
plan: 01
subsystem: ui
tags: [cleanup, skin, egui, screenshots]
requires: []
provides:
  - "ff-proto binary, overlay::theme and the Cinzel font are gone; the skin is the only overlay look"
  - "pal::RECESS, pal::CONTROL_GLYPH and fonts::SUMMARY_SIZE carry the values theme.rs used to hold"
affects: [35-02]
tech-stack:
  added: []
  patterns: ["before/after PNG pixel diff (shots/pngdiff.ps1) as the no-visual-change guard"]
key-files:
  created:
    - .planning/phases/35-prototype-retirement-docs/shots/pngdiff.ps1
    - .planning/phases/35-prototype-retirement-docs/shots/shoot.sh
  modified:
    - Cargo.toml
    - src/skin/palette.rs
    - src/skin/fonts.rs
    - src/skin/mod.rs
    - src/overlay/body.rs
    - src/overlay/config_view.rs
    - src/overlay/rule_list.rs
    - src/overlay/widgets.rs
    - src/overlay/apply.rs
    - src/overlay/shell.rs
    - src/overlay/mod.rs
    - assets/ASSETS.md
  deleted:
    - src/bin/ff-proto.rs
    - src/overlay/theme.rs
    - assets/Cinzel-Regular.ttf
    - assets/Cinzel-OFL.txt
key-decisions:
  - "Compared screenshots with a worktree-built old exe where scroll-animation timing made captures flaky"
requirements-completed: [CLEAN-01]
duration: ~1h
completed: 2026-10-17
---

# Phase 35 Plan 01: Prototype Retirement Summary

**ff-proto, overlay::theme and Cinzel removed; every former theme value now read from the skin, and 12 before/after screenshots diff to 0 pixels.**

## Commits

- `02620e8` chore(35-01): baseline gates, before screenshots, pngdiff.ps1 / shoot.sh
- `d2bd8e5` refactor(35-01): retire overlay::theme (call sites repointed at `pal::*` / `fonts::SUMMARY_SIZE`; `RECESS`, `CONTROL_GLYPH` added with a value test; unused `header_plate`, `zebra_row`, `HeaderResponse` deleted)
- `3c9132f` refactor(35-01): delete ff-proto and Cinzel (bin, `[[bin]]`, font files, `CINZEL` family, ASSETS.md rows and section, doc comments)

## Gate results

| Gate | Before | After |
|---|---|---|
| clippy --all-targets warning lines (`... \| grep -E "^warning" \| grep -v generated \| wc -l`) | 122 | 121 (<= baseline; CONTEXT's 126 used a different count) |
| cargo test, lib | 1392 passed | 1389 passed (4 theme tests removed, 1 palette test added) |
| cargo test, integration binaries | 111 passed | 111 passed, 0 failed |
| cargo fmt --check | n/a | clean |
| `git diff -- tests/fixtures` | empty | empty |
| `cargo build --bins` | | builds ferrisfilter, ff, ff-cli (a stale `ff-proto.exe` remains only in the out-of-repo scratch target dir) |
| `grep -rniE "ff-proto\|ff_proto\|cinzel\|theme::" src tests tools assets Cargo.toml` | | 0 hits |

Cargo.lock did not change (bin removal only).

## Visual diff (shots/diff.txt)

All 12 before/after pairs are identical (0 differing pixels): rules, rule12, rule14, rule16,
rule17, rule18, economy-rule, config, config-kite, config-style-S, economy, about.

Screenshot captures are not deterministic for scrolled views: the scroll offset depends on animation
timing, so two captures of the same binary can differ by a scroll offset (this affected rules
expands and `--expand-style kite`; the first before/repeat comparison showed it). Handling:
- `shoot.sh` repeats each capture until two consecutive captures are byte-identical; 10 of 11 shots settle.
- `config-kite` never settled reliably, so I built the old code (commit 02620e8, in a temporary git
  worktree, since removed) and captured kite 8 times with each exe; old capture 1 and new capture 5
  are byte-identical at the same scroll offset, and those are the files kept.
- `config-style-S` (`--expand-style S`) is deterministic and was added as an extra stable pair using the old exe.

No non-zero diff remains, so there was nothing to fix or explain.

## Deviations from Plan

**1. [Rule 3 - Blocking] Screenshot nondeterminism.** Found in Task 1; the plan anticipated it
("record as nondeterministic"). Resolved as described above rather than discounting a region.
**2. [Rule 3 - Blocking] Screenshot mode refuses to overwrite a staged filter from a previous run**, so
`shoot.sh` deletes the temp game folder before each capture.
**3. Clippy count** is 122 -> 121 by my counting command (not 126); the ceiling used is the measured baseline, as the plan says.
**4.** `shots/before-repeat/` was used only to establish determinism and was deleted afterwards.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

Files deleted, commits `02620e8`, `d2bd8e5`, `3c9132f` present, diff.txt all identical, gates above run on the final tree.
