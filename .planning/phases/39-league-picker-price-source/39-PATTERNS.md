# Phase 39: League Picker & Price Source - Pattern Map

**Mapped:** 2026-10-18
**Files analyzed:** 14 new/modified
**Analogs found:** 14 / 14 (all in-repo, symbol-anchored; Phase 37/38 files are mid-flight, so re-read each file at execution time)

Anchor rule: every excerpt below is cited by file plus symbol. Line numbers were read in this worktree on 2026-10-18 and will drift once 37/38 land.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/sync/league.rs` (+`list_leagues`, `default_league`) | service | request-response | same file `fetch_index_state` / `resolve_league_with_override` | exact |
| `src/sync/models.rs` (derive on `EconomyLeague`/`IndexState`) | model | transform | same file | exact |
| `src/sync/mod.rs` (drop `OVERLAY_LEAGUE`, `sync_economy_blocking(base_dir, league)`, `fetch_leagues_blocking`) | service | request-response | same file `sync_economy_blocking` | exact |
| `tests/fixtures/index-state-2026-10-18.json` | test fixture | n/a | `tests/fixtures/index-state.json` | exact |
| `src/settings.rs` (+`league`, `price_source`, `price_policy`, salvage arms) | model/config | CRUD (file) | same file `reload_after_apply` | exact |
| `src/overlay/leagues.rs` (NEW) | model (framework-free) | event-driven (worker + channel) | `src/overlay/economy.rs` `EconomySync` | exact |
| `src/overlay/mod.rs` (`pub(crate) mod leagues;`) | config | n/a | existing `mod economy;` line | exact |
| `src/overlay/apply_all.rs` (`FilterInput.policy`) | service | request-response | same file `prepare` | exact |
| `src/overlay/config_state.rs` (tiers-in-use flag) | model | CRUD | same file `validate` / `has_tier_errors` | exact |
| `src/overlay/config_view.rs` (`show_tiers_section(.., enabled)`) | component | request-response | same file `show_filter` (`add_enabled_ui`) | role-match |
| `src/overlay/shell.rs` (`EconomyTabInputs`, `EconomyClicks`, `economy_tab`, `StatusInputs.ended_league`, `status_for`, `tab_markers`) | component | request-response | same file | exact |
| `src/overlay/app.rs` (fields, polling, handlers, marker, preview key, status inputs) | controller | event-driven | same file `set_reload_after_apply`, `economy_clicks.sync`, `refresh_staged_economy` | exact |
| `src/overlay/screenshot.rs` (`--leagues`, `--league`, `--price-source`) | utility | transform | same file `--tier-text` | exact |
| `CLAUDE.md` | docs | n/a | existing Economy sync paragraph | exact |

## Pattern Assignments

### `src/sync/league.rs` (service, request-response)

**Analog:** same file, `fetch_index_state`.

```rust
use crate::sync::models::IndexState;
pub const INDEX_STATE_URL: &str = "https://poe.ninja/poe2/api/data/index-state";

pub async fn fetch_index_state<C: crate::sync::client::HttpClient>(
    client: &C,
) -> miette::Result<IndexState> {
    let body = client.get_json(INDEX_STATE_URL, &[]).await?;
    serde_json::from_str(&body)
        .map_err(|e| miette::miette!("Cannot parse index-state response: {e}"))
}
```
Add `list_leagues(&client) -> miette::Result<Vec<EconomyLeague>>` (wraps the above, returns `economy_leagues`) and `default_league(&[EconomyLeague]) -> Option<&EconomyLeague>` = first `!hardcore && url != "standard" && url != "hardcore"` (ignores `indexed`; contrast `resolve_league`'s `.find(|l| !l.hardcore && l.indexed)`, which must stay untouched for the CLI).

**Test pattern** (same file `mod tests`): helper `league(name, url, hardcore, indexed) -> EconomyLeague` and `make_state(vec![..])` build `IndexState { economy_leagues }` literally. `FakeClient` shape from `src/sync/metadata.rs` `mod tests`:
```rust
impl HttpClient for FakeClient {
    async fn get_json(&self, url: &str, _query: &[(&str, &str)]) -> miette::Result<String> { ... }
}
```
(native `async fn` in trait impl, not `impl Future`). Drive with `tokio::runtime::Runtime::new().unwrap().block_on(..)`. Load the new fixture with `include_str!("../../tests/fixtures/index-state-2026-10-18.json")`.

### `src/sync/models.rs`

`IndexState` and `EconomyLeague` currently derive only `serde::Deserialize`; the struct-literal test helpers and the league state need `Debug, Clone, PartialEq` added (`#[derive(Debug, Clone, PartialEq, serde::Deserialize)]`). No `deny_unknown_fields`, so `displayName`, `oldEconomyLeagues` etc. are ignored already.

