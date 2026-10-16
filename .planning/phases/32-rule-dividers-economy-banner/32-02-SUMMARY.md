---
phase: 32-rule-dividers-economy-banner
plan: 02
subsystem: overlay-foundations
tags: [rust, settings, virtual-list, icons]

requires: [32-01]
provides:
  - "Settings.collapsed_sections + section_collapsed / set_section_collapsed / copy_section_state + ECONOMY_SECTION_KEY"
  - "virtual_list::build_offsets_mixed"
  - "FlagIcon::ChevronRight, ChevronDown, Rename"
affects: [32-05]

key-files:
  created:
    - .planning/phases/32-rule-dividers-economy-banner/shots/02-regression.png
  modified:
    - src/settings.rs
    - src/overlay/app.rs
    - src/overlay/virtual_list.rs
    - src/skin/icons.rs

key-decisions:
  - "Economy collapsed state lives under the reserved key `ff:economy`, so a user section named Economy never shares it"
  - "A wrongly typed collapsed_sections drops only that key to defaults, with a warning naming it"

requirements-completed: []  # SECT-03 foundations only; the UI lands in plan 05

duration: ~20min
completed: 2026-10-16
---

# Phase 32 Plan 02: Settings, mixed offsets and section icons Summary

**Per-filter collapsed-section state in ff-settings.json (with salvage), mixed-height virtual list offsets, and painted chevron and pencil icons, all unit-tested and not yet wired to the UI.**

## Accomplishments
- `Settings::collapsed_sections` (filter -> section -> collapsed), skipped when empty, with the three helpers; absent state means user sections expanded and the economy section collapsed. The salvage branch in `load` handles a wrongly typed value without losing `filter`, `reload_after_apply` or unknown keys.
- `build_offsets_mixed(heights, expanded)`; `build_offsets` now delegates to it (equivalence tested for n in 0..5).
- `FlagIcon::ChevronRight`, `ChevronDown`, `Rename`, painted with the existing stroke and no new colours; icon test array grown to 9.

## Task Commits
1. Task 1: collapsed sections in settings - `2a6149f`
2. Task 2: mixed offsets, icons, regression shot - `af724ce`

## Gates
- `cargo test`: all green (1200 lib tests plus integration suites); `git status --porcelain tests/fixtures` empty.
- `cargo fmt --all --check`: clean.
- clippy `^warning` count: 126 (baseline).

## Regression screenshot
`shots/02-regression.png` (RULES tab on the 32-01 sample filter): identical to before this plan. Six rules, search hint `Search 6 rules, ...`, `+ Add rule` button, footer `+ 60 locked rule blocks (economy)` with Remove, status `Filter out of date - press Apply`. Nothing from this plan is visible yet, as intended.

## Deviations from Plan

**1. [Rule 3 - Blocking] `settings_with_filter` listed Settings fields explicitly**
- **Issue:** adding the field broke compilation in `src/overlay/app.rs`, and a naive fix would also have dropped collapsed state whenever the filter was switched.
- **Fix:** it now uses `..current.clone()`, so every field (and future ones) is preserved.
- **Files:** src/overlay/app.rs (commit 2a6149f)

## Known Stubs
None (the new APIs are consumed by Plan 05).

## Threat Flags
None. T-32-06 is covered by `wrongly_typed_collapsed_sections_keeps_other_keys`.

## Self-Check: PASSED
- src/settings.rs, src/overlay/virtual_list.rs, src/skin/icons.rs, shots/02-regression.png present; commits 2a6149f and af724ce exist.
