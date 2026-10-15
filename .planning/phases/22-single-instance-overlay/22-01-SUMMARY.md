---
phase: 22-single-instance-overlay
plan: 01
subsystem: overlay
tags: [windows-sys, single-instance, mutex, win32]
requires: []
provides:
  - "overlay::instance single-instance gate (acquire, Guard::take_show_request/ack, signal_running, notify_unresponsive)"
affects: [22-02]
tech-stack:
  added: []
  patterns: ["Named Local mutex + auto-reset show/ack events", "debug/release instance-name split"]
key-files:
  created: [src/overlay/instance.rs]
  modified: [Cargo.toml, src/overlay/mod.rs]
decisions:
  - "Fail open on unexpected CreateMutexW errors; ERROR_ACCESS_DENIED counts as AlreadyRunning"
  - "ack event is reset before the show event is set, so stale acks never count"
metrics:
  tasks: 2
  files: 3
completed: 2026-10-13
requirements-completed: [INST-01]
---

# Phase 22 Plan 01: Single-Instance Gate Summary

Self-contained `overlay::instance` module: a `Local\` named mutex plus auto-reset show/ack events, with separate debug and release names and a MessageBox when the running instance does not answer within 2 s. No new crate; two extra `windows-sys` features.

## Tasks

1. Features, module skeleton, pure naming and wait classification.
2. Windows implementation with in-process tests.

Both are in commit `6fe5c32`.

## Deviations from Plan

**1. [Process] Tasks 1 and 2 committed together.** The module was written complete in one pass, so the intermediate stub state was never built. All 13 tests (7 pure, 6 Windows) pass. No code deviation.

No Rule 1-4 fixes were needed. `launch()`, `app.rs` and the generator are untouched.

## Verification

- `cargo test --lib overlay::instance`: 13 passed.
- Full `cargo test`: 738 lib tests passed, plus integration suites (cli_errors 12, cli_filter_copy 4, cli_generate 16, dynamic_filter 15, filter_roundtrip 22, filter_structure 8, overlay_rules 4, sync_prices 8) and 1 doctest. 0 failures; fixtures unchanged.
- `cargo fmt --check` clean; clippy reports nothing for `instance.rs`.

## Known Stubs

None (the non-Windows `imp` is an intentional no-op).

## Threat Flags

None beyond the plan's threat model.

## Self-Check: PASSED

src/overlay/instance.rs exists; commit 6fe5c32 exists.
