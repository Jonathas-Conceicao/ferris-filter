---
phase: 23-auto-reload-filter-in-game-on-apply
reviewed: 2026-10-13T00:00:00Z
depth: standard
iteration: 2
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
  critical: 0
  warning: 2
  info: 4
  total: 6
status: issues_found
---

# Phase 23: Code Review Report (iteration 2)

**Reviewed:** 2026-10-13
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

This is a re-review of the Phase 23 diff (`b4f46b3..HEAD`) after the iteration-1 fixes
(eaaf993..88e50d5). I checked each fix against the code and looked for regressions.
`cargo clippy --all-targets` reports nothing in the reviewed files. The targeted lib tests
(`overlay::chat`, `settings`, `overlay::app`, `overlay::config_view`, `overlay::apply`) pass:
91 of 91.

How each iteration-1 finding was resolved:

- **CR-01: fixed correctly.** `apply_message_after_conceal` keeps the interim line only while a
  reload is pending and that line is still showing. `reload.poll()` now runs after the conceal
  block, so a result and a hide in the same frame cannot wipe the result. I traced
  `reset_filter_state` (clears `reload_pending`), the row-edit clear (`app.rs:1886`), and the Busy
  path. None of them can leave a stale interim line that survives a conceal.
- **CR-02: resolved as a documented limitation, per the author.** The module doc, the caption and
  the `reloaded_message` doc now describe the open-chat behaviour accurately.
- **WR-01: wording kept, per the author.** The tooltip says what the status certifies, and
  `send_batch` treats `sent < requested` as a failure.
- **WR-02: fixed correctly.** `ours_after_write` reads the sequence number after the guard has
  closed the clipboard. The emptied/untouched split matches where `EmptyClipboard` sits. The null
  owner fallback is gone. `with_owner` skips the send when there is no owner.
- **WR-03: done, but it opened a new gap.** The modifier check now runs before focus moves. It is
  no longer near any key press, so it protects the focus step but not the keystrokes (WR-01
  below).
- **WR-04: fixed only for the salvage path.** A file that is not valid JSON still fails open
  (WR-02 below).
- **WR-05: fixed correctly.** Both null branches return `Err`. `SaveClipboard` aborts before
  `saved` is set, so no restore runs.

No Critical issues remain.

## Warnings

### WR-01: Moving the modifier check first left every keystroke without a modifier check (regression from the WR-03 fix)

**File:** `src/overlay/chat.rs:100-120` (test `plan_invariants` at `chat.rs:440-448`)

**Issue:** Before the fix, the plan was FocusGame, Wait(60), VerifyForeground,
**VerifyModifiersUp**, then the clipboard steps and the first Enter. That put the check a few
milliseconds before the first key. Now `VerifyModifiersUp` is step 0 and runs before
`FocusGame`. Between the check and the keys come:

- the focus change,
- the 60 ms settle,
- `SaveClipboard` and `SetClipboard` (each can retry `OpenClipboard` for up to about 40 ms),
- two foreground checks.

The keys then go out at roughly 70 ms (first Enter), 130 ms (Ctrl+A, Ctrl+V) and 170 ms (final
Enter) after the check, and no modifier check runs anywhere in that time.

That gap starts at the exact moment a PoE player is most likely to press a modifier. The game has
just come to the front, and holding **Alt** to show ground labels is a reflex. If Alt goes down
in the gap:

- the first Enter arrives as Alt+Enter (the DXGI fullscreen toggle unless the game disables it);
- Ctrl+A and Ctrl+V arrive as Ctrl+Alt+A and Ctrl+Alt+V;
- the chord's `Ctrl` key-up releases a Ctrl the user may be holding.

