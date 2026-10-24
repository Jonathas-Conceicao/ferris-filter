# Architecture Patterns: v0.7 Integration

**Domain:** Rust CLI + egui overlay (FerrisFilter), v0.7 "Initial Public Release and Strictness Selector"
**Researched:** 2026-10-17
**Mode:** Subsequent milestone, integration research against the existing code
**Overall confidence:** HIGH on integration points (read from the code), MEDIUM on release/installer tooling (not verified against external docs here; flagged below)

## Summary

Almost every v0.7 feature funnels through ONE seam that already exists: `InputPaths` (`src/compile.rs:73-165`) plus the app-root resolver (`src/paths.rs:98`). Today `InputPaths::relative_to(root)` fabricates seven file paths from a single directory. Per-user config needs two directories (config, cache) and embedded defaults, so the single-`root` assumption is the main thing to break. Do that first; the game-folder setting, league/price-source setting and updater all hang off the new "locations" layer. The Strictness Selector and the style rework are independent of that layer and touch only CONFIG state and `styles-default.json`/fixtures.

Key discovery: the overlay does not read only `styles`/`tiers`/`prices`. It also reads **`config.json`** (class catalog, `src/overlay/app.rs:308`, built from `paths.config`) and the sync reads **`mapping.json`** from `base_dir` (`src/sync/prices.rs:462`). A shipped exe must embed those too, not just the `-default.json` files. Missing from the milestone text.

## Current Facts (verified in code)

| Fact | Where |
|------|-------|
| App root = `FERRISFILTER_HOME` (must exist) else walk up from exe to dir holding `styles-default.json`, else cwd, else error | `src/paths.rs:55-109` |
| `InputPaths` has config, styles(+default), prices, tiers(+default), prices_fallback, tiers_fallback, all joined on one root; `styles_source()`/`tiers_source()` do live-else-default via `read_source` | `src/compile.rs:73-165, 168-200` |
| Fallback pair is chosen in `load_priced_with` by `PriceSourcePolicy::{Auto, ForceFallback}`; `Auto` falls back when prices absent/empty | `src/compile.rs:186-195, 340-412` |
| Game folder default hard-coded `%USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2` in ONE function, re-exported by `dynamic::default_destination` | `src/install.rs:13-20`, `src/dynamic.rs:34` |
| `default_destination()` consumers: CLI `apply` default `--filter` (`src/run.rs:~150-165`), `install` (`src/install.rs:36`), overlay game dir (`src/overlay/app.rs:~917, 3248`, `filter_target`), screenshot guard (`src/overlay/screenshot.rs:279`) | |
| Settings = `ff-settings.json` under the app root; typed fields `filter`, `reload_after_apply`, `collapsed_sections`, plus `#[serde(flatten)] extra` (unknown keys preserved) | `src/settings.rs:24-39, 117` |
| Overlay league is `sync::OVERLAY_LEAGUE` const; passed to `run_sync_prices(.., Some(OVERLAY_LEAGUE), base_dir)`; also fed to the Economy tab as `league:` | `src/sync/mod.rs:51-62`, `src/overlay/app.rs:2996` |
| `fetch_index_state` already returns the full league list (name, url, hardcore, indexed) | `src/sync/league.rs:93`, `src/sync/models.rs` |
| Sync runs on `EconomySync` worker (start/poll/settle, panic-safe, single writer) | `src/overlay/economy.rs:83-178`; started at `src/overlay/app.rs:3138-3147` |
| Economy tab info reads `<root>/prices.json` (`EconomyInfo::read`) | `src/overlay/app.rs:847-882` |
| `build.rs` bakes `FF_VERSION` (git describe) and repo URL into the exe; falls back to `CARGO_PKG_VERSION` | `build.rs`, `src/build_info.rs` |
| Quit path: tray Quit -> `quit_dirty()` -> Unapplied-changes popup -> `QuitChoice` -> `quit_step`; `quit_after_apply` waits for the in-game reload worker | `src/overlay/app.rs:108-118, 1882-1910, 2699-2720`, `src/overlay/apply.rs` |
| Screenshot guard requires `FERRISFILTER_HOME`, `USERPROFILE`, and the game dir derived from `USERPROFILE` all under temp | `src/overlay/screenshot.rs:225-285` |
| HTTP client user agent is hard-coded `FerrisFilter/0.1 ...` | `src/sync/client.rs:28` |
| No `.github/`, no installer dir; `repository` in Cargo.toml is `.../ferris-filter` (the user agent says `poe2_myfilter`; reconcile before building the update checker URL) | repo root |
| Many tests resolve repo files via `CARGO_MANIFEST_DIR` + `-default.json` / `config.json` (`loader.rs:344`, `emitter.rs:698`, `models/config.rs:938,1516,2141`, `overlay/body.rs`, `classes.rs`) | |

## Recommended Architecture

### Layer 1 (new): `locations` module

Replace "one root" with a small struct, resolved once at startup and passed where `root: PathBuf` is passed today.

```
AppDirs {
    config_dir: PathBuf,   // styles.json, tiers.json, ff-settings.json   (%APPDATA%\FerrisFilter | ~/.config/ferrisfilter)
    cache_dir:  PathBuf,   // prices.json (+ downloaded installer under cache_dir/updates)  (%LOCALAPPDATA%\FerrisFilter | ~/.cache/ferrisfilter)
    source: Home | Platform   // Home = FERRISFILTER_HOME override, both dirs = that one folder
}
```

