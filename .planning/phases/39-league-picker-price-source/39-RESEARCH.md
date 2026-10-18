# Phase 39: League Picker & Price Source - Research

**Researched:** 2026-10-18
**Domain:** Rust / egui overlay (FerrisFilter): poe.ninja league list on a worker thread, a saved price-source setting threaded into the Apply pricing path, Economy-tab widgets
**Confidence:** HIGH (everything below was read in the code on disk in this worktree; the live poe.ninja shape was supplied by the orchestrator)

## Summary

No new crates and no new skin widgets. The league list reuses the existing `index-state` fetch (`sync::league::fetch_index_state`, generic over the `HttpClient` trait, so it fakes with a struct like `FakeClient` in `src/sync/metadata.rs`). The picker is `ff_dropdown_ex`, the switch is `ff_segmented`, the refresh button is Phase 37's `widgets::ff_icon_button(ui, &skin.refresh, "Refresh", enabled)` (already committed, 37-02, commit 1cffa12). The price source is a new `ff-settings.json` key that selects `PriceSourcePolicy::Auto` or `ForceFallback` at the **four** places the overlay hard-codes `Auto` today (Apply's `apply_all::prepare`, the staged-economy preview `staged_economy_body`, the economy-section refresh preview, and the Economy tab's "Last sync / Priced items" info stays about `prices.json`).

Four findings the planner must act on:

1. **`apply_all::prepare` has no policy parameter.** It hard-codes `PriceSourcePolicy::Auto` (apply_all.rs ~L189/193). Thread the policy through `FilterInput` (one new field `policy: PriceSourcePolicy`; 7 construction sites: app.rs ~L601 and 6 in apply_all.rs tests). Economy pricing only happens inside the `filter` branch, so this is the only plumbing Apply needs. `compile::load_priced_with` under `ForceFallback` already ignores the live tiers (pending or on disk) and loads the fallback pair, and `PricedData`/`PriceSource::Fallback(Forced)` come back; no compile.rs change is needed.
2. **The unapplied marker must not price per frame.** The UI-SPEC says "derived by comparing output"; do it as a pure comparison of the *effective source* instead: remember `applied_policy` (the policy in force when the filter was last loaded/applied this session) and mark when `effective(applied_policy) != effective(current_policy)` and the filter has an economy section. `effective(policy, prices_live)` = Fallback if `policy == ForceFallback` or `!economy_info.live`, else Live. Live without a usable `prices.json` is already fallback pricing, so toggling Live <-> Fallback there produces identical output and correctly does not mark. Feed the flag into the **existing `style_pending` slot** (`style_pending || source_pending`) for `combined_dirty` and `footer_actions_combined`, and add it to the Economy marker in `tab_markers`. Do not feed it into `rules_revertible` / `rules_revert_live` (Revert must not undo it and must not open a popup). A sync or league pick never touches it.
3. **Under Fallback the Tiers editor needs a real "tiers unused" path, not only greying.** `ConfigState::validate()` (config_state.rs ~L529) refuses Apply on any row error, and `has_tier_errors()` drives `bar_state.tier_errors` / the Apply gate. With the editor greyed, a user who typed `abc` then switched to Fallback would have Apply blocked by a field they cannot fix. Add a way to ignore row errors while Fallback is on (recommended: `ConfigState::set_tiers_in_use(bool)` set by the app each frame before the gate is computed; `validate()` and `has_tier_errors()` return the "not in use" answer when false; `tier_rows()` keeps reporting `error`/`edge` so the inline red text returns when the editor is enabled). Pending tier edits stay in memory, `tiers_dirty()` stays true, the Economy ` *` stays (UI-SPEC rule). `prepare_write` compares `self.tiers` (last valid values) with `tiers_applied`, so a bad buffer never reaches `tiers.json`.
4. **Phase 37/38 are mid-flight in this worktree** (uncommitted edits to app.rs, config_view.rs, shell.rs, folder_dialog.rs, tests/isolation_guard.rs). Plans for Phase 39 must anchor on symbol names, never line numbers, and must re-read each file at execution time. Everything this phase depends on from them is named in "Cross-phase dependencies".

**Primary recommendation:** Build bottom-up: (a) pure model (`sync::league` list + default pick, `Settings` keys, `overlay::leagues` state machine + the pure UI-state table), (b) policy plumbing (`FilterInput.policy`, preview, marker, tier-validation bypass), (c) `shell::economy_tab` widgets, (d) app wiring + status line, (e) screenshot flags + CLAUDE.md. Keep the table-driven pure functions as the test surface so no test needs a window or a network.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LEAGUE-01 | League picker lists poe.ninja's current leagues; choice saved; replaces `OVERLAY_LEAGUE` | `league::list_leagues(&client)` (new, generic over `HttpClient`) -> `Vec<EconomyLeague>`; worker `LeagueList`; `Settings.league`; `ff_dropdown_ex`; delete `OVERLAY_LEAGUE` (sync/mod.rs L61, app.rs ~L3248) |
| LEAGUE-02 | Sync uses the chosen league and its outcome names it | `sync_economy_blocking(base_dir, league)`; `run_sync_prices` already returns the resolved league and `economy::success_message(league)` already names it; nothing to add to the message |
| LEAGUE-03 | Saved league no longer listed reported explicitly, never switched silently | `leagues::view()` pure table -> "<name> (ended)" synthetic option, Sync disabled, `StatusInputs.ended_league`; no write until a pick |
| LEAGUE-04 | Live / Fallback choice; Fallback disables Sync + picker; next Apply prices from the fallback pair | `Settings.price_source` -> `PriceSourcePolicy`; `FilterInput.policy`; `ff_segmented`; marker via effective-source comparison |
| LEAGUE-05 | Under Fallback the Tiers editor is greyed with a tooltip (fallback tiers fixed) | `show_tiers_section(.., enabled)` inside `add_enabled_ui`; Phase 38's Reset button forced disabled; `ConfigState` tiers-not-in-use path (finding 3) |
</phase_requirements>

## Architectural Responsibility Map

