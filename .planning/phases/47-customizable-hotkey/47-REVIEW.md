---
phase: 47-customizable-hotkey
reviewed: 2026-10-23T00:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src/overlay/app.rs
  - src/overlay/hotkey.rs
  - src/overlay/chord.rs
  - src/overlay/config_state.rs
findings:
  critical: 0
  warning: 1
  info: 5
  total: 6
status: issues_found
---

# Phase 47: Code Review Report (iteration 4)

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

This pass checks commit `4288d96`, which fixes iteration 3's WR-01
(retrying a default chord that failed at start-up). I traced
`pick_hotkey`, `hotkey_base`, `no_live_chord`, `hotkey_retry_possible`,
`saved_hotkey_stale`, the Reset `reset_enabled` expression,
`capture_live_hotkey`, and `HotkeyRuntime::{startup, inert, begin_swap,
can_register}` in four hotkey states. I ran the two app tests the fix
touches or adds, which use fake registrars only. Both pass. No test
that registers a real OS hotkey was run.

**The fix works for the case it targets.** With no saved key and the
default taken at start-up (`live = None`, `shown = DEFAULT`, not stale,
registrar present):
- Capturing `DEFAULT` with nothing pending makes it pending.
- Capturing it again clears it, so the WR-07 escape still works.
- A pending other chord can be cleared by capturing `DEFAULT` or by
  Reset.
- Reset is enabled while nothing is pending.
- Apply reaches `begin_swap(DEFAULT)` and registers it.

Other checks, per state:

- **Registered chord:** unchanged. `no_live` is false, and the base is
  the live chord.
- **No hotkey service, nothing saved:** `can_register()` is false.
  Capturing the shown chord only clears, and Reset is greyed when
  nothing is pending.
- **No pending edit at launch:** confirmed. `pick_hotkey` runs only
  from a capture commit, Reset, and `capture_live_hotkey`. The last one
  does nothing when no chord is live.
- **Minor semantic, not a defect:** in that state, Reset with a pending
  other chord clears the edit instead of staging `DEFAULT`. A second
  Reset stages it.

**The escape is still gated on `!saved_hotkey_stale()`.**
`saved_hotkey_stale()` is also true for "saved valid chord, nothing
registered". So the WR-07 trap is still there in the failed-saved-chord
state and the no-service-with-saved-key state. This fix also makes
that state more common (WR-01 below).

WR-05 and WR-02 are deferred to the author and are not raised again.

## Warnings

### WR-01: When the saved chord is "stale" only because nothing registered, a pending hotkey edit still cannot be dropped and blocks every Apply

**File:** `src/overlay/app.rs:2839-2841` (`no_live_chord`),
`src/overlay/app.rs:2852-2858` (`saved_hotkey_stale`),
`src/overlay/app.rs:2869-2890` (`pick_hotkey`)

**Issue:** `saved_hotkey_stale()` returns true whenever
`saved_hotkey().is_some()` and `saved != live`. When nothing is
registered, `live` is `None`, so **any** saved valid chord counts as
stale. `no_live_chord()` is then false, `hotkey_base()` is `None`, and
every capture runs `set_hotkey(chord, None)`. That always leaves a
pending edit, so re-capturing never clears it. Reset can only stage
`DEFAULT` and then greys out. This happens in three states:

1. **The saved default fails again on a later launch.** The new retry
   path writes `"hotkey": "Ctrl+Shift+Z"` explicitly; the new test
   asserts `saved_hotkey_in(...) == Some("Ctrl+Shift+Z")`. If another
   app holds the default on any later launch, the state is: saved
   `DEFAULT`, live `None`, shown `DEFAULT`, and stale is true. This
   commit's own recovery path leads here. The same happens to anyone
   who ever saved the default through Reset.
2. **A saved chord X fails, and the default fallback fails too.**
   Startup sets `shown = DEFAULT`, `live = None`, and the saved X is
   stale.
3. **No hotkey service (`GlobalHotKeyManager::new()` failed) and a
   saved X.** `startup` returns `inert(X, NO_SERVICE)`, and saved X is
   not equal to live `None`.

