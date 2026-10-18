---
phase: 36-per-user-locations-embedded-defaults
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/36-per-user-locations-embedded-defaults/36-REVIEW.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 36: Code Review Fix Report

**Scope:** Critical + Warning (`--fix --auto`, no `--all`). Info items IN-01..IN-03 are out of scope and left recorded in 36-REVIEW.md.

## Fixed

### WR-01: Live `prices.json` that cannot be stat'ed silently falls back

**Commit:** 088cea2
**Files:** `src/loader.rs`
`load_prices_optional` now uses `Path::try_exists()`: `Ok(false)` is absent (`None`), `Ok(true)` loads, and a stat error returns `LoadError::Read` naming the path (the `Cannot read` message), so `PriceSourcePolicy::Auto` no longer falls back to the built-in prices for an unreadable live file. New unit test `load_prices_optional_absent_is_none_and_unstatable_is_read_error` (absent path -> `None`; a path with `\0` -> `LoadError::Read`).

### WR-02: `--tiers` / `--styles` accept the name of any built-in

**Commit:** 1e91efd
**Files:** `src/compile.rs`
`builtin_for_bare_name` takes an allow-list: `with_tiers_or_builtin` allows `tiers-default.json` and `tiers-fallback.json`, `with_styles_or_builtin` only `styles-default.json`. Any other missing explicit path stays a `Cannot read` error. Tests extended: `tiers-default.json` falls back to the embedded default; `config.json`, `prices-fallback.json`, `styles-default.json` get no tiers fallback; `mapping.json`, `tiers-default.json`, `config.json` get no styles fallback.

## Auto iteration

Iteration 1 fixed both in-scope findings. Re-check of the two changed functions and their callers (`compile.rs` Auto price policy already propagates the `Result`; `run.rs` calls `with_*_or_builtin` unchanged) found no new Critical or Warning issues, so the loop stopped after one pass.

## Verification

Full `cargo test --no-fail-fast` with `FERRISFILTER_HOME` pointed at an empty temp dir: 1549 passed, 0 failed, 0 ignored across 14 test binaries (the temp home stayed empty).
