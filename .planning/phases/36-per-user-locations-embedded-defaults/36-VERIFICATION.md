---
phase: 36-per-user-locations-embedded-defaults
verified: 2026-10-18T00:00:00Z
status: human_needed
score: 5/5 must-haves verified (code and tests); 2 real-machine checks outstanding
overrides_applied: 0
human_verification:
  - test: "Launch the release `ff` on a machine with a truly empty %APPDATA%\\FerrisFilter (folder absent) and no config.json / mapping.json / *-default / *-fallback files beside the exe. Open each tab, then change a style and a tier cutoff and press Apply."
    expected: "Default styles, tiers, class picker groups and fallback prices all appear; Apply succeeds; %APPDATA%\\FerrisFilter is created and holds styles.json, tiers.json and ff-settings.json; nothing is written beside the exe or in the working directory."
    why_human: "Needs the real GUI and the real OS per-user folder. Tests are forbidden from resolving it (lib-test panic, isolation_guard), so that path is only exercised through the pure resolver."
  - test: "Press Sync economy on the Economy tab against the live network (with and without FERRISFILTER_HOME set)."
    expected: "prices.json is written into %APPDATA%\\FerrisFilter (or into FERRISFILTER_HOME when set, with nothing written to %APPDATA%); the status line names the league."
    why_human: "Real network call to poe.ninja and real per-user folder."
---

# Phase 36: Per-user locations and embedded defaults - Verification Report

**Phase Goal:** A fresh install with no files of its own works, and the user's styles, tiers, settings and synced prices live in per-user folders that tests and screenshot mode can never confuse with real ones.
**Status:** human_needed (all automated and code-level checks pass; no gaps)
**Re-verification:** No, initial verification

## Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Empty home shows default styles, tiers, class catalog, fallback prices; Apply works | VERIFIED (code + tests) | `src/defaults.rs` embeds all six data files with `include_str!` (styles-default, tiers-default, prices-fallback, tiers-fallback, config, mapping) and has a test that each equals the on-disk file and parses. `InputPaths::from_dirs` (`src/compile.rs:180-185`) uses `DefaultSource::Embedded`. Overlay class catalog loads `defaults::CONFIG` (`src/overlay/app.rs:348`), sync mapping loads `defaults::MAPPING` (`src/sync/prices.rs:467`). Tests `economy_from_an_empty_home_uses_the_built_ins_and_writes_nothing`, `load_inputs_falls_back_to_both_defaults_and_creates_no_live_file`, `load_economy_falls_back_to_defaults_and_creates_no_live_file`. A live file that exists but is broken errors with its path rather than falling back (`read_source`, `try_exists`). Real-GUI launch: see human item 1. |
| 2 | Apply writes styles/tiers/settings under the per-user folder (created if missing); Sync writes prices.json there | VERIFIED (code) | `locations::from_env` resolves `dirs::config_dir()/FerrisFilter` (`src/locations.rs`). Folder creation only in real writers: `settings::save` (`create_dir_all(root)`), `apply_all::commit_with` (`create_dir_all`, `apply_all.rs:326`, only for changed config files), `sync/prices.rs:427` (`write_prices_atomic`). `sync` Prices base dir is `locations::from_env().root` (`src/sync/mod.rs:25`). Real-folder and network behaviour: human items 1 and 2. |
| 3 | `FERRISFILTER_HOME` pins everything; per-user folder untouched | VERIFIED | `resolve_from` returns the env dir when set and never falls through (a non-directory is `HomeNotADirectory`); unit tests `env_dir_wins...`, `platform_not_called_when_home_set`, `env_missing_dir_is_error_and_does_not_fall_through`. Every consumer goes through `from_env`. Orchestrator run with HOME on an empty temp dir: `%APPDATA%\FerrisFilter` absent before and after. |
| 4 | `ff-cli apply`/`sync`/`install` use the same folders as the overlay; `generate` stays cwd-relative; fixture test passes | VERIFIED | `apply` (`src/run.rs:181`) and `sync` (`src/sync/mod.rs:25`) call `locations::from_env`; overlay uses the same via `resolve_root`. `install` copies a cwd file to the game folder derived from USERPROFILE (unchanged; the game-folder redesign is Phase 40/GAME-xx, matching 36-CONTEXT). `cmd_generate` untouched (cwd inputs). Old walk-up resolver `src/paths.rs` is deleted with no references left. Orchestrator run: cli_generate 20 and filter_roundtrip 25 pass, 0 failures overall (`artifact_guard`/snapshot included). |
| 5 | Tests and `--screenshot` refuse to touch real config/game folder | VERIFIED | `from_env` panics under `#[cfg(test)]` unless `FERRISFILTER_HOME` is set; overlay `resolve_root` returns an error under test. `tests/isolation_guard.rs` (5 tests) statically requires spawned `ff`/`ff-cli` apply/sync to pin both env vars, bans env mutation and the old resolver. Screenshot `guard_env` (`src/overlay/screenshot.rs:264`) requires FERRISFILTER_HOME, USERPROFILE and the game folder under a temp folder with a Temp/tmp component, rejects `..`, and refuses to overwrite a file in a folder it did not create. An empty FERRISFILTER_HOME fails the guard (not under temp), so it cannot reach the real folder. |

