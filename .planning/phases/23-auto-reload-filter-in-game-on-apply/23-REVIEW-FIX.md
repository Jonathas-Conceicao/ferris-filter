---
phase: 23-auto-reload-filter-in-game-on-apply
fixed_at: 2026-10-13T00:00:00Z
review_path: .planning/phases/23-auto-reload-filter-in-game-on-apply/23-REVIEW.md
iteration: 3
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 23: Code Review Fix Report

**Fixed at:** 2026-10-13
**Source review:** .planning/phases/23-auto-reload-filter-in-game-on-apply/23-REVIEW.md (iteration 3)
**Iteration:** 3 (final; --auto cap reached)

**Summary (iteration 3):**
- Findings in scope: 1 (0 critical, 1 warning; Info findings out of scope)
- Fixed: 1
- Skipped: 0

Verification: `cargo fmt --check` clean; full `cargo test` passes (791 lib tests plus every
integration suite, 881 total, including `artifact_guard` and `generate_snapshot`). No fixture
changed. Built with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`.

## Iteration 3 Fixed Issues

### WR-01: RELOAD dropped a reload-off from a warned settings load

**Files modified:** `src/overlay/app.rs`
**Commit:** 98d316f
**Applied fix:** `reload()` now goes through a pure `settings_after_reload(current, loaded, warned,
same_root)`. A clean load or a root change replaces the settings as before; a warned load on the
same root keeps the in-memory filter and unknown keys but takes `reload_after_apply: false` from
the file, so the CONFIG toggle, the status line and the next Apply agree. Unit test
`reload_after_a_warned_load_honours_reload_off_but_keeps_the_filter`.

## Remaining (Info, out of scope)

- IN-01: a modifier pressed mid-sequence stops the send after the first Enter, which can leave the
  game's chat box open (empty or holding `/reloaditemfilter`).
- IN-02: an invalid filter name next to another wrongly typed key falls back to `dynamic.filter`
  without saying why the name was rejected.

---

# Iteration 2 (previous report)

**Fixed at:** 2026-10-13
**Source review:** .planning/phases/23-auto-reload-filter-in-game-on-apply/23-REVIEW.md
**Iteration:** 2

**Summary:**
- Findings in scope: 2 (0 critical, 2 warning; Info findings out of scope)
- Fixed: 2
- Skipped: 0

Verification: `cargo fmt --check` is clean, the build has no warnings, and the full `cargo test`
passes (790 lib tests plus every integration suite, including `artifact_guard` and
`generate_snapshot`). No fixture changed. Built with
`CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`. No test sends keystrokes, touches
the clipboard or launches `ff`.

## Fixed Issues

### WR-01: Moving the modifier check first left every keystroke without a modifier check

**Files modified:** `src/overlay/chat.rs`, `.planning/phases/23-auto-reload-filter-in-game-on-apply/23-MANUAL-VERIFICATION.md`
**Commit:** 91c9c67
**Applied fix:** The early polling check `VerifyModifiersUp` stays as step 0, before
`FocusGame`, so a modifier held at click time still fails while the stripe has focus. A new step,
`VerifyModifiersStillUp`, checks `win::modifiers_held()` once and does not wait. It now runs in
three places, each directly after a `VerifyForeground`:
- before the first Enter,
- before the Ctrl+A / Ctrl+V pair,
- before the final Enter.

Both checks fail with the shared `MODIFIER_HELD` reason, so the status text is the same. Because
`execute` already restores the clipboard on a failure after `SaveClipboard`, a mid-sequence abort
restores it exactly once.

Test changes:
- `plan_is_exact` now covers the 20-step plan.
- `plan_invariants` drops the old "exactly one modifier check" rule. It now checks that every tap,
  and the first chord, comes directly after `VerifyModifiersStillUp`, with `VerifyForeground`
  right before that. It also checks that Ctrl+V is the only chord without its own check (it
  follows Ctrl+A directly), that the immediate check is used only in front of a key group, and that
  it appears three times. The polling check must still be first and appear once.
- `first_tap_failure_restores_once` now finds the Enter tap by position instead of using index 7.
- New: `a_modifier_pressed_mid_sequence_aborts_before_the_next_key_group` fails each of the three
  checks in turn. It asserts that no key from the following group is sent and that the clipboard
  is restored once.
- The unused `is_wait` test helper was removed.

The manual script gains check 12b, a modifier pressed after the game takes focus. Check 12 is
unchanged and still holds. CLAUDE.md and the CHANGELOG do not describe the modifier check, so they
were left alone.

**Status:** fixed: requires human verification. This is sequencing logic, and the real
`GetAsyncKeyState` timing can only be checked in game (manual check 12b). A modifier pressed
within microseconds after a check, while its key batch is going out, is still not caught. Nothing
outside the game can close that window.

### WR-02: The WR-04 fail-closed rule does not apply when `ff-settings.json` is not valid JSON

**Files modified:** `src/settings.rs`, `CLAUDE.md`, `CHANGELOG.md`
**Commit:** 7e760e7
**Applied fix:** `settings::load` has a new `unusable` fallback for a file that exists but cannot
be used. It returns `Settings { reload_after_apply: Some(false), ..default }` with the warning
"ff-settings.json could not be used (...); using dynamic.filter, and reload after Apply is off."
It is used in two places:
- a read error other than `NotFound`;
- a file that does not parse even as a JSON map: invalid JSON, a trailing comma, or JSON that is
  not an object.

What did not change:
- A missing file still loads the default, with reload on and no warning.
- The salvage path from WR-04 and the invalid-name fallback work as before.
- The filter fallback (`dynamic.filter`) is the same in every case.

New tests:
- `trailing_comma_opt_out_stays_off` (the reviewer's case);
- `json_that_is_not_an_object_fails_closed`;
- `unreadable_settings_file_fails_closed` (a directory at the settings path gives a non-NotFound
  read error);
- `missing_file_keeps_reload_on`.

`corrupt_file_warns_and_is_not_touched` now also asserts that reload is off and checks the warning
text.

CLAUDE.md (the reload setting note) and the CHANGELOG `[Unreleased]` entry now say that an unusable
settings file turns the reload off.

**Note:** the in-memory value is `Some(false)`, the same as in the WR-04 salvage path. The CONFIG
toggle therefore shows off. If the user later changes the filter selection or the toggle, the save
writes `"reload_after_apply": false` (or the new toggle value) over the broken file. The saved
value then matches what the stripe showed. Overwriting an unparseable file on save is existing
behaviour.

**Status:** fixed: requires human verification (fallback policy change).

## Iteration 1 (for continuity)

Iteration 1 fixed all seven in-scope findings, with commits eaaf993..88e50d5:
- CR-01: the reload status survives the conceal (eaaf993).
- CR-02: the open-chat limitation is documented (db479fa).
- WR-01: the "Reloaded in game" tooltip, plus the elevation caveat (1112850).
- WR-02: the clipboard is never lost on a failed write (8a702ee).
- WR-03: the modifier check moved before focus (4855086).
- WR-04: a wrongly typed `reload_after_apply` fails closed (23acab1).
- WR-05: a clipboard read failure is an error (88e50d5).

The full report is in `23-REVIEW-FIX.iter1.md`.

## Notes

- Info findings IN-01 to IN-04 are out of scope (`fix_scope: critical_warning`). The reviewer
  suggested adding the foreground check before Ctrl+V together with WR-01 (iteration-1 IN-01). It
  was not added. Ctrl+V still follows Ctrl+A directly, and `plan_invariants` makes that the only
  chord without its own checks.

---

_Fixed: 2026-10-13_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_

## Author decision (2026-10-15)

Recorded at the v0.5 wrap-up.

Remaining Info findings in 23-REVIEW.md (IN-01, IN-02, and earlier-iteration Info items not fixed) are accepted as tech debt.
