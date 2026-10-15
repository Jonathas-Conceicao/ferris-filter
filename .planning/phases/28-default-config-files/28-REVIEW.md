---
phase: 28-default-config-files
reviewed: 2026-10-15T00:00:00Z
depth: standard
files_reviewed: 12
files_reviewed_list:
  - .gitignore
  - src/compile.rs
  - src/loader.rs
  - src/paths.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/apply_all.rs
  - src/overlay/body.rs
  - src/overlay/config_state.rs
  - src/dynamic.rs
  - tests/cli_generate.rs
  - tests/dynamic_filter.rs
findings:
  critical: 0
  warning: 4
  info: 5
  total: 9
status: issues_found
---

# Phase 28: Code Review Report

**Reviewed:** 2026-10-15
**Depth:** standard
**Files Reviewed:** 12 (plus a spot-check of the test-only path switches in `src/emitter.rs`, `src/models/*.rs`, `src/pricer.rs`, `src/styled.rs`, `src/overlay/{classes,config_view,rule_list,rules}.rs`, `tests/{cli_errors,filter_roundtrip,sync_prices}.rs`)
**Status:** issues_found

## Summary

I reviewed the diff `9cfb229..HEAD` against the phase-28 contract: choosing between the live and default source, no fallback for an explicit `--styles`/`--tiers`, the changed-on-disk guard with an "absent" baseline, source switching when the live file is created or deleted, the `-default` files never being written, and the app-root marker.

The core design is sound:
- `InputPaths` keeps the live path as the only write target. `with_styles`/`with_tiers` clear the default, so an explicit path never falls back.
- `apply_all::prepare` guards the live path against `live_baselines()`, where `None` means the file must still be absent. `safe_write` treats a `None` guard as "must not exist", so a file created since load is refused and OVERWRITE handles that case correctly.
- No production write path touches a `-default.json` file. The overlay's production readers (`load_styles_at`, `load_classes_at`, `ConfigState`, `load_economy`, `load_priced_with`) all go through `*_source()`.
- The resolver marker is `styles-default.json` everywhere.

No blockers. The weak points are:
- **Mismatched baselines:** `ConfigState` reads the content baseline and the guard baseline from the disk at two separate moments, so they can disagree (WR-01).
- **Latent live-file creation:** the `ids_unpersisted` flag can create the live file even though nothing was edited (WR-02).
- **Error handling:** an error message tells the user to edit the default file (WR-03), and `Path::exists()` treats an I/O error as "absent" (WR-04).

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: Content and guard baselines come from separate reads, so a live file created during load can be overwritten without a conflict

**File:** `src/overlay/config_state.rs:138-150` (load), `src/overlay/config_state.rs:391-393` and `409-411` (follow_disk); also `src/compile.rs:162-163`
**Issue:** `load` makes three separate filesystem accesses: it resolves the source with `styles_source()` (an `exists()` check), reads the content from that source, then reads the live file again with `read_optional` to get the guard baseline. If `styles.json` is created between the content read (which came from `styles-default.json`) and the `read_optional`, the state ends up with:
- `styles_bytes` = the default content;
- `styles_live` = `Some(<the new live bytes>)`.

If the user edits a style before the next poll's `follow_disk` runs, Apply then:
1. compares the live file with `styles_live`, finds them equal, and reports no conflict;
2. overwrites the externally created file with edits built on the defaults.

That is exactly the silent overwrite the guard exists to prevent. `follow_disk` has the same split: `now` is read from `src`, then `live` is read again. The reverse race (the live file deleted between `exists()` and `read(src)`) turns into a spurious load error.
**Fix:** Read the live file once and derive both baselines from that single read:
```rust
let styles_live = read_optional(&paths.styles)?;
let (styles_src, styles_bytes) = match &styles_live {
    Some(b) => (paths.styles.clone(), b.clone()),
    None => {
        let d = source(&paths.styles, paths.styles_source())?; // now resolves to the default
        let b = read(&d)?;
        (d, b)
    }
};
```
Do the same for tiers and in both branches of `follow_disk`. The content baseline then equals the guard baseline whenever the live file exists, and the guard baseline can never be newer than the content.

### WR-02: `ids_unpersisted` from a default source makes any Apply create a live `styles.json` without user edits

**File:** `src/overlay/config_state.rs:143`, `433`, `545`
**Issue:**
- **How it happens:** when the content comes from `styles-default.json` and that file lacks ids, `assign_missing_ids()` returns true and sets `ids_unpersisted`. `prepare_write` then treats styles as changed. `prepare` runs with `config: Some(..)` on every Apply, RULES-only included, and with a `None` baseline and a still-absent file the guard passes. So the next Apply writes a live `styles.json` the user never edited.
- **Why it matters:** this breaks DEF-03 ("An unedited CONFIG never writes; the first Apply with CONFIG edits creates the live file"). It also pins the user to a snapshot of the defaults, so later updates to `styles-default.json` silently stop reaching them.
- **Why the flag is useless here:** ids from the default are derived deterministically from the name, and the default file is never written. Persisting them buys nothing.
- **Why it is latent:** the committed `styles-default.json` carries all 12 ids today. Nothing guards that invariant, though: the test `load_falls_back_to_defaults_and_creates_nothing` relies on it but does not assert it.

