---
phase: 47-customizable-hotkey
reviewed: 2026-10-23T00:00:00Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - src/overlay/chord.rs
  - src/overlay/hotkey.rs
  - src/overlay/app.rs
  - src/overlay/apply_all.rs
  - src/overlay/config_state.rs
  - src/overlay/config_view.rs
  - src/overlay/instance.rs
  - src/overlay/mod.rs
  - src/overlay/screenshot.rs
  - src/overlay/tray.rs
  - src/overlay/win.rs
  - src/settings.rs
  - src/skin/textures.rs
  - src/skin/widgets.rs
  - tools/ff-art/src/main.rs
findings:
  critical: 0
  warning: 1
  info: 5
  total: 6
status: issues_found
---

# Phase 47: Code Review Report (iteration 3)

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

This pass re-reviews the `9a56894..HEAD` diff, focusing on the fix commits
`aaead74..7e2fe92` listed in `47-REVIEW-FIX.md`. For each fix I traced the
change against the egui-winit 0.36.2 sources (`on_keyboard_input`,
`is_cut_command` / `is_copy_command` / `is_paste_command`) and against the
hotkey runtime. The lib tests for `overlay::chord`, `settings::` and
`overlay::config_state` pass (128 tests). No test that registers a real OS
hotkey was run.

Verdict on each fix:

- **CR-01 (reserved chords): correct.** `is_bindable = has_modifier && !is_reserved`
  covers both the capture and `settings::usable_hotkey` / `saved_hotkey`. The
  in-game reload sends only Enter and Ctrl+V, and Ctrl+V is reserved, so no
  bindable chord can intercept the reload sequence.
- **CR-02 (Ctrl+Shift+Delete): correct.** egui checks Cut before Copy and Paste.
  `delete_down || !ctrl` resolves Shift+Delete and Ctrl+Shift+Delete to Delete,
  and Ctrl+X / Ctrl+Shift+X / Ctrl+Alt+X to X. Ctrl+Shift+Insert reaches Copy
  before Paste, so the Insert probe still commits it correctly. The frame-time
  sampling race now applies to Delete as well as Insert (IN-01).
- **WR-01 (paste chords): fixed as described.** The only side effect is a small
  capture / file asymmetry (IN-05).
- **WR-03 (hotkey-only Apply skips the in-game reload): correct.**
  `filter_written` comes from `WriteOutcome::Written`, so identical bytes count
  as not written. Any rule edit, style rewrite, `# ff:` normalisation or
  economy re-pricing that changes the bytes still reloads. Apply & quit does not
  depend on a reload having started (`run_deferred_quit` / `quit_after_apply`
  work with `reload_pending = None`).
- **WR-04 (pending chord survives config reloads): correct** on all three
  replacement paths. No other path replaces `ConfigState` after construction.
- **WR-06 (Reload warning): correct** for the case it targets. The fixer's
  rebuttal holds: a warned Reload keeps the session settings through
  `settings_after_reload`. Two wording gaps remain (IN-04).
- **WR-07 (drop the edit with no live chord): fixes the trap but regresses a
  recovery path** (WR-01 below).

**Accepted / deferred, not re-raised:** previous WR-05 (Shift-only chords on
printable keys) and WR-02 (non-US punctuation `VK_OEM_*` mismatch) are deferred
to the author in `.planning/phases/AUTONOMOUS-V072-DEFERRED.md`. WR-02 is also
listed under the module's Known limitations.

## Warnings

### WR-01: After the WR-07 fix, a default chord that failed at start-up can no longer be retried without a restart

**File:** `src/overlay/app.rs:2829-2835` (`hotkey_base`), with `hotkey_row_view` at `src/overlay/app.rs:2983-2984`
**Issue:** `hotkey_base()` now returns `Some(live.unwrap_or(shown))` when the
saved value is not stale. Here is the common case where that goes wrong:

1. There is no `hotkey` key in `ff-settings.json`.
2. At start-up `Ctrl+Shift+Z` is held by another app, so it does not register.
3. Then `live = None`, `shown = DEFAULT`, `saved_hotkey_stale() == false`, and
   the base is `Some(DEFAULT)`.

After the user closes the other app, they have no way to make FerrisFilter take
`Ctrl+Shift+Z`:

- Capturing `Ctrl+Shift+Z` equals the base, so `set_hotkey` clears the edit and
  nothing is pending.
- Reset is greyed, because `reset_enabled = chord != default || (stale && …)`
  is false.
- Apply has nothing to swap.

Before this fix, the base was `None`, so capturing the default made a pending
edit. Apply then reached `begin_swap(DEFAULT)` → `register`. That is exactly the
recovery `hotkey.rs::swap_from_failed_start_up_registers_and_clears_the_failure`
covers, and the UI can no longer reach it. The status line and tray tooltip
keep reading `FerrisFilter — Ctrl+Shift+Z unavailable: in use by another app`.
The only ways out are a relaunch, or the detour of applying a different chord
and then applying `Ctrl+Shift+Z` again.