- Resolution order: `FERRISFILTER_HOME` (existing rule: must be an existing directory, no silent fall-through) -> both dirs = that folder (exactly today's dev behaviour, so tests/screenshot mode/dev runs stay byte-compatible) -> else platform dirs. Create dirs lazily on first write only (current code "never creates a file" on read; keep that).
- Use the `dirs` crate (or `directories`) for `config_dir()`/`cache_dir()`; for the game folder use the Windows Known Folder API via `dirs::document_dir()` (MEDIUM confidence: `dirs` uses `SHGetKnownFolderPath(FOLDERID_Documents)` on Windows, which handles OneDrive redirection and localized names; verify in the crate docs in the phase). `windows-sys` is already a dependency (Cargo.toml:49) and could call `SHGetKnownFolderPath` directly with the `Win32_UI_Shell` feature if the author prefers no new crate. Recommendation: `dirs` (tiny, well-known), cfg-free.
- `paths::app_root()` becomes `paths::app_dirs() -> Result<AppDirs, RootError>`. The walk-up-from-exe + cwd + `styles-default.json` marker logic is DELETED for the shipped path (an installed exe has no repo around it). Keep a dev convenience only if wanted: it is replaced by `FERRISFILTER_HOME=<repo>` (the dev-warning in CLAUDE.md already recommends a temp home). Decision for roadmap: drop the walk-up entirely (simpler, removes the `is_root` marker concept and its ~15 tests in `paths.rs`); dev runs set `FERRISFILTER_HOME` or just use the per-user dir.
- `RootError::NotFound` mostly disappears: platform dirs always resolve; failure = `dirs` returns `None` (rare), surfaced like today's `self.root: Err(String)` which already drives a status message and disables Sync/Apply (`app.rs:1183, 2159, 3138`).

### Layer 2 (modified): `InputPaths` + embedded defaults

Do NOT keep `styles_default: Option<PathBuf>` pointing at a file. Introduce an embedded-default source:

```rust
// new: src/defaults.rs
pub const STYLES_DEFAULT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/styles-default.json"));
pub const TIERS_DEFAULT:  &str = include_str!(".../tiers-default.json");
pub const PRICES_FALLBACK:&str = include_str!(".../prices-fallback.json");
pub const TIERS_FALLBACK: &str = include_str!(".../tiers-fallback.json");
pub const CONFIG_DEFAULT: &str = include_str!(".../config.json");
pub const MAPPING:        &str = include_str!(".../mapping.json");
```

- `read_source(live, default)` (`compile.rs:171`) returns a `PathBuf`; callers then `load_*(path_str)`. Change its result to an enum `Source { File(PathBuf), Embedded(&'static str) }` and give `loader` string-based entry points (`load_styles_str`, `load_tiers_str`, `load_prices_str`). `loader::load_styles(path)` etc. stay as thin wrappers so the ~30 `repo_file(...)`-based tests and `--styles`/`--tiers` explicit-path semantics ("no fallback") are untouched. The files remain the committed, tested source; `include_str!` just snapshots them at build time, so `styles-default.json` etc. stay the single source of truth and `artifact_guard` keeps reading the repo files (no behaviour change for tests).
- `InputPaths` gets constructed two ways: `relative_to(dir)` (kept: CLI/tests/dev; defaults `Some(embedded)` or file as today) and `from_dirs(&AppDirs)` (config files in config_dir, `prices` in cache_dir). Since `relative_to` currently resolves defaults from the same dir, tests that stage `styles-default.json` into a temp dir (`compile.rs:591-650`, `app.rs:3602`, `apply_all.rs:439`) either keep working (file defaults retained when constructed via `relative_to`) or get simplified. Lowest risk: keep a `DefaultSource::{File, Embedded}` field in `InputPaths`; `relative_to` -> `File` (unchanged tests), `from_dirs` -> `Embedded`.
- Fallback pair (`prices_fallback`, `tiers_fallback`): same treatment, they are always embedded in the shipped app (no live copy exists, they are never written). `load_priced_with` (`compile.rs:340-412`) reads them via the Source enum.
- `config.json`: used by the overlay only for class groups (`app.rs:308`, `classes.rs`) and by `generate`. Shipped overlay: load embedded `CONFIG_DEFAULT` for the class catalog; `generate` keeps taking `--config` (default: cwd `config.json`, see CLI below). The load validates config styles against the style set (`loader::load_config(config_path, styles_path)`), so a string-based `load_config_str(config, styles_str)` is needed. Note: user rename of a custom style used by `config.json` already warns (CLAUDE.md caveat); with embedded config this only matters for `generate`.
- `mapping.json`: `run_sync_prices` reads `base_dir/mapping.json` (`prices.rs:462`). Embed it (`MAPPING`) and add a `base_dir`-less path for the overlay/CLI sync; `sync metadata` (the developer command that rebuilds `mapping.json` from RePoE exports) keeps writing the repo file. **Pitfall:** tests like `tests/sync_prices.rs` pass a temp `base_dir` containing a mapping; keep an optional override parameter.

### Layer 3: CLI path semantics (how generate/sync/install change)

Current: `generate`, `sync`, `install` are cwd-relative on purpose ("Deliberate asymmetry", `paths.rs:16`, CLAUDE.md `apply` row); `apply` and the overlay use the app root. v0.5 audit listed this as tech debt (W1). v0.7 resolves it naturally:

| Command | Change | Rationale |
|---------|--------|-----------|
| `apply` | inputs from `AppDirs`; `--filter` default = `<settings game folder>/<settings filter name>` (read from `ff-settings.json` in config_dir), else OS Documents default | one configured game folder shared with the overlay; no more OneDrive hard-code |
| `sync` | writes `prices.json` to `cache_dir` (not cwd); `mapping` embedded; `--league` unchanged. Add a one-line note when `FERRISFILTER_HOME` unset that output goes to the cache dir | this is the fix for the W1 asymmetry: sync output is read by apply, so they must agree |
| `generate` | KEEP cwd-relative for `config.json` and `--output` (it is a developer/author command that compiles a repo config; its fixtures/`cli_generate.rs`/`artifact_guard` depend on cwd = repo). Styles/tiers/prices: continue to use `InputPaths::conventional()` (bare relative names, `styles.json` -> `styles-default.json` file fallback) when run from the repo. | zero risk to the byte-for-byte baseline; `generate` is not part of the end-user installer story |
| `install` | `--dest` default from the settings/OS-Documents resolver | removes the second copy of the OneDrive path |

Roadmap decision to surface: whether `generate` should also fall back to per-user dirs when no `config.json` is in cwd. Recommend NOT in v0.7 (keeps `cli_*` tests and byte-for-byte contract untouched); document that `generate` is a repo/dev command.

### Layer 4 (modified): game folder setting

- Store: new typed field `Settings::game_folder: Option<String>` in `ff-settings.json` (`src/settings.rs:24`), serialised with `skip_serializing_if = "Option::is_none"` so existing files and the unknown-key-preserving flatten are untouched. A relative or non-existent value is validated like the filter name (`valid_filter_name` precedent, `settings.rs:309`): must be absolute and, at use time, an existing dir. Same fail-closed philosophy already used for `reload_after_apply` (invalid value => visible warning).
- New resolver `game::folder(settings: &Settings) -> Option<PathBuf>`: setting if valid, else OS Documents + `My Games\Path of Exile 2` (via `dirs::document_dir`), else `None`. `install::default_dest` and `dynamic::default_destination` (`dynamic.rs:34`) are replaced by it; the `USERPROFILE` assumption and the 3 error strings "Cannot resolve the default PoE2 config folder (USERPROFILE is not set)" (`app.rs:917,3248`, `run.rs:159`, `install.rs:40`) change wording.
- Overlay: `filter_target(game_dir, name)` (`app.rs:~905`) already takes `game_dir: Option<&Path>`, so the call-site change is confined to where the `game_dir` field is computed at startup and in RELOAD (`settings_with_filter`/RELOAD keep-settings, `app.rs:~935`). Changing the folder = same flow as switching filter: if rules are dirty, reuse the `Switch filter?` confirm (`switch_decision`, `app.rs:~890`); otherwise apply at once, save settings, re-list filters (`refresh_filters`), reload the working filter. The filter name should stay if it exists in the new folder, else fall back to default.
- Picker UI: Config tab "Filter" section (`config_view.rs`, 2702 lines) gets a "Game folder" row: text of the path + a **Browse** button. Native folder dialog needs a crate (`rfd`, MEDIUM confidence it fits an eframe app; works without extra runtime on Windows). A dialog blocks the UI thread: run it on a short worker thread polled per frame (same pattern as `EconomySync`) or use `rfd::AsyncFileDialog`. Also offer a typed path field with validation so the feature degrades without a dialog.
- Screenshot guard: `screenshot::guard_env` derives the game dir from `USERPROFILE` (`screenshot.rs:279`). With the Documents lookup via the Known Folder API, `USERPROFILE` no longer determines the game folder on Windows (the API ignores the env var), which would BREAK the guard's premise and could write staged filters into the real Documents folder. Fix: in screenshot mode the game folder must come from the settings in the temp `FERRISFILTER_HOME` (stage `game_folder` into the temp home's `ff-settings.json`, or an explicit `FERRISFILTER_GAME_DIR` env that the resolver honours first and the guard checks). Recommend adding `FERRISFILTER_GAME_DIR` as the first resolution step (also useful for tests and CI) and having the guard require it under temp. This is the highest-risk regression of the milestone; test it first.
- `ff-settings.json` location moves with `AppDirs` (config_dir), so the guard's `FERRISFILTER_HOME`-under-temp check still covers settings, styles, tiers, prices (all collapse into the home folder under override).

