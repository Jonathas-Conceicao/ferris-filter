# Phase 37: Game Folder Resolver & Picker - Research

**Researched:** 2026-10-18
**Domain:** Rust / egui overlay (Windows): game-folder resolution, native folder dialog (`rfd`), test isolation, game-art extraction
**Confidence:** HIGH on resolver, isolation and art; MEDIUM on dialog z-order over the topmost overlay (needs a manual check)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
**Default and storage**
- Default game folder: OS Documents folder (`dirs::document_dir()`: follows OneDrive redirection and localized names) + `My Games\Path of Exile 2`. For the author this resolves to the current `OneDrive\Documentos` path.
- Stored as a `game_folder` key in `ff-settings.json`, written only when the user picks a folder; absent means the default.
- `FERRISFILTER_GAME_DIR` overrides everything. Screenshot mode requires it to be under temp (replaces the `USERPROFILE`-derived game-folder check).
- `ff-cli apply --filter` and `ff-cli install --dest` defaults use the same resolver; the hard-coded `OneDrive\Documentos` path (`src/install.rs:13-21`, screenshot tests) and the help text naming it are removed.

**Config tab layout (Filter section)**
- Game folder row: label, the path cut with an ellipsis (full path on hover), a Browse button.
- Filter row: the existing filter dropdown with the refresh button directly beside it.
- Refresh re-lists the folder's `.filter` and `.mp3` / `.wav` files only: it does not re-read the loaded filter's contents (that is Reload), changes no selection, never writes, never discards edits.
- No "Use default" button; Browse's tooltip names the default path. (The default itself is unchanged.)

**Picking a folder**
- Native folder dialog (`rfd`), opened at the current folder, shown above the always-on-top overlay (owned by the overlay window or otherwise brought to the front).
- A pick applies and is saved at once, like the filter choice; with unapplied rule edits the Switch filter? popup asks first.
- Filter selection when the folder changes (and on a fresh install): keep the filter with the **same name** if the new folder has one; otherwise select the **first `.filter` found** in the folder. `dynamic.filter` is not special anywhere. Only when the folder has no `.filter` at all is the current name used as a new file that the first Apply creates.
- A folder with no `.filter` files is allowed (the rule above).

**Missing folder and the refresh art**
- Missing game folder: the status line reads "Game folder not found: <path>"; clicking it opens the Config tab.
- Refresh art: located in the game files with `ff-art` (`list` / `dump`) at development time and added as an `assets/poe2/manifest.toml` entry, extracted into `assets/poe2/` and compiled into the binary like the rest of the skin (nothing is read at run time). If no suitable game art is found, the button is a text button reading a capital **R**; work does not stop for this, and the Release Review can pick a replacement.

### Claude's Discretion
- Exact widget composition, the `rfd` threading / parenting approach, and how the settings key is migrated in tests.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GAME-01 | Default game folder = Documents + `My Games\Path of Exile 2` | `dirs::document_dir()` is `SHGetKnownFolderPath(FOLDERID_Documents)` (OneDrive and localized names handled, ignores `USERPROFILE`); new pure resolver in `src/locations.rs` |
| GAME-02 | Config tab shows folder + Browse; choice saved/restored | typed `game_folder` field in `Settings`; rfd sync `pick_folder` on a worker thread |
| GAME-03 | Folder change with unapplied rule edits asks first | reuse `pending_switch` / Switch filter? popup; generalise the pending value (see Pattern 4) |
| GAME-04 | Missing folder shows status pointing to the picker | new `StatusInputs` field + click-to-Config jump in `status_for` |
| GAME-05 | Refresh button re-lists filters and sounds | `refresh_filters()` + `sound_files::refresh(ctx)`; no write, no selection change |
| GAME-06 | Refresh uses the game's art via ff-art manifest | Candidate found: `common/webtoolbarrefresh{normal,hover,down}.dds` (round arrow on square button, 3 states) |
</phase_requirements>

## Summary

The phase is mostly wiring on top of Phase 36's `locations.rs`. Three pieces carry real risk: (1) the folder dialog must not make the overlay hide itself (the visibility state machine hides the window on any focus loss, and a hidden owner hides its owned dialog), (2) test isolation: after this phase `USERPROFILE` no longer influences anything, so every spawned binary and the screenshot guard must pin `FERRISFILTER_GAME_DIR` instead, and (3) the existing tests that derive the game folder from `USERPROFILE` (`tests/dynamic_filter.rs` `default_filter_path_uses_userprofile`, `src/overlay/screenshot.rs` tests, `tests/isolation_guard.rs`) must be rewritten.

