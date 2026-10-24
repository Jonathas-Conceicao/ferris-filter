---
phase: 46-request-throttling-cli-user-agent
plan: 01
subsystem: sync
tags: [throttle, user-agent, ff-state]
requires: []
provides:
  - "src/throttle.rs: RequestLog, is_fresh, load/save of ff-state.json"
  - "CLI_USER_AGENT and ReqwestClient::new(user_agent)"
affects: [46-02]
tech-stack:
  added: []
  patterns: ["pure clock-injected decision module", "source-scan guard test"]
key-files:
  created: [src/throttle.rs]
  modified: [src/lib.rs, src/sync/models.rs, src/sync/client.rs, src/sync/mod.rs, tests/isolation_guard.rs, .gitignore]
key-decisions:
  - "Update replay fields (version, released, app version) persisted so the up-to-date message replays identically after restart"
  - "League list capped at 200 entries"
requirements-completed: [THR-01, THR-03, THR-04, THR-05, THR-06]
duration: ~25min
completed: 2026-10-23
---

# Phase 46 Plan 01: Throttle core and CLI user agent Summary

Pure `RequestLog` (per-league sync, league list, up-to-date update check, one hour from last success, future time = expired, bad file = empty log) persisted as `ff-state.json`, plus a separate `FerrisFilter-CLI/<ver>` user agent for the CLI with a guard test keeping the CLI path free of throttle references.

## Commits
- a73f261: throttle module, EconomyLeague now Serialize
- f42eca1: CLI_USER_AGENT, `ReqwestClient::new(user_agent)`, guard test, `/ff-state.json` ignored

## Deviations from Plan
None. Judgement call: the guard test also checks that `cmd_sync`/`run` never use the overlay agent via `new(client::USER_AGENT)` / `new(USER_AGENT)`.

## Verification
`cargo test --locked` green (1714 lib tests plus integration suites); no fixture, Cargo.toml or Cargo.lock change.

## Known Stubs
None.

## Self-Check: PASSED
