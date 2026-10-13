---
phase: 23-auto-reload-filter-in-game-on-apply
plan: 01
subsystem: overlay
tags: [reload, settings, input-seam, worker]
requires: []
provides:
  - "overlay::chat (reload_steps, execute, decide, ReloadSend, InputSink)"
  - "apply.rs reload status builders and REASON_* constants"
  - "Settings.reload_after_apply (absent = on)"
affects: [23-02, 23-03, 23-04]
tech-stack:
  added: []
  patterns: [InputSink test seam, single-flight mpsc worker, per-key settings salvage]
key-files:
  created: [src/overlay/chat.rs]
  modified: [src/overlay/apply.rs, src/settings.rs, src/overlay/mod.rs, src/overlay/app.rs]
key-decisions:
  - "execute() restores the clipboard exactly once on any abort after SaveClipboard; a final restore failure never fails the send"
  - "settings salvage now keeps a valid filter name and a bool reload flag independently"
requirements-completed: [RELOAD-01]
duration: 20min
completed: 2026-10-13
---

# Phase 23 Plan 01: Pure reload core Summary

Pure, fully unit-tested core of auto-reload: the `/reloaditemfilter` step plan behind an `InputSink` seam, trigger decision, single-flight worker, status strings and the `reload_after_apply` setting (default on).

## Commits
- 73a27f1: status strings, setting, `settings_with_filter` carries the new field
- efaf3e5: `src/overlay/chat.rs` and `pub mod chat;`

## Verification
Full `cargo test` passes (767 lib tests plus integration, fixtures untouched); `cargo fmt --check` clean; chat.rs has no egui/windows_sys/Escape.

## Deviations from Plan
None - plan executed as written. (app.rs edit limited to the one `settings_with_filter` line.)

## Known Stubs
None.

## Self-Check: PASSED
