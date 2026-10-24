---
phase: 47-customizable-hotkey
plan: 04
subsystem: overlay Config view / hotkey capture
tags: [hotkey, capture, config, egui, screenshot, KEY-01, KEY-02, KEY-03, KEY-04]
requires: [47-01, 47-02, 47-03]
provides:
  - config_view::show_overlay, HotkeyRowView, HotkeyCaption, ConfigViewCtx.hotkey, ConfigUi.hotkey_button_rect
  - ConfigAction::{StartHotkeyCapture, CancelHotkeyCapture, ResetHotkey}
  - OverlayApp capture state (capture, capture_was_focused), capture_frame, capture_live_hotkey, end_interrupted_capture, hotkey_row_view, pick_hotkey, saved_hotkey_stale
  - win::windows_key_held, win::insert_held
  - screenshot flags --hotkey <chord>, --capture-hotkey
affects: [CLAUDE.md, AUTONOMOUS-V072-DEFERRED.md]
tech-stack:
  added: []
  patterns: [drain key events at the top of the frame with ctx.input_mut before any widget reads them]
key-files:
  created: []
  modified:
    - src/overlay/config_view.rs
    - src/overlay/win.rs
    - src/overlay/app.rs
    - src/overlay/screenshot.rs
    - CLAUDE.md
    - .planning/phases/AUTONOMOUS-V072-DEFERRED.md
