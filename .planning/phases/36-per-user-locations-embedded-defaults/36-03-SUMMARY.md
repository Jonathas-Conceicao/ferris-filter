---
phase: 36-per-user-locations-embedded-defaults
plan: 03
subsystem: infra
tags: [rust, compile, embedded-defaults, input-paths]
requires: [36-01, 36-02]
provides:
  - "compile.rs: DefaultSource, Source, InputPaths::in_home / from_dirs, with_tiers_or_builtin / with_styles_or_builtin"
  - "styles_source / tiers_source return Source; read_source takes Option<&DefaultSource>"
affects: [36-04, 36-05, 36-06, 36-07]
key-files:
  modified: [src/compile.rs, src/paths.rs, src/overlay/app.rs, src/overlay/config_state.rs]
key-decisions:
  - "conventional() / relative_to() keep File defaults so generate is byte-identical"
  - "Overlay callers renamed to transitional styles_source_path / tiers_source_path (removed in 36-07)"
requirements-completed: [LOC-01, LOC-02, LOC-04]
duration: ~40min
completed: 2026-10-18
---

# Phase 36 Plan 03: InputPaths embedded defaults Summary

`InputPaths` defaults can now be embedded. `in_home(root)` / `from_dirs(&AppDirs)` read live `styles.json` / `tiers.json` / `prices.json` from the folder and fall back to the compiled-in styles/tiers defaults and the embedded fallback pair. An economy compile against an empty folder succeeds, reports the fallback price source, and creates no file.

## Tasks
1. `c7f2664` `DefaultSource` / `Source` (label, bytes, as_file), `in_home`, `from_dirs`, `with_tiers_or_builtin`, `with_styles_or_builtin` (bare built-in name only, only when the file is absent), `read_source` returning `Source`. A failed existence check on a live file stays `LoadError::Read` and never falls back. `paths.rs::is_root` adapted to `as_file`. Nine overlay call sites renamed mechanically.
2. `a95f076` `load_styles_from` / `load_tiers_from` / `load_prices_from` dispatch file (existing path loaders, messages unchanged) vs embedded (36-02 `*_str` loaders labelled `<file> (built in)`). Used by `load_inputs`, `load_economy`, `load_priced_with`.

## Verification
Full `cargo test --no-fail-fast --tests` with `FERRISFILTER_HOME` on a temp dir: lib 1428 passed / 3 failed (the known baseline `economy_in_the_file_follows_pending_style_visibility`, `staged_economy_preview_follows_pending_tier_edits`, `staged_economy_preview_is_re_priced_after_a_sync`, unchanged, not fixed by this plan); cli_errors 12, cli_filter_copy 4, cli_generate 20 (byte-for-byte fixture), dynamic_filter 19, filter_roundtrip 25, filter_structure 18, overlay_rules 4, sync_prices 8: all pass. 11 new compile tests. Clippy not re-run beyond the known pre-existing baseline; no new warnings were looked for in the lib test target beyond that.

## Deviations from Plan
None. In task 1 the loaders were bridged with `Source::label()` strings so the crate compiled; task 2 replaced that with real dispatch.

## Decisions
- `load_inputs` with an embedded styles source (no current caller) loads the config with `load_json` + `attach_styles`, labelled with the built-in name.
- When config.json is missing and styles cannot be resolved, the placeholder source has an empty label, preserving the historical error order.

## Known Stubs
None.

## Self-Check: PASSED
