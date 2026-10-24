---
phase: 47-customizable-hotkey
fixed_at: 2026-10-23T00:00:00Z
review_path: .planning/phases/47-customizable-hotkey/47-REVIEW.md
iteration: 2
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 47: Code Review Fix Report

**Fixed at:** 2026-10-23
**Source review:** .planning/phases/47-customizable-hotkey/47-REVIEW.md
**Iteration:** 2

**Summary:**
- Findings in scope: 1
- Fixed: 1
- Skipped: 0

## Fixed Issues

### WR-01: After the WR-07 fix, a default chord that failed at start-up can no longer be retried without a restart

**Files modified:** `src/overlay/app.rs`, `src/overlay/hotkey.rs`
**Commit:** 4288d96
**Status:** fixed: requires human verification (state-handling logic)
**Applied fix:**
- `hotkey_base()` returns `None` again when no chord is live (it was
  `Some(shown)` after WR-07).
- `pick_hotkey` handles the "no live chord, saved value not stale" case
  on its own. If the user picks the shown chord, a pending edit is
  cleared (this keeps the WR-07 escape). With nothing pending, the shown
  chord becomes pending so that Apply retries the registration. The retry
  only happens when a registrar exists. With no hotkey service, picking
  the shown chord only clears.
- Reset is enabled in the same state (live chord absent, registrar
  present) while nothing is pending.
- New `HotkeyRuntime::can_register()` (registrar present).
- I updated `the_header_close_tooltip_and_base_follow_the_live_chord` for
  the inert runtime. With no service, picking the default never becomes
  pending, and Reset is greyed.
- New app-level test
  `a_default_that_failed_at_start_up_can_be_retried_by_capture_or_reset`.
  It uses a FakeRegistrar wrapper whose taken set is shared, and it
  covers this sequence:
  1. No saved key, and the default is taken at start-up.
  2. Reset is enabled. Pressing it makes the default pending.
  3. Capturing the default again clears the edit. A pending Ctrl+Alt+K
     is cleared the same way.
  4. Capturing the default with nothing pending makes it pending again.
  5. The other app frees the chord, and Apply runs. The log shows
     `register Ctrl+Shift+Z`, `live() == Some(DEFAULT)`, the failure line
     is gone and `hotkey` is saved.

  No real OS hotkey is registered.
- `cargo test --locked` passes in full (lib: 1787 passed). No new
  dependencies. `tests/fixtures` is unchanged.

---

_Fixed: 2026-10-23_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
