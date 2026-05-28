---
phase: 04-sync
verified: 2026-05-25T00:00:00Z
status: verified
score: 5/5 must-haves verified; live network behavior confirmed by human sign-off (2026-05-28)
overrides_applied: 0
human_verified: 2026-05-28T00:00:00Z
human_verification_result: "Confirmed working. Live integration proven via `sync prices --standard` (318 items, league=Standard). Softcore no-flag path fails safe when no challenge league is active — intended behavior. Signed off by user."
human_verification:
  - test: "Run `ferrisfilter sync` (no flags) against the live poe.ninja API"
    expected: "prices.json is written; items[0..2] are Exalted/Chaos/Divine; `league` field is the current softcore trade league display name (e.g. 'Fate of the Vaal'); `fetchedAt` is a non-empty ISO-8601 timestamp; file is non-empty"
    why_human: "All offline logic is proven by integration tests. The live API can only be verified by actually calling it — confirms API surface is still live, endpoint returns a non-empty response for the active league, and the file is written to disk correctly end-to-end."
  - test: "Run `ferrisfilter sync --hc` and `ferrisfilter sync --standard`"
    expected: "`--hc` writes prices.json with `league` set to the HC trade league name; `--standard` writes it with league='Standard'. Both produce a non-empty file."
    why_human: "League flag wiring to the live index-state response can only be confirmed with a real network call against poe.ninja."
---

# Phase 4: Sync Verification Report

**Phase Goal:** `ferrisfilter sync` fetches live PoE2 economy data from poe.ninja and writes a well-formed `prices.json`; the poe.ninja PoE2 API surface is confirmed via a spike before any HTTP code is written.
**Verified:** 2026-05-25 (offline) · 2026-05-28 (human sign-off — live sync confirmed working)
**Status:** verified
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | The poe.ninja PoE2 API spike is resolved: base URL, league-resolution endpoint, per-category type/overviewName values, response field names, and baseType field are all confirmed and documented before any HTTP client code was written | VERIFIED | `04-RESEARCH.md` §SPIKE FINDINGS documents live HTTP probe results from 2026-05-25: confirms base URL `https://poe.ninja/poe2/api/data/index-state`, all 21 type values tested, `primaryValue`/`core.rates` field names confirmed, absence of `baseType` field documented with workaround. Research file predates all implementation commits. |
| 2 | `ferrisfilter sync` without flags writes prices.json with Exalt, Chaos, Divine as the first three entries, from the current softcore trade league | VERIFIED (offline) | `happy_path_anchors_first_and_correct_values` asserts `items[0]="Exalted Orb"`, `items[1]="Chaos Orb"`, `items[2]="Divine Orb"`. `happy_path_writes_schema_correct_prices_json` round-trips via `PricesFile` and asserts anchor ordering. `run_sync_prices` defaults to `standard=false, hc=false` (softcore). Live network behavior flagged for human check. |
| 3 | `sync --hc` and `sync --standard` (combinable) target the correct league; league name and timestamp appear in prices.json | VERIFIED (offline) | `league_resolution_all_four_flag_combos` covers all 4 combos with fixture assertions. `league_name_recorded_in_prices_output` asserts `prices.league == "Fate of the Vaal"` (not url slug "vaal"). `run_sync_prices` builds `PricesFile { league: league_name, fetched_at: format_utc_timestamp(...) }` — ISO-8601 UTC string written to file. |
| 4 | Uncut gems are included in prices.json only at gem level >= 18 | VERIFIED | `gem_filter_excludes_level_16` on `overview-uncutgem.json` (levels 16/18/20) asserts exactly 2 items (18 and 20 survive, 16 excluded). `parse_gem_level` handles both `(Level N)` parenthetical and `-lN` id-slug formats. `mapping.json` has `"minLevel": 18` in `gemFilter`. |
| 5 | A network failure or empty poe.ninja response does not overwrite or corrupt an existing prices.json | VERIFIED | `failure_safety_byte_equal_on_network_error` writes a known-good prices.json, snapshots bytes, drives a failed sync (all-empty overview → empty item list → `write_prices_atomic` refuses), then asserts bytes are unchanged. `write_prices_atomic_empty_returns_err_and_no_file` asserts Err is returned and no file created. `NamedTempFile::persist` atomic rename is in production code. |

