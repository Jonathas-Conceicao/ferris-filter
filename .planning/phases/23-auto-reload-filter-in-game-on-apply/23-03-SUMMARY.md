---
phase: 23-auto-reload-filter-in-game-on-apply
plan: 03
subsystem: overlay
tags: [reload, config-toggle, winsink, worker]
requires: ["23-01", "23-02"]
provides:
  - "CONFIG FILTER toggle 'Reload filter in game after Apply' (saved on change)"
  - "chat::WinSink production InputSink"
  - "run_apply reload hook with interim and result status lines"
affects: [23-04]
tech-stack:
  added: []
  patterns: [save-on-change setting, one-shot worker polled per frame]
key-files:
  created: []
  modified: [src/overlay/config_view.rs, src/overlay/app.rs, src/overlay/chat.rs]
key-decisions:
  - "The toggle persists immediately via settings::save and never touches CONFIG pending/dirty state"
  - "Interim 'Reloading in game...' line is replaced by the result only while it is still the displayed message"
  - "Clipboard restored only if clipboard_sequence still equals our write; a no-text clipboard is cleared"
requirements-completed: []
duration: 25min
completed: 2026-10-13
---

# Phase 23 Plan 03: Overlay wiring for auto-reload Summary

The CONFIG FILTER section now has a persisted reload toggle with the selected-filter caveat, and a Written RULES Apply decides, runs the `/reloaditemfilter` sequence on a worker thread through `WinSink`, and replaces the interim status with the result or the manual Refresh reminder plus reason.

## Commits
- 859b32d: CONFIG toggle, `ConfigAction::SetReloadAfterApply`, `settings_with_reload`, `set_reload_after_apply`
- 79b7be1: `WinSink`, `run_apply(ctx)` hook, `PendingReload`, `landed_reload_message`, `skipped_reload_message`, per-frame `reload.poll()`

## Verification
Full `cargo test` passes (777 lib tests plus integration; fixtures untouched); `cargo fmt --check` clean; clippy reports nothing in the touched files; `cargo build --release --bin ff` succeeds. No test constructs `WinSink` or calls a Win32 input/clipboard wrapper; chat.rs contains no `unsafe`.

## Deviations from Plan
None - plan executed as written.

## Known Stubs
None.

## Notes
In-game behaviour (focus, paste timing, clipboard restore) is verified manually in plan 23-04.

## Self-Check: PASSED