`rfd` 0.17.2 (Jan 2026) fits the tree: it depends on `raw-window-handle 0.6` (Cargo.lock has 0.6.2, shared with winit 0.30.13 / eframe 0.36.1) and `windows-sys 0.61` (the crate's own version). On Windows it uses the Shell `IFileOpenDialog` with `FOS_PICKFOLDERS`, initialises COM itself per call, and `FileDialog` is `Send`, so the recommended shape is: build the dialog (including `set_parent(frame)`) on the UI thread, move it into a `std::thread`, call blocking `pick_folder()`, send the result back over an `mpsc` channel, `ctx.request_repaint()`.

For art, `ff-art list refresh` found the exact asset the author describes: `art/textures/interface/2d/2dart/uiimages/common/webtoolbarrefreshnormal|hover|down.dds`, a 52x52 square button with a baked-in round arrow, with 4K siblings. A second candidate (`optionsmenu/optionsiconrefresh.dds`, a bare grey-beige round arrow, 72x72, 92x92 in 4K) is the glyph-only fallback matching the UI-SPEC's "glyph on the skin button". Either way the `R` text button remains the non-blocking fallback.

**Primary recommendation:** Add `rfd = { version = "0.17", default-features = false }`; resolver `locations::game_dir_from(env, saved, documents)` (pure) + `game_dir()` wrapper; use the `webtoolbarrefresh*` three-state art as the whole 34x34 button (same `[TextureHandle; 3]` pattern as `close`); keep the overlay visible while a dialog is open by feeding `focused = true` into `visibility::advance`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Game folder resolution (env > saved > Documents) | Library (`locations.rs`) | CLI (`install`/`apply` defaults), overlay | The one place that asks the OS for a folder (guarded by `isolation_guard::dirs_only_in_locations`) |
| `game_folder` persistence | Library (`settings.rs`) | Overlay (writes on pick) | Pure load/save with unknown keys preserved |
| Folder dialog | Overlay (worker thread) | Win32 owner = overlay HWND | Native dialog is a GUI concern; UI thread must never block |
| Filter/sound re-listing | Overlay (`refresh_filters`, `sound_files`) | `settings::list_filters` | Existing caches, never per frame |
| Missing-folder status | Overlay shell (`status_for`) | Config tab | Priority-ordered one-line status |
| Refresh art | Build-time dev tool (`ff-art`) | `src/skin/textures.rs` | Compiled in via `include_bytes!` |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rfd | 0.17.2 (published 2026-01-12) | Native folder picker | Named in CONTEXT; 36M downloads, repo github.com/PolyMeilex/rfd, MIT, MSRV 1.92 (toolchain is nightly 1.100) [VERIFIED: crates.io API, cargo info] |
| dirs | 6.0.0 (already a dependency) | `document_dir()` | Windows: `known_folder_documents()` = `SHGetKnownFolderPath(FOLDERID_Documents)` [VERIFIED: dirs-sys 0.5.0 / dirs 6.0.0 source in cargo registry] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tempfile | 3 (already) | Tests | Temp game folders |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| rfd sync on `std::thread` | `rfd::AsyncFileDialog` | Needs an executor; the crate already has tokio but a plain thread + `mpsc` is simpler and equally non-blocking. Not recommended. |
| rfd | Hand-rolled `IFileOpenDialog` via windows-sys | Don't hand-roll (COM vtables). |

**Installation:**
```bash
cargo add rfd@0.17 --no-default-features
```
`default = ["xdg-portal", "wayland"]` are Linux-only; dropping them is harmless on Windows and keeps the tree small. `common-controls-v6` is only for message dialogs; not needed. On Windows rfd adds only `windows-sys` features (`Win32_System_Com`, `Win32_UI_Shell(_Common)`, `Win32_UI_WindowsAndMessaging`), a feature union on the existing 0.61.2.

**Version verification:** `cargo search rfd` returned `rfd = "0.17.2"`; crates.io API `max_version 0.17.2`, created 2026-01-12. Cargo.lock currently has no `rfd`; `raw-window-handle 0.6.2` is present, matching rfd's `^0.6`.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| rfd | crates.io | 5+ yrs (2020-11) | ~14M recent | github.com/PolyMeilex/rfd | not run (pip unavailable) | Approved: user-named in CONTEXT.md, crates.io metadata and source inspected [ASSUMED re slopcheck] |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none. slopcheck could not be installed (no `pip`), so formally `[ASSUMED]`; the planner should keep a `cargo tree -i rfd` / `cargo audit`-style sanity step but a human-verify checkpoint is optional since the user chose the crate.

## Architecture Patterns

### System Architecture Diagram

```
FERRISFILTER_GAME_DIR (env) ──┐
ff-settings.json game_folder ─┼─> locations::game_dir_from(env, saved, documents) ──> Option<PathBuf>
dirs::document_dir() + My Games\Path of Exile 2 ─┘          │
                                                            ├─> overlay: OverlayApp.game_dir (build + reload)
                                                            ├─> CLI apply default filter / install default dest
                                                            └─> screenshot guard / stage_filter

Config tab [Browse] --(UI thread: FileDialog::new().set_parent(frame).set_directory(existing ancestor))-->
   worker std::thread: pick_folder() (blocking, own COM init) --mpsc--> OverlayApp.poll_folder_pick()
        None (cancel) -> nothing
        Some(path) == current -> nothing
        Some(path) -> rules dirty? --yes--> pending folder + Switch filter? popup --Stay--> drop
                                    --no / Switch--> apply_folder(path):
                                         save settings.game_folder, game_dir = path,
                                         refresh_filters(), pick filter (same name > first .filter > keep name),
                                         switch_to(name) path (reset state, refresh_inputs, sound_files::set_dir)

Config tab [Refresh] -> refresh_filters() + sound_files::refresh(ctx)   (no write, no selection change)

status_for: root_error > event > grammar_gate > filter_error > GAME FOLDER NOT FOUND(jump: Config tab) > syncing > unknown > migrated > absent > indicator
```

### Recommended Project Structure
```
src/locations.rs        # + GAME_DIR_ENV, game_dir_from (pure), game_dir(saved), documents-based default, default_game_dir_hint()
src/settings.rs         # + Settings.game_folder: Option<String>, validated accessor
src/install.rs          # default_dest() delegates to locations; text updated
src/dynamic.rs          # default_destination() delegates (or is removed)
src/overlay/folder_dialog.rs  # NEW: FolderPick worker (spawn/poll), start_dir(), pure helpers
src/overlay/app.rs      # folder state, ConfigAction::{BrowseFolder, RefreshLists}, apply_folder, status input
src/overlay/config_view.rs    # two labelled rows, refresh button
src/overlay/shell.rs    # StatusInputs.game_folder_missing + jump target (tab)
src/skin/textures.rs    # + refresh: [TextureHandle; 3]; src/skin/widgets.rs + ff_icon_button
assets/poe2/common/refresh_{normal,hover,down}.png ; manifest.toml entries
```

### Pattern 1: Pure resolver (mirrors `resolve_from`)
**What:** `game_dir_from(env: Option<&OsStr>, saved: Option<&Path>, documents: impl FnOnce() -> Option<PathBuf>) -> Option<PathBuf>`: non-empty env wins verbatim (no existence check: a missing folder is reported by the status line, not an error); else a saved absolute path; else `documents()?.join("My Games").join("Path of Exile 2")`. `game_dir()` wraps it with `std::env::var_os(GAME_DIR_ENV)`, `dirs::document_dir`.
**Test guard:** like `from_env`, add `#[cfg(test)]` panic in the wrapper unless `FERRISFILTER_GAME_DIR` is set, so a unit test can never reach the real Documents folder. `OverlayApp::build` already takes `game_dir` as a parameter (tests inject it) and `reload()` only re-resolves under `#[cfg(not(test))]`; keep that.
**Must stay in `locations.rs`:** `tests/isolation_guard.rs::dirs_only_in_locations` fails any `dirs::` use elsewhere. [VERIFIED: tests/isolation_guard.rs]

### Pattern 2: rfd on a worker thread with a parent
```rust
// Source: rfd 0.17.2 src/file_dialog.rs (set_parent reads the handles immediately; FileDialog is Send+Sync)
let dialog = rfd::FileDialog::new()
    .set_title("Select your Path of Exile 2 folder")
    .set_directory(existing_ancestor_of(&current)) // SetFolder: must exist, else ignored
    .set_parent(frame);                            // &eframe::Frame: HasWindowHandle + HasDisplayHandle
let (tx, rx) = std::sync::mpsc::channel();
let ctx = ui.ctx().clone();
std::thread::spawn(move || {
    let _ = tx.send(dialog.pick_folder());          // blocks this thread only; rfd does CoInitializeEx itself
    ctx.request_repaint();
});
```
- `set_parent` needs the `Frame` (available in `OverlayApp::ui(&mut self, ui, frame)`), so build the `FileDialog` there (on the UI thread) and move it; only the resulting `RawWindowHandle` crosses threads, which rfd documents as an accepted `unsafe impl Send`. [CITED: rfd source comment "I don't like sending RawWindowHandle between threads but here we go anyways"]
- Windows backend: `IFileDialog::Show(hwndOwner)` with `FOS_PICKFOLDERS`; the owner is disabled for the dialog's duration (modal to the overlay) while the egui frame loop keeps running on the main thread. [VERIFIED: rfd win_cid/file_dialog/dialog_ffi.rs]
- Keep a `FolderPick { rx, open: bool }` state; poll with `try_recv` each frame (cheap, the app already repaints every 50 ms). A disconnected channel (thread panic) = treat as cancel and clear `open`.
- `ConfigAction::BrowseFolder` is produced in `config_view`; because `Frame` is only in `ui()`, either handle the action in `ui()` right where actions are drained (the drain site in `app.rs` ~L2440 runs inside the `ui` call chain; pass `frame` down) or stash `want_browse = true` and spawn at the top of the next `ui()`.

### Pattern 3: Keep the overlay visible while the dialog is open (CRITICAL)
`visibility::advance(state, focused, toggle)` turns `ShownFocused` into `Hidden` on the first unfocused frame, and `win::conceal` is `ShowWindow(SW_HIDE)` on the overlay HWND. When the dialog takes focus, the overlay loses focus, so the overlay would hide, and **hiding an owner also hides its owned dialog**. UI-SPEC already says "the overlay does not auto-hide"; the concrete implementation is in `app.rs` ~L2845: `let focused = real_focus || self.folder_pick.is_open();` (and treat a toggle request as before: hotkey toggling while a dialog is open should be ignored or not, planner's choice; recommend ignore). After the dialog closes, the game or the overlay gets focus; if the user clicks the game next, normal auto-hide resumes. Note the hotkey is already gated to game/overlay foreground, and the dialog is neither, so it is naturally ignored. [VERIFIED: src/overlay/visibility.rs, app.rs]
Also guard the other focus consumers if any code uses `focused` (grep `viewport().focused`): only the one site found.

### Pattern 4: Pending folder pick reuses the Switch filter? popup
`pending_switch: Option<String>` currently holds a filter name and the popup shows `target`. For a folder pick the confirm must apply a folder, not just a name. Recommended: add `pending_folder: Option<PathBuf>` next to it; the popup flag is `pending_switch.is_some() || pending_folder.is_some()`, `ConfirmSwitch` takes whichever is set (folder first), `CancelSwitch` clears both; popup `target` shows the folder's display path (title and buttons unchanged per UI-SPEC). Alternative (also fine): `enum PendingSwitch { Filter(String), Folder(PathBuf) }`. `switch_decision(rules_dirty, current, target)` is string-based; for folders use `rules_dirty` directly (Noop if equal to current, Confirm if dirty, else apply). Do not mark CONFIG dirty.

### Pattern 5: Applying a pick
1. Save first via `settings::save(root, &next)` with `next.game_folder = Some(path.to_string_lossy())` (unknown keys preserved by the `flatten` extra map; a failed save still switches for the session and says so, like `switch_to`).
2. `self.game_dir = Some(path)`; `refresh_filters()`; choose the filter: `contains_name(list, current)` (case-insensitive) keep it (use the listed spelling), else `list.first()`, else keep current name (the "absent filter" status then applies and the first Apply creates it).
3. Reuse `switch_to(name)` for the rest (it saves the filter name, resets state, `refresh_inputs`, `refresh_filters`, sound dir). Merge into one settings save to avoid two writes: build `next` with both `filter` and `game_folder`, then call the shared tail. Even when the chosen name equals the current one the loaded filter file changes (different folder), so the reload/reset must run regardless of `switch_decision`'s Noop.
4. `sound_dir_stale = true` is set by `reset_filter_view` and re-lists sounds on the next frame.

### Pattern 6: Refresh button (no state change)
`ConfigAction::RefreshLists` → `self.refresh_filters(); super::sound_files::refresh(ctx)`; touch nothing else (no `reset_*`, no message, no dirty flag). Note `sound_files::refresh` re-lists the cached dir, which is the loaded filter's folder: correct after a plain refresh. Disabled when `filters_cache.is_none()` (folder missing/unreadable) per UI-SPEC.

### Pattern 7: Status line for a missing folder
`StatusInputs` gains `game_folder_missing: Option<String>` (text `Game folder not found: <path>`, `StatusKind::Error`), placed after `filter_error` and before `syncing`. `StatusLine.jump: Option<BlockId>` currently carries a rule id; add a second target (e.g. change to `enum StatusJump { Rule(BlockId), Tab(Tab) }` or add `go_tab: Option<Tab>`), and the click handler at app.rs ~L2690 (`status: status.jump.is_some() && line.clicked()`) switches `self.tab = Tab::Config`. Missing = `game_dir` is `Some` and `!dir.is_dir()` (compute with `filters_cache.is_none()` already derived from `list_filters(dir).ok()`, avoiding a per-frame stat; set a `game_dir_missing` bool in `refresh_filters`, the only place that touches the disk). When `game_dir` is `None` (no Documents folder at all), keep the existing `filter_target` error text but reword (see Pitfall 6).

### Anti-Patterns to Avoid
- **Blocking `pick_folder()` on the UI thread:** freezes egui and stalls the hotkey/visibility loop.
- **Calling `set_parent` on the worker thread:** `Frame` is not available there.
- **Probing the filesystem per frame** (OneDrive reads are slow, noted in `app.rs:231`): keep existence in the `refresh_filters` cache.
- **Passing a nonexistent directory to `set_directory`:** `SetFolder` fails; walk up to the nearest existing ancestor, else `dirs::document_dir()` (via `locations`, not `dirs::` directly) or skip.
- **Writing `game_folder` when the user never picked:** absent must mean default (and the default may differ per machine).
- **Canonicalizing the picked path:** project convention (no `\\?\` paths, `locations.rs` header).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Folder picker | Raw COM `IFileOpenDialog` | `rfd::FileDialog::pick_folder` | COM init, vtables, owner handling, `SetFolder` quirks |
| Documents location | `%USERPROFILE%\Documents` or `OneDrive\...` string building | `dirs::document_dir()` (in `locations.rs`) | OneDrive Known Folder Move and localized names |
| Texture slicing/loading | custom image code | existing `tex!` + ff-art manifest `[[image]]` | consistent skin pipeline |
| Settings round-trip | ad-hoc JSON edit | `Settings` + `settings::save` | unknown keys preserved, atomic safe write |

## Runtime State Inventory
Not a rename/migration phase, but there is one runtime-state fact worth recording:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | Existing users' `ff-settings.json` has no `game_folder` (absent means default); nothing to migrate | none |
| Live service config | None | none |
| OS-registered state | None | none |
| Secrets/env vars | New optional `FERRISFILTER_GAME_DIR`; `USERPROFILE` stops mattering (the old hard-coded join used it; `dirs` reads the registry/Known Folders and ignores it) | Update every test spawn and the guard (below) |
| Build artifacts | None | none |

## Test isolation (LOC-05 continuation)

Because `dirs::document_dir()` ignores `USERPROFILE`, any test that spawns a binary resolving the game folder without `FERRISFILTER_GAME_DIR` would reach the real Documents folder. Required changes:
1. `locations::game_dir()` wrapper panics under `#[cfg(test)]` unless `FERRISFILTER_GAME_DIR` is set (mirror `from_env`).
2. `tests/isolation_guard.rs::spawns_that_resolve_the_folder_set_home`: the required pins become `FERRISFILTER_HOME` **and** `FERRISFILTER_GAME_DIR` (drop the `USERPROFILE` requirement; `USERPROFILE` may remain harmlessly). Update the doc comment accordingly. Also extend the "resolves" detection to `"install"` spawns without `--dest`, since `install` now resolves the game folder. Keep the `.cargo/config.toml` rule: do not set the vars globally (hides a missing pin).
3. Existing spawn sites: `tests/dynamic_filter.rs` (helper `apply` at ~L72 plus ~L145, 338, 382, 497, 512), `tests/cli_errors.rs:332`, `tests/cli_generate.rs:265`: add `.env("FERRISFILTER_GAME_DIR", <temp>/game or a temp subpath)`. `dynamic_filter.rs` already uses `<dir>/game/`; point the env at it. `default_filter_path_uses_userprofile` (L399-406) must be rewritten as "default filter path uses FERRISFILTER_GAME_DIR": env = `<dir>/nogame`, expect failure naming that path (`dynamic.filter`), assert it was not created; rename the test. Add a case: env unset is impossible to test safely through a spawn; test the pure `game_dir_from` instead (env empty is unset; saved used; documents closure called lazily and joined correctly).
4. Screenshot mode (`src/overlay/screenshot.rs`): `guard_env` requires `FERRISFILTER_HOME` and `FERRISFILTER_GAME_DIR` (set and non-empty; no fallback to the default, since the default is the real folder) under temp; change `paths_under_temp(home, profile, game, temp)` to `(home, game, temp)`, delete `USERPROFILE` from the module docs, error messages and the CLAUDE.md text (the docs row naming `USERPROFILE`), and `stage_filter` reads the game dir from the env var. Rewrite the three tests at ~L455-515 that join `OneDrive/Documentos/My Games/Path of Exile 2` (they only need arbitrary game paths now). Keep `temp_looks_real` (TEMP-pointing-at-profile defence) and the "name the offending variable" assertions with `FERRISFILTER_GAME_DIR`.
5. Overlay unit tests construct `OverlayApp::build(..., game_dir)` with temp dirs already; `config_view` tests pass `game_dir: None` or temp. No change besides new tests. Never call `locations::game_dir()` from tests.

CLI wiring: `run.rs:159`, `dynamic::default_filter_path`, `install::default_dest`: all take the resolver. Should the CLI honour a saved `game_folder`? Recommended **yes** for consistency ("same resolver"): CLI loads settings from the app root when it resolves (`settings::load(root)` creates nothing) and passes `saved` in. Error texts change from "USERPROFILE is not set" to "Cannot find the Documents folder; set FERRISFILTER_GAME_DIR or pass --filter/--dest" (update the two `cli.rs` help lines at L76 and L111 to "[default: the game folder (Documents\My Games\Path of Exile 2, or FERRISFILTER_GAME_DIR)]"). The `install` unit tests that call `default_dest` need review; `cargo test` must not touch the real folder.

## Refresh button art (GAME-06)

Search run (read-only, `CARGO_TARGET_DIR=%TEMP%\ff-art-target`, dumps only to temp scratch, nothing written under the repo): `cargo ff-art list refresh` against `C:\Program Files (x86)\Steam\steamapps\common\Path of Exile 2` returned these relevant UI textures [VERIFIED: ff-art output]:

| Candidate | Look (viewed) | Size | Notes |
|-----------|---------------|------|-------|
| `art/textures/interface/2d/2dart/uiimages/common/webtoolbarrefreshnormal.dds` / `...hover.dds` / `...down.dds` | Dark square button, orange-bronze frame, **round arrow baked in** | 52x52; `common/4k/` siblings exist | **Best match** to "round arrow on a square button", three states like `buttonclose*` |
| `art/textures/interface/2d/2dart/uiimages/ingame/optionsmenu/optionsiconrefresh.dds` | Bare grey-beige round arrow (counter-clockwise), no button | 72x72; `optionsmenu/4k/` 92x92 | Glyph-only fallback if a glyph-on-skin-button is preferred (UI-SPEC wording) |
| `shop/mtxcachearrowrefresh{default,highlight,clicked}.dds`, `expedition/vendorwindows/refreshvendorbutton*.dds`, `common/icongamblerefresh.dds` | not viewed | | Other-context refresh art; not preferred |

Recommendation (a decision for the planner, flagged because UI-SPEC drafted "glyph on `skin.button`"): use `webtoolbarrefresh*` as the **whole button** (it is already a square button with the round arrow; `skin.button` 3-slice cannot be reused under it without double frames). Draw at 34x34pt (`ROW_HEIGHT`), states normal / hover(or focus) / down; disabled = same normal texture tinted `Color32::from_white_alpha(90)` (the same tint `ff_button_ex` uses for disabled). If the executor prefers the UI-SPEC's literal glyph-on-button, use `optionsiconrefresh` tinted `pal::BUTTON_TEXT`. The `R` fallback never blocks.

Manifest entry shape (existing pattern, `[[image]]`, prefer the 4K sibling, trim the transparent gutter; no `crop`/`clear` needed):
```toml
# Refresh button: the web-toolbar refresh (round arrow on a square button), three states.
[[image]]
path = "art/textures/interface/2d/2dart/uiimages/common/webtoolbarrefreshnormal.dds"
out = "common/refresh_normal.png"
prefer_4k = true
trim = true
[[image]]
path = "art/textures/interface/2d/2dart/uiimages/common/webtoolbarrefreshhover.dds"
out = "common/refresh_hover.png"
prefer_4k = true
trim = true
[[image]]
path = "art/textures/interface/2d/2dart/uiimages/common/webtoolbarrefreshdown.dds"
out = "common/refresh_down.png"
prefer_4k = true
trim = true
```
Then `cargo ff-art` (set `CARGO_TARGET_DIR` outside the repo if `ff.exe` runs), commit the three PNGs, and in `src/skin/textures.rs` add `pub refresh: [TextureHandle; 3]` loaded like `close` (`tex!(ctx, "poe2/common/refresh_normal.png")` etc., see textures.rs L97/L150). Add a `ff_icon_button`-style widget in `src/skin/widgets.rs`: allocate `vec2(34.0, 34.0)` (`ROW_HEIGHT`), `Sense::click()` when enabled else `hover`, `resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, "Refresh"))`, state from `is_pointer_button_down_on` / `hovered || has_focus` (same logic as `ff_button_ex`), paint the texture with the pixel-snapping helpers used for `close`. Execution gotcha: `trim = true` removes the transparent right/bottom gutter; verify the 4K result is square after trim (otherwise drop `trim` or add `crop = [0,0,w,w]`), and the executor must view the PNG once (ff-art `zoom`/`sheet`, or the Read tool on the PNG).

## Common Pitfalls

### Pitfall 1: The overlay hides when the dialog opens
**What goes wrong:** the dialog vanishes (owner hidden) or the overlay disappears behind it; the window state machine reads the focus loss as "clicked away".
**How to avoid:** Pattern 3. **Warning signs:** dialog flashes and closes; `Hidden` visibility during `open`.

### Pitfall 2: Dialog appears behind the always-on-top overlay
**What goes wrong:** the overlay is `WindowLevel::AlwaysOnTop` (topmost); a non-owned dialog can open underneath. **How to avoid:** `set_parent(frame)` (owned windows stay above their owner in z-order; documented Win32 behaviour [CITED: learn.microsoft.com "Window Features: Owned Windows"; MEDIUM, not exercised in this session]). **Verification:** manual check with the real game (human-verify item, consistent with Phase 36's deferred UAT); fallback: after the dialog starts, nothing else is needed, but if it still opens behind, call `SetForegroundWindow`/topmost on the dialog is not possible from rfd, so the fallback is to drop `WindowLevel::AlwaysOnTop` temporarily while the dialog is open (egui `ViewportCommand::WindowLevel(Normal)` then restore).

### Pitfall 3: Dialog start directory missing
`set_directory` with a nonexistent path is ignored or errors in `SetFolder`; pass the nearest existing ancestor (`Path::ancestors().find(|p| p.is_dir())`).

### Pitfall 4: Settings key validation
A hand-edited `game_folder` that is empty, relative or not a string must not poison the resolver: ignore it (fall to default) and surface a `settings::load` warning like the other keys, or treat as absent silently; never join a name onto an unvalidated path. A saved folder that no longer exists is *not* invalid: it is the missing-folder status case. The first-launch and "unusable settings" path (`unusable()` returns defaults with reload off) must also leave `game_folder` absent.

### Pitfall 5: Env override and the picker
When `FERRISFILTER_GAME_DIR` is set, Browse is disabled (UI-SPEC); do not write `game_folder` then. The resolver returns the env value even when it is not an existing dir (so a nonexistent override reaches the missing-folder status, which screenshot state 3 relies on).

### Pitfall 6: Stale "USERPROFILE is not set" texts
`filter_target` (app.rs ~L969), the unit test asserting the exact string (app.rs ~L3317) and `run.rs` all say `USERPROFILE is not set`. With `document_dir()` the only failure is "no Documents folder": reword, update the test, and keep it a status/error (not a panic).

### Pitfall 7: Selection when the folder changes while a draft or popup is open
`reset_filter_view` clears `pending_switch`; clear `pending_folder` and `config_ui.filter_draft` there too, and refuse a new Browse while a pick or popup is pending.

### Pitfall 8: `Settings` equality and serialization
`Settings` derives `PartialEq`/`Serialize` with `skip_serializing_if`; add `game_folder` with `#[serde(default, skip_serializing_if = "Option::is_none")]` so existing files re-serialise byte-identically (the flatten `extra` map must not also capture it: a typed field and the flatten map do not conflict because serde assigns known keys to the field).

## Code Examples

### Pure resolver + wrapper
```rust
// locations.rs (sketch)
pub const GAME_DIR_ENV: &str = "FERRISFILTER_GAME_DIR";
pub fn game_dir_from(
    env: Option<&OsStr>,
    saved: Option<&Path>,
    documents: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(v) = env.filter(|v| !v.is_empty()) { return Some(PathBuf::from(v)); }
    if let Some(p) = saved.filter(|p| p.is_absolute()) { return Some(p.to_path_buf()); }
    documents().map(|d| d.join("My Games").join("Path of Exile 2"))
}
pub fn game_dir(saved: Option<&Path>) -> Option<PathBuf> {
    #[cfg(test)] { /* panic unless GAME_DIR_ENV set, like from_env */ }
    game_dir_from(std::env::var_os(GAME_DIR_ENV).as_deref(), saved, dirs::document_dir)
}
pub fn game_dir_is_pinned() -> bool { std::env::var_os(GAME_DIR_ENV).is_some_and(|v| !v.is_empty()) }
```

### Ellipsis path with hover (egui)
Use a `Label::truncate()` with `.show_tooltip_text`/`on_hover_text(full)` inside a `ui.allocate_ui_with_layout` of width `available - BROWSE_W - FIELD_GAP`; `Label::new(RichText::new(path).size(SUMMARY_SIZE).color(VALUE)).truncate()` keeps one line with `…`. [ASSUMED: egui 0.36 `Label::truncate` API unchanged; check with a compile.]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `USERPROFILE\OneDrive\Documentos\...` string join | `SHGetKnownFolderPath(FOLDERID_Documents)` via `dirs` | this phase | Works for non-OneDrive and non-Portuguese setups |
| rfd 0.15 README examples (`raw-window-handle 0.5`) | rfd 0.17 on `raw-window-handle 0.6` | 0.16+ | Matches eframe 0.36 / winit 0.30 |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | An owned dialog stays above a topmost owner (Win32 owner z-order rule) | Pitfall 2 | Dialog could open behind the overlay; fallback: temporarily drop AlwaysOnTop; needs a manual check |
| A2 | `rfd` passes slopcheck (could not run) | Package Legitimacy | Low: user-named, long-lived crate with verified repo |
| A3 | egui 0.36 `Label::truncate` yields a single-line ellipsis | Code Examples | Minor; adapt with `egui::text::LayoutJob` truncation |
| A4 | Windows disabling the owner from another thread behaves (modal to the overlay only) and the egui loop keeps rendering | Pattern 2 | Cosmetic; the dialog is still usable |
| A5 | `trim = true` on the 4K refresh textures yields a square image | Art | Executor adjusts `crop`; no impact on design |

## Open Questions (RESOLVED)

1. **Should hotkey toggling be ignored while the dialog is open?**
   - Known: the hotkey is gated to game/overlay foreground, and the dialog is neither.
   - Recommendation: ignore `toggle_requested` while `folder_pick.open`.
   - RESOLVED: yes; plan 37-05 ignores the toggle while the dialog is open.
2. **Should the CLI honour a saved `game_folder`?**
   - Recommendation: yes (consistency with "same resolver"); falls back to default when the app root or file is absent. Document in `--help`.
   - RESOLVED: yes (orchestrator decision); plan 37-03.
3. **Refresh art whole-button vs glyph.** Recommendation above (whole button); UI-SPEC wording says glyph, so the planner should note the deviation in the plan and keep `R` as fallback.
   - RESOLVED: whole-button art accepted (orchestrator decision), `R` text button fallback; plan 37-02.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust nightly | build | yes | rustc 1.100.0-nightly (2026-09-03) | n/a (rfd MSRV 1.92 satisfied) |
| PoE2 Steam install | ff-art extraction | yes | `C:\Program Files (x86)\Steam\steamapps\common\Path of Exile 2` | `R` text button |
| ff-art build | extraction | yes (built in `%TEMP%\ff-art-target` during research) | n/a | n/a |
| pip / slopcheck | legitimacy gate | no | n/a | package marked `[ASSUMED]` |

## Validation Architecture
Skipped: `workflow.nyquist_validation` is `false` in `.planning/config.json`. (Practical note for plans: tests run with `cargo test`; new pure units are `locations::game_dir_from`, settings `game_folder` round trip and invalid values, `status_for` ordering, the folder-selection rule as a pure function `choose_filter(list, current) -> String`, `existing_ancestor`, and `visibility` unchanged with a `focused || dialog_open` helper; GUI parts through `ff --screenshot` states listed in UI-SPEC.)

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2/V3 Auth/Session | no | local desktop tool |
| V4 Access Control | no | n/a |
| V5 Input Validation | yes | validate `game_folder` (non-empty, absolute) on load; filter names stay validated by `valid_filter_name` before any join |
| V6 Cryptography | no | n/a |
| V12 Files | yes | never canonicalize; writes only through `safe_write`/`settings::save`; refresh is read-only (`read_dir`) |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Hand-edited settings pointing outside intended folder / traversal via filter name | Tampering | Name validation before join (existing); picker path is a user choice, absolute only |
| Tests/screenshot writing the real game folder | Tampering | `FERRISFILTER_GAME_DIR` required under temp, `.ff-screenshot-staged` marker, panic-in-test wrapper |
| Worker-thread result applied after state changed | Tampering/logic | Apply on the UI thread only, on `try_recv`; ignore results if app is closing |

## Sources

### Primary (HIGH confidence)
- rfd 0.17.2 source (`~/.cargo/registry`, `src/file_dialog.rs`, `src/backend/win_cid/*`) and raw GitHub master: `set_parent`, `Send`, `set_directory`, `pick_folder`, `init_com`, `FOS_PICKFOLDERS`.
- crates.io API for rfd 0.17.2 (features, deps, dates, downloads).
- dirs 6.0.0 / dirs-sys 0.5.0 source: `document_dir` -> `FOLDERID_Documents`.
- Repository code: `src/locations.rs`, `settings.rs`, `install.rs`, `dynamic.rs`, `overlay/{app,config_view,shell,screenshot,visibility,win,sound_files}.rs`, `tests/isolation_guard.rs`, `tests/dynamic_filter.rs`, `Cargo.toml`, `Cargo.lock`.
- `cargo ff-art list refresh` / `dump` (read-only) on the local Steam install; PNGs viewed.

### Secondary (MEDIUM)
- Win32 owned-window z-order semantics (training knowledge, to be confirmed in-game).

### Tertiary (LOW)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, versions and deps verified in source and registry.
- Architecture: HIGH for resolver/status/isolation; MEDIUM for dialog z-order (A1).
- Pitfalls: HIGH, derived from reading the current code (visibility state machine, guard tests).

**Research date:** 2026-10-18
**Valid until:** 2026-11-17
