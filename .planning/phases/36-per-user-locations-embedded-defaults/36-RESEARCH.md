# Phase 36: Per-user Locations & Embedded Defaults - Research

**Researched:** 2026-10-17
**Domain:** Rust path resolution, `include_str!` embedded defaults, test/screenshot isolation (no web research needed; all findings are codebase greps)
**Confidence:** HIGH (call-site inventory is from grep of this worktree); MEDIUM on `dirs` behaviour (Cargo.lock + training knowledge)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- One folder for everything (no config / cache split): `%APPDATA%\FerrisFilter` on Windows, `~/.config/ferrisfilter` on Unix (`dirs::config_dir()` + the app name). `prices.json` lives there too.
- Resolution order: `FERRISFILTER_HOME` if set (must be an existing directory, else an error, as today), otherwise the per-user folder. The walk-up from the exe to `styles-default.json` and the working-directory fallback are **removed**.
- The folder is created by the installer (Phase 40) and by Apply / Sync when it is missing. Reads with nothing on disk use the built-in defaults; a first launch writes nothing.
- File names unchanged: `styles.json`, `tiers.json`, `ff-settings.json`, `prices.json`.
- Built-ins embedded with `include_str!` of the committed repo files (one copy): `styles-default.json`, `tiers-default.json`, `prices-fallback.json`, `tiers-fallback.json`, `config.json` (overlay class catalog), `mapping.json` (sync).
- Only `styles.json` and `tiers.json` can be overridden on disk. `config.json` and `mapping.json` are always the built-in copies for the overlay, `apply` and `sync`.
- Unchanged write rule: an unedited file is never written; a fresh install's folder stays empty until first Apply / Sync.
- A broken live file still fails with a message naming the file, now with its full per-user path.
- CLI: `ff-cli apply` reads styles/tiers from the per-user folder (or `FERRISFILTER_HOME`); `--filter` default stays the current game-folder path. `ff-cli sync` writes `prices.json` to the per-user folder; explicit flags keep working. `ff-cli install` still copies `universal.filter` from the cwd to the game folder. `generate` is unchanged (cwd-relative, byte-for-byte fixtures unchanged).
- Test isolation: one `AppDirs`-style resolver is the only code that asks the OS for a folder; everything else is handed paths. Tests build it on temp folders and never call the OS lookup. Screenshot mode keeps requiring `FERRISFILTER_HOME` under temp; with it set the per-user folder is never consulted. A first-run test: empty temp home loads defaults; Apply writes exactly the expected files in the expected folder.

### Claude's Discretion
- Type and module names (`AppDirs`, `defaults.rs`, `Source::Embedded`), how `InputPaths` is reshaped, and the file-watcher adaptation.

### Deferred Ideas (OUT OF SCOPE)
- Game folder resolver / picker and `FERRISFILTER_GAME_DIR` (Phase 37).
- Installer-created folder (Phase 40).
- No migration code from old app-root files (author starts fresh).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LOC-01 | Fresh install, no files: defaults work, Apply works | Embedded `Source`, `*_str` loaders, `safe_write` parent-dir creation (sections 1, 2, 5) |
| LOC-02 | Styles/tiers/settings/prices saved in one per-user folder | `AppDirs`, `create_dir_all` in safe_write/settings (sections 1, 5) |
| LOC-03 | `FERRISFILTER_HOME` overrides | `AppDirs::resolve_from` pure core (section 1) |
| LOC-04 | `ff-cli apply/sync/install` use same folders | `run.rs:181`, `sync/mod.rs:19`, CLI help (section 4) |
| LOC-05 | Tests/screenshot never touch real config/game folder | Isolation design + grep guard test (section 6) |
</phase_requirements>

## Summary

The change is mostly plumbing, concentrated in four files: `src/paths.rs` (resolver, rewrite), `src/compile.rs` (`InputPaths`/`read_source`/`load_priced_with`, add embedded variants), `src/loader.rs` (path-string loaders -> add text loaders), and `src/overlay/config_state.rs` + `app.rs` (byte-level reads and the styles stamp watcher). Everything else is a handful of one-line call-site swaps. All loaders today take a `&str` **path** (`loader::load_json` does `fs::read_to_string(path)` at `loader.rs:69`), so embedded defaults need a text-based twin (`load_json_str(content, name)`) with the same miette `NamedSource` error (name = display label) - this is the single most important enabling refactor.

