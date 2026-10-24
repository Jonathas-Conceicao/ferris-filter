---
phase: 47-customizable-hotkey
plan: 03
subsystem: overlay hotkey wiring
tags: [hotkey, apply, settings, tray, config]
requires: [47-01]
provides:
  - ConfigState::hotkey_pending / set_hotkey(chord, base) / hotkey_dirty / clear_hotkey
  - apply_all SettingsPart (ff-settings.json as the 4th all-or-none target), Prepared::hotkey(), Committed.hotkey_saved, apply_all::SETTINGS_UNUSABLE
  - OverlayApp field `hotkey: HotkeyRuntime`, OverlayApp::hotkey_base()
  - TrayHandles::set_hotkey_state(tooltip, failure)
  - app::resolve_root is pub(crate) (used by launch())
affects: [47-04 Config view capture row]
tech-stack:
  added: []
  patterns: [validate -> register -> write around the all-or-none writer]
key-files:
  created: []
  modified:
    - src/overlay/config_state.rs
    - src/overlay/apply_all.rs
    - src/overlay/app.rs
    - src/overlay/mod.rs
    - src/overlay/tray.rs
    - src/overlay/instance.rs
    - src/overlay/hotkey.rs
    - .planning/phases/AUTONOMOUS-V072-DEFERRED.md
decisions:
  - Apply order is validate -> register -> write; a chord in use refuses the whole Apply and writes nothing
  - ff-settings.json is not in the changed-on-disk check; its guard is the bytes read in prepare
  - Committed.hotkey_saved is Some whenever the settings target was committed (Written or Unchanged); apply_everything aborts the swap if it is not the pending chord
  - Start-up reads settings::load(root).0.saved_hotkey(); only an unresolvable folder means no saved chord
  - hotkey_base() is None after a start-up fallback, so saving Ctrl+Shift+Z explicitly is a real edit (no stuck warning)
  - RULES Reload and folder changes never re-register; only start-up and Apply do
metrics:
  duration: ~45 min
  completed: 2026-10-23
  tasks: 3
  files: 8
---

# Phase 47 Plan 03: Hotkey as a CONFIG preference, end to end — Summary

The show / hide chord now works like any other CONFIG setting. A pending chord is a counted CONFIG edit that Revert can undo. Apply saves it into `ff-settings.json`, written together with `styles.json`, `tiers.json` and the filter (all or none), and every other key in that file is kept. The new chord is registered after validation and before any write. The old one is unregistered only after every write succeeds. A chord another app owns refuses the whole Apply and writes nothing. At start-up the saved chord is registered, falling back to Ctrl+Shift+Z with a warning. The tray tooltip, the tray failure line and the header close tooltip follow the live chord. The second-launch message no longer names a chord.

## Tasks

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1 | Pending chord in ConfigState and the settings target in apply_all | 3701d5f | src/overlay/config_state.rs, src/overlay/apply_all.rs |
| 2 | Live runtime at start-up, tray, mentions and shutdown | ae3318b | src/overlay/mod.rs, tray.rs, instance.rs, hotkey.rs, app.rs |
| 3 | Swap the chord inside the one Apply, and the Config dirty marker | 24ecb61 | src/overlay/app.rs |
| - | Judgement calls for the author | 2c45f74 | .planning/phases/AUTONOMOUS-V072-DEFERRED.md |

## What changed

- **ConfigState** (`hotkey_pending: Option<Chord>`):
  - `set_hotkey(chord, base)` clears the edit when the chord equals `base`.
  - The edit is counted in `dirty_count` and `scope_dirty_count(Styles)`, and cleared by `revert()` and `revert_scope(Styles)`.
  - It is deliberately not part of `styles_dirty`, `styles_changed` or `follow_disk`.
- **apply_all**:
  - `prepare` builds a `SettingsPart` after all the other refusals. A missing file means no guard. A file that is unreadable or is not a JSON object gives the exact UI-SPEC refusal sentence.
  - `commit_with` creates the folder, then writes the settings file as the last target through the same `write_all_or_none_before` call.
  - On success the pending chord is cleared and `hotkey_saved` is reported.
