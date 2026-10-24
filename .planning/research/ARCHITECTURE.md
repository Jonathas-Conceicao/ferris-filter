# Architecture Patterns: v0.7.2 Small UI Improvements

**Domain:** FerrisFilter (Rust, egui/eframe overlay + CLI) - integrating eight features into the shipped v0.7.1 architecture
**Researched:** 2026-10-23
**Overall confidence:** HIGH for integration points (read from the code), MEDIUM for egui drag/auto-scroll behaviour and global-hotkey rebind behaviour (not yet spiked)

All paths are relative to `C:\Users\jonat\Repositories\poe2_myfilter`. Line numbers are from the tree at research time and will drift.

## Headline Findings

1. **No new crate is needed for any of the eight features.** Details at the end ("New dependency review"). Nothing here should be added to `Cargo.toml`.
2. **The "style from action lines" parser already exists**: `rule_style_view()` in `src/overlay/body.rs` (about line 2646) turns a rule's action entries plus Show/Hide into a `StyleView` and reports which lines it cannot show exactly (`StyleBinding.inexact`), and `style_leftovers()` lists repeated-family and unsupported lines. Auto Detect should wrap these, not rewrite them. What is missing is `StyleView -> Style` (a small `to_style()`).
3. **A single emitter already exists for style actions**: `emitter::style_action_lines()` (`src/emitter.rs` about line 221) feeds `generate`, the economy section (`generate_economy`) and the styled-rule rewrite (`styled::rewrite_block`). Style alpha is therefore a one-function emitter change plus a type change on `Style`; there are not three emitters to patch.
4. **The Rules editor already has an alpha-capable colour path** (`StyleView.alpha = true`, `widgets::colour_slot_rgba`, `StyleIntent::Colour { value: Option<[u8;4]> }`). The CONFIG editor is the opaque one. Alpha for styles mostly means flipping CONFIG onto the Rules path, and then deleting the opaque path.
5. **Drag reorder cannot be built from the existing `Move(id, Dir)` primitive.** `WorkingFilter::move_rule` (`src/overlay/rules.rs` about line 1699) is a one-step swap/hop, and its `moved across Continue` bookkeeping toggles per step. A drag needs a new "move run to insertion point" primitive in `src/filter/structure.rs` and a net-crossing computation.
6. **All three network actions already take a `work` closure** (`EconomySync::start`, `LeagueList::start`, `UpdateJob::start`). Throttling is therefore a wrapper at three call sites in `src/overlay/app.rs`, not a rewrite of the workers.
7. **Auto Detect is a cross-model edit** (ConfigState gets new styles, WorkingFilter gets tags) while Revert is per tab. This is the main design risk of the milestone (see Pitfall A below).

## Recommended Architecture (delta view)

The overlay keeps its existing layering: framework-free models (`overlay/rules.rs`, `config_state.rs`, `leagues.rs`, `economy.rs`) -> views (`rule_list.rs`, `config_view.rs`, `body.rs`, `style_lines.rs`) -> skin widgets (`src/skin/`). New work follows the same split: pure logic in a framework-free module with unit tests, drawing in the view/skin layer.

```
                      +---------------------------+
   Config tab view -> | ConfigAction (new:        |
   (config_view.rs)   |  AutoDetect, MoveStyleTo, |
                      |  SetHotkey, ResetHotkey)  |
                      +-------------+-------------+
                                    v
 OverlayApp (app.rs) --- handle_config_actions / dispatch ---+
   | config: ConfigState (pending styles/tiers)              |
   | rules : WorkingFilter (doc, tags, moved flags)          |
   | settings: Settings  (+ hotkey key)                      |
   | throttle: RequestLog (NEW, persisted)                   |
   | hotkey_manager (kept alive even if register failed)     |
   +--> style_detect::plan(...)  (NEW, pure)                 |
   +--> Document::move_run_to(...) (NEW, filter/structure.rs)+
   +--> compile::load_economy_with(pending styles, tiers) -> pricer (tier labels, NEW) -> emitter
```

## New vs Modified, per Feature

### 1. Style colour alpha

**Modify**
- `src/models/config.rs`: `Style.text/border/background` change from `Option<[u8; 3]>` to `Option<Colour>`.
  - Recommended `Colour { rgb: [u8;3], alpha: Option<u8> }` with a custom serde impl: accept a 3-array or a 4-array, serialise a 3-array when alpha is `None` and a 4-array otherwise. **Normalise alpha 255 to `None` on load** so equality (dirty checks, Auto Detect) is structural and files without alpha re-serialise byte-identically. A hand-written `[r,g,b,255]` would then re-serialise as 3 values on the next styles.json write; that loses no meaning and only happens when the overlay writes the file anyway. Document it.
  - Alternative (sibling `textAlpha` fields) keeps `[u8;3]` consumers untouched but doubles the fields in every comparison; not recommended.
  - Add `Style::same_look(&self, other)` that compares every field except `id` (used by Auto Detect; `PartialEq` includes `id`).