Single-process desktop app (egui); "tiers" are the layers of the crate.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| League list fetch + parse + default pick | `sync::league` (pure + async over `HttpClient`) | `sync::mod` (blocking wrapper owning a tokio runtime) | Same layer as the existing `fetch_index_state`; testable with a fake client |
| League list state machine (NotStarted/Loading/Loaded/Failed) | `overlay/leagues.rs` (new, framework-free) | `app.rs` (owns it, polls once per frame) | Same contract as `overlay/economy.rs`: no egui types, unit-testable |
| Picker/Sync/status decision table | `overlay/leagues.rs` pure fn `view(...)` | `shell.rs` renders it | UI-SPEC state table becomes one table-driven test |
| Saved league + price source | `src/settings.rs` (`Settings`, load salvage, save) | `app.rs` (`set_league`, `set_price_source`) | Same as `reload_after_apply` |
| Pricing by source | `compile::PriceSourcePolicy` (unchanged) | `overlay/apply_all.rs`, `app.rs::staged_economy_body` (pass it) | Policy already exists and is tested |
| Greying + tooltips | `shell::economy_tab`, `config_view::show_tiers_section` | `skin::widgets` (no change) | Views render `EconomyTabInputs`; they never decide |
| Status line | `shell::status_for` (+ `StatusInputs.ended_league`) | `app.rs` builds the input | One priority table, already tested |
| Screenshot injection | `overlay/screenshot.rs` (`ShotArgs`) | `app.rs` build (debug) | Never fetches in screenshot mode |

## User Constraints (from CONTEXT.md)

### Locked Decisions
(verbatim from 39-CONTEXT.md `<decisions>`)

