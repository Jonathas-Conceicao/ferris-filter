---
phase: 18-filter-as-store-economy-section
plan: 01
subsystem: filesystem
tags: [paths, app-root, safe-write, backup, atomic-write]
requires: []
provides:
  - "paths::{HOME_ENV, RootError, resolve_from, app_root}"
  - "safe_write::{write_guarded, WriteOutcome, WriteError, backup_path, MAX_ATTEMPTS}"
affects: [18-02, 18-03, 18-04]
tech-stack:
  added: []
  patterns: [pure core + thin env wrapper, injectable persist/sleep for retry tests]
key-files:
  created: [src/paths.rs, src/safe_write.rs]
  modified: [src/lib.rs]
key-decisions:
  - "Marker files are checked via InputPaths::relative_to, no re-spelled names"
  - "Backup written from the guarded in-memory loaded bytes with create_new, never fs::copy"
requirements-completed: [STORE-05, EDIT-09, EDIT-10]
duration: 10min
completed: 2026-10-12
---

# Phase 18 Plan 01: App root and safe write Summary

App-root resolver (FERRISFILTER_HOME, exe walk-up, cwd, visible error) and a guarded, one-time-backed-up, atomic filter write with Windows lock retry; both are unit-tested library modules, not yet wired in.

## Tasks
1. App-root resolver, `src/paths.rs` (10 tests), commit af35a57.
2. Guarded write, `src/safe_write.rs` (10 tests), commit 3b2cb1f.

`cargo test --lib -- paths:: safe_write::` passes 20 tests. The build shows no warnings from either module.

## Deviations from Plan
None. The plan executed as written.

## Known Stubs
None.

## Self-Check: PASSED