Isolation is the top risk. Current tests are already safe against env races: **no test calls `set_var`/`remove_var`**; integration tests set `FERRISFILTER_HOME`/`USERPROFILE` per spawned `Command` (`tests/dynamic_filter.rs:70,138,327,371`, `tests/cli_errors.rs:331`) and overlay unit tests assign `app.root = Ok(path)` directly (`app.rs:4876,4910,5231...`) so none call `app_root()`. After the change, the per-user fallback means any test that spawns `ferrisfilter apply`/`sync` (or `ff`) **without** `FERRISFILTER_HOME` would silently hit the real `%APPDATA%\FerrisFilter`. Keep the pure-core-plus-thin-wrapper split, never call `dirs::` anywhere except the thin wrapper, and add a grep-style test (see section 6).

`dirs` is **not a direct dependency** (Cargo.toml has no `dirs`); `dirs 6.0.0` is already in Cargo.lock as a transitive dep of `tray-icon 0.24.2` (and `poe_data_tools` for ff-art). Add `dirs = "6"` directly; no new crate enters the tree. [VERIFIED: Cargo.lock lines 1303-1330, 4290, 6074]

**Primary recommendation:** Introduce `src/locations.rs` (replaces `paths.rs` body; keep `HOME_ENV`, `RootError`) with `AppDirs { root: PathBuf }`; `AppDirs::resolve_from(env_home: Option<&OsStr>, platform: Option<PathBuf>)` pure, `AppDirs::from_env()` the only `dirs::config_dir()` caller. Add `src/defaults.rs` with the six `include_str!` consts. Reshape `InputPaths` so `*_default` becomes a `DefaultSource::{File(PathBuf), Embedded(&'static str)}` and `read_source` returns a `Source::{File(PathBuf), Embedded{label,text}}`, keeping `relative_to` (file defaults) so ~60 existing tests are untouched and adding `InputPaths::from_dirs(&AppDirs)` (embedded defaults).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Folder resolution (`AppDirs`) | Library (`locations.rs`) | CLI/overlay wrappers | One pure fn + one env wrapper |
| Embedded defaults | Library (`defaults.rs`) | - | `include_str!` consts, compile-time |
| Source selection live-else-default | `compile.rs` | `config_state.rs` (bytes) | Existing DEF-03 logic |
| Parent-dir creation on write | `safe_write.rs` / `settings.rs` | - | Writes are the only place the folder is created |
| Test/screenshot isolation | `screenshot::guard_env` + test guard | - | Env-based, per-process |

## Standard Stack

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| dirs | 6 (lock: 6.0.0) | `config_dir()` (Windows: Known Folder RoamingAppData = `%APPDATA%`; Unix `~/.config`) | Already compiled in via tray-icon; `[VERIFIED: Cargo.lock]` presence, behaviour `[ASSUMED]` from training |

**Package Legitimacy Audit:** `dirs` is already resolved in the committed `Cargo.lock` as a transitive dependency of `tray-icon` (`[VERIFIED: Cargo.lock]`); slopcheck was not run (no new unknown crate). Age/downloads not measured: `[ASSUMED]` well-known (dirs-rs, 6.x). Planner: no human-verify checkpoint needed since lock already pins it, but add it as a direct dep in the same plan and run `cargo tree -i dirs` to confirm a single version.

**Important `dirs` caveat:** on Windows `dirs::config_dir()` uses the Known Folder API and **ignores the `APPDATA` env var** `[ASSUMED]`. So tests cannot isolate by setting `APPDATA`; only `FERRISFILTER_HOME` isolates. Do not add any test that "redirects" via `APPDATA`.

## Call-site Inventory (file:line)

