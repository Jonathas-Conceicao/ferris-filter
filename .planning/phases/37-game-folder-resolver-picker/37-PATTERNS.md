# Phase 37: Game Folder Resolver & Picker - Pattern Map

**Mapped:** 2026-10-18
**Files analyzed:** 19 (new/modified)
**Analogs found:** 18 / 19 (all line numbers from the worktree at mapping time)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/locations.rs` (+`GAME_DIR_ENV`, `game_dir_from`, `game_dir`) | utility (resolver) | request-response (pure) | same file `resolve_from` / `from_env` | exact |
| `src/settings.rs` (+`game_folder` field, validation) | model | file-I/O (JSON round trip) | same file `Settings.filter` + `load` per-key salvage | exact |
| `src/install.rs` `default_dest()` | utility | request-response | same file (replace body) | exact |
| `src/dynamic.rs` `default_destination()` / `default_filter_path()` | utility | request-response | same file (delegate) | exact |
| `src/run.rs` (apply default `--filter`, ~L157) / `src/cli.rs` help L76, L111 | controller (CLI) | request-response | same lines | exact |
| `src/overlay/folder_dialog.rs` (NEW) | service (worker) | event-driven (thread + mpsc) | `src/overlay/sound_files.rs` (cache + pure helpers); sync worker in `app.rs` (thread + channel) | role-match |
| `src/overlay/app.rs` (folder state, `apply_folder`, `ConfigAction` handling, focus, status input) | controller | event-driven | `select_filter` / `switch_to` / `ConfirmSwitch` in same file | exact |
| `src/overlay/config_view.rs` (`show_filter`, `ConfigAction`, `ConfigViewCtx`) | component | request-response | same file `show_filter` (L301-379) | exact |
| `src/overlay/shell.rs` (`StatusInputs`, `StatusLine`, `status_for`) | component | request-response | same file row for `filter_error` / `grammar_gate` | exact |
| `src/overlay/screenshot.rs` (`guard_env`, `paths_under_temp`, `stage_filter`, tests) | utility / test | request-response | same file L225-300, L455-515 | exact |
| `src/skin/textures.rs` (+`refresh: [TextureHandle; 3]`) | config (asset registry) | transform | `close: [TextureHandle; 3]` | exact |
| `src/skin/widgets.rs` (+`ff_icon_button`) | component | request-response | `ff_close` (L181-194) + `ff_button_ex` (L1146-1206) | exact |
| `assets/poe2/manifest.toml` + `assets/poe2/common/refresh_*.png` | config | file-I/O | `buttonclose*` entries (L128-141) | exact |
| `Cargo.toml` (+`rfd`) | config | n/a | `dirs = "6"` line 41 | role-match |
| `tests/isolation_guard.rs` | test | file-I/O (source scan) | same file L55-90 | exact |
| `tests/dynamic_filter.rs`, `tests/cli_errors.rs:332`, `tests/cli_generate.rs:265` | test | request-response | `apply` helper `tests/dynamic_filter.rs:70-80` | exact |
| `tests/ui/README.md` | docs | n/a | same file L14-22 (mentions `USERPROFILE`) | exact |
| `CLAUDE.md` / `CHANGELOG.md` (screenshot-mode text naming `USERPROFILE`, default game path) | docs | n/a | existing rows | exact |
| Worker `rfd` dialog spawn | service | event-driven | NO close analog in codebase for `rfd`; use RESEARCH Pattern 2 | none |

## Pattern Assignments

### `src/locations.rs` (utility, pure resolver)

**Analog:** same file, `resolve_from` + `from_env` (lines 54-103). Mirror them exactly: pure core with every input a parameter, thin env wrapper with a `#[cfg(test)]` panic.

**Pure core pattern** (lines 54-76):
```rust
pub fn resolve_from(
    home: Option<&OsStr>,
    platform: impl FnOnce() -> Option<PathBuf>,
) -> Result<AppDirs, RootError> {
    if let Some(value) = home.filter(|v| !v.is_empty()) {
        let dir = PathBuf::from(value);
        ...
    }
    match platform() { ... }
}
```

