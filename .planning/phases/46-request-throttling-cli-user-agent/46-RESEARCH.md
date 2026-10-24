# Phase 46: Request Throttling & CLI User Agent - Research

**Researched:** 2026-10-23
**Domain:** Rust (egui overlay), pure throttle state + small JSON state file, reqwest user agent
**Confidence:** HIGH (all findings come from reading this repo's code; no external libraries involved)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- One hour per kind, from the last **successful** request; Sync is keyed per league. After an error the next click goes out.
- A click inside the hour does not reach the server and shows **exactly** the real outcome message from the last result (no "(cached)" hint). It does not advance Last sync.
- A fake refresh only happens when there is data to show: if `prices.json` is missing or holds another league, Sync goes out even inside the hour.
- An update check that found a newer release is never throttled (the downloaded installer may have been deleted on quit); only an "up to date" answer is.
- State lives in a small separate file, `ff-state.json`, in the FerrisFilter folder (not `ff-settings.json`). It holds the last-success times (unix seconds) and the cached league list, so the league dropdown is not empty after a restart within the hour.
- Deleting `ff-state.json` lifts every wait: supported but **undocumented** (do not mention it in README / UI).
- A time in the future counts as expired. A missing or unreadable state file means no throttle (not an error shown to the user).
- Overlay UA: `FerrisFilter/<ver> (github.com/Jonathas-Conceicao/ferris-filter; contact: ...)` as today. CLI (`ff-cli`, `ferrisfilter`): `FerrisFilter-CLI/<ver> (...)`, same repo and contact text. The CLI never reads or writes `ff-state.json`.

### Claude's Discretion
- The throttle module's shape (pure `RequestLog` with injected clock wrapping the three call sites in `app.rs`), how the UA is passed (`ReqwestClient` parameter).

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| THR-01 | One request per kind per hour (sync per league, league list, update check) | `RequestLog::is_fresh` + three gate points in app.rs |
| THR-02 | Click inside the hour = same outcome, no server, no Last sync advance | Fake `work` closures through existing `start()` seams; Last sync is prices.json mtime, which a fake never touches |
| THR-03 | Only success starts the hour | Record at poll time only on Ok / UpToDate, never on Err/Failed |
| THR-04 | Times + league list persist in ff-state.json | New `src/throttle.rs` load/save, atomic temp+persist like settings.rs |
| THR-05 | Newer release is never throttled | Record only `CheckResult::UpToDate`; never record `Ready` |
| THR-06 | CLI unthrottled, own UA | `ReqwestClient::new(user_agent)`; CLI path (`cmd_sync` -> `run`) never touches throttle |
</phase_requirements>

## Summary

All three network kinds already run through a "`work` closure on a worker thread, result polled once per frame" seam: `EconomySync::start(F: FnOnce()->Result<String,String>)` (`src/overlay/economy.rs`), `LeagueList::start(F: FnOnce()->Result<Vec<EconomyLeague>,String>)` (`src/overlay/leagues.rs`), `UpdateJob::start(F: FnOnce(UpdateProgress)->CheckResult)` (`src/overlay/update_job.rs`). A fake refresh is therefore just a different closure returning the cached result immediately; the existing poll/message code then produces the byte-identical message with no new rendering path. [VERIFIED: codebase read]

"Last sync" is `EconomyInfo.mtime_secs` = mtime of `<root>/prices.json` (`app.rs` ~1008-1050), re-read by `prices_changed()` after every poll. A fake sync writes nothing, so Last sync cannot advance. [VERIFIED: codebase]

UA: `USER_AGENT` const in `src/sync/client.rs` is used by `ReqwestClient::new()` (sync, leagues, CLI) and `ReqwestUpdateHttp::new()` (`src/update.rs:841`, overlay only). The CLI reaches the network only through `sync::cmd_sync -> run()` (two `ReqwestClient::new()` calls at `src/sync/mod.rs:55,67`); the overlay only through `sync_economy_blocking` (:81... `ReqwestClient::new()` ~L55) and `fetch_leagues_blocking`. No other `ReqwestClient`/`USER_AGENT` users exist outside those files and tests. [VERIFIED: grep]

**Primary recommendation:** Add a pure, framework-free `src/throttle.rs` (`RequestLog`, serde, all methods take `now: u64`), keep `app.rs` edits to three gate points plus three record points, and give `ReqwestClient::new` a `user_agent: &'static str` parameter with `USER_AGENT` (overlay) and a new `CLI_USER_AGENT`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary | Rationale |
|------------|-------------|-----------|-----------|
| Throttle decision + state (de)serialisation | lib `src/throttle.rs` (pure) | — | Testable with injected `now`, no egui, no network |
| Gate/record wiring | overlay `app.rs` | `update_job.rs` | Only the overlay throttles; owns click handlers and poll sites |
| ff-state.json location | `locations::AppDirs.root` | — | Same folder as ff-settings.json |
| UA selection | `sync/client.rs` + callers | `sync/mod.rs` | Caller decides: overlay entry points vs `cmd_sync` |

## Standard Stack

No new dependencies (hard constraint). Already available: `serde` 1 (derive), `serde_json` 1, `tempfile` 3 (non-dev dependency, used by `settings::save`), `std::time::{SystemTime, UNIX_EPOCH}` (already used in `app.rs` for `now_secs`). No chrono/time crates; none needed (unix seconds are `u64`). [VERIFIED: Cargo.toml lines 30-49, app.rs]

`EconomyLeague` (`src/sync/models.rs:12`) currently derives only `Deserialize, Debug, Clone, PartialEq`; add `serde::Serialize` (additive, fields are plain `name,url,hardcore,indexed`, no renames) so the list can be cached in ff-state.json. Package Legitimacy Audit: not applicable, no packages installed.

## Architecture Patterns

### Flow
```
click (Sync / refresh / first Economy visit / Check)
   -> app gate: RequestLog.decide_*(key, now)  [+ data-present check]
        |-- Fresh  -> start(fake closure returning cached result) -> normal poll -> same message
        `-- Go     -> start(real closure) -> poll
                        Ok/UpToDate -> RequestLog.record_*(now) -> save ff-state.json (best effort)
                        Err/Ready/Failed -> record nothing
CLI: cmd_sync -> run() -> ReqwestClient::new(CLI_USER_AGENT)   (no throttle import)
```

### Recommended module: `src/throttle.rs` (add `pub mod throttle;` in lib.rs)
```rust
pub const WINDOW_SECS: u64 = 3600;
pub const FILE_NAME: &str = "ff-state.json";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RequestLog {
    pub sync: BTreeMap<String, u64>,        // key: league name lower-cased
    pub leagues_at: Option<u64>,
    pub leagues: Vec<EconomyLeague>,        // cached list
    pub update_at: Option<u64>,
    pub update_version: Option<String>,     // see Open Question 1
    pub update_released: Option<i64>,
}

/// Pure: fresh iff last <= now and now - last < 3600 (future time = expired).
pub fn is_fresh(last: Option<u64>, now: u64) -> bool {
    last.is_some_and(|t| t <= now && now - t < WINDOW_SECS)
}
```
Methods: `sync_fresh(&self, league, now)`, `record_sync(&mut self, league, now)`, `leagues_fresh(now)` (also requires non-empty cached list), `record_leagues(list, now)`, `update_fresh(now)`, `record_update(version, released, now)`. Free fns `load(root) -> RequestLog` (any read/parse error -> `Default`, silent) and `save(root, &RequestLog) -> io::Result<()>` copying `settings::save` (create_dir_all, `NamedTempFile::new_in(root)`, write, `sync_all`, `persist`). Use `#[serde(default)]` so unknown/missing keys are tolerated; a wrongly typed value fails the whole parse -> Default (no throttle), which is the decided behaviour. Prune `sync` entries older than the window on save to keep the file small.

### app.rs injection points (keep contained)
Add one field `request_log: crate::throttle::RequestLog` (loaded once at construction from `root` when Ok; Default otherwise; never in tests via `resolve_root` which errors under `cfg(test)`), plus a small `pending_real: PendingReal { sync: Option<String>, leagues: bool }` flag set when a real request starts. Prefer putting helper methods (`gate_sync`, `gate_leagues`, `gate_update`, `record_*`) in a new `impl App` block in a new file (e.g. `src/overlay/throttle_glue.rs`) so app.rs only gains call lines. Exact sites in `src/overlay/app.rs`:

1. **Sync** (~L4058-4070): inside `if economy_clicks.sync && ... let Some(league) ... let Ok(root)`. Gate: fake iff `request_log.sync_fresh(&league, now)` AND `economy_info.league` equals `league` (case-insensitive) AND `economy_info.live` (data present). Fake closure: `move || Ok(league)` (the Ok payload is the league name; `success_message(&league)` builds the message, identical text). Real closure unchanged; set `pending_real.sync = Some(league)`. Record in `poll_economy()` (~L2255): when state is `Finished(Ok)` and `pending_real.sync.take()` is Some, `record_sync`. On `Failed` just clear the pending flag. Caveat: `run_sync_prices` returns the league it actually fetched (may differ in spelling from requested); key on the requested league (what the click compares) and the fake message uses the requested name; accept, or return the real name through the success message by storing it in `request_log` (not needed).
2. **League list** (L3877 first Economy visit, L4145 refresh button): both call `self.leagues.start(fetch_leagues)`. Replace with `self.start_leagues()` doing: if `leagues_fresh(now)` -> `start(move || Ok(cached))`, else `start(fetch_leagues)` and set `pending_real.leagues = true`. Record in the `if self.leagues.poll()` block (L3543): when `state()` is `Loaded(list)` and pending_real.leagues was set, `record_leagues(list.clone(), now)`; on `Failed`/`RefreshFailed` clear flag, record nothing. Hydrate on startup: nothing to do, because the first-visit `start` already serves the cache from a fresh log (so the dropdown is populated after a restart within the hour). Note `NotStarted` on first visit with Loading state flashes for one frame, acceptable.
3. **Update check** (`start_update_check`, L2227): if `update_fresh(now)` and `update_version == running version` build the fake `CheckResult::UpToDate{version, released}` and `self.update.start(move |_| result)`; else the existing real closure. Record needs the structured result: `UpdateJob::poll` only returns `(String, UpdateKind)`. Smallest change: in `update_job.rs` `poll`'s `Done` arm store `self.last_done = Some(result.clone())` and add `pub fn take_done(&mut self) -> Option<CheckResult>`; app.rs calls it right after `self.update.poll(...)` (L3546) and records only `UpToDate` (never `Ready`, THR-05). (`UpdateKind::Ok` happens to equal UpToDate only, but matching on `CheckResult` is safer than on the kind.) `CheckResult` already derives `Clone, PartialEq`. Fake results must not be re-recorded (would restart the hour): pass a `fake` flag or only record when the check was real.
4. All saves are best effort: `let _ = throttle::save(root, &log)`; failure is silent (consistent with "not an error shown to the user"). Never write in screenshot mode (`is_shot()`) or tests.

### Pure-throttle testability
All decisions take `now: u64`; tests construct `RequestLog` and pass literal seconds (boundary 3599/3600/3601, future time, per-league keys, empty cached list, failure not recorded by simply not calling record). Disk tests use `tempfile::tempdir()` with `locations::AppDirs::at(tmp)` style roots (no env resolution); `from_env()` panics under cfg(test) unless `FERRISFILTER_HOME` is set, so never call it. The app-level wiring can be tested with the existing `settle`/`poll` loops using fake clocks only if the gate helpers take `now` as a parameter (do this; production passes `SystemTime::now()` at the call site).

### User agent
```rust
pub const USER_AGENT: &str = concat!("FerrisFilter/", env!("CARGO_PKG_VERSION"), " (github.com/Jonathas-Conceicao/ferris-filter; contact: jonathasaoc@gmail.com)");
pub const CLI_USER_AGENT: &str = concat!("FerrisFilter-CLI/", env!("CARGO_PKG_VERSION"), " (github.com/Jonathas-Conceicao/ferris-filter; contact: jonathasaoc@gmail.com)");
impl ReqwestClient { pub fn new(user_agent: &'static str) -> miette::Result<Self> { ... .user_agent(user_agent) ... } }
```
Callers: `sync_economy_blocking` and `fetch_leagues_blocking` pass `USER_AGENT`; both arms of `run()` (CLI `sync` prices and metadata) pass `CLI_USER_AGENT`. Update the doc comment on `USER_AGENT` (it says "the one HTTP user agent"). Extend `user_agent_tests`: CLI starts with `FerrisFilter-CLI/<ver>`, contains the repo, no "myfilter", and `!USER_AGENT.starts_with("FerrisFilter-CLI")`. `ReqwestUpdateHttp` stays on `USER_AGENT` (overlay-only). Beware `starts_with("FerrisFilter/")` for the overlay UA still holds.

### Anti-Patterns
- Throttling inside `sync_economy_blocking`/`cmd_sync` (the CLI would be throttled, and the lib fn has no state).
- Recording at click time (an error would then start the hour; THR-03).
- Recording from a fake request (would extend the hour forever).
- A "(cached)" suffix or touching `prices.json` (breaks THR-02 / Last sync).
- Writing ff-state.json into `ff-settings.json` or via `save_keys` (different file by decision).

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| Atomic JSON write | copy the `NamedTempFile::new_in(root)` + `persist` pattern of `settings::save` |
| Message text for replay | existing `economy::success_message`, `update::message_for`, league-list state; never store message strings |
| Time | `SystemTime::now().duration_since(UNIX_EPOCH)` (pattern already in app.rs ~L3880) |

## Runtime State Inventory
Not a rename phase. New persisted artifact: `ff-state.json` in the FerrisFilter folder. Add `/ff-state.json` to `.gitignore` next to `/ff-settings.json` (line 32). Add a CLAUDE.md row/paragraph describing it (the overlay notes in CLAUDE.md are the maintained reference); do NOT mention deleting it as a way to lift the wait, and do not touch README/CHANGELOG (author-owned; Release Review drafts).

## Common Pitfalls

1. **Fake sync inside the hour but prices.json belongs to another league / missing** -> must go out. Gate on `economy_info.league`/`live`, not only the log. Also the user may have switched Live/Fallback or deleted prices.json after syncing.
2. **Update version drift:** after the app is upgraded within the hour, a cached "up to date" for the old version would be wrong. Store/compare the running version (`update::running_version`/`CARGO_PKG_VERSION`); mismatch = not fresh.
3. **Fake update must not reuse a `Ready`** (never recorded; THR-05) and must not touch `work_dir`; `UpdateJob::start` resets `slot.dir = None` already, fine.
4. **Double record:** a fake poll goes through the same poll code; guard with `pending_real` flags.
5. **League list cached as empty:** `list_leagues` already errors on empty; also require `!leagues.is_empty()` when fresh.
6. **Clock skew:** `t > now` -> expired (locked decision), implemented in `is_fresh`; no underflow because the `t <= now` check precedes the subtraction.
7. **Update fake when worker closure runs on a thread:** `message_for` needs `utc_offset_minutes` at poll time (unchanged, so the release date text matches the real one only if `released` is persisted; see Open Question 1).
8. **Tests:** `isolation_guard` requires any spawned `cargo_bin(` test to pin `FERRISFILTER_HOME`/`FERRISFILTER_GAME_DIR`; unit tests must not resolve the real folder. Do not add real-network tests. The `#[cfg(test)]` twins `fetch_leagues`, `check_for_update_blocking`, `resolve_root` already prevent it.
10. **Rate-limited / offline update results** are `Failed` -> not recorded -> next click goes out (consistent with THR-03).

## Code Examples

Fake-closure gate (sync), sketch:
```rust
let fake = self.request_log.sync_fresh(&league, now)
    && self.economy_info.live
    && self.economy_info.league.as_deref().is_some_and(|l| l.eq_ignore_ascii_case(&league));
if fake {
    let l = league.clone();
    self.economy.start(move || Ok(l));
} else {
    self.pending_real_sync = Some(league.clone());
    self.economy.start(move || crate::sync::sync_economy_blocking(&base_dir, &league).map_err(|e| e.to_string()));
}
```

## State of the Art
Not applicable (no external tech). GitHub unauthenticated limit is 60 req/h/IP (CONTEXT.md); the hourly throttle keeps the update check far below it.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Persisting `update_version` + `update_released` in ff-state.json is acceptable (CONTEXT lists only times and league list) | Open Questions | If author wants only times, a post-restart fake "up to date" message cannot be byte-identical (release date unknown) |

## Open Questions (RESOLVED)

1. **Replaying the "up to date" message after a restart.** Message is `Up to date (<ver>, released <date>)`; `released` comes from GitHub and is not derivable offline. Recommendation: persist `update_version` and `update_released` (tiny, same file) and treat a version mismatch as not fresh. Without it, the only exact option is to not fake after a restart (violates "survive a restart" for updates). Planner should include these two fields and note it in the plan for author visibility.
   RESOLVED: adopted in plan 46-01 (Claude's discretion): `RequestLog` persists `update_app_version`, `update_version` and `update_released`; an app-version mismatch is not fresh.
2. **Sync fake wording.** The success message names the league and "Press Apply"; replay is exact via the league name. If `run_sync_prices` returned a differently spelled league than requested, the replay would show the requested spelling. Low risk; recommendation: also keep the returned name in the log (`sync: BTreeMap<String, SyncEntry{at, name}>`) if the planner wants strict equality. Simpler map of times is acceptable.
   RESOLVED: strict equality chosen. `RequestLog.sync` maps the lower-cased requested league to `SyncEntry { at, name }`, `name` being the league the real sync returned; the fake replays `success_message(name)` and the data-present check accepts prices.json holding either spelling (plans 46-01 Task 1, 46-02 Task 1).
3. **Brief "Syncing…/Checking…" flash** during a fake (one frame, since `start` sets Running/Checking before the poll). Acceptable; avoiding it needs state-machine changes outside the contained scope.
   RESOLVED: accepted; recorded as a judgement call in AUTONOMOUS-V072-DEFERRED.md (plan 46-02 Task 3).

## Environment Availability
No external dependencies beyond the Rust toolchain (nightly via `rust-toolchain.toml`). Tests run offline.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in), `tempfile` for disk tests |
| Config | `rust-toolchain.toml`, `.cargo/config.toml` |
| Quick run | `cargo test --lib throttle` |
| Full suite | `cargo test --locked` |

### Phase Requirements -> Test Map
| Req | Behavior | Type | Command | Exists? |
|-----|----------|------|---------|---------|
| THR-01/02 | fresh inside 3600s, expired at >=3600, per-league key | unit | `cargo test --lib throttle` | Wave 0 (new file) |
| THR-02 | fake sync/leagues/update yields same message as real, Last sync (mtime) unchanged | unit (app/economy with fake closures, injected `now`) | `cargo test --lib overlay` | Wave 0 |
| THR-03 | Err / Failed never recorded | unit | `cargo test --lib throttle` | Wave 0 |
| THR-04 | save/load roundtrip incl. league list; missing/garbage file = default, no error; future time expired | unit w/ tempdir | `cargo test --lib throttle` | Wave 0 |
| THR-05 | Ready never recorded; UpToDate recorded; version mismatch not fresh | unit | `cargo test --lib throttle` | Wave 0 |
| THR-06 | CLI UA string; overlay UA unchanged; CLI path references no `throttle` (source-scan test in the style of `tests/isolation_guard.rs` over `src/sync/mod.rs` `cmd_sync`/`run`, `src/run.rs`, `src/bin/ff-cli.rs`) | unit + guard | `cargo test user_agent` | Wave 0 |

Sampling: per task `cargo test --lib <module>`; per wave `cargo test --locked`; phase gate full suite green.

Wave 0 gaps: `src/throttle.rs` tests, `user_agent_tests` extension, overlay gate tests, CLI-no-throttle guard.

## Security Domain

| ASVS | Applies | Control |
|------|---------|---------|
| V5 Input validation | yes | ff-state.json is untrusted local input: serde with `#[serde(default)]`, any parse failure -> default; never panics; league list cached only from a successful parse; size sanity optional (cap list length) |
| V12 Files | yes | atomic temp+persist in the FerrisFilter folder only; no path from the file content is ever used |
| V2/V3/V4/V6 | no | n/a |

Threats: tampered state file only lifts/extends a local wait (accepted, by design); a hostile cached league name is only displayed and sent back as a query param exactly as a fetched one would be, so no new surface. UA contains the existing public contact text only.

## Sources
- Codebase (primary, verified): `src/sync/client.rs`, `src/sync/mod.rs`, `src/update.rs` (L476-583, 829-960), `src/overlay/{app.rs,economy.rs,leagues.rs,update_job.rs}`, `src/settings.rs` (L381-450), `src/locations.rs`, `src/run.rs`, `tests/isolation_guard.rs`, `Cargo.toml`, `.gitignore`.
- `.planning/phases/46-.../46-CONTEXT.md`, `.planning/REQUIREMENTS.md` (THR-01..06).

## Metadata
**Confidence:** Standard stack HIGH (nothing new); Architecture HIGH (seams verified); Pitfalls HIGH.
**Research date:** 2026-10-23. **Valid until:** until the three `start()` seams or `ReqwestClient` change.