- `src/emitter.rs` `style_action_lines()`: write `alpha.unwrap_or(255)` instead of the literal `255` (three lines). The fixed overrides (`background_override: Option<[u8;3]>` in `src/pricer.rs`/`src/sync/mapping.rs` and the bronze `175 96 37 240` in `emit_unique_sections`) stay as they are; they replace the style's background and are not styles.
- `src/overlay/style_lines.rs`: `StyleView::from_style` carries real alpha and sets `alpha: true`; once both callers are on the alpha path, delete the `alpha` flag and the opaque branch (`widgets::colour_slot`, `SlotResult`) if nothing else uses it (3 sites in `src/overlay/widgets.rs`).
- `src/overlay/config_state.rs`: `StyleEdit::Colour { value: Option<[u8;3]> }` becomes `Option<[u8;4]>` (or `Option<Colour>`); `edit_style` maps it onto `Colour`.
- `src/overlay/config_view.rs`: `label_look`/`style_row` previews already take `StyleView`; verify the item-label preview composites alpha (it paints `from_rgba_unmultiplied` already in `style_lines::label_look`).
- `src/loader.rs` / id assignment / inline-section comparison use `Style: PartialEq`; compile-fix only.
- Tests: emitter unit tests that set `st.text = Some([1,2,3])` (lines ~1026-1095) need the new type; add alpha cases.

**Do not touch:** `styles-default.json`, `tests/fixtures/seed-styles.json`, `seed-universal.filter`. Styles without alpha must keep emitting `... 255`; `artifact_guard` is the proof.

**Docs to update when done:** CLAUDE.md `styles.json` row and the Config note currently state "styles store RGB and the filter always emits alpha 255, so the style colour picker has no alpha".

### 2. Auto Detect styles

**New**
- `src/overlay/style_detect.rs` (framework-free, no egui; same contract as `economy.rs`/`leagues.rs`):
  - `style_of_rule(entries: &[BodyEntry], keyword: BlockKeyword) -> Option<Style>`: calls `body::rule_style_view`, returns `None` when `keyword == Minimal`, when `style_leftovers(..)` is non-empty (repeated family lines, unsupported actions such as `CustomAlertSoundOptional`, `...IfAlertSound`), or when `binding.inexact` is non-empty. Otherwise `StyleView::to_style()`.
  - `plan(rules: &[Candidate], pending: &StyleSet) -> DetectPlan`:
    - `Candidate { id: BlockId /* group leader */, look: Option<Style>, tagged: bool, disabled_ok }`.
    - Match order: built-ins in set order, then customs in set order, each with `Style::same_look` against the **pending** values; first match wins -> `tag(id, name)`.
    - Remaining distinct looks (deduped by `same_look`, in first-seen file order) -> `new_styles: Vec<(String, Style)>` named `Custom Style N` (N counts up, skipping names already present) and their rules tagged.
    - Inexpressible rules -> counted as `left_custom`, untouched.
  - A bare `Show` equals `unchanged` automatically when the pending `unchanged` is still blank; a bare `Hide` is distinct from `hidden` (font 21 + drop sound) and becomes a custom style. That falls out of exact equality; no special-casing.