### Layer 5 (modified): league + price source settings

Two new settings in `ff-settings.json`: `league: Option<String>` (display name; slug derivable from the index list) and `price_source: Option<"live"|"fallback">` (absent = live). Both are Economy-tab choices, saved on change like `reload_after_apply` (`settings.rs:224 save`), not part of the all-or-none Apply (they gate sync and pricing, not the filter file) -- but the pricing change takes effect only on Apply, consistent with Sync.

Data flow changes:

1. League list: new `sync::list_leagues_blocking() -> Result<Vec<League>>` wrapping `fetch_index_state` (`league.rs:93`) on a worker, same pattern as `EconomySync` (reuse or generalise `EconomySync` into a small `Worker<T>` helper; the start/poll/panic-safe scaffold at `economy.rs:83-178` is exactly what the update checker also needs, so extract it in this milestone rather than copy it three times). Cache the list in app state; refresh on entering the Economy tab (like `refresh_economy_info`, `app.rs:3163`) and on a "refresh" click. Fall back to the saved league string if offline.
2. Replace `sync_economy_blocking(base_dir)` (`sync/mod.rs:55`) with `sync_economy_blocking(cache_dir, league: &str)`. Delete `OVERLAY_LEAGUE`; `resolve_league_with_override` (`league.rs:64`) already accepts name-or-slug, so the existing "pinned league fails loudly with the available list" behaviour carries over. Default when nothing is chosen: do NOT silently auto-pick via the flapping `indexed` flag (the documented reason for the pin). Require an explicit choice before Sync enables, or default to the first non-HC entry of the list with the name shown in the success line (as today). Recommend: preselect from the list, display it, and require one click confirmation the first time (saved afterwards).
3. Price source = fallback: `economy_body_with(paths, policy, styles, tiers)` already takes `PriceSourcePolicy` (`dynamic.rs:254`), and `Auto` vs `ForceFallback` already exists (`compile.rs:189`). Only plumbing is needed: the overlay stores a `PriceSourcePolicy` derived from the setting and passes it at every call site that currently hardcodes `Auto` (grep `PriceSourcePolicy::Auto` in `overlay/apply*.rs`, `dynamic.rs:467-602` are tests). Note `ForceFallback` on `ForcedFallbackMissing` error text says `--early-game`; with embedded fallback it can no longer be missing, so that arm becomes unreachable in the shipped path.
4. Sync disabled when price source = fallback: `EconomyTabInputs.sync_enabled` (`app.rs:3001`, `shell::economy_tab`) gets one more condition and a tooltip (`sync_disabled_tip`).
5. Tier editor interaction: CLAUDE.md / audit W3 already states "CONFIG tier edits have no effect while fallback prices are in use". With fallback now a first-class user choice this stops being an edge case. Decide in the phase: when source = fallback the Tiers editor edits `tiers-fallback.json` semantics? The fallback tiers are exalt-denominated and embedded (read-only), so the cutoff editor cannot write them. Simplest honest behaviour: disable (grey with tooltip) the Tiers section while fallback is selected. Flag for the author.
6. `EconomyInfo::read` (`app.rs:861`) reads `<root>/prices.json`; it must read `cache_dir/prices.json` and report "Price source: Fallback (chosen)" distinct from "Fallback (no live prices)". Add a `PriceSource` surface on the tab: v0.5 debt item "surface price_source" folds in for free.
7. `sync` HTTP user agent should carry the real version (`client.rs:28` says `0.1`); trivial, do with the updater since it shares the client.