### `src/sync/mod.rs` (service, request-response)

**Analog:** `sync_economy_blocking` (owns its own tokio runtime; called from a worker thread).
```rust
pub fn sync_economy_blocking(base_dir: &std::path::Path) -> miette::Result<String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| miette::miette!("Cannot create async runtime: {e}"))?;
    rt.block_on(async {
        let client = ReqwestClient::new()?;
        prices::run_sync_prices(&client, false, false, Some(OVERLAY_LEAGUE), base_dir).await
    })
}
```
Change to `(base_dir, league: &str)` passing `Some(league)`; delete the `OVERLAY_LEAGUE` const and its whole doc block (also fix the dangling `[`OVERLAY_LEAGUE`]` intra-doc link in the `sync_economy_blocking` doc, and the stale "automatic resolution" doc paragraph above it). Add `fetch_leagues_blocking() -> miette::Result<Vec<EconomyLeague>>` with the identical runtime scaffold calling `league::list_leagues(&client).await`. Gate: `rg OVERLAY_LEAGUE src` returns nothing.

### `src/settings.rs` (model, file CRUD)

**Analog:** `reload_after_apply` (field, accessor, salvage arm, round-trip tests).

Field pattern:
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub reload_after_apply: Option<bool>,
```
Add `league: Option<String>` and `price_source: Option<String>` the same way, before `collapsed_sections`/`extra`.

Salvage arm pattern in `load()` (the `Err(e)` branch of `from_str::<Settings>`): each key is `extra.remove("key")`, matched `Some(String)` / `None` / `Some(_) => { fallbacks.push(..); None }`. Copy the `game_folder` arm:
```rust
let game_folder = match extra.remove("game_folder") {
    Some(serde_json::Value::String(s)) if usable_folder(&s) => Some(s),
    None => None,
    Some(_) => {
        fallbacks.push("`game_folder` is not an absolute folder path, so the default game folder is used".to_string());
        None
    }
};
```
Then add both keys to the rebuilt `Settings { filter, game_folder, reload_after_apply, collapsed_sections, extra }` literal (also any other literal in the file/tests). Pitfall: forgetting `extra.remove` leaves the bad value re-serialised with no warning. An unknown `price_source` string is NOT an error (reads Live). Tests to copy: `reload_after_apply_defaults_on_and_is_not_serialized`, `reload_after_apply_false_round_trips`, the bad-type loop with `format!(r#"{{"filter":"ssf.filter","reload_after_apply":{bad},"future":1}}"#)` asserting the warning names the key and `extra` lacks it.

### `src/overlay/leagues.rs` (NEW; framework-free model)

**Analog:** `src/overlay/economy.rs` `EconomySync` (module doc states "Framework-free by contract"; no egui types).

Worker core to mirror:
```rust
use std::sync::mpsc::{Receiver, TryRecvError};
pub fn start<F>(&mut self, work: F) where F: FnOnce() -> Result<String, String> + Send + 'static {
    if self.is_running() { return; }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || { let _ = tx.send(work()); });
    self.rx = Some(rx);
    self.state = SyncState::Running;
}
pub fn poll(&mut self) -> bool {
    let Some(rx) = self.rx.as_ref() else { return false; };
    match rx.try_recv() {
        Ok(Ok(..)) => { ..; self.rx = None; true }
        Ok(Err(e)) => { ..; self.rx = None; true }
        Err(TryRecvError::Empty) => false,
        Err(TryRecvError::Disconnected) => { /* "the sync worker stopped unexpectedly" */ ..; true }
    }
}
```
New: `ListState { NotStarted, Loading, Loaded(Vec<EconomyLeague>), Failed(String) }`, `LeagueList { state, rx, last_good }` with `start`, `poll`, `state()`, `inject()` (screenshot/tests). Keep `EconomySync` unchanged (15 tests). Test helper to copy: `settle()` with a 5 s wall-clock deadline loop calling `poll()` + 1 ms sleep.

Also here: pure `view(...) -> LeagueView` (UI-SPEC state table), `effective_fallback`, `source_pending` (RESEARCH "Code Examples"). Table-driven tests, no egui.

### `src/overlay/apply_all.rs` (service, request-response)

**Analog:** `prepare` and `FilterInput`.
```rust
pub struct FilterInput<'a> {
    pub loaded: &'a LoadedFilter,
    pub styled: Result<String, String>,
    pub plan: EconomyPlan,
}
...
let priced = match config {
    Some(c) => dynamic::economy_body_with(&paths, PriceSourcePolicy::Auto, c.pending_styles(), c.pending_tiers()),
    None => dynamic::economy_body(&paths, PriceSourcePolicy::Auto),
};
let (text, source) = priced.map_err(|e| ApplyAllError::Invalid(e.to_string()))?;
used_fallback = matches!(source, PriceSource::Fallback(_));
```
Add `pub policy: PriceSourcePolicy` to `FilterInput` and replace both `PriceSourcePolicy::Auto` with `input.policy`. Construction sites to update: `FilterInput { loaded, styled, plan: working.economy_plan() }` in app.rs `apply_everything` (this is a free function taking `filter`/`config` args, so pass `self.settings.price_policy()` in as a parameter) plus the test constructions in `apply_all.rs` (set `policy: PriceSourcePolicy::Auto`). `compile::load_priced_with` under `ForceFallback` already ignores live tiers and loads the embedded pair; no compile.rs change.

### `src/overlay/config_state.rs` (model)

**Analog:** `has_tier_errors` / `validate` / `config_apply_enabled` gate.
```rust
pub fn has_tier_errors(&self) -> bool { !self.errors.row.is_empty() }
...
pub fn validate(&self) -> Result<(), ConfigApplyError> {
    if self.has_tier_errors() || self.tiers_error().is_some() || self.styles_error().is_some() {
        let reason = self.styles_error().or_else(|| self.tiers_error())
            .unwrap_or_else(|| "Fix the tier errors first".to_string());
        return Err(ConfigApplyError::Invalid(reason));
    }
```
Add a `tiers_in_use: bool` field (default true) with `set_tiers_in_use(bool)`; when false, `has_tier_errors()` and the tier branches of `validate()` / the apply-enabled predicate (the `&& !self.has_tier_errors() && self.tiers_error().is_none()` line) ignore tier errors, but `tier_rows()` keeps reporting `error` so inline red text returns with the editor. `tiers_dirty()` stays true. Tests: `s.set_tier_text("A", "abc"); assert!(s.has_tier_errors());` pattern (see the revert-scope test), then flag off => `validate().is_ok()`.

### `src/overlay/config_view.rs` (component)

**Analog:** `show_filter` greying in this file:
```rust
let combo = ui
    .add_enabled_ui(ctx.filters.is_some(), |ui| { egui::ComboBox::from_id_salt("cfg-filter") ... })
```
`show_tiers_section(ui, skin, state) -> Vec<ConfigAction>` has no enabled input today; wrap the call (in `shell::economy_tab` closure or inside the fn) in `ui.add_enabled_ui(!fallback, ..)` and attach `.response.on_disabled_hover_text("The fallback tiers are fixed.")`. If the tooltip does not show (child widgets eat hover), add `ui.interact(rect, id, Sense::hover())` over the block rect. Phase 38 adds the `Reset to defaults` heading button via a right-to-left child `Ui` (see 38-PATTERNS `shell.rs`); that button is forced disabled under Fallback.

### `src/overlay/shell.rs` (component, request-response)

**Analog:** `economy_tab` (rows via `w::ff_row(ui, idx, label, |ui| ..)`; renumber stripes 0..3).

Current rows to replace (League read-only label with `"Pinned in this build."`, Last sync, Priced items, Price source text). New order: Price source switch, League dropdown + refresh, Last sync, Priced items.

Value helper and Last sync row stay:
```rust
let value = |ui: &mut egui::Ui, text: &str, color: egui::Color32| {
    ui.label(egui::RichText::new(text).font(fonts::regular(15.0)).color(color))
};
```
Reused strings for the switch tooltip third line (move from the old Price source row):
`"No prices.json yet. Using the built-in early-league prices. Sync to get live ones."` and `"prices.json could not be used ({problem}) so the built-in early-league prices apply. ..."` (reword per UI-SPEC).

Switch: `crate::skin::editor::ff_segmented(ui, skin, &mut idx, &["Live", "Fallback"], w::CONTROL_WIDTH)`; `resp.changed()` => new click field.

League row geometry (Phase 37 Filter row): dropdown width `CONTROL_WIDTH - ROW_HEIGHT - FIELD_GAP`, `ui.add_space(FIELD_GAP)`, then `w::ff_icon_button(ui, &skin.refresh, "Refresh", enabled)` (the `skin::widgets` one, NOT `skin::editor::ff_icon_button`, the minimap tile). Dropdown signature:
`ff_dropdown_ex(ui, skin, id_salt, &mut usize, &[&str], width, caption_color: Option<Color32>) -> Response`. Disabled via `ui.add_enabled_ui(view.dropdown_enabled, ..)`; if it does not dim, also pass `Some(pal::DISABLED)`.

Sync button (existing, keep shape):
```rust
let enabled = inputs.sync_enabled && !inputs.syncing;
let mut r = w::ff_button_ex(ui, skin, label, 180.0, enabled, false);
if !enabled && !inputs.syncing && let Some(tip) = inputs.sync_disabled_tip { r = r.on_hover_text(tip); }
```
Compute the extra conditions and tooltip priority in app.rs (`view.sync_league`, fallback) and keep `sync_disabled_tip` semantics (root error first).

Structs to extend (all constructors incl. tests must gain fields):
```rust
pub(crate) struct EconomyClicks { pub sync: bool, pub add: bool, pub remove: bool }  // derives Default, PartialEq; add source: Option<usize>, league_pick: Option<usize/String>, refresh_leagues: bool
pub(crate) struct EconomyTabInputs<'a> { league, mtime_secs, now_secs, utc_offset_minutes, priced_items, live, problem, syncing, sync_enabled, sync_disabled_tip, section }
```
`league: &str` is replaced by the `LeagueView` data; `EconomyClicks` loses `Copy`-safety if a `String` is added (it derives `Clone, Copy`), so prefer an index or `Option<usize>`.

**Status line**: `StatusInputs` + `status_for` already contain `game_folder_missing` and `StatusLine.open_tab`. Insert directly after the `syncing` branch:
```rust
if inputs.syncing { return plain(SYNC_RUNNING.to_string(), StatusKind::Info); }
// NEW
if let Some(text) = inputs.ended_league {
    return StatusLine { open_tab: Some(ShellTab::Economy), ..plain(text, StatusKind::Warning) };
}
if let Some((text, id)) = inputs.unknown { ... }
```
`StatusInputs` literals to update: shell.rs tests `empty()` / `full()`, and app.rs (`shell::status_for(StatusInputs {` at the live build site and two test sites).

**tab_markers**: signature `(rules_dirty, styles_dirty, tiers_dirty, economy_staged) -> [bool; 4]`, Economy = `tiers_dirty || economy_staged`. Keep the signature; pass `economy_staged || source_pending` is NOT right (Revert popup logic uses `economy_staged`); instead add a fifth parameter or OR at the single call site in app.rs (`shell::tab_markers(rules_edited, styles_dirty, tiers_dirty, economy_staged)`) only for the marker, never for the Revert/popup computation.

### `src/overlay/app.rs` (controller, event-driven)

**Persist-a-setting pattern** (analog `set_reload_after_apply` + `settings_with_reload`):
```rust
fn settings_with_reload(current: &Settings, on: bool) -> Settings {
    Settings { reload_after_apply: Some(on), ..current.clone() }
}
fn set_reload_after_apply(&mut self, on: bool) {
    let next = settings_with_reload(&self.settings, on);
    let save_error = match &self.root { Ok(root) => settings::save(root, &next).err(), Err(_) => None };
    self.settings = next;
    if let Some(err) = save_error {
        self.config_message = Some((format!("Could not save ff-settings.json: {err}"), true));
    }
}
```
Copy for `set_league(name)` and `set_price_source(live)`. Never touches `ConfigState` or `apply_state`. Check Phase 37's `adopt_settings` helper when it lands.

**Test-safe I/O seam** (analog `resolve_root`): production fn plus `#[cfg(test)]` twin returning `Err("unit tests never resolve ...")`. Copy for `fetch_leagues()` calling `crate::sync::fetch_leagues_blocking().map_err(|e| e.to_string())`; the test twin returns `Err("unit tests never fetch the league list".into())`.

**Sync click** (replace the `OVERLAY_LEAGUE` use; current code in the frame loop):
```rust
if economy_clicks.sync && let Ok(root) = &self.root {
    let base_dir = root.clone();
    self.economy.start(move || crate::sync::sync_economy_blocking(&base_dir).map_err(|e| e.to_string()));
}
```
Add `let league = view.sync_league.clone()` and refuse when Fallback or no league; call `sync_economy_blocking(&base_dir, &league)`. Remove the `league: crate::sync::OVERLAY_LEAGUE` field in the `EconomyTabInputs { .. }` built in the `ShellTab::Economy` arm of the frame. Poll next to `if self.economy.poll() {` (once per frame); add `self.leagues.poll()`. Fetch trigger in the Economy arm only: `matches!(list.state(), NotStarted) && !self.is_shot()`.

**EconomyInfo** (struct with `mtime_secs, priced_items, live, problem`; `read(root)` uses `crate::loader::load_prices_optional`): add `league: Option<String>` from `PricesFile.league` (non-empty); update the `economy_info_reads_prices_json` test.

**staged economy preview** (analog `staged_economy_body` / `refresh_staged_economy`):
```rust
fn staged_economy_body(root: &Path, config: Option<&ConfigState>) -> Result<String, String> {
    ... dynamic::economy_body_with(&paths, PriceSourcePolicy::Auto, c.pending_styles(), c.pending_tiers()) ...
    None => dynamic::economy_body(&paths, PriceSourcePolicy::Auto),
```
Add a `policy` parameter; in `refresh_staged_economy` include `source_pending` in `wanted` for `EconomyPlan::Refresh` and add the policy to the cache key `staged_econ_inputs: Option<(StyleSet, Vec<TierEntry>)>` (compared with `.is_some_and(|(s, t)| s == styles && t.as_slice() == tiers)`).

**Unapplied marker**: the existing `style_pending` slot is computed in four places (`overwrite_decision`, ~L1898; the combined-dirty helper ~L2188; ~L2252; the frame body ~L3113):
```rust
let style_pending = self.working.as_mut().is_some_and(|w| w.style_rewrite_pending());
...
let combined_dirty = apply::combined_dirty(rules_dirty, style_pending, config_dirty);
```
OR `source_pending` into each (a small `fn source_pending(&self) -> bool` on `App`), and set `applied_policy` where `apply_state = Applied` is set and on filter (re)load. Do not feed it into `rules_revertible` / revert popup logic.

**Screenshot injection** (analog `tier_text` in the debug build path):
```rust
if let (Some((tier, text)), Ok(state)) = (&shot.tier_text, &mut app.config) { state.set_tier_text(tier, text); }
```
Write `league`/`price_source` to the temp `ff-settings.json` (via `settings::save`) before the first frame; `leagues.inject(..)` from `--leagues`.

**Per-frame tier gating**: set `config.set_tiers_in_use(!fallback)` before `bar_state` (`tier_errors: self.config.as_ref().is_ok_and(|c| c.has_tier_errors())`) and before `run_apply`.

### `src/overlay/screenshot.rs` (utility, transform)

**Analog:** `--tier-text` (struct field `tier_text: Option<(String,String)>`, local `let mut tier_text = None;`, match arm with `value("--tier-text")?`, error `format!("--tier-text needs TIER=TEXT, got `{t}`")`, plus tests `tier_text_is_read` and the `p(&[...])` helper). Enum-valued flags: copy `--dropdown` (`DROPDOWNS` const + `contains` check + `unknown dropdown` error) for `--price-source live|fallback`. Add `leagues: Option<String-or-enum>` (`none`, `loading`, or comma list), `league: Option<String>`, update the module doc header usage line and the `ShotArgs` construction. Phase 37 changes the temp-folder guard (`USERPROFILE` -> `FERRISFILTER_GAME_DIR`); follow what the file enforces at execution time.

### `tests/fixtures/index-state-2026-10-18.json`

Hand-write the six `economyLeagues` (name/url/hardcore/indexed, values in RESEARCH "Live poe.ninja shape") plus `oldEconomyLeagues`. Do not touch `tests/fixtures/index-state.json` (used by `tests/sync_prices.rs`).

## Shared Patterns

### Settings that are not edits
**Source:** `set_reload_after_apply` in `src/overlay/app.rs`. **Apply to:** `set_league`, `set_price_source`. Save immediately, keep in memory even on save failure, report via `config_message` with `Could not save ff-settings.json: <err>`, never mark CONFIG dirty.

### Framework-free model + table-driven tests
**Source:** `src/overlay/economy.rs` module doc and tests (`settle`, `a_started_sync_is_running_until_it_reports`). **Apply to:** `leagues.rs`, `source_pending`, `view()`. No egui types in signatures.

### No network / no real folders in tests
**Source:** `resolve_root` `#[cfg(test)]` twin; fake `HttpClient` in `sync/metadata.rs`. **Apply to:** league fetch, anything touching `%APPDATA%\FerrisFilter`. Run lib tests with `FERRISFILTER_HOME` and `FERRISFILTER_GAME_DIR` set to a temp dir; integration tests pin `FERRISFILTER_HOME` (isolation_guard).

### Greying a control with a tooltip
**Source:** `shell::economy_tab` Sync button (`ff_button_ex(.., enabled, ..)` then `r.on_hover_text(tip)` when disabled) and `config_view::show_filter` `add_enabled_ui`. **Apply to:** league dropdown, refresh, Sync, Tiers editor, Reset button.

### Status-line priority
**Source:** `shell::status_for` chain (`root_error`, `event`, `grammar_gate`, `filter_error`, `game_folder_missing`, `syncing`, [`ended_league` new], `unknown`, `migrated`, `absent`, indicator). Add a priority test: ended beats unknown, loses to syncing.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | | | Every new element has a same-file or sibling analog. The only genuinely new logic is the pure effective-source marker comparison (`source_pending`) and the `view()` decision table, specified in 39-RESEARCH.md Patterns 3 and 5. |

## Cross-phase hot spots (re-read at execution)

`app.rs` (Economy arm of the frame, `StatusInputs` build, `handle_config_actions`, `economy_clicks` handling), `shell.rs` (`economy_tab`, `EconomyTabInputs`, `EconomyClicks`, `StatusInputs`), `config_view.rs` (`show_tiers_section`) are also edited by 37 and 38 (refresh icon button, `game_folder_missing`, `Reset to defaults` / `tiers_reset_enabled` / `reset_tiers`). Plans must `depends_on` those plans and every struct-literal site (including tests) must gain the new fields.

## Metadata

**Analog search scope:** `src/sync/`, `src/settings.rs`, `src/overlay/{app,shell,economy,apply_all,config_state,config_view,screenshot}.rs`, `src/skin/{widgets,editor}.rs`, prior pattern maps 37/38
**Files scanned:** about 14 read or grepped
**Pattern extraction date:** 2026-10-18