**Wrapper with test guard** (lines 79-93) - copy for `game_dir()`; use `GAME_DIR_ENV` and keep `dirs::document_dir` here only (`tests/isolation_guard.rs::dirs_only_in_locations` fails any `dirs::` elsewhere):
```rust
pub fn from_env() -> Result<AppDirs, RootError> {
    #[cfg(test)]
    {
        let set = std::env::var_os(HOME_ENV).is_some_and(|v| !v.is_empty());
        if !set {
            panic!("tests must not resolve the real per-user folder: build AppDirs::at(<temp dir>) instead");
        }
    }
    resolve_from(std::env::var_os(HOME_ENV).as_deref(), dirs::config_dir)
}
```
Differences from the analog: env value is returned verbatim with NO `is_dir` check (a missing override must reach the missing-folder status); `saved` is accepted only if `is_absolute()`; no `Result`. Sketch in RESEARCH "Pure resolver + wrapper".

**Tests** (lines 105-168): copy the `boom()` closure (panics if platform lookup called) to prove `documents` is lazy; `empty_env_is_unset`; `per_user_folder_is_not_created` style (resolver never creates the folder).

---

### `src/settings.rs` (model, JSON round trip)

**Analog:** same file. `Settings` struct (lines 21-35) - add field in the exact attribute style of `filter`:
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub filter: Option<String>,
```
Add `game_folder: Option<String>` the same way (existing files re-serialise byte-identically, Pitfall 8). Accessor next to `filter_name()` / `reload_after_apply()` (lines 98-107): return `Option<&Path>` only when non-empty and absolute, else `None`.

**Per-key salvage in `load`** (lines 128-175): the wrongly-typed-key branch removes each key from the `extra` map (`extra.remove("filter")`, `Some(serde_json::Value::String(n)) if valid_filter_name(&n).is_ok()`) and records a fallback string. Add a `game_folder` branch the same way (string, non-empty, absolute; otherwise absent plus a fallback note). The `unusable()` closure builds `Settings { reload_after_apply: Some(false), ..Settings::default() }`; `game_folder` stays absent there.

**Save with unknown keys preserved:** `settings::save(root, &next)`; build `next` with the helper pattern from `app.rs:975`:
```rust
fn settings_with_filter(current: &Settings, name: &str) -> Settings {
    Settings { filter: Some(name.to_string()), ..current.clone() }
}
```
Add `settings_with_folder` (sets `game_folder` and `filter` together to avoid two writes).

**Listing for refresh / selection rule:** `list_filters(dir)` (lines 349-365, sorted case-insensitively, so "first `.filter` found" = `list.first()`), `contains_name(list, name)` (L368, case-insensitive). The pure `choose_filter(list, current) -> String` should use these two.

---

### `src/install.rs`, `src/dynamic.rs`, `src/run.rs`, `src/cli.rs` (CLI defaults)

**Analog/old code to replace** `install.rs:11-21`:
```rust
pub(crate) fn default_dest() -> Option<PathBuf> {
    let profile = std::env::var_os("USERPROFILE")?;
    let mut path = PathBuf::from(profile);
    path.push("OneDrive"); path.push("Documentos"); ...
```
Replace body with the `locations::game_dir(saved)` call. `dynamic.rs:34` `default_destination()` already "delegates to install"; keep it a one-line delegate (or call locations directly) so `dynamic.rs:104 default_filter_path()` and `app.rs:1092,1107,2228` need no change of shape. Update comments at `install.rs:1-8` and `dynamic.rs:28-33`.

**Error texts to reword** (`install.rs:40`, `run.rs:159`, `app.rs:969`, test `app.rs:3317`): all say "Cannot resolve the default PoE2 config folder (USERPROFILE is not set)". New text per RESEARCH: "Cannot find the Documents folder; set FERRISFILTER_GAME_DIR or pass --filter/--dest". `cli.rs:76` and `cli.rs:111` help lines name `%USERPROFILE%\OneDrive\Documentos...`; replace with the resolver wording.

Optional saved folder for the CLI: `settings::load(root)` creates nothing (see load docs, missing file = default) - pass `settings.game_folder` as `saved`.

---

### `src/overlay/app.rs` (controller, event-driven)

**Analog:** `select_filter` / `switch_to` / `ConfirmSwitch` (same file).

**Decision + pending pattern** (lines 2477-2487, 2467-2472):
```rust
fn select_filter(&mut self, name: String) {
    let rules_dirty = self.working.as_ref().is_some_and(|w| w.is_dirty());
    match switch_decision(rules_dirty, self.settings.filter_name(), &name) {
        SwitchDecision::Noop => self.pending_switch = None,
        SwitchDecision::Switch => self.switch_to(name),
        SwitchDecision::Confirm => self.pending_switch = Some(name),
    }
}
...
ConfigAction::ConfirmSwitch => { if let Some(name) = self.pending_switch.take() { self.switch_to(name); } }
ConfigAction::CancelSwitch => self.pending_switch = None,
```
Add `pending_folder: Option<PathBuf>` beside `pending_switch` (field at L235; init L1313). For folders use `rules_dirty` directly (Noop if equal to current dir). Clear it in `reset_filter_view` (L1475 `self.pending_switch = None;`) and `CancelSwitch`. Popup flag (L1813) becomes `self.pending_switch.is_some() || self.pending_folder.is_some()`; popup `target` (L1874) shows the folder display path when `pending_folder` is set. Update test at L5765.

**Applying a pick** - copy `switch_to` (lines 1490-1503), which is the shared tail:
```rust
fn switch_to(&mut self, name: String) {
    let next = settings_with_filter(&self.settings, &name);
    let save_error = match &self.root { Ok(root) => settings::save(root, &next).err(), Err(_) => None };
    self.settings = next;
    self.reset_filter_state();
    self.refresh_inputs();
    self.refresh_filters();
    if let Some(err) = save_error {
        self.config_message = Some((format!("Could not save ff-settings.json: {err}"), true));
    }
}
```
`apply_folder(path)`: set `self.game_dir = Some(path)`, `refresh_filters()`, compute name via `choose_filter`, build `settings_with_folder`, then the same save/reset tail (runs even if the name is unchanged, since the file source changed). Must not touch `config_ui` dirtiness.

**Refresh** (`ConfigAction::RefreshLists`): `refresh_filters()` (L1385-1390) + `super::sound_files::refresh(ctx)`; nothing else. `refresh_filters` is also the only place to compute a cached `game_dir_missing` (avoid per-frame stat; see comment near L231).
```rust
fn refresh_filters(&mut self) {
    self.filters_cache = self.game_dir.as_deref().and_then(|dir| settings::list_filters(dir).ok());
}
```

**Action drain site:** L2431-2473 (`match action { ConfigAction::... }`). `Frame` is only in `ui()`; either pass `frame` down to this drain or set `want_browse` and spawn at top of next `ui()`.

**Keep overlay visible while the dialog is open** (line 2845-2851):
```rust
let focused = ui.ctx().input(|i| i.viewport().focused).unwrap_or(false);
let next = if self.is_shot() { Visibility::ShownFocused }
           else { visibility::advance(self.visibility, focused, self.toggle_requested) };
```
Change to `focused || self.folder_pick.is_open()` (and ignore `toggle_requested` while open). This is the only `viewport().focused` consumer.

**Reload re-resolution** (L2226-2229) already gated `#[cfg(not(test))]`; keep gating and pass the saved folder: `self.game_dir = locations::game_dir(self.settings.game_folder_path())` style (via `dynamic::default_destination` shim if retained).

**Status input:** where `StatusInputs { ... }` is built (L3144 and L5069 test) add `game_folder_missing`; click handling (L2681-2691, `status.jump.is_some() && line.clicked()`; L3174 `jump_to = status.jump`) needs a second target for "open Config tab" (enum or `go_tab`).

---

### `src/overlay/config_view.rs` (component)

**Analog:** `show_filter` lines 301-379. Keep the frame/spacing scaffold:
```rust
egui::Frame::new().fill(pal::ROW_EVEN).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
    ui.set_width(ui.available_width());
    ui.spacing_mut().item_spacing.y = 4.0;
    body::caption(ui, "GAME FOLDER FILTER");   // REMOVE (UI-SPEC)
```
Existing combo to wrap in a labelled row, shrinking `width(ui.available_width() - 8.0)` by `ROW_HEIGHT + FIELD_GAP`:
```rust
egui::ComboBox::from_id_salt("cfg-filter").selected_text(shown)
    .width(ui.available_width() - 8.0)
    .show_ui(ui, |ui| { ... actions.push(ConfigAction::SelectFilter(name.clone())); ... })
```
Disabled state when folder missing: `ui.add_enabled_ui(ctx.filters.is_some(), ...)`, text `"(game folder not found)"` (L319, keep). Hover text L344-352: drop `saved_note` "Default: dynamic.filter." (UI-SPEC decision 5).

**Two-column labelled rows:** `skin::widgets::ff_field_row(ui, index, label, tip, |ui| ...)` (widgets.rs L513-553; control rect starts at `ROW_PAD + LABEL_COL_W + LABEL_GAP`, left-to-right centred layout). Buttons: `ff_button(ui, skin, "Browse", 96.0)` and `ff_button_ex(ui, skin, text, min_width, enabled, danger)` (widgets.rs L1140-1206). `ConfigViewCtx.skin` is `Option<&Skin>` (headless tests pass `None`), so guard skin-dependent widgets with `if let Some(skin) = ctx.skin`, falling back to `small_button(...)` as `filter_draft_row` does (L419-425).

**Path ellipsis:** `egui::Label::new(RichText::new(path).size(fonts::SUMMARY_SIZE).color(pal::VALUE)).truncate()` + `.on_hover_text(full)`; the value colour/size idiom is at L371-375:
```rust
ui.label(RichText::new(RELOAD_TOGGLE_LABEL).size(crate::skin::fonts::SUMMARY_SIZE).color(pal::VALUE))
```
Missing folder: use `pal::WARN`.

**New actions** in `ConfigAction` (L46-78): add `BrowseFolder`, `RefreshLists` beside `SelectFilter/CreateFilter/ConfirmSwitch/CancelSwitch`. New `ConfigViewCtx` (L154-177) fields: `game_dir_pinned: bool` (Browse disabled, tooltip `Set by FERRISFILTER_GAME_DIR`), `folder_dialog_open: bool`, `default_game_dir: Option<&Path>` (Browse tooltip), `game_dir_missing: bool`.

---

### `src/overlay/shell.rs` (component, status line)

**Analog:** `status_for` rows (lines 135-190). Add `game_folder_missing: Option<String>` to `StatusInputs` (L135-145) and a row immediately after `filter_error`, before `syncing`:
```rust
if let Some(text) = inputs.filter_error {
    return plain(text, StatusKind::Error);
}
// NEW: game folder not found -> StatusLine { text, kind: StatusKind::Error, jump: <Config tab target>, tip: None }
if inputs.syncing { ... }
```
`StatusLine` (L120-130) has `jump: Option<BlockId>`; add a tab target (e.g. `StatusJump` enum or `go_tab`). Tests: update `empty()` / `full()` helpers (L1040-1066) and add a `row4b_game_folder_missing_*` test in the style of `row3_grammar_gate_jumps` (L1086-1094); `row2` event must still outrank it.

---

### `src/overlay/screenshot.rs` (utility + tests)

**Analog:** same file. Current guard (L225-285) takes `(home, profile, game_dir, temp)`:
```rust
for (name, path) in [
    ("FERRISFILTER_HOME", home),
    ("USERPROFILE", profile),
    ("the game folder derived from USERPROFILE", game_dir),
] {
    let climbs = path.components().any(|c| matches!(c, std::path::Component::ParentDir));
    if climbs || !canonical_lossy(path).starts_with(&temp) { return Err(format!(...)); }
}
```
Change to `(home, game_dir, temp)` with names `FERRISFILTER_HOME` and `FERRISFILTER_GAME_DIR`; `guard_env()` reads `GAME_DIR_ENV` (must be set and non-empty, no fallback to the real default); `game_dir()` (L279-282, currently `crate::dynamic::default_destination()`) reads the env var; `stage_filter` (L289+) keeps using it. Keep `temp_looks_real` and "names the offending variable" assertions. Rewrite tests at L455-515 that join `OneDrive/Documentos/My Games/Path of Exile 2` (they only need arbitrary paths) and the `err.contains("USERPROFILE")` assertion (L513) to `FERRISFILTER_GAME_DIR`. Module doc line 8 also names `USERPROFILE`.

---

### `src/skin/textures.rs`, `src/skin/widgets.rs`, `assets/poe2/manifest.toml` (refresh art)

**Textures analog:** `close: [TextureHandle; 3]` (field L90 area, load at L125-129):
```rust
close: [
    tex!(ctx, "poe2/common/close_normal.png"),
    tex!(ctx, "poe2/common/close_hover.png"),
    tex!(ctx, "poe2/common/close_down.png"),
],
```
Add `pub refresh: [TextureHandle; 3]` with `poe2/common/refresh_{normal,hover,down}.png`. The `every_embedded_texture_decodes` test (L173+) covers decoding; `load_light` covers headless tests.

**Manifest analog** (`assets/poe2/manifest.toml` L128-141):
```toml
[[image]]
path = "art/textures/interface/2d/2dart/uiimages/common/buttonclosenormal.dds"
out = "common/close_normal.png"
prefer_4k = true
trim = true
```
New entries per RESEARCH use `common/webtoolbarrefresh{normal,hover,down}.dds` -> `common/refresh_{normal,hover,down}.png`. Verify square after `trim` (A5). Fallback if art unusable: `R` text via `ff_button_ex(ui, skin, "R", 34.0, enabled, false)`.

**Widget analog:** `ff_close` (widgets.rs L181-194) for the three-state texture paint, and `ff_button_ex` (L1146-1206) for enabled/disabled logic:
```rust
let rect = Rect::from_center_size(center, Vec2::splat(size));
let resp = ui.interact(rect, ui.id().with("ff_close"), Sense::click());
resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, "Close"));
let state = if resp.is_pointer_button_down_on() { 2 } else if resp.hovered() { 1 } else { 0 };
slice::paint_image(ui.painter(), &skin.close[state], rect, Color32::WHITE);
```
and from `ff_button_ex`: `let enabled = enabled && ui.is_enabled();`, `Sense::click()` vs `Sense::hover()`, `ui.allocate_exact_size(vec2(w, 34.0), sense)`, disabled tint `Color32::from_white_alpha(90)`, `resp.hovered() || resp.has_focus()` for hover. `ff_icon_button(ui, skin, tex: &[TextureHandle;3], label, enabled) -> Response` using `allocate_exact_size(vec2(ROW_HEIGHT, ROW_HEIGHT), ..)` (ROW_HEIGHT = widgets.rs L440; FIELD_GAP = L1289).

---

### `src/overlay/folder_dialog.rs` (NEW, worker)

**Analog:** `src/overlay/sound_files.rs` for module shape (small, UI-agnostic state plus pure helpers with `#[cfg(test)]` tests; `set_dir(ctx, ..)` / `refresh(ctx)` / `get(ctx)` at L58-77) and the pure-helper style. No existing `rfd` use. Follow RESEARCH Pattern 2: build `rfd::FileDialog::new().set_title("Select your Path of Exile 2 folder").set_directory(existing_ancestor).set_parent(frame)` on the UI thread, `std::thread::spawn` calling blocking `pick_folder()`, send over `mpsc`, `ctx.request_repaint()`. State `FolderPick { rx: Option<Receiver<Option<PathBuf>>> }` with `is_open()` and `poll()` (disconnected = cancel). Pure helpers to unit-test: `existing_ancestor(&Path) -> Option<&Path>` (`ancestors().find(|p| p.is_dir())`), `choose_filter`. Never canonicalize the picked path.

---

### `Cargo.toml`

Existing deps `dirs = "6"` (L41), `eframe = "0.36"` (L42). Add `rfd = { version = "0.17", default-features = false }`.

---

### Tests: `tests/isolation_guard.rs`, `tests/dynamic_filter.rs`, `tests/cli_errors.rs`, `tests/cli_generate.rs`

**Guard analog** (`tests/isolation_guard.rs` L55-90): the scan finds each spawn, decides `resolves` and requires both env pins:
```rust
let resolves = is_ff || chain.contains("\"apply\"") || chain.contains("\"sync\"");
if resolves && !(chain.contains(".env(\"FERRISFILTER_HOME\"") && chain.contains(".env(\"USERPROFILE\"")) { bad.push(...) }
```
Change the second pin to `.env("FERRISFILTER_GAME_DIR"`; add `chain.contains("\"install\"") && !chain.contains("--dest")` to `resolves`; update the assertion message and header comment (L6).

**Spawn analog** (`tests/dynamic_filter.rs` L70-80):
```rust
Command::cargo_bin("ferrisfilter").expect("binary").current_dir(dir)
    .env("FERRISFILTER_HOME", dir)
    .env("USERPROFILE", dir)
    .arg("apply").args(args).assert()
```
Add `.env("FERRISFILTER_GAME_DIR", dir.join("game"))` here and at the other spawn sites (`dynamic_filter.rs` L145, 338, 382, 497, 512; `cli_errors.rs:332`; `cli_generate.rs:265`). `USERPROFILE` pins can stay harmlessly or be dropped. Rewrite `default_filter_path_uses_userprofile` (L399-406) to pin `FERRISFILTER_GAME_DIR=<dir>/nogame` and assert failure naming that path and that it was not created; drop `dir.join("OneDrive")` assertion. Module doc at L6 mentions `USERPROFILE`.

## Shared Patterns

### Config choices that are not edits save immediately and never mark CONFIG dirty
**Source:** `app.rs` `switch_to` L1490-1503 and `set_reload_after_apply` (called from `ConfigAction::SetReloadAfterApply`, L2447). Save via `settings::save`, store result in `config_message` on failure, never call config dirty paths.
**Apply to:** folder pick, refresh.

### Save-failure message
```rust
self.config_message = Some((format!("Could not save ff-settings.json: {err}"), true));
```

### Deferred-action pattern
Widgets only push `ConfigAction`s into a `Vec`; `app.rs` drains and mutates (L2431-2473). Apply to Browse and Refresh.

### Pure-core + thin-wrapper with `#[cfg(test)]` real-environment panic
**Source:** `locations.rs` L54-93. **Apply to:** `game_dir_from` / `game_dir`; tests must never call `game_dir()` without `FERRISFILTER_GAME_DIR`.

### Filter-name validation before any join
**Source:** `app.rs` `filter_target` L961-972 (`settings::valid_filter_name(name)` then `dir.join(name)`). Unchanged; only the `None` message text changes.

### Isolation (no real folders from tests)
`USERPROFILE` stops mattering because `dirs::document_dir()` ignores it; every spawn and the screenshot guard must pin `FERRISFILTER_GAME_DIR`.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `rfd` dialog thread (inside `src/overlay/folder_dialog.rs`) | service | event-driven | No native-dialog or `rfd` code exists; use RESEARCH Pattern 2/3. Closest structural analog is the Sync economy worker thread in `app.rs` (grep `sync` worker), not read in detail here. |

## Metadata

**Analog search scope:** `src/locations.rs`, `src/settings.rs`, `src/install.rs`, `src/dynamic.rs`, `src/run.rs`, `src/cli.rs`, `src/overlay/{app,config_view,shell,screenshot,sound_files,visibility}.rs`, `src/skin/{textures,widgets}.rs`, `assets/poe2/manifest.toml`, `tests/{isolation_guard,dynamic_filter}.rs`
**Pattern extraction date:** 2026-10-18
