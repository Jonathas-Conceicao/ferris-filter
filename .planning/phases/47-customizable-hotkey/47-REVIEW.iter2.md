---
phase: 47-customizable-hotkey
reviewed: 2026-10-23T00:00:00Z
depth: standard
files_reviewed: 16
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
  - assets/poe2/manifest.toml
findings:
  critical: 2
  warning: 7
  info: 3
  total: 12
status: issues_found
---

# Phase 47: Code Review Report

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 16
**Status:** issues_found

## Summary

I reviewed the diff `9a56894..HEAD` for the configurable show / hide hotkey. The
registration lifecycle looks right. On every Apply path (validation refusal,
changed-on-disk refusal, failed or rolled-back write, unchanged bytes, Overwrite,
Apply & quit) the code registers the new chord before it unregisters the old
one, gives the new chord back on a failed write, and releases it at shutdown.
All registration runs on the UI (main) thread. The settings write keeps every
other key and joins the all-or-none write as its last target. It refuses an
unusable file rather than replacing it.

The defects are in the capture and in which chords it accepts:

- egui's clipboard shortcuts are mapped back to keys incorrectly.
- The capture accepts chords that break FerrisFilter's own in-game reload
  sequence, or that swallow typing system-wide.
- On non-US keyboard layouts, punctuation keys register a different key than
  the one pressed.
- Some pending-edit edges: the hotkey edit is dropped by unrelated reloads, a
  hotkey-only Apply still triggers an in-game reload, and Reload can show a
  false status line.

## Critical Issues

### CR-01: Ctrl+V (and other OS / app-critical chords) can be bound, which breaks the in-game reload and system clipboard

**File:** `src/overlay/chord.rs:344-347` (`is_bindable`), used by `capture_step` and `settings::usable_hotkey`
**Issue:** Any Ctrl/Shift/Alt chord is accepted. The in-game reload after Apply
sends `Enter`, `Ctrl+V`, `Enter` via `SendInput`. `RegisterHotKey` also
intercepts injected input. So with `Ctrl+V` bound:

- The paste never reaches the game.
- The overlay gets a toggle press while the game is foreground, so it shows
  itself.
- The second Enter submits empty chat.
- The status line still says `Reloaded in game`.

`Ctrl+V` / `Ctrl+C` / `Ctrl+X` / `Ctrl+Z` / `Ctrl+A` also stop working in every
application on the machine. `Alt+F4` and `Alt+Tab` can be captured too, or
saved in the file. Capturing Alt+F4 also closes the eframe window, because
there is no close-request handling. Nothing in the UI warns about any of this.

**Fix:** Add a reserved list that both `capture_step` (as a new
`RejectReason::Reserved`, caption `Used by Windows or FerrisFilter`) and
`usable_hotkey` refuse:

```rust
const RESERVED: &[Chord] = &[ /* Ctrl+V (reload paste), Ctrl+C, Ctrl+X, Ctrl+A, Ctrl+Z,
    Alt+F4, Alt+Tab, Alt+Space, Alt+Escape-equivalents, Ctrl+Shift+Escape (Escape already refused) */ ];
pub fn is_bindable(&self) -> bool {
    self.key.is_valid()
        && (self.ctrl || self.shift || self.alt || matches!(self.key, ChordKey::F(_)))
        && !RESERVED.contains(self)
}
```

At minimum, refuse `Ctrl+V`, because it conflicts with the overlay's own reload
mechanism.

### CR-02: Ctrl+Shift+Delete is captured, saved and registered as Ctrl+Shift+X

**File:** `src/overlay/chord.rs:635-642`
**Issue:** egui-winit 0.36 `is_cut_command` turns `Shift+Delete` into
`Event::Cut` even when Ctrl is also held. It returns before any `Key` event is
pushed. `capture_step` then picks the key with `if input.ctrl { X } else { Delete }`.
So `Ctrl+Shift+Delete` (and `Ctrl+Alt+Shift+Delete`) becomes `Ctrl+Shift+X`. A
different chord from the one pressed is saved and registered system-wide.
`Ctrl+Shift+X` is a common application shortcut. The Copy branch already avoids
this with a `GetAsyncKeyState(VK_INSERT)` probe; Cut has no equivalent probe.
**Fix:** Probe Delete the same way Insert is probed:

