---
phase: 49-drag-and-drop-reorder
plan: 03
subsystem: filter-model
tags: [drag-and-drop, document, row-action]
requires: []
provides:
  - "filter::MoveDest and Document::move_run_to (banner-aware multi-gap block move)"
  - "RowAction::MoveTo and WorkingFilter::move_rule_to"
affects: [49-04]
tech-stack:
  added: []
key-files:
  modified: [src/filter/structure.rs, src/filter/mod.rs, src/overlay/rules.rs]
key-decisions:
  - "move_rule_to sets moved-across-Continue iff the run or a rule passed over has Continue, else clears it (mirrors move_rule)"
requirements-completed: [DND-03, DND-04]
completed: 2026-10-23
---

# Phase 49 Plan 03: MoveDest, move_run_to and RowAction::MoveTo Summary

`Document::move_run_to` moves a block or group run, with its leading comments and trailing blanks, to a `MoveDest` (Before block, AfterBanner, BeforeBanner, End) in one edit, and `RowAction::MoveTo` applies it with the arrows' rules. The UI does not emit it yet (plan 04).

## What was built

- `MoveDest` (re-exported as `crate::filter::MoveDest`) and `move_run_to` in `src/filter/structure.rs`. It resolves an insertion line in the old line vector and splices the run region (`infos[first].span.start .. infos[last].unit.end`, as `swap_runs` does). A destination inside or after the economy banner, `AfterBanner` on the economy banner, and a run inside the economy section give `SectionLocked`. Unknown ids give `UnknownBlock`. A non-consecutive or empty run, or one spanning a banner, gives `NoNeighbour`. A no-op destination returns `Ok(false)`. The result goes through `commit_structural`, so a bad splice fails closed.
- Blank-line hygiene: the file's last unit normally has no trailing blank, so surplus end-of-file blanks are moved next to the moved run (after it when it left the end, before it when it arrived there). This keeps the line multiset exact. The unterminated-final-line and CRLF handling mirror `swap_runs`.
- `RowAction::MoveTo { id, dest }` in `src/overlay/rules.rs`: added to `action_target` (so followers and economy ids are refused by `dispatch`) and to the dispatch and `block_action` arms. It is non-coalescing, since `coalesce_edits` only merges `EditLine`. `move_rule_to` moves the whole group via `groups.members`. `Ok(false)` gives `ActionOutcome::default()` (`changed: false`, no status). Success gives `focus == Some(id)`. `Err` is a refusal through `section_refusal`. `move_rule`, `Move`, `can_move_*` and `move_section` are unchanged.

## Judgement call

Continue flag: set iff the run or any rule passed over in file order (collapsed-section rules included, banners excluded) has `Continue`, otherwise `moved.remove(&id)`, so a non-crossing drop clears a flag set by an earlier move. This is what `move_rule` does on a non-crossing step.

## Tests

13 `move_run_to_*` tests in structure.rs: down/up, groups of 2 and 3, comments travel, AfterBanner, BeforeBanner (including the first banner), End with and without economy, missing final newline, CRLF, no-ops, economy refusals, unknown ids and broken runs, and a multiset/id round trip over every block and destination of a bannered file. 5 `move_to_*` tests in rules.rs: group as a unit, follower and economy refusal, Continue set and cleared, collapsed-section target, and no-op.

`cargo test --locked` is green (1823 lib tests plus the integration suites, including the `overlay_rules` fuzz test). `tests/fixtures` unchanged. No new dependencies.

## Deviations from Plan

- TDD gate for Task 2: the rules.rs implementation was written before its tests, so there is no separate RED commit for it (the tests passed on first run). Task 1 has a proper RED commit (df86ed2) and GREEN commit (37a3af2).
- `cargo fmt` was run per file (`rustfmt` on the changed files only), so no unrelated files were touched.

## Commits

- df86ed2: test(49-03): add failing move_run_to tests
- 37a3af2: feat(49-03): implement MoveDest and Document::move_run_to
- feat(49-03): add RowAction::MoveTo and WorkingFilter::move_rule_to (see git log)

## Self-Check: PASSED