The regression test `the_header_close_tooltip_and_base_follow_the_live_chord`
only covers the inert (no-service) runtime, where a retry could never succeed.
**Fix:** With no live chord, make re-capturing the shown chord toggle the edit
instead of comparing against a fixed base. Clear the edit when one is pending.
Otherwise make the shown chord pending, so Apply retries the registration:

```rust
fn pick_hotkey(&mut self, chord: Chord) {
    let no_live = self.hotkey.live().is_none() && !self.saved_hotkey_stale();
    let Ok(config) = &mut self.config else { return };
    let before = config.hotkey_pending();
    if no_live && chord == self.hotkey.shown() {
        if before.is_some() {
            config.clear_hotkey();          // drop the edit (WR-07)
        } else {
            config.set_hotkey(chord, None); // retry the registration on Apply
        }
    } else {
        config.set_hotkey(chord, self.hotkey_base());
    }
    if config.hotkey_pending() != before { self.clear_events_on_edit(); }
}
```

Keep `hotkey_base()` returning `None` in the no-live case. You can also limit
the retry branch to a runtime that has a registrar, so that with no hotkey
service a capture of the shown chord only clears. Add an app-level test: no
saved key, default taken at start-up, then capture DEFAULT → pending → Apply →
`register Ctrl+Shift+Z` logged and `live() == Some(DEFAULT)`.

## Info

### IN-01: Insert / Delete probes are sampled at frame time, not event time (carried over, now also Cut)

**File:** `src/overlay/app.rs:2933-2934`, `src/overlay/win.rs:476-485`
**Issue:** `insert_held()` and the new `delete_held()` read `GetAsyncKeyState`
when the frame runs. A `Ctrl+Insert` tapped and released before the frame is
read as `Ctrl+C` (refused as reserved). A fast `Ctrl+Shift+Delete` is read as
`Ctrl+Shift+X`, a different chord that is then committed and saved. The window
is small, because input wakes the frame at once.
**Fix:** Optional. Latch the down transition (for example, check the
`GetAsyncKeyState` low "pressed since last call" bit, or keep a short-lived flag
set from a raw `Key::Delete` / `Key::Insert` seen on the same frame). Otherwise,
document the race in the module's Known limitations.

### IN-02: A changed-on-disk ff-settings.json at commit is reported as a generic write error (carried over)

**File:** `src/overlay/apply_all.rs:452-473`
**Issue:** The guard mismatch on the settings target is not one of the
styles / tiers / filter slots. It falls through to
`Nothing was applied: …`. No data is lost.
**Fix:** Map it to a clear sentence, for example
`ff-settings.json changed while applying; nothing was applied. Press Apply again.`

### IN-03: The header close tooltip names a chord that is not registered (carried over)

**File:** `src/overlay/app.rs:4129`
**Issue:** `Hide ({shown})` is shown even when `hotkey.live()` is `None`.
**Fix:** Use plain `Hide` when `self.hotkey.live().is_none()`.

### IN-04: Reload hotkey messages: misleading wording when the live chord is the default, silence for a valid hand edit

**File:** `src/settings.rs:184-199` (`reload_hotkey_warning`), `src/overlay/app.rs:3256-3259`
**Issue:**
- When the live chord is `Ctrl+Shift+Z` and the file now holds an unusable
  value, the reworded text says
  `Ctrl+Shift+Z stays in use until the next launch`. It actually stays in use
  after the next launch too, since start-up falls back to the same chord.
- A clean Reload of a hand edit to a *different valid* chord (say
  `Ctrl+Alt+J` while `Ctrl+Alt+K` is live) shows no message at all. The row
  keeps showing `Ctrl+Alt+K`. The saved value is now stale, so Reset is enabled
  even though the shown chord is the live one. The new chord silently waits for
  the next launch.

**Fix:**
- Say `<live> stays in use` with no time clause when `live == Some(DEFAULT)`.
- Optionally, on a clean Reload where `saved_hotkey() != live()`, add
  `` `hotkey` <saved> takes effect on the next launch or Apply ``.

### IN-05: Paste-derived chords are refused by the capture but accepted from the file

**File:** `src/overlay/chord.rs:707-713`, `RESERVED` at `src/overlay/chord.rs:96-108`
**Issue:** The capture refuses `Ctrl+Shift+V`, `Ctrl+Alt+V` and
`Shift+Alt+Insert` with the caption `Used by Windows or FerrisFilter`. They are
not in `RESERVED`, so `settings::usable_hotkey` accepts them from a hand-edited
file. The caption is also slightly inaccurate for them: the real reason they are
refused is egui's clipboard dependence. This is documented in the module's
Known limitations and harmless.
**Fix:** Either add these three chords to `RESERVED`, so the file and the
capture agree, or keep the asymmetry and leave it documented as is.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
