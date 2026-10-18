---
phase: 42-release-review
reviewed: 2026-10-18T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - Cargo.toml
  - src/overlay/app.rs
  - src/overlay/instance.rs
  - src/overlay/mod.rs
  - src/sync/league.rs
  - tests/cli_generate.rs
  - tests/isolation_guard.rs
findings:
  critical: 0
  warning: 1
  info: 1
  total: 2
status: issues_found
---

# Phase 42: Code Review Report

**Reviewed:** 2026-10-18
**Depth:** standard (diff `3412b2a..HEAD`, limited to the listed files)
**Files Reviewed:** 7
**Status:** issues_found

## Summary

The diff is a cleanup: rustfmt, removing an unused windows-sys feature, swapping the
`OpenMutexW` access-right constant, moving and rewording comments, a temp-dir change in one
test, and (from 42-03) the version bump to `0.7.0`. I checked each claim that it changes
nothing:

- **`SYNCHRONIZATION_SYNCHRONIZE`**: I checked it against the windows-sys 0.61 source in the
  local registry. It is `1048576u32` (`0x00100000`), the same value as `PROCESS_SYNCHRONIZE`.
  `OpenMutexW` is declared there with `dwdesiredaccess: SYNCHRONIZATION_ACCESS_RIGHTS`, so the
  new constant is the correct type. It lives in `Win32_System_Threading`, which is still
  enabled. `mutex_gone` and the `--quit` path both go through `open_mutex`, so they get the
  same access mask as before. `PROCESS_SYNCHRONIZE` is still imported and still used by
  `OpenProcess` (instance.rs:426), so no import was left unused.
- **`Win32_System_Diagnostics_ToolHelp` removed**: nothing in `src/` or `tools/` references
  `ToolHelp` or `CreateToolhelp32Snapshot`. This is safe.
- **Version 0.7.0**: `Cargo.lock` was bumped to match (line 1724), so `--locked` builds still
  work.
- **`tests/cli_generate.rs`**: `tempfile` is a regular dependency (Cargo.toml:40), so
  integration tests can use it. `tmp` lives until the end of the test, so the directory is
  not deleted before the command runs. Passing `home` (a `&Path`) to `.env` is fine.
- **Formatting**: `rustfmt --check --edition 2024` passes on every reviewed file.
- **`app.rs`**: the moved `RELOAD` doc now sits on `fn reload`, and `sync_tiers_in_use` keeps
  its own doc. The new deferred-quit comment matches `run_deferred_quit` (app.rs:2654-2669):
  `quit_after_apply` shuts down directly, and `quit_after_reload` calls `request_quit()` again.
  Removing the phase ids from test comments does not affect any test.

Nothing changes runtime behaviour or breaks the Windows build. One rewritten comment is less
accurate than it should be.

## Warnings

### WR-01: `--quit` comment suggests the events are only opened, but `CreateEventW` creates them when they are missing

**File:** `src/overlay/mod.rs:46-50`
**Issue:** The new comment reads "It only opens the instance mutex, never creates it (the
`-ready` / `-quit` events are opened with CreateEventW)". `open_event` (instance.rs:302-310)
calls `CreateEventW`, which **creates** the named event when it does not exist. This happens
in `quit_running_named` (instance.rs:656-657) if the instance exits between `open_mutex`
succeeding and the event calls. The code handles that case: it checks
`ERROR_ALREADY_EXISTS`, returns `Unreachable`, and never creates the mutex. So the safety claim
("can never become the running instance") is still true. But "opened with CreateEventW" makes
it sound as if `--quit` never creates any kernel object, and that is exactly the race a
maintainer would need to know about. The comment was edited in this phase (40 IN-01) to be
more accurate.
**Fix:**
```rust
// running instance to quit and exit with the outcome's code. It never creates the instance
// mutex, so it can never become the running instance. The `-ready` / `-quit` events are
// opened through CreateEventW; if the instance vanished in between, that creates a fresh
// event, which is detected (no ERROR_ALREADY_EXISTS) and reported as Unreachable. It
// registers no hotkey, tray or window.
```

## Info

### IN-01: The "Optimise dependencies" comment is now separated from the section it describes

**File:** `Cargo.toml:65-69`
**Issue:** Moving `[build-dependencies]` up fixed the original problem (the comment used to
sit above `[build-dependencies]`). But the comment now ends with a blank line before
`[profile.dev.package."*"]`. Everywhere else in the file (the `[build-dependencies]` and
`[profile.release]` comments) the comment sits directly on its section, so this one reads as
a free-standing note.
**Fix:** Delete the blank line between `# Our own crate stays at opt-level 0 for debugging.`
and `[profile.dev.package."*"]`.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