- `StyleView::to_style(&self) -> Option<Style>` in `src/overlay/style_lines.rs` (Show/Hide only; sound volume uses the shown volume, so a line with no volume reads as the game default 50/100, which matches what `style_action_lines` would write).
- `ConfigState::add_styles(Vec<(String, Style)>)` in `src/overlay/config_state.rs`: loops `StyleSet::insert_new` (which assigns ids and appends after built-ins), as one batch.
- `WorkingFilter::tag_rules(&[(BlockId, String)]) -> ActionOutcome` in `src/overlay/rules.rs`: one doc clone, `set_meta(id, "style", tag)` for every member of each group, commit once, bump `generation`. Use `set.tag_for(name)` for the canonical `<id> <name>` form. Do **not** go through `RowAction::ApplyStyle` per rule: that path calls `rewrite_block` immediately, which rewrites a 3-token colour to 4 tokens and reorders lines to canonical order. (Apply's `rewrite_styled_with` will do that canonicalisation later anyway for every tagged rule; see Pitfall B.)
- A way to enumerate candidates without drawing: `WorkingFilter::rows()` already excludes economy rows (`economy_rows()` is separate) and counts a `# ff:group` once; `body_view(id)` returns the entries `rule_style_view` needs; `RuleRow.style` (`StyleState`) tells whether a rule is already tagged.

**Modify**
- `src/overlay/config_view.rs`: a **Auto Detect** button in the Styles section next to Reset to defaults / + Add style; `ConfigAction::AutoDetect`. Disabled (tooltip says why) when no filter is loaded, the filter has no untagged rules, or styles failed to load.
- `src/overlay/app.rs` `handle_config_actions`: order matters:
  1. build candidates from `self.rules` and the pending set from `self.config`;
  2. `config.add_styles(new_styles)`;
  3. sync the rules model to the new pending set (the existing `working.set_styles_over(config.pending_styles().clone(), config.applied_styles())` call that follows config edits, lines ~335/690) so the new names resolve;
  4. `rules.tag_rules(tags)`;
  5. status-line event message: `Auto Detect: N tagged, M new styles, K left Custom`.
- The staged economy preview re-prices automatically when styles change (existing behaviour); nothing to add.

**Data flow:** purely in-memory. Nothing is written until the one shared Apply, which already writes `styles.json` (CONFIG has edits) and the filter (rules edited) all-or-none. No Apply-pipeline change is needed.

### 3. Drag-and-drop reordering (rules and custom styles)

**New**
- `src/filter/structure.rs`: `Document::move_run_to(run: &[BlockId], at: InsertAt) -> Result<(), EditError>` where `InsertAt` is `BeforeBlock(BlockId) | AfterBanner(section ordinal) | BeforeBanner(section ordinal) | EndOfUserSections` (the position just above the economy banner). It must be a line-level move because "dropped at the bottom of section A" and "dropped at the top of section B" are the same gap between two blocks but differ by whether the banner line comes before or after the rule. Reuse the region logic of `swap_runs` (a run travels with the blank lines after each block; ids follow blocks) and the refusal rules of `rule_step`/`move_neighbour` (never into or below the economy section; `# ff:group` run stays contiguous).
  - Fallback if time is short: loop `rule_step` + `swap_runs`/`move_section`. It works (500 rules is cheap) but see Pitfall C for the Continue flag.
- `src/overlay/rules.rs`: `RowAction::MoveTo { id: BlockId, at: DropTarget }`, handled in `dispatch` beside `Move`; it computes `moved` (the "moved across Continue" set) from the **set of rules the run passed over**: flagged when any passed-over rule or the moved rule has `Continue`, cleared otherwise. `coalesce_edits`/`action_target` get the new variant. Keep `Move`/`MoveSection` (section headers keep their arrows unless the author decides otherwise).
- `src/overlay/drag_reorder.rs` (framework-free helper shared by both lists, with unit tests):
  - `DragState { key: DragKey, grab: Vec2 }` (stored in `ListUi` and `ConfigUi`).
  - `drop_slot(offsets: &[f32], y: f32, valid: impl Fn(usize) -> bool) -> Option<usize>`: nearest valid gap, built on `virtual_list::build_offsets_mixed` (which already gives every row top, including the variable-height expanded row).
  - `autoscroll_step(pointer_y: f32, viewport: Range<f32>, dt: f32) -> f32`: speed ramps up inside an edge zone (about 48pt), 0 outside.
  - Escape/outside-list cancellation helpers.
- `src/skin/widgets.rs`: `ff_drag_handle(ui, skin, enabled) -> Response` (`Sense::drag`), drawing the three states.
- `src/skin/textures.rs`: add `reorder_handle: [TextureHandle; 3]` to `Skin` via `tex!(ctx, "custom/icon_reoder_normal.png")` (hover, pressed likewise). The author's files are named `icon_reoder_*` (typo in the filename); either keep the names or rename the six files (png + ase) in one commit before wiring. Check the PNG sizes against the 24x24-ish handle slot (not measured here).

**Modify**
- `src/overlay/rule_list.rs`: replace the two `ff_arrow` calls in the rule row (about line 1098) with the handle; in `show_list` (line ~458) keep the `reorder_enabled` flag (false while searching: handle drawn greyed with the existing "Clear the search to reorder" tooltip). Drag lifecycle inside `show_list`:
  - on `drag_started`: collapse the expanded row (`list.expanded = None`) so offsets are uniform; record `DragState`.
  - each frame while dragging: compute slot from the pointer and the cumulative offsets, paint the drop marker line, paint the shadow, apply wheel + edge autoscroll by passing the new `offset` to `ff_scroll_area_viewport` (it already accepts `offset: Option<f32>`; the viewport rect gives the current offset).
  - on release over a valid slot: push `RowAction::MoveTo`; on Escape or release outside the list: cancel.
  - economy rows and the locked header: no handle, never a valid slot; the slots at or below the economy header are invalid.
- Shadow rendering: re-run the row's draw function into a `Ui` on a `Tooltip`-order layer with reduced opacity and throw-away action/frame sinks. There is precedent for this "read-only copy with throwaway state" in the economy rows, which run the editor body against a throwaway list state. (MEDIUM: `Ui::multiply_opacity`/layer painter behaviour should be confirmed in egui 0.36 with a spike.)
- `src/overlay/config_view.rs` `style_row` (about line 1057): handle in the Order column for custom rows (built-ins keep the greyed placeholder); `ConfigAction::MoveStyle { name, up }` becomes `MoveStyleTo { name, index }`. Style rows are few and not virtualised, so slots come from the collected row rects, not offsets.
- `src/models/config.rs` `StyleSet`: `move_custom_to(name, index)` next to `move_custom` (line ~913), clamped so a custom style never lands above a built-in; `ConfigState::move_style_to` mirrors `move_style` (line ~624) and is one pending edit, reverted by Config Revert.

**Order of implementation inside the phase:** the pure pieces first (`drop_slot`, `autoscroll_step`, `move_run_to` + tests), then the Config table (small, not virtualised, proves the handle/shadow/marker), then the virtualised Rules list.

### 4. Section header redraw (Social-panel style)

**Modify**
- `src/overlay/rule_list.rs` `draw_section_header` (line ~629): replace the flat `pal::PANEL` row and 1px lines by the game's bar (three-slice) and a separate toggle plate with two glyph states (up chevron above a dot = expanded, dot above a down chevron = collapsed). Keep: whole bar clickable, title left-aligned, `{n} rules`, rename pencil, arrows, remove x, economy padlock. The existing separate chevron hit-rect (`chev`) becomes the plate. If a 3px gap between bars is added, change only the heights fed to `virtual_list::build_offsets_mixed` (`SECTION_HEADER_HEIGHT`), nothing else in the virtual list.
- `src/skin/textures.rs`: `section_bar: ThreeSlice`, `section_toggle: [TextureHandle; N]` (or plate + painted glyphs).
- `assets/poe2/manifest.toml` + `tools/ff-art`: new `[[image]]` entries found with `ff-art list`. This step needs the author's PoE2 install (committed PNGs are used afterwards, so CI and other developers are unaffected). The backlog's stated fallback (paint plate and glyph with vectors in `src/skin/icons.rs`, where `FlagIcon::ChevronRight/ChevronDown` already exist) should be the plan B.
- `FlagIcon::ChevronRight/ChevronDown` become unused after the redraw unless the Config tables use them; remove dead code in the Release Review.
- The economy section shares this function (padlock, collapsed default), so test both.

### 5. Search field icons (clear X + magnifier)

**New**
- `src/skin/widgets.rs`: `ff_search_field(ui, id: Option<Id>, text: &mut String, hint: &str, width: f32) -> SearchResponse { response, cleared }`. Internally extract the body of `ff_text_field_ex` into a private function taking `left_inset`/`right_inset`; `ff_text_field_ex` calls it with the current 10/10 insets so every other field's layout (and its layout probes in tests) is unchanged. The search field uses a larger left inset for the lens and a right inset for the X disc; a click on the X clears the text and calls `request_focus()` (keeps focus), shown only while the field has text.
- Art: cream magnifier and the cream disc with a dark X from the Social panel (`.planning/research/ui-refs/social-search-fields.png`), either as extracted textures (ff-art `[[image]]`) or vector-painted (`skin/icons.rs`). Same ff-art caveat as section headers; do both extractions in one session.

**Modify (the two existing call sites)**
- `src/overlay/rule_list.rs` (about line 1465): today the RULES search draws its own text "Clear" inside the field and handles Escape. Replace the field and that block with `ff_search_field`; keep the Escape behaviour.
- `src/overlay/body.rs` (about line 1315): the **+ condition** picker's search (no clear affordance today) uses the same widget; it requests focus on first frame, preserve that.

### 6. Customizable global hotkey

**Modify**
- `src/overlay/hotkey.rs`: stop hardcoding. `register_toggle()` becomes `register(hotkey: HotKey) -> (GlobalHotKeyManager, HotkeyStatus)` returning the manager **even on a failed registration** (today it returns `None`), because a later rebind needs a live manager. Add `rebind(manager: &GlobalHotKeyManager, old: Option<HotKey>, new: HotKey) -> Result<HotkeyStatus, String>`: register the new combo first, unregister the old one only on success, so a conflict leaves the working binding in place. `release_toggle` takes the current `HotKey`. Keep `DEFAULT_HOTKEY = Ctrl+Shift+Z` as the fallback; the "hardcoded" unit test becomes a test of the default.
- `src/settings.rs`: `Settings.hotkey: Option<String>` using `global_hotkey::hotkey::HotKey`'s `Display`/`FromStr` (both exist in global-hotkey 0.8: `into_string`, `impl FromStr`) as the stored text; `Settings::hotkey_binding() -> (HotKey, Option<String /*warning*/>)` returning the default plus a warning for a wrong type or unparseable value (same pattern as `price_source` and `league`). Persist with `save_keys(root, settings, &["hotkey"])` (add `save_hotkey`), so it never rewrites an unusable file. Update the "unusable file means reload off" paragraph in CLAUDE.md if relevant: an unusable `ff-settings.json` also means the default hotkey.
- `src/overlay/mod.rs` `launch()`: it registers the hotkey and builds the tray before `OverlayApp::new`, and today the settings are loaded inside the app. Resolve the FerrisFilter folder and read the saved hotkey **before** registering (fall back to the default on any error). Registering the default and then rebinding in `OverlayApp::new` would briefly take `Ctrl+Shift+Z` from another program and is not recommended. The tray tooltip text (`FerrisFilter - {combo} to toggle`) uses the binding text.
- `src/overlay/tray.rs` / `src/overlay/app.rs`: update the tray tooltip on rebind (confirm `tray-icon` `set_tooltip` is reachable through `TrayHandles`; MEDIUM).
- `src/overlay/app.rs` (the poll at about line 3501): compare `event.id` with the **current** registered id (already read from `self.hotkey_status`, so updating the status on rebind is enough). Keep `win::foreground_is_game_or_overlay`.
- `src/overlay/config_view.rs`: a "Show / hide hotkey" row (Filter section is the natural home; or a small "Overlay" section) with the current combo, a **Change** button that arms capture, and **Reset**. `ConfigAction::SetHotkey(HotKey)` / `ResetHotkey`. Saved at once, like the league and price-source picks (not a pending edit; does not mark anything unapplied).
- Status line: a failed rebind keeps the old binding and says so; a failed startup registration keeps the existing tray tooltip/menu message.

**New**
- `src/overlay/hotkey_keys.rs`: capture helper and `egui::Key -> global_hotkey::hotkey::Code` table, plus `egui::Modifiers -> global_hotkey::hotkey::Modifiers`. Use `Event::Key { physical_key, pressed, modifiers, .. }` so the layout does not matter. Rules to decide: require at least one modifier (recommended, to avoid swallowing plain typing keys system-wide, the reason the original binding became Ctrl+Shift+Z), Escape cancels, modifier-only presses are ignored, and refuse a few reserved combos.
- Capture state in `ConfigUi`. **While capture is armed, ignore global-hotkey events** (or unregister temporarily): pressing the current combo would otherwise hide the overlay in the middle of capturing it (Pitfall E).
- Use `global_hotkey::hotkey::{Code, Modifiers}` (re-exported) - **do not** depend on `keyboard-types` directly.

**Thread affinity:** `rebind` runs from the egui update loop, which is the main thread that created the manager and runs the Win32 message loop (the same constraint `hotkey.rs` and `tray.rs` already document). Do not move it to a worker. Lib tests must never register real hotkeys; follow the existing `#[cfg(test)]` twin pattern used for `fetch_leagues`/`resolve_root`.

### 7. Tier price range in economy names

**Modify**
- `src/pricer.rs`: `PricedData` gains `tier_labels: BTreeMap<String, String>` (tier letter to the suffix, e.g. `"< 2 chaos"`, `">= 30 chaos"`), filled in `price_tiers()` from the `ValidatedTiers` it already receives (before normalisation: use each entry's own `unit`, `Unit::as_str()` is `chaos`/`exalt`). Rule: a tier covers `[its min, next higher tier's min)`, so show the upper bound `< next.min`; the top tier (S) shows `>= its.min`. If the next tier is in a different unit (not expected, validation is per unit group), omit the suffix rather than guess. Number format `fmt_amount(f64)`: shortest form, no trailing zeros (`0.5`, `2`, `12`). `PricedData::empty()` gets an empty map.
- `src/emitter.rs`: `emit_currency_sections` and `emit_unique_sections` already take `tags: bool`; pass the label map and change the two `# ff:name` writes (lines ~554 and ~632) to `"{tier} tier {category} {label}"`. `generate` (tags = false) writes no `# ff:name`, so `seed-universal.filter` is byte-identical. The `# ff:style <id> <T>` line is unchanged.
- Pending tiers flow through unchanged: `compile::load_economy_with(paths, policy, styles, live_tiers)` already re-prices with the overlay's pending tier edits, so the staged-economy preview renames itself when a cutoff changes. The fallback pair uses its own `tiers-fallback.json` units (exalt), also automatic.
- Tests that pin the old names must change: `src/emitter.rs` (~1275, ~1278), `src/dynamic.rs` (~608), `src/overlay/app.rs` (~6809 `contains(" tier ")` is safe). Nothing outside tests parses `# ff:name` back (search matches substrings, the RULES row shows it), so the old names are not load-bearing.
- Glyph decision: `>=` is safe in the filter file and in Fontin; `≥` needs a font-coverage check (the filter header already contains a non-ASCII em dash, so the file itself is fine, the overlay font is the question). Recommend `>=` unless a screenshot shows the glyph renders.
- One-time effect: an existing economy section is regenerated on the next Apply, so the first Apply after upgrading shows a text change in the economy section; expected.

### 8. Request throttling (and distinct CLI user agent)

**New**
- `src/overlay/throttle.rs` (framework-free):
  ```rust
  pub enum RequestKind { Sync(String /*league*/), Leagues, UpdateCheck }
  pub struct RequestLog { /* BTreeMap<String key, u64 unix seconds> */ }
  pub enum Decision { Go, FakeRefresh { age_secs: u64 } }
  impl RequestLog {
      pub fn decide(&self, kind: &RequestKind, now: u64) -> Decision; // window 3600 s
      pub fn record_success(&mut self, kind: &RequestKind, now: u64);
  }
  ```
  A failed request never calls `record_success`. A stored time in the future (clock moved back) counts as expired (`Go`), never as a lockout.
- Persistence: recommended a separate state file in the FerrisFilter folder (for example `ff-state.json`) written with the same atomic temp-file persist pattern as `settings::save_keys`, holding the request times **and the cached results a fake refresh must replay** (the league list, the last update-check outcome). Reasons: `ff-settings.json` is a hand-editable preferences file with "unusable file means defaults and reload off" semantics; mixing machine-written cache into it complicates that contract. The author's backlog suggested `ff-settings.json`; that also works (new keys through `save_keys`) and is the smaller change, so this is a decision to put to the author. Either way resolve the path in `src/locations.rs` only, and keep `tests/isolation_guard.rs` happy (tests pin `FERRISFILTER_HOME`).
- The "fake refresh" must run the **normal flow**, so it returns a synthetic worker result through the same channel:
  - **Sync (per league):** `EconomySync::start(|| Ok(league))` immediately. Only fake when `prices.json` exists, parses and records the same league; otherwise `Go` (never claim a refresh the disk cannot back up). The success message and `refresh_economy_info()` then behave exactly as after a real sync.
  - **League list:** `LeagueList::start(|| Ok(cached_list))`. A cached list must exist on disk for the common case "restart, open Economy tab (auto-fetch)" to be throttled; if no cache exists, `Go`.
  - **Update check:** replay the cached outcome. Open question: a "newer version found" result involves a download in `%TEMP%` that is swept; recommended rule is to throttle only an "up to date" result and, when a newer release was found, re-verify the local download without network or `Go`.

**Modify**
- `src/overlay/app.rs`: wrap the three call sites: `economy.start(...)` for Sync (~line 4069), the league-list start (uses `fetch_leagues`, ~line 383) including the first-visit auto fetch, and `start_update_check` (~line 2227). Add a `RequestLog` field to `OverlayApp`, record on a *successful real* result in `poll_economy`/league poll/`receive_update_message`. The status line should read the same as a real refresh (author's "looks the same" requirement); optionally tooltip the age.
- `src/sync/client.rs`: split the user agent. Keep `USER_AGENT` for the overlay and add `CLI_USER_AGENT` (for example `FerrisFilter-CLI/<ver> (<repo>)`); `ReqwestClient::new()` takes the agent (or add `new_cli()`), used by `sync::cmd_sync` -> `run` for the CLI, while `sync_economy_blocking` / `fetch_leagues_blocking` / `ReqwestUpdateHttp` (src/update.rs ~841) use the overlay agent. Update `user_agent_names_the_public_repo` and add a CLI one.
- The CLI (`ff-cli sync`, `sync metadata`) is **not** throttled and does not read the log.
- Screenshot mode and tests never reach the network (existing rule); the new `RequestLog` must be injectable (pass `now` explicitly, never read the clock inside `decide`).

## Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `models::config::Style` / `Colour` (modified) | Style data incl. alpha, `same_look` | loader, emitter, config_state, style_lines |
| `emitter::style_action_lines` (modified) | Single definition of a style's lines | generate, economy, styled rewrite |
| `overlay::style_detect` (new, pure) | Rule -> `Style`, plan tags/new styles | body (`rule_style_view`), config_state, rules |
| `overlay::rules::WorkingFilter` (modified) | `tag_rules`, `MoveTo` | filter::structure, style set |
| `filter::structure::Document::move_run_to` (new) | Line-level run move | rules.rs |
| `overlay::drag_reorder` (new, pure) | Slot/auto-scroll arithmetic | rule_list, config_view, virtual_list |
| `skin::widgets` (`ff_drag_handle`, `ff_search_field`) (new) | Drawing only | views |
| `overlay::hotkey` (modified) + `hotkey_keys` (new) | Register/rebind/capture mapping | app, settings, tray, config_view |
| `overlay::throttle` (new, pure) | Decide Go / FakeRefresh, persist | app, settings/state file, workers |
| `pricer::tier_labels` (new) | Range suffix per tier | emitter, compile |
| `sync::client` (modified) | Per-surface user agent | sync, update |

## Data Flow Changes

- **Pending-edit models:** unchanged in shape. Config (styles, tiers) and Rules (doc) stay separate and are written by the one shared Apply, all-or-none. The only new cross-model edit is Auto Detect (styles added to Config, tags added to Rules). Drag reorder adds a Rules edit (`MoveTo`) and a Config edit (`MoveStyleTo`), each reverted by its own tab's Revert exactly like the arrows today.
- **Apply pipeline (`src/overlay/apply_all.rs`):** no structural change. Alpha rides inside `Style` -> `style_action_lines`; tier names ride inside `generate_economy`; Auto Detect output is ordinary pending styles + tags. `rewrite_styled_with` still canonicalises every tagged rule at Apply.
- **Settings persistence:** one new key (`hotkey`), saved at once through `save_keys`; throttle state in a new state file or new keys (decision). Unknown keys are preserved by `save_keys` in either case.
- **Network:** three kinds gain a gate; CLI is unchanged apart from its user agent.

## Patterns to Follow

### Pattern 1: Pure model + thin view
New logic (`style_detect`, `drag_reorder`, `throttle`, `tier_labels`) goes into framework-free modules with unit tests; the views only draw and emit actions. This is how `leagues.rs`, `economy.rs` and `virtual_list.rs` are built and tested.

### Pattern 2: One action vocabulary, deferred mutation
Rows push `RowAction`/`ConfigAction` values during drawing; `apply_action` mutates afterwards. Drag release pushes one `MoveTo`; it never mutates during the frame.

### Pattern 3: `work` closure seam
Workers take a closure (`EconomySync::start`, `LeagueList::start`, `UpdateJob::start`) and the production body is `#[cfg(not(test))]`. The throttle decides which closure to hand over (real or synthetic).

### Pattern 4: Settings written key-by-key
`settings::save_keys` writes only the changed key and never replaces a file it could not parse. Reuse it for the hotkey.

## Anti-Patterns to Avoid

- **Per-step `Move` calls for a drag.** Quadratic cloning, wrong Continue flag, no banner-side control. Use `move_run_to`.
- **Applying the style through `ApplyStyle` in Auto Detect.** Rewrites bytes at once and one rule at a time (many undo-visible edits); use the batch tag.
- **Registering the default hotkey and then rebinding.** Steals a combo from another app at start-up.
- **Reading the wall clock inside the throttle decision.** Makes tests flaky; pass `now`.
- **Persisting alpha 255 as a 4-array.** Breaks byte-identical round trips for equal styles; normalise to `None`.

## Pitfalls Specific to This Integration

**A. Auto Detect and per-tab Revert (HIGH).** Revert is per tab: Rules Revert drops the tags but leaves the new `Custom Style N` in Config; Config Revert removes the styles while rules still carry tags to them, which then show `unknown style` (flagged, never blocking; Apply then drops the tag via `drop_unknown_style_tags`). This is consistent with the stated "Revert/Reload work as usual" but will surprise users. Options: (1) accept and say so in the status message, (2) have Config Revert of an Auto-Detect-created style also strip the tags it added (needs a small provenance record), (3) a single "Undo Auto Detect" snapshot. Decide before planning; (1) is cheapest.

**B. Detection is semantic, Apply is canonicalising (MEDIUM).** Equality is on parsed values, but a tagged rule is rewritten at Apply to canonical order and 4-token colours, and the rewrite replaces whole action lines (trailing `# comments` on action lines go with them). A rule with a trailing comment on an action line should probably be treated as "stay Custom" or the loss accepted explicitly. Also a rule whose actions are exactly a style's but in another order is detected as a match and reordered on Apply.

**C. `moved across Continue` for multi-gap moves (HIGH).** `move_rule` sets/clears `self.moved` per step. A dragged move must compute the flag from the whole set of rules passed over (and from the destination), not toggle per step. Also keep a `# ff:group` run contiguous and moving as one unit.

**D. Drop slot vs banner ordering (HIGH).** Gaps adjacent to a section header have two meanings (end of the section above vs start of the one below). The drop marker must show which; `InsertAt` must carry it. The divider right below the economy section and the economy header itself are invalid targets.

**E. Hotkey capture eats its own trigger (HIGH).** While capture is armed the current global combo still fires (the overlay is foreground, which the hotkey allows) and hides the window. Suppress toggles during capture.

**F. Hotkey: manager must outlive a failed registration (MEDIUM).** Today a failed registration drops the manager, so a user who picked a free combo after a conflict at start-up could never rebind. Keep the manager always.

**G. Throttle fake refresh can lie (MEDIUM).** Faking a sync when `prices.json` is missing, for another league, or newer than the log entry gives stale data with a success message. Gate on the file actually matching. Same for a league list with no cache.

**H. Alpha in previews (LOW).** The CONFIG row preview and item-label preview now composite semi-transparent colours; check they draw over the background rather than replacing it.

**I. Section-header height change (LOW).** Anything that hard-codes `SECTION_HEADER_HEIGHT` (probes in `rule_list.rs` tests, `scroll_offset_for` jump-to-rule) must use the same constant.

## Suggested Build Order

Dependencies: alpha before Auto Detect (equality includes alpha); art extraction before the widgets that need it; section header and search field before drag (same files, `rule_list.rs` and `Skin::load`; avoids merge conflicts and re-testing the same list twice); everything that edits `config_view.rs` (alpha editor, Auto Detect button, hotkey row, style-row handle) is serialised across phases rather than parallelised.

| # | Phase | Contents | Why here |
|---|-------|----------|----------|
| 1 | **Style alpha** | `Colour` type, `Style::same_look`, emitter, `StyleView`/CONFIG editor on the alpha path, remove opaque path | Foundation; unblocks Auto Detect; byte-identical guard is the acceptance test |
| 2 | **Auto Detect styles** | `style_detect.rs`, `StyleView::to_style`, `ConfigState::add_styles`, `WorkingFilter::tag_rules`, Config button, status message | Needs alpha; highest semantic risk, so early; no new art |
| 3 | **Economy tier-range names** | `pricer` labels, emitter name writes, test updates | Small, backend-only, independent; `generate` fixtures untouched |
| 4 | **Request throttling + CLI user agent** | `throttle.rs`, state persistence, three call-site wrappers, `client.rs` split | Independent of UI work; touches `app.rs` call sites once |
| 5 | **Customizable hotkey** | `hotkey.rs` rework, settings key, `launch()` ordering, capture row, tray tooltip | Independent; spike global-hotkey rebind first |
| 6 | **Game-art widgets: search icons + section headers** | ff-art manifest entries (needs the author's game install), `ff_search_field`, `draw_section_header` redraw | Shared art session and shared `Skin` additions; precedes drag |
| 7 | **Drag-and-drop reorder** | `move_run_to`, `RowAction::MoveTo`, `drag_reorder.rs`, `ff_drag_handle`, Config table then Rules list, remove rule/style arrows | Largest; builds on the redrawn rows and handle texture |
| 8 | **Release Review** | Clean-up (opaque colour path, dead `FlagIcon`s), screenshots (add debug `--screenshot` flags for new states, for example hotkey capture armed, drag in progress), version 0.7.2, CHANGELOG draft, author review | Fixed by project convention |

Phases 3, 4 and 5 are independent of each other and could be reordered or run in parallel by file ownership (3: `pricer`/`emitter`; 4: `app.rs` call sites + `sync`; 5: `hotkey`/`settings`/`mod.rs`), subject to the `config_view.rs` serialisation above (only 5 touches it).

**Research flags for phases**
- Phase 2: needs a design decision on Pitfall A (Revert coupling) before planning; otherwise standard.
- Phase 5: short spike on `global-hotkey` 0.8 register-new-then-unregister-old behaviour and conflict errors, and on `tray-icon` tooltip update from the live tray handle.
- Phase 6: `ff-art list` exploration for the Social panel art (the backlog gives a texture directory hint only); keep the vector-painted fallback.
- Phase 7: spike egui 0.36 behaviour for wheel scrolling plus a forced scroll offset during a widget drag, and layer opacity for the shadow; everything else is standard.
- Phases 1, 3, 4, 8: standard patterns, unlikely to need deeper research.

## Scalability Considerations

| Concern | ~50 rules | ~500 rules (NeverSink-sized) | Notes |
|---------|-----------|------------------------------|-------|
| Drag slot lookup | trivial | binary search on cumulative offsets (`partition_point`) | reuse `virtual_list` |
| Auto Detect | trivial | one `body_view` + parse per untagged rule, run once on click | not per frame; cache nothing |
| `move_run_to` | trivial | one line-vector rebuild per drop | same cost class as `swap_runs` |
| Economy names | none | none | string formatting at Apply/preview only |

## New Dependency Review (flagged for the author)

**None required.** Checked each feature for a possible crate and found the existing dependencies sufficient:

| Feature | Possible temptation | Verdict |
|---------|--------------------|---------|
| Drag and drop | `egui_dnd` or similar | Not needed and a poor fit (virtualised list, custom shadow, section constraints); use egui pointer/drag responses and `drag_reorder.rs` |
| Hotkey capture | `keyboard-types` (a dependency of `global-hotkey`) | Use the `Code`/`Modifiers` re-exported by `global_hotkey::hotkey`; hand-write the `egui::Key` table. Do not add `keyboard-types` as a direct dependency |
| Throttle time | `chrono`/`time` | `std::time::SystemTime` seconds are enough; display-time conversion already exists via `Win32_System_Time` |
| Alpha, Auto Detect, tier labels | none | pure std/serde |
| Search icons, section art | image crates | `image` (png) is already a dependency; assets are committed PNGs through `tex!` |
| Persisted state | new serializer | `serde_json` + `tempfile` are already used by `settings::save` |

If the Phase 7 spike shows egui's built-in behaviour cannot do wheel + auto-scroll during a drag, the fix is still our own code (offset control through `ff_scroll_area_viewport`'s `offset` parameter), not a crate.

## Open Decisions for the Author

1. Alpha representation: 3/4-array `Colour` with 255 normalised away (recommended) versus sibling alpha fields.
2. Auto Detect Revert coupling (Pitfall A) and how to treat rules whose action lines carry trailing comments (Pitfall B).
3. Do section headers get a drag handle too (the backlog's open point), or keep the arrows (this research assumes they keep them).
4. Throttle persistence: separate state file (recommended) versus keys in `ff-settings.json`; what an update-check fake refresh replays when a newer version was found.
5. CLI user-agent string and whether the overlay agent changes at all.
6. Hotkey rules: require a modifier (recommended); behaviour for combos another app already owns (keep old binding, report).
7. Tier-range glyph `>=` versus `≥`.
8. Filename typo `icon_reoder_*`: keep or rename before wiring.

## Sources

- Code read for this document (HIGH): `src/models/config.rs`, `src/emitter.rs`, `src/styled.rs`, `src/pricer.rs`, `src/compile.rs`, `src/settings.rs`, `src/overlay/{rules,body,style_lines,config_state,config_view,rule_list,virtual_list,hotkey,mod,app,leagues,economy,update_job}.rs`, `src/skin/{widgets,textures,icons}.rs`, `src/sync/{client,mod}.rs`, `src/update.rs`, `src/filter/structure.rs`.
- Project documents (HIGH): `CLAUDE.md`, `.planning/PROJECT.md`, `.planning/ROADMAP.md` backlog items 999.19 to 999.27.
- `global-hotkey` 0.8.0 source in the local cargo registry (HIGH for `HotKey::into_string`/`FromStr`/`Display` existence; rebind and conflict behaviour not yet verified, MEDIUM).
- egui 0.36 drag/scroll/opacity behaviour: not verified against docs in this pass (LOW to MEDIUM; spike in Phase 7).
