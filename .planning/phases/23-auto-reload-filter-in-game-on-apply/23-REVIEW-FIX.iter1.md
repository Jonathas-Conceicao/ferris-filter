---
phase: 23-auto-reload-filter-in-game-on-apply
fixed_at: 2026-10-13T19:51:32Z
review_path: .planning/phases/23-auto-reload-filter-in-game-on-apply/23-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 23: Code Review Fix Report

**Fixed at:** 2026-10-13T19:51:32Z
**Source review:** .planning/phases/23-auto-reload-filter-in-game-on-apply/23-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 7 (2 critical, 5 warning; Info findings out of scope)
- Fixed: 7
- Skipped: 0

Verification: `cargo fmt --check` clean, and the full `cargo test` passes (785 lib tests plus
every integration suite, `artifact_guard`/`generate_snapshot` included). No fixture changed. Built
with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`. No keystrokes, clipboard
access or `ff` launch in any test.

## Fixed Issues

### CR-01: A successful focus change hides the stripe and erases every reload status message

**Files modified:** `src/overlay/app.rs`
**Commit:** eaaf993
**Applied fix:** Added the pure `apply_message_after_conceal`. When the stripe is concealed while a
reload is in flight and its interim line is still showing, the line is kept. In every other case
the line is cleared as before. The `reload.poll()` block now runs after the conceal block, so a
result and a hide in the same frame cannot clear the result just written. As a result, the final
outcome ("Reloaded in game", or the Refresh reminder with the reason) replaces the interim line
while the stripe is hidden and is shown on the next reveal. The stripe still hides when the game
takes focus, as orchestrator guidance preferred. New tests:
`conceal_keeps_only_an_in_flight_reload_line` and
`a_reload_result_that_lands_while_hidden_is_shown_on_the_next_reveal`. The modifier-check part of
this finding is fixed under WR-03 (4855086). Manual check 1 was updated in 1112850.
**Status:** fixed: requires human verification (state-handling logic; confirm in game that the
result appears on the next Ctrl+Shift+Z).

### CR-02: The first Enter sends the user's unfinished chat text

**Files modified:** `src/overlay/chat.rs`, `src/overlay/config_view.rs`, `CLAUDE.md`, `CHANGELOG.md`, `.planning/phases/23-auto-reload-filter-in-game-on-apply/23-MANUAL-VERIFICATION.md`
**Commit:** db479fa
**Applied fix:** Documentation only, per the orchestrator. No key was added before the first Enter,
because with chat closed it would reach the game's bindings. The `chat.rs` module doc now says
Ctrl+A only replaces a prefix the game pre-fills when chat opens. It also adds a "Known limitation"
paragraph: an already-open chat box with text has that text sent first, and the command is not
sent. Nothing outside the game can detect this, and EE2 and APT have the same limitation. The
guarantee is reworded to "one click, one send sequence". The CONFIG caption, the CLAUDE.md overlay
note and the CHANGELOG `[Unreleased]` entry state the same behaviour. Manual check 7 now records
this as a known limitation, and a new check 7b covers the chat pre-fill case. A test asserts that
the caption contains "Close the chat box".

### WR-01: "Reloaded in game" is reported without evidence the game received the command

**Files modified:** `src/overlay/win.rs`, `src/overlay/apply.rs`, `src/overlay/config_view.rs`, `CLAUDE.md`, `CHANGELOG.md`, `.planning/phases/23-auto-reload-filter-in-game-on-apply/23-MANUAL-VERIFICATION.md`
**Commit:** 1112850
**Applied fix:** The status wording is kept, per the author's decision. The toggle now has its own
tooltip, `RELOAD_TOGGLE_TIP`, which says:
- "Reloaded in game" means the command was sent to the game window.
- A game running as administrator ignores the keys.
- The filter can still be refreshed by hand through Options > Game > Item Filter > Refresh.

This also removes the duplicated caption from IN-05.

On the code side, a short `SendInput` count was already a failure. It is now checked as
`sent < requested`, and the error message names elevation as the likely cause. The `send_batch`
and `reloaded_message` docs explain that UIPI blocking is usually invisible to the return count.

CLAUDE.md and the CHANGELOG document what the status certifies and the elevation caveat. In the
manual script, check 1 now expects the result on the next reveal, and a new check 16 covers an
elevated game.

### WR-02: A failed clipboard write after `EmptyClipboard` loses the user's clipboard

**Files modified:** `src/overlay/win.rs`, `src/overlay/chat.rs`, `src/overlay/app.rs`, `src/overlay/apply.rs`, `CLAUDE.md`
**Commit:** 8a702ee
**Applied fix:** `write_clipboard_text` now returns `ClipboardWriteError { message, emptied }`.
After a failure that follows a successful `EmptyClipboard`, `emptied` is `true`. The pure
`ours_after_write` treats such a failure as ours, so the restore guard puts the saved text back.
A failure before anything changed leaves the clipboard alone.

The `OpenClipboard(NULL)` fallback is removed: `ClipboardOwner` is no longer optional in any
clipboard call. A new pure `chat::with_owner` turns a `Send` decision without a cached stripe
window into `Skip(SkipReason::NoWindow)`. That status reads "the overlay window is not ready" and
is listed in CLAUDE.md.

New tests: `clipboard_write_failure_restores_once_and_sends_no_key`,
`a_write_that_emptied_the_clipboard_still_counts_as_ours`,
`a_send_without_a_clipboard_owner_is_skipped`, plus the extended skip-reason tests.

### WR-03: The modifier check runs after focus has already moved to the game

**Files modified:** `src/overlay/chat.rs`, `.planning/phases/23-auto-reload-filter-in-game-on-apply/23-MANUAL-VERIFICATION.md`
**Commit:** 4855086
**Applied fix:** `Step::VerifyModifiersUp` is now the first step, before `FocusGame`. Updated
tests:
- `plan_is_exact` matches the new order.
- `plan_invariants` asserts the modifier check is first and appears exactly once.
- `first_verify_failure_sends_nothing_and_never_saved` finds its step by position.
- The new `modifier_failure_happens_before_focus_moves` checks that a modifier failure never
  focuses the game.

Manual check 12 now expects the stripe to stay open and show the reason immediately.

### WR-04: A wrongly typed `reload_after_apply` turns the keystroke feature back on

**Files modified:** `src/settings.rs`, `CLAUDE.md`
**Commit:** 23acab1
**Applied fix:** In the salvage path, a non-boolean `reload_after_apply` now becomes `Some(false)`,
so a broken setting turns the reload off. The salvage warning lists each unusable key and its
fallback, for example "`reload_after_apply` is not true or false, so reload after Apply is off".
It only says "using dynamic.filter" when the filter itself fell back. JSON `null` still means
"absent", so the default stays on.

Tests:
- `wrongly_typed_reload_flag_keeps_the_filter_and_other_keys_and_fails_closed` covers `"yes"`,
  `"false"`, `0`, `1` and `[]`.
- `wrongly_typed_filter_warning_names_the_filter_fallback_only` is new.

CLAUDE.md documents that an unreadable value means off.
**Status:** fixed: requires human verification (fallback policy change).

### WR-05: The clipboard read treats a `GlobalLock` failure as "clipboard had no text"

**Files modified:** `src/overlay/win.rs`, `src/overlay/chat.rs`
**Commit:** 88e50d5
**Applied fix:** `read_clipboard_text` now returns `Err("could not read the clipboard")` when
`CF_UNICODETEXT` is advertised but `GetClipboardData` or `GlobalLock` returns null. The send then
stops at `SaveClipboard`, before anything is written. The new test
`unreadable_clipboard_aborts_before_anything_is_written` checks that this case sends no key and
runs no restore.

## Notes

- Info findings IN-01 to IN-06 were out of scope (`fix_scope: critical_warning`). IN-05, the
  duplicated toggle caption, is resolved as a side effect of WR-01.
- The Win32 wrapper changes in `win.rs` (WR-01, WR-02, WR-05) are verified only by compiling and by
  the pure-seam tests. The real send needs `23-MANUAL-VERIFICATION.md`, now with checks 7b and 16
  added.

---

_Fixed: 2026-10-13T19:51:32Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