### 1. Resolver (`paths.rs`)
- `src/paths.rs:22` `HOME_ENV`; `:26-37` `RootError {HomeNotADirectory, NotFound}`; `:49` `is_root` (marker `styles-default.json` via `InputPaths::relative_to(dir).styles_default`); `:56` `resolve_from(env, exe_dir, cwd)`; `:98` `app_root()`; tests `:108-225` (~12 tests of walk-up/cwd/marker: all deleted or rewritten; keep the env-must-be-a-directory and empty-env cases).
- `app_root()` callers: `src/run.rs:181` (`cmd_apply`), `src/overlay/app.rs:1183` (`App::new`), `:2159` (RELOAD). Doc comment `app.rs:10`. No others (grep of `src` and `tests`). `HOME_ENV` is also used by `src/overlay/screenshot.rs:265,295`.
- `RootError::NotFound` goes away (per-user folder always resolves unless `dirs::config_dir()` returns `None`; map that to a new `RootError::NoConfigDir` with a "set FERRISFILTER_HOME" help). `app.rs` stores `root: Result<PathBuf,String>` (`:1183`), driving the status line and disabling Sync/Apply (`:1704,1714,1785,2887,2900,3005,3076`); keep that shape so the message flow is unchanged.
- **The folder may not exist** in the per-user case. `FERRISFILTER_HOME` must exist (error kept). For the per-user folder the resolver must NOT require existence and must NOT create it (first launch writes nothing).

### 2. Disk reads of shipped files
| File | Reader | Location |
|------|--------|----------|
| `config.json` | overlay class catalogue | `app.rs:306-320` `load_classes_at` (via `InputPaths.config`), `refresh_inputs` `app.rs:1328-1354`; also `generate` (`run.rs:86`, stays cwd file) |
| `styles(-default).json` | `InputPaths::styles_source` -> `loader::load_styles(path)` | `compile.rs:145,265,308`; `app.rs:296 load_styles_at`; `app.rs:325 styles_stamp_at`; `config_state.rs:133 read_effective`, `:156 load`, `:407 follow_disk` |
| `tiers(-default).json` | `tiers_source` | `compile.rs:357`; `config_state.rs` same functions |
| `prices-fallback.json` / `tiers-fallback.json` | `compile.rs:365-369, 384-386` (`load_priced_with`) | via `paths.prices_fallback/tiers_fallback` |
| `prices.json` | `compile.rs:377`; `app.rs:862 EconomyInfo::read(root)`; sync writer `sync/prices.rs:621` | stays in the per-user folder |
| `mapping.json` | `sync/prices.rs:462` (`base_dir.join("mapping.json")` -> `load_mapping(&str)` at `sync/mapping.rs:115`); `sync/metadata.rs:20,71` (`MAPPING_PATH`, developer-only `sync metadata`: leave on the cwd file) |
| `ff-settings.json` | `settings.rs:117 path(root)`, `:126 load`, `:224 save`, `:239 save_section_state`; callers `app.rs:1187,1429,1446,1482,2163`; `screenshot.rs:299` (`settings::load(home)`) |
| Overlay-written live files | `config_state.rs:512 styles_path()`, `:516 tiers_path()` -> `apply_all.rs:239-240` write targets |

Other `InputPaths::relative_to` users (all take a root dir; keep working): `app.rs:283,298,307,326`; `apply_all.rs:182`; `config_state.rs:157,407,512,517`; `dynamic.rs` tests `:466,554,572,582,631`; `compile.rs` tests. Embedded defaults inside Rust unit tests already use `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/..."))`: `loader.rs:283,344`, `emitter.rs:749`, `pricer.rs:466,877-882`, `sync/mapping.rs:232`, `sync/prices.rs:1171` - proves the pattern and relative paths (`defaults.rs` in `src/` uses `concat!(env!("CARGO_MANIFEST_DIR"), "/styles-default.json")`).

