---
phase: 32-rule-dividers-economy-banner
plan: 04
subsystem: overlay-rule-model
tags: [rust, overlay, sections, rule-model]

requires: [32-01, 32-03]
provides:
  - "WorkingFilter::{sections, economy_rows, entries, shadowing}"
  - "SectionRow, ListEntry, RuleRow.section, ActionOutcome.section"
  - "RowAction::{AddSection, AddSectionAbove, RenameSection, RemoveSection, MoveSection}"
affects: [32-05]

key-files:
  modified:
    - src/overlay/rules.rs
    - src/filter/divider.rs
    - src/filter/section.rs
  created:
    - .planning/phases/32-rule-dividers-economy-banner/shots/04-regression.png

key-decisions:
  - "Row can_move flags come from one file-order layout pass (units, banners, economy banner), checked equal to Document::rule_step in tests"
  - "ListEntry::Header.section indexes the projected sections(); RuleRow.section is the document banner ordinal"
  - "Economy rows are separate read-only RuleRows (index 0); rule actions on any block that is not a row are refused with the economy message"

requirements-completed: []  # model only; SECT-02..05 become user-visible with the UI in plan 05

duration: ~60min
completed: 2026-10-16
---

# Phase 32 Plan 04: Sections in the rule model Summary

**`WorkingFilter` now exposes sections, read-only economy rows and a collapse/search-aware display entry list, routes the five section actions to the Plan 03 Document ops, hops rules across dividers, and computes the WR-02 shadowing set.**

## Accomplishments
- `sections()` (projected with the staged economy add/remove), `economy_rows()`, `entries(query, collapsed)` (collapse hides rules; a search ignores collapse, lists headers only for sections with a match, `m of n` via `matches`), `shadowing()`.
- `rebuild` stays one pass: a layout of units and banners gives row `section` and `can_move_*` without per-row `rule_step`/`group::analyze`.
- Section actions are in-memory edits (dirty, undone by Revert/Reload) with the exact status copy; rename is sanitised; economy targets are refused and change nothing.
- `move_rule` uses `Document::rule_step`: `Swap` as before, `CrossBanner(k)` performs `move_section(k, !up)` (groups hop as one, no Continue flag).

## Task Commits
1. Sections, economy rows, entries, WR-02 set - `444e720`
2. Section actions, rule hops, economy guard, regression shot - `ce0438d`

## Gates
- `cargo test` all green (1225 lib tests plus integration); `cargo fmt --all --check` clean; clippy `^warning` 126 (baseline); `tests/fixtures` unchanged.
- Regression shot `shots/04-regression.png` (RULES tab, sample filter): identical look to before. Six rules, search hint `Search 6 rules, ...`, `+ Add rule`, rule 1 up and rule 6 down arrows greyed, others enabled, footer `+ 60 locked rule blocks (economy)` with Remove.

## Deviations from Plan
**1. [Rule 1 - Bug] rule_step let a rule hop up into the economy section** (`src/filter/divider.rs`, Plan 03 code)
- The economy section runs to the next banner, so the first rule below the divider that follows it could hop up over that divider and land inside the economy section, where Apply regenerates it away. `rule_step` now returns `Blocked` for an up-hop over a banner whose previous banner is the economy one; the row flags mirror it. Covered by the existing OD-3 test and the new rule_step equality test.

**2. [Rule 1 - Bug] `sanitize_name` left double spaces** (`src/filter/section.rs`)
- A newline inside a name became a space next to existing spaces (`New   name`). Whitespace runs now collapse to one space, matching the plan's `"New name"` expectation.

**3. Test replaced:** `move_flags_agree_with_move_neighbour` (swap-only) was superseded by `rule_move_flags_equal_rule_step`, which also covers adjacent banners and a group at a section edge.

## Known Stubs
None.

## Threat Flags
None. T-32-13 (`economy_targets_are_refused_and_nothing_changes`), T-32-14 (`rename_sanitises_and_remove_keeps_the_rules`), T-32-15 (single-pass layout, equality test) covered.

## Self-Check: PASSED
- Commits 444e720 and ce0438d exist; shots/04-regression.png present.
