---
phase: 36-per-user-locations-embedded-defaults
reviewed: 2026-10-18T00:00:00Z
depth: standard
files_reviewed: 21
files_reviewed_list:
  - src/cli.rs
  - src/compile.rs
  - src/defaults.rs
  - src/lib.rs
  - src/loader.rs
  - src/locations.rs
  - src/overlay/app.rs
  - src/overlay/apply_all.rs
  - src/overlay/config_state.rs
  - src/overlay/screenshot.rs
  - src/paths.rs
  - src/run.rs
  - src/settings.rs
  - src/sync/mapping.rs
  - src/sync/metadata.rs
  - src/sync/mod.rs
  - src/sync/prices.rs
  - tests/cli_generate.rs
  - tests/dynamic_filter.rs
  - tests/isolation_guard.rs
  - Cargo.toml
findings:
  critical: 0
  warning: 2
  info: 3
  total: 5
status: issues_found
---

# Phase 36: Code Review Report

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 21 (src/paths.rs is deleted; checked that no references remain)

## Summary

The key concerns hold up. A live file that exists but is broken errors with its full path in the overlay and in `apply`. This goes through `read_source`, which uses `try_exists`, and `read_optional` in `config_state`. An embedded default is only chosen when the live file is absent. Tests build `AppDirs::at(temp)` or pin `FERRISFILTER_HOME` and `USERPROFILE` on every spawned `apply`, `sync` and `ff`. The lib-test panic in `locations::from_env` and `resolve_root()` covers unit tests, and the isolation guard covers integration spawns. Folder creation happens only in the real writers: `settings::save`, `write_prices_atomic` (after the empty-items check) and `apply_all::commit_with` (only for a changed config file). The screenshot guard still runs before `new_screenshot` and `stage_filter`, and an empty `FERRISFILTER_HOME` is refused by it (`canonical_lossy("")` is not under temp), so it cannot reach the real folder through the empty-means-unset rule in `resolve_from`. No blockers found. The warnings and info items below are robustness and coverage gaps.

## Warnings

### WR-01: Live `prices.json` that cannot be stat'ed silently falls back to the built-in prices

**File:** `src/loader.rs:115` (used by `src/compile.rs:533`, `PriceSourcePolicy::Auto`)
**Issue:** `load_prices_optional` decides "absent" with `Path::exists()`, which maps any stat error (permission denied, sharing violation, a path component that is not a directory) to `false`. In `Auto`, "absent" now leads straight to the embedded `prices-fallback.json` + `tiers-fallback.json` with only a warning. This is the failure mode `read_source` was written to prevent for styles and tiers (`try_exists` -> `LoadError::Read`, "an unreadable live file must not silently fall back"). Prices were not given the same treatment. With embedded defaults a user gets a plausible-looking filter built from stale fallback prices, with no error naming the file.
**Fix:**
```rust
pub fn load_prices_optional(path: &str) -> Result<Option<PricesFile>, LoadError> {
    match std::path::Path::new(path).try_exists() {
        Ok(false) => Ok(None),
        Ok(true) => load_json(path).map(Some),
        Err(e) => Err(LoadError::Read { path: path.to_string(), source: e }),
    }
}
```
Add a test with a path containing `\0`, as the existing `read_source` test does.

### WR-02: `--tiers` / `--styles` accept the name of any built-in, not just the matching one

**File:** `src/compile.rs:~270-285` (`builtin_for_bare_name`, `with_tiers_or_builtin`, `with_styles_or_builtin`)
**Issue:** `defaults::by_file_name` returns any of the six built-ins. `apply --tiers prices-fallback.json` or `--tiers config.json` (file absent) therefore reads the embedded prices or config text as tier cutoffs. `--styles mapping.json` does the same for styles. It fails at parse or validation, but the message names `prices-fallback.json (built in)`, a file the user never meant, instead of the expected `Cannot read <path>`. The documented rule is "tiers-fallback.json, tiers-default.json" for tiers and "styles-default.json" for styles.
**Fix:** Restrict the lookup per call, for example `builtin_for_bare_name(given).filter(|b| matches!(b.file_name, "tiers-default.json" | "tiers-fallback.json"))` for tiers, and `== "styles-default.json"` for styles. Extend `with_tiers_or_builtin_falls_back_only_for_a_bare_built_in_name` with `"config.json"` and `"prices-fallback.json"`.

## Info

### IN-01: Isolation guard has coverage gaps

**File:** `tests/isolation_guard.rs:62-95, 117-130`
**Issue:**
- `spawns_that_resolve_the_folder_set_home` only checks that the two `.env(...)` calls exist. It does not check the value is a temp dir, so `.env("FERRISFILTER_HOME", manifest())` passes.
- It matches `cargo_bin("ff")` exactly, so `cargo_bin("ff-cli")` is only caught if the same chain has a literal `"apply"` or `"sync"`. An `apply(&args)` helper that passes the subcommand through a variable is missed.
- The chain end is the first `;`, `.assert(`, `.output(` or `.ok(`. A `;` inside a string literal ends it early and can hide a missing pin.
- `no_env_mutation` and `no_old_resolver` report only the first hit per needle per file.

**Fix:** Also match `"ff-cli"`. Alternatively, ban direct `cargo_bin` use outside one helper that always pins the env. Report all hits per file.

### IN-02: `sync --help` test pins home to the shared system temp folder

**File:** `tests/cli_generate.rs:261`
**Issue:** `std::env::temp_dir()` is the shared top-level temp folder, not a private directory. It is harmless here because `--help` exits before resolving, but it sets a bad precedent for copy-paste and is a real directory that could one day receive writes.
**Fix:** Use `tempfile::tempdir()` and pin both variables to it.

### IN-03: `sync` and `generate` now read and write different locations

**File:** `src/sync/mod.rs:23-31`, `src/cli.rs` (Sync help)
**Issue:** `sync` writes `prices.json` to the per-user folder, but `generate` (unchanged, working-directory relative) still reads `./prices.json`. Running `sync` then `generate` in the repo silently uses the fallback prices (with a warning). The CLAUDE.md "Repository layout" rows still describe `prices.json` at the repo root, and the `generate` help does not mention the split. `sync metadata` also still uses `.` and now needs a rebuild to take effect.
**Fix:** Say in the `generate` help and in CLAUDE.md that `generate` reads `./prices.json` and `sync` writes to the FerrisFilter folder (copy the file, or point `generate` at it). Update the stale CLAUDE.md `prices.json` and app-root rows.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