### 3. `InputPaths` / `read_source` (`compile.rs`)
- `InputPaths` fields `compile.rs:64-77`; `conventional()` `:87` (used only by `cmd_generate`, keep exactly); `relative_to` `:94`; `with_styles/with_tiers` `:119,137` (explicit override => no default); `styles_source/tiers_source` `:145,151`; `read_source` `:164` (try_exists -> live, else default, else `LoadError::NoSource{path, default}`).
- `load_inputs` `:261` (uses `load_config_with_live(path strings)`), `load_economy` `:304`, `load_economy_with` `:320`, `load_priced_with` `:349`.
- Recommended reshape (keeps tests stable; matches ARCHITECTURE.md): 
  - `enum DefaultSource { File(PathBuf), Embedded(&'static str /*text*/, &'static str /*label e.g. "built-in styles"*/) }`.
  - `read_source(live, Option<&DefaultSource>) -> Result<Source, LoadError>` where `Source::File(PathBuf) | Source::Embedded{label, text}`; add `Source::{bytes() -> io::Result<Vec<u8>>, label() -> String}`.
  - Fallback pair becomes `enum FallbackPair { Files{prices,tiers}, Embedded }`; `relative_to` -> Files (compile.rs tests `:476-503` that assert the exact paths keep passing), `from_dirs` -> Embedded.
  - `config`: for `from_dirs`, an `Embedded` config (add `ConfigSource` enum or `Option<&'static str>`). `load_inputs` (the `generate` path) always uses `conventional()` so stays file-based; `load_economy` never reads config.
- Required new loader fns in `loader.rs`: `load_json_str<T>(content: &str, name: &str)` (refactor `load_json` to call it), `load_styles_str`, `load_tiers_str`, `load_prices_str`, `load_config_str_with_live`. `NamedSource::new(name, content)` keeps miette snippets.
- **Error text rules:** a broken *live* file's `LoadError::Read/Parse` already names `path` -> now the full per-user path automatically. Embedded parse failures (a build-time bug) must use label `styles-default.json (built in)`. The `NoSource` variant (`LoadError::NoSource`) can no longer occur for `from_dirs` (embedded always exists); keep for `relative_to`.

### 4. CLI
- `src/cli.rs:19` `--config` default `config.json` (generate, unchanged); `:24,30,84` help mentions `tiers-fallback.json` files; `:64-70` `Apply` doc says "the FerrisFilter folder (FERRISFILTER_HOME, else the folder holding ff-cli.exe, else the working directory)" -> rewrite to "FERRISFILTER_HOME, else %APPDATA%\FerrisFilter". `:91-93` `--styles` doc "beside the inputs"; `--tiers` doc "relative path resolved against the FerrisFilter folder". Also `src/paths.rs` module doc, `app.rs:10` doc, CLAUDE.md `ferrisfilter apply` row, `README.md` (user rewrites by hand - keep light) and CHANGELOG `[Unreleased]`.
- `run.rs:181-192` `cmd_apply`: replace `paths::app_root()` + `InputPaths::relative_to(&root)` with `AppDirs::from_env()?` + `InputPaths::from_dirs(&dirs)`; `under_root(&root, t)` (`:25`) keeps resolving overrides against the folder.
  - **Behaviour break to decide:** `apply --tiers tiers-fallback.json` (documented use, `cli.rs:30,84`; tested at `tests/dynamic_filter.rs:359` with an alt-tiers copy in the root) used to find the repo's `tiers-fallback.json` in the app root. With the per-user folder it won't exist. Recommendation: in `cmd_apply` if the relative override does not exist under the folder and its file name equals a built-in name (`tiers-fallback.json`, `tiers-default.json`) use the embedded text. Flag to planner as a small decision; the test at `dynamic_filter.rs:359` keeps working because it copies the file to the root.
- `sync` : `sync/mod.rs:19` `cmd_sync` passes `Path::new(".")` as `base_dir` -> change to the `AppDirs` root (create with `create_dir_all` only at write time). `sync/mod.rs:55-62 sync_economy_blocking(base_dir)` already takes the dir; overlay passes `root.clone()` (`app.rs:3139-3148`). Change `prices::run_sync_prices(client, standard, hc, league, base_dir)` (`sync/prices.rs:449`) so the mapping comes from embedded text (`Mapping` parsed from `defaults::MAPPING`, add `load_mapping_str`) and `base_dir` is only the *output* dir. `write_prices_atomic` (`prices.rs:~620`) must `create_dir_all(base_dir)` first. `sync metadata` keeps `MAPPING_PATH = "mapping.json"` (cwd, dev command; note that after it rewrites the file a rebuild is needed to embed it - add a one-line doc note). No test calls `run_sync_prices` with a temp base_dir (`tests/sync_prices.rs:44` reserved/unused), so the signature can change freely.
- `install.rs:28 cmd_install` copies cwd `universal.filter`; `default_dest` `install.rs:14` uses `USERPROFILE` (Phase 37 replaces). **No change in Phase 36**, but its help text must stay accurate.
- `dynamic::default_filter_path` `dynamic.rs:104`, `default_destination` `:34` -> unchanged this phase.