### Layer 6 (new): Strictness Selector (Config tab)

Pure derived state, no new persisted data.

- Definition: tiers S..F in `styles` map to visible/hidden by their `action`. The preset N = number of lowest tiers hidden: Flat Broke hides nothing; Scrounging hides F; Getting By hides E,F; Comfy D-F; Loaded C-F; Div Farmer B-F. (As written in PROJECT.md; "S-F" means styles S,A,B,C,D,E,F.)
- Where: a pure function in `src/overlay/config_state.rs` (1525 lines, owns pending style set `ConfigState`):
  - `fn strictness(styles: &StyleSet) -> Strictness` -> `Custom` unless the visibility pattern of [S,A,B,C,D,E,F] is exactly "all Show except a suffix of hidden tiers of length 0..=5" (Show/Hide only; `action` field).
  - `fn set_strictness(&mut self, s: Strictness)` sets the `action` of the relevant built-ins to Show/Hide as ONE edit on the pending set (revertible, marks Config dirty, goes through the same undo/Revert path as the existing per-style visibility switch at `config_state.rs`). Selecting Custom is a no-op (it merely labels a state; it cannot be selected into unless pattern is non-matching; render it as non-interactive/selected when derived).
  - Because it reads the PENDING set, the row updates live as the user flips the individual style switches in the table (and vice versa), exactly as the requirement states: "derived from the S-F styles' visibility; a hand edit shows Custom".
- Edge: a built-in that is Show but is a `hidden` style does not count; only S..F matter. What to do with patterns like "hide only D" (non-suffix): Custom. What about "hide S"? Custom. Fully-hidden (all hidden) is also Custom (not in the list).
- View: new `strictness_row` in `config_view.rs` above the style table (Styles section), one row of exclusive buttons using the existing skin segmented switch (`skin::editor` segmented switch) -- seven labels in a 660pt window needs measurement: likely two-line or smaller font; verify in screenshot mode early (cosmetic risk, not architectural).
- Economy effect: the rewrite chain already maps style action -> `Show`/`Hide` for economy blocks and styled user rules at Apply (CLAUDE.md: "a preset-styled user rule shows pending style's Show/Hide"), so the selector needs no emitter change. Verify tier-keyed economy blocks use the tier style, not a tier name check.
- Tests: pure-function table tests (all 7 patterns + a handful of non-matching), no UI needed.

### Layer 7 (modified): built-in style rework