- **launch()**:
  - Reads the saved chord, then calls `HotkeyRuntime::startup(GlobalHotKeyManager::new().ok()…, saved)`. The manager is kept even when registration fails.
  - The tray is built from `tooltip()` / `failure_line()`.
  - The legacy `register_toggle`, `release_toggle`, `TOGGLE_*` and `HotkeyStatus` are removed.
- **OverlayApp**:
  - Holds `hotkey: HotkeyRuntime`.
  - The poll uses `live_id()`, and the foreground rule is unchanged.
  - `begin_shutdown` calls `hotkey.release()` at the old position.
  - The header tooltip reads `Hide (<shown>)`.
  - The start-up hotkey warning is appended to the settings warning (`"{a} {b}"`).
- **apply_everything** takes `&mut HotkeyRuntime` and calls `begin_swap` → commit → `finish_swap` or `abort_swap`, then appends ` Show / hide is now <chord>.`.
- **run_apply**, after a successful save:
  - sets the in-memory `settings.hotkey` and clears `hotkey_invalid`;
  - calls `tray.set_hotkey_state(...)`.
- **Config tab marker and Config Revert** use `styles_dirty() || hotkey_dirty()`. The Quit popup and the unapplied indicator pick the edit up through `is_dirty()`.

## Verification

- `cargo test --locked`: all green (1763 lib tests). New tests:
  - config_state: 2
  - apply_all: 6
  - tray: 1
  - hotkey: 1
  - app: 10, including register/unregister ordering, in-use refusal, rollback on a write failure, no registrar calls without an edit, the stale saved chord (in use and invalid), the in-memory saved chord, Quit/Revert counting, warning joining, and the relative `game_folder` that keeps the chord.
- `git diff --stat tests/fixtures`: empty.
- `GlobalHotKeyManager::new` is called only in `src/overlay/mod.rs`. Every test uses `FakeRegistrar` or an inert runtime.
- Acceptance greps all hold: no legacy symbols, the instance message names no chord, and `"Hide (Ctrl+Shift+Z)"` does not appear in app.rs.
- clippy reports nothing new in the touched code. The remaining collapsible-if warnings around the hotkey poll were there before this plan.

## Deviations from Plan

- **TDD gate:** the tests in each task were written alongside the implementation and committed in one `feat` commit per task. There are no separate RED `test(...)` commits. Every behavior test from the plan exists and passes.
- **`ConfigState::clear_hotkey()`** is a small addition. `adopt` does not know whether the settings target was written, so `commit_with` clears the edit explicitly.
- **`OverlayApp::hotkey_base`** carries `#[cfg_attr(not(test), allow(dead_code))]` until the Config row in Plan 04 calls it.
- **`app::resolve_root` is now `pub(crate)`** so `launch()` resolves the folder exactly the way the app does.
- **`cargo fmt`** also reformatted `src/skin/editor.rs` and `src/skin/widgets.rs` (an existing diff). Those changes were reverted, so they are not part of these commits.

## TDD Gate Compliance

Warning: no separate RED `test(47-03)` commits exist (see Deviations). The GREEN `feat(47-03)` commits contain both the tests and the implementation.

## Notes for Plan 04

- Capture or Reset: `config.set_hotkey(chord, app.hotkey_base())`.
- Show the applied chord with `app.hotkey.shown()`. Show the pending chord with `config.hotkey_pending()`.
- If there is no `ConfigState` (CONFIG failed to load), no chord edit can exist.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: src/overlay/config_state.rs, src/overlay/apply_all.rs, src/overlay/app.rs, src/overlay/mod.rs, src/overlay/tray.rs, src/overlay/instance.rs, src/overlay/hotkey.rs
- FOUND commits: 3701d5f, ae3318b, 24ecb61, 2c45f74