decisions:
  - Capture runs as app methods taking &egui::Context (capture_frame, end_interrupted_capture), so tests drive them with ctx.run_ui instead of the full eframe ui()
  - Outside-click cancel looks at this frame's PointerButton pressed events against the last drawn button rect; the press still reaches its widget (reset still resets)
  - Tab switch / hide / popup cancels are checked once per frame right before the popup block, after tab and close handling
  - reset_enabled uses saved_hotkey_stale (invalid saved value, or saved != live) rather than hotkey_base().is_none(), so the inert screenshot runtime (live None) keeps Reset greyed at the default
  - Keybind tints kept at the UI-SPEC starting values (GOLD rest, #E6C882 lit)
metrics:
  duration: ~50 min
  completed: 2026-10-23
  tasks: 3
  files: 6
---

# Phase 47 Plan 04: Show / hide overlay row and capture Summary

The CONFIG tab now reads Filter, Overlay, Styles. The Overlay section has one `Show / hide overlay`
row: the game's keybind button holds the chord and the round reset button sits beside it. A click
captures the next chord. Key events are drained at the top of each frame, so no other widget sees
them. Every cancel path from the UI-SPEC works, and the live hotkey counts as that chord while
capturing. Reset makes Ctrl+Shift+Z pending. Two screenshot flags show every row state.

## Tasks

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1 | Overlay section and Win32 key helpers | 974afae | src/overlay/config_view.rs, src/overlay/win.rs, src/overlay/app.rs (placeholder arms) |
| 2 | Capture wiring and screenshot flags | c3b53ae | src/overlay/app.rs, src/overlay/screenshot.rs |
| 3 | CLAUDE.md, screenshots, deferred checks | 132e8d2 | CLAUDE.md, .planning/phases/AUTONOMOUS-V072-DEFERRED.md |

## What changed

- **config_view.rs**:
  - New `show_overlay` and its guarded call in `show_sections`; `show_styles` is untouched (verified with `git diff -U0 | grep "fn show_styles"`, 0 hits).
  - The 208pt `ff_keybind_button` is coloured VALUE / HEADING / WARN by `HotkeyCaption`, followed by `FIELD_GAP` and then `ff_reload_button("Reset hotkey")`.
  - Tooltips follow the UI-SPEC copy. There is no standing caption text.
  - In headless tests (no skin), the row falls back to plain egui buttons.
- **win.rs**: `windows_key_held()` (VK_LWIN / VK_RWIN) and `insert_held()` (VK_INSERT) through a small `key_down` helper. Both return `false` off Windows and are re-exported.
- **app.rs**:
  - `capture_frame` runs first in `ui()`:
    1. Ends the capture on a focused-to-unfocused transition, or on a pointer press outside the button.
    2. Drains Key / Text / Copy / Cut / Paste / Ime events and calls `chord::capture_step`, with the Windows key read from `keys_down` Super* OR `win::windows_key_held()`, and Insert from `win::insert_held()`.
    3. On commit it calls `pick_hotkey` (`set_hotkey(chord, hotkey_base())`, which clears the last Apply message on a real change).
  - Hotkey poll: while capturing, the live id calls `capture_live_hotkey()` (the applied chord, no toggle).
  - `end_interrupted_capture` runs before the popup block: another tab, Hidden, or an active popup.
  - `hotkey_row_view` builds the row:
    - Rest caption: the pending chord, else the shown one.
    - Capture caption: `held_caption`, or the reject caption.
    - `reset_enabled`: chord != DEFAULT, or (stale saved chord and DEFAULT not pending).
  - `StartHotkeyCapture` surrenders egui focus. `ResetHotkey` cancels any capture, then picks DEFAULT.
  - The `dead_code` allow on `hotkey_base` is gone.
- **screenshot.rs**: `ShotArgs.hotkey` and `capture_hotkey`, their parser arms (`--hotkey` needs a non-empty value), the module doc, and a parse test.
- **new_screenshot**:
  - Writes the raw `--hotkey` value into the temp home's ff-settings.json with `settings::render_hotkey`, before `build`.
  - Builds `HotkeyRuntime::inert(saved_hotkey or DEFAULT)`.
  - `--capture-hotkey` forces the Config tab and starts the capture.
- **CLAUDE.md**:
  - Overlay note: the configurable hotkey, which is still system-wide and foreground-only. The tray tooltip names the live chord; the second-launch message names none.
  - `hotkey` key in the ff-settings.json row.
  - `--hotkey <chord>` / `--capture-hotkey` in the ff row.
  - Config has three sections, with the Overlay row described.
  - Config ` *` covers hotkey edits.
  - The one Apply also writes ff-settings.json when a chord edit is pending.

## Verification

- `cargo test --locked`: all green (lib 1781 tests). New tests:
  - config_view: 5 (start, cancel on second click, reset enabled/disabled, no row, source order).
  - app: 13 (commit, a text field gets nothing, focus surrendered, Escape not seen by others, rejected caption, held caption, every cancel path, a never-focused window keeps capturing, capturing the applied chord clears a pending one, the live hotkey while capturing, reset both ways, stale reset, no ConfigState).
  - screenshot: 1 (parse).
- `git diff --stat tests/fixtures README.md CHANGELOG.md installer/portable-readme.txt`: empty.
- `GlobalHotKeyManager::new` is called only in src/overlay/mod.rs. No test registers an OS hotkey.
- Clippy: nothing new in the touched code. The collapsible-if warnings around the hotkey and tray polls were there before. rustfmt was applied to app.rs and screenshot.rs only; the existing editor.rs diff was left alone.
- Screenshots in `C:\Users\jonat\AppData\Local\Temp\claude\ff47-shots\` (separate home per shot), each checked with Read:
  - `rest.png`: Ctrl+Shift+Z, reset greyed, reset aligned under the Filter refresh button.
  - `custom.png`: Ctrl+Alt+K, reset live.
  - `capture.png`: lit plate, `Press a key…`. The unfocused screenshot window did not cancel the capture.
  - `invalid.png`: Ctrl+Shift+Z, reset live, status line `ff-settings.json could not be fully …`.
- Compared with `options-keybind-rows.png`, the plate reads close to the game's (dark, gold bottom line). The tints were not changed: rest `pal::GOLD` #C8A25A, lit `#E6C882`, disabled GOLD at alpha 90.

## Deviations from Plan

- **Screenshot folder**: the orchestrator's `C:/Users/jonat/AppData/Local/Temp/claude/ff47-shots/` was used instead of the plan's `.../Temp/ff47/`.
- **Art substitutes line**: Plan 02 already wrote a `Phase 47:` line under "Art substitutes for review", so none was added. The judgement calls already recorded by Plans 01-03 (a refused registration refuses the whole Apply, Reload never re-registers, ff-art `key_white`) were not repeated. New entries: README / portable-readme wording, the capture limitations (numpad, AltGr, F12), the reset-while-capturing / clipboard-chord behaviour, and the tints.
- **reset_enabled rule**: it uses the stale-saved-chord predicate rather than the bare `hotkey_base().is_none()`. Screenshot mode's inert runtime has no live chord, so the bare form would have left Reset live at the default.
- **TDD gate**: tests were written with the implementation in each `feat` commit. There are no separate RED commits (same as Plan 03).
- **[Rule 3] Placeholder arms in Task 1**: app.rs got `hotkey: None` and no-op match arms so Task 1 compiled on its own. Task 2 replaced them.

## TDD Gate Compliance

Warning: there are no separate `test(47-04)` RED commits. Each `feat(47-04)` commit contains its tests and the implementation, and every planned behaviour test exists and passes.

## Known Stubs

None.

## Threat Flags

None. Draining happens only while capturing, and every cancel path ends the capture (T-47-12). `--hotkey` is written only after the screenshot guard has checked the temp home, and it goes through normal settings validation (T-47-14).

## Self-Check: PASSED

- FOUND: src/overlay/config_view.rs (fn show_overlay), src/overlay/app.rs (capture_step, input_mut), src/overlay/screenshot.rs (--capture-hotkey), CLAUDE.md (`hotkey`)
- FOUND commits: 974afae, c3b53ae, 132e8d2
- FOUND screenshots: rest.png, custom.png, capture.png, invalid.png
