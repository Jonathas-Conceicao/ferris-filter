---
phase: 47-customizable-hotkey
fixed_at: 2026-10-23T00:00:00Z
review_path: .planning/phases/47-customizable-hotkey/47-REVIEW.md
iteration: 3
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 47: Code Review Fix Report

**Fixed at:** 2026-10-23
**Source review:** .planning/phases/47-customizable-hotkey/47-REVIEW.md
**Iteration:** 3 (final)

**Summary:**
- Findings in scope this iteration: 1 (1 warning; Info out of scope)
- Fixed: 1
- Skipped: 0

At the end, `cargo test --locked` (full suite) passes: 1790 lib tests
plus all integration suites. `tests/fixtures` is unchanged, and no new
dependency was added. The new tests and the earlier escape/retry tests
use fake registrars only. None registers a real OS hotkey.

## All three iterations at a glance

| Iteration | Finding | Result | Commit |
|---|---|---|---|
| 1 | CR-01 OS / app-critical chords (Ctrl+V ...) bindable | fixed | aaead74 |
| 1 | CR-02 Ctrl+Shift+Delete captured as Ctrl+Shift+X | fixed | b240cac |
| 1 | WR-01 paste chords capturable only with text on clipboard | fixed | f823528 |
| 1 | WR-03 hotkey-only Apply still sends `/reloaditemfilter` | fixed | 9848e35 |
| 1 | WR-04 pending hotkey edit lost on styles/tiers reload | fixed | 99f7da7 |
| 1 | WR-06 Reload warning names the wrong live chord | fixed | 40f6023 |
| 1 | WR-07 hotkey edit cannot be dropped with no live chord | fixed | 735d78d |
| 1 | WR-05 Shift-only chords | skipped (author decision) | 7e2fe92 (docs) |
| 1 | WR-02 non-US punctuation keys | skipped (deferred, documented) | 7e2fe92 (docs) |
| 2 | WR-01 failed start-up default cannot be retried | fixed: requires human verification | 4288d96 |
| 3 | WR-01 stale-only-because-unregistered saved chord traps the edit | fixed: requires human verification | b9f96d9 |

Details of the earlier iterations:
- Iteration 1 is in `47-REVIEW-FIX.iter2.md`. `47-REVIEW-FIX.iter3.md`
  is an identical copy.
- Iteration 2 is in `47-REVIEW-FIX.iter3b.md`.

## Fixed Issues (iteration 3)

### WR-01: When the saved chord is "stale" only because nothing registered, a pending hotkey edit still cannot be dropped and blocks every Apply

**Files modified:** `src/overlay/app.rs`
**Commit:** b9f96d9
**Status:** fixed: requires human verification (state-handling logic)

**Applied fix:**
- **`saved_hotkey_stale()`:** the saved chord is now compared with
  `live().unwrap_or(shown())` instead of `live()`. With nothing
  registered, a saved chord counts as stale only when it differs from
  the chord the button shows. An invalid saved value is still stale.
  The registered-chord case is unchanged.
- **`pick_hotkey()`:** a new first branch. When no chord is live and
  the captured chord is the one already pending, the edit is cleared,
  whatever the saved value. After that come the WR-07 toggle and the
  retry branch from iteration 2, as before.

**Effect on the three states the review named:**
1. **Saved `Ctrl+Shift+Z`, default taken again.** This is no longer
   stale. It behaves like the iteration 2 case:
   - Capturing the default toggles it pending / clear.
   - Reset is live with nothing pending.
   - Apply registers the chord once it is free.
2. **Saved X with the default also taken.** This is still stale, so
   Reset stays live until the default is pending. Re-capturing the
   pending chord now drops it.
3. **No hotkey service, saved X.** This is no longer stale. Reset
   stages the default, and capturing the default or X clears it. With
   no registrar, capturing the shown chord never makes it pending.

**Tests added (fake registrars or an inert runtime only):**
- `a_saved_default_that_fails_again_can_be_dropped_or_retried` (state 1).
  It checks capture, re-capture, Reset, and that Apply registers
  `Ctrl+Shift+Z` after the other app lets go.
- `a_saved_chord_and_the_default_both_taken_never_trap_the_edit`
  (state 2).
- `no_hotkey_service_with_a_saved_chord_never_traps_the_edit` (state 3).

**Earlier tests re-checked, all still passing:**
- `a_default_that_failed_at_start_up_can_be_retried_by_capture_or_reset`
- `the_header_close_tooltip_and_base_follow_the_live_chord`
- `a_stale_saved_chord_keeps_reset_live_until_the_default_is_pending`
- `a_stale_saved_chord_can_be_reset_to_the_default_and_saved`
- `reset_makes_the_default_pending_or_clears_the_edit`
- `a_captured_chord_becomes_the_pending_hotkey`

**For human verification:** in state 2, capturing the default while
another chord is pending stages the default. It does not clear the
edit, because the saved X is still stale and saving the default
explicitly is a real edit there. A second capture of the default then
clears it.

---

_Fixed: 2026-10-23_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 3_
