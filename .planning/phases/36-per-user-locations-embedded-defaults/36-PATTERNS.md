# Phase 36: Per-user Locations & Embedded Defaults - Pattern Map

**Mapped:** 2026-10-17
**Files analyzed:** 17 (3 new, 14 modified) plus 2 new test files
**Analogs found:** 17 / 17 (all are in-repo; most are self-analogs, i.e. the file being modified holds the pattern to extend)

All excerpts were read from this worktree. Line numbers are from the current tree.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/locations.rs` (new, replaces `paths.rs` body) | utility (resolver) | request-response (pure fn + env wrapper) | `src/paths.rs` | exact (same split) |
| `src/defaults.rs` (new) | config (embedded consts) | transform | `src/loader.rs:283,344` test consts | role-match |
| `src/lib.rs` | config (module list) | n/a | itself (`pub mod paths;`) | exact |
| `Cargo.toml` (`dirs = "6"`) | config | n/a | `tempfile = "3"` line 40 | exact |
| `src/loader.rs` (`*_str` loaders) | service (loader) | file-I/O -> transform | `load_json` / `load_styles` / `load_tiers` in same file | exact |
| `src/sync/mapping.rs` (`load_mapping_str`) | service | transform | `load_mapping` `:115` | exact |
| `src/compile.rs` (`DefaultSource`, `Source`, `FallbackPair`, `from_dirs`) | model/service | file-I/O | `InputPaths`, `read_source` `:64-182` | exact |
| `src/safe_write.rs` (`create_dir_all` in stage) | utility | file-I/O | `stage` `:370-380` | exact |
| `src/settings.rs` (`create_dir_all` in `save`/`save_section_state`) | utility | file-I/O | `save` `:224-232` | exact |
| `src/sync/mod.rs` + `src/sync/prices.rs` (embedded mapping, output-only `base_dir`) | service | request-response + file-I/O | `cmd_sync` / `sync_economy_blocking` / `run_sync_prices` | exact |
| `src/run.rs` (`cmd_apply`) | controller (CLI) | request-response | itself `:181-192` | exact |
| `src/cli.rs` (help text) | config | n/a | itself `:64-93` | exact |
| `src/overlay/config_state.rs` (`read_effective`, `file_label`) | service/state | file-I/O | itself `:107-111,125-190` | exact |
| `src/overlay/app.rs` (`App::new`, `load_styles_at`, `load_classes_at`, `styles_stamp_at`, `staged_economy_body`) | component/state | file-I/O | itself `:282-330,1183` | exact |
| `src/overlay/apply_all.rs:182,239-240` | service | file-I/O | `InputPaths::relative_to` use | role-match |
| `src/overlay/screenshot.rs` (`HOME_ENV` import path, guard test) | utility | request-response | itself `:262-296` | exact |
| `tests/isolation_guard.rs` (new) | test | batch (source scan) | `tests/cli_errors.rs`, `tests/dynamic_filter.rs` spawn helpers | partial |
| first-run test (new, in `app.rs` tests or `tests/dynamic_filter.rs`) | test | file-I/O | `config_root()` rig `app.rs:3595-3612`; `apply()` helper `dynamic_filter.rs:67` | role-match |

## Pattern Assignments

### `src/locations.rs` (utility, request-response)

**Analog:** `src/paths.rs` (keep pure-core-plus-thin-wrapper; keep `HOME_ENV`, `RootError`)

**Imports + constants** (`paths.rs:18-22`):
```rust
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Environment variable that pins the FerrisFilter folder.
pub const HOME_ENV: &str = "FERRISFILTER_HOME";
```

**Error enum pattern** (`paths.rs:24-37`; drop `NotFound`, add `NoConfigDir` with a "set FERRISFILTER_HOME" help):
```rust
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum RootError {
    #[error("FERRISFILTER_HOME is set to `{path}`, which is not an existing directory")]
    HomeNotADirectory { path: String },
    #[error("Cannot find the FerrisFilter folder (the one holding styles-default.json): {tried}")]
    #[diagnostic(help("Set FERRISFILTER_HOME to the FerrisFilter folder, or keep ff.exe inside it."))]
    NotFound { tried: String },
}
```

**Core pattern: env branch** (`paths.rs:61-69`, keep verbatim; empty env counts as unset, set-but-not-dir is an error and never falls through):
```rust
if let Some(value) = env.filter(|v| !v.is_empty()) {
    let dir = PathBuf::from(value);
    if dir.is_dir() { return Ok(dir); }
    return Err(RootError::HomeNotADirectory { path: dir.display().to_string() });
}
```

**Thin wrapper** (`paths.rs:96-106`; the only env reader. New one must be the only `dirs::` caller and call it lazily, so make the platform lookup an `impl FnOnce() -> Option<PathBuf>`):
```rust
/// Never canonicalizes (`\\?\` paths would change every rendered message).
pub fn app_root() -> Result<PathBuf, RootError> {
    let exe = std::env::current_exe().ok();
    let cwd = std::env::current_dir().ok();
    resolve_from(std::env::var_os(HOME_ENV).as_deref(), exe.as_deref().and_then(Path::parent), cwd.as_deref())
}
```
New shape: `AppDirs { root: PathBuf }`; `resolve_from(home, platform: impl FnOnce()->Option<PathBuf>)`; `from_env()` calls `resolve_from(std::env::var_os(HOME_ENV).as_deref(), dirs::config_dir)`. Folder segment: `FerrisFilter` on Windows, `ferrisfilter` on Unix (`cfg!(windows)`). The per-user folder is neither required to exist nor created. Under `#[cfg(test)]`, `from_env` panics when `FERRISFILTER_HOME` is unset (RESEARCH section 6).

