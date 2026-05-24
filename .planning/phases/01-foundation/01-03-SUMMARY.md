---
phase: 01-foundation
plan: "03"
subsystem: data-models
tags: [tiers, prices, validation, assert_cmd, cli-integration-tests]
dependency_graph:
  requires: [01-01]
  provides: [tiers-model, prices-model, load-tiers, cli-integration-tests]
  affects: [src/loader.rs, src/main.rs, src/models/tiers.rs, src/models/prices.rs]
tech_stack:
  added: []
  patterns: [TryFrom-validation, serde-deny-unknown-fields, assert_cmd-integration-tests, serde_path_to_error]
key_files:
  created:
    - tiers.json
    - prices.json
    - src/models/tiers.rs
    - src/models/prices.rs
    - tests/cli_generate.rs
    - tests/cli_errors.rs
  modified:
    - src/loader.rs
    - src/main.rs
decisions:
  - "tiers.json ordered ascending within each unit group (chaos group: F→E→D→C→B; div group: A→S); cross-unit transitions not compared (Pitfall 6)"
  - "serde_json::Value used for config loading in Plan 03 worktree (Config type is Plan 02 domain); wired to full Config at merge time"
  - "cli_errors.rs uses std::env::temp_dir() instead of tempfile crate to avoid new dependency"
  - "generate_in_repo_root_succeeds test conditionally skips if config.json absent (plan 02 parallel worktree)"
metrics:
  duration: "5m 17s"
  completed_date: "2026-05-23"
  tasks_completed: 2
  files_changed: 8
requirements: [CFG-02, CFG-03, CLI-05]
---

# Phase 1 Plan 3: Tiers/Prices Models + CLI Integration Tests Summary

**One-liner:** `tiers.json` + `prices.json` fixtures with `ValidatedTiers::TryFrom` (completeness + within-unit ascending), `load_tiers` wrapper, and `assert_cmd` CLI tests proving field-path errors, tier validation errors, and exit codes.

## What Was Built

### Task 1: tiers.json + prices.json fixtures and validated models (commit `e021c90`)

**tiers.json** — ordered list of 7 lower-bound entries covering all tiers S–F with `unit` tags.
Two unit groups: `div` (A at 1.0, S at 5.0) and `chaos` (F at 0.1, E at 0.5, D at 2.0, C at 10.0, B at 30.0).
Each group is strictly ascending by `min` within itself; cross-unit ordering is intentionally not validated (Phase 3 will normalize div → chaos).

**prices.json** — `{ league, fetchedAt, items }` wrapper with 4 items: Exalt, Chaos, Divine, and a Mirror of Kalandra with `baseType`. Demonstrates nullable `exalt`/`chaos`/`div` fields.

**src/models/tiers.rs** — Full implementation:
- `Unit` enum (`chaos` | `div`) with `serde(rename_all = "lowercase")`
- `TierEntry { tier, min, unit }` with Deserialize/Serialize/Clone
- `ValidatedTiers(pub Vec<TierEntry>)` newtype wrapper
- `TierValidationError` enum (thiserror + miette Diagnostic): `MissingTiers(Vec<String>)` and `NotAscending { prev, prev_min, curr, curr_min, unit }`
- `TryFrom<Vec<TierEntry>> for ValidatedTiers` enforcing all 7 tiers present AND within-unit ascending
- Inline unit tests: round-trip, fixture load, missing-tier Err, non-ascending Err, cross-unit Ok

**src/models/prices.rs** — Full implementation:
- `PricesFile { league, fetched_at, items }` with `camelCase` + `deny_unknown_fields`
- `PriceItem { name, base_type?, exalt?, chaos?, div? }` with `camelCase` + `deny_unknown_fields`
- Inline unit tests: round-trip, null field parse, baseType parse

### Task 2: Wire generate + assert_cmd integration tests (commit `2bb2ec1`)

**src/loader.rs** — Added `load_tiers(path) -> miette::Result<ValidatedTiers>`:
- Calls `load_json::<Vec<TierEntry>>(path)?` (existing generic helper)
- Applies `ValidatedTiers::try_from(entries).map_err(miette::Error::from)`

**src/main.rs** — Wired `Commands::Generate { output }`:
- Calls `loader::load_json::<serde_json::Value>("config.json")?` (exercises field-path error path)
- Calls `loader::load_tiers("tiers.json")?` (exercises tier validation error path)
- Prints stub message with style count and tier count on success
- Both file loads surface as non-zero miette exits on failure

**tests/cli_generate.rs** — 4 assert_cmd tests:
- `help_lists_subcommands` — `--help` stdout contains "generate" and "sync"
- `generate_help_shows_output_flag` — `generate --help` stdout contains "--output"
- `unknown_subcommand_exits_code_2` — exits code 2 with "unrecognized subcommand" (CLI-05)
- `generate_in_repo_root_succeeds` — exits 0 when config.json + tiers.json present (conditional skip if absent)

