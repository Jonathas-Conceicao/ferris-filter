---
phase: 47-customizable-hotkey
fixed_at: 2026-10-23T00:00:00Z
review_path: .planning/phases/47-customizable-hotkey/47-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 7
skipped: 2
status: partial
---

# Phase 47: Code Review Fix Report

**Fixed at:** 2026-10-23
**Source review:** .planning/phases/47-customizable-hotkey/47-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 9 (2 critical, 7 warning; Info out of scope)
- Fixed: 7
- Skipped: 2 (WR-05 by author decision, WR-02 documented and deferred)

`cargo test --locked` is green (1786 lib tests plus the integration suites).
`tests/fixtures` is unchanged and no new dependency was added. `cargo fmt --check`
still reports diffs in `src/skin/editor.rs` and `src/skin/widgets.rs`. Those diffs
were there before this run, and this run did not touch either file.

## Fixed Issues

### CR-01: Ctrl+V (and other OS / app-critical chords) can be bound

**Files modified:** `src/overlay/chord.rs`, `src/settings.rs`, `CLAUDE.md`
**Commit:** aaead74
**Applied fix:** Added a documented `chord::RESERVED` list: `Ctrl+V` (the in-game
reload's paste), `Ctrl+C` / `X` / `A` / `Z`, `Alt+F4`, `Alt+Tab`, `Shift+Alt+Tab`,
`Alt+Space` and `Ctrl+Alt+Delete`. Escape and the Windows key were already refused.
`is_bindable` is now `has_modifier() && !is_reserved()`, so `settings::usable_hotkey`
also refuses a hand-saved reserved chord and falls back to Ctrl+Shift+Z with the usual
warning. The capture shows the new `RejectReason::Reserved` caption `Used by Windows or
FerrisFilter`, in the existing rejected (amber) style. Only the exact chord is
reserved, so `Ctrl+Shift+C` is still free. Tests were added and existing tests that
used Ctrl+Z / Ctrl+C / Ctrl+X / Ctrl+V as sample chords were adjusted. CLAUDE.md
describes the rule in the `ff-settings.json` row and in the Overlay capture text.

### CR-02: Ctrl+Shift+Delete is captured as Ctrl+Shift+X

**Files modified:** `src/overlay/chord.rs`, `src/overlay/win.rs`, `src/overlay/app.rs`
**Commit:** b240cac
**Applied fix:** Added `win::delete_held()` (`GetAsyncKeyState(VK_DELETE)`, plus a
stub for other platforms) and `CaptureInput::delete_down`. A Cut event now resolves to
Delete when Delete is held or Ctrl is not held, and to X otherwise. The new test
checks that a Cut with Ctrl+Shift and Delete held commits `Ctrl+Shift+Delete`.

### WR-01: Paste chords capturable only when the clipboard holds text

**Files modified:** `src/overlay/chord.rs`, `CLAUDE.md`
**Commit:** f823528
**Applied fix:** The capture now refuses every `Event::Paste` (`Ctrl+V`,
`Ctrl+Shift+V`, `Ctrl+Alt+V`, `Shift+Insert`, `Shift+Alt+Insert`) with the reserved
caption, and `Shift+Insert` (Windows paste) was added to `RESERVED`. When the
clipboard is empty, egui sends no event, so the button keeps showing `Press a key…`.
The result is the same either way: the chord cannot be captured. This case is now
listed under the module's Known limitations. `Ctrl+Shift+V`, `Ctrl+Alt+V` and
`Shift+Alt+Insert` can still be typed into `ff-settings.json` by hand.

### WR-03: A hotkey-only Apply still sends `/reloaditemfilter`

**Files modified:** `src/overlay/app.rs`, `CLAUDE.md`
**Commit:** 9848e35
**Status:** fixed: requires human verification (logic change)
**Applied fix:** `AllResult` now carries `filter_written`, which is true when the
filter's bytes changed and were written. `AllResult::hotkey_only()` is true when a
chord was saved and neither the filter, `styles.json` nor `tiers.json` was written. In
that case `run_apply` skips `chat::decide` and the outcome reads only `Show / hide is
now <chord>.` (plus any other extras). Every other Apply works as before, including
one that writes identical bytes. Tests cover a hotkey-only Apply (`hotkey_only`) and a
hotkey Apply that also has a style edit or a rule edit (not `hotkey_only`).

### WR-04: A pending hotkey edit is discarded by styles.json / tiers.json reloads

**Files modified:** `src/overlay/config_state.rs`, `src/overlay/app.rs`, `CLAUDE.md`
**Commit:** 99f7da7
**Applied fix:** Added `ConfigState::reloaded(root, &previous)`, which loads fresh
config state but keeps the previous `hotkey_pending`. It is used by
`reload_changed_files` (File changed on disk -> Reload), the Config-only reload, and
the root-moved / failed-config branch of RULES Reload. A unit test checks that a style
edit is dropped while the chord edit stays.

### WR-06: Reload warning claims Ctrl+Shift+Z is used while another chord is live

**Files modified:** `src/settings.rs`, `src/overlay/app.rs`, `CLAUDE.md`
**Commit:** 40f6023
**Status:** fixed: requires human verification (message logic)
**Applied fix:** Added `settings::reload_hotkey_warning(warning, live)`, which only
rewrites the `hotkey` sentence on Reload. It now reads `` `hotkey` is not a usable key
combination; <live> stays in use until the next launch ``. With no chord registered,
it ends without naming a chord. The session's `hotkey` / `hotkey_invalid` fields are
left as they were. I checked the review's second claim (`hotkey_invalid = true`
replacing the settings): a hotkey warning already makes `settings_after_reload` keep
the session settings, so that part does not happen. Keeping the session chord in
`self.settings` in other cases would also let a later whole-file save overwrite a
hand edit.

