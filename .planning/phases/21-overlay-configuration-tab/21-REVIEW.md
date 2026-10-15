---
phase: 21-overlay-configuration-tab
reviewed: 2026-10-13T00:00:00Z
depth: standard
files_reviewed: 17
files_reviewed_list:
  - src/safe_write.rs
  - src/settings.rs
  - src/styled.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/body.rs
  - src/overlay/config_state.rs
  - src/overlay/config_view.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/widgets.rs
  - src/models/config.rs
  - src/models/tiers.rs
  - src/loader.rs
  - src/lib.rs
  - .gitignore
findings:
  critical: 0
  warning: 0
  info: 2
  total: 2
status: clean
---

# Phase 21: Code Review Report (iteration 2)

**Reviewed:** 2026-10-13
**Depth:** standard
**Files Reviewed:** 17 (focus: fix commits e7d229f..3259de7)
**Status:** clean (no Critical/Warning)

## Summary

The fixes for WR-01, WR-02, WR-04, WR-05, IN-02 and IN-03 were traced and are correct, and I found no regressions.

- **WR-01** (`app.rs`): resetting `outside_dismissed` on `OutsideEdit` re-raises the guard, so Apply is no longer a silent no-op.
- **WR-02** (`config_state.rs`): `overwrite()` restores the old baselines and mtimes when the forced apply fails. The forced path still requires valid styles and tiers. The `can_apply` bypass applies only to `force`, which only `overwrite()` calls. `write_all_or_none` is all-or-none, so restoring the baselines after a failure cannot desync them from a partial write.
- **WR-04** (`app.rs`): `reload()` keeps the in-memory settings when the load warns and the root is unchanged. When the root changed, the loaded settings are used.
- **WR-05** (`settings.rs`): the generic-map fallback drops only `filter` and keeps `extra`. The new test covers it.
- **IN-02** and **IN-03**: correct. `applied_message` now takes the real tier count.
- **WR-03** (skipped by the fixer): I confirm the "not a defect" judgement.
  - `economy_plan()` returns `Refresh` whenever the economy section is `Present`, so tier-dependent output is regenerated on every RULES Apply.
  - Styled rules are rewritten on every Apply.
  - A `Keep` plan with a section present occurs only for a damaged or absent section, so clearing the notice there is acceptable.

## Info

### IN-01: Settings warning for an invalid filter name also suppresses the in-memory update on reload

**File:** `src/overlay/app.rs:863`
**Issue:** `settings::load` also warns when the saved filter name is invalid. In that case the earlier valid in-memory choice is kept. The result is benign, but the on-disk and in-memory settings can diverge until the next save.
**Fix:** None required. Optionally distinguish a parse or IO warning from a validation warning.

### IN-02: Overwrite partial-failure comment

**File:** `src/overlay/config_state.rs:483`
**Issue:** The baseline-restore correctness depends on `write_all_or_none` being truly atomic. A comment would record that assumption.
**Fix:** Add a one-line comment.

---

_Reviewed: 2026-10-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