### 5. Overlay and writes
- `config_state.rs:133 read_effective(live, resolve) -> (PathBuf, Vec<u8>, Option<Vec<u8>>)`: change first tuple element to a `Source` (or `(label, bytes)`). `file_label(&src)` (used in all error messages `"<file name>: <reason>"`) - decide label for embedded: `styles-default.json`. Spec says live-file errors show the full per-user path; `file_label` currently shows only the file name (CONTEXT: "now with its full per-user path"). **Gap:** `file_label` strips directories - planner should make live-file errors use `path.display()` (check `config_state.rs` tests that assert the short form: grep `file_label` and the `"styles.json:"` assertions at `app.rs:3720-3727`).
- `styles_live`/`tiers_live` baselines (`:99,159-187,409-450`) already model "live absent" as `None`; the embedded case keeps that (`live == None`). `follow_disk` `:407-450` re-resolves each poll: works if `read_effective` returns the new `Source`.
- `app.rs:325 styles_stamp_at` returns `Option<(PathBuf, SystemTime)>` and is stored in `styles_stamp` (`:199`); with an embedded source there is nothing to stat. Change the type to `Option<StylesStamp>` where `StylesStamp = Embedded | File(PathBuf, SystemTime)`; creating/deleting the live file flips the variant (still detected, as `app.rs:3732` test intends). Tests to rewrite: `app.rs:3689-3770` (`assert_no_live`, `styles_stamp_switches_on_live_create_and_delete`, "neither exists" at `:3720-3727` becomes impossible for `from_dirs` but stays for file defaults).
- `app.rs:296 load_styles_at`, `:306 load_classes_at`: route through `InputPaths::from_dirs` and the `Source`-aware loaders; class catalog uses the embedded `config.json` always. Line `app.rs:4305` and rig helper `app.rs:3602-3610` (copies `*-default.json` into temp as live files) stay valid if the rig still builds via `relative_to`; add a **second rig** for the first-run test (empty temp root + `from_dirs`).
- `safe_write.rs`: `stage` (`:370-375`) and `write_guarded` (`:177`, `:240`) use `NamedTempFile::new_in(parent)`, which fails if the folder is missing; **no `create_dir_all` exists**. Add `fs::create_dir_all(parent)` in the stage functions (only reached when something is actually written, preserving "unedited file never written"). Same for `settings::save`/`save_section_state` (`settings.rs:224-262`, both `new_in(root)`).
- Fold clicks (`save_section_state`) write `ff-settings.json` and therefore create the folder; acceptable (no Apply involved) but note it contradicts "a first launch writes nothing" only after user interaction, not at launch.
- `EconomyInfo::read(root)` `app.rs:862` reads `root/prices.json`: unchanged.
- The `filter_stat`/sound list code is unrelated.