- `styles-default.json` edited (author supplies details before planning). Built-ins "no longer use custom sound files": S and A currently use `CustomAlertSound "uwu.mp3" 300`; this swaps them to built-in `PlayAlertSound` ids. Consequences, all mechanical but wide:
  - Fixture `tests/fixtures/seed-universal.filter` regenerated (deliberate generator-output change; `artifact_guard`, `generate_snapshot` insta snapshots in `tests/snapshots`, `parity-baseline-earlygame.filter` if it embeds styles). One consolidated re-baseline phase/plan (v0.3 precedent: "One consolidated fixture re-baseline at the last emitter-output phase").
  - Many unit tests include `styles-default.json` and assert literal text; grep for `uwu` and for assertions over style colours in `body.rs`, `style_lines.rs`, `styled.rs`, `config_state.rs`, `emitter.rs`.
  - `assets/uwu.mp3` and the CLAUDE.md note about copying it into the game folder become obsolete; the sound-file listing feature (`sound_files.rs`) is unaffected.
  - Existing users who have a live `styles.json` from earlier versions keep their old styles (the live file wins over the embedded default). Document in CHANGELOG; there is no reset button. Not a blocker; a possible later "Reset to defaults" is out of scope.
  - The Strictness Selector assumes S..F built-ins keep names/ids `builtin:S`.. (`builtin:<name>` ids are load-validated), so the rework must not rename them.
- Ordering: rework must land BEFORE embedding is finalised? No: embedding reads the file at build, so order is independent, but doing the rework first avoids regenerating fixtures twice if the embedding refactor touches test constants. Recommended: rework early (cheap, isolated), with the single fixture re-baseline there.

### Layer 8 (new): Check for updates (About tab)

- New module `src/update.rs` (pure logic + a blocking network function) and a small state machine in the overlay, mirroring `EconomySync`:
  - `UpdateCheck` states: `Idle | Checking | UpToDate | Available{version, installer_url, size} | Downloading | Ready{path} | Failed(msg)`.
  - Worker thread (use the generalised worker helper extracted in Layer 5): GET `https://api.github.com/repos/<owner>/<repo>/releases/latest` (unauthenticated, 60 req/h per IP: fine for a manual button; set a `User-Agent` header, GitHub rejects requests without one; reuse `ReqwestClient`). Compare tag (`v0.7.0`) with `build_info` version via `semver`-style comparison: the baked `FF_VERSION` can carry a `-dev` suffix (`build.rs`); a dev build must compare by core version and never offer a "downgrade". Parse the asset named like `FerrisFilter-Setup-<ver>.exe` (decide the name in the CI phase and hard-code the match prefix; ignore the zip).
  - Download to `cache_dir/updates/<file>` (not the system temp, so a failed cleanup does not leave junk across reboots; the task said "temp": cache_dir is the sanctioned per-user temp-like location and is covered by the home override for tests). Stream to `<file>.part`, rename on completion; verify size equals the API `size`. MEDIUM-confidence consideration: add a SHA-256 check by publishing a `.sha256` asset in CI and verifying it; unsigned installer + SmartScreen makes integrity checking worth the ~30 lines (author decision; `sha2` crate).
  - Security: the updater downloads and executes code. Only follow the `browser_download_url` on `github.com`/`githubusercontent.com` hosts of the configured repo; refuse redirects off-host; never run without an explicit click ("offers to run it").
  - Launch: `std::process::Command::new(installer).spawn()` detached (Windows: `CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS` via `CommandExt::creation_flags`), then trigger quit. "Offers to run it" = a button **Install and quit** in the About tab after download.
