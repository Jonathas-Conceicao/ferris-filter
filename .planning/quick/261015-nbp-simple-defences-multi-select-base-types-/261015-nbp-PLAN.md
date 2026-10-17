---
quick_id: 261015-nbp
description: "Simple Defences: multi-select base types, written as grouped blocks"
date: 2026-10-15
mode: quick
branch: topic/dev
---

# Quick Task 261015-nbp: Simple Defences as multi-select base types, written as grouped blocks

Design approved by the author (brief, 2026-10-15). Outside any milestone.

## Goal

The Simple Defences card becomes one line of seven multi-select chips
(`Full AR` `Full EV` `Full ES` `AR/EV` `AR/ES` `EV/ES` `AR/EV/ES`). The selection is exactly
what the rule matches. A selection one filter block cannot express is written as n consecutive
identical blocks (differing only in their zero-based defence lines), each tagged
`# ff:group <id> <k>/<n>`, and the overlay treats such a group as ONE rule everywhere.

## Tasks

### Task 1: Pure group model (`src/filter/group.rs`) + document primitives

- files: `src/filter/group.rs` (new), `src/filter/mod.rs`, `src/filter/structure.rs`,
  `src/overlay/conditions.rs` (DefenceSeg moves to `filter::group`, re-exported)
- action:
  - `DefenceSeg`, `Constraint = [DefenceSeg; 3]`, the seven `COMBOS` in chip order, `mask_of`,
    `cover(selection) -> Vec<Constraint>` (minimum exact cover by product sets, ties: fewer
    lines, then a fixed order; 0 and 127 give one all-Any block), cached in a `OnceLock` table.
  - `parse_meta("<id> <k>/<n>")`, `fresh_gid(doc)`, zero-based reading of a block's defence
    lines (`block_constraint`), and `analyze(doc) -> GroupIndex` (roles Single / Leader(members) /
    Follower(leader) / Broken) with the validity rules: consecutive blocks, blank-only gaps,
    k = 1..n, same n, n >= 2, gid used by one run only, no economy block, not invalid-disabled,
    zero-based defence lines, identical span text apart from defence lines and the group line.
  - `Document::replace_blocks_after(anchor, old, new_spans)` (structural: drop the `old` blocks
    that directly follow `anchor`, insert new spans after it; old ids reused by position) and
    `Document::swap_runs(a, b)` (generalised `move_block` for adjacent block runs).
- verify: exhaustive test over all 127 selections (exact + minimal via independent BFS),
  analyze() valid/broken cases, primitives keep every other byte.
- done: unit tests green.

### Task 2: WorkingFilter groups (one rule = one row) + every op acts on the group

- files: `src/overlay/rules.rs`, `src/dynamic.rs` (blocking errors per rule), `src/styled.rs`
  (report counts and rule indices per rule)
- action:
  - rebuild(): followers are not rows; a leader row has index/flags/summary of the rule;
    `RuleRow.group_size`, `RuleRow.group_broken`, `RuleRow.simple_mask` (union for a group,
    matched combos for a zero-based single block); move flags from the run's ends.
  - `RowAction::SetDefenceSelection { id, mask }` (re-covers; adds/removes blocks; marker added
    with a fresh id past 1 block, removed at 1 block).
  - Grouped rule: SetEnabled / Delete / Duplicate (new gid, after the group) / Move (runs swap
    as units, also a single rule passing a group) act on all members; every other edit runs on
    the leader then re-syncs the followers from it (their own defence lines kept verbatim).
    Atomic: a failure restores the snapshot.
  - blocking errors and style report count each group once, with row-numbered indices.
- verify: tests for load as one row, byte identity through load/Apply text, each op keeps
  copies identical, broken groups are separate rows with the flag, Apply gate.
- done: `cargo test` green.

### Task 3: Chip card UI, flag, docs

- files: `src/overlay/body.rs`, `src/overlay/rule_list.rs`, `src/overlay/app.rs`,
  `CLAUDE.md`, `CHANGELOG.md`
- action: replace Any|Has|No rows and the combination strip with one wrapped line of seven
  toggle chips (tooltip names the base type); a click emits `SetDefenceSelection`; X collapses
  to no defence lines; `group broken` flag with tooltip; docs updated.
- verify: egui harness tests (click chip -> one action, group card shows union), fmt, full
  test suite, `artifact_guard` / fixtures byte-identical.
- done: all green, docs updated, manual checklist written to this folder.