### 6. Isolation (LOC-05)
- `screenshot.rs:262-286 guard_env`: needs `FERRISFILTER_HOME`, `USERPROFILE`, derived game dir all under temp; `paths_under_temp` `:225-258` pure. With `FERRISFILTER_HOME` required, `AppDirs::from_env()` takes the Home branch and never calls `dirs::config_dir()`. **Add belt and braces:** `AppDirs::from_env()` called when `cfg!(debug_assertions)` and `--screenshot` was parsed is already impossible because guard runs first (`overlay/mod.rs:138`); order must stay guard-before-`App::new`. Add a unit test asserting `guard_env` fails when `FERRISFILTER_HOME` is unset (exists at `screenshot.rs:515`; keep). Game-folder isolation stays on `USERPROFILE` until Phase 37.
- `dirs::config_dir()` ignores env vars on Windows `[ASSUMED]`, so `USERPROFILE=temp` does not protect `%APPDATA%`. The guard therefore must keep *requiring* `FERRISFILTER_HOME` (it does).
- Unit tests: all overlay/unit tests construct `AppDirs`/roots from `tempfile`; none may call `AppDirs::from_env()`. Enforce with `#[cfg(test)]` in `from_env` -> `panic!("tests must not resolve the real per-user folder")` when `FERRISFILTER_HOME` is unset. Integration tests link the non-test lib, so they need the second guard: add `tests/isolation_guard.rs` that scans `tests/*.rs` and requires every `.cargo_bin("ferrisfilter"|"ff-cli"|"ff")` call (`dynamic_filter.rs:67,134,324,368`; `cli_errors.rs`; `cli_filter_copy.rs:43-88`; `cli_generate.rs`; `sync_prices.rs`) whose subcommand is `apply`/`sync` to set `FERRISFILTER_HOME`; plus a grep test that `dirs::` appears only in `src/locations.rs`. Env-var races: not an issue because the suite never mutates the process env; keep it that way (a `set_var` ban in the grep test; in edition 2024 `set_var` is `unsafe` anyway).
- Existing spawn sites already set `FERRISFILTER_HOME` for `apply`; `cli_filter_copy.rs`/`cli_generate.rs` spawn `generate`/`install`/`--help` (cwd-relative, unaffected).

### Tests that break (must be edited)
1. `src/paths.rs` tests `:108-225` (walk-up, cwd marker, `NotFound`).
2. `tests/dynamic_filter.rs:311 app_root_from_ferrisfilter_home_wins_over_cwd` stays (HOME wins over cwd) - still valid. Others in that file copy `styles.json`/`tiers.json` into `HOME` as live files: still valid. `workspace_defaults` (`:55`) copies `styles-default.json` into HOME to test file defaults - after the change the HOME copy of `*-default.json` is ignored; the test "apply reads defaults when no live files" (`~:395`) should now assert the embedded defaults produce identical output (strengthens DEF-03 -> LOC-01). Verify no test asserts `NoSource` for `apply`.
3. `src/overlay/app.rs:3689-3770` stamp tests; `:3720` asserts error mentions `styles-default.json`.
4. `src/compile.rs:590-650` read_source tests: keep (File defaults) and add Embedded variants.
5. `tests/cli_errors.rs:~301` expects `config.json` in stderr for a missing config - generate path, unchanged.
6. `.cargo/config.toml` could add `[env] FERRISFILTER_HOME` for dev - **don't** (it is also visible to `cargo test` and would defeat first-run tests). Dev workflow = documented env var; update CLAUDE.md Development warning.

## Architecture Patterns

```
env FERRISFILTER_HOME? --yes--> must be dir ------------+
        | no                                              |
        v                                                 v
dirs::config_dir()/FerrisFilter  (not created)        AppDirs { root }
                         \________________________/        |
                                      |                     |
   InputPaths::from_dirs(&AppDirs) <--+                     |
     styles: root/styles.json   default: Embedded(STYLES)   |
     tiers : root/tiers.json    default: Embedded(TIERS)    |
     prices: root/prices.json   fallback: Embedded pair     |
     config: Embedded(CONFIG)                               |
        |                                                   |
        v                                                   v
 read_source -> Source::{File,Embedded} -> loader::*_str    settings::load/save(root)
        |                                                   safe_write (create_dir_all
        v                                                   on first real write)
 compile::load_economy -> dynamic::economy_body -> Apply
 sync_economy_blocking(root) -> embedded MAPPING -> root/prices.json
```

