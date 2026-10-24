---
phase: 47-customizable-hotkey
verified: 2026-10-23T00:00:00Z
status: human_needed
score: 4/4 roadmap success criteria verified in code (live-game behaviour pending human check)
overrides_applied: 0
human_verification:
  - test: "Capture Ctrl+Alt+K on the Config tab, Apply, then press it over the game"
    expected: "Outcome ends `Show / hide is now Ctrl+Alt+K.`; Ctrl+Alt+K toggles the window, Ctrl+Shift+Z no longer does, no restart"
    why_human: "Real OS RegisterHotKey behaviour cannot be exercised in tests (tests use a fake registrar by rule)"
  - test: "Bind a chord another running app owns and Apply"
    expected: "`<new> is in use by another app. <old> still works; nothing was applied.`; old chord still toggles; edit still pending"
    why_human: "Needs a second app that owns a global hotkey; depends on global-hotkey mapping RegisterHotKey failure to AlreadyRegistered"
  - test: "Foreground rule after a swap: press the new chord in a browser/editor"
    expected: "Nothing happens outside the game or the overlay"
    why_human: "Foreground detection is a live Win32 check"
  - test: "Hover the tray icon and the header close control after Apply; quit and relaunch"
    expected: "`FerrisFilter — Ctrl+Alt+K to toggle`, `Hide (Ctrl+Alt+K)`; chord kept after relaunch"
    why_human: "Tray hover is a live shell interaction"
  - test: "Capture edge cases in the live window (Z alone, Escape, click elsewhere, tab switch, live hotkey while capturing, bare F8, reset + Apply)"
    expected: "As listed in AUTONOMOUS-V072-DEFERRED.md Phase 47 items 9-14"
    why_human: "Real keyboard input/OS event delivery; already stacked in the deferred list"
  - test: "Screenshot eye check of the keybind row art (rest/custom/capture/invalid.png)"
    expected: "Matches the game's Options -> Input keybind button look"
    why_human: "Visual appearance; stacked as deferred items 5-8"
---

# Phase 47: Customizable Hotkey Verification Report

**Phase Goal:** The player chooses the show / hide chord.
**Verified:** 2026-10-23
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (roadmap success criteria + plan must-haves)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | CONFIG tab has a Show / hide overlay row in the game's keybind style; clicking the chord button captures the next chord | VERIFIED | `config_view.rs:364` calls `show_overlay` between Filter and Styles; `show_overlay` (l.373-441) draws `ff_row("Show / hide overlay")` with `skin::widgets::ff_keybind_button` (208pt) and `ff_reload_button` reset; click pushes `StartHotkeyCapture` -> `app.rs:3442 start_capture`. `capture_frame` runs first in `ui()` (`app.rs:3748`), drains key/text/clipboard/IME events via `ctx.input_mut` and feeds `chord::capture_step`; `Commit(chord)` -> `pick_hotkey`. Art: `assets/poe2/options/keybind_{border,plate}.png` from `optionsbuttonborder.dds`/`optionsbutton.dds` manifest entries; textures in `skin/textures.rs:173-174`. Screenshots rest/capture/invalid show the row and `Press a key…`. |
| 2 | A captured chord is a pending Config edit (Revert discards); Apply saves it in ff-settings.json and it works at once, still only while game/overlay is foreground | VERIFIED (code) | `ConfigState.hotkey_pending` folded into `dirty_count` (l.477) and `RevertScope::Styles` count (l.808), cleared by Config revert (l.786). `apply_all::prepare` step 4 renders `settings::render_hotkey` as the 4th all-or-none target; `commit` clears the edit and returns `hotkey_saved`. `apply_everything` (`app.rs:688-717`): validate -> `begin_swap` (register new beside old) -> commit -> `finish_swap` (unregister old) or `abort_swap`. Hotkey poll (`app.rs:3780-3797`) matches `self.hotkey.live_id()` and still gates on `win::foreground_is_game_or_overlay`. Live OS behaviour needs human check. |
| 3 | No-modifier chord refused (bare F1-F24 allowed); chord owned by another app refused, old binding kept, status says why | VERIFIED (code) | `Chord::is_bindable` (`chord.rs:344`) requires Ctrl/Shift/Alt unless `ChordKey::F(_)`; `capture_step` records `NoModifier`/`WindowsKey`/`Unsupported` and keeps capturing. `GlobalHotKeyManager` registrar maps `AlreadyRegistered` -> `RegisterError::InUse`; `begin_swap` returns `"{new} is in use by another app. {old} still works; nothing was applied."` and Apply returns `AllResult::failed(message)` before any write. Unit tests `swap_to_a_taken_chord_is_refused_and_keeps_the_old_one` and app-level test (l.8701) cover it with a fake registrar. |
| 4 | Reset restores Ctrl+Shift+Z; invalid saved chord falls back with a warning; tray tooltip shows the live chord | VERIFIED (code) | `ConfigAction::ResetHotkey` -> `pick_hotkey(chord::DEFAULT)`; reset greyed at default except after a stale fallback (`hotkey_row_view`, `saved_hotkey_stale`). `settings::load` drops an unusable `hotkey` with `` `hotkey` is not a usable key combination, so Ctrl+Shift+Z is used `` and sets `hotkey_invalid`; `HotkeyRuntime::startup` falls back on an in-use saved chord with its own warning; both joined into `list_status` (`app.rs:1683`). Tray built from `hotkey.tooltip()` (`mod.rs:102`) and updated by `tray.set_hotkey_state` after a saving Apply (`app.rs:2183`); header close tooltip `Hide ({shown})` (`app.rs:4099`); second-launch message no longer names a chord (`instance.rs:154`). |

