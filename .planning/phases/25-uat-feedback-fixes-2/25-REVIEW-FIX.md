---
phase: 25-uat-feedback-fixes-2
fixed_at: 2026-10-14T00:00:00Z
review_path: .planning/phases/25-uat-feedback-fixes-2/25-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 25: Code Review Fix Report

**Fixed at:** 2026-10-14
**Source review:** .planning/phases/25-uat-feedback-fixes-2/25-REVIEW.md
**Iteration:** 1 (single pass)

**Summary:**
- Findings in scope: 4 (Critical + Warning). The 5 Info findings are out of scope, except IN-03's
  caption text, which was fixed because it was a one-line change next to the reworked Apply.
- Fixed: 4 (plus IN-03)
- Skipped: 0

Verification:
- `cargo test` passes: 867 lib tests plus every integration target, including `cli_generate`
  (`artifact_guard`, `generate_snapshot`).
- `cargo fmt --check` is clean.
- Nothing under `tests/` changed, so the fixtures stay byte-identical.
- The non-test build has no warnings. The one test-build warning (`unused mut` at
  `src/overlay/rule_list.rs:942`) was there before these fixes and was left alone.
- Every test uses temp roots only. `tiers.json` (unrelated local modification) was never staged.

## Fixed Issues

### CR-01: A clean CONFIG still guards and writes styles.json/tiers.json

**Files modified:** `src/overlay/apply_all.rs`, `src/overlay/config_state.rs`, `src/overlay/app.rs`
**Commit:** 04c35d8
**Status:** fixed: requires human verification (state/ownership logic)
**Applied fix:**
- **`apply_all::prepare`:** a config file is guarded only when `ConfigWrite::styles_changed()` /
  `tiers_changed()` says it has a pending change.
- **`commit_with`:** a config file is pushed as a `Target` under the same condition. A clean file
  is never compared with its baseline, never refused, and never written, with or without
  OVERWRITE.
- **New `ConfigState::follow_disk()`:** for each file without pending edits whose bytes on disk
  differ from the baseline, it re-reads, parses and validates the file. It then adopts the result
  as both the working and the applied snapshot, plus the baseline. Tier buffers are resynced.
  - A file with edits keeps its baseline, so the changed-on-disk refusal and OVERWRITE still
    apply to it.
  - A clean file that no longer loads returns an error and leaves the state unchanged.
- **`apply_everything`:** calls `follow_disk()` before rendering, so a RULES-only Apply renders
  with what is on disk. If a clean file no longer loads, the Apply is refused with
  "Nothing was applied: tiers.json: ...". This matches the pre-phase behaviour, where economy
  pricing read the file from disk.
- **`poll_styles`:** also calls it every poll, so the CONFIG tab follows outside edits. Errors are
  ignored there; the next Apply reports them.
- **Tests:**
  - `apply_all::clean_config_is_neither_guarded_nor_written`: an outside `tiers.json` edit with a
    clean config. Apply and OVERWRITE both succeed and leave all three files byte-identical.
  - `config_state::follow_disk_adopts_only_clean_files`: a clean tiers file is adopted, an edited
    styles file keeps its baseline, edits survive, and an invalid clean file is an error with no
    change.
  - `app::clean_config_follows_outside_edits_and_never_writes_them_back`: a RULES-only Apply after
    outside edits to both files renders `SetFontSize 38` from the disk styles and leaves both
    files byte-identical. A following OVERWRITE does the same with a second outside edit.

### WR-01: CONFIG Apply writes a filter whose economy markers are damaged

**Files modified:** `src/overlay/apply_all.rs`, `src/overlay/apply.rs`, `src/overlay/config_view.rs`, `src/overlay/app.rs`
**Commit:** ca8ee5e
**Applied fix:**
- **`prepare`:** refuses with `ApplyAllError::Invalid` when `input.loaded.has_section()` errors,
  whatever the economy plan.
- **`config_apply_enabled`:** returns false for `FilterState::Damaged` when a filter part would be
  written.
- **Tooltip:** the CONFIG footer gets a `filter_damaged` flag (`ConfigFooterState` ->
  `ConfigViewCtx`), and a disabled Apply then shows the existing damaged-markers sentence. That
  sentence is now the shared const `apply::DAMAGED_MARKERS_MESSAGE`, which `filter_state_message`
  also uses.
- **Tests:**
  - `apply_all::damaged_markers_refuse_even_a_config_only_apply`: CONFIG edit, damaged filter,
    Keep plan, with and without overwrite. The result is `Invalid`, all three files are unchanged,
    and there is no `.ffbak`. The test was confirmed to fail with the check disabled.
  - An extra `config_apply_enabled` assertion for `Damaged`.

### WR-02: The one-time `.ffbak` is created before the all-or-none guard

**Files modified:** `src/safe_write.rs`, `src/overlay/apply_all.rs`
**Commit:** 2559431
**Applied fix:**
- **`safe_write::write_all_or_none_before(targets, before_persist)`:** new function. It runs the
  hook once, after guard (a), stage (c) and preflight (d), and before the first rename. A hook
  error refuses as `NothingWritten`.
- **Injectable variant:** `write_all_or_none_with_hook` adds the hook to the existing persist
  injection. `write_all_or_none` and `write_all_or_none_with` keep their signatures (they pass a
  no-op hook).
- **`commit_with`:** its writer now receives the pre-persist step, and `ensure_backup` runs inside
  it. A commit-time refusal (read-only or locked file, missing destination, a filter changed
  between prepare and commit) therefore leaves no `.ffbak` behind.
- **Test:** `apply_all::commit_time_refusal_leaves_no_ffbak` covers two cases:
  - (i) `tiers.json` made read-only after `prepare`: the preflight refuses with
    "Nothing was applied", the files are unchanged, and there is no `.ffbak`.
  - (ii) the filter is edited between `prepare` and `commit`: the result is
    `ChangedOnDisk([Filter])`, there is no `.ffbak`, and the config stays dirty.

  `third_rename_failure_rolls_back_all` now goes through the hooked writer.

### WR-03: CONFIG load-error "Reload from disk" discards RULES edits and does not reload the config

**Files modified:** `src/overlay/app.rs`
**Commit:** 39edf88
**Status:** fixed: requires human verification (UI action routing)
**Applied fix:**
- **New pure helper `config_reload_plan(config_failed, changed)`:** returns `ConfigOnly` when the
  config failed to load or nothing is pending, and `Changed` otherwise.
- **`ConfigAction::Reload` arm:**
  - `ConfigOnly` always re-runs `ConfigState::load(root)` and keeps a pending `Filter` refusal in
    `changed_on_disk`, so the filter and the RULES edits are untouched.
  - `Changed` keeps the shared footer RELOAD (`reload_changed`).
- **Deliberate change from the review's sketch:** the sketch also took the config-only path when
  the list held only `Filter` and the config was loaded. That would have broken the CONFIG
  footer's RELOAD for a filter-only refusal, which 25-CONTEXT requires on both tabs. The
  config-only path is therefore keyed on `config.is_err()`. That is the only state in which the
  load-error view, and so its button, is shown.
- **Test:** `app::config_load_error_reload_retries_the_config_and_spares_the_filter`.
  `OverlayApp` cannot be built headless (it needs a tray), so the routing decision is what the
  test covers.

### IN-03 (out of scope, trivially adjacent): Reload toggle caption still says "after a RULES Apply"

**Files modified:** `src/overlay/config_view.rs`
**Commit:** 33dd8a5
**Applied fix:** the caption and its doc comment now say "after Apply". The existing caption test
checks substrings that are unchanged.

---

_Fixed: 2026-10-14_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