```rust
// win.rs
pub fn delete_held() -> bool { key_down(VK_DELETE) }
// CaptureInput gets `delete_down`; chord.rs
egui::Event::Cut => {
    let key = if input.delete_down { ChordKey::Delete } else { ChordKey::Letter(b'X') };
    ...
}
```

Add a test: Cut with ctrl+shift held and `delete_down: true` commits
`Ctrl+Shift+Delete`.

## Warnings

### WR-01: Ctrl+V, Shift+Insert and their variants cannot be captured when the clipboard holds no text

**File:** `src/overlay/chord.rs:643-650`
**Issue:** egui-winit only pushes `Event::Paste` when the clipboard holds
non-empty text, and in every case it returns before pushing a `Key` event. With
an empty clipboard, or an image or files on it, these chords produce no event:
`Ctrl+V`, `Ctrl+Shift+V`, `Ctrl+Alt+V`, `Shift+Insert`, `Shift+Alt+Insert`. The
capture just keeps reading `Press a key…` with no rejection shown. Whether the
chord can be captured depends on what the clipboard holds.
**Fix:** If `Ctrl+V` gets reserved (CR-01), refuse the Paste-derived chords
outright with a visible rejection, so the behaviour is the same either way.
Otherwise, detect V or Insert going down with `GetAsyncKeyState` while capturing
and synthesize the press. Also say this in the module's "Known limitations".

### WR-02: On non-US layouts, punctuation keys register a different physical key than the one captured

**File:** `src/overlay/chord.rs:517-526` (`key_from_event`), `ChordKey::code`
**Issue:** Non-letter keys are resolved from egui's *physical* key (US position)
to `Code::Semicolon`, `Code::Quote` and so on. global-hotkey then maps those to
the fixed US `VK_OEM_1`, `VK_OEM_7` and so on. `VK_OEM_*` codes are assigned per
layout. On German QWERTZ, the key at the US `;` position is `Ö` (`VK_OEM_3`),
but `VK_OEM_1` is `Ü`. The user presses `Ctrl+Ö`, the button shows `Ctrl+;`, and
Apply registers the `Ü` key. Letters are handled correctly (logical key wins,
`VK_A..Z` follow the layout); punctuation is not.
**Fix:** Resolve punctuation through the logical key, or through the VK of the
pressed key (`MapVirtualKeyW(scan, MAPVK_VSC_TO_VK_EX)`), and store that.
Otherwise refuse the OEM punctuation keys on non-US layouts. At minimum, list
this under Known limitations.

### WR-03: A hotkey-only Apply still sends `/reloaditemfilter` to the game

**File:** `src/overlay/app.rs:2178-2200`
**Issue:** When a filter is loaded, `apply_everything` always returns a filter
path: identical bytes still count as applied. `run_apply` therefore runs
`chat::decide` → focus the game, Enter, Ctrl+V, Enter, even when the only
pending edit is the hotkey. A change that has nothing to do with the filter
steals focus, hides the overlay, types into game chat and overwrites the
clipboard. The chat sequence also carries the open-chat risk described in
CLAUDE.md.
**Fix:** Skip the in-game reload when the filter bytes did not change and no
style or tier edit was part of the Apply. For example, have `AllResult` report
whether anything the filter depends on was applied:

```rust
let filter_affected = styles_written || tiers_written || committed.filter.as_ref().is_some_and(|f| f.written);
```

Report only `Show / hide is now X.` in that case.

### WR-04: A pending hotkey edit is silently discarded by reloads of styles.json / tiers.json

**File:** `src/overlay/app.rs:571-576` (`reload_changed_files`), `src/overlay/app.rs:3450-3459` (`ConfigReload::ConfigOnly`)
**Issue:** The hotkey edit lives inside `ConfigState`, and these paths replace
the whole state with `ConfigState::load(root)`:

- choosing Reload in the File changed on disk popup after `styles.json` or
  `tiers.json` changed;
- the Config reload;
- a reload where the root changed.

The chord is not stored in either file, but the user's captured chord vanishes
without any message. CLAUDE.md says a changed `styles.json` / `tiers.json`
discards *config edits*. Users will not expect that to include a keybind that is
saved in `ff-settings.json`.
**Fix:** Carry the pending chord across the reload:

```rust
let keep = config.as_ref().ok().and_then(ConfigState::hotkey_pending);
*config = ConfigState::load(root);
if let (Ok(c), Some(ch)) = (config.as_mut(), keep) { c.set_hotkey(ch, None /* or base */); }
```

