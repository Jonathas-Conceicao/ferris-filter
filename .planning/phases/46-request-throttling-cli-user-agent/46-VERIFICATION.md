---
phase: 46-request-throttling-cli-user-agent
verified: 2026-10-23T00:00:00Z
status: human_needed
score: 4/4 must-haves verified
human_verification:
  - test: "Run the overlay: Sync economy twice within an hour, refresh the league list, Check for updates twice; restart and repeat"
    expected: "Second click sends no request, shows the same message, Last sync does not advance; an error click goes out at once; a newer-release check is never throttled; deleting ff-state.json lifts the wait"
    why_human: "Needs running overlay and network (already listed in AUTONOMOUS-V072-DEFERRED.md)"
---

# Phase 46 Verification

**Goal:** The app stops hammering poe.ninja and GitHub, and the CLI identifies itself.

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|---|---|---|
| 1 | Inside an hour of a success (per league sync, league list, up-to-date check) a click sends nothing and replays the last outcome without advancing Last sync | VERIFIED | `src/throttle.rs` (`sync_fake`, `cached_leagues`, `cached_update`); `src/overlay/app/throttle_glue.rs` wraps the three starts with fake closures (`economy.start(move || Ok(name))`, `leagues.start(Ok(list))`, `update.start(result)`), wired in `app.rs` (sync click, both league starts, `start_update_check`). Fakes are never recorded (`pending_real`), so the hour is not extended. |
| 2 | After an error the next click goes out; a newer-release check is never throttled | VERIFIED | `record_after_polls` records only real successes and calls `forget_sync` on failure; `record_update` records only `CheckResult::UpToDate`, so `cached_update` is None otherwise. |
| 3 | Times and league list survive restart in a state file in the FerrisFilter folder; deleting it lifts the wait | VERIFIED | `ff-state.json` loaded in `OverlayApp::build` via `throttle::load(root)`, saved via `throttle::save`; missing/unreadable file loads as an empty log (no throttle). `.gitignore` has `/ff-state.json`. |
| 4 | CLI never throttled; CLI UA `FerrisFilter-CLI/<ver> (...)`, overlay keeps `FerrisFilter/<ver> (...)` | VERIFIED | `src/sync/client.rs` adds `CLI_USER_AGENT`; `ReqwestClient::new(user_agent)`; `sync/mod.rs` CLI paths (`run`, metadata) use `CLI_USER_AGENT`, overlay paths use `USER_AGENT`; `src/update.rs:841` keeps `USER_AGENT`. Throttle module is only referenced from the overlay glue, not the CLI. |

## Requirements
THR-01..THR-06 are all claimed by the plans and all satisfied by the evidence above. Bookkeeping notes (not blockers): `.planning/REQUIREMENTS.md` still shows THR-02 as `[ ]`, and the traceability table lists all six as `Pending`; update these at phase close.

## Author rule check
Deleting `ff-state.json` as a way to lift the wait is not documented in `CLAUDE.md`, `README.md` or `CHANGELOG.md` (the only `ff-state` text, in `CLAUDE.md`, says a missing file means no wait and nothing about deleting it). OK.

## Spot checks
`cargo test --locked`: all suites pass (1729 lib tests, 0 failures), including the `throttle_glue` tests, the CLI UA test and `isolation_guard`.

## Anti-patterns
No TBD/FIXME/XXX found in the phase's changed source files via review; none blocking.

## Human verification
End-to-end behaviour with a live overlay and network is deferred (see `AUTONOMOUS-V072-DEFERRED.md`).