**Tests to keep/rewrite** (`paths.rs:117-148`): keep `env_dir_wins_without_marker`, `env_missing_dir_is_error_and_does_not_fall_through`, `env_pointing_at_file_is_error`, `empty_env_is_unset` (adapt the last to assert the per-user closure result). Delete walk-up/cwd/marker tests (`:150-225`). Add: closure NOT called when HOME is set (use a closure that panics or sets a flag).

Callers to repoint: `src/run.rs:181`, `src/overlay/app.rs:1183`, `src/overlay/app.rs:2159`, `src/overlay/screenshot.rs:265,295` (`HOME_ENV`). Either keep `paths` as a re-export shim or update all five.

---

### `src/defaults.rs` (config, transform)

**Analog:** embedded-const pattern at `src/loader.rs:283` and `:344`:
```rust
const TIERS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tiers-default.json"));
const STYLES: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/styles-default.json"));
```
Make these `pub const` for the six files: `styles-default.json`, `tiers-default.json`, `prices-fallback.json`, `tiers-fallback.json`, `config.json`, `mapping.json`. Also expose display labels such as `"styles-default.json (built in)"` for embedded parse errors. Caveat from `loader.rs:484`: `include_str!` returns on-disk bytes, which are CRLF under `core.autocrlf=true`; check how that note is handled before relying on byte equality with embedded text. Register `pub mod defaults;` and `pub mod locations;` in `src/lib.rs` next to `pub mod paths;` (`lib.rs:21`).

---

### `src/loader.rs` (service, file-I/O -> transform): add `*_str` twins

**Analog:** `load_json` `loader.rs:69-87`. Refactor so `load_json(path)` reads then delegates to `load_json_str(&content, name)`:
```rust
pub fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, LoadError> {
    let content = std::fs::read_to_string(path).map_err(|e| LoadError::Read { path: path.to_string(), source: e })?;
    let de = &mut serde_json::Deserializer::from_str(&content);
    serde_path_to_error::deserialize(de).map_err(|e| {
        let inner = e.inner();
        let offset = SourceOffset::from_location(&content, inner.line(), inner.column());
        JsonLoadError {
            file: path.to_string(),
            field_path: e.path().to_string(),
            cause: inner.to_string(),
            src: miette::NamedSource::new(path, content),
            location: offset,
        }.into()
    })
}
```
**Wrappers to twin** (`loader.rs:119-122, 220-225, 152-164`):
```rust
pub fn load_tiers(path: &str) -> Result<ValidatedTiers, LoadError> {
    let entries: Vec<TierEntry> = load_json(path)?;
    Ok(ValidatedTiers::try_from(entries)?)
}
pub fn load_styles(path: &str) -> Result<StyleSet, LoadError> {
    let mut set: StyleSet = load_json(path)?;
    set.assign_missing_ids();
    set.validate()?;
    Ok(set)
}
// load_config_with_live(path, styles_path, live_styles_path): load_json(path) then load_styles(styles_path),
// then check_legacy_styles(...) naming live_styles_path.
```
Add `load_styles_str`, `load_tiers_str`, `load_prices_str`, `load_config_str_with_live`. Note `load_prices_optional` (`:101-106`) uses `Path::exists()`; the embedded fallback needs no such check.

