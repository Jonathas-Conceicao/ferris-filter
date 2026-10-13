---
phase: 23-auto-reload-filter-in-game-on-apply
reviewed: 2026-10-13T00:00:00Z
depth: standard
iteration: 3
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
  warning: 1
  info: 2
  total: 3
status: issues_found
---

# Phase 23: Code Review Report (iteration 3)

**Reviewed:** 2026-10-13
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

This re-review covers the Phase 23 diff (`b4f46b3..HEAD`) after the iteration-2 fixes (91c9c67,
7e760e7). `cargo clippy --all-targets` reports nothing in the reviewed files, and `cargo test --lib`
passes (790 of 790).

How each iteration-2 finding was resolved:

- **WR-01: fixed correctly.** `reload_steps()` now runs `VerifyForeground` and then
  `VerifyModifiersStillUp` directly before the first Enter, before the Ctrl+A / Ctrl+V pair, and
  before the final Enter. The polling `VerifyModifiersUp` is still step 0. When any of these checks
  fails, `execute` restores the clipboard once and sends no key from the next group. The new
  `a_modifier_pressed_mid_sequence_aborts_before_the_next_key_group` test covers all three
  checks. `plan_invariants` now requires the check-pair in front of every tap and the first chord,
  and Ctrl+V can only come directly after Ctrl+A. The window that remains (a modifier pressed while
  a batch is going out) cannot be closed from outside the game, and the fix report says so.
- **WR-02: fixed in `settings::load`, but there is a hole in the caller.** A read error other than
  NotFound, invalid JSON, and JSON that is not an object now all return `Some(false)` with a warning
  that says reload is off. A missing file still means on. `RELOAD` in `app.rs` does not apply the
  loaded value when there is a warning, so in that path the new warning is false (WR-01 below).

I am not re-raising the accepted items: CR-02 (open chat), the "Reloaded in game" wording, the
write-back of `false` over an unusable file, and the iteration-1/2 Info items.

## Warnings

### WR-01: After RELOAD, a broken or opted-out settings file shows "reload after Apply is off" while the reload stays on

**File:** `src/overlay/app.rs:996-1015` (with `src/settings.rs:55-128`)

**Issue:** `reload()` keeps the in-memory settings whenever `settings::load` returns a warning and
the root did not change:

```rust
if warning.is_none() || old_root.as_ref() != Ok(root) {
    self.settings = settings;
}
```

It still shows the warning (`self.list_status = settings_warning`). Since the iteration-2 WR-02 fix
and the WR-04 salvage fix, that warning can say "...and reload after Apply is off" or
"`reload_after_apply` is not true or false, so reload after Apply is off". The returned
`Some(false)` is thrown away, so reload stays **on**, and the next Apply sends keystrokes. Cases
that reach this:

1. The overlay runs with reload on. The user hand-edits `ff-settings.json` to opt out and leaves a
   trailing comma (`{ "reload_after_apply": false, }`), the exact case WR-02 was about, then
   presses RELOAD. The status line says reload is off, but the next Apply sends.
2. The same happens with `"reload_after_apply": "false"` (the WR-04 salvage path).
3. The file is valid and contains `"reload_after_apply": false`, but the stored `filter` name is
   invalid (the strict path's `warn(...)` at `settings.rs:122-127`). The whole loaded struct is
   dropped, including a valid opt-out that parsed fine.

The fail-closed rule from WR-02/WR-04 therefore holds at startup (`app.rs:656`, which uses the
loaded value) but not on RELOAD, and the status text says the opposite of what happens. The CONFIG
toggle shows the real (on) value, so the screen contradicts itself.

The "keep what we have" rationale (a momentarily unreadable file should not reset the chosen
filter) still holds for `filter` and `extra`. It does not justify keeping reload on: off is the safe
direction, and it is the direction the warning now promises.

**Fix:** Keep the in-memory filter and unknown keys on a warning, but always take an off value:

```rust
let (settings, warning) = settings::load(root);
if warning.is_none() || old_root.as_ref() != Ok(root) {
    self.settings = settings;
} else if settings.reload_after_apply == Some(false) {
    // Fail closed: the warning says reload is off, so make it so (WR-02/WR-04).
    self.settings.reload_after_apply = Some(false);
}
warning
```

Extract this as a pure function (`merge_reloaded_settings(current, loaded, warned, same_root)`) and
add a unit test for the trailing-comma opt-out on RELOAD. If the "using dynamic.filter" part of the
warning should also be accurate in the keep path, change the text there (for example "kept the
current filter").

## Info

### IN-01: A mid-sequence modifier abort now leaves the game's chat box open more often

**File:** `src/overlay/chat.rs:113-132`
**Issue:** The iteration-2 WR-01 fix is correct, and aborting is better than sending Ctrl+Alt+A.
But the new abort points come after the first Enter. If Alt is pressed during the ~60 ms
`CHAT_OPEN_MS` wait, the chat box is left open and empty. If it is pressed before the final
Enter, the chat box is left holding `/reloaditemfilter`. Until the player closes the chat box, skill
keys go into it. This is iteration-1 IN-03 (failure after the paste), but the Alt reflex now makes
it more likely. Because of the CR-01 conceal behaviour (iteration-2 IN-02), the failure reason only
appears on the next reveal.
**Fix:** No code change is required. Optionally, make the `MODIFIER_HELD` failure text for the
second and third checks say that the chat box may still be open ("...; close the chat box in
game"), and mention this in manual check 12b.

### IN-02: The salvage-path filter fallback does not say why the name was rejected

**File:** `src/settings.rs:84-93`
**Issue:** In the strict path, an invalid stored name reports `` `<name>`: <problem> ``. In the
salvage path (some other key is wrongly typed), an invalid string name and a non-string value both
report only "using dynamic.filter", without the name or the `NameProblem`. This is cosmetic but
inconsistent.
**Fix:** In the `Some(Value::String(n))` case with an invalid name, push
`format!("`{n}`: {problem}; using {DEFAULT_FILTER}")`.

---

_Reviewed: 2026-10-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
