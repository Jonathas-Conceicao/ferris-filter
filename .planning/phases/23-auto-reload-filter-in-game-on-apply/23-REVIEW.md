---
phase: 23-auto-reload-filter-in-game-on-apply
reviewed: 2026-10-13T00:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - Cargo.toml
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/chat.rs
  - src/overlay/config_view.rs
  - src/overlay/mod.rs
  - src/overlay/win.rs
  - src/settings.rs
findings:
  critical: 2
  warning: 5
  info: 6
  total: 13
status: issues_found
---

# Phase 23: Code Review Report

**Reviewed:** 2026-10-13
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

I reviewed the Phase 23 diff (`b4f46b3..HEAD`): the `chat.rs` step plan and executor, the Win32
wrappers in `win.rs` (window lookup, focus, `SendInput`, clipboard), the `run_apply` and `update`
wiring in `app.rs`, the status strings in `apply.rs`, the CONFIG toggle, and the settings field.
The unit tests for `chat`, `settings` and `win` pass (30/30).

The low-level Win32 code is sound in the places I checked. The `INPUT` layout and `cbSize` are
correct. Chords go out in one atomic `SendInput` batch. `OpenProcess` handles are always closed.
The `EnumWindows` context pointer outlives the call. `GlobalAlloc` ownership is correct on every
path through `SetClipboardData`. A drop guard closes the clipboard on every path after a
successful open.

The defects are in how the feature behaves around that code:

1. Moving focus to the game makes the stripe hide itself through its existing click-away rule,
   and hiding clears the status line. So the user never sees "Reloaded in game" or any failure
   reason after the focus step.
2. The first Enter of the plan sends any text the user had left in an open chat box. That is an
   unintended second chat message from one click.

Clipboard restore and the settings fallback also have edge cases where they lose data or fail
open.

## Critical Issues

### CR-01: A successful focus change hides the stripe and erases every reload status message

**File:** `src/overlay/app.rs:1465-1491` (with `src/overlay/app.rs:875-891`, `src/overlay/chat.rs:89-104`, `src/overlay/visibility.rs:93-98`)

**Issue:** Clicking APPLY activates the stripe, so it is in `Visibility::ShownFocused`. The
worker's first step, `Step::FocusGame`, calls `SetForegroundWindow(game)`. On the next frame
(at most about 50 ms later), `visibility::advance(ShownFocused, focused = false, _)` returns
`Hidden`. That triggers `Effect::Conceal`, and the conceal branch runs
`self.apply_message = None`.

When the worker's result reaches `self.reload.poll()`, `landed_reload_message` compares the
current status line (`None`) with `pending.interim`, finds no match, and drops the result.

As a result, on every send where focus succeeded:
- The stripe disappears. It is not a deliberate dismissal; it is a side effect of the focus change.
- "Wrote <path>. Reloaded in game" is never shown. CONTEXT "Status after Apply" requires it.
- Every failure after focus is silent: modifier held (`VerifyModifiersUp` runs after
  `FocusGame`), clipboard busy, a later `VerifyForeground`, or a `SendInput` short count. The user
  gets neither the reason nor the `RELOAD_INSTRUCTION` Refresh reminder. The reload did not
  happen and nothing says so.

Manual-verification checks 1, 12 and 13 expect status text that this code path cannot display.
The `landed_reload_message` doc comment treats "stripe concealed" as a rare, acceptable case, but
it happens on every send.

**Fix:** Keep the result outside the status line that conceal clears, and show it on the next
reveal. For example, store it in its own field that conceal leaves alone. Also keep the stripe
from hiding when it is the reload that moved focus:

```rust
// app.rs: a reload-owned focus change must not count as a user click-away
let reload_in_flight = self.reload.is_running();
let next = if reload_in_flight && self.visibility == Visibility::ShownFocused && !self.toggle_requested {
    Visibility::ShownUnfocused // stay visible, never-focused guard semantics
} else {
    visibility::advance(self.visibility, focused, self.toggle_requested)
};
...
// and on landing, if the interim line was cleared, still surface failures:
if let Some(result) = self.reload.poll() {
    if let Some(p) = self.reload_pending.take() {
        let msg = landed_reload_message(self.apply_message.as_deref(), &p, &result)
            .or_else(|| result.is_err().then(|| /* build failure line */));
        if msg.is_some() { self.apply_message = msg; }
    }
}
```

Separately, move `Step::VerifyModifiersUp` before `Step::FocusGame`. Then a held modifier
aborts before focus moves, and the stripe can still show the reason.

### CR-02: The first Enter sends the user's unfinished chat text, which is a second chat message from one click

**File:** `src/overlay/chat.rs:87-107` (module doc `chat.rs:7-10`)