**`src/sync/mapping.rs`:** twin `load_mapping` (`:115-119`):
```rust
pub fn load_mapping(path: &str) -> miette::Result<Mapping> {
    let content = std::fs::read_to_string(path).map_err(|e| miette::miette!("Cannot read {path}: {e}"))?;
    serde_json::from_str(&content).map_err(|e| miette::miette!("Cannot parse {path}: {e}"))
}
```
as `load_mapping_str(content, label)`.

---

### `src/compile.rs` (model/service, file-I/O)

**Analog:** itself. Keep `conventional()` (`:87-89`) and `relative_to` (`:94-106`) unchanged so `generate` and ~60 tests stay byte-identical; add `InputPaths::from_dirs(&AppDirs)`.

**Struct** (`:63-77`):
```rust
pub struct InputPaths {
    pub config: PathBuf,
    pub styles: PathBuf,
    pub styles_default: Option<PathBuf>,   // None for explicit --styles override
    pub prices: PathBuf,
    pub tiers: PathBuf,
    pub tiers_default: Option<PathBuf>,
    pub prices_fallback: PathBuf,
    pub tiers_fallback: PathBuf,
}
```
Reshape per RESEARCH: `DefaultSource::{File(PathBuf), Embedded(&'static str, &'static str)}`, `FallbackPair::{Files, Embedded}`, config embedded variant. `with_styles` / `with_tiers` (`:119-141`) set the default to `None`; keep that "explicit path never falls back" semantic.

**Source selection to extend** (`:164-182`):
```rust
pub fn read_source(live: &Path, default: Option<&Path>) -> Result<PathBuf, LoadError> {
    let exists = |p: &Path| p.try_exists().map_err(|e| LoadError::Read { path: path_str(p), source: e });
    if exists(live)? { return Ok(live.to_path_buf()); }
    match default {
        Some(d) if exists(d)? => Ok(d.to_path_buf()),
        Some(d) => Err(LoadError::NoSource { path: path_str(live), default: path_str(d) }),
        None => Ok(live.to_path_buf()),
    }
}
```
An unreadable live file must stay a `LoadError::Read`, never fall back. Embedded default: return `Source::Embedded{label,text}` instead of `NoSource`.

**Consumers to adapt** (all route through the `Source`-aware `*_str` loaders):
- `load_economy` `:304-315`: `loader::load_styles(&path_str(&paths.styles_source()?))`.
- `load_priced_with` `:349-...`: fallback arms use `loader::load_prices_optional(&path_str(&paths.prices_fallback))` and `loader::load_tiers(&path_str(&paths.tiers_fallback))`; live arm uses `paths.tiers_source()`. `ForcedFallbackMissing` stays for the File pair only.
- `load_inputs` `:261-286`: generate path, leave file-based.

**Tests to keep, add Embedded twins** (`compile.rs:590-625`): `read_source_prefers_live`, `read_source_falls_back_to_the_default` (asserts the live file is never created), `read_source_with_neither_file_names_both`. Tests at `:476-503` assert exact `relative_to` paths and must still pass.

---

### `src/safe_write.rs` and `src/settings.rs` (utility, file-I/O): create the folder on first real write

