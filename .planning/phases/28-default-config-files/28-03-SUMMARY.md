---
phase: 28-default-config-files
plan: 03
subsystem: overlay (CONFIG tab, combined Apply, styles poll)
tags: [defaults, styles, tiers, overlay, guard]
requires: [28-02]
provides:
  - ConfigState::live_baselines (None = live file absent) as the changed-on-disk baselines
  - ConfigState load/follow_disk via styles_source / tiers_source
  - apply_all guard on the live path against the live baseline
  - app.rs styles_stamp_at (source path, mtime) replacing styles_mtime_at
affects: [28-04]
tech-stack:
  added: []
  patterns: ["content baseline = bytes of the source read; guard baseline = bytes of the LIVE file or None"]
key-files:
  created: []
  modified: [src/overlay/config_state.rs, src/overlay/apply_all.rs, src/overlay/app.rs, src/overlay/apply.rs, src/overlay/body.rs]
decisions:
  - "Two baselines per config file: styles_bytes/tiers_bytes (content of the source read, what an unedited file keeps) and styles_live/tiers_live (live file bytes or None, the guard)"
  - "follow_disk moves the live baseline even when the content is identical (a live file created with the default bytes), but reports changed only on content change"
  - "Parse/validation errors name the file actually read (styles-default.json when falling back); a NoSource error still starts with the live file name"
  - "styles_unavailable_message and the picker's unchecked tooltip say 'The styles could not be read' (the error names the file)"
metrics:
  duration: ~10 min
  completed: 2026-10-15
requirements: [DEF-01, DEF-03]
---

# Phase 28 Plan 03: Overlay readers follow the effective source Summary

The overlay (CONFIG tab, combined Apply, RULES styles, class catalogue, styles poll) now reads
`styles-default.json` / `tiers-default.json` when the live files are absent, writes only the live
`styles.json` / `tiers.json`, and guards the live path with an "absent" baseline.

## What was done

- **config_state.rs:** new fields `styles_live` / `tiers_live: Option<Vec<u8>>`, helper
  `read_optional` (NotFound -> None) and `source` (NoSource mapped to `"<live name>: ..."`).
  `load` and `follow_disk` read through `styles_source()` / `tiers_source()`; `follow_disk` switches
  source on create/delete of the live file and moves the live baseline for a clean file; a dirty file
  keeps both baselines. `live_baselines()` added; `adopt` sets the live baseline only for a file that
  was written. `styles_path()` / `tiers_path()` stay the live write targets (docs updated).
- **apply_all.rs `prepare`:** compares `styles_now` / `tiers_now` (live path) with
  `c.live_baselines()`; non-overwrite guards are the live baselines (None = must still be absent, so
  `safe_write` creates the file). Absent/absent: no conflict; created or deleted since load:
  `ChangedOnDisk`.
- **app.rs:** `load_styles_at` and `load_classes_at` use the resolved styles source;
  `styles_mtime_at` -> `styles_stamp_at(root) -> Option<(PathBuf, SystemTime)>`, field
  `styles_mtime` -> `styles_stamp`; the poll compares stamps, so a source switch counts as a change.
- **Checker note:** `apply.rs` `styles_unavailable_message` now reads "The styles could not be read,
  so styled rules cannot be updated: {err}"; the app.rs test asserting the old text was updated.

## Tests added (14)

- config_state: `load_falls_back_to_defaults_and_creates_nothing`, `load_prefers_the_live_file`,
  `load_without_any_styles_file_names_both`, `follow_disk_switches_source_on_live_create_and_delete`,
  `follow_disk_keeps_live_baseline_of_an_edited_file`.
- apply_all (`Fixture::with_defaults`): `first_config_apply_from_defaults_creates_live_and_keeps_defaults`,
  `unedited_config_from_defaults_writes_nothing`, `guard_live_absent_and_still_absent_is_no_conflict`,
  `guard_live_created_since_load_is_a_conflict_and_overwrite_writes`,
  `guard_live_deleted_since_load_is_a_conflict`.
- app.rs (`defaults_only_root`, `rig_at`): `defaults_only_root_loads_styles_and_classes_and_creates_nothing`,
  `no_styles_file_at_all_names_both`, `styles_stamp_switches_on_live_create_and_delete`,
  `combined_apply_from_defaults_creates_live_styles_and_keeps_defaults`.

## Verification

- `cargo test`: lib 1041 passed (was 1027); integration 96 passed (12+4+20+16+23+8+4+8+1); 0 failed,
  run with the author's differing working-tree `styles.json` / `tiers.json` present.
- `cargo fmt --check` clean.
- `git diff --exit-code -- tests/fixtures tests/snapshots`: clean.
- `styles.json` / `tiers.json` still ` M`, unstaged; sha256 equal to the parked copies.

## Deviations from Plan

**1. [Rule 1 - Bug] Picker tooltip also named the wrong file**
- **Found during:** Task 2 (checker note)
- **Issue:** `body.rs` `STYLE_UNCHECKED_TIP` said "styles.json could not be read" just like the
  refusal message the checker flagged.
- **Fix:** reworded to "The styles could not be read; this style is not applied until they load."
- **Files modified:** src/overlay/body.rs (not in the plan's file list)
- **Commit:** a769214

## Commits

- `847ea53` feat(28-03): CONFIG tab and combined Apply read the defaults, guard the live path
- `a769214` feat(28-03): RULES styles, class catalogue and styles poll follow the effective source

## Self-Check: PASSED

- Both commits present on the branch; all five modified files exist.