Pattern: pure core + thin env wrapper (already in `paths.rs`):
```rust
pub fn resolve_from(home: Option<&OsStr>, platform: Option<PathBuf>) -> Result<AppDirs, RootError> {
    if let Some(v) = home.filter(|v| !v.is_empty()) { /* must be dir, as today */ }
    platform.map(|p| AppDirs { root: p.join("FerrisFilter") }).ok_or(RootError::NoConfigDir)
}
pub fn from_env() -> Result<AppDirs, RootError> {  // the only dirs:: caller
    resolve_from(std::env::var_os(HOME_ENV).as_deref(), dirs::config_dir())
}
```
Note the folder name: Windows `FerrisFilter`, Unix `ferrisfilter` (CONTEXT: `~/.config/ferrisfilter`) - use `cfg!(windows)` for the segment.
Careful: call `dirs::config_dir()` lazily (only when HOME unset) so Home mode never consults the OS - make `platform` a closure `impl FnOnce() -> Option<PathBuf>`.

### Don't Hand-Roll
| Problem | Use | Why |
|---------|-----|-----|
| Config dir | `dirs::config_dir()` | Known Folder API, OneDrive-redirect aware |
| Atomic write | existing `safe_write` + `tempfile` | already guards changed-on-disk |
| Embedding | `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/x.json"))` | same bytes `artifact_guard` reads |