**Analog `safe_write.rs:362-380`** (add `fs::create_dir_all(parent_of(path))` before `new_in`, mapped through the same `err`; only reached when something is really written, preserving "unedited file never written"):
```rust
fn stage(path: &Path, bytes: &[u8]) -> Result<tempfile::NamedTempFile, WriteError> {
    let err = |source: io::Error| WriteError::Unwritable { path: path.display().to_string(), source };
    let mut t = tempfile::NamedTempFile::new_in(parent_of(path)).map_err(err)?;
    t.write_all(bytes).and_then(|_| t.as_file().sync_all()).map_err(err)?;
    Ok(t)
}
```
Also the second `NamedTempFile::new_in(dest_dir)` at `safe_write.rs:177` (the `.ffbak`; dest is the game folder, which exists, so likely no change) and the `write_guarded` site mentioned in RESEARCH (`:240`).

**Analog `settings.rs:224-262`** (`save` and `save_section_state` each do `NamedTempFile::new_in(root)`; add `fs::create_dir_all(root)?` first in both):
```rust
pub fn save(root: &Path, settings: &Settings) -> io::Result<()> {
    let mut text = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    text.push('\n');
    let mut t = tempfile::NamedTempFile::new_in(root)?;
    t.write_all(text.as_bytes())?;
    t.as_file().sync_all()?;
    t.persist(path(root)).map_err(|e| e.error)?;
    Ok(())
}
```
Tests: write into a not-yet-existing subfolder succeeds; unchanged content creates nothing.

---

### `src/sync/mod.rs`, `src/sync/prices.rs` (service, request-response + file-I/O)

**Analog `sync/mod.rs:19-27`** (`cmd_sync` hardcodes `std::path::Path::new(".")`; switch to `AppDirs::from_env()?.root`):
```rust
rt.block_on(run(standard, hc, league, sub, std::path::Path::new(".")))?;
```
`sync_economy_blocking(base_dir)` (`:55-62`) already takes the dir; the overlay passes `root.clone()` (`app.rs:3139-3148`), so it needs no signature change.

**`prices.rs:461-463`** (mapping read becomes embedded; `base_dir` becomes output only):
```rust
let mapping_path = base_dir.join("mapping.json");
let mapping = load_mapping(&mapping_path.to_string_lossy())?;
```
**`prices.rs:~620`** `write_prices_atomic(&base_dir.join("prices.json")...)`: add `create_dir_all(base_dir)` before the temp file. `sync metadata` (`sync/metadata.rs:20,71`, `MAPPING_PATH`) stays on the cwd file; add a doc line that a rebuild is needed to embed the rewritten mapping. `tests/sync_prices.rs` does not call `run_sync_prices` with a temp dir, so the signature can change freely.

---

### `src/run.rs` `cmd_apply` (controller, request-response)

**Analog:** itself `run.rs:181-192` and `under_root` `:25-32`:
```rust
let root = paths::app_root().map_err(miette::Error::from)?;
let mut inputs = InputPaths::relative_to(&root);
if let Some(t) = tiers {
    eprintln!("note: --tiers {t} — using its cutoffs instead of tiers.json");
    inputs = inputs.with_tiers(under_root(&root, t));
}
if let Some(s) = styles {
    eprintln!("note: --styles {s} — using it instead of styles.json");
    inputs = inputs.with_styles(under_root(&root, s));
}
```
Replace first two lines with `AppDirs::from_env()` + `InputPaths::from_dirs(&dirs)`. Open decision A3: a relative `--tiers tiers-fallback.json` that does not exist under the folder, whose file name equals a built-in name, should fall back to the embedded text. The fallback warning text at `:196-200` mentions `prices-fallback.json`; keep it. `--filter` default stays `dynamic::default_filter_path` (Phase 37 owns it).

---

### `src/cli.rs` (config, help text)

Update `Apply` doc `:64-70` ("FERRISFILTER_HOME, else the folder holding ff-cli.exe, else the working directory" -> "FERRISFILTER_HOME, else %APPDATA%\FerrisFilter"), `--tiers` doc "relative path resolved against the FerrisFilter folder" and `--styles` doc "beside the inputs" `:91-93`; check `:24,30,84` mentions of the fallback files. `--config` default `config.json` (`:19`) is `generate`, unchanged. Add a `sync` help line that `prices.json` is written to the per-user folder.

---

### `src/overlay/config_state.rs` (state, file-I/O)

