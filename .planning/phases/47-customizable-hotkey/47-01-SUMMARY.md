---
phase: 47-customizable-hotkey
plan: 01
subsystem: overlay hotkey core
tags: [hotkey, chord, settings, global-hotkey, egui]
requires: []
provides:
  - src/overlay/chord.rs (Chord, ChordKey, DEFAULT, parse/text/is_bindable/modifiers/code/id, key_from_event, held_caption, RejectReason, CaptureState/CaptureInput/CaptureStep, capture_step)
  - src/overlay/hotkey.rs (RegisterError, HotkeyRegistrar, HotkeyRuntime, test-only FakeRegistrar)
  - src/settings.rs (Settings.hotkey, Settings.hotkey_invalid, saved_hotkey, render_hotkey)
affects: [47-03 app wiring, 47-04 Config view]
tech-stack:
  added: []
  patterns: [registrar seam with fake for tests, pure capture state machine over egui events]
key-files:
  created: [src/overlay/chord.rs]
  modified: [src/overlay/mod.rs, src/overlay/hotkey.rs, src/settings.rs, .planning/phases/AUTONOMOUS-V072-DEFERRED.md]
decisions:
  - Chord text is FerrisFilter's own (Ctrl, Shift, Alt order); parse is case-insensitive, splits on '+', last token is the key; global-hotkey FromStr/Display not used
  - Escape is a Cancel in capture and Forbidden in parse; Win/Windows/Super/Meta/Cmd tokens Forbidden
  - CaptureState carries a private last_held triple so a modifier change clears the rejection; build it with CaptureState::default()
  - startup(None, saved) shows the saved chord (else default) with reason "the hotkey service is unavailable"; an in-use failure reason reads "in use by another app"
  - A non-in-use start-up failure of the saved chord warns "`hotkey` <c> could not be registered (<e>), so Ctrl+Shift+Z is used"
  - Invalid saved hotkey warning is wrapped as "ff-settings.json could not be fully used; <UI-SPEC sentence>." and appended after other key warnings
  - render_hotkey keeps the contracted Result<Vec<u8>, ()> with a local clippy::result_unit_err allow; save_keys shares a render_object helper (byte-identical)
metrics:
  duration: ~25 min
  completed: 2026-10-23
  tasks: 3
  files: 5
---

# Phase 47 Plan 01: Hotkey core (chord, runtime, settings key) Summary

Pure, unit-tested core for the configurable show / hide hotkey: a `Chord` type with its own
`Ctrl+Shift+Alt+<key>` text form and egui-key table, a capture state machine that handles
modifiers, Escape, rejections and egui's Copy/Cut/Paste events, a `HotkeyRegistrar` seam with a
`HotkeyRuntime` (start-up fallback, register-new-before-unregister-old swap, abort, release,
tooltip texts), and a validated `hotkey` key in `ff-settings.json`. No UI or app wiring; the
legacy `register_toggle` / `release_toggle` / `TOGGLE_*` are untouched and still used.

## Tasks

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1 | Pure chord module | da0aa97 | src/overlay/chord.rs, src/overlay/mod.rs |
| 2 | Registrar seam and live runtime | 5027fe7 | src/overlay/hotkey.rs |
| 3 | `hotkey` key in ff-settings.json | cc2c6ba | src/settings.rs |
| - | Judgement calls noted for the author | f605a42 | .planning/phases/AUTONOMOUS-V072-DEFERRED.md |

## Verification

- `cargo test --locked`: all green (lib 1742 tests, including 14 chord, 17 hotkey and 9 new settings tests).
- `git diff --stat tests/fixtures`: empty.
- No test constructs `GlobalHotKeyManager`; all runtime tests use `FakeRegistrar`.
- `cargo clippy --all-targets`: no diagnostics in the three touched source files.
- `cargo build --locked`: succeeds.

## Deviations from Plan

None in behaviour. Minor additions within the contract:
- `Chord` also implements `Display` (same text as `text()`), and `HotkeyRuntime` has a manual `Debug`.
- `RegisterError` derives `Debug, Clone, PartialEq, Eq`.
- `render_hotkey` carries `#[allow(clippy::result_unit_err)]` to keep the contracted signature.

## Notes for Plans 03 / 04

- Build capture state with `CaptureState::default()` (it has a private `last_held` field).
- `begin_swap` returns the finished status sentence on refusal; `finish_swap` / `abort_swap` are no-ops for the live chord.
- `FakeRegistrar` is `pub(crate)` at module level in `hotkey.rs` under `#[cfg(test)]`.
- `cargo fmt --check` reports a pre-existing diff in `src/skin/editor.rs` (not touched here).

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: src/overlay/chord.rs, src/overlay/hotkey.rs, src/settings.rs
- FOUND commits: da0aa97, 5027fe7, cc2c6ba, f605a42