**Score:** 4/4 truths verified at code level; live OS/tray behaviour routed to human verification.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/overlay/chord.rs` | Chord, DEFAULT, parse/text/is_bindable, capture state machine | VERIFIED | 1088 lines, 33 tests pass |
| `src/overlay/hotkey.rs` | HotkeyRegistrar seam, HotkeyRuntime | VERIFIED | startup/begin/finish/abort/release/tooltip; fake-registrar tests |
| `src/settings.rs` | `hotkey` key, validation, `render_hotkey` | VERIFIED | other keys preserved; unusable file refuses render |
| `src/overlay/config_state.rs` | `hotkey_pending` edit | VERIFIED | dirty/revert wiring |
| `src/overlay/apply_all.rs` | SettingsPart 4th target, `hotkey_saved` | VERIFIED | |
| `src/overlay/tray.rs` | `set_hotkey_state` | VERIFIED | |
| `src/overlay/config_view.rs` | `show_overlay`, actions | VERIFIED | |
| `src/skin/widgets.rs` | `ff_keybind_button` | VERIFIED | 5 keybind tests pass |
| `assets/poe2/options/keybind_*.png` + manifest | extracted game art | VERIFIED | manifest l.301/307 |
| `src/overlay/screenshot.rs` | `--hotkey`, `--capture-hotkey` | VERIFIED | documented in CLAUDE.md |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| settings.rs | chord.rs | `usable_hotkey` -> `Chord::parse` + `is_bindable` | WIRED |
| hotkey.rs | global_hotkey | `AlreadyRegistered` -> `InUse` | WIRED |
| app.rs apply_everything | HotkeyRuntime swap | between `prepare` and `commit` | WIRED |
| apply_all.rs | `settings::render_hotkey` | settings target bytes | WIRED |
| mod.rs launch | `HotkeyRuntime::startup` | `settings::load(root).saved_hotkey()` before `tray::build` | WIRED |
| config_view show_overlay | `ff_keybind_button` | row control column | WIRED |
| app.rs ui() | `chord::capture_step` | `ctx.input_mut` drain at frame top | WIRED |
| capture commit / reset | `ConfigState::set_hotkey` | `pick_hotkey` with `hotkey_base()` | WIRED |
| begin_shutdown | `hotkey.release()` | before tray and instance name | WIRED |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Chord rules / capture | `cargo test --locked --lib chord` | 33 passed | PASS |
| Hotkey runtime, settings, app wiring (fake registrar) | `cargo test --locked --lib hotkey` | 41 passed | PASS |
| Keybind widget | `cargo test --locked --lib keybind` | 5 passed | PASS |
| Full suite | orchestrator `cargo test --locked` | 1781 lib tests pass | PASS (reported) |

No test registering a real OS hotkey was run.

### Probe Execution

None declared; not applicable.

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| KEY-01 | 47-01, 47-02, 47-04 | SATISFIED | Truth 1 |
| KEY-02 | 47-03, 47-04 | SATISFIED (live part needs human) | Truth 2; Quit popup and tab ` *` use `is_dirty`/scope counts that include the hotkey |
| KEY-03 | 47-01, 47-03, 47-04 | SATISFIED (live in-use needs human) | Truth 3 |
| KEY-04 | 47-01, 47-03, 47-04 | SATISFIED | Truth 4 |

No orphaned requirements: REQUIREMENTS.md maps exactly KEY-01..04 to Phase 47.

### Anti-Patterns Found

No TBD/FIXME/XXX/TODO/todo!/unimplemented! added in the phase diff under `src/` or `tools/`. README.md / CHANGELOG.md untouched (per CLAUDE.md); their stale Ctrl+Shift+Z wording is noted in the deferred list for the Release Review (Info).

### Human Verification Required

Live checks are already stacked in `.planning/phases/AUTONOMOUS-V072-DEFERRED.md` (Phase 47 items 5-14):

1. Screenshot eye check of the Overlay row (rest/custom/capture/invalid.png).
2. Capture Ctrl+Alt+K, Apply, confirm it toggles over the game with no restart and Ctrl+Shift+Z stops.
3. Capture refusals (Z alone, Escape, click elsewhere, tab switch, live hotkey while capturing).
4. Chord owned by another app is refused with the old binding kept.
5. Revert and the Quit popup count a pending chord.
6. Tray tooltip / header close tooltip show the live chord; relaunch keeps it.
7. Bare F-key accepted; reset + Apply restores Ctrl+Shift+Z.

### Gaps Summary

No code gaps. Every roadmap criterion and plan must-have is implemented and wired; the swap is register-new-before-unregister-old and the refusal writes nothing. Remaining items are live OS/tray/visual checks that cannot be exercised programmatically.

---

_Verified: 2026-10-23_
_Verifier: Claude (gsd-verifier)_
