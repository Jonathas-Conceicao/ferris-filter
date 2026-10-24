---
phase: 46-request-throttling-cli-user-agent
reviewed: 2026-10-23T00:00:00Z
depth: standard
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
  critical: 1
  warning: 3
  info: 5
  total: 9
status: issues_found
---

# Phase 46: Code Review Report

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

I reviewed the diff from `ccdb620` to HEAD: the pure `RequestLog` throttle, the overlay glue,
the `UpdateJob::take_done` seam, the split between the overlay and CLI user agents, and the
CLI isolation guard.

Most of the binding decisions are met:
- Only real successes are recorded. The record is made after polling, never when the button
  is clicked, and never for a fake.
- Sync is keyed per league (lower-cased).
- A time in the future counts as expired.
- Only an `UpToDate` update answer is recorded, so a newer release is never throttled.
- A fake replay uses the same `success_message` / `message_for` text and does not touch
  `prices.json`, so Last sync does not move.
- A missing or corrupt `ff-state.json` loads as an empty log.
- The write is atomic (temp file then persist).
- The CLI builds both of its clients with `CLI_USER_AGENT` and never reaches the throttle.
- No user-facing text tells the user to delete `ff-state.json`.
- The tests use temp homes and closures, never the network.

**The main defect:** the "Sync goes out when `prices.json` is missing or holds another
league" rule is checked against a cached `EconomyInfo`. That cache is re-read only at
start-up, when the user switches to the Economy tab, and after a sync. As a result a fake
"prices refreshed from X" can be shown while the file on disk is missing or holds a
different league.

## Critical Issues

### CR-01: Fake-sync gate reads a stale `EconomyInfo`, so a fake can run when prices.json is missing or holds another league

**File:** `src/overlay/app/throttle_glue.rs:49-54` (cache is filled only by `src/overlay/app.rs:1618`, `:1676`, `:2277`, `:4094`)

**Issue:** `start_sync_gated_with` decides whether to fake from `self.economy_info.league` and
`self.economy_info.live`. These come from `EconomyInfo::read`, which runs only:
- at start-up,
- when the user switches to the Economy tab,
- after a sync finishes (`prices_changed`).

Nothing re-reads `prices.json` when Sync is clicked.

Realistic sequence, using the documented CLI and overlay workflow:
1. The user syncs league A in the overlay.
2. While the Economy tab stays open, the user runs `ff-cli sync --league B`, or deletes
   `prices.json`.
3. Within the hour, the user clicks Sync again for A.

Result:
- The gate still sees `league == A`, `live == true`.
- It replays "Economy prices refreshed from A. Press Apply…" and sends nothing.
- `poll_economy` → `prices_changed` then re-reads the file and shows B, or Never.
- The status line has told the user A's prices were refreshed. The next Apply compiles B's
  prices, or the fallback pair.

This breaks the binding decision: "if `prices.json` is missing or holds another league, Sync
goes out even inside the hour."

**Fix:** Re-read the file at click time, before deciding. It is one small file read on a user
click, not per frame:
```rust
pub(super) fn start_sync_gated_with<F>(&mut self, league: String, now: u64, real: F)
where F: FnOnce() -> Result<String, String> + Send + 'static,
{
    if self.economy.is_running() {
        return;
    }
    self.refresh_economy_info(); // the gate must see prices.json as it is now
    if let Some(name) = self.request_log.sync_fake(
        &league,
        self.economy_info.league.as_deref(),
        self.economy_info.live,
        now,
    ) { ... }
```
Add a glue test: write a fresh log and a `prices.json` holding league A, build the app, then
rewrite `prices.json` to league B (or delete it), and assert `pending_real.sync.is_some()`
after `start_sync_gated_with(A, …)`.

## Warnings

### WR-01: A failed real sync does not clear the earlier success, so the next click can still be faked

**File:** `src/overlay/app/throttle_glue.rs:124-138`, `src/throttle.rs:82-95`

**Issue:** The decision says: "After an error the next click goes out." When a real sync for
league L fails, `record_after_polls` drops the pending record but keeps the earlier fresh
`sync[L]` entry.

A real sync can go out inside the hour (because `prices.json` held another league, or was
unusable) and fail. If the file then comes back to holding L, the next click replays the
older success instead of trying again. Example: `ff-cli sync --league L`, or CR-01's stale
cache. The user's most recent overlay result was a failure, yet the overlay reports success.

**Fix:** On a failed real sync, drop the entry for that league:
```rust
economy::SyncState::Finished(outcome) => {
    let failed = outcome.is_failure();
    if let Some(p) = self.pending_real.sync.take() {
        if failed {
            changed |= self.request_log.sync.remove(&p.requested.to_lowercase()).is_some();
        } else { /* record as today */ }
    }
}
```
Add `RequestLog::forget_sync(&str)` to keep the key normalisation in one place.