**Score:** 5/5 truths verified (offline); live network behavior requires human verification (see below).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/sync/client.rs` | HttpClient trait + ReqwestClient impl | VERIFIED | `trait HttpClient` with `async fn get_json`, `ReqwestClient` with 30s timeout, `error_for_status()`, ring TLS provider |
| `src/sync/models.rs` | IndexState/EconomyLeague + ExchangeOverview/ExchangeCore/ExchangeItem/ExchangeLine serde models | VERIFIED | All 6 structs present with `#[serde(default)]` on collection fields; 3 deserialization tests pass |
| `src/sync/league.rs` | `resolve_league` + `fetch_index_state` | VERIFIED | 7 unit tests cover all 4 flag combos + both empty-list Err cases + slug-vs-name guard |
| `src/sync/mapping.rs` | Mapping/OverviewType/GemFilter models + `load_mapping` + `normalize_item_class` | VERIFIED | Override table covers PoE2 class names; `include_str!` tests against committed mapping.json |
| `src/sync/metadata.rs` | `cmd_sync_metadata` — fetch RePoE, build mapping, atomic write | VERIFIED | 6 unit tests with FakeClient; validates non-empty + parseable; NamedTempFile::persist |
| `src/sync/prices.rs` | `build_prices` + `parse_gem_level` + `write_prices_atomic` + `run_sync_prices` | VERIFIED | 12 unit tests covering conversion, anchor ordering, gem filter, empty-rates skip, atomic refuse-empty |
| `mapping.json` | Committed mapping with overviewTypes + gemFilter | VERIFIED | 20 overviewTypes (12 currency + 8 Unique types), `gemFilter.type="UncutGem"`, `minLevel=18` |
| `tests/sync_prices.rs` | 8 integration tests with FakeClient | VERIFIED | All 8 pass: happy path, schema write, league resolution, league recording, gem filter, failure-safety, atomic refuse, generate loop |
| `tests/fixtures/index-state.json` | 4-league fixture | VERIFIED | Contains all 4 expected leagues with correct fields |
| `tests/fixtures/overview-currency.json` | Currency overview with `primaryValue` | VERIFIED | 6 items, rates `{chaos: 220.0, exalted: 48.89}`, primary="divine" |
| `tests/fixtures/overview-uncutgem.json` | Gem fixture with levels 16/18/20 | VERIFIED | Contains `"Level 16"`, `"Level 18"`, `"Level 20"` items |
| `tests/fixtures/overview-uniques.json` | UniqueWeapon fixture | VERIFIED | Contains Voidforge and Shaper's Touch |
| `tests/fixtures/overview-empty.json` | Off-season empty fixture | VERIFIED | `{"core":{"items":[],"rates":{},"primary":"divine"},"lines":[]}` |
| `src/lib.rs` | [lib] target exposing all modules | VERIFIED | Exposes `pub mod sync` (and all other modules); both [[bin]] targets still compile |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/main.rs` | `src/sync/mod.rs` | `sync::cmd_sync(sub)` dispatch | WIRED | `Commands::Sync { sub } => sync::cmd_sync(sub)?` |
| `src/sync/mod.rs` | `prices::run_sync_prices` | `Prices{standard,hc}` arm | WIRED | Line 22: `prices::run_sync_prices(&client, standard, hc).await` |
| `src/sync/mod.rs` | `metadata::cmd_sync_metadata` | `Metadata` arm | WIRED | Line 26: `metadata::cmd_sync_metadata(&client).await` |
| `src/sync/prices.rs` | `core.rates` | `primaryValue * rates["chaos"]` conversion | WIRED | `compute_prices` closure uses `chaos_rate.map(|r| primary_value * r)` |
| `src/sync/prices.rs` | `prices.json` | `NamedTempFile::persist` atomic write | WIRED | `write_prices_atomic` uses `NamedTempFile::new_in(dir)` + `persist(target)` |
| `src/sync/league.rs` | INDEX_STATE_URL | `fetch_index_state` | WIRED | `client.get_json(INDEX_STATE_URL, &[])` |
| `tests/sync_prices.rs` | `build_prices`/`resolve_league` | FakeClient returning fixture strings | WIRED | All 8 tests use FakeClient; no `reqwest` import in test file |
| `tests/sync_prices.rs` | generate loop | `ferrisfilter generate` via assert_cmd | WIRED | `generate_loop_with_sync_produced_prices` runs binary and asserts exit 0 + non-empty filter |
| `mapping.json` | `sync prices` runtime | `load_mapping("mapping.json")` in `run_sync_prices` | WIRED | Called before any overview fetch |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|-------------------|--------|
| `src/sync/prices.rs` — `build_prices` | `items: Vec<PriceItem>` | `overviews: &[(String, ExchangeOverview)]` from caller | Yes — populated from parsed HTTP response bodies | FLOWING |
| `src/sync/prices.rs` — `run_sync_prices` | `overviews` | `client.get_json(OVERVIEW_URL, &query)` + `serde_json::from_str` | Yes — each overview fetched from poe.ninja per type | FLOWING |
| `src/sync/league.rs` — `fetch_index_state` | `IndexState` | `client.get_json(INDEX_STATE_URL, &[])` | Yes — fetched from poe.ninja index-state | FLOWING |
| `src/sync/mapping.rs` — `load_mapping` | `Mapping` | `std::fs::read_to_string("mapping.json")` | Yes — reads committed repo artifact | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| sync prices --help shows --hc and --standard | `cargo run -- sync prices --help` (verified during plan 01) | Shows both flags | PASS |
| Full test suite passes | `cargo test` | 113 tests (92 unit + 8 CLI + 8 sync + 1 doc): 0 failures | PASS |
| cargo build succeeds | `cargo build` | 1 known non-blocking warning (async_fn_in_trait on HttpClient) | PASS |
| Live API call (softcore sync) | Cannot run without live network | N/A | SKIP — human_needed |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| CLI-02 | 04-01, 04-04 | User can run `ferrisfilter sync` to fetch prices | SATISFIED | `ferrisfilter sync` dispatches to `run_sync_prices`; 8 integration tests prove the pipeline |
| CLI-04 | 04-01, 04-03, 04-04 | `sync` accepts combinable `--standard` and `--hc` flags; neither → softcore trade | SATISFIED | `SyncSubcommand::Prices { standard: bool, hc: bool }`; `league_resolution_all_four_flag_combos` tests all combos |
| SYNC-01 | 04-01, 04-03, 04-04 | `sync` fetches PoE2 economy data from poe.ninja and writes prices.json | SATISFIED (offline) | Full pipeline proven via FakeClient + fixtures; `run_sync_prices` wires index-state → overviews → build → write |
| SYNC-02 | 04-03, 04-04 | prices.json ordered list; first three entries always Exalt, Chaos, Divine | SATISFIED | `happy_path_anchors_first_and_correct_values` and `happy_path_writes_schema_correct_prices_json` assert [0..2] ordering |
| SYNC-03 | 04-02, 04-03, 04-04 | Includes currency, uniques, other tracked drops; gems only at level >= 18 | SATISFIED | mapping.json covers 12 currency + 8 unique types; `gem_filter_excludes_level_16` proves level cutoff |
| SYNC-04 | 04-03, 04-04 | Handles missing/empty responses without corrupting existing prices.json | SATISFIED | `failure_safety_byte_equal_on_network_error` proves byte-equality; `write_prices_atomic` refuses empty items; NamedTempFile::persist |
| SYNC-05 | 04-03, 04-04 | Records fetched league and timestamp in prices.json | SATISFIED | `league_name_recorded_in_prices_output` asserts display name recorded; `format_utc_timestamp` in `run_sync_prices` writes ISO-8601 |

All 7 phase requirements satisfied.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/sync/prices.rs` | 230 | `unwrap_or(Path::new("."))` on `parent()` | Info | Benign — only fires when path has no parent, defaulting to CWD. Not a stub. |
| `src/sync/metadata.rs` | 88-108 | Refresh loop does nothing (maps ot → ot unchanged) | Info | The `cmd_sync_metadata` refreshes the mapping structurally but the per-row class refresh logic is a no-op (keeps committed value). This is by design: the curated overviewTypes structure is poe.ninja-based, not RePoE-based. Only class confirmation from RePoE would update it, and none of the current types need updating. Not a blocker — `sync metadata` correctly validates RePoE is reachable and non-empty before writing. |

