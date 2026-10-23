---
phase: 46-request-throttling-cli-user-agent
reviewed: 2026-10-23T00:00:00Z
depth: standard
iteration: 2
files_reviewed: 11
files_reviewed_list:
  - src/throttle.rs
  - src/overlay/app/throttle_glue.rs
  - src/overlay/app.rs
  - src/overlay/update_job.rs
  - src/sync/client.rs
  - src/sync/mod.rs
  - src/sync/models.rs
  - src/lib.rs
  - tests/isolation_guard.rs
  - CLAUDE.md
  - .gitignore
findings:
  critical: 0
  warning: 0
  info: 7
  total: 7
status: clean
---

# Phase 46: Code Review Report (re-review)

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 11
**Status:** clean

## Summary

This re-review covers the diff `ccdb620..HEAD`, focused on the four fix commits (3d0b893,
1703993, a0ac0e9, 33afc05). For `src/overlay/app.rs` and `CLAUDE.md` only the phase 46 hunks
were reviewed. `cargo test --locked --lib throttle` passes (23 tests, including the three new
ones).

Verdict on the earlier Critical and Warning findings:

- **CR-01 (stale `EconomyInfo` gate): fixed correctly.** `start_sync_gated_with` now calls
  `refresh_economy_info()` after the `is_running` guard and before `sync_fake`. The re-read
  only replaces `economy_info` with what is on disk now, so the Last sync, Priced items and
  problem displays get more accurate, not less. In production the sync path is reached only
  with `root` = `Ok`, so the `Err` branch of the refresh never wipes the info at click time.
  `gate_rereads_prices_json_at_click_time` covers both cases ("rewritten for another league"
  and "deleted"). The existing tests now go through real `prices.json` files, so they
  exercise the same read.
- **WR-01 (failed real sync keeps the earlier success): fixed correctly.**
  - `forget_sync` uses the same lower-cased key as `record_sync` and `sync_fake`.
  - `is_failure()` covers both `Err` and a worker that disconnected (a panic).
  - `changed` is set only when an entry was really removed, so a failure with no earlier
    entry still writes no file (`failed_sync_records_nothing` holds).
  - Ordering: polls run before `record_after_polls` in `update()` (app.rs 3552-3562), and
    clicks are handled later in the same frame. So a `clear_message` cannot turn `Finished`
    into `Idle` before the failure is seen.
- **WR-02 (CLAUDE.md folder row): fixed.** The list now says "(and the overlay alone
  `ff-state.json`)". The creation sentence names the overlay's first successful league-list
  fetch or up-to-date check. Both match `throttle::save` (`create_dir_all`) and the binding
  decision that the CLI never touches the file.
- **WR-03 (fake update path clears the real flag): fixed correctly.** The flag is cleared
  only when the fake start succeeds. `UpdateJob::start` refuses unless the state is `Idle`,
  so a successful fake start means no real check is in flight whose flag could be lost.
  `refused_fake_update_keeps_the_running_real_check_recorded` covers this.

**New issues from the fixes:** none at Critical or Warning level. I also traced:
- the league-list recording against `LeagueList::poll`: a failed refresh becomes
  `RefreshFailed`, never `Loaded`, so an old list is never re-recorded as a fresh success;
- the `last_done` lifecycle in `UpdateJob`: it is taken every frame, so a fake's result is
  drained without being recorded.

The remaining items are Info: five carried over unchanged from the previous review, plus two
small new ones.

## Info

### IN-01: `missing_root_or_garbage_file_means_no_throttle` does not test what its name says (carried over)

**File:** `src/overlay/app/throttle_glue.rs:526-533`

**Issue:**
- The garbage `ff-state.json` is written into one temp dir, but `app_with(None)` builds the
  app on a different one, so the app never loads the garbage file.
- The `Err(root)` case is never built.

**Fix:** Write the garbage file into the same home before `OverlayApp::build`, and add a
case built with `Err(..)` for root that checks a sync goes out for real and nothing is saved.

### IN-02: `throttle::save` duplicates the atomic-write pattern without the Windows persist retry (carried over)

**File:** `src/throttle.rs:158-167`

**Issue:** This copies the temp-file and `persist` code instead of using
`safe_write`'s retry on a sharing violation. A briefly locked file (antivirus, OneDrive)
silently loses the record. The only effect is extra requests later.

**Fix:** Route it through the shared retrying helper.

### IN-03: A sync that returns an empty league name replays a different message (carried over)

**File:** `src/throttle.rs:82-95`

**Issue:** `record_sync` stores the requested name when the real sync returned `""`. The
real outcome was then `success_message("")` ("Economy prices refreshed."), but the replay says
"…refreshed from <requested>." That breaks "exactly the real outcome message". It is
unlikely, because `run_sync_prices` returns the resolved league. The test
`sync_spelling_and_key_case` pins the current behaviour.

**Fix:** Store the returned name as is, and use the requested name only for the
`prices_league` comparison.

### IN-04: Two overlay instances overwrite each other's ff-state.json entries (carried over)

**File:** `src/overlay/app.rs:1536-1540`, `src/overlay/app/throttle_glue.rs:172-180`

**Issue:** The log is loaded once in `build` and written whole on each record. A debug build
and a release `ff` can run side by side, and then the last one to write wins. The result is
extra requests only.

**Fix:** Re-load and merge before saving, keeping the newer time for each key, or accept this
and document it.

### IN-05: A replayed update check skips the 24 h update-folder sweep (carried over)

**File:** `src/overlay/app/throttle_glue.rs:105-116`; `CLAUDE.md` (Check for updates note)

**Issue:** CLAUDE.md says old update folders "are swept at the next check". A replayed check
never calls `check_for_update_blocking`, so the sweep only runs on a real check.

**Fix:** Run the sweep on the fake path too, or say "at the next real check" in the doc.

### IN-06: CLAUDE.md does not mention that a failed sync clears the hour for that league

**File:** `CLAUDE.md` (Economy sync note, "a failure never starts the hour")

**Issue:** Since WR-01, a failed real sync also removes an earlier success for that league,
so the next click goes out even inside the hour. The note only says a failure never
*starts* the hour. The text is not wrong, but it is incomplete next to the code.

**Fix:** Change it to "a failure never starts the hour and ends the current one for that
league".

### IN-07: `real_path_when_other_league_not_live_or_expired` does not cover the "not live" case

**File:** `src/overlay/app/throttle_glue.rs:350-366`

**Issue:** The test covers these cases:
- another league,
- a missing `prices.json`,
- a file holding a different league,
- an expired entry.

It never covers an existing but unusable `prices.json` (an empty item list, or malformed),
where `live == false` while a file is present. That is the one input of `sync_fake` that only
the pure test reaches. Since CR-01, the glue reads the real file, so this case can now be
tested end to end.

**Fix:** Add a case that writes the seed with `"items": []` (or invalid JSON) for the same
league and asserts `pending_real.sync.is_some()`.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
