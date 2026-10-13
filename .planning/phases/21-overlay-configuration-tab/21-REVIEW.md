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
  warning: 5
  info: 3
  total: 8
status: issues_found
---

# Phase 21: Code Review Report

**Reviewed:** 2026-10-13
**Depth:** standard
**Files Reviewed:** 17
**Status:** issues_found

## Summary

The multi-file writer (`write_all_or_none`), the filter-name validation (`settings::valid_filter_name`, re-checked in `filter_target` before every join) and the style/tier models are sound. I found no path traversal, no injection (`sound.custom` is validated in `StyleSet::validate`), and no data-loss path in the writer itself. The defects are in the CONFIG tab's outside-edit handling and its out-of-date notice. I did not run cargo.

## Warnings

### WR-01: Apply is a silent no-op after "Keep editing" on the outside-edit guard

**File:** `src/overlay/app.rs:1010` (`config_apply`), `src/overlay/config_view.rs:1048-1049` (guard), `:1090-1093` ("Keep editing")
**Issue:** "Keep editing" sets `cfg.outside_dismissed = true`, which hides the guard and shows the normal Apply button again. If the user then presses Apply, `write_all_or_none` returns ChangedOnDisk, and `config_apply` handles `Err(OutsideEdit(which))` by only setting `self.config_outside = which`. It sets no message. Because `outside_dismissed` is still true, `show_footer` computes `guard == false`, so nothing is shown. The user presses Apply and nothing visibly happens, forever, until they switch tabs (`disarm_all` resets the flag) or reload.
**Fix:** On `OutsideEdit`, also reset the dismissal, or set a message:
```rust
Err(ConfigApplyError::OutsideEdit(which)) => {
    self.config_outside = which;
    self.config_ui.outside_dismissed = false; // re-raise the guard
}
```

### WR-02: `overwrite()` replaces the baselines before validating, so a failed Overwrite silently adopts the outside edit

**File:** `src/overlay/config_state.rs:420-432` (`overwrite`), `:281-287` (`can_apply` gate in `apply_inner`)
**Issue:** `overwrite` assigns `styles_bytes`/`tiers_bytes`/mtimes from disk first, then calls `apply_inner(true)`. If `apply_inner` returns early with `Invalid` or a `Write` error, the baselines already match the outside-edited files. The in-memory model is still the old one. The next normal Apply then passes the changed-on-disk guard and overwrites the outside edit with no prompt. The Overwrite button is also shown when `dirty_count() == 0`. In that case `can_apply()` is false and the user gets "Config not applied: Fix the tier errors first", which is a misleading message when there are no tier errors.
**Fix:** Take local copies of the disk bytes, run validation (and skip the `dirty_count() > 0` requirement when forcing) before committing them into `self`, and restore the old baselines on any error. Alternatively, hide or disable Overwrite when `dirty_count() == 0` and offer only "Reload from disk".

### WR-03: The "Filter out of date" notice is cleared by a RULES Apply that did not regenerate the tier-dependent output

**File:** `src/overlay/app.rs:785-796` (`run_apply`), `:308-323` (`apply_working`)
**Issue:** `config_changed_pending` is cleared on any successful RULES Apply, including `Unchanged`. The economy section is repriced only for `EconomyPlan::Refresh`/`Add`. The plan for a filter with no economy section is `Keep`/`Remove`, so a tiers-only CONFIG Apply followed by a RULES Apply on a filter with no economy section clears the notice. The notice is then correct only if the economy section is present and refreshed. Please confirm `economy_plan()` always returns `Refresh` when a section exists; if it can return `Keep`, the notice lies.
**Fix:** Clear the flag only when the apply actually rewrote styled rules or refreshed the economy. Otherwise keep the flag, or compare against the mtimes as `compute_config_pending` does.

### WR-04: `reload()` can silently reset the chosen filter to `dynamic.filter`

**File:** `src/overlay/app.rs:854-871`
**Issue:** RELOAD re-reads `ff-settings.json`. If the file is momentarily unreadable or invalid, `settings::load` returns the default (filter `None`). `self.settings` is replaced with that default, so the session's active filter reverts to `dynamic.filter`. The warning goes into `list_status`, which `reset_filter_state()` and later calls may overwrite. The user's current filter selection is lost with at most a transient message. A subsequent `switch_to` also saves the default's empty `extra`, dropping unknown keys that were already in memory.
**Fix:** On a warning, keep the in-memory `self.settings` and only surface the warning.

### WR-05: A single non-string `filter` key discards all of the user's settings

**File:** `src/settings.rs:26-33,50-53`
**Issue:** `filter: Option<String>` plus `#[serde(flatten)] extra` means `"filter": 5` fails deserialization for the whole file. The result is `Settings::default()` with a warning, and the next `save` writes a file without the unknown keys that load promised to preserve ("Unknown top-level keys survive a load/save round trip").
**Fix:** On a type error, deserialize into `serde_json::Map` first, pull `filter` out leniently, and keep `extra`.

## Info

### IN-01: A `-0` cutoff is accepted and serialized as `-0.0`

**File:** `src/overlay/config_state.rs:586-589`
**Issue:** `parse_cutoff("-0")` returns `-0.0` (`-0.0 >= 0.0` is true), so `tiers.json` can get `-0.0`. Likewise, `1e3` shows as dirty because the buffer is compared as text against `format!("{}", min)`.
**Fix:** Normalise with `v + 0.0`, or `if v == 0.0 { 0.0 }`. Compare dirtiness by value, not by buffer text.

### IN-02: The create-filter draft is dropped before the action succeeds

**File:** `src/overlay/config_view.rs:1019-1022`, `src/overlay/app.rs:960-975`
**Issue:** `filter_draft_row` clears `cfg.filter_draft` the moment it emits `CreateFilter`. If the handler rejects the name (it already exists in the game folder), the typed name is lost and the user must retype it.
**Fix:** Let the handler clear the draft on success only, as `AddStyle` does.

### IN-03: `applied_message` hardcodes "7 tiers"

**File:** `src/overlay/config_state.rs:556-558`
**Issue:** The message ignores `report.tiers` and always says "7 tiers" (the `ConfigApplyReport.tiers` field is otherwise unused).
**Fix:** `format!("Applied config: {styles} styles, {tiers} tiers.")`.

---

_Reviewed: 2026-10-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
