---
phase: 37-game-folder-resolver-picker
plan: 07
subsystem: test-isolation
tags: [rust, tests, screenshot, isolation]
requires: [37-01, 37-03]
provides:
  - screenshot guard on FERRISFILTER_HOME + FERRISFILTER_GAME_DIR
  - isolation_guard requiring FERRISFILTER_GAME_DIR on every resolving spawn
key-files:
  modified: [src/overlay/screenshot.rs, tests/isolation_guard.rs, tests/cli_errors.rs, tests/cli_generate.rs, tests/dynamic_filter.rs, tests/ui/README.md]
decisions:
  - "USERPROFILE pins in cli_errors / cli_generate were replaced (not kept beside) by FERRISFILTER_GAME_DIR, since USERPROFILE no longer affects anything"
  - "game_dir() in screenshot mode returns the env value verbatim and has no fallback; empty counts as unset"
metrics:
  tasks: 2
  completed: 2026-10-18
---

# Phase 37 Plan 07: Test isolation on FERRISFILTER_GAME_DIR Summary

Screenshot mode now refuses to run unless FERRISFILTER_HOME and FERRISFILTER_GAME_DIR are both set, non-empty and under a real-looking temp folder; USERPROFILE is no longer consulted. The source-scan isolation guard requires FERRISFILTER_GAME_DIR (and HOME) on every spawn of apply, sync, ff, or install without --dest.

## Commits

- f715777: feat(37-07): screenshot guard requires FERRISFILTER_GAME_DIR under temp
- 221a972: test(37-07): isolation guard pins FERRISFILTER_GAME_DIR, install without --dest counted

## Verification

- `cargo test --lib overlay::screenshot`: 8 passed (tests rewritten: game paths are arbitrary temp subpaths; guard names FERRISFILTER_GAME_DIR; `..` escape refused; temp-without-Temp component refused).
- `cargo test --test isolation_guard --test cli_errors --test cli_generate --test dynamic_filter`: 5 / 12 / 20 / 24 passed.
- All integration test files (`cargo test --no-fail-fast --test '*'`): no failures. The full `--lib` suite was not run (too slow here; orchestrator runs it), so the "3 known baseline overlay::app failures" comparison is left to the orchestrator.
- `grep -rn "USERPROFILE\|Documentos" src/` is empty; in tests only doc text in isolation_guard.rs and the stale `.env("USERPROFILE", ...)` pins in dynamic_filter.rs remain (harmless; the GAME_DIR pin sits next to each).
- **Negative check:** removed the `.env("FERRISFILTER_GAME_DIR", ...)` line from the `apply` helper in tests/dynamic_filter.rs: `spawns_that_resolve_the_folder_set_home` failed with `["tests/dynamic_filter.rs:73"]`; the line was restored.
- **Real game folder:** `ls -la` of `C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2` before and after the integration run: identical. All runs used temp FERRISFILTER_HOME / FERRISFILTER_GAME_DIR.

## Deviations from Plan

- [Scope] The "full `cargo test --no-fail-fast`" before/after check covered integration tests plus the screenshot lib tests, not the whole lib suite (per the orchestrator's instruction).
- Fixed the stale doc line in tests/dynamic_filter.rs header (USERPROFILE -> FERRISFILTER_GAME_DIR); not in the plan's file list but doc-only.

## Known Stubs

None.

## Threat Flags

None. T-37-07 and T-37-08 are implemented as planned.

## Human verification

- Run `ff --screenshot` (debug) with FERRISFILTER_GAME_DIR unset, then pointed at a non-temp folder: both must refuse with a message naming the variable. (Covered by unit tests of the pure helper; a real invocation was not run to avoid any chance of pointing at real folders.)
- Follow tests/ui/README.md once with the new `$env:FERRISFILTER_GAME_DIR` recipe.

## For the Release Review

- CLAUDE.md `ff` row (debug-only screenshot mode): "refuses to run unless `FERRISFILTER_HOME`, `USERPROFILE` and the game folder derived from it all live under the temp folder" should now say `FERRISFILTER_HOME` and `FERRISFILTER_GAME_DIR` (both required, under temp); `--filter` is staged into `FERRISFILTER_GAME_DIR`.
- CHANGELOG: nothing user-visible (dev/test tooling only).

## Self-Check: PASSED

- f715777 and 221a972 exist; modified files present.