**League picker**
- The Economy tab's "League (pinned)" row becomes a dropdown of poe.ninja's PoE2 economy leagues, from the `index-state` endpoint sync already uses (`economyLeagues`: name, slug, hardcore, indexed). A refresh button beside it (Phase 37's) re-fetches the list.
- The list is fetched on a worker thread the first time the Economy tab is opened in a session and on refresh; nothing is fetched at start-up.
- Default when nothing is saved: the first league in poe.ninja's list that is neither Standard nor Hardcore. The `indexed` flag is not trusted. The league is written to `ff-settings.json` only when the user picks one.
- Offline / fetch failed: the picker shows the saved (or default) league name and stays usable for Sync; its tooltip says the list could not be loaded.

**League end (LEAGUE-03)**
- A saved league no longer in the list: the picker shows "<name> (ended)" in amber, Sync economy is disabled, and the status line says "<name> is no longer on poe.ninja; pick a league". Nothing is switched automatically.
- Prices already synced for the old league keep working for Apply until the next sync; Last sync still shows the league it priced.

**Price source (LEAGUE-04 / LEAGUE-05)**
- A `Live | Fallback` segmented switch on the Economy tab; tooltip recommends Fallback for league start and non-trade players.
- Saved at once in `ff-settings.json` (like the reload toggle). When the loaded filter has an economy section, a change marks unapplied changes, because Apply will write different prices.
- Under Fallback: Sync economy, the league picker, the league refresh button and the Tiers editor (with its Reset to defaults) are greyed, each with a tooltip (the fallback tiers are fixed).
- Live with no `prices.json` yet (fresh install): unchanged behaviour, priced from fallback with the reason on hover, until the first Sync.

**Defaults and CLI**
- New installs start on Live.
- The CLI keeps its own `--league` and automatic resolution (dev tool); it does not read the overlay's league setting.

### Claude's Discretion
- Worker extraction (a generic worker from `EconomySync`), settings key names, the exact status texts beyond those above, how the "(ended)" state is modelled.

### Deferred Ideas (OUT OF SCOPE)
None. (Out of scope by implication: reworking CLI `generate`/`apply`, listing `oldEconomyLeagues`, auto-refresh timers.)

## Project Constraints (from CLAUDE.md and the orchestrator)

- Plans must **not** edit `CHANGELOG.md` or `README.md` (human-written; orchestrator/author own them).
- **CLAUDE.md must be updated** (allowed): delete the `OVERLAY_LEAGUE` pinned-league note (the "Economy sync from the overlay" paragraph: "the league is pinned in `sync::OVERLAY_LEAGUE` ... Update that constant at the start of each league ..."), the "League (pinned)" mention in the tabs paragraph ("**Economy** holds League (pinned), Last sync ... Price source ..."), and describe: the league dropdown + refresh, Live | Fallback switch, the two new `ff-settings.json` keys (add to the `ff-settings.json` row), the new status-line condition in the priority list, the new `--leagues` / `--league` / `--price-source` screenshot flags (the `ff` row's flag list), and that the CLI still resolves automatically. Keep edits accurate and light (memory: human-written docs).
- Tests never touch the real `%APPDATA%\FerrisFilter`, the real game folder, or the network. Unit tests in the lib use the `#[cfg(test)] fn resolve_root() -> Err(..)` pattern (app.rs ~L333); follow it for the league fetch: `#[cfg(test)]` version of the production fetch function returns `Err("unit tests never fetch")`. Integration tests that spawn binaries must pin `FERRISFILTER_HOME` (isolation_guard).
- Overlay UI "not texty" (memory): no standing paragraphs; explanations are tooltips. UI-SPEC already conforms.
- Run lib tests with `FERRISFILTER_HOME` and `FERRISFILTER_GAME_DIR` pointed at a temp dir (37-02 summary did this).
- Layer rule: model files (`economy.rs`, new `leagues.rs`, `config_state.rs`, `settings.rs`, `apply_all.rs`) contain no egui types.

## Standard Stack

No new dependencies. [VERIFIED: Cargo.toml read this session]

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| reqwest (rustls-no-provider) + tokio rt | 0.13 / 1 | league list GET on a worker thread | already used by `sync::client::ReqwestClient` |
| serde / serde_json | 1.0 | `IndexState`, `Settings` | already used |
| egui / eframe | 0.36 | dropdown, switch, disabled `Ui` | already used |
| tempfile | 3 | settings round-trip tests | already used |

**Package Legitimacy Audit:** not applicable: this phase installs no external packages (no `Cargo.toml` change is needed by any requirement). slopcheck not run. `rfd` (Phase 37) is not used here.

## Live poe.ninja shape (supplied by the orchestrator, observed 2026-10-18, HTTP 200)

`GET https://poe.ninja/poe2/api/data/index-state` keys: `economyLeagues`, `oldEconomyLeagues`, `snapshotVersions`, `buildLeagues`, `oldBuildLeagues`. `economyLeagues`, in order (name / url / hardcore / indexed; `displayName` equals `name` on each):

1. Forbidden Rites / forbiddenrites / false / false
2. Runes of Aldur / runesofaldur / false / **true**
3. HC Forbidden Rites / forbiddenriteshc / true / false
4. HC Runes of Aldur / runesofaldurhc / true / false
5. Standard / standard / false / false
6. Hardcore / hardcore / true / false

`oldEconomyLeagues`: Fate of the Vaal (`vaal`), HC Fate of the Vaal (`vaalhc`).

Consequences:
- `models::IndexState`/`EconomyLeague` already deserialize this (no `deny_unknown_fields`; `displayName` and the extra top-level keys are ignored). [VERIFIED: models.rs]
- The default rule picks **Forbidden Rites** although `indexed` is true on Runes of Aldur: the exact case the CONTEXT distrusts the flag for.
- `tests/fixtures/index-state.json` already exists but is the **old** Fate of the Vaal shape (4 leagues). Add a new fixture `tests/fixtures/index-state-2026-10-18.json` holding the six `economyLeagues` entries above (and `oldEconomyLeagues`, trimmed of the huge `snapshotVersions`/`buildLeagues`; the file need not be the full 28 KB, but keep the key names exact) and use it, via `include_str!`, in unit tests and the fake `HttpClient`. Do not touch the old fixture (used by `tests/sync_prices.rs`).

## Architecture Patterns

### Data flow

```
ff-settings.json (app root = %APPDATA%\FerrisFilter)
  league: Option<String>        price_source: Option<String> ("live"|"fallback", else Live)
        |                              |
        v                              v
 App.settings  ------------------> price_policy() -> PriceSourcePolicy::{Auto, ForceFallback}
        |                              |-> FilterInput.policy -> apply_all::prepare -> dynamic::economy_body[_with]
        |                              |-> staged_economy_body(root, config, policy)  (Add preview / Refresh preview)
        |                              |-> source_pending (marker) -> style_pending slot, Economy tab ` *`
        |                              |-> EconomyTabInputs.fallback  (greys switch rows, Sync, picker, refresh, tiers, Reset)
        v
 Economy tab opened first time ──> LeagueList::start(fetch_leagues_blocking)  [worker thread, tokio runtime inside]
        |                              (never in tests / screenshot mode; refresh button restarts it)
        v
 LeagueList::poll() once per frame -> ListState {NotStarted|Loading|Loaded(Vec<EconomyLeague>)|Failed(String)}
        |
        v
 leagues::view(saved, list_state, prices_league, fallback, syncing) -> LeagueView
        {options, selected, caption_kind(Value|Disabled|Warn), dropdown_enabled, refresh_enabled,
         refresh_tip, dropdown_tip, sync_league: Option<String>, sync_tip: Option<&str>, ended: Option<String>}
        |-> shell::economy_tab draws it, returns EconomyClicks{source, league_pick, refresh_leagues, sync, ...}
        |-> StatusInputs.ended_league (warning, jump to Economy tab)
        v
 Sync click -> economy.start(|| sync::sync_economy_blocking(&root, &league))  (league from view().sync_league)
```

### Recommended file changes

```
src/sync/models.rs        # EconomyLeague (+IndexState) derive Debug, Clone, PartialEq
src/sync/league.rs        # + list_leagues(&client) -> Vec<EconomyLeague>; + default_league(&[EconomyLeague]) -> Option<&EconomyLeague>
src/sync/mod.rs           # delete OVERLAY_LEAGUE; sync_economy_blocking(base_dir, league: &str); + fetch_leagues_blocking()
src/settings.rs           # + league, price_source keys; price_policy(); load() salvage arms
src/overlay/leagues.rs    # NEW framework-free: ListState, LeagueList (worker), LeagueView + view()
src/overlay/mod.rs        # pub(crate) mod leagues;
src/overlay/apply_all.rs  # FilterInput.policy -> prepare passes it to economy_body[_with]
src/overlay/config_state.rs # tiers-in-use flag (validate/has_tier_errors)
src/overlay/config_view.rs  # show_tiers_section(.., enabled)
src/overlay/shell.rs      # EconomyTabInputs/EconomyClicks, economy_tab rows, StatusInputs.ended_league, tab_markers unchanged signature
src/overlay/app.rs        # fields, polling, handlers, marker, preview policy, status inputs, screenshot injection
src/overlay/screenshot.rs # --leagues, --league, --price-source
tests/fixtures/index-state-2026-10-18.json  # NEW
CLAUDE.md                 # see Project Constraints
```

### Pattern 1: League list is pure over `HttpClient` (fake, no network)
`sync::league` already has `fetch_index_state<C: HttpClient>(&C)`. Add:

```rust
// Source: src/sync/league.rs pattern (fetch_index_state) + src/sync/client.rs (HttpClient)
pub async fn list_leagues<C: HttpClient>(client: &C) -> miette::Result<Vec<EconomyLeague>> {
    Ok(fetch_index_state(client).await?.economy_leagues)
}

/// First non-hardcore league that is neither Standard nor Hardcore; `indexed` is ignored.
pub fn default_league(leagues: &[EconomyLeague]) -> Option<&EconomyLeague> {
    leagues.iter().find(|l| !l.hardcore && l.url != "standard" && l.url != "hardcore")
}
```

`HttpClient::get_json` returns `impl Future` without `Send`; tests drive it with a hand-rolled `block_on` or a tiny `tokio::runtime::Builder::new_current_thread()` (tokio `rt-multi-thread` + `macros` features are enabled; use `tokio::runtime::Runtime::new().block_on` as `sync/mod.rs` does). Copy the `FakeClient` shape from `src/sync/metadata.rs` ~L162-L200 (a struct returning a recorded string, or an error).

The default rule skips `hardcore == true` entries too (UI-SPEC researcher decision 1): "HC Forbidden Rites" must never be the default for a softcore player. Over the live list this yields Forbidden Rites; over `[Standard, Hardcore]` it yields `None`.

### Pattern 2: Worker state machine, framework-free
Keep `EconomySync` as is (it has 15 tests; extraction is discretionary and risky). Add `overlay/leagues.rs` with a small struct mirroring it:

```rust
// Source: src/overlay/economy.rs (EconomySync::start/poll) - same mpsc + thread::spawn shape
pub enum ListState { NotStarted, Loading, Loaded(Vec<EconomyLeague>), Failed(String) }
pub struct LeagueList { state: ListState, rx: Option<Receiver<Result<Vec<EconomyLeague>, String>>> }
impl LeagueList {
    pub fn start<F>(&mut self, work: F) where F: FnOnce() -> Result<Vec<EconomyLeague>, String> + Send + 'static
    pub fn poll(&mut self) -> bool   // call once per frame; Disconnected => Failed("the league worker stopped unexpectedly")
    pub fn inject(&mut self, state: ListState)   // screenshot mode + tests
}
```
`start` is a no-op while `Loading` (refresh disabled while a fetch runs anyway). A Failed/Loaded state is *kept* while a refresh runs? UI-SPEC: refresh "starts a new fetch"; the state table has "Loading" rows. Recommended: on refresh keep showing the last good list (do not blank the dropdown): store `last_good: Option<Vec<..>>` so a refresh does not flash "Loading…" or flip the ended-state; the spec's Loading rows apply to the first fetch. If this is judged out of spec, say so in the plan; either way the table in `view()` takes `ListState` only.

Production work function (the only place a runtime/network is touched), in `sync/mod.rs`:
```rust
pub fn fetch_leagues_blocking() -> miette::Result<Vec<EconomyLeague>> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| miette::miette!("Cannot create async runtime: {e}"))?;
    rt.block_on(async { let client = ReqwestClient::new()?; league::list_leagues(&client).await })
}
```
App side: `#[cfg(not(test))] fn fetch_leagues() -> Result<Vec<EconomyLeague>, String> { crate::sync::fetch_leagues_blocking().map_err(|e| e.to_string()) }` and a `#[cfg(test)]` twin returning `Err("unit tests never fetch the league list".into())`.

Trigger: in the frame, `if self.tab == ShellTab::Economy && matches!(list.state(), NotStarted) && !self.is_shot() { list.start(fetch_leagues) }`. Idempotent, covers a status-line click that opens the tab, never at start-up, never refetches on tab switch. Repaint: the frame loop already `request_repaint_after(50ms)`; call `list.poll()` next to the other polls.

### Pattern 3: One pure decision function for the whole league UI (the UI-SPEC state table)
`leagues::view(inputs) -> LeagueView` is the table-driven test surface.

Inputs: `saved: Option<&str>`, `state: &ListState`, `prices_league: Option<&str>` (from `EconomyInfo`), `fallback: bool`, `syncing: bool`, `root_error_tip: Option<&str>`.

Effective league (what Sync uses): `saved`, else (Loaded) `default_league(list)`, else (Failed) `prices_league`, else none.

Rows to encode (copy UI-SPEC exactly): Loading+saved / Loading+none / Loaded selected / Failed+saved / Failed+none+prices league / Failed+none+nothing / Loaded+saved missing (ended) / Loaded+none+no default / Fallback overrides everything (all disabled, tip `Not used with Fallback prices.`). Also: `syncing` disables the dropdown and refresh. "Ended" = `Loaded` **and** `saved` set **and** not in list (exact name equality); `Failed`/`Loading` never produce ended. Options: names in list order; ended adds a synthetic first option `"<name> (ended)"`, selected, caption `WARN`. Failed with a known league: a single-entry list of that league. Sync tooltip priority: root error > `Not used with Fallback prices.` > `Loading the league list.` > `Press refresh to load the league list.` > `Pick a league first.`

A pick in the dropdown returns the *name* (map back from the option index, ignoring the synthetic ended entry); picking an entry equal to the effective default still saves it (explicit choice). Picking the synthetic entry (already selected) must be a no-op.

### Pattern 4: Settings keys follow `reload_after_apply`
`Settings` gains `league: Option<String>` and `price_source: Option<String>` (both `#[serde(default, skip_serializing_if = "Option::is_none")]`). Helpers: `price_policy() -> PriceSourcePolicy` (`Some("fallback")` => `ForceFallback`, anything else incl. absent/unknown => `Auto`; consider case-insensitive), `saved_league() -> Option<&str>` (non-empty after trim; an empty string reads as unset).

**Pitfall in `settings::load`:** the strict `from_str::<Settings>` fails if `league` is, say, a number, and the salvage branch (settings.rs ~L120-L190) then rebuilds `Settings { filter, game_folder, reload_after_apply, collapsed_sections, extra }`: add `league: None, price_source: None` there **and** `extra.remove("league")` / `extra.remove("price_source")` arms with a fallback sentence each (else the bad value stays in `extra` and is re-serialised, and no warning names it). An unknown `price_source` *string* is not an error (reads Live). An unusable whole file (not an object) yields `Settings{reload_after_apply: Some(false), ..Default}` which already drops both keys: fine (Live + default league). Add `settings_with_league(&Settings, &str)` / `settings_with_price_source` helpers in app.rs next to `settings_with_reload` (`..current.clone()` keeps unknown keys). `settings_after_reload` (merge on a warned reload) uses `current.clone()`, so the new keys survive a transient bad read. Re-check against Phase 37's `adopt_settings` helper when it lands.

Saving: `set_league(name)` / `set_price_source(live)` copy `set_reload_after_apply` (app.rs ~L1682): build next, `settings::save(root, &next)`, assign `self.settings = next` even when the save failed, put `Could not save ff-settings.json: <err>` in `config_message` (problem=true) on failure. These are not CONFIG edits: they never touch `ConfigState`, never set `apply_state`.

### Pattern 5: Policy plumbing
- `FilterInput { policy: PriceSourcePolicy }`; `prepare` uses it in both `economy_body_with` and `economy_body` calls. `run_apply`/`apply_everything` (app.rs ~L601) pass `self.settings.price_policy()`; it is a parameter of the free function (it takes `filter`/`config` args, not `&self`), so thread it as an argument. Existing apply_all tests set `policy: PriceSourcePolicy::Auto`. New tests: with a seeded live `prices.json` and `ForceFallback`, `prepare(...)` yields `used_fallback()==true` and a body different from Auto.
- `staged_economy_body(root, config, policy)`; `refresh_staged_economy` must (a) include `source_pending` in `wanted` for `EconomyPlan::Refresh`, and (b) add the policy to the cached key `staged_econ_inputs` (currently `(StyleSet, Vec<TierEntry>)`; add a third element or a small struct) so the read-only economy rows are re-priced when the source flips. `prices_changed()` already clears the key after a sync.
- `EconomyInfo` gets `league: Option<String>` (from `PricesFile.league`, non-empty). Used for the Last sync hover (` - <league>`) and the failed-nothing-saved default. Update `economy_info_reads_prices_json` test.
- The marker: `applied_policy: PriceSourcePolicy` field on `App`, set at build and after every successful Apply/Overwrite (where `apply_state = Applied` is set) and when the filter is (re)loaded/switched/reverted-by-reload. Pure fn `source_pending(applied, now, prices_live, has_economy_section) -> bool`. `has_economy_section` = `working.economy()` is `EconomyView::Present` (a staged add also changes pricing but is already `economy_staged`).

Assumption to flag (A2): `applied_policy` at start-up is assumed to equal the policy the file was last written with. A file last written under the other policy by a previous session would not be marked until the user toggles. Cheap to accept; the alternative (diff against file bytes) would false-mark after any sync, which the spec forbids.

### Pattern 6: Economy tab layout (shell.rs `economy_tab`)
Rows 0..3: `Price source` (segmented), `League` (dropdown + refresh), `Last sync`, `Priced items`. All inside `ff_row`.
- Switch: `crate::skin::editor::ff_segmented(ui, skin, &mut idx, &["Live","Fallback"], CONTROL_WIDTH)`, `.on_hover_text(tip)`; `resp.changed()` => `clicks.source = Some(idx)`. Never disabled. Tooltip third line (reason) only when `policy==Live && !live`: reuse the two strings currently in the tab (`No prices.json yet...`, `prices.json could not be used (<problem>)...`), reworded per UI-SPEC.
- League row geometry (Phase 37 Filter row): inside the `ff_row` control `Ui` (left-to-right, 252pt): `add_enabled_ui(view.dropdown_enabled, |ui| ff_dropdown_ex(ui, skin, "economy-league", &mut sel, &opts, CONTROL_WIDTH - ROW_HEIGHT - FIELD_GAP, caption_color))`, `ui.add_space(FIELD_GAP)`, then `ff_icon_button(ui, &skin.refresh, "Refresh", view.refresh_enabled)`; tooltips via `on_hover_text` / `on_disabled_hover_text`. Constants `ROW_HEIGHT`, `FIELD_GAP`, `CONTROL_WIDTH` are in `skin::widgets`. [VERIFIED: ff_row / ff_icon_button read]
- `dropdown_shell` is painter-drawn with `Sense::click()`; a disabled `Ui` removes click sense and dims painter output (precedent: config_view.rs ~L327 `add_enabled_ui(ctx.filters.is_some(), ..)`). Verify the dim in the screenshot; if the dropdown does not dim, pass `caption_color = Some(pal::DISABLED)` as well.
- `ff_dropdown_ex` returns a `Response` whose `changed()` fires on pick; map `sel` index -> name via the view's option list.
- Sync button: `enabled = inputs.sync_enabled && !inputs.syncing && view.sync_league.is_some() && !fallback`; tooltip = first applicable per priority above (root error first, so keep `sync_disabled_tip` semantics). `sync_enabled` today is `actions.sync` (a footer-derived flag that includes root ok): keep it and AND the new conditions inside `economy_tab` or when building inputs in app.rs. Prefer app.rs so the click handler can use the same boolean (never trust the UI-disabled button alone: the handler also checks `view.sync_league` and `!fallback`).
- Tiers: `tiers` closure gets `locked: bool`; Phase 38's Reset button: `enabled = inputs.tiers_reset_enabled && !fallback`, tooltip `The fallback tiers are fixed.` when `fallback`, else 38's `Already at the defaults.`. Editor: `add_enabled_ui(!fallback, ..)` around `show_tiers_section`, with the tooltip on the scope response (`InnerResponse.response.on_disabled_hover_text(..)`). Risk (R1): in egui a scope response senses hover only; child widgets that claim hover may swallow it. If the tooltip does not appear in a manual check, add `ui.interact(group_rect, id, Sense::hover())` over the block's rect after the content and attach the tooltip there.

### Pattern 7: Status line
Add `ended_league: Option<String>` to `StatusInputs` (already has `game_folder_missing` from Phase 37 on disk) and a branch directly **after** `if inputs.syncing { ... }` and before `unknown`:
```rust
if let Some(text) = inputs.ended_league {
    return StatusLine { open_tab: Some(ShellTab::Economy), ..plain(text, StatusKind::Warning) };
}
```
`StatusLine.open_tab` and the clickable-status handling arrive with Phase 37-05 (check they landed; the field exists on disk already). The app builds `ended_league` as `view.ended.map(|n| format!("{n} is no longer on poe.ninja; pick a league"))` and only when `!fallback`. Update both `StatusInputs` constructors in tests (shell.rs `empty()` ~L1040 and app.rs test ~L5069 area) and add a priority test (`ended` beats `unknown`, loses to `syncing`).

### Anti-Patterns to Avoid
- **Fetching at start-up or in `build()`/tests/screenshot mode.** Trigger only from the Economy-tab branch of the frame, gated off in `#[cfg(test)]` and `is_shot()`.
- **Auto-switching league** when the saved one is missing, or writing the computed default to settings. Only an explicit pick writes.
- **Trusting `indexed`** anywhere in the overlay path. `resolve_league*` (CLI) keeps using it; do not touch.
- **Treating a failed fetch as "ended".** Unknown is not ended.
- **Pricing in the frame loop** to derive the marker (use the pure effective-source comparison).
- **Calling `sync_economy_blocking` with the effective league when Fallback is on**; the handler must refuse.
- **Editing the old `tests/fixtures/index-state.json`.**

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fake network | A mock server / env switch | `HttpClient` trait + a `FakeClient` struct (pattern in `sync/metadata.rs`) | Already the repo's seam |
| Worker + channel | Async runtime in the UI | `std::thread::spawn` + `mpsc` as in `EconomySync`; runtime created inside the thread | Proven; keeps UI thread free |
| Price-source branching | A second pricing path | `PriceSourcePolicy::{Auto, ForceFallback}` via `compile::load_priced_with` | Already pairs fallback prices with fallback tiers and ignores live tiers |
| Segmented control, dropdown, refresh button, disabled look | New widgets | `ff_segmented`, `ff_dropdown_ex`, `ff_icon_button`, `add_enabled_ui` | Spec forbids new widgets |
| League name matching in sync | Re-implement slug/name matching | `resolve_league_with_override(state, false, false, Some(name))` (already called by `run_sync_prices`; case-insensitive on name or slug; error lists available leagues) | A league that ended between listing and syncing fails loudly with the list |
| Settings persistence | Hand JSON writes | `settings::save` + helpers | Atomic write, unknown keys preserved |

## Runtime State Inventory

Not a rename/migration phase. State touched: `ff-settings.json` gains two optional keys (absent-tolerant, no migration; old files load unchanged); `prices.json` is only read for its `league` field (already in the format, `deny_unknown_fields` unchanged). No stored data, OS registrations, secrets or build artifacts carry the removed `OVERLAY_LEAGUE` string (grep: only `src/sync/mod.rs`, `src/overlay/app.rs`, CLAUDE.md). None found elsewhere. [VERIFIED: grep]

## Common Pitfalls

### Pitfall 1: Blocked Apply behind a greyed field
**What goes wrong:** pending bad tier text + Fallback => `validate()` refuses Apply, the editor is disabled, the user is stuck.
**How to avoid:** tiers-in-use flag (finding 3); test: bad buffer, Fallback on => `validate().is_ok()`, `apply_enabled`, Apply writes nothing to `tiers.json`; back to Live => error shows again and blocks.
**Warning signs:** `Config not applied: Fix the tier errors first` while the Tiers box is dim.

### Pitfall 2: Marker never clears / false-marks
**What goes wrong:** deriving from "tiers_dirty"/counters or from file bytes. **How to avoid:** the pure `source_pending` comparison; tests for: no section => false; Live<->Fallback with usable prices => true then false on switch back; with no usable prices => false both ways; a sync => unchanged; after Apply => `applied_policy` resets, marker clears.

### Pitfall 3: Stale staged-economy preview
**What goes wrong:** `staged_econ_inputs` key omits the policy, so flipping the source leaves the read-only rows priced with the old source. **How to avoid:** include policy in the key; test with the key logic as a pure helper.

### Pitfall 4: Revert popup logic
**What goes wrong:** the Economy tab's Revert ("Discard economy changes?") counts `tiers_dirty || economy_staged`. Do not add `source_pending` there. Revert must not undo the switch.

### Pitfall 5: Salvage branch drops/duplicates the new keys
See Pattern 4. Test: `{"league": 5, "price_source": true}` loads with `None`s, a warning naming both keys, and unknown keys intact; `{"price_source":"bogus"}` loads, no warning, policy Auto.

### Pitfall 6: Sync with a stale `OVERLAY_LEAGUE` replaced by an unlisted saved league
`run_sync_prices` re-fetches `index-state` itself and errors with the available leagues if the name is gone; the existing failure status carries it. Fine; do not pre-validate against the cached list in the click handler beyond "ended => disabled".

### Pitfall 7: Concurrent worker results after Fallback/league change
A league-list result landing after the user went to Fallback is harmless (state only). A sync in flight when the user flips to Fallback: allowed (spec); the result still lands in `prices.json`. Do not clear the sync message on flip.

### Pitfall 8: Two phases editing the same hot spots
`app.rs` (Economy tab inputs, `StatusInputs` build, `handle_config_actions`, `economy_clicks` handling), `shell.rs` (`economy_tab`, `EconomyTabInputs`, `EconomyClicks`, `StatusInputs`) and `config_view.rs` (`show_tiers_section`) are also edited by 37 and 38. Plans must declare `depends_on` the 37/38 plans that finish those files and re-read before editing. Struct-literal sites that must gain fields: every `EconomyTabInputs { .. }`, `EconomyClicks`, `StatusInputs { .. }` construction, including tests.

## Code Examples

### Effective-source and marker (pure)
```rust
// New, in overlay/leagues.rs or apply.rs (no egui)
pub fn effective_fallback(policy: PriceSourcePolicy, prices_live: bool) -> bool {
    policy == PriceSourcePolicy::ForceFallback || !prices_live
}
pub fn source_pending(applied: PriceSourcePolicy, now: PriceSourcePolicy,
                      prices_live: bool, has_economy_section: bool) -> bool {
    has_economy_section && effective_fallback(applied, prices_live) != effective_fallback(now, prices_live)
}
```

### Test: league default over the live list (no network)
```rust
// Source: tests belong in src/sync/league.rs #[cfg(test)]; fixture added by this phase
const LIVE: &str = include_str!("../../tests/fixtures/index-state-2026-10-18.json");
let state: IndexState = serde_json::from_str(LIVE).unwrap();
assert_eq!(default_league(&state.economy_leagues).unwrap().name, "Forbidden Rites");
assert!(default_league(&[standard(), hardcore()]).is_none());
```

### Test: list_leagues through a fake client
```rust
struct Fake(String);
impl HttpClient for Fake {
    fn get_json(&self, _u: &str, _q: &[(&str, &str)]) -> impl Future<Output = miette::Result<String>> {
        let b = self.0.clone(); async move { Ok(b) }
    }
}
let rt = tokio::runtime::Runtime::new().unwrap();
let list = rt.block_on(list_leagues(&Fake(LIVE.into()))).unwrap();
assert_eq!(list.len(), 6);
```

## State of the Art

| Old | Current | Impact |
|-----|---------|--------|
| `sync::OVERLAY_LEAGUE` literal, edited each league | saved choice from the live list | no code change per league; stale pin impossible, ended league explicit |
| Read-only "Price source" text row, derived from `prices.json` | user-chosen switch + tooltip reason | `EconomyTabInputs.live/problem` become tooltip inputs only |

**Deprecated:** `OVERLAY_LEAGUE`, its doc comment, the `Pinned in this build.` hover, and the CLAUDE.md note.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Setting key names `league` and `price_source` with values `"live"`/`"fallback"` (UI-SPEC suggestion; discretion) | Pattern 4 | Low; rename is local |
| A2 | `applied_policy` at start-up equals the policy the file was last written with; marker derived from effective-source comparison rather than literal output diff | Pattern 5 | Medium-low; a file written under the other policy by an earlier session is not flagged until toggled. Needs author OK or a different baseline |
| A3 | On refresh the last good list stays visible (no blank "Loading…") | Pattern 2 | Low; spec silent for refresh-after-success |
| A4 | A scope response from `add_enabled_ui` shows a disabled hover tooltip over the Tiers block (R1) | Pattern 6 | Low; fallback via `ui.interact` over the rect |
| A5 | `ff_dropdown_ex` dims under a disabled `Ui` through the painter opacity (not read in egui source this session) | Pattern 6 | Low; fix by also passing `DISABLED` caption colour |
| A6 | Exact (case-sensitive) name equality is the right "in the list" test for the saved league | Pattern 3 | Low; names come from the same field |

## Open Questions

1. **Ignore row errors under Fallback via `ConfigState` flag vs. app-side gating?**
   - Known: gates read `has_tier_errors()` / `validate()` in several places (`apply::config_apply_enabled`, `bar_state`, `OverwriteBlock`, `apply_all::prepare`).
   - Recommendation: the `ConfigState` flag (single source), set each frame from `settings.price_policy()` before gates are computed, and also set before `run_apply`.
2. **Does `ff-cli apply` honour `price_source`?** CONTEXT: CLI keeps its own behaviour; recommend no (it keeps `Auto`/`--early-game`). Mention in CLAUDE.md that only the overlay's Apply reads it.
3. **Screenshot `--leagues` parsing detail** (`HC ` prefix => hardcore; `Standard`/`Hardcore` slugs; others slug = lowercase name without spaces) is spec'd; `url` is not displayed, so exact slug values only matter for `default_league` (Standard/Hardcore must get `standard`/`hardcore`).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust nightly (rust-toolchain.toml) + cargo | build/tests/clippy/fmt | assumed present (project builds; Phases 37/38 ran `cargo test`) | nightly | none |
| Network / poe.ninja | nothing in tests | not used | n/a | fake `HttpClient`; live check already done |
| Windows desktop + GPU window | `ff --screenshot` captures | needed for the screenshot gate only | n/a | unit/headless tests cover logic |

Not probed in this session (planning-only run).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (lib + `tests/` integration), `insta` for the generate snapshot, egui headless `ctx.run_ui` for widget tests (see shell.rs `draw()`) |
| Config file | none (Cargo.toml; `rust-toolchain.toml` nightly) |
| Quick run command | `FERRISFILTER_HOME=<temp> FERRISFILTER_GAME_DIR=<temp> cargo test --lib <module>::` (e.g. `sync::league`, `overlay::leagues`, `settings`, `overlay::shell`, `overlay::apply_all`, `overlay::config_state`, `overlay::app`) |
| Full suite command | `cargo test` then `cargo clippy --all-targets -- -D warnings` then `cargo fmt --check` (run lib tests with the temp-dir env vars set; Windows: set them via the shell, never in code or `.cargo/config.toml`) |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LEAGUE-01 | `list_leagues` over the 2026-10-18 fixture returns 6 leagues in order; `default_league` = Forbidden Rites; `[Standard, Hardcore]` => None; HC never default | unit | `cargo test --lib sync::league` | Wave 0: new fixture + tests |
| LEAGUE-01 | `Settings` league/price_source round-trip, absent-tolerant, bad type salvage with warning, unknown keys kept | unit | `cargo test --lib settings::` | extend |
| LEAGUE-01 | Fetch trigger: first Economy visit starts once, tab switch does not refetch, refresh restarts, never in screenshot/test | unit (app) | `cargo test --lib overlay::app` | Wave 0 |
| LEAGUE-02 | Sync uses `view().sync_league`; `sync_economy_blocking` has no pinned constant (grep gate `OVERLAY_LEAGUE` returns nothing in `src/`) | unit + grep | `cargo test --lib overlay::leagues` ; `rg OVERLAY_LEAGUE src` | Wave 0 |
| LEAGUE-03 | `view()` table: ended detection, failed != ended, caption/colour/tips, Sync disabled; status-line priority (after syncing, before unknown; suppressed under Fallback) | unit | `cargo test --lib overlay::leagues overlay::shell::` | Wave 0 |
| LEAGUE-04 | `price_policy()`; `prepare` with `ForceFallback` => fallback body + `used_fallback`; marker table (`source_pending`); preview key includes policy; Revert unaffected; Sync/picker/refresh disabled under Fallback | unit | `cargo test --lib overlay::apply_all overlay::app overlay::shell::` | extend + Wave 0 |
| LEAGUE-05 | Tiers disabled under Fallback, Reset forced disabled with `The fallback tiers are fixed.`; bad tier text + Fallback does not block Apply, tiers.json unchanged; back to Live re-blocks | unit | `cargo test --lib overlay::config_state overlay::config_view overlay::shell::` | Wave 0 |
| all | `economy_tab` renders every state without panic and reports clicks (`draw()` helper) | unit (headless egui) | `cargo test --lib overlay::shell::economy_tab` | extend `inputs()` helper + clicks default test (add new fields) |

### Sampling Rate
- **Per task commit:** the module-scoped `cargo test --lib <module>::` for the touched module.
- **Per wave merge:** `cargo test` (all lib + integration), `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- **Phase gate:** full suite green plus the screenshot gate below, before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `tests/fixtures/index-state-2026-10-18.json` (the six `economyLeagues` + `oldEconomyLeagues`)
- [ ] `src/overlay/leagues.rs` (module + tests; register in `overlay/mod.rs`)
- [ ] `FakeClient` for `list_leagues` in `sync/league.rs` tests
- [ ] No framework install needed.

### Screenshot gate (debug build, executor looks at each PNG)
Every run with `FERRISFILTER_HOME` and `FERRISFILTER_GAME_DIR` under a fresh temp folder whose path has a `Temp`/`tmp` component (Phase 37 replaced the `USERPROFILE` requirement in the guard; the UI-SPEC still mentions `USERPROFILE`, follow what `screenshot.rs` enforces when the plan runs). Never the real `%APPDATA%\FerrisFilter` or game folder; the league fetch never starts. New flags: `--leagues "<a,b,..>"|none|loading`, `--league <name>`, `--price-source live|fallback`; extend `ShotArgs`, `parse`, its unit tests, the module docs, and `app.rs` debug injection (write `league`/`price_source` to the temp `ff-settings.json` before the first frame; `inject` the `ListState`).

UI-SPEC captures (all `--tab economy` unless noted): (1) live list, no saved: `Forbidden Rites`, Live lit; (2) `--league "Runes of Aldur"`; (3) `--league "Fate of the Vaal"`: `Fate of the Vaal (ended)` amber, Sync greyed, plus a `--tab rules` capture showing the status warning; (4) `--leagues none` with and without `--league`; (5) `--leagues loading`: `Loading…`; (6) `--price-source fallback`: switch on Fallback, dropdown, refresh, Sync, Tiers editor and Reset all dim; (7) `--filter <fixture with economy section> --price-source fallback` with a seeded live `prices.json` in the temp home: Economy tab label ends with ` *`; (8) pixel checks: refresh 34x34pt, 10pt gap, dropdown+button right edge aligned with the switch's right edge, row stripes 0..3.

## Security Domain

> `security_enforcement` not seen set to false in the files read; included. (`.planning/config.json` was not read in this run.)

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | n/a (desktop tool, no accounts) |
| V3 Session Management | no | n/a |
| V4 Access Control | no | n/a |
| V5 Input Validation | yes | league name from the network and from `ff-settings.json` is only ever compared to the list or passed as a `league=` query parameter through `reqwest`'s `.query()` (URL-encoded); never joined into a path or shell. Settings values: wrong types fall back per key with a warning |
| V6 Cryptography | no | TLS stays rustls via the existing client |
| V12 Communications | yes | HTTPS GET to the fixed `INDEX_STATE_URL`; 30 s timeout already on `ReqwestClient`; response parsed leniently, parse failure => `Failed`, never a panic |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Hostile/garbled index-state response (huge, wrong types) | Tampering / DoS | `serde_json` typed parse into `IndexState`; error => `ListState::Failed`; no `unwrap` on the worker |
| Tampered `ff-settings.json` league string with odd characters | Tampering | never used as a path; query-encoded; empty/whitespace treated as unset; displayed with truncation by the dropdown |
| Network call on the UI thread freezing an overlay over the game | DoS | worker thread; poll once per frame; no fetch at start-up |
| Test or screenshot run hitting the network / real folders | Info disclosure / Tampering | `#[cfg(test)]` stub, `is_shot()` gate, fake `HttpClient`, existing screenshot temp-folder guard, `isolation_guard` |

## Cross-phase dependencies (what 39 consumes; verify at execution time)

- **Phase 37 (on disk, committed):** `skin::widgets::ff_icon_button(ui, textures: &[TextureHandle; 3], label: &str, enabled: bool) -> Response`, `Skin.refresh`, 34x34pt (`ROW_HEIGHT`), disabled = dimmed normal art, honours a disabled `Ui`. Use: `w::ff_icon_button(ui, &skin.refresh, "Refresh", enabled)`; the 37-02 summary states there is no "R" fallback (art delivered). The existing `skin::editor::ff_icon_button` (minimap tile) is a **different** function; import the `widgets` one.
- **Phase 37 (in flight, uncommitted/not yet executed):** `game_folder` setting, `StatusInputs.game_folder_missing`, `StatusLine.open_tab` + clickable status, `adopt_settings`, config_view Filter row with refresh, screenshot guard change (`USERPROFILE` -> `FERRISFILTER_GAME_DIR`). 39 inserts its status branch right after `syncing`, which is after `game_folder_missing` in 37's order.
- **Phase 38 (planned):** `EconomyTabInputs.tiers_reset_enabled`, `EconomyClicks.reset_tiers`, a `Reset to defaults` `ff_button_ex(.., 150.0, ..)` at the right of the Tiers heading in `economy_tab`, `Popup::ResetTiers`. 39 ANDs `!fallback` into `tiers_reset_enabled` and swaps its tooltip. 38 also changes `styles-default.json` (no settings keys). Note 38 may change `tab_markers`/dirty counting (`tiers_dirty_count` unit-aware): re-read.
- Phase 36: `locations::from_env().root` is the app root holding `ff-settings.json`, `prices.json`; embedded fallback pair behind `InputPaths::prices_fallback/tiers_fallback` (`Source::Embedded`), so `ForceFallback` works with no files present.

## Sources

### Primary (HIGH confidence) - code read in this worktree
- src/sync/league.rs, models.rs, mod.rs, client.rs, metadata.rs (FakeClient), prices.rs (`run_sync_prices`, `PricesFile` in src/models/prices.rs)
- src/compile.rs (`PriceSourcePolicy`, `load_priced_with`, `load_economy[_with]`), src/dynamic.rs (`economy_body[_with]`), src/overlay/apply_all.rs (`prepare`, `FilterInput`)
- src/overlay/app.rs (EconomyInfo, `set_reload_after_apply`, `staged_economy_body`, `refresh_staged_economy`, Economy tab inputs, sync click, status inputs, `resolve_root` cfg(test) pattern), shell.rs (`economy_tab`, `status_for`, `tab_markers`), economy.rs (`EconomySync`), config_view.rs (`show_tiers_section`), config_state.rs (`validate`, dirty counts), src/settings.rs, src/skin/widgets.rs (`ff_row`, `dropdown_shell`, `ff_dropdown_ex`, `ff_icon_button`), src/skin/editor.rs (`ff_segmented`), screenshot.rs
- .planning/phases/39-.../39-CONTEXT.md, 39-UI-SPEC.md; 37-02-SUMMARY.md, 37-05/06 plans; 38-RESEARCH.md, 38-05 plan
- Orchestrator-supplied live poe.ninja observation (2026-10-18)

### Secondary / Tertiary
- none (no web lookups; none needed)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (no new dependencies; all reused APIs read)
- Architecture: HIGH (every integration point located; only the 37/38 in-flight edits are unverified)
- Pitfalls: HIGH for 1-6 (read in code); MEDIUM for R1/A5 (egui disabled-hover/dim behaviour not exercised)

**Research date:** 2026-10-18
**Valid until:** until Phases 37/38 land (their edits change anchors); the poe.ninja shape is stable short-term but the `indexed` flag is known to flip.
