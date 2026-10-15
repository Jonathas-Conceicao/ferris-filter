---
phase: 28-default-config-files
plan: 02
subsystem: config loading / app root
tags: [defaults, styles, tiers, resolver, cli]
requires: [28-01]
provides:
  - InputPaths::styles_default / tiers_default + styles_source() / tiers_source()
  - compile::read_source (live, else -default.json, else LoadError::NoSource)
  - app-root marker styles-default.json
affects: [28-03, 28-04]
tech-stack:
  added: []
  patterns: ["reads go through InputPaths::*_source(); writes target the live InputPaths::styles/tiers"]
key-files:
  created: []
  modified: [src/compile.rs, src/loader.rs, src/paths.rs, tests/cli_generate.rs, tests/dynamic_filter.rs]
decisions:
  - "load_inputs keeps the historical error order: a missing config.json is reported before a styles NoSource"
  - "LoadError::NoSource reuses code ferrisfilter::io::read_error and the 'Cannot read' prefix"
  - "src/run.rs unchanged: the fallback lives in the library; the --styles/--tiers notes stay"
metrics:
  duration: ~25 min
  completed: 2026-10-15
requirements: [DEF-03, DEF-04]
---

# Phase 28 Plan 02: Read-source helper and app-root marker Summary

`generate` and `apply` now read `styles-default.json` / `tiers-default.json` when the live
`styles.json` / `tiers.json` are absent (never creating them), explicit `--styles`/`--tiers` stay
strict, and the app root is marked by `styles-default.json`.

## What was done

- **src/compile.rs:** consts `STYLES_DEFAULT_FILE` / `TIERS_DEFAULT_FILE`; `InputPaths` gains
  `styles_default` / `tiers_default: Option<PathBuf>` (set by `relative_to`, bare names via
  `conventional()`, cleared by `with_styles` / `with_tiers`); `pub fn read_source(live, default)`
  (existence check only) and `styles_source()` / `tiers_source()`. `load_inputs`, `load_economy`
  and `load_priced_with`'s live-tiers closure read through them. The fallback pair is untouched.
- **src/loader.rs:** `LoadError::NoSource { path, default }` -
  "Cannot read `styles.json` or `styles-default.json`: neither file exists", help
  "Restore styles-default.json, or create styles.json.". `Read` is unchanged.
- **src/paths.rs (DEF-04):** `is_root` checks `styles_default`; module/`is_root` docs,
  `RootError::NotFound` and the `resolve_from` message name `styles-default.json`. Tests use the new
  marker; new `live_styles_json_alone_is_not_a_root`.
- **Tests:** compile.rs gains 13 tests (default fields, `with_*` clearing, live preferred, fallback
  styles / tiers / both, explicit never falls back, neither names both, no live file created,
  defaults-only output == live-name output, load_economy fallback and NoSource for tiers).
  cli_generate.rs: `seed_dir_with_defaults` + `generate_falls_back_to_default_files`,
  `generate_prefers_live_styles_over_default`, `generate_explicit_styles_flag_never_falls_back`,
  `generate_without_any_styles_file_names_both`; `generate_styles_flag_reads_alternate_path` now
  also asserts `styles-default.json` in the error. dynamic_filter.rs: `workspace_defaults` +
  `apply_falls_back_to_default_files_and_creates_no_live_file` (section equal to the live-name run).
- **sync:** `grep -rn "styles\|tiers" src/sync` finds only doc comments and `cmd_sync` passes no
  styles/tiers path: sync reads neither file. No change.

## Verification

- `cargo test`: lib 1027 passed; integration 96 passed (12+4+20+16+23+8+4+8+1); 0 failed, run with
  the author's differing working-tree `styles.json` / `tiers.json` present. `artifact_guard` and
  `generate_snapshot` pass.
- `cargo fmt --check` clean; no new clippy warnings in the touched files.
- `git diff --exit-code -- tests/fixtures tests/snapshots`: clean.
- `styles.json` / `tiers.json` still ` M`, unstaged; sha256 equal to the parked copies.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Error order for a missing config.json**
- **Found during:** Task 1
- **Issue:** resolving the styles source before `load_config` made a dir missing both config.json
  and the styles report `NoSource` instead of the config `Cannot read` (broke
  `missing_config_surfaces_as_a_load_read_error`).
- **Fix:** when `styles_source()` fails and config.json does not exist, `load_config` runs first
  and reports the config (the styles path is then never read).
- **Commit:** f556b65

**2. Acceptance grep nuance:** `grep -n '"styles.json"' src/paths.rs` matches two lines, both in
the plan-mandated test `live_styles_json_alone_is_not_a_root` (which must write a live
`styles.json`). Production code in paths.rs no longer names `styles.json`.

## Commits

- `f556b65` feat(28-02): read styles/tiers from the -default.json files when the live file is absent
- `537db63` test(28-02): CLI coverage for generate/apply default-file fallback

## Self-Check: PASSED

- Both commits present on the branch; all five modified files exist.
