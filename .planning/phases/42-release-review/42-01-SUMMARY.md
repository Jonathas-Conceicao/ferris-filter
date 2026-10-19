---
phase: 42-release-review
plan: 01
subsystem: code-cleanup
tags: [rustfmt, cleanup, release-review]
requirements: [REV-01]
key-files:
  modified:
    - Cargo.toml
    - src/overlay/app.rs
    - src/overlay/instance.rs
    - src/overlay/mod.rs
    - src/sync/league.rs
    - tests/cli_generate.rs
    - tests/isolation_guard.rs
metrics:
  completed: 2026-10-18
---

# Phase 42 Plan 01: Code cleanup Summary

Formatting committed alone, the unused ToolHelp feature and stale comments removed, v0.7 ids swept from code; no behaviour change, suite identical before and after.

## Commits

- `db4d8b4` style(42-01): cargo fmt (src/sync/league.rs, tests/isolation_guard.rs; app.rs was already clean)
- `27769df` refactor(42-01): drop dead feature, stale comments and v0.7 ids

## Tests (FERRISFILTER_HOME / FERRISFILTER_GAME_DIR under temp, CARGO_TARGET_DIR outside repo)

- Before: 1811 passed, 0 failed, 0 ignored.
- After: 1811 passed, 0 failed, 0 ignored. Per-test result lists are identical (diff empty). No test renamed.

## Clippy (`cargo clippy --all-targets`)

- Before and after: 0 errors, 134 warning lines (lib 82, lib test 127 with 82 duplicates, cli_generate 2). Not increased. (The "~125 errors" in CONTEXT are warnings in this toolchain.)
- Lint classes (each distinct class seen once in the `#[warn]` notes; many more instances than classes): collapsible_if, unwrap_or_default, unnecessary_map_or, type_complexity, too_many_arguments, single_element_loop, result_large_err, redundant_guards, needless_lifetimes, manual_is_multiple_of, items_after_test_module, if_same_then_else, get_first, field_reassign_with_default, explicit_auto_deref, expect_fun_call, doc_overindented_list_items, doc_lazy_continuation, comparison_to_empty, collapsible_match, bool_comparison, assertions_on_constants. Left for a later cleanup.

## Cleaned

- 40 IN-03: removed `Win32_System_Diagnostics_ToolHelp` from windows-sys features (no users; build passes; Cargo.lock unchanged).
- 40 IN-05: `[build-dependencies]` moved above the "Optimise dependencies" comment, which now sits on `[profile.dev.package."*"]`. Version untouched (`0.7.0-rc.1`).
- 40 IN-01: comment in `overlay/mod.rs` now says only the mutex is never created.
- 40 IN-02: `OpenMutexW` now uses `SYNCHRONIZATION_SYNCHRONIZE` (same value 0x00100000, correctly typed, in the already-enabled Threading module). `Foundation::SYNCHRONIZE` does not exist in windows-sys 0.61 (the plain one is in Storage::FileSystem, which would need a new feature), so this constant was used instead.
- 40 IN-10: deferred-quit frame-loop comment reworded.
- 39 IN-01: the RELOAD doc paragraph moved from `sync_tiers_in_use` to `reload`; `sync_tiers_in_use` has its own doc.
- 36 IN-02: `sync --help` test uses `tempfile::tempdir()` for FERRISFILTER_HOME and FERRISFILTER_GAME_DIR.
- Id sweep: 3 hits in lines added since v0.6 (`WR-01 (iteration 2)` doc, `Phase 37-05` and `Phase 39-05` section headers) reworded; sweep count is now 0. No bug-narration wording found in added comments.

## Open info items (left untouched)

39 IN-02..IN-09, 41 IN-01..IN-09, 40 IN-04/06/07/08/09/11/12, 37 IN-*, 38 IN-* (behaviour changes, out of scope).

## Worker dedupe

worker dedupe: deferred. Fails criterion 3 and 4: `UpdateJob` is multi-message (progress `Downloading` then `Done`, a shared work-dir slot, `poll(utc_offset)` looping) and does not fit a simple one-shot helper without special cases; for `EconomySync` and `LeagueList` the shared part is only about 8 lines each (spawn + channel + try_recv), with different disconnect and state handling, so a helper module would not reduce net line count.

## Deviations from Plan

None beyond the `SYNCHRONIZATION_SYNCHRONIZE` choice above. Shell note: baseline and after runs used literal Windows temp paths for the env variables (the sandbox refused cygpath/mktemp command forms).

## Self-Check: PASSED