**tests/cli_errors.rs** — 4 assert_cmd tests:
- `missing_config_json_exits_nonzero_with_message` — "Cannot read" in stderr (CFG-02)
- `malformed_config_json_exits_nonzero_with_file_ref` — "config.json" in stderr (CFG-02)
- `missing_tier_in_tiers_json_exits_nonzero` — "missing required tier" in stderr (CFG-03)
- `non_ascending_tiers_json_exits_nonzero` — "not strictly ascending" in stderr (CFG-03)

## Test Results

```
18 tests total: 10 unit (tiers.rs: 5, prices.rs: 4, round-trip: 1) + 4 cli_errors + 4 cli_generate
All 18 pass. cargo build exits 0. cargo test exits 0.
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed conflicting `From<TierValidationError>` impl**
- **Found during:** Task 1 cargo build
- **Issue:** `TierValidationError` implements `Diagnostic + Send + Sync + 'static`, so miette already has a blanket `From<E>` impl. Explicit `impl From<TierValidationError> for miette::Error` conflicts.
- **Fix:** Removed the manual `impl From` block; `miette::Error::from(e)` still works via the blanket impl.
- **Files modified:** `src/models/tiers.rs`
- **Commit:** `e021c90` (included in Task 1 commit)

**2. [Rule 2 - Missing derives] Added `#[derive(Debug)]` to `ValidatedTiers`**
- **Found during:** Task 1 cargo test
- **Issue:** Test assertions with `{other:?}` and `{result:?}` required `Debug` on `ValidatedTiers`.
- **Fix:** Added `#[derive(Debug)]` to the `ValidatedTiers` struct.
- **Files modified:** `src/models/tiers.rs`
- **Commit:** `e021c90`

**3. [Rule 3 - Parallel constraint] Used `serde_json::Value` instead of `Config` in main.rs**
- **Found during:** Task 2 implementation
- **Issue:** `Config` type is defined in `src/models/config.rs` which is Plan 02's domain (stub `// populated in plan 02` in this worktree). Importing it would cause compile errors in this worktree.
- **Fix:** Used `serde_json::Value` for config loading in `cmd_generate`. The `load_json::<serde_json::Value>` call still exercises the `serde_path_to_error` error path for syntax errors and reports "config.json" in the error. Full typed `Config` validation will be active after the wave merges (Plan 02 defines `Config`).
- **Impact:** In the merged binary, `cmd_generate` will need updating to use `Config` instead of `Value` for full field-path + type validation. This is a known Phase 1 parallel-execution constraint, documented in the code comment.
- **Files modified:** `src/main.rs`

**4. [Rule 3 - Missing dependency] Used `std::env::temp_dir` instead of `tempfile` crate**
- **Found during:** Task 2 cli_errors.rs authoring
- **Issue:** `tempfile` crate was not in `[dev-dependencies]`. Rather than adding a new dependency, used `std::env::temp_dir()` with a timestamp-derived unique directory name.
- **Fix:** Rewrote `make_temp_dir` helper using only std library.
- **Files modified:** `tests/cli_errors.rs`

## Known Stubs

**src/main.rs — `serde_json::Value` config loading**
- File: `src/main.rs`, `cmd_generate` function
- The `loader::load_json::<serde_json::Value>("config.json")?` call will accept any valid JSON.
  Type-level field validation (deny_unknown_fields, enum variants, etc.) requires the full `Config` type from Plan 02.
- **Intentional for this wave:** Plan 02 runs in parallel and owns `config.rs`. The wire-up to `Config` is the first task of whichever plan merges the wave-2 worktrees.
- **Resolves:** When `src/models/config.rs` is populated (Plan 02 merge), replace `serde_json::Value` with `crate::models::config::Config` in `cmd_generate`.

## Self-Check: PASSED

Files exist:
- `tiers.json` FOUND
- `prices.json` FOUND
- `src/models/tiers.rs` FOUND (contains ValidatedTiers, TryFrom, MissingTiers, NotAscending)
- `src/models/prices.rs` FOUND (contains PricesFile, PriceItem, Option<f64>)
- `src/loader.rs` FOUND (contains load_tiers)
- `src/main.rs` FOUND (wired generate)
- `tests/cli_generate.rs` FOUND (contains cargo_bin, --output assertion, code 2 assertion)
- `tests/cli_errors.rs` FOUND (contains missing/malformed config assertions)

Commits exist:
- `e021c90` FOUND (Task 1: tiers/prices models)
- `2bb2ec1` FOUND (Task 2: loader + main + integration tests)

cargo build: 0 (verified)
cargo test: 0, 18/18 passing (verified)