**Issue:** The plan is `Tap(Enter)` and then `Chord(Ctrl, A)`, `Chord(Ctrl, V)`, `Tap(Enter)`.
The module doc and manual check 7 (assumption A2) say Ctrl+A clears partial text already in the
chat box. It cannot. If the chat input is still open when the user clicks the stripe (PoE keeps
it open across focus loss), the first `Tap(Enter)` submits that text before Ctrl+A runs.

The text could be a half-typed public message, a whisper (`@name ...`), or another slash command.
After that:
1. The chat box closes.
2. Ctrl+A and Ctrl+V go to the game world.
3. The final Enter opens an empty chat box and leaves it open.
4. `/reloaditemfilter` is never sent.
5. The status still reports success (see WR-01).

This breaks the CONTEXT "ToS posture" guarantee (one click, one command). It sends user-authored
text the user never confirmed, and it cannot be undone. The CONFIG caption ("Close the chat box
before pressing Apply") is only a request to the user; the code does nothing to prevent the
problem.

**Fix:** The game's chat state cannot be read, so make the sequence safe when chat is already
open. Clear the input before the first Enter, so an open box submits an empty line (which PoE
ignores) instead of the user's text:

```rust
pub fn reload_steps() -> Vec<Step> {
    vec![
        Step::VerifyModifiersUp,
        Step::FocusGame,
        Step::Wait(FOCUS_SETTLE_MS),
        Step::VerifyForeground,
        Step::SaveClipboard,
        Step::SetClipboard(RELOAD_COMMAND.to_string()),
        Step::VerifyForeground,
        // If chat is already open, Ctrl+A + Backspace empties it so the next Enter sends nothing.
        Step::Chord(Key::Ctrl, Key::A),
        Step::Tap(Key::Backspace),
        Step::Tap(Key::Enter),
        ...
    ]
}
```

Before relying on this, check in game whether Ctrl+A or Backspace does anything when chat is
closed. Otherwise, make the caption and the manual script say plainly that an open chat box will
be sent. Also correct the module doc, which currently claims Ctrl+A handles this case.

## Warnings

### WR-01: "Reloaded in game" is reported without any evidence the game received the command

**File:** `src/overlay/apply.rs:92-95`, `src/overlay/chat.rs:112-136`

**Issue:** `Ok(())` from `execute` only means each `SendInput` call returned the count it was
given. It does not show that the command reached the game. The `send_batch` doc admits that UIPI
blocking (game elevated, ff not) cannot be detected. Keys dropped by timing (manual check 15) and
the CR-02 open-chat case also end in `Ok(())`. In all of these, the status says
"Reloaded in game" and drops the Refresh reminder, so the user trusts a filter that was never
reloaded.

**Fix:** Phrase the success line as what was actually done, and keep the fallback visible:
`"Wrote <path>. Sent /reloaditemfilter to the game (if nothing changed: Options > Game > Item Filter > Refresh)"`.
Or append `RELOAD_INSTRUCTION` in the shorter form.

### WR-02: A failed clipboard write after `EmptyClipboard` loses the user's clipboard permanently

**File:** `src/overlay/win.rs:458-497`, `src/overlay/chat.rs:294-318`

**Issue:** `write_clipboard_text` calls `EmptyClipboard()` first, then can still fail at
`GlobalAlloc`, `GlobalLock` or `SetClipboardData`. On that `Err`, `WinSink.ours` is never set.
`execute` then runs `RestoreClipboard`, but the guard
`ours == Some(win::clipboard_sequence())` compares `None` with `Some(..)` and skips the restore.
The user's clipboard has already been emptied and is never restored.

The same happens when `owner` is `None`. Microsoft documents that `SetClipboardData` fails after
`OpenClipboard(NULL)` + `EmptyClipboard`, which is exactly the case the `ClipboardOwner` comment
describes.

**Fix:** In `RestoreClipboard`, restore when `ours` is `None` but `saved` is `Some`. Our failed
write is the only thing that could have emptied the clipboard. Alternatively, have
`write_clipboard_text` return the post-`EmptyClipboard` sequence number even on failure:

```rust
Step::RestoreClipboard => {
    let saved = self.saved.take();
    let ours = self.ours.take();
    let untouched_since_us = ours.map_or(true, |s| s == win::clipboard_sequence());
    if untouched_since_us { /* restore saved as today */ }
    Ok(())
}
```

Also skip the send entirely when `owner` is `None`. Do not fall back to a null owner.

### WR-03: The modifier check runs after focus has already moved to the game

**File:** `src/overlay/chat.rs:89-92`

**Issue:** `VerifyModifiersUp` comes after `FocusGame`. A held modifier therefore aborts the
send, but focus has already left the stripe, and CR-01 then hides the stripe and erases the
reason. The user is moved into the game with no reload and no message. The check reads global
key state and does not depend on focus, so it can run before any side effect.

**Fix:** Make `Step::VerifyModifiersUp` the first step. Update `plan_is_exact` and
`first_verify_failure_sends_nothing_and_never_saved` to match.

### WR-04: A wrongly typed `reload_after_apply` turns the keystroke feature back on

**File:** `src/settings.rs:76-79`, `src/settings.rs:41-43`

**Issue:** In the salvage path, any value that is not a JSON bool (for example `"false"` or `0`,
easy to write when editing `ff-settings.json` by hand) becomes `None`, and `None` means on.
A user who opted out of input injection is silently opted back in. The warning text from `warn`
reads "...could not be used (...); using dynamic.filter." That mentions the filter, which was
salvaged correctly, and says nothing about the reload setting changing. For a feature that sends
synthetic keystrokes, a corrupt opt-out should fall back to off, or at least the warning should
say what happened.

**Fix:**
```rust
let reload_after_apply = match extra.remove("reload_after_apply") {
    Some(serde_json::Value::Bool(b)) => Some(b),
    Some(_) => Some(false), // unreadable opt-out: fail closed
    None => None,
};
```
Also make the warning name the key that could not be used, instead of always saying
"using dynamic.filter".

### WR-05: The clipboard read treats a `GlobalLock` failure as "clipboard had no text", so the restore empties it

**File:** `src/overlay/win.rs:434-442`, `src/overlay/chat.rs:314`

**Issue:** `read_clipboard_text` returns `Ok(None)` when `GetClipboardData` or `GlobalLock`
returns null, even though `IsClipboardFormatAvailable(CF_UNICODETEXT)` just said text is there.
`WinSink` records `Some(None)`, and the restore step then calls `clear_clipboard`. Text that was
on the clipboard is deleted instead of restored.

**Fix:** Return `Err("could not read the clipboard")` from those two branches. `SaveClipboard`
then aborts before anything is written, which is the existing fail-closed path.

## Info

### IN-01: `Chord(Ctrl, V)` has no foreground check directly before it

**File:** `src/overlay/chat.rs:99-100`
**Issue:** Every tap and the Ctrl+A chord is preceded by `VerifyForeground`, but Ctrl+V follows
Ctrl+A directly. The race window is tiny, but the doc claim "a foreground check precedes ... every
key press" is not literally true, and `plan_invariants` exempts this step on purpose.
**Fix:** Insert `Step::VerifyForeground` between the two chords, or fix the doc.

### IN-02: The `Busy` branch after `self.reload.start` is dead code

**File:** `src/overlay/app.rs:892-898`
**Issue:** `decide` was already given `self.reload.is_running()` on the same thread with no poll
in between, so `start` cannot return `false` here.
**Fix:** Replace the branch with `debug_assert!(started)`, or keep it and add a comment saying it
is defensive.

### IN-03: A failure after the chat box opens leaves pasted text waiting in the game

**File:** `src/overlay/chat.rs:96-105`, `src/overlay/chat.rs:127-131`
**Issue:** If `VerifyForeground` at step 13 fails after Ctrl+V, chat stays open with
`/reloaditemfilter` in it. The user's next Enter in game sends it. That is harmless for this
command, but it is a send the user did not trigger with Apply.
**Fix:** Document it in the failure message, for example "chat may still be open".

### IN-04: Only `CF_UNICODETEXT` is saved, so rich clipboard content is downgraded

**File:** `src/overlay/win.rs:375-377`
**Issue:** Images are documented as lost, but copied HTML/RTF (for example from a browser or
Word) is also reduced to plain text, and files copied in Explorer are lost. The write and restore
also add two entries to Windows clipboard history (Win+V) and cloud clipboard sync.
**Fix:** Mention the formatting loss in the CHANGELOG or caption. Optionally, set the registered
`ExcludeClipboardContentFromMonitorProcessing` format alongside the command text so it stays out
of clipboard history.

### IN-05: The toggle's caption is shown twice

**File:** `src/overlay/config_view.rs:370-381`
**Issue:** `RELOAD_TOGGLE_CAPTION` is both the toggle's hover text and the helper line directly
beneath it.
**Fix:** Drop the `on_hover_text`, or use a shorter tooltip.

### IN-06: PoE1 also matches the game-window rule

**File:** `src/overlay/win.rs:36-59`
**Issue:** "Path of Exile" plus `PathOfExile*.exe` also matches a running PoE1 client, which would
receive focus, keystrokes and a clipboard swap. The command is harmless there, but this is
another case of the WR-01 false "Reloaded in game".
**Fix:** Match the title "path of exile 2", or add a comment saying PoE1 is accepted on purpose.

---

_Reviewed: 2026-10-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