**Score:** 5/5

## Requirements Coverage

| Requirement | Description | Status | Evidence |
|-------------|-------------|--------|----------|
| LOC-01 | Fresh install works from built-in defaults | SATISFIED | Truth 1 |
| LOC-02 | Per-user folder for styles/tiers/settings/prices, created on Apply/Sync (installer part is Phase 40) | SATISFIED (real-machine check pending) | Truth 2 |
| LOC-03 | `FERRISFILTER_HOME` override | SATISFIED | Truth 3 |
| LOC-04 | CLI apply/sync/install share folders; generate cwd-relative | SATISFIED | Truth 4 |
| LOC-05 | Tests and screenshot mode cannot touch real folders | SATISFIED | Truth 5 |

No orphaned requirement IDs: REQUIREMENTS.md maps exactly LOC-01..05 to Phase 36, all claimed by plans and marked Complete.

## Code Review Findings Assessment

- **WR-01** (`load_prices_optional` uses `Path::exists`, `src/loader.rs:115`, confirmed present): an un-stat-able live `prices.json` falls back to the built-in prices with the existing fallback warning instead of an error. This is a robustness gap, not a failure of any success criterion (criterion 1 concerns an absent file; the fallback still warns). Recommend fixing with `try_exists` before release. WARNING, non-blocking.
- **WR-02** (`--tiers`/`--styles` accept any built-in file name): an error-message accuracy issue on a dev-only override that fails at parse; no criterion depends on it. WARNING, non-blocking.
- Info items IN-01..03 (guard coverage gaps, shared temp dir in one `--help` test, `generate` reading `./prices.json` while `sync` writes the per-user folder) do not undermine a criterion. IN-03 is a documentation point (CLAUDE.md rows are stale) and is covered by the accepted "For the Release Review" outcome of 36-07.

## Anti-Patterns

None blocking. `src/paths.rs` removal verified; no TBD/FIXME debt markers were introduced in the reviewed files (review reports none).

## Behavioral Spot-Checks

Not run by the verifier (cargo and GUI runs were restricted); relied on the orchestrator's isolated full run (`cargo test --no-fail-fast`, 0 failures, `%APPDATA%\FerrisFilter` untouched) and on reading the code paths above.

## Accepted Deviations

Plan 36-07's CHANGELOG entry and README fix were deliberately deferred to the Release Review per PROJECT.md Milestone Close-out; not a gap.

## Gaps Summary

No gaps. Status is human_needed only because the fresh-install launch on a real empty %APPDATA% and a real network Sync cannot be exercised by tests by design.

---

_Verified: 2026-10-18_
_Verifier: Claude (gsd-verifier)_