This breaks the `modifiers_held` doc guarantee (`win.rs:341-342`: "never sends a key-up for a key
the user holds"). The new `plan_invariants` assertion that the check appears exactly once locks
this gap into the tests.

**Fix:** Keep the early check, so a modifier held at click time still fails while the stripe has
focus. Also re-check right before each key group, using a fail-fast form that does not wait
(polling for 300 ms after the game has focus would just delay the failure):

```rust
pub enum Step {
    // ...
    /// Fail if a modifier is held; polls briefly (used before focus moves).
    VerifyModifiersUp,
    /// Fail at once if a modifier is held (used right before keys).
    VerifyModifiersStillUp,
}

// reload_steps():
Step::VerifyForeground,
Step::VerifyModifiersStillUp,
Step::Tap(Key::Enter),
Step::Wait(CHAT_OPEN_MS),
Step::VerifyForeground,
Step::VerifyModifiersStillUp,
Step::Chord(Key::Ctrl, Key::A),
Step::Chord(Key::Ctrl, Key::V),
Step::Wait(PASTE_SETTLE_MS),
Step::VerifyForeground,
Step::VerifyModifiersStillUp,
Step::Tap(Key::Enter),
```

A failure there now reaches the user, because the CR-01 fix keeps the result for the next reveal.
Update `plan_invariants` so it requires a modifier check immediately before every `Tap` and before
the first `Chord`, instead of asserting a count of one.

### WR-02: The WR-04 fail-closed rule does not apply when `ff-settings.json` is not valid JSON

**File:** `src/settings.rs:59-63`, `src/settings.rs:110`

**Issue:** The WR-04 fix makes an unreadable `reload_after_apply` turn the reload off, but only in
the salvage branch. That branch runs only when the file parses as a JSON object. Two cases still
fall back to `Settings::default()`, which means reload is **on**:

1. **Invalid JSON** (`Err(_) => (Settings::default(), ...)` at line 110). WR-04 was about
   hand-editing, and the most common hand-edit mistake is a syntax error, not a wrong type. For
   example, a trailing comma or a missing quote, as in `{ "reload_after_apply": false, }`. A user
   who opted out of synthetic keystrokes is opted back in.
2. **An unreadable file** (line 62), for example a sharing violation while another process has it
   open.

In both cases the warning still says only "...could not be used (...); using dynamic.filter." It
does not say that reload after Apply is now on. This is the same silent fail-open WR-04 described,
reached through a different path.

**Fix:** Apply the same policy wherever a settings file exists but cannot be used, and say so in
the warning:

```rust
let unusable = |reason: String| {
    (
        Settings { reload_after_apply: Some(false), ..Settings::default() },
        Some(format!(
            "{SETTINGS_FILE} could not be used ({reason}); using {DEFAULT_FILTER}, \
             and reload after Apply is off."
        )),
    )
};
// Err(e) if not NotFound => return unusable(e.to_string()),
// Err(_) from the Map parse => unusable(e.to_string()),
```

A missing file (NotFound) still means the default, which is on. Add a test with a trailing-comma
file that contains `"reload_after_apply": false`.

## Info

### IN-01: `null` and duplicate keys behave differently in the salvage path than in the strict path

**File:** `src/settings.rs:84-97`
**Issue:**
- In the strict path, `"reload_after_apply": null` parses as `None`, which means on, with no
  warning. The fix report relies on this ("JSON null still means absent"). In the salvage path
  (reached when some *other* key is wrongly typed), the same `null` matches `Some(_)`. It becomes
  `Some(false)`, and the warning says it "is not true or false".
- `"filter": null` behaves the same way: silent in the strict path, a "using dynamic.filter"
  fallback in the salvage path.
- When strict parsing fails for a reason neither key explains (for example a duplicated
  `"filter"` key, which `Map` accepts and `Settings` rejects), the warning says "using the
  defaults", even though the salvaged values are in use.

**Fix:** Add a `Some(serde_json::Value::Null) | None => None` arm to both matches. Change the
empty-fallbacks text to something like "the readable keys were kept".

### IN-02: A reload result now appears long after the Apply that caused it

**File:** `src/overlay/app.rs:602-615`, `src/overlay/app.rs:1495-1507`
**Issue:** The CR-01 fix is correct, but it changes how the status line behaves. A successful send
always hides the stripe, so "Reloaded in game" (or the failure reason) is only shown on the next
reveal, which could be minutes or a whole map later. It then reads like a report about whatever
the player is doing now. That is the situation the author's 2026-09-11 rule ("a message that
outlives its own question is worse than no message") was written to prevent. This is acceptable
as a trade-off, but it is an exception to that rule, and the conceal comment does not say so.
**Fix:** Add a sentence to the conceal comment (and optionally CLAUDE.md) saying the reload line
is the one message that can outlive its frame, and why. Optionally, show a timestamp or "(last
Apply)" prefix when a kept result is revealed.

### IN-03: The clipboard is not restored if any other process writes it during the 120 ms window

**File:** `src/overlay/chat.rs:357-372`
**Issue:** The restore runs only when `ours == clipboard_sequence()`. Clipboard managers that
re-publish what they read (Ditto in some modes, RDP's `rdpclip`, VM guest tools) increase the
sequence number within milliseconds of our write. When that happens the restore is skipped, and
the user's clipboard is left holding `/reloaditemfilter` for good, with no message. The guard
protects text the user copies during the window, but it cannot tell that apart from a
re-publish of our own text.
**Fix:** Before skipping, read the clipboard. If it still equals `RELOAD_COMMAND`, restore anyway.
Skip only when its content differs from what we wrote.

### IN-04: Iteration-1 Info items still open

**File:** `src/overlay/chat.rs:112-113` (IN-01), `src/overlay/app.rs:911-917` (IN-02),
`src/overlay/chat.rs:113-116` (IN-03), `src/overlay/win.rs:408-414` (IN-04), `src/overlay/win.rs`
`is_game_window` (IN-06)
**Issue:** These were out of scope for the fixer and are unchanged:
- IN-01: `Chord(Ctrl, V)` has no foreground check directly before it. The doc at `chat.rs:96`
  still says "every key press".
- IN-02: the `Busy` branch after `start` cannot be reached.
- IN-03: a failure after the paste leaves the command waiting in the chat box.
- IN-04: rich text and file clipboard content is downgraded or lost, and the command shows up in
  clipboard history.
- IN-06: a PoE1 client also matches the game-window rule.

IN-05 was resolved through `RELOAD_TOGGLE_TIP`.
**Fix:** See `23-REVIEW.iter1.md`. If WR-01 above is fixed, add the foreground check before
Ctrl+V at the same time.

---

_Reviewed: 2026-10-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