Alternatively, move `hotkey_pending` out of `ConfigState` into `OverlayApp`
alongside the runtime.

### WR-05: Shift-only chords on printable keys are accepted and swallow typing system-wide

**File:** `src/overlay/chord.rs:344-347`
**Issue:** Shift counts as the required modifier, so `Shift+Z`, `Shift+1`,
`Shift+/`, `Shift+Space` and `Shift+Enter` can all be bound. This phase removed
the comment in `hotkey.rs` explaining why the default moved from `Shift+Z` to
`Ctrl+Shift+Z`: a global `Shift+<letter>` is swallowed everywhere on the
machine, including PoE2 chat. The capture now allows that same regression with
no warning.
**Fix:** If the chord's only modifier is Shift and the key is a letter, digit,
punctuation key, Space or Enter, reject it with `Add Ctrl or Alt`. Keep
Shift+F-keys and Shift+navigation keys allowed. If the author wants to keep
these chords, show at least an amber tooltip or caption.

### WR-06: After Reload, the status line can claim "Ctrl+Shift+Z is used" while a different chord is live

**File:** `src/overlay/app.rs:3193-3232`, `src/settings.rs:386-402`
**Issue:** Reload calls `settings::load_detailed` again and puts its warning on
the status line. Example: an earlier Apply made `Ctrl+Alt+K` live, then the file
was hand-edited to an unusable value. The warning then reads
`` `hotkey` is not a usable key combination, so Ctrl+Shift+Z is used ``, but the
runtime still has `Ctrl+Alt+K` registered (Reload never touches it). When the
load carries no other warning, `self.settings` is replaced with
`hotkey_invalid = true`, so `hotkey_base()` becomes `None`. Capturing the live
chord then becomes a pending edit instead of clearing one.
**Fix:** On Reload, build the hotkey part of the message from the runtime (for
example, `` `hotkey` is not usable; Ctrl+Alt+K stays active until restart ``).
Or keep `hotkey` / `hotkey_invalid` from the session when the live chord did not
come from the file.

### WR-07: With no live chord, a hotkey edit cannot be dropped without discarding style edits, and it blocks every Apply

**File:** `src/overlay/app.rs:2801-2818` (`hotkey_base`), `src/overlay/config_state.rs:788-797`
**Issue:** When registration failed at start-up, `hotkey.live()` is `None` and
`hotkey_base()` returns `None`. This happens when the default is owned elsewhere
or `GlobalHotKeyManager::new()` failed. In that state, every capture and every
Reset becomes a pending chord. If that chord cannot be registered, `begin_swap`
refuses the whole Apply, rule and style edits included. The only way out is the
Config Revert, which also throws away every style edit
(`RevertScope::Styles` clears both). In the normal case, re-capturing the live
chord clears the edit, but there is no live chord here.
**Fix:** Give the hotkey row its own way to drop the edit. Let a capture of
`hotkey.shown()` clear the edit when `live()` is `None` and the saved value is
not stale. Alternatively, add a dedicated "discard hotkey edit" control, or
re-capture-to-clear against `shown`.

## Info

### IN-01: Insert detection is sampled at frame time, not event time

**File:** `src/overlay/win.rs:474-479`, `src/overlay/app.rs:2853` (`capture_frame`)
**Issue:** `insert_held()` reads `GetAsyncKeyState` when the frame runs. If
Ctrl+Insert is tapped and released before that, it is read as `Ctrl+C`. The
race window is small because input wakes the frame at once.
**Fix:** Optional: catch the Insert down transition with a low-level check, or
document the race.

### IN-02: A changed-on-disk ff-settings.json at commit is reported as a generic write error

**File:** `src/overlay/apply_all.rs:441-455`
**Issue:** `changed_on_disk_path()` is only matched against the styles, tiers
and filter slots. A guard mismatch on the settings target falls through to
`ApplyAllError::Write("Nothing was applied: …")`. Nothing is written, so no data
is lost. The message does not use the Reload / Overwrite flow, which the
settings file has no baseline for anyway.
**Fix:** Map it to a clear sentence, for example
`ff-settings.json changed while applying; nothing was applied. Press Apply again.`

### IN-03: The header close tooltip names a chord that is not registered

**File:** `src/overlay/app.rs:4099`
**Issue:** `Hide ({shown})` is shown even when `failure_line()` is `Some` (no
chord registered).
**Fix:** Use plain `Hide` when `self.hotkey.live().is_none()`.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
