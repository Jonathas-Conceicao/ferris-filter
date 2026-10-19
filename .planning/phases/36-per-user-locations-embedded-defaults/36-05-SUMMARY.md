---
phase: 36-per-user-locations-embedded-defaults
plan: 05
subsystem: overlay
tags: [config-state, apply, embedded-defaults, first-run]
requires: [36-03]
provides:
  - "ConfigState loads built-in styles/tiers from an empty folder via InputPaths::in_home"
  - "Apply prices the economy from the folder's prices.json, else the built-in fallback pair"
  - "Apply creates the FerrisFilter folder only when it writes styles.json / tiers.json"
affects: [overlay]
key-files:
  modified:
    - src/overlay/config_state.rs
    - src/overlay/apply_all.rs
    - src/overlay/app.rs
decisions:
  - "Live-file errors name the full path; embedded-source errors use Source::label() (built in)"
  - "A resolution error is prefixed with the live file's full path"
  - "Folder creation sits in commit_with, not prepare, so a refused Apply creates nothing"
metrics:
  tasks: 2
  completed: 2026-10-18
requirements: [LOC-01, LOC-02, LOC-03]
---

# Phase 36 Plan 05: Overlay Config State and Apply on Embedded Defaults Summary

ConfigState and the shared Apply now run on `InputPaths::in_home` and `Source`, so an empty or missing folder works and the folder is created only by a real config write.

## Tasks

1. **ConfigState on Source** (c4f4997): `read_effective` returns `(Source, bytes, live bytes)`; an embedded source has no live baseline, so it is never dirty and never written. `file_label` and the `source` helper are gone; errors use the full live path or `Source::label()`. `follow_disk` still sees live-file creation and deletion. Tests: empty folder loads built-ins with 0 entries left behind, full-path load errors, `-default.json` files in the folder are ignored, race test adapted to `Source`.
2. **Apply** (e488713): economy pricing uses `in_home`; `commit_with` calls `create_dir_all` on the parent of each config file that is actually written (filter-only Apply creates nothing; the game folder is untouched). Tests `first_run_apply_writes_only_the_filter` (root stays empty, `used_fallback` true) and `apply_creates_the_folder_for_config_edits` (only `styles.json` appears in the new folder).

## Deviations from Plan

- **[Rule 3 - Blocking] `staged_economy_preview_follows_pending_tier_edits`** (src/overlay/app.rs, not in the plan's file list): the test used the repo root as app root and relied on a live `prices.json` there. With embedded defaults and no live prices it prices from the fallback pair, which ignores pending tiers. It now uses a temp root holding the frozen seed prices.
- rustfmt reflowed a few lines of 36-05 code in the files above.

## Baseline failures

With `FERRISFILTER_HOME` pointed at an empty temp dir, the three known failures were
`economy_in_the_file_follows_pending_style_visibility`,
`staged_economy_preview_follows_pending_tier_edits` and
`staged_economy_preview_is_re_priced_after_a_sync`.
Moving CONFIG onto the embedded defaults fixed two of them directly. The third needed the test-rig fix above.

## Verification

Full `cargo test --no-fail-fast` (`FERRISFILTER_HOME` set to an empty temp dir): lib 1436 passed, 0 failed. Integration tests: cli_errors 12, cli_filter_copy 4, cli_generate 20, dynamic_filter 23, filter_roundtrip 25, filter_structure 18, overlay_rules 4, sync_prices 8, doctests 1. All passed.

`cargo clippy --all-targets` warnings in config_state.rs (complex tuple type on `read_effective`, large `Err` closures) are of the existing kinds; the baseline of about 118 errors under `-D warnings` was not touched.

## Known Stubs

None.

## Self-Check: PASSED

Commits c4f4997 and e488713 exist; the three modified files exist.