## Common Pitfalls
1. **Tests hit real %APPDATA%** (data loss). Mitigation: section 6 guards; resolver closure; cfg(test) panic.
2. **Embedded `styles-default.json` ids:** `assign_missing_ids` runs on load and "ids_unpersisted" logic (`config_state.rs:165`) only counts for live files; keep behaviour: embedded source -> not marked dirty.
3. **`file_label` strips directories** so error messages would not "name the full per-user path" (CONTEXT requirement). Change for live files.
4. **`read_effective` reads bytes twice** (existence then resolve) - embedded `bytes()` is infallible; keep the race-handling test `config_state.rs:940`.
5. **Windows path `\\?\`:** never canonicalize the root (existing comment `paths.rs` `app_root`).
6. **Fallback pair `--tiers tiers-fallback.json`** regression (section 4).
7. **Fold clicks create the folder**; fine, but `safe_write` and `settings::save` need `create_dir_all` or the first Apply on a fresh install fails with `Unwritable` (LOC-01 success criterion 1).
8. **`sync metadata`** still rewrites the cwd `mapping.json`; embedded copy changes only on rebuild.
9. **`generate` must stay byte-identical:** it uses `conventional()` (bare relative names, file defaults). Do not route it through `from_dirs`. `artifact_guard`/`cli_generate` snapshot tests untouched.
10. Debug-only `ff` binary run from the repo without `FERRISFILTER_HOME` now uses real %APPDATA% (CLAUDE.md "Development warning" must be rewritten).

## Environment Availability
No new external tools. `cargo` (nightly per `rust-toolchain.toml`) already required. Step skipped otherwise.

## Validation Architecture

| Property | Value |
|----------|-------|
| Framework | built-in `cargo test` + `assert_cmd` 2.2, `insta` 1.47.2 |
| Config | none (nightly toolchain file) |
| Quick run | `cargo test --lib locations defaults compile` |
| Full suite | `cargo test` |

| Req | Behaviour | Test | Command | Exists |
|-----|-----------|------|---------|--------|
| LOC-01 | empty temp home: `ConfigState::load`, class catalog, economy body (fallback prices) load; Apply works | unit (overlay rig) + CLI `apply --add-economy` with empty HOME | `cargo test first_run` | Wave 0 |
| LOC-02 | Apply writes `styles.json`/`tiers.json`/`ff-settings.json` under (created) home subfolder; `sync` write creates folder | unit with HOME = temp/`sub` not yet existing (via `AppDirs` constructor, not env) | `cargo test per_user` | Wave 0 |
| LOC-03 | HOME wins; bad HOME errors; per-user never consulted (closure not called) | unit on `resolve_from` | `cargo test locations` | rewrite `paths.rs` tests |
| LOC-04 | `apply` reads HOME; `sync` base_dir = AppDirs; help text mentions %APPDATA% | `tests/dynamic_filter.rs`, `cli_generate.rs` help | `cargo test --test dynamic_filter` | extend |
| LOC-05 | spawn-site grep, `dirs::` grep, `guard_env` refusals | `tests/isolation_guard.rs` + `screenshot.rs:515` | `cargo test isolation` | Wave 0 |
| (regression) | byte-for-byte fixture | `cargo test --test cli_generate` / `artifact_guard` | existing | yes |

Sampling: per task `cargo test --lib`; per wave `cargo test`.

## Security Domain
ASVS V5 (input validation): live JSON still goes through serde + `validate()`; V12 (file handling): `FERRISFILTER_HOME` must be a directory; settings filter-name validation unchanged; no new network. Path traversal: unchanged `valid_filter_name`. Per-user folder is the user's own profile (no elevation).

## Assumptions Log
| # | Claim | Section | Risk |
|---|-------|---------|------|
| A1 | `dirs::config_dir()` on Windows ignores `APPDATA` env and returns Roaming AppData | Stack | If it honours the var, tests could isolate via env - harmless |
| A2 | `dirs` 6.x API `config_dir()` returns `Option<PathBuf>` | Pattern | trivial compile fix |
| A3 | `apply --tiers tiers-fallback.json` built-in-name fallback is desired | CLI | Author may prefer cwd-relative or removal |

## Open Questions
1. `--tiers tiers-fallback.json` under apply (A3) - recommend built-in-name fallback to embedded.
2. Should `AppDirs::from_env()` hard-panic under `cfg(test)` when HOME unset? Recommend yes (lib unit tests only).
3. `file_label` short vs full path for live files (CONTEXT says full per-user path): recommend full path for live, bare label for embedded.

## Recommended Plan Decomposition (waves)

**Wave 1 (parallelizable, no behaviour change):**
- 36-01 `src/locations.rs` (AppDirs, `resolve_from` pure + `from_env`, `RootError` update, rewrite `paths.rs` tests, add `dirs = "6"`), keep `paths::` re-exports or update 3 call sites later.
- 36-02 `src/defaults.rs` (six `include_str!` consts) + `loader.rs` `*_str` loaders (`load_json` delegates), `sync/mapping.rs load_mapping_str`.
- 36-03 `safe_write.rs` + `settings.rs` `create_dir_all` on real write; tests: write into a not-yet-existing folder; no-write when unchanged.

**Wave 2 (depends on 1):**
- 36-04 `compile.rs`: `DefaultSource`/`Source`/`FallbackPair`, `InputPaths::from_dirs`, `load_priced_with` + `load_economy` using text loaders; keep `relative_to`/`conventional` unchanged; extend compile tests (embedded cases, error naming).
- 36-05 `sync`: embedded mapping, output-dir-only `base_dir`, `cmd_sync` -> AppDirs, `write_prices_atomic` creates folder; overlay Sync call unchanged.

**Wave 3 (depends on 2):**
- 36-06 overlay: `config_state.rs` (`read_effective` -> Source, `file_label`), `app.rs` (`App::new`/RELOAD via `AppDirs`, `load_styles_at`, `load_classes_at` embedded config, `StylesStamp`), `apply_all.rs:182`, `staged_economy_body`; rewrite stamp tests.
- 36-07 CLI: `run.rs:181`, `--tiers` built-in fallback, `cli.rs` help, then docs (CLAUDE.md rows `styles.json`/`ff-settings.json`/`apply`/Development warning, CHANGELOG Unreleased).

**Wave 4 (verification):**
- 36-08 first-run test (empty temp home: defaults load, Apply writes exactly `styles.json`? - note: unedited files are not written, so first Apply of an untouched config writes only the filter; assert the exact set), per-user-folder creation test, `tests/isolation_guard.rs`, screenshot guard test, run `cargo test`, byte-for-byte fixture, and a manual check listing `%APPDATA%\FerrisFilter` before/after `cargo test` and `--screenshot`.

## Sources
- Local greps of the worktree (all file:line above) - HIGH.
- `.planning/research/ARCHITECTURE.md`, `PITFALLS.md` (training-knowledge items re-tagged ASSUMED).
- `Cargo.lock` for `dirs 6.0.0` - HIGH.

**Research date:** 2026-10-17 | **Valid until:** until Phase 37 changes game-folder code (~30 days)
