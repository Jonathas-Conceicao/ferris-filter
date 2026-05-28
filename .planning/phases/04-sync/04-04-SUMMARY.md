---
phase: 04-sync
plan: "04"
subsystem: sync
tags: [testing, fixtures, fake-client, integration-tests, lib-target, tdd, offline]
dependency_graph:
  requires: [04-01, 04-02, 04-03]
  provides: [sync-integration-tests, offline-verification-gate, lib-crate-target]
  affects:
    - Cargo.toml
    - src/lib.rs
    - src/main.rs
    - tests/sync_prices.rs
    - tests/fixtures/index-state.json
    - tests/fixtures/overview-currency.json
    - tests/fixtures/overview-uniques.json
    - tests/fixtures/overview-uncutgem.json
    - tests/fixtures/overview-empty.json
tech_stack:
  added:
    - "[lib] target in Cargo.toml (crate name ferrisfilter, path src/lib.rs)"
    - "tempfile crate (already present from plan 01) used in tests for temp dirs"
  patterns:
    - "FakeClient implementing HttpClient trait with URL+type-param dispatch to fixture strings"
    - "include_str! for compile-time fixture embedding in integration tests"
    - "tempfile::tempdir() for isolated test file I/O (avoids Windows open-handle conflicts)"
    - "CRLF normalization via .replace(\"\\r\\n\", \"\\n\") on all filter text comparisons"