### WR-07: With no live chord, a hotkey edit cannot be dropped

**Files modified:** `src/overlay/app.rs`
**Commit:** 735d78d
**Status:** fixed: requires human verification (logic change)
**Applied fix:** `hotkey_base()` now returns `live().unwrap_or(shown())` when the saved
value is not stale, so with nothing registered, capturing the chord on the button
again clears the edit. A stale saved value (start-up fallback) still gives `None`, so
saving the default stays an explicit edit. The test that expected `None` for an inert
runtime was updated to check the new behaviour.

## Skipped Issues

### WR-05: Shift-only chords on printable keys swallow typing system-wide

**File:** `src/overlay/chord.rs:344-347`
**Reason:** Skipped by the author's binding decision ("at least one modifier", so Shift
counts). The note and the two options (reject with `Add Ctrl or Alt`, or keep with an
amber warning) were added under "Phase 47:" in
`.planning/phases/AUTONOMOUS-V072-DEFERRED.md` (commit 7e2fe92) for the author to
decide.
**Original issue:** `Shift+Z`, `Shift+1`, `Shift+/`, `Shift+Space`, `Shift+Enter` can
be bound and swallow that character everywhere, PoE2 chat included.

### WR-02: Non-US layouts register a different punctuation key than the one pressed

**File:** `src/overlay/chord.rs:517-526`
**Reason:** A full fix is large: it needs the pressed key's VK through
`MapVirtualKeyW(scan, MAPVK_VSC_TO_VK_EX)`, which egui does not expose, plus
layout-aware labels. As allowed for this run, the limitation is documented in the
module's Known limitations (commit 3545982) and deferred in
`.planning/phases/AUTONOMOUS-V072-DEFERRED.md` (commit 7e2fe92).
**Original issue:** On layouts like German QWERTZ, pressing `Ctrl+Ö` shows `Ctrl+;`
and registers `VK_OEM_1` (the `Ü` key).

---

_Fixed: 2026-10-23_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
