# Phase 46: Request Throttling & CLI User Agent - Context

**Gathered:** 2026-10-23
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous run 43-50; author answered every question up front in prose)

<domain>
## Phase Boundary

The overlay sends each kind of network request at most once per hour (Sync economy per league, the
league list, Check for updates), faking a refresh inside the hour; the CLI is not throttled but
identifies itself with its own user agent. Requirements THR-01..06.

</domain>

<decisions>
## Implementation Decisions

### Throttle rules (author)
- One hour per kind, from the last **successful** request; Sync is keyed per league. After an error
  the next click goes out.
- A click inside the hour does not reach the server and shows **exactly** the real outcome message
  from the last result (no "(cached)" hint). It does not advance Last sync.
- A fake refresh only happens when there is data to show: if `prices.json` is missing or holds
  another league, Sync goes out even inside the hour.
- An update check that found a newer release is never throttled (the downloaded installer may have
  been deleted on quit); only an "up to date" answer is.

### Persistence (author)
- State lives in a small separate file, `ff-state.json`, in the FerrisFilter folder (not
  `ff-settings.json`, which stays user preferences). It holds the last-success times (unix seconds)
  and the cached league list, so the league dropdown is not empty after a restart within the hour.
- Deleting `ff-state.json` lifts every wait — a supported but **undocumented** use case (do not
  mention it in README / UI).
- A time in the future counts as expired. A missing or unreadable state file means no throttle (and
  is not an error shown to the user).

### User agents (author)
- Overlay: `FerrisFilter/<ver> (github.com/Jonathas-Conceicao/ferris-filter; contact: ...)` as today.
- CLI (`ff-cli`, `ferrisfilter`): `FerrisFilter-CLI/<ver> (...)`, same repo and contact text.
- The CLI never reads or writes `ff-state.json`.

### Claude's Discretion
- The throttle module's shape (research: a pure `RequestLog` with an injected clock wrapping the
  three call sites in `app.rs`), how the user agent is passed (`ReqwestClient` parameter).

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/sync/client.rs` `USER_AGENT` (shared today by sync and `src/update.rs`), `user_agent_tests`.
- Network workers in `src/overlay/app.rs` (sync, league list in `leagues.rs`, update check in
  `update_job.rs`) take a `work` closure — a fake refresh can hand in a synthetic one.
- `src/settings.rs` atomic JSON write helpers; `src/locations.rs` for the FerrisFilter folder.

### Established Patterns
- Tests pin `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` (enforced by `tests/isolation_guard.rs`);
  no real network in tests (`UpdateHttp` seam, fake clients).

### Integration Points
- Economy tab (Sync, League refresh), About tab (Check for updates), status line outcomes.

</code_context>

<specifics>
## Specific Ideas

- GitHub allows 60 unauthenticated API requests per hour per IP; `src/update.rs` already reports
  the rate limit as its own outcome.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