**Analog:** itself. `read_effective` `:125-140` returns `(PathBuf, Vec<u8>, Option<Vec<u8>>)`; change the first element to a `Source` (File or Embedded). Keep the single-read, race-handling shape:
```rust
fn read_effective(live: &Path, resolve: impl FnOnce() -> Result<PathBuf, LoadError>)
    -> Result<(PathBuf, Vec<u8>, Option<Vec<u8>>), String> {
    if let Some(bytes) = read_optional(live)? {
        return Ok((live.to_path_buf(), bytes.clone(), Some(bytes)));
    }
    let src = source(live, resolve())?;
    let bytes = fs::read(&src).map_err(|e| format!("{}: {e}", file_label(&src)))?;
    let live_bytes = (src == live).then(|| bytes.clone());
    Ok((src, bytes, live_bytes))
}
```
Embedded case: bytes are `text.as_bytes().to_vec()`, `live_bytes = None`.

**Load** (`:156-187`): `ids_unpersisted = styles.assign_missing_ids() && styles_live.is_some();` already means an embedded/default source is never dirty (pitfall 2); preserve.

**`file_label`** (`:107-111`) strips directories, but CONTEXT requires live-file errors to show the full per-user path:
```rust
fn file_label(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}
```
Use `path.display()` for live files and the bare default name for embedded; update tests asserting the short form (`app.rs:3720-3727`, grep `"styles.json:"`). Keep the race test near `config_state.rs:940`. `follow_disk` `:407-450` re-resolves per poll; it works once `read_effective` returns the new `Source`.

---

### `src/overlay/app.rs` (state, file-I/O)

**Analog:** itself. Replace `relative_to(root)` with `from_dirs`-style paths in:

`load_styles_at` / `load_classes_at` / `styles_stamp_at` (`:295-330`):
```rust
fn load_styles_at(root: &Path) -> Result<StyleSet, String> {
    let path = InputPaths::relative_to(root).styles_source().map_err(|e| e.to_string())?;
    loader::load_styles(&path.to_string_lossy()).map_err(|e| e.to_string())
}
fn styles_stamp_at(root: &Path) -> Option<(PathBuf, SystemTime)> {
    let path = InputPaths::relative_to(root).styles_source().ok()?;
    let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok()?;
    Some((path, mtime))
}
```
`load_classes_at` must use the embedded `config.json` always. `styles_stamp` (`:199`) becomes `Option<StylesStamp>` with `Embedded | File(PathBuf, SystemTime)`; creating or deleting the live file flips the variant, which is still detected. `staged_economy_body` (`:282`) uses `InputPaths::relative_to(root)`; switch to the embedded-default constructor.

`App::new` (`:1183`):
```rust
let root = crate::paths::app_root().map_err(|e| e.to_string());
```
Keep `root: Result<PathBuf,String>` so the status-line/disable flow (`:1704,1714,1785,2887,2900,3005,3076`) is unchanged; also `:2159` RELOAD. Guard ordering: `screenshot::guard_env` runs before `App::new` (`overlay/mod.rs:138`) and must stay so.

**Test rig analog** (`app.rs:3599-3612`) for the second, first-run rig (empty temp root, no copies):
```rust
fn config_root() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (src, dst) in [("styles-default.json", "styles.json"), ("tiers-default.json", "tiers.json")] {
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join(src), dir.path().join(dst)).unwrap();
    }
    dir
}
```
Tests to rewrite: `app.rs:3689-3770` (`assert_no_live`, `styles_stamp_switches_on_live_create_and_delete`, "neither exists" at `:3720`).

---

### `src/overlay/screenshot.rs` (utility, request-response)

**Analog:** `guard_env` `:262-286`; keep requiring `FERRISFILTER_HOME` (with the per-user fallback the temp `USERPROFILE` no longer protects `%APPDATA%`, since `dirs` ignores env vars on Windows, assumption A1):
```rust
pub(crate) fn guard_env() -> Result<(), String> {
    let home = std::env::var_os(crate::paths::HOME_ENV).ok_or_else(|| "screenshot mode needs FERRISFILTER_HOME set to a temp folder".to_string())?;
    let profile = std::env::var_os("USERPROFILE").ok_or_else(|| ...)?;
    let game_dir = game_dir()?;
    paths_under_temp(Path::new(&home), Path::new(&profile), &game_dir, &std::env::temp_dir())
}
```
Only the `crate::paths::HOME_ENV` path changes (`:265,295`). Keep the existing refusal test (`screenshot.rs:515`).

