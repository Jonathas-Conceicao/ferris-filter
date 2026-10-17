---
phase: 32
fixed_at: 2026-10-16T00:00:00Z
review_path: .planning/phases/32-rule-dividers-economy-banner/32-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 32: Code Review Fix Report

**Fixed at:** 2026-10-16
**Source review:** .planning/phases/32-rule-dividers-economy-banner/32-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6
- Fixed: 6
- Skipped: 0

Final checks: `cargo test` green, `cargo fmt --all --check` clean, clippy warning count 126 (limit 126), `tests/fixtures` unchanged.

## Fixed Issues

### CR-01: Removing the divider directly below the economy section turned its rules into economy rules

**Files modified:** `src/filter/divider.rs`, `src/overlay/rules.rs`, `src/overlay/rule_list.rs`, `src/overlay/app.rs`, `tests/filter_structure.rs`
**Commit:** b24867c
**Applied fix:** `remove_section` refuses (`SectionLocked`) when the previous banner is the economy banner. New `Document::section_can_remove` feeds `SectionRow.can_remove`, and the header x button is no longer drawn when it is false. Regression tests in `divider.rs` and `tests/filter_structure.rs`. CLAUDE.md Sections note updated.

### CR-02: Moving the divider below the economy section down put user rules inside the economy section

**Files modified:** same commit as CR-01 (the two fixes share the `follows_economy` helper)
**Commit:** b24867c
**Applied fix:** `hop` returns `None` for `!up` when the divider follows the economy banner, so `section_can_move(k, false)` is false and `move_section` returns `NoNeighbour`. Covered by the same tests.

### WR-01: Batched section actions addressed banners by a stale ordinal

**Files modified:** `src/overlay/rules.rs`
**Commit:** 9a4b2ae
**Applied fix:** `coalesce_edits` now stable-partitions `RenameSection` actions to the front of the batch, so a pending rename runs before `AddSection` or `RemoveSection` shifts the ordinals. `carry_section_state` already read the pre-batch ordinals, which now match. Test: `rename_runs_before_a_structural_action_in_the_same_batch`.

### WR-02: A rule in a collapsed section could not be reached by the status-line jump

**Files modified:** `src/overlay/app.rs`
**Commit:** 8dca894
**Applied fix:** New `section_to_reveal` and `App::reveal_rule` open the collapsed user section holding the target rule. They are called from the status-line jump and after row actions (covers a rule moved into a collapsed neighbour, via `pending_scroll`). Test: `a_rule_in_a_collapsed_section_is_revealed`.

### WR-03: Toggling a section overwrote an unusable `ff-settings.json`

**Files modified:** `src/overlay/app.rs`, `CLAUDE.md`
**Commit:** 52dc1cb
**Applied fix:** New `App.settings_unusable` flag (set from the load warning at start and on Reload). `store_settings` goes through `save_view_settings`, which skips the write while the flag is set, so the fold stays in memory only. An explicit Config save (filter switch, reload toggle) that succeeds clears the flag. Test: `a_fold_click_does_not_overwrite_an_unusable_settings_file`.

### WR-04: Collapsed state keyed by section name leaked between sections

**Files modified:** `src/settings.rs`, `src/overlay/app.rs`, `src/overlay/rules.rs`, `CLAUDE.md`
**Commit:** e1e01a9
**Applied fix:**
- `settings::section_key` stores a user name that starts with `ff:` or `user:` under a `user:` prefix, so it can never equal the reserved `ff:economy` key.
- `Settings::forget_section_state` is new.
- `section_state_after` (replaces the copy-only carry) moves the flag on rename and drops it on remove. It resets "Section" on add, so new sections start expanded.
- It touches a flag only when no other section shares the name (duplicate names still share one flag, by design of the name-keyed spec).
- Tests: `section_key_never_equals_the_economy_key`, `forget_section_state_drops_the_flag`, `section_state_follows_rename_remove_and_add`.

Notes: the old rename comment said the old entry stayed so a Revert could find it. That is now dropped, as the review asked, so reverting a rename restores the section as expanded unless it shared its name. CR-01 and CR-02 are one commit because they share a helper and one test layout. Work was done directly on the milestone branch rather than in a separate worktree, as the caller specified the working directory.

---

_Fixed: 2026-10-16_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
