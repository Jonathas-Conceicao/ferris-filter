---
phase: 46-request-throttling-cli-user-agent
fixed_at: 2026-10-23T00:00:00Z
review_path: .planning/phases/46-request-throttling-cli-user-agent/46-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 46: Code Review Fix Report

**Fixed at:** 2026-10-23
**Source review:** .planning/phases/46-request-throttling-cli-user-agent/46-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (1 Critical, 3 Warning; Info out of scope)
- Fixed: 4
- Skipped: 0

`cargo test --locked` is green after all fixes (lib: 1729 passed). No new dependencies.
`src/overlay/app.rs` was not touched.

## Fixed Issues

### CR-01: Fake-sync gate reads a stale `EconomyInfo`

**Files modified:** `src/overlay/app/throttle_glue.rs`
**Commit:** 3d0b893
**Applied fix:** `start_sync_gated_with` calls `refresh_economy_info()` (re-reads
`prices.json`) before asking `sync_fake`, so a file removed or rewritten for another league
since the cached read makes the click go out. Glue tests now drive the gate through real
`prices.json` files (seed prices with the league set) instead of assigning `economy_info`
by hand. New test `gate_rereads_prices_json_at_click_time` covers both "rewritten for
League B" and "deleted".
**Status:** fixed: requires human verification (logic change to the throttle decision)

### WR-01: A failed real sync does not clear the earlier success

**Files modified:** `src/throttle.rs`, `src/overlay/app/throttle_glue.rs`
**Commit:** 1703993
**Applied fix:** Added `RequestLog::forget_sync(&str)` (same lower-cased key as
`record_sync`). `record_after_polls` now drops the league's entry when a pending real sync
finishes with a failure, and saves `ff-state.json` if an entry was removed. Tests:
`forget_sync_uses_the_same_key` (pure) and `failed_real_sync_forgets_an_older_success`
(glue: fresh entry, prices for another league, real sync fails, entry gone in memory and on
disk, the next click with matching prices still goes out).
**Status:** fixed: requires human verification (state-handling logic)

### WR-02: CLAUDE.md says the CLI's apply and sync find ff-state.json

**Files modified:** `CLAUDE.md`
**Commit:** 33afc05
**Applied fix:** In the `ferrisfilter apply` row the list now says "... `prices.json` (and
the overlay alone `ff-state.json`)", and the folder-creation sentence adds "or the overlay's
first successful league-list fetch or up-to-date update check (`ff-state.json`)".

### WR-03: The fake update path clears `pending_real.update` even when the job refused to start

**Files modified:** `src/overlay/app/throttle_glue.rs`
**Commit:** a0ac0e9
**Applied fix:** The fake branch clears the flag only when `self.update.start(...)` returns
true. New test `refused_fake_update_keeps_the_running_real_check_recorded`: a blocked real
check is running, a fresh log entry appears, the fake start is refused, the flag survives,
and the real `UpToDate` is recorded at the poll.
**Status:** fixed: requires human verification (state-handling logic)

---

_Fixed: 2026-10-23_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