### WR-02: CLAUDE.md says the CLI's apply and sync find ff-state.json, and the folder-creation list is now incomplete

**File:** `CLAUDE.md:41` (the `ferrisfilter apply` row)

**Issue:** Two parts of this row are now wrong:
- **Who reads the file.** The edit inserted `ff-state.json` into "apply, sync and the overlay
  find `styles.json` / `tiers.json` / `ff-settings.json` / `ff-state.json` / `prices.json` in
  `FERRISFILTER_HOME`…". That says the CLI's `apply` and `sync` find `ff-state.json`, which
  contradicts the new `ff-state.json` row ("The CLI … never reads or writes it") and the
  binding decision.
- **What creates the folder.** The same row still says "The folder is created by the first
  Apply that writes styles/tiers, the first settings save … or the first Sync". But
  `throttle::save` calls `create_dir_all(root)`. So a successful league-list fetch or an
  up-to-date check now creates the folder too.

CLAUDE.md asks that this reference be kept accurate.

**Fix:**
- Remove `ff-state.json` from that list, or add "(`ff-state.json`: overlay only)".
- Extend the creation sentence: "…, the first Sync, or the first successful league-list fetch
  or update check (`ff-state.json`)".

### WR-03: The fake update path clears `pending_real.update` even when the job refused to start

**File:** `src/overlay/app/throttle_glue.rs:102-108`

**Issue:** On the fake branch, `self.pending_real.update = false` runs before, and regardless
of, the result of `self.update.start(...)`.

If `start` refuses because a job is already in flight, that job's pending flag is cleared. If
that job is a real check, its eventual `UpToDate` is never recorded. The real branch does this
correctly (it sets the flag only when `started`). This can happen when the wall clock steps
backwards and makes an older entry fresh again during a real check, or in any future path that
calls this while a check runs.

**Fix:**
```rust
if let Some(result) = self.request_log.cached_update(env!("CARGO_PKG_VERSION"), now) {
    let started = self.update.start(move |_| result);
    if started {
        self.pending_real.update = false;
    }
    return started;
}
```

## Info

### IN-01: `missing_root_or_garbage_file_means_no_throttle` does not test what its name says

**File:** `src/overlay/app/throttle_glue.rs:455-462`

**Issue:**
- The garbage `ff-state.json` is written into one temp dir, but the app is built by
  `app_with(None)` on a different temp dir. So the app never loads the garbage file.
- The `Err(root)` ("missing root") case is never built.

The test only repeats `throttle::load` coverage that already exists.

**Fix:** Let `app_with` take raw file text, or write the garbage before `OverlayApp::build` in
the same home. Add a case that builds with `Err(..)` for root, and check that a sync then goes
out for real and that `record_after_polls` writes nothing.

### IN-02: `throttle::save` duplicates the settings write pattern without a retry on a locked file

**File:** `src/throttle.rs:152-161`

**Issue:** This copies the temp-file and `persist` code from `src/settings.rs:386-389` and
`:443-446`. `src/safe_write.rs` already retries `persist` on Windows sharing violations
(antivirus, OneDrive). Without that retry, a briefly locked `ff-state.json` silently loses the
record, which only means extra requests later.

**Fix:** Share one helper, ideally the retrying `safe_write::persist_with_retry`.

### IN-03: A sync that returns an empty league name replays a different message

**File:** `src/throttle.rs:82-87`

**Issue:** `record_sync` stores the requested name when the real sync returned `""`. Shown
side by side:

| Case | Message |
|---|---|
| Real sync, empty name | "Economy prices refreshed. Press Apply…" (`success_message("")`) |
| Fake replay | "Economy prices refreshed from <requested>. Press Apply…" |

That breaks "exactly the real outcome message". This is unlikely today, because
`run_sync_prices` returns the resolved league.

**Fix:** Store the returned name as is, and fall back to the requested name only for the
`prices_league` comparison in `sync_fake`.

### IN-04: Two instances share ff-state.json, and the last one to write wins

**File:** `src/overlay/app.rs:1536-1539`, `src/overlay/app/throttle_glue.rs:160-167`

**Issue:** The log is loaded once in `build`, and the whole in-memory copy is written on every
record. A debug build and a release `ff` can run side by side (CLAUDE.md). When they do, each
one overwrites the other's entries. The result is extra requests, not wrong behaviour.

**Fix:** Before saving, re-`load` the file and merge, keeping the newer `at` for each key.
Alternatively, document that this is accepted.

### IN-05: A faked update check skips sweeping old update folders

**File:** `src/overlay/app/throttle_glue.rs:102-108`

**Issue:** CLAUDE.md says update folders older than 24 h "are swept at the next check". A
replayed check never calls `check_for_update_blocking`, so no sweep runs. The sweep is
harmless to defer, but the documented behaviour now applies only to real checks.

**Fix:** Run the sweep on the fake path too. Otherwise, change the doc to say "at the next
real check".

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