key_files:
  created:
    - src/lib.rs — lib crate entry point, exposes pub mod sync/models/cli/emitter/loader/pricer/error
    - tests/sync_prices.rs — 8 integration tests covering all SYNC/CLI-04 criteria via FakeClient
    - tests/fixtures/index-state.json — recorded index-state with 4 leagues (SC+HC trade, Standard, Hardcore)
    - tests/fixtures/overview-currency.json — recorded Currency overview with 6 items, rates {chaos:220, exalted:48.89}
    - tests/fixtures/overview-uniques.json — synthetic UniqueWeapon overview (Voidforge, Shaper's Touch)
    - tests/fixtures/overview-uncutgem.json — synthetic UncutGem overview with levels 16/18/20
    - tests/fixtures/overview-empty.json — off-season empty overview (empty items/lines/rates)
  modified:
    - Cargo.toml — added [lib] entry (name=ferrisfilter, path=src/lib.rs)
    - src/main.rs — refactored to use ferrisfilter:: imports instead of inline mod declarations
decisions:
  - "[lib] target added by moving module declarations from main.rs to lib.rs; main.rs uses ferrisfilter:: re-exports — both [[bin]] targets (ferrisfilter + ff) still compile unchanged"
  - "seed_prices.rs left with its local struct duplicates (already noted as intentional in its doc comment; updating it is not required for this plan)"
  - "FakeClient matches on URL substring + type query param; empty type_value matches any type for that URL"
  - "tempfile::tempdir() used in tests instead of NamedTempFile (avoids Windows 'Access is denied' when file handle is still open during atomic rename)"
  - "All 8 tests written in single pass (Tasks 1 and 2 combined) since test infrastructure was shared"
metrics:
  duration_minutes: 25
  completed_date: "2026-05-25"
  tasks_completed: 2
  files_modified: 9
---

# Phase 04 Plan 04: Sync Integration Tests (Offline Verification Gate) Summary

Fixture-driven offline integration tests locking the full sync pipeline: `src/lib.rs` + `[lib]` target enable `ferrisfilter::sync::*` in tests; a `FakeClient` (zero live network) serves 5 recorded fixtures proving schema-correct output, league resolution, gem ≥18 filtering, byte-equal failure-safety, and the closed sync→generate loop.

## What Was Built

### Crate shape change: [lib] target

- Added `[lib]` entry in `Cargo.toml` pointing to `src/lib.rs`.
- `src/lib.rs` exposes `pub mod` for all modules: `sync`, `models`, `cli`, `emitter`, `loader`, `pricer`, `error`.
- `src/main.rs` now uses `ferrisfilter::` imports instead of inline `mod` declarations — both `[[bin]]` targets (`ferrisfilter` and `ff`) compile unchanged; `default-run = "ff"` still honored.
- All 92 unit tests + 8 CLI tests pass after the restructure.

### Recorded fixtures (tests/fixtures/)

| File | Purpose |
|------|---------|
| `index-state.json` | 4 leagues: SC indexed trade, HC trade, Standard, Hardcore |
| `overview-currency.json` | 6 currency items; rates {chaos: 220.0, exalted: 48.89}; primary="divine" |
| `overview-uniques.json` | 2 uniques (Voidforge, Shaper's Touch) for UniqueWeapon type |
| `overview-uncutgem.json` | 3 gems at levels 16/18/20 (tests the ≥18 filter) |
| `overview-empty.json` | All-empty off-season response (tests the Pitfall 1 skip path) |

Math validation for `overview-currency.json` (rates.chaos=220.0, primary="divine"):
- Exalted Orb: pv=0.02045 → chaos = 0.02045 × 220 = 4.499 ≈ 4.5 (explicit override kicks in → exalt=1.0)
- Chaos Orb: pv=0.004545 → chaos = 0.004545 × 220 = 0.9999 ≈ 1.0 (explicit override: chaos=1.0)
- Divine Orb: pv=1.0 → chaos = 1.0 × 220 = 220.0

### FakeClient + tests/sync_prices.rs (8 integration tests)

**FakeClient** implements `HttpClient` by dispatching `(url_substring, type_query_value) → fixture_string`; returns `Err` when no response is registered (network-error sentinel path).

**Tests:**

1. **`happy_path_anchors_first_and_correct_values`** (SYNC-01/02/03) — builds from currency+uniques+gem fixtures, asserts items[0..2] are Exalted/Chaos/Divine, chaos values are converted (not raw), category/class populated, Voidforge has category "Uniques", gem level filter excludes 16.

2. **`happy_path_writes_schema_correct_prices_json`** — writes via `write_prices_atomic` to tempdir path, reads back as `PricesFile`, asserts schema-correct (league, fetched_at, anchors in order).

3. **`league_resolution_all_four_flag_combos`** (SYNC-05/CLI-04) — parses index-state fixture, asserts all four `(standard, hc)` flag combos resolve to the correct display name (not url slug).

4. **`league_name_recorded_in_prices_output`** — verifies the resolved league name is recorded in `PricesFile.league` and is not the url slug.

5. **`gem_filter_excludes_level_16`** (SYNC-03/D-09) — drives build over uncutgem fixture (levels 16/18/20), asserts exactly 2 items (18 and 20 survive, 16 excluded).

6. **`failure_safety_byte_equal_on_network_error`** (SYNC-04/D-07/T-04-17) — writes known-good prices.json, snapshots bytes, runs sync build with all-empty data (→ `write_prices_atomic` refused → Err), asserts bytes are byte-equal after the failed attempt.

7. **`write_prices_atomic_empty_returns_err_and_no_file`** (T-04-18) — asserts `write_prices_atomic` with empty items returns Err and creates no file.

8. **`generate_loop_with_sync_produced_prices`** — writes sync-produced prices.json to temp dir, copies config.json/tiers.json/mapping.json, runs `ferrisfilter generate`, asserts exit 0 and non-empty filter with `Show` blocks. Applies CRLF normalization per STATE.md decision.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Windows NamedTempFile open-handle conflict with write_prices_atomic**
- **Found during:** Task 1 — `happy_path_writes_schema_correct_prices_json` panicked with "Access is denied (os error 5)" on `write_prices_atomic`.
- **Issue:** Using `tempfile::NamedTempFile::new()` creates an open file handle; passing `tmp.path()` to `write_prices_atomic` then has the function create another temp file and rename it to the same path, which Windows rejects while the original handle is open.
- **Fix:** Changed to `tempfile::tempdir()` and write to a named path inside it (`tmp_dir.path().join("prices.json")`), which has no open file handle conflicts.
- **Files modified:** `tests/sync_prices.rs`
- **Commit:** `82c9ca1`

### Structural Change (per plan instructions)

**Crate shape change: `[lib]` target added** (documented per plan task 1 requirement)
- `src/lib.rs` created; `Cargo.toml` updated with `[lib]` entry.
- `src/main.rs` now imports from the lib crate via `ferrisfilter::` instead of inline `mod` declarations.
- `seed_prices.rs` binary retains its local struct duplicates (already annotated as intentional in its doc comment; no behavior change).
- Both binaries (`ferrisfilter` + `ff`) and `default-run = "ff"` compile and run unchanged.
- The existing `src/bin/seed_prices.rs` now *could* use `ferrisfilter::models::prices::*` but updating it is not required for this plan (it's a one-time migration tool that has already run).

### Scope Decision

Tasks 1 and 2 tests were written in a single pass (both tasks share `tests/sync_prices.rs` as the only file). All 8 tests were committed in one commit since the FakeClient infrastructure for Task 1 is identical to what Task 2 needs.

## Threat Mitigations Applied (from threat_model)

| Threat ID | Mitigation | Where |
|-----------|-----------|-------|
| T-04-17 | byte-equal assertion test proves failed sync never mutates prices.json | `failure_safety_byte_equal_on_network_error` |
| T-04-18 | empty fixture + write refusal test prove empty/error → Err, no corrupt output | `write_prices_atomic_empty_returns_err_and_no_file`, `failure_safety_byte_equal_on_network_error` |
| T-04-19 | FakeClient only — no live network calls in any test; asserted via no-reqwest grep | `tests/sync_prices.rs` (no reqwest import) |

## Verification Gate

Closes ROADMAP Phase 4 success criteria 2–5:

| Criterion | Test | Result |
|-----------|------|--------|
| Criterion 2: anchors-first + softcore default | `happy_path_anchors_first_and_correct_values`, `happy_path_writes_schema_correct_prices_json` | ✓ Exalted[0]/Chaos[1]/Divine[2], converted values |
| Criterion 3: league resolution + recording | `league_resolution_all_four_flag_combos`, `league_name_recorded_in_prices_output` | ✓ All 4 flag combos, display name recorded |
| Criterion 4: gem ≥18 | `gem_filter_excludes_level_16` | ✓ Level 16 excluded, 18+20 included |
| Criterion 5: byte-equal failure-safety | `failure_safety_byte_equal_on_network_error` | ✓ Bytes identical after failed sync |
| Criterion 1: spike | 04-RESEARCH.md §SPIKE | No code needed |

## Known Stubs

None — all sync functionality is implemented. The `FakeClient` constructors (`happy_path`, `all_empty`, `network_error_for_overview`) are defined but unused in tests (tests build `FakeClient` inline); these are dead code warnings only and do not affect test correctness or production behavior.

## Threat Flags

No new security-relevant surface. All new code is test-only (no new network endpoints, auth paths, or file access patterns in production code).

## Self-Check: PASSED

Files exist:
- `src/lib.rs` ✓
- `tests/sync_prices.rs` ✓
- `tests/fixtures/index-state.json` ✓
- `tests/fixtures/overview-currency.json` ✓
- `tests/fixtures/overview-uniques.json` ✓
- `tests/fixtures/overview-uncutgem.json` ✓
- `tests/fixtures/overview-empty.json` ✓

Commits exist:
- `82c9ca1` (feat(04-04): lib target + fixtures + FakeClient + integration tests) ✓

Test results: `cargo test --test sync_prices` → 8/8 passed ✓
Full suite: `cargo test` → 113 tests (92 unit + 12 CLI + 8 sync + 1 doc) all pass ✓
