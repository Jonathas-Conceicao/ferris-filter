---
phase: 41-check-for-updates
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/41-check-for-updates/41-REVIEW.md
iteration: 2
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 41: Code Review Fix Report

**Fixed at:** 2026-10-18
**Source review:** .planning/phases/41-check-for-updates/41-REVIEW.md
**Iteration:** 2

**Summary:**
- Findings in scope: 2 (WR-01, WR-02; the Info findings IN-01..IN-08 are out of scope)
- Fixed: 2
- Skipped: 0

As instructed, the work was done directly on `milestone/v0.7-initial-public-release-and-strictness-selector`
in the `milestone-closeout` worktree. No extra worktree or branch was created.

Verification:
- `cargo test --locked --lib update`: 55 passed.
- `cargo test --locked --lib overlay::`: 915 passed.
- rustfmt was run on the two touched files only.
- `cargo clippy --lib --tests` reports no warnings in `src/update.rs` or `src/overlay/app.rs`.

The tests use no network and only temp dirs. They never start an installer, and they never lock a
file for real (the open is a seam).

## Fixed Issues

### WR-01: A worker message that arrives while a sync is still running still hides that sync's outcome

**Files modified:** `src/overlay/app.rs`
**Commit:** 0819e88
**Status:** fixed: requires human verification (logic change in message precedence)
**Applied fix:** I applied both halves of the suggested fix:
- `receive_update_message` now also counts `SyncState::Running` as a reason to rank behind.
- A new `poll_economy()` replaces the inline `economy.poll()` in the frame. When a sync outcome
  lands while an update message is showing, it sets `update_behind`, so the newer sync result
  ranks first.

I also extracted `sync_outcome()`, which the frame and the tests both use. While the sync is still
running, the update message keeps showing, because nothing outranks it yet.

New tests:
- `a_sync_started_after_a_check_shows_its_outcome_over_the_check_message` runs the review's
  sequence: Check (the offline test stub), Sync (a closure blocked on a channel), the update
  message arrives, the sync is released with a failure, and the event is the sync failure.
- `a_sync_outcome_landing_after_an_update_message_ranks_first` covers a successful sync that
  finishes behind an update message that was already showing.

**Not done:** I did not add a per-message sequence counter. It would be the general fix, but it is
a bigger change, and IN-07 stays open.

### WR-02: Every open failure at launch is reported as "installer is no longer there", with no retry for a sharing violation

**Files modified:** `src/update.rs`
**Commit:** 44859c3
**Applied fix:** `launch_verified_with` now hands off to `launch_verified_inner`, which takes the
open function and the retry delay as seams. `open_with_retry` behaves as follows:
- It retries raw OS error 32 up to `OPEN_RETRIES` (3) times, 250 ms apart, the same way
  `spawn_installer` does.
- `NotFound` maps to `InstallerCheck::Gone`.
- Any other error maps to a new `LaunchError::Open(cause)`. Its error box reads: "Could not open
  the FerrisFilter X installer to check it (cause), so it was not started. It was saved to
  <path>."
- A read error during the hash maps to `Open` too.

**Decision:** an open error on a path that exists but is not a file still maps to `Gone`. Opening
a folder on Windows fails with access denied, and "no installer there" is the honest answer in
that case. This keeps the existing directory row in
`launch_refuses_a_changed_or_missing_installer_without_starting_it`.

New tests:
- `launch_retries_a_sharing_violation_then_starts_the_installer`: two error-32 results, then the
  real locked open, and the installer starts.
- `launch_reports_a_sharing_violation_that_outlasts_the_retries`: 1 + 3 opens, then `Open`.
- `launch_maps_only_a_missing_file_to_gone`: `NotFound` gives `Gone` after one open. Access denied
  on a file that is still there gives `Open` after one open, with no retry.
- The `Open` text was added to `launch_error_texts`.

As with `Spawn`, the work folder is kept after an `Open` error, because the error text points to
the saved file.

## Iteration 1 (record)

All 5 findings in scope were fixed. The full details are in git history and in `41-REVIEW-FIX.iter2.md`.
- **CR-01** (9d93376): an Apply & quit that stays open because of unused tier edits now cancels
  the install intent.
- **WR-01** (6c07a47): added the `update_behind` flag, so a late worker message ranks below newer
  Apply, CONFIG, sync or list messages. This iteration completed it for a sync that is still
  running.
- **WR-02** (d723e35): the installer is hashed again through a read-only, non-deletable handle,
  which stays open until the process has started. This iteration added the retry and the error
  mapping.
- **WR-03** (8339209): an RC build is offered its final release, and the up-to-date text names the
  running pre-release.
- **WR-04** (8e65f93): the work folder is created, published and cancelled under one lock, and the
  download stops when shutdown begins.

---

_Fixed: 2026-10-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
