---
phase: 04-sync
plan: "01"
subsystem: sync
tags: [async, http, reqwest, tokio, serde, cli, poe-ninja]
dependency_graph:
  requires: []
  provides: [sync-cli-surface, http-client-seam, poe-ninja-models]
  affects: [src/cli.rs, src/main.rs, src/sync/mod.rs, src/sync/client.rs, src/sync/models.rs, Cargo.toml]
tech_stack:
  added:
    - reqwest 0.13 (rustls-no-provider + json + query features; pure-Rust TLS via ring)
    - tokio 1 (rt-multi-thread + macros; async runtime internal to cmd_sync)
    - tempfile 3 (atomic file writes via NamedTempFile::persist)
    - rustls 0.23 with ring feature (required crypto provider for rustls-no-provider)
  patterns:
    - Runtime-internal async: tokio::Runtime::new().block_on() keeps main() sync
    - Injectable HTTP seam: HttpClient trait + ReqwestClient impl; downstream uses generic C: HttpClient
    - Lenient serde: #[serde(default)] on collection fields; no deny_unknown_fields on API models
key_files:
  created:
    - src/sync/mod.rs — cmd_sync entry point with tokio runtime-internal dispatch
    - src/sync/client.rs — HttpClient trait + ReqwestClient (30s timeout, error_for_status, ring TLS)
    - src/sync/models.rs — IndexState/EconomyLeague + ExchangeOverview/ExchangeCore/ExchangeItem/ExchangeLine + 3 deserialization tests
  modified:
    - Cargo.toml — added reqwest/tokio/tempfile/rustls deps
    - src/cli.rs — replaced flagless Sync with Sync{sub: Option<SyncSubcommand>} + SyncSubcommand enum
    - src/main.rs — added mod sync; changed dispatch arm to sync::cmd_sync(sub)?; removed old cmd_sync stub
decisions:
  - "reqwest 0.13 uses rustls-no-provider + ring instead of rustls-tls (feature does not exist in 0.13)"
  - "reqwest query feature required for .query() method on RequestBuilder (not in json feature)"
  - "HttpClient takes generic C: HttpClient in downstream plans (not &dyn) — avoids dyn-compat issues"
metrics:
  duration_minutes: 6
  completed_date: "2026-05-25"
  tasks_completed: 3
  files_modified: 6
---

# Phase 04 Plan 01: Sync Network Foundation Summary

Async/HTTP/serde foundation for `sync`: reqwest + tokio deps, `sync prices`/`sync metadata` CLI surface, injectable `HttpClient` seam, and poe.ninja response models with inline deserialization tests.

## What Was Built

- **Cargo.toml deps:** reqwest 0.13 (rustls-no-provider + json + query), tokio 1 (rt-multi-thread + macros), tempfile 3, rustls 0.23 (ring provider). No C compiler required.
- **CLI surface:** `ferrisfilter sync --help` lists `prices` and `metadata` subcommands. `sync prices --help` shows `--standard` and `--hc` flags. Bare `sync` dispatches to `Prices{false,false}` (softcore trade default) and returns "not yet implemented" error — wiring confirmed.
- **`src/sync/mod.rs`:** Runtime-internal pattern: `cmd_sync()` is sync, creates `tokio::runtime::Runtime`, calls `block_on(run(sub))`. `main()` stays sync; generate path unchanged.
- **`src/sync/client.rs`:** `HttpClient` trait with `async fn get_json(url, query) -> miette::Result<String>`. `ReqwestClient` wraps `reqwest::Client` with FerrisFilter User-Agent, 30s timeout, and `.error_for_status()` (T-04-03). `ReqwestClient::new()` installs ring as the rustls crypto provider.
- **`src/sync/models.rs`:** All 6 serde structs from the plan `<interfaces>` block, exactly matching 04-RESEARCH.md confirmed live responses. `#[serde(default)]` on `lines`, `core.items`, `core.rates`. Three inline tests covering index-state, currency overview, and empty-overview (Pitfall 1 guard).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] reqwest 0.13 `rustls-tls` feature does not exist**
- **Found during:** Task 1 — `cargo build` failed with "reqwest does not have that feature"
- **Issue:** The plan specified `features = ["rustls-tls", "json"]` but reqwest 0.13 renamed this feature to `rustls` (which requires aws-lc-rs C compiler) and `rustls-no-provider` (pure Rust, no C required).
- **Fix:** Used `features = ["rustls-no-provider", "json", "query"]` + `rustls = { version = "0.23", features = ["ring"] }` to provide the ring crypto provider. This achieves the identical goal: pure-Rust TLS with no C compiler, wasm-forward-compatible.
- **Files modified:** `Cargo.toml`
- **Commit:** `4a9dfa7` (updated in `3d7d562` to add query feature)

**2. [Rule 3 - Blocking] reqwest `query` feature not included in initial deps**
- **Found during:** Task 3 GREEN phase — `cargo test` failed: "no method named `query` found for struct `RequestBuilder`"
- **Issue:** The `.query()` method on `RequestBuilder` is gated behind the `query` feature (separate from `json`). This was missing from the initial dep spec.
- **Fix:** Added `"query"` to reqwest features.
- **Files modified:** `Cargo.toml`
- **Commit:** `3d7d562`

## TDD Gate Compliance

RED gate: `403172e` — `test(04-01): add failing tests for poe.ninja serde models (RED)`
GREEN gate: `3d7d562` — `feat(04-01): implement HttpClient trait + ReqwestClient + poe.ninja serde models (GREEN)`

Both gates present in order. REFACTOR phase not needed — implementation was clean on first pass.

## Threat Mitigations Applied (from threat_model)

| Threat ID | Mitigation | Where |
|-----------|-----------|-------|
| T-04-01 | HTTPS-only; `rustls-no-provider` + ring TLS | `ReqwestClient::new()` |
| T-04-03 | `.error_for_status()` turns 4xx/5xx into Err | `client.rs:37` |
| T-04-04 | 30s `.timeout()` on client builder | `client.rs:22` |
| T-04-05 | `get_json` takes caller-supplied URL; plans 02/03 use compile-time constant URLs | Design constraint documented |

## Known Stubs

- `sync::run()` returns `Err("sync prices not yet implemented")` and `Err("sync metadata not yet implemented")` — intentional stubs; implementations land in plans 04-02 (metadata) and 04-03 (prices).

## Self-Check: PASSED

Files exist:
- `src/sync/mod.rs` ✓
- `src/sync/client.rs` ✓
- `src/sync/models.rs` ✓

Commits exist:
- `4a9dfa7` (Task 1: deps + skeleton) ✓
- `bb0efb6` (Task 2: CLI restructure) ✓
- `403172e` (Task 3 RED: failing tests) ✓
- `3d7d562` (Task 3 GREEN: implementation) ✓
