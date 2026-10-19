---
phase: 36-per-user-locations-embedded-defaults
plan: 01
subsystem: infra
tags: [rust, config-folder, include_str, dirs]
requires: []
provides:
  - "src/locations.rs: AppDirs, resolve_from, from_env, HOME_ENV, RootError"
  - "src/defaults.rs: Builtin + six embedded shipped files"
affects: [36-02, 36-03, 36-04, 36-05, 36-06, 36-07]
tech-stack:
  added: ["dirs 6 (direct; already in Cargo.lock via tray-icon)"]
  patterns: ["pure resolver core + thin env wrapper", "lazy platform closure"]
key-files:
  created: [src/locations.rs, src/defaults.rs]
  modified: [Cargo.toml, Cargo.lock, src/lib.rs, src/paths.rs]
key-decisions:
  - "from_env panics under cfg(test) when FERRISFILTER_HOME is unset or empty"
  - "paths.rs only re-exports HOME_ENV; its old body stays until 36-07"
requirements-completed: [LOC-01, LOC-02, LOC-03, LOC-05]
duration: ~15min
completed: 2026-10-18
---

# Phase 36 Plan 01: Locations and Embedded Defaults Summary

Per-user folder resolver (`AppDirs`, FERRISFILTER_HOME else `dirs::config_dir()` + app name, no walk-up or cwd fallback) and `defaults.rs` embedding the six shipped files via `include_str!`.

## Tasks
1. `cd36d37` locations.rs resolver, `dirs = "6"`, `pub mod locations`, `HOME_ENV` re-export in paths.rs. 8 tests.
2. `5344eeb` defaults.rs with `Builtin`, `label()`, `ALL`, `by_file_name`. 5 tests (equal to committed files, valid JSON, lookup, label, unique names).

## Verification
- `cargo test --lib` filtered runs: locations 8, defaults 5, paths 11 pass; `cargo test --test cli_generate` 20 pass.
- `cargo tree -i dirs`: single `dirs v6.0.0`. `dirs::` appears only in locations.rs.

## Deviations from Plan
None in code. Note: `cargo clippy --all-targets -- -D warnings` reports many errors in pre-existing code (lib test target); none in the new modules. Out of scope, not fixed.

## Decisions
- Lib test default `cargo test` filter takes one pattern, so test groups were run separately.
- Nothing calls `AppDirs::from_env` yet (callers switch in 36-04/36-06).

## Known Stubs
None.

## Self-Check: PASSED