- Quit integration: reuse the tray-Quit machinery. Today `quit_dirty()` -> `Popup::Quit` -> `apply::quit_step` (`apply.rs`, `app.rs:1882-1910`). Add `QuitReason::Update(installer_path)` so the same popup (Apply & quit / Quit without applying / Cancel) gates it, and on the final "QuitNow" the app first spawns the installer then performs the existing shutdown order (give up hotkey, tray icon, single-instance name at once, per the CLAUDE.md relaunch note so the installer's relaunch/`[Run]` works). Spawn the installer AFTER releasing the single-instance name and before process exit, otherwise an installer that launches `ff.exe` at the end hits "already running". Also the installer must handle a running `ff.exe` (Inno Setup `CloseApplications` / mutex; see installer section) because the user can also run the installer by hand.
- About tab (`about_view.rs`: `about_tab(ui, skin)` is currently stateless, 2 params): needs access to app state; change the signature to take `&mut UpdateUiState` and return an action enum (`CheckUpdates`, `InstallAndQuit`), following how `economy_tab` returns `economy_clicks` (`app.rs:2995-3014`). Keep the "not texty" UI guidance (memory: overlay UI not texty): one button + one status line (use the global status bar `ff_status` rather than adding text blocks on About).
- Screenshot mode: never performs network; add `--update-state available|uptodate|failed` debug flag only if About screenshots are needed in the README (Release Review step 2 retakes screenshots), compiled out of release like the others.
- Dev builds: `FF_VERSION` is `-dev` between tags; the checker should say "development build" rather than offering to downgrade.

### Layer 9 (new): release CI and installer

Files (new, nothing exists today):

| Path | Purpose |
|------|---------|
| `.github/workflows/release.yml` | on `push: tags: ['v*']`: checkout with `fetch-depth: 0` (build.rs runs `git describe --tags`, needs tags/history) -> toolchain from `rust-toolchain.toml` -> `cargo test` -> `cargo build --release` -> build installer -> zip -> extract CHANGELOG section -> `gh release create` |
| `.github/workflows/ci.yml` (optional) | PR/push: fmt, clippy baseline, `cargo test` on windows-latest (only if wanted; milestone text asks only for the tag workflow) |
| `installer/ferrisfilter.iss` (Inno Setup) or `installer/wix/` | Start menu shortcut for `ff.exe`; per-user install (`PrivilegesRequired=lowest`, `{autopf}` -> `{localappdata}\Programs\FerrisFilter`) so no UAC, which also makes self-update from the app painless; close running instance; optional "Run FerrisFilter" |
| `scripts/changelog-section.ps1` (or inline) | Extract `## [X.Y.Z]` section from `CHANGELOG.md` for the release notes |

Recommendations:
- Installer: Inno Setup, not WiX/MSI (MEDIUM confidence; widely used, single script, preinstalled on GitHub's windows runners via `choco install innosetup` or `iscc` already present; verify on the runner image in the phase). It supports `/SILENT /CLOSEAPPLICATIONS`, which the updater could pass later. WiX is heavier for "basic installer with a Start menu entry".
- What ships in the installer: `ff.exe`, `ferrisfilter.exe`/`ff-cli.exe` (optional; the milestone says Start menu entry for `ff`), LICENSE, README. NOT config/styles/tiers (embedded). `assets/poe2/*` and fonts are `include_bytes!` already (CLAUDE.md `src/skin` row), so a single exe suffices; confirm no runtime asset reads remain (`grep -rn "assets/" src`).
- Version: the three version sources agree if the tag `vX.Y.Z` equals `Cargo.toml` version; PROJECT.md already makes "version set in Release Review" and `/gsd-complete-milestone` checks tag == Cargo.toml version. CI should add a hard check step (`cargo metadata` version == tag minus `v`) and fail the release otherwise, and pass the version to Inno via `/DAppVersion=...`. In CI, the checkout's `git describe` will be exactly the tag so `FF_VERSION` is clean (no `-dev`).
- Note the repository URL inconsistency (`Cargo.toml` `ferris-filter` vs the user agent `poe2_myfilter` vs memory/remote): `build.rs` prefers the `origin` remote; in CI `origin` is the Actions checkout URL (`https://github.com/<owner>/<repo>`), fine, but the update checker needs a stable constant `owner/repo` -- single const in `build_info.rs`, sourced from `CARGO_PKG_REPOSITORY`, and the author must confirm it is the final public repo name before the first tag.
- Windows GUI subsystem is already set for release `ff` (CLAUDE.md `ff` row). No signing (out of scope): installer shows SmartScreen; document in README.
- Draft-then-publish: create the release as draft, upload artifacts, publish last, so a failed build never leaves a published release with missing assets that the update checker would then see.
- Workflow permissions: `contents: write` only on the release job.

## Component Boundary Table

| Component | Status | Responsibility | Communicates With |
|-----------|--------|----------------|-------------------|
| `src/locations.rs` (name TBD; replaces `paths.rs` resolver) | NEW/REPLACE | `AppDirs` (config_dir, cache_dir), `FERRISFILTER_HOME` override, game-folder resolution (setting -> Documents) | everything below |
| `src/defaults.rs` | NEW | `include_str!` of styles/tiers defaults, fallback pair, config, mapping | `compile.rs`, `loader.rs`, `sync/prices.rs`, overlay classes |
| `compile.rs` `InputPaths`/`read_source` | MODIFIED | add `Source::{File, Embedded}`, `from_dirs`, keep `relative_to` for CLI/tests | `loader.rs`, `dynamic.rs`, `run.rs`, overlay |
| `loader.rs` | MODIFIED | add `*_str` loaders; path loaders delegate | `compile.rs` |
| `settings.rs` | MODIFIED | add `game_folder`, `league`, `price_source` typed fields; keep flatten; same fail-closed validation | overlay `app.rs`, `run.rs` (apply default filter) |
| `install.rs` / `dynamic::default_destination` | MODIFIED | delegate to game-folder resolver; message text | `run.rs`, overlay |
| `run.rs` / `cli.rs` | MODIFIED | apply default `--filter`, sync -> cache_dir, install default; help strings with the OneDrive path updated | `locations`, `sync`, `dynamic` |
| `sync/mod.rs`, `sync/prices.rs`, `sync/league.rs` | MODIFIED | delete `OVERLAY_LEAGUE`; `sync_economy_blocking(cache_dir, league)`; `list_leagues_blocking`; embedded mapping | overlay economy worker |
| `overlay/economy.rs` | MODIFIED/GENERALISED | extract generic `Worker<T>` used by sync, league list, update check | `app.rs` |
| `overlay/app.rs` (5656 lines) | MODIFIED | hold `AppDirs`+`Settings`; game folder state; league list; price source policy into Apply; update state; quit reason | all overlay modules |
| `overlay/config_state.rs` | MODIFIED | `Strictness` derive + set on pending style set | `config_view.rs` |
| `overlay/config_view.rs` | MODIFIED | Game folder row + Browse; Strictness row | `app.rs` |
| `overlay/shell.rs` (`economy_tab`, `EconomyTabInputs`) | MODIFIED | league dropdown, price-source switch, sync disabled state | `app.rs` |
| `overlay/about_view.rs` | MODIFIED | Check for updates button/status, Install and quit | `app.rs` |
| `src/update.rs` | NEW | GitHub releases query, version compare, download, launch | `sync/client.rs`, `build_info.rs`, `app.rs` |
| `overlay/screenshot.rs` | MODIFIED | guard must cover the new config/cache/game folder resolution | `locations` |
| `styles-default.json`, fixtures, snapshots | MODIFIED | style rework + re-baseline | tests |
| `.github/workflows/release.yml`, `installer/*.iss` | NEW | release pipeline | tags, CHANGELOG.md |

## Data Flow (after v0.7)

```
startup:  FERRISFILTER_HOME? -> (home, home)  else (APPDATA\FerrisFilter, LOCALAPPDATA\FerrisFilter)
          settings.json (config_dir) -> game_folder?, filter, league?, price_source?, reload_after_apply
          game_dir = FERRISFILTER_GAME_DIR? > settings.game_folder > Documents\My Games\Path of Exile 2

Config pending set:  styles.json|embedded default + tiers.json|embedded default
                     -> strictness(): derived from S..F actions   <-> style table switches

Sync (Economy tab):  league list worker -> dropdown -> settings.league
                     Sync button (disabled if price_source=fallback) -> worker -> cache_dir/prices.json

Apply:   pending styles+tiers + (prices.json | embedded fallback pair per price_source)
         -> economy_body_with(.., policy) -> game_dir/<filter>

Update:  About "Check" -> worker -> GitHub latest -> compare -> download to cache_dir/updates
         -> "Install and quit" -> Quit popup if dirty -> release single-instance/hotkey/tray -> spawn installer -> exit
```

## Patterns to Follow

### Pattern 1: Pure core + thin I/O wrapper (already the house style: `paths::resolve_from`)
**What:** resolve dirs with a pure function taking env/dirs-lookups as parameters; tests inject temp dirs.
**When:** `locations`, game-folder resolver, update version compare, strictness derive.

### Pattern 2: One worker abstraction
**What:** generalise `EconomySync` (start / poll / panic-safe settle). League list, sync, update check and download all use it. Avoid three near-identical copies (the Release Review phase exists precisely to strip such duplication).

### Pattern 3: Settings with fail-closed unknown values
**What:** every new `ff-settings.json` key follows the `reload_after_apply` precedent; invalid value => documented, visible warning, safe behaviour (e.g. invalid `price_source` => fallback with warning? Choose conservative: treat as live but warn). Unknown keys preserved via flatten.

### Pattern 4: Embedded default behind the same "live else default" rule
**What:** `Source::Embedded` slots into `read_source` so every existing reader (`styles_source`, overlay file watcher `app.rs:1361`) keeps its semantics. The file watcher that reloads styles when `styles.json` OR `styles-default.json` changes (`app.rs:296-330, 1361`) must be adapted: with embedded defaults only the live file is watched; "no live file" is a stable state.

## Anti-Patterns to Avoid

- **Dual-source drift:** hand-copying defaults into Rust constants instead of `include_str!` of the committed JSON. Keep the JSON as the single source (tests and `artifact_guard` read it).
- **Writing to the profile in tests/screenshot mode:** every test touching `AppDirs` must set `FERRISFILTER_HOME`; add a debug assertion in the platform-dir branch of tests (`cfg(test)`) that panics if resolution falls to the real `%APPDATA%`.
- **Auto-picking the league via `indexed`:** the documented flap; never reintroduce silently.
- **Spawning the installer before releasing the instance mutex/hotkey/tray.**
- **Changing `generate` output or cwd semantics** while moving everything else: it protects the byte-for-byte baseline.

## Pitfalls specific to integration (feeding PITFALLS.md)

1. Screenshot guard bypass via Known Folder API (see Layer 4): critical.
2. `config.json` and `mapping.json` omitted from embedding: overlay class picker and sync break on a clean machine; invisible on the author's dev machine because the repo is the root. Add a test that runs core flows with an empty `FERRISFILTER_HOME`.
3. "Works on my machine" because the author's real `%APPDATA%\FerrisFilter` won't exist until first write; verify first-run (no files at all) in the overlay: config tab loads from embedded defaults, first Apply creates `styles.json`/`tiers.json`, directories created with `create_dir_all`, atomic write (`safe_write.rs`) must create parent dirs.
4. Migration: users of v0.6 (the author only) have a repo-root `styles.json`/`tiers.json`/`ff-settings.json`. Decide: one-time copy from `FERRISFILTER_HOME`/old root, or document manual move. Recommend nothing automatic (public release has no prior users); the author sets `FERRISFILTER_HOME` for dev.
5. `build.rs` `git describe` needs tags in CI (`fetch-depth: 0`); a shallow checkout gives the `Cargo.toml` version with unclear `-dev`.
6. Single-instance + installer: Inno `CloseApplications` needs the window title/restart manager to find `ff.exe`; an overlay hidden to the tray may not respond to close requests. Provide an installer-side `taskkill`-style `[UninstallRun]`/`PrepareToInstall` fallback or an exit message handled by the overlay (`instance.rs` already listens for second-launch messages; a `--quit` argument that tells the running instance to exit would be a clean hook; MEDIUM complexity, flag for phase research).
7. Update check rate-limit/offline: show an error in the status line, never block the UI.
8. Long text on About: the memory note says avoid walls of text in the overlay.
9. Test count: ~1392 lib tests; many load `repo_file("styles-default.json")` via temp dirs and `relative_to`. Keep `relative_to` semantic stable to avoid mass edits.

## Suggested Build Order

Dependencies: locations/defaults (A) -> game folder (B) and league/price source (C) and updater (E); style rework (D) independent but must precede the final fixture baseline; strictness (F) depends only on the style set (and benefits from D); CI/installer (G) needs a working embedded-defaults exe and a version scheme; Release Review last.

1. **Phase A: Per-user locations and embedded defaults.** `AppDirs`, `defaults.rs`, `Source::Embedded`, `*_str` loaders, embed config/mapping, `InputPaths::from_dirs`, settings moved, CLI `apply`/`sync`/`install` reconciled, `paths.rs` walk-up retired, file-watcher adapted, first-run (empty home) test. Highest risk, touches the widest surface; do it first so everything else builds on the final layout. Keep screenshot guard green (add `FERRISFILTER_GAME_DIR` here, even though the picker comes later).
2. **Phase B: Game folder setting + picker.** `Settings::game_folder`, resolver with Documents API, overlay wiring (switch popup reuse), Config UI row + Browse (`rfd` worker), CLI default `--filter`. Depends on A (settings location, guard).
3. **Phase C: Built-in style rework and fixture re-baseline.** Independent; small. Place before F so the selector is verified against the final built-ins. Include the single fixture/snapshot regeneration and CHANGELOG-neutral notes (no changelog entries during the milestone, per the close-out rules).
4. **Phase D: Strictness Selector.** Pure derive/set + Config row; needs C's final S..F styles; test table-driven.
5. **Phase E: League picker and price source.** Extract generic worker; league list; remove `OVERLAY_LEAGUE`; settings; `ForceFallback` plumbing; sync disable; tier-editor behaviour under fallback; prices read from cache_dir. Depends on A (cache dir, embedded fallback pair) and settings plumbing from B.
6. **Phase F: Release CI and installer.** `release.yml`, Inno script, changelog extraction, version check, zip. Can run in parallel with D/E logically (depends only on A for a self-contained exe), but sequence after E so the first tag ships the final binary; develop and dry-run on a pre-release tag (`v0.7.0-rc1`; build.rs `--match v*` handles it; the update checker must ignore prereleases or compare them correctly). Needs the installer asset naming decided before G.
7. **Phase G: Check for updates.** Depends on F (asset naming, release feed, repo constant) and the worker helper from E. Quit integration. Because the feed only exists after the first real release, test against a fixture JSON plus the pre-release dry run.
8. **Phase H: Release Review** (cleanup incl. duplication from the worker/embedded refactors, README screenshots, `Cargo.toml` -> 0.7.0, changelog draft, author review), as mandated by PROJECT.md.

Alternative: swap F and G ordering is impossible (G needs F's feed); D and C could merge into one phase if the author prefers fewer phases.

## Research Flags for Phases

| Phase | Needs deeper research? | Why |
|-------|------------------------|-----|
| A | Moderate | `dirs` vs direct Win32 Known Folder; `safe_write` parent-dir creation; list of every `app_root()`/`relative_to` call site (`run.rs:181`, `app.rs:1183, 2159`, tests) |
| B | Moderate | `rfd` in eframe/Windows (threading, window parenting with an always-on-top docked window: the dialog may open behind the overlay; verify z-order); screenshot guard redesign |
| C | Low | Mechanical; author supplies the new styles |
| D | Low | Pure logic; measure the 7-button row at 660pt |
| E | Low-moderate | poe.ninja league list shape is already modelled; decide default league policy and tier editor under fallback |
| F | Moderate-high | Inno Setup on GitHub runners, CHANGELOG extraction, tag-vs-Cargo version gate, `gh release` draft flow, pre-release naming (not verified against live docs in this pass) |
| G | Moderate-high | GitHub API behaviour (rate limits, redirects, asset URLs), safe launch of the installer from a GUI-subsystem process, handing off the single-instance name, `ff.exe` file lock during upgrade |

## Scalability Considerations

Not user-scale sensitive. Relevant "scale" is update/economy refresh frequency: poe.ninja sync is manual; update check is manual (one request per click), well inside GitHub's unauthenticated limit.

## Confidence

| Area | Confidence | Notes |
|------|------------|-------|
| Seams in existing code (InputPaths, settings, sync, quit, guard) | HIGH | read directly; line numbers above |
| Embedding set (config.json, mapping.json also needed) | HIGH | verified `app.rs:308`, `prices.rs:462` |
| Strictness design | HIGH | pure derivation over existing model |
| League/price-source plumbing | HIGH | policy enum and league list already exist |
| Crate choices (`dirs`, `rfd`, `sha2`) | MEDIUM | from training knowledge, not re-verified this pass |
| Inno Setup / Actions specifics | MEDIUM-LOW | not verified against current docs; flag for phase research |
| Installer-vs-running-overlay handoff | MEDIUM | design follows `instance.rs` behaviour described in CLAUDE.md; not exercised |

## Sources

- Repository code at the paths cited above (src/paths.rs, src/compile.rs, src/settings.rs, src/install.rs, src/dynamic.rs, src/run.rs, src/cli.rs, src/sync/*, src/overlay/{app,economy,screenshot,about_view,config_state,config_view,classes}.rs, build.rs, Cargo.toml), `.planning/PROJECT.md`, `CLAUDE.md`
- External crate/tool claims (dirs, rfd, Inno Setup, GitHub Releases API) are from training knowledge and need verification in their phases.