In each state, the user captures or resets. If the chord is still
taken, or there is no registrar at all, `begin_swap` refuses the whole
Apply: `"... is in use by another app; nothing was applied."` or
`"... could not be registered (...); nothing was applied."`. Rule and
style edits are refused with it. Capturing again only swaps one pending
chord for another, so the row has no way to drop the edit. The only
way out is the Config Revert, which also discards every style edit
(`RevertScope::Styles`). That is exactly the trap WR-07 described.

State 3 is the worst: the edit can never succeed, and Reset itself
creates the trap. Reset is enabled because `chord X != DEFAULT`.

Before this commit, states 2 and 3 were already trapped. This commit
adds state 1, which the retry flow now produces on its own.

**Fix:** Two parts:
- Treat a saved chord as stale only when it differs from the chord
  that is **shown**, not from the live one, when nothing is live. Only
  then does saving the default explicitly have a purpose.
- When no chord is live, let a re-capture of the chord that is already
  pending drop the edit in every state.

For example:

```rust
fn saved_hotkey_stale(&self) -> bool {
    let target = self.hotkey.live().or(Some(self.hotkey.shown()));
    self.settings.hotkey_invalid
        || self.settings.saved_hotkey().is_some_and(|saved| Some(saved) != target)
}

fn pick_hotkey(&mut self, chord: Chord) {
    // ...
    let before = config.hotkey_pending();
    if self.hotkey.live().is_none() && before == Some(chord) {
        config.clear_hotkey();            // escape in every no-live state
    } else if no_live && chord == shown {
        if before.is_none() && retry { config.set_hotkey(chord, None) } else { config.clear_hotkey() }
    } else {
        config.set_hotkey(chord, base);
    }
    // ...
}
```

Note that with the first change, state 1 (saved `DEFAULT`, live
`None`) becomes a non-stale "no live chord" state. It then gets the
same toggle and Reset behaviour as the case that commit `4288d96`
fixed. Add app tests for states 1 and 3:
- Saved `"Ctrl+Shift+Z"`, default taken: capture `DEFAULT` gives
  pending, capturing again clears it, and Apply after the release
  registers the chord.
- Inert runtime with saved `Ctrl+Alt+K`: Reset gives pending `DEFAULT`,
  and capturing `DEFAULT` again (or capturing `Ctrl+Alt+K`) clears it.

## Info (carried over, still open)

### IN-01: Insert / Delete probes are sampled at frame time

**File:** `src/overlay/app.rs:2963-2964`, `src/overlay/win.rs`
(`insert_held` / `delete_held`)

**Issue:** A fast `Ctrl+Insert` can be read as `Ctrl+C`, which is
refused. A fast `Ctrl+Shift+Delete` can be read as `Ctrl+Shift+X`, a
different chord, which is then saved.

**Fix:** Latch the down transition, or document the race under Known
limitations.

### IN-02: A changed-on-disk ff-settings.json at commit is reported as a generic write error

**File:** `src/overlay/apply_all.rs` (commit error mapping)

**Issue:** The error falls through to `Nothing was applied: …`. No
data is lost.

**Fix:** Give it a dedicated sentence, for example
`ff-settings.json changed while applying; nothing was applied. Press Apply again.`

### IN-03: The header close tooltip names a chord that is not registered

**File:** `src/overlay/app.rs:4162`

**Issue:** The tooltip reads `Hide ({shown})` even when `hotkey.live()`
is `None`.

**Fix:** Use plain `Hide` when nothing is live.

### IN-04: Reload hotkey wording

**File:** `src/settings.rs` (`reload_hotkey_warning`)

**Issue:** Two cases:
- When the live chord is the default, the message says `stays in use
  until the next launch`. That clause is misleading, because the
  default stays in use after the next launch too.
- A clean Reload of a hand edit to a different valid chord shows no
  message.

**Fix:** Drop the time clause when the live chord is the default. You
can also add a `takes effect on the next launch or Apply` note.

### IN-05: Paste-derived chords are refused by capture but accepted from the file

**File:** `src/overlay/chord.rs` (`RESERVED`)

**Issue:** The capture refuses `Ctrl+Shift+V`, `Ctrl+Alt+V` and
`Shift+Alt+Insert`. They are not in `RESERVED`, so
`settings::usable_hotkey` accepts them from a hand-edited file. This is
documented and harmless.

**Fix:** Add them to `RESERVED`, or keep the asymmetry as documented.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