No blockers or stubs found in production code. All `unwrap()` calls in sync modules are inside `#[cfg(test)]` blocks.

### Human Verification Required

#### 1. Live softcore sync run

**Test:** Run `ferrisfilter sync` (no flags) from the repo root directory.
**Expected:** Command exits 0; `prices.json` is written with items[0]="Exalted Orb", items[1]="Chaos Orb", items[2]="Divine Orb"; `league` is the current softcore trade league display name (e.g. "Fate of the Vaal"); `fetchedAt` is a non-empty ISO-8601 UTC string; items list is non-empty.
**Why human:** All logic is proven offline. Live confirmation requires poe.ninja to be reachable and the active league to have non-empty currency data — both conditions can only be checked with a real network call.

#### 2. League flag targeting

**Test:** Run `ferrisfilter sync --hc` and `ferrisfilter sync --standard` separately.
**Expected:** `--hc` writes `prices.json` with `league` set to the HC trade league name; `--standard` sets it to "Standard". Both produce non-empty files.
**Why human:** The offline tests prove the flag → league name mapping logic is correct, but the end-to-end CLI → poe.ninja API flow with real flags can only be validated against the live endpoint.

### Gaps Summary

No gaps. All 5 success criteria are satisfied — 4 fully verified in code, 1 requiring live network verification (SC2/SC3 live API behavior). The offline integration test suite (8 tests) provides comprehensive proof of correctness for all sync logic.

The one informational note: `sync metadata`'s refresh loop preserves committed class values unchanged when RePoE data confirms them — this is the intended behavior per plan 04-02 (the overviewTypes structure is curated, not derived from RePoE). The command correctly validates RePoE is reachable before writing, satisfying the failure-safety contract.

---

_Verified: 2026-05-25_
_Verifier: Claude (gsd-verifier)_
