---
phase: 22-single-instance-overlay
plan: 02
subsystem: overlay
tags: [single-instance, win32, egui]
requires: ["22-01"]
provides:
  - "launch() gated on instance::acquire(); running stripe reveals and acks on a second launch"
affects: []
tech-stack:
  added: []
  patterns: ["per-frame zero-timeout poll of the show event, reveal-only (never toggle)"]
key-files:
  created: [.planning/phases/22-single-instance-overlay/22-MANUAL-VERIFICATION.md]
  modified: [src/overlay/mod.rs, src/overlay/app.rs, CLAUDE.md, CHANGELOG.md, .planning/REQUIREMENTS.md]
decisions:
  - "Show request is its own branch in update(), outside the hotkey foreground gate; only reveals when Hidden; always acks"
metrics:
  tasks: 2
  files: 6
completed: 2026-10-13
---

# Phase 22 Plan 02: Wire Single-Instance Gate Summary

`launch()` now calls `instance::acquire()` as its first statement, so a second `ff` exits before registering a hotkey or tray icon. `OverlayApp` holds the `Guard` and polls `take_show_request()` each frame, revealing a hidden stripe through the existing `visibility::advance`/`win::reveal` path (no focus steal) and acking.

## Tasks

1. Gate `launch()` and poll the show request in `OverlayApp::update` - commit `3f428fe`.
2. CLAUDE.md note, CHANGELOG entry, INST-01 traceability, `22-MANUAL-VERIFICATION.md` - docs commit (see git log).

## Deviations from Plan

None - plan executed as written.

## Verification

- `cargo build --bins` OK; `cargo fmt --check` clean.
- Full `cargo test`: 738 lib tests passed, plus cli_errors 12, cli_filter_copy 4, cli_generate 16, dynamic_filter 15, filter_roundtrip 22, filter_structure 8, overlay_rules 4, sync_prices 8, 1 doctest. 0 failures; fixtures unchanged.
- GUI behaviour is UAT-pending via 22-MANUAL-VERIFICATION.md (INST-01 "Complete (UAT pending)"). Release build was not run by the executor.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

Modified files and 22-MANUAL-VERIFICATION.md exist; commit 3f428fe exists.