**Fix:** Count the ids as unpersisted only when the live file was the source:
```rust
let ids_unpersisted = styles.assign_missing_ids() && styles_live.is_some();
// follow_disk:
self.ids_unpersisted = assigned && self.styles_live.is_some();
```

### WR-03: Legacy inline-`styles` error tells the user to edit `styles-default.json`

**File:** `src/compile.rs:257-264` -> `src/loader.rs:141-149` -> `src/models/config.rs:324-337`
**Issue:** When the live `styles.json` is absent, `load_inputs` passes the default path as `styles_path`. `check_legacy_styles` then renders the rejection as "Move any styles you changed or added into styles-default.json, then delete the `styles` section…".

That advice is wrong for this phase:
- the default file is committed and documented as never edited;
- following the advice puts the user's local changes back into a tracked file, which is the exact git-diff problem the phase set out to remove;
- the correct action is to create the live `styles.json`.
**Fix:** Pass both paths, or the live path, into the advice text. For example, have `load_inputs` call `load_config(config, source)` but construct `InlineStylesSection { styles_path: path_str(&paths.styles), .. }`. The wording would then be "...differ from styles-default.json... Move them into styles.json (create it from styles-default.json)". Add a test that runs a config with a differing inline section against a defaults-only root.

### WR-04: `read_source` uses `Path::exists()`, so an unreadable live file silently falls back to the defaults

**File:** `src/compile.rs:162-174`
**Issue:** `Path::exists()` returns `false` for any metadata error, not just NotFound. Examples are permission denied on the path, a dangling link, or a transient sharing violation from an antivirus or OneDrive scan.

In the CLI (`generate`, `apply`), a `styles.json` or `tiers.json` that exists but cannot be stat'ed is then treated as absent. The filter is generated from the defaults with no warning, and the user's customised styles or tier cutoffs vanish from the output without an error.

The overlay happens to catch this case, because `read_optional` surfaces non-NotFound errors. The two paths therefore disagree.
**Fix:**
```rust
match live.try_exists() {
    Ok(true) => return Ok(live.to_path_buf()),
    Ok(false) => {}
    Err(e) => return Err(LoadError::Read { path: path_str(live), source: e }),
}
```
Use the same distinction for the default (`try_exists` on `d`).

## Info

### IN-01: Stale doc comment on `with_tiers` still calls `tiers.json` committed

**File:** `src/compile.rs:133-134`
**Issue:** The comment says "without touching the committed `tiers.json` the test oracle depends on". Since this phase `tiers.json` is git-ignored, and the oracle reads `tiers-default.json`.
**Fix:** Change it to "the live `tiers.json` (the oracle reads `tiers-default.json`)".

### IN-02: Error-order workaround passes an empty styles path as a sentinel

**File:** `src/compile.rs:257-262`
**Issue:** `Err(_) if !paths.config.exists() => String::new()` relies on `load_config` failing on the config before it ever reads `styles_path`. If `config.exists()` is false only because of a metadata error while the read later succeeds, or if `load_config` ever reorders its reads, the user gets "Cannot read ``".
**Fix:** Make it explicit: when the styles source fails and the config is missing, call `loader::load_json::<Config>(config)` (or a dedicated config read) and return its error. Otherwise return the `NoSource` error. Never pass `""` as a path.

### IN-03: The guard ignores changes to the default file under pending CONFIG edits

**File:** `src/overlay/apply_all.rs:236-257`
**Issue:** CONFIG edits are made on top of the defaults while no live file exists. If `styles-default.json` changes before Apply (for example after a `git pull`), Apply snapshots the old defaults plus the edit into the new live file, with no notice. This follows the locked decision ("the guard compares against the live path, never the default file"), so it is not a defect. It is still worth a line in `28-MANUAL-VERIFICATION.md` or CLAUDE.md.
**Fix:** Document it. Optionally, when `live_baselines()` is `None`, also compare the default file against `baselines()` and report a soft conflict.

### IN-04: Existing installs without `styles-default.json` no longer resolve an app root

**File:** `src/paths.rs:49-53`
**Issue:** A pre-phase-28 folder holding `styles.json` but no `styles-default.json` now fails with `NotFound`, unless `FERRISFILTER_HOME` is set. The live files are perfectly usable, but the folder is not recognised. This is the intended DEF-04 behaviour; the CHANGELOG should mention that upgrades must bring the `-default.json` files along.
**Fix:** Add a CHANGELOG/upgrade note. Optionally, the not-found message could mention a `styles.json` it found without a default beside it.

### IN-05: New CLI tests leak temp directories

**File:** `tests/cli_generate.rs` (`seed_dir_with_defaults`); `tests/dynamic_filter.rs` (`workspace_defaults`)
**Issue:** The directories are created under `std::env::temp_dir()` with a timestamp name and are never removed, which follows the existing helpers' pattern. Each run leaves several directories behind.
**Fix:** Return a `tempfile::TempDir` (as `src/compile.rs` tests do), or remove the directory at the end of each test.

---

_Reviewed: 2026-10-15_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
