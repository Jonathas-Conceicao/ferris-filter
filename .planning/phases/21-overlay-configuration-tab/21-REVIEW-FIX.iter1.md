---
phase: 21
fixed_at: 2026-10-13T00:00:00Z
review_path: .planning/phases/21-overlay-configuration-tab/21-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 4
skipped: 1
status: partial
---

# Phase 21: Code Review Fix Report

**Source review:** 21-REVIEW.md  **Iteration:** 1
Also fixed Info findings IN-02 and IN-03 (cheap). cargo test (lib 725 tests) and cargo fmt --check pass. Work was done on the current branch directly (no worktree), per the task's constraints.

## Fixed Issues

### WR-01: Apply silent no-op after "Keep editing"
**Files:** `src/overlay/app.rs` — **Commit:** e7d229f
On `OutsideEdit`, `outside_dismissed` is reset so the guard re-appears. (Logic fix; requires human verification. No regression test: egui app state.)

### WR-02: Failed Overwrite adopts the outside edit
**Files:** `src/overlay/config_state.rs` — **Commit:** be14164
Old baselines/mtimes are restored if the forced apply fails; a forced apply no longer requires `dirty_count() > 0` (still requires valid data). Tests added: `failed_overwrite_keeps_the_outside_edit_detectable`, `overwrite_with_no_edits_rewrites_from_memory`. Requires human verification.

### WR-04: reload() resets chosen filter on unreadable settings
**Files:** `src/overlay/app.rs` — **Commit:** 5905196
In-memory settings are kept when the load warns and the root did not change. Requires human verification (no unit test).

### WR-05: Non-string `filter` discards all settings
**Files:** `src/settings.rs` — **Commit:** 772fad3
Falls back to parsing a generic map, dropping only `filter` and keeping `extra`. Test: `non_string_filter_keeps_other_keys`.

### IN-03: "7 tiers" hardcoded
**Files:** `src/overlay/config_state.rs`, `src/overlay/app.rs` — **Commit:** 4c6d014

### IN-02: Create-filter draft dropped before success
**Files:** `src/overlay/config_view.rs` — **Commit:** 3259de7
(The handler already clears the draft on success.)

Formatting follow-up: a2fc3bd (cargo fmt).

## Skipped Issues

### WR-03: "Filter out of date" notice cleared by RULES Apply
**File:** `src/overlay/app.rs:785-796`
**Reason:** Not a defect on inspection. `economy_plan()` returns `Refresh` whenever an economy section is `Present`, and styled rules are rewritten on every Apply (`text_with_styles`), so a Keep plan means there is no tier-dependent output to regenerate. Only a `Damaged` economy section yields `Keep` with a section present; that case is an edge the reviewer's confirmation request does not justify extra plumbing for.
**Original issue:** Notice could be cleared without tier-dependent output being regenerated.

IN-01 is out of scope (info, not requested).

---
_Fixer: Claude (gsd-code-fixer)_