---

### `tests/isolation_guard.rs` (new, test, batch source scan)

**Analog for spawn style:** `tests/dynamic_filter.rs:64-74`:
```rust
fn apply(dir: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("ferrisfilter").expect("binary")
        .current_dir(dir)
        .env("FERRISFILTER_HOME", dir)
        .env("USERPROFILE", dir)
        .arg("apply").args(args).assert()
}
```
The new test scans `tests/*.rs` (`fs::read_dir` + `env!("CARGO_MANIFEST_DIR")`) for `cargo_bin(` sites whose subcommand is `apply`/`sync` and requires `.env("FERRISFILTER_HOME"` in the same call chain; asserts `dirs::` appears only in `src/locations.rs`; bans `set_var`/`remove_var` in tests. Spawn sites: `dynamic_filter.rs:67,134,324,368`, `cli_errors.rs:~331`, `cli_filter_copy.rs:43-88`, `cli_generate.rs`, `sync_prices.rs`. `generate`/`install`/`--help` spawns are cwd-relative and exempt. Do not add `[env] FERRISFILTER_HOME` to `.cargo/config.toml`.

**First-run test (LOC-01/02):** empty temp home through the `from_dirs` rig: `ConfigState::load`, class catalog, and `economy_body` (fallback prices) all succeed; Apply writes only the filter (unedited `styles.json`/`tiers.json` are not written); an `AppDirs` pointing at a not-yet-existing `temp/sub` gets created by the first real write. `tests/dynamic_filter.rs` `workspace_defaults` (`:55`, copies `*-default.json` into HOME) should now assert the embedded defaults give identical output.

## Shared Patterns

### Pure core plus thin env wrapper
**Source:** `src/paths.rs:56-106`. **Apply to:** `locations.rs`; anything new that needs the OS. Only `AppDirs::from_env` may call `dirs::config_dir()`; everything else takes `&AppDirs` or a `&Path`. Never canonicalize.

### Live-else-default read (DEF-03)
**Source:** `src/compile.rs:164-182` and `config_state.rs:125-140`. **Apply to:** styles and tiers only (`config.json`/`mapping.json` are always built in). Unreadable live file is an error naming that file, never a silent fallback; reads never create files.

### Atomic write, folder created only at real write time
**Source:** `safe_write.rs:370-380`, `settings.rs:224-232`, `sync/prices.rs write_prices_atomic`. **Apply to:** every writer into the per-user folder (`create_dir_all` then `NamedTempFile::new_in` then persist).

### Loader error shape
**Source:** `loader.rs:69-87` (`JsonLoadError` with `NamedSource`). **Apply to:** all `*_str` loaders; embedded label e.g. `styles-default.json (built in)`, live files keep the full path.

### Embedding
**Source:** `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/<file>"))` (`loader.rs:283,344`, also `emitter.rs:749`, `pricer.rs:466`, `sync/mapping.rs:232`, `sync/prices.rs:1171`). **Apply to:** `defaults.rs`.

### Test isolation
**Source:** `app.rs:3595-3612` temp rigs; per-`Command` `.env("FERRISFILTER_HOME", dir)`. **Apply to:** all new tests; none may call `AppDirs::from_env()` or mutate the process env.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `tests/isolation_guard.rs` source-scan test | test | batch | No existing test scans source files; use `std::fs` + `env!("CARGO_MANIFEST_DIR")` as in the `include_str!` tests |
| `StylesStamp` enum (`Embedded | File`) | model | New concept; only the current `Option<(PathBuf, SystemTime)>` tuple exists |

## Metadata

**Analog search scope:** `src/` (paths, compile, loader, safe_write, settings, run, cli, sync, overlay/app, config_state, screenshot), `tests/`, `Cargo.toml`.
**Files scanned:** about 20 read or grepped.
**Pattern extraction date:** 2026-10-17
