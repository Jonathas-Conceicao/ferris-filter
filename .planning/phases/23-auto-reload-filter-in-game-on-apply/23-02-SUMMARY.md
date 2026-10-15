---
phase: 23-auto-reload-filter-in-game-on-apply
plan: 02
subsystem: overlay
tags: [win32, sendinput, clipboard, game-window]
requires: []
provides:
  - "overlay::win: is_game_title, is_game_exe, is_game_window, find_game_window, focus_game, game_is_foreground"
  - "overlay::win: modifiers_held, send_tap, send_chord"
  - "overlay::win: read/write/clear_clipboard_text, clipboard_sequence, ClipboardOwner, GameWindow"
affects: [23-03, 23-04]
tech-stack:
  added: []
  patterns: [safe wrapper per unsafe call with SAFETY comment, cfg(not(windows)) stand-ins, Drop guard for CloseClipboard]
key-files:
  created: []
  modified: [Cargo.toml, src/overlay/win.rs]
key-decisions:
  - "Game window = visible top-level window with title containing 'path of exile' AND process image name starting 'pathofexile'; unqueryable process is not the game; own pid skipped"
  - "Hotkey gate stays title-only via the shared is_game_title predicate"
  - "focus_game fails closed: plain SetForegroundWindow, no AttachThreadInput tricks"
  - "CF_UNICODETEXT defined locally (13) to avoid the Win32_System_Ole feature"
requirements-completed: []
duration: 15min
completed: 2026-10-13
---

# Phase 23 Plan 02: Raw Win32 reload wrappers Summary

Safe, SAFETY-commented wrappers in `src/overlay/win.rs` for finding the Path of Exile 2 window by title plus exe name, focusing it, detecting held modifiers, injecting a key tap or chord in one `SendInput` batch (vk plus scan code), and save/set/clear of Unicode clipboard text; all have non-Windows stand-ins.

## Commits
- 84c69ad: windows-sys features, pure predicates, find/focus/foreground
- 9161828: modifiers, SendInput, clipboard wrappers

## Verification
Full `cargo test` passes (772 lib tests plus integration; fixtures untouched); `cargo fmt --check` clean; clippy reports nothing in win.rs. Tests call only the pure predicates and read-only `find_game_window`/`clipboard_sequence`; no input is sent and the clipboard is never touched.

## Deviations from Plan

### Auto-fixed Issues
**1. [Rule 3 - Blocking] `BOOL` import path**
- **Issue:** `windows_sys::Win32::Foundation::BOOL` does not exist in 0.61.2.
- **Fix:** imported `windows_sys::core::BOOL`.
- **Commit:** 84c69ad

Note: `Cargo.lock` is git-ignored, so it is not part of the commits; no versions changed.

## Known Stubs
None.

## Self-Check: PASSED
