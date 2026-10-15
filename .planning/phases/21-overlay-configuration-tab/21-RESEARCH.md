# Phase 21: Overlay Configuration Tab - Research

**Researched:** 2026-10-13
**Domain:** Brownfield Rust/egui 0.36 overlay (`ff`): CONFIG tab for filter selection, named styles (`styles.json`) and tier cutoffs (`tiers.json`)
**Confidence:** HIGH (every finding below was read from the code in this repo or from the installed egui 0.36.x sources)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Layout**
- **RULES | CONFIG** tabs at the top of the stripe. CONFIG has three sections: **Filter**, **Styles** and **Tiers**.

**Filter selection (STORE-02)**
- A dropdown of the `*.filter` files in the game folder, plus **New empty filter** (STORE-03 semantics from Phase 18).
- The choice is remembered across launches in a **git-ignored, per-user `ff-settings.json`** at the app root, resolved through Phase 18's resolver. The default is `dynamic.filter`.
- Switching filters while the rules tab has unapplied edits asks first; it never silently discards them.

**Style editor (CONF-02)**
- **List and preview:** styles are listed in file order, each row expanding into an editor, and each row shows a preview chip in its text, border and background colours.
- **Editable fields:** action (Show/Hide); text, border and background colours, through the egui colour picker with alpha; font size; alert sound or custom sound; minimap icon (size, colour, shape); beam (colour and `Temp`); drop-sound toggle. Hide styles get the full field set (Phase 17).
- **Adding:** a new style needs a unique, non-empty name. Renaming is not supported in v0.5.
- **Deleting:** Built-ins can't be deleted: S, A, B, C, D, E, F, `unchanged` and `hidden`. Other styles can be deleted even while rules use them. On the next Apply, affected rules keep their current actions and their `#ff:style` line is dropped.

**Tier editor (CONF-03)**
- One row per tier, S to F, each with a min-value control. The unit is shown read-only.
- Text buffers are kept separate from parsed values. `ValidatedTiers` is the single validator, and its errors (e.g. cutoffs out of order) show inline. Invalid cutoffs can't be applied.
- The tab edits the live `tiers.json` only. The committed `prices-fallback.json` / `tiers-fallback.json` pair is never touched.

**Apply and Revert (CONF-04)**
- Config Apply validates both files, writes both to temp files, then renames both: all or nothing.
- It writes **only** `styles.json` and `tiers.json`. The filter is then shown as out of date until the RULES tab's Apply regenerates the economy section and style-owned actions.
- Config Revert discards unsaved config edits. Config edits are held in memory, as with the rules tab.
- An mtime check against outside edits to `styles.json`/`tiers.json` is the same safety idea as Phase 18's guard.

### Claude's Discretion
- Widget layout in the narrow stripe, preview chip design, and the exact wording of the out-of-date indicator.

### Deferred Ideas (OUT OF SCOPE)
- Load message configuration (999.3, online-only).
- Style rename.
- Per-user styles location.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CONF-01 | The overlay has a configuration tab, separate from the rule editor. | `OverlayApp` has no tab concept today (single `CentralPanel`); add `Tab` enum + `config_view.rs`; footer panel ids/scroll salts must be distinct (Pitfall 6). |
| CONF-02 | Add, edit, delete named styles; built-ins cannot be deleted. | `StyleSet` has NO public mutators and NO save (only `cfg(test)` `insert`/`remove`); need public API + renderer (Standard Stack, Pattern 2). Alpha mismatch (Open Question 1). Tag-drop behaviour change in `WorkingFilter::text_with_styles` (Pattern 5). |
| CONF-03 | Edit tier cutoffs in `tiers.json`; invalid shown inline and cannot be applied. | `ValidatedTiers::try_from` validates per-unit group IN FILE ORDER and does not reject NaN; editor must parse itself and keep file order (Pitfall 3, Pattern 3). |
| CONF-04 | Own Apply/Revert; Apply writes all files or none. | New `safe_write::write_all_or_none` (guard both, stage both temps, preflight, persist, rollback) (Pattern 1, Pitfall 1). |
| STORE-02 | Select a different filter in the game folder; remembered across launches. | New `settings` module + `ff-settings.json` at app root, `OverlayApp::load_filter` currently hardcodes `dynamic::default_filter_path()` (Pattern 4). |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- Milestone branching workflow; docs live in `.planning/`. Work is on branch `milestone/v0.5-generic-filter-editor-initial-app-configuration`.
- **`CHANGELOG.md`**: keep `[Unreleased]` current as work lands.
- **CLAUDE.md "Repository layout" must be kept accurate**: add an `ff-settings.json` row (git-ignored, per-user, app root) and describe the CONFIG tab in the overlay note; the stale "Styles tab" wording is irrelevant but the existing overlay prose should mention the tab.
- `tests/fixtures/seed-universal.filter` is the byte-for-byte acceptance baseline; `artifact_guard` enforces it. `generate_on_seed()` copies the **committed repo-root** `config.json`, `tiers.json`, `styles.json` (tests/cli_generate.rs:33). Never let any test or manual smoke run rewrite those repo-root files.
- Repo-root `universal.filter`, `prices.json`, `dynamic.filter` are generated/git-ignored; `dynamic_base.json`/`dynamic.json` are gone (obsolete).
- Layer rule (`overlay/mod.rs`, `PITFALLS.md` section 8): pure logic in non-egui modules, egui is a thin view. `overlay/apply.rs` is the model ("no GUI-framework types of any kind").
- Game folder: `%USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2`. Tests must never touch it.
- Wording: user prefers prose discussion over option votes (memory); no emojis.

## Summary

The phase is an integration of five pieces onto code that already exists, plus three genuine gaps that CONTEXT assumes are done but are not. (1) `StyleSet` can be loaded and validated but has **no public mutation API and no save/render function** (the CONTEXT "StyleSet load/save from Phase 17" claim is only half true: `insert`/`remove` are `#[cfg(test)] pub(crate)`; serialisation derives exist but nothing writes `styles.json`). (2) There is **no tab concept** in `OverlayApp::ui` (one `CentralPanel` containing header, status block, FILTER heading, search, list, with the pinned `Panel::bottom("ff-footer")` claimed inside it). (3) There is **no multi-file atomic write**: `safe_write::write_guarded` is single-file (full-bytes changed-on-disk guard, `.ffbak`, temp+persist with Windows retry). The "mtime guard from Phase 18" is in fact a **byte-compare** guard in `safe_write`; mtime is only used by `poll_styles` (2 s poll of `styles.json`). The config guard can use both: bytes at Apply time (authoritative), mtime for the cheap tab-activation check.

Two spec-vs-model conflicts need a decision before planning tasks. **Alpha:** `Style.text/border/background` are `Option<[u8; 3]>` and the emitter hard-codes `255` (`emitter.rs:228-238`), so the "colour picker with alpha" in CONTEXT/UI-SPEC cannot be stored without changing the model, `style_action_lines`, `styled.rs` comparison and potentially fixtures. Recommend the picker run with `Alpha::Opaque` (exists in egui 0.36.1/.2) and surface the deviation (Open Question 1). **Tag dropping:** today an unknown `#ff:style` tag is only *reported* (`StyleReport.unknown`, "unknown style" flag) and kept; CONTEXT requires the tag be **dropped** on the next Apply for deleted styles. This is a small overlay-only change (`rewrite_styled` is called only from `WorkingFilter::text_with_styles`; the CLI does not use it).

Third hazard: deleting the non-built-in styles `Recycle`, `kite` or `exceptional` is allowed by the locked rule but `config.json` references them (`exceptional` x3, `kite` x2, `Recycle` x2 plus `alwaysShow.recycle`), and `compile::load_inputs` calls `check_rules` (compile.rs:205), so `ferrisfilter generate` would hard-fail afterwards. Not a CONTEXT violation, but the delete confirm should warn (Open Question 3).

**Primary recommendation:** Build bottom-up in four layers: (a) library: `StyleSet` mutators + `render_styles`/`render_tiers` + `safe_write::write_all_or_none` + `settings` module + `styled::drop_unknown_style_tags`; (b) pure overlay model `config_state.rs` (drafts, dirty count, validation, snapshot compare, byte/mtime guard) with no egui types; (c) `config_view.rs` egui rendering returning a `ConfigAction` list; (d) `app.rs` wiring (Tab enum, filter switching, out-of-date flag). Test (a) and (b) exhaustively with temp dirs; (c)/(d) get headless `run_ui` smoke tests only.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Tab switching, accordion, colour pickers, preview chip | egui view (`overlay/config_view.rs`, `widgets.rs`) | - | Presentation only; mutations returned as `ConfigAction`, applied after the frame (same deferred-action rule as `RowAction`). |
| Draft buffers, dirty count, validation, snapshot compare, guard bookkeeping | Pure overlay model (`overlay/config_state.rs`) | library | Must be testable without a window (`overlay/apply.rs` precedent). |
| `StyleSet` mutation, `styles.json`/`tiers.json` rendering | Library (`models/config.rs`, new `models/render.rs` or `loader.rs`) | - | Not egui; reused by tests. |
| All-or-none two-file write | Library (`safe_write.rs`) | - | Same module owns the existing guarded single-file write and its Windows retry policy. |
| `ff-settings.json` read/write | Library (`settings.rs`, new) | `paths::app_root` | Path is a parameter; lib never prints. |
| Game-folder listing, filename validation | Library (`settings.rs` or `dynamic.rs`) | overlay (cached result) | Pure `fn list_filters(dir) -> Vec<String>`; overlay caches, refreshes on tab activation. |
| `#ff:style` tag drop on Apply | Library (`styled.rs`) invoked from `WorkingFilter::text_with_styles` | overlay | Styles/filter edit logic already lives in `styled.rs`. |
| Out-of-date indicator + filter switch | `app.rs` + `apply.rs` pure fns | - | Existing indicator machinery lives there. |

## Standard Stack

No new dependencies. Everything needed is already in `Cargo.toml` (`egui`/`eframe` 0.36, `serde`/`serde_json`, `indexmap` with `serde`, `tempfile` (regular dependency), `thiserror`, `miette`). [VERIFIED: Cargo.toml]

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| egui / eframe | 0.36 (registry has 0.36.1 and 0.36.2 sources) | UI | Already the overlay framework. [VERIFIED: Cargo.toml, ~/.cargo/registry] |
| serde_json | 1.0 | Render/parse `styles.json`, `tiers.json`, `ff-settings.json` | Existing. |
| tempfile | 3 | Stage temp files in the destination directory for atomic replace | Already used by `safe_write` (`NamedTempFile::new_in` + `persist`). |
| indexmap | 2 | `StyleSet` is `IndexMap<String, Style>` (file order) | Existing; `shift_remove` preserves order on delete. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rendered one-line-per-entry JSON | `serde_json::to_string_pretty` | Pretty output explodes the committed 12-line `styles.json` into ~300 lines (git noise on a tracked file). Per-entry compact lines keep the diff reviewable. Both round-trip value-equal. |
| `DragValue` for tier mins | text `TextEdit` + `String` buffer | UI-SPEC mandates separate buffers (invalid mid-edit text like `2.` or empty must be representable); `DragValue` holds only a parsed number. Use TextEdit. |
| Windows transactional NTFS (TxF) | rename + rollback | TxF is deprecated by Microsoft and unusable on OneDrive folders; use rename + rollback. [ASSUMED] |

**Installation:** none.

## Package Legitimacy Audit

No external packages are added by this phase. slopcheck not applicable.

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### Current structure (what exists, what to touch)

```
src/
├── models/config.rs   Style{text,border,background:Option<[u8;3]>, font:Option<u8>, minimap, beam, sound{id:Option<u8>,custom:Option<String>,vol:u16}, action, disable_drop_sound}
│                      StyleSet(IndexMap) : get/names/len/from_pairs/validate/check_rules; insert/remove are cfg(test) only  <- ADD pub mutators
│                      FilterColor (11 variants), MinimapShape (12), StyleAction  <- no ALL lists; overlay uses string lists COLOURS/SHAPES in filter/keywords.rs
├── models/tiers.rs    TierEntry{tier,min:f64,unit}, Unit{Chaos,Div,Exalt}, ValidatedTiers::try_from, TierValidationError::{MissingTiers,NotAscending{prev,prev_min,curr,curr_min,unit}}
├── loader.rs          load_styles(&str)->StyleSet(validated) / load_tiers(&str)->ValidatedTiers ; no save
├── compile.rs         InputPaths::relative_to(root) -> {config,styles,prices,tiers,*_fallback}   (single place file names live)
├── paths.rs           app_root()/resolve_from(env,exe_dir,cwd); root marker = styles.json ; FERRISFILTER_HOME override
├── safe_write.rs      write_guarded(path, loaded:Option<&[u8]>, new) (bytes guard, .ffbak, tempfile persist, 5 retries on os errors 5/32/33; read-only detection)
├── dynamic.rs         LoadedFilter::load(path) (absent file == empty filter), apply_edited, default_filter_path() = <game folder>/dynamic.filter
├── styled.rs          rewrite_styled(doc,&StyleSet)->StyleReport{rewritten,unknown,failed}; style_name_ok; rewrite_block
└── overlay/
    ├── app.rs         OverlayApp (1670 lines): root, filter:Result<LoadedFilter>, working:Option<WorkingFilter>, list:ListUi, apply_state, apply_message, styles_mtime (2 s poll), run_apply/revert/reload/load_filter()
    ├── apply.rs       PURE: ApplyState, indicator(), footer_actions(), messages, *_message fns
    ├── rules.rs       WorkingFilter (+StyleState Known/Unknown/Unchecked, set_styles, text_with_styles, style_rewrite_pending, unknown_styles, is_dirty, revert, on_applied)
    ├── rule_list.rs   ListUi, show_list (virtualized), outcome area helpers live in app.rs (outcome_lines/outcome_area, OUTCOME_MAX_HEIGHT=96)
    ├── body.rs        rule body controls; font_control/sound_control/minimap_control/effect_control/colour_combo/dot/filter_colour/slider_pair are PRIVATE and tied to BodyEntry/RowAction
    └── widgets.rs     header_plate, zebra_row, circular_toggle, toggle_chip, badge, themed_slider(i64), number_field(i64 only), colour_swatch([u8;4], Alpha::OnlyBlend), rgba_caption, chip_list, arrow_button, remove_x_button
```

### Recommended additions

```
src/
├── settings.rs            NEW  ff-settings.json load/save + filter-name validation + game-folder listing (pure; paths are params)
├── safe_write.rs          ADD  write_all_or_none(&[Target]) 
├── models/config.rs       ADD  StyleSet::{get_mut, insert_new, remove_custom, iter, is_builtin}, ALL lists on enums
├── models/render.rs (or loader.rs) NEW render_styles(&StyleSet)->String, render_tiers(&[TierEntry])->String
├── styled.rs              ADD  drop_unknown_style_tags(doc,&StyleSet)->Vec<(usize,String)>
└── overlay/
    ├── config_state.rs    NEW  pure ConfigState (no egui): working styles/tiers, snapshots, tier buffers, dirty count, validation, guard, apply()/revert()/reload()
    ├── config_view.rs     NEW  egui rendering -> Vec<ConfigAction>; style_preview_chip lives in widgets.rs
    ├── app.rs             EDIT Tab enum, tab strip, config footer panel, filter switch, out-of-date flag, immediate style refresh after Config Apply
    ├── widgets.rs         ADD  style_preview_chip, colour_swatch with Alpha param (or opaque variant), decimal text field helper
    └── body.rs            EDIT promote colour_combo/dot/filter_colour (and a minimap/beam control that takes plain values) to pub(crate) so the style editor really shares them
```

### System data-flow

```
 launch ─> paths::app_root() ─> settings::load(root/ff-settings.json) ─> filter_name (default dynamic.filter)
                                   │                                         │
                                   │                               game_folder.join(filter_name)
                                   │                                         v
                                   │                              LoadedFilter::load ─> WorkingFilter (RULES tab)
                                   v
        ConfigState::load(root) reads styles.json + tiers.json  (keeps bytes+mtime as guard baseline, snapshot, tier buffers)

 CONFIG tab frame:  config_view::show(ui,&mut ConfigState,&rules_info) ─> Vec<ConfigAction>  (no mutation during render)
        after frame: apply actions ─> ConfigState edits ──> dirty/validation recomputed

 CONFIG Apply:  validate(styles.validate(), ValidatedTiers::try_from(parsed), all buffers parse)
                  ─> guard: bytes of styles.json/tiers.json == baseline ? else OutsideEdit prompt
                  ─> render both ─> safe_write::write_all_or_none  (stage temps ─> preflight ─> persist A ─> persist B ─> rollback A on B failure)
                  ─> success: new baseline; app: styles_mtime update + WorkingFilter::set_styles(new) ; config_changed_pending = true
                  
 RULES Apply (existing): set_styles(load) ─> text_with_styles(rewrite_styled + NEW drop_unknown_style_tags) ─> economy_body(reads tiers.json) ─> apply_edited
                  ─> success (Written OR Unchanged): config_changed_pending = false
```

### Pattern 1: all-or-none two-file write (library, `safe_write.rs`)

**What:** Mirror `write_guarded_with`'s structure (injectable `persist` and `sleep` for tests) but over N targets.
**Steps (order matters):**
1. For each target: destination dir must exist; re-read current bytes; compare with `loaded` (`ChangedOnDisk` naming the file) - nothing touched yet.
2. Targets whose new bytes equal loaded bytes are skipped (`Unchanged`), so an Apply that edited only tiers does not rewrite `styles.json` (and does not bump its mtime / re-trigger `poll_styles`).
3. Stage a `NamedTempFile::new_in(dest_dir)` per remaining target, `write_all` + `sync_all`. Any error drops all temps (auto-deleted). **Nothing renamed yet.**
4. Preflight each target: not read-only (`is_read_only`) and openable for write without truncation (`OpenOptions::new().write(true).open(path)`, then drop). This converts most "second rename will fail" cases into a pre-rename refusal.
5. Persist in a fixed order using the existing retry loop (backoff 50..400 ms on os errors 5/32/33).
6. If persist k>0 fails after persist 0..k-1 succeeded: **roll back** by writing the in-memory `loaded` bytes back to each already-replaced target via the same temp+persist path, then return an error that says whether rollback succeeded ("restored" vs "styles.json could not be restored; its previous contents are in X").
**Why:** Windows `MoveFileExW(REPLACE_EXISTING)` (what `persist` uses) is atomic per file but there is no cross-file atomicity. Rollback from the in-memory bytes is the only practical closure. [CITED: tempfile `NamedTempFile::persist` semantics; Windows rename behaviour ASSUMED from `safe_write.rs` comments on sharing violations]
**Test seam:** a `persist` closure that fails on the 2nd call asserts styles.json is restored byte-for-byte (`persist_second_fails_restores_first`).
No `.ffbak` for these two files: both are git-tracked; keep it out unless the planner wants parity.

### Pattern 2: `StyleSet` public API and renderer

```rust
// models/config.rs  (names are suggestions)
impl StyleSet {
    pub fn is_builtin(name: &str) -> bool { Self::REQUIRED.contains(&name) }
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Style> { self.0.get_mut(name) }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Style)> { self.0.iter().map(|(k, v)| (k.as_str(), v)) }
    /// Err(reason) for empty/untrimmed/control-char names (styled::style_name_ok), duplicates,
    /// and case-only shadows of REQUIRED (validate() rejects those; reject at add time too).
    pub fn insert_new(&mut self, name: &str, style: Style) -> Result<(), NameError> { .. }
    pub fn remove_custom(&mut self, name: &str) -> Option<Style> { if Self::is_builtin(name) { None } else { self.0.shift_remove(name) } }
}
```

Render one style per line to keep the committed file's shape; field order from `Style` already matches the file (`text, border, background, font, minimap, beam, sound, action, disableDropSound`):

```rust
pub fn render_styles(set: &StyleSet) -> String {
    let mut out = String::from("{\n");
    let n = set.len();
    for (i, (name, st)) in set.iter().enumerate() {
        out.push_str(&format!("  {}: {}{}\n",
            serde_json::to_string(name).unwrap(), serde_json::to_string(st).unwrap(),
            if i + 1 < n { "," } else { "" }));
    }
    out.push_str("}\n");
    out
}
```
`Sound` serialises `"id":null` for custom sounds (the committed file omits it). Either add `#[serde(skip_serializing_if = "Option::is_none")]` to `Sound.id`/`custom` (safe: `Config.styles` is `#[serde(skip)]`, so no fixture or `config.json` output changes) or accept the verbose `null`. Deserialisation of a missing `Option` field already works. Add a test that re-parses `render_styles(committed)` to an equal `StyleSet` and that `render_tiers` output re-validates.

### Pattern 3: tier rows (pure model)

- Keep `Vec<TierEntry>` in **loaded file order**; the display order S..F is presentation. Edit `min` by tier name (never by index).
- Buffers: `BTreeMap<String /*tier*/, String>` (or `[String; 7]` by tier index) seeded from `format!("{}", min)` (Rust prints `2` for `2.0` and `0.1` for `0.1`; both parse back exactly).
- Per edit: parse buffer: `s.trim().parse::<f64>()`, require `is_finite() && >= 0.0`; on failure show `Enter a number`, **leave last good value in the model**, and keep the row "invalid" so Apply is disabled (a parse failure is an error even though the model still holds a valid number).
- On every successful parse run `ValidatedTiers::try_from(entries.clone())`; map `NotAscending{curr,prev,prev_min,..}` to inline error under the `curr` row, edge stroke on `prev` and `curr`. Return type holds `HashMap<tier, String>` errors; `MissingTiers` shows in the outcome area.
- Buffers are resynced from the model only on Revert/Reload/Apply success, and never while that row's TextEdit has focus.

### Pattern 4: settings + filter selection

```rust
// settings.rs
#[derive(Serialize, Deserialize, Default)]
pub struct Settings { #[serde(default, skip_serializing_if = "Option::is_none")] pub filter: Option<String>,
                      #[serde(flatten)] pub extra: serde_json::Map<String, serde_json::Value> } // preserve unknown keys
pub const SETTINGS_FILE: &str = "ff-settings.json";
pub const DEFAULT_FILTER: &str = crate::dynamic::FILTER_FILE; // "dynamic.filter"
pub fn load(root: &Path) -> (Settings, Option<String /*warning*/>)   // missing => default; corrupt => default + warning, file untouched
pub fn save(root: &Path, s: &Settings) -> io::Result<()>             // NamedTempFile::new_in(root) + persist
pub fn valid_filter_name(name: &str) -> Result<(), NameProblem>      // bare name, ends .filter (case-insens), no separators, no "..", no <>:"/\|?*, no trailing dot/space, not a Windows device name (CON, NUL, COM1..), len bound
pub fn list_filters(dir: &Path) -> io::Result<Vec<String>>          // *.filter files only (ffbak ends .ffbak so is excluded), case-insensitive sort
```
- A stored name that fails `valid_filter_name` (hand-edited, `..\x`) is treated as corrupt -> default + warning. The name is joined onto the game folder only after validation (path-traversal guard).
- A remembered name whose file is gone loads as an **absent/empty filter** (STORE-03 semantics, no special case): the status block already says "The filter does not exist yet", and nothing is written until Apply. Same path handles `Create filter` (file not created at create-time; name persisted immediately per UI-SPEC decision 3).
- `OverlayApp::load_filter()` is an associated fn that hardcodes `dynamic::default_filter_path()`; change to `load_filter(game_dir, name)`. Factor `reload()` (which resets `filter, working, list, query, needs_reload, apply_message, applied_this_session, apply_state` then `refresh_inputs`) into `switch_to(name)` and have `reload()` call it with the current name. `reload()` also re-resolves the root; settings must be re-read there too.
- Cache the directory listing (OneDrive reads are slow); refresh on CONFIG-tab activation, after Create, and after RELOAD. Never `read_dir` per frame. Compare "already exists" case-insensitively (NTFS).
- Switch guard condition = `working.is_dirty()` only (rule edits and staged economy op). `style_rewrite_pending` is derived, not an edit, and must not trigger the guard.
- ComboBox: selected text comes from the model's confirmed name; a pick becomes a `ConfigAction::SelectFilter(name)` handled after the frame; if guarded, store `pending_switch: Option<Target>`; the model name does not change until confirmed (this gives the "selection reverts" behaviour for free).

### Pattern 5: dropping `#ff:style` for deleted styles

`rewrite_styled` skips disabled and economy blocks and only *reports* unknown names. Add (do not change existing semantics/tests):
```rust
pub fn drop_unknown_style_tags(doc: &mut Document, styles: &StyleSet) -> Vec<(usize, String)>
```
iterating non-economy blocks, `doc.meta_get(id,"style")` not in `styles` -> `doc.remove_meta(id,"style")`, returning `(rule_index, name)`. Call it in `WorkingFilter::text_with_styles` on the clone (after `rewrite_styled`), add `dropped` to the report (or a second return), and have `app.rs::apply_working` append a sentence ("Removed the style link from N rules whose style no longer exists."). Consequences to handle in the same plan:
- `style_rewrite_pending()` compares `text_with_styles()` to `text()`, so it turns true automatically - the RULES indicator goes stale correctly.
- The row flag/explanation for `StyleState::Unknown` (rule_list.rs flags_for, body.rs `STYLE_UNKNOWN_EXPLAIN`) says "nothing is rewritten on Apply"; update copy to say the link is removed on Apply. The footer `unknown_styles_message` line stays useful before Apply.
- Whether to include **disabled** rules: recommended yes (a dangling tag would reappear as Unknown on re-enable). `remove_meta` on a disabled block was not exercised in this research; verify in a test, and fall back to enabled-only if the edit layer refuses.
- Existing test `styled::tests::unknown_style_is_reported_and_untouched` and `rules::tests::unknown_styles_listed_enabled_only` keep passing if the drop lives in the new function and not in `rewrite_styled`.
- A typo'd hand-written tag is also dropped on Apply (not only deleted ones). This is the literal CONTEXT behaviour; the Apply message makes it visible.

### Pattern 6: tab strip and panels in `OverlayApp::ui`

- Add `tab: Tab` (`Rules|Config`) and `config: ConfigState` to `OverlayApp`. Insert the strip after `header_plate` and before the status block. Branch: `Tab::Rules` runs the existing body unchanged; `Tab::Config` runs `config_view`.
- The RULES footer is `Panel::bottom(Id::new("ff-footer"))` created inside the central closure before content (so APPLY always stays on screen - there is a regression test, `the_apply_button_stays_on_screen_...`). The CONFIG footer must be a **separate** `Panel::bottom(Id::new("ff-config-footer"))` claimed before its scroll area, same inner-margin frame.
- `outcome_area` hardcodes `ScrollArea::id_salt("outcome")`: add an `id_salt` parameter (or call it only from one tab per frame and accept shared offset - better to parametrise). `outcome_lines` is RULES-specific; build a small config equivalent producing `Vec<OutcomeLine>`.
- Dirty marker: label `RULES *` / `CONFIG *` when the *other* tab has unapplied edits (`working.is_dirty()` or `config.dirty_count() > 0`).
- Keep `request_repaint_after(50ms)` as is; do the mtime-on-activation check when the tab changes, not per frame.

### Anti-Patterns to Avoid
- **Mutating `ConfigState` inside the render closure.** Collect `ConfigAction`s, apply after the loop (existing `RowAction` convention).
- **Index-keyed egui ids for style rows.** Style rows come and go (add/delete); key by style name via `ui.push_id(("style", name))`, and keep `expanded_style: Option<String>`.
- **Per-frame `fs::metadata`/`read_dir`** on OneDrive paths.
- **Writing tests that call `app_root()` against the repo** or point `USERPROFILE` at the real profile.
- **Re-running the whole style validation to decide button state each frame** without caching - it is cheap (12 entries) but `StyleSet::validate()` returns only the first error; fine for an error line.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Tier ordering/completeness validation | Own comparison loop | `ValidatedTiers::try_from(Vec<TierEntry>)` | Single validator per CONTEXT; but add own parse (finite, >= 0) because NaN passes `<=` comparisons. |
| Style value validation | Own range checks | `StyleSet::validate()` (required styles, case-shadow, custom sound quote/control chars, font 1..=45, minimap size 0..=2, sound id 1..=16, vol 0..=300, exactly one of id/custom) | Same checks the generator uses; surfaces its message inline. |
| Atomic single-file replace + Windows retry | New persist loop | The `persist`/backoff/`is_read_only` logic in `safe_write.rs` (extract helpers) | Already handles sharing violations and OneDrive. |
| App-root discovery | `current_dir()` | `paths::app_root()` / `resolve_from` | STORE-05; `FERRISFILTER_HOME` makes tests hermetic. |
| File names | String literals | `compile::InputPaths::relative_to(root)` (`styles`, `tiers`) and `dynamic::FILTER_FILE` | "The only place the conventional file names appear in the library." `ff-settings.json` should get a `settings::SETTINGS_FILE` const. |
| Style name legality for tags | Own regex | `styled::style_name_ok` (+ trim) | The tag reader trims; a padded name would read back as a different style. |
| Colour picker | Custom HSV widget | `egui::color_picker::color_edit_button_hsva` / `color_edit_button_srgba` | Already wrapped by `widgets::colour_swatch`. |
| Sliders / ints | New slider | `widgets::themed_slider` + `number_field` (i64) | Same vocabulary as Phase 20. |
| Dropdown | custom popup | `egui::ComboBox::from_id_salt(..).selected_text(..).show_ui(..)` | Used in body.rs (`colour_combo`, minimap). |

**Key insight:** this phase is mostly *plumbing between existing validated pieces*; the risk is in the seams (two-file commit, id/state hygiene, tag drop, alpha), not in new algorithms.

## Runtime State Inventory

Not a rename/refactor phase, but it adds persisted state and mutates committed files:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | New `ff-settings.json` (app root). `styles.json`/`tiers.json` are **committed** repo-root files that Config Apply rewrites. | Add `/ff-settings.json` to `.gitignore` (current `.gitignore` has no entry; verified `git check-ignore` returned nothing for it). First Apply re-renders `styles.json` formatting - accept, test value-equality. |
| Live service config | None. | None. |
| OS-registered state | None. | None. |
| Secrets/env vars | None new. | `FERRISFILTER_HOME` already honoured. |
| Build artifacts | None. | None. |

During development `ff.exe` under `target\` walks up to the repo root as app root (`paths::resolve_from`), so manual testing of Config Apply **rewrites the committed styles.json/tiers.json that `artifact_guard` consumes**. Smoke-test with `FERRISFILTER_HOME` pointing at a temp copy.

## Common Pitfalls

### Pitfall 1: Two renames are not one transaction
**What goes wrong:** styles.json replaced, then tiers.json fails (editor/OneDrive holds it) -> the two files disagree, and the economy (tiers) and rule rewrite (styles) drift.
**How to avoid:** stage both temps first, preflight both targets, persist, roll back the first from in-memory loaded bytes on failure (Pattern 1). Report precisely which state remains.
**Warning signs:** a test with an injected persist failure on the second call leaves a changed first file.

### Pitfall 2: The "mtime guard" in the brief is really a byte guard
Phase 18's `write_guarded` compares full bytes against the load baseline. For config files use the same: remember loaded bytes (and mtime for the cheap check on tab activation). mtime alone can miss same-timestamp edits and false-positives after your own write; bytes at Apply time is the authoritative check. After your own successful write, reset both baselines to what you wrote.

### Pitfall 3: `ValidatedTiers` quirks
- Compares `windows(2)` **in the order of the Vec within each unit**; a file in S..F order would be "descending" and fail. Never reorder entries to display order before validating/saving; preserve loaded order.
- `NaN`/`inf` pass `<=` comparisons - editor parse must reject them. `f64` parse also accepts `1e3`, `inf`, `nan`, `+5`; decide to accept `1e3` or not (reject `inf`/`nan` through `is_finite`).
- Strictly ascending only **within a unit group**: tiers.json is all `chaos` so every pair is checked; do not invent cross-unit rules.
- Saved numbers: `serde_json` prints `8.0` as `8.0`; fine.

### Pitfall 4: Alpha cannot be stored (Open Question 1)
`colour_swatch` uses `Alpha::OnlyBlend` and returns `[u8;4]`. Feeding it a style colour and writing back RGB silently drops alpha edits and the preview would show a translucency the filter never emits (emitter always 255). Use an opaque variant and show `R G B` only (or `A 255`).

### Pitfall 5: Deleting non-required styles referenced by `config.json`
`kite`, `Recycle`, `exceptional` are deletable per CONTEXT yet needed by `config.json`; `compile::load_inputs` -> `check_rules` makes `ferrisfilter generate` fail afterwards. `load_classes_at` (`load_config`) does not call `check_rules`, so the overlay itself keeps working, hiding the problem. Warn in the delete confirm using the already-loaded config if available, or document.

### Pitfall 6: egui id and state clashes
- Two `Panel::bottom` with the same id, and `ScrollArea::id_salt("outcome")` shared across tabs share persisted state. Use distinct ids/salts.
- `widgets::number_field` and the body `draft_field` keep drafts in egui temp memory keyed by a salt; tier text buffers belong in `ConfigState` (spec) - salt tier `TextEdit`s by tier letter. Style-editor sliders/number fields must salt with the style name **and** field (`("style", name, "font")`), or two expanded states leak between styles when the expanded row changes. `ListUi` solves this with an `epoch` bumped on expanded-row change (`forget_card_state`); mirror it with a `config.epoch`.
- `egui::Popup::close_all(ctx)` when the expanded style changes (the colour picker popup otherwise stays attached to the old id) - same call `show_list` makes.
- `ComboBox::from_id_salt` needs `Hash + Debug` salts (see `colour_combo`); `widgets::salted_id` hashes arbitrary salts.

### Pitfall 7: Text buffers vs parsed values
Never rewrite a buffer while its TextEdit is focused (spec) and never replace the model with the buffer's parse failure. Revert/Reload/Apply-success must reset buffers or the stale text reappears. Style-name draft (`Add style`) and filter-name draft live in `ConfigState`, cleared on Discard/Create/tab-away-disarm.

### Pitfall 8: "Filter out of date" derived from mtimes alone
`safe_write` returns `Unchanged` without touching the file, so after a RULES Apply that changes nothing the filter's mtime is still older than `styles.json`. UI-SPEC says the line clears on RULES Apply success -> use a session bool (`config_changed_pending`) set on Config Apply and cleared on RULES Apply success (Written **and** Unchanged); use the mtime comparison only to initialise it at load/filter-switch. Compare against the *active* filter's mtime (absent file -> not stale).

### Pitfall 9: Immediate vs polled style refresh
`poll_styles` reloads `styles.json` into `WorkingFilter` within 2 s of an mtime change and keeps its own `styles_mtime`. After Config Apply call the same refresh synchronously (`styles_mtime = styles_mtime_at(root)` + `working.set_styles(load_styles_at(root))`), otherwise the RULES indicator lags 2 s. Config Apply must also set `apply_state = Unapplied` when it was `Applied` (the existing `style_pending` check will do it on the next frame once styles reload).

### Pitfall 10: Sharing the Phase 20 widgets
UI-SPEC decision 14 says no second implementation of the action controls, but `font_control`, `sound_control`, `minimap_control`, `effect_control` are private, take `&Card`/`BodyEntry`, and emit `RowAction`s on filter-line text. The *typed* `Style` cannot reuse them directly. Realistic shared layer: `widgets::{themed_slider, number_field, toggle_chip, circular_toggle, colour_swatch, badge}` plus promoting `colour_combo`, `dot`, `filter_colour` (and SIZE/shape lists) to `pub(crate)`. Plan an explicit extraction task, and keep style-editor-specific glue in `config_view.rs`.

### Pitfall 11: `number_field` is `i64` and unclamped
It commits any parseable integer (sliders never clamp by design). Style fields are `u8`/`u16` with narrow ranges: convert with clamping/validation at the model boundary (`font` 1..=45 -> `u8`, sound id 1..=16, vol 0..=300 -> `u16`), and let `StyleSet::validate()` produce the inline error for out-of-range typed values rather than panicking on `as u8`.

### Pitfall 12: New-style legality
Beyond non-empty/unique: trim input, `style_name_ok`, reject case-only shadows of the nine required names (`validate()` returns `ShadowsRequired`), and consider JSON-key/tag characters. `#ff:style <name>` is read by the tokenizer-based meta reader - names containing spaces round-trip? Not verified here; add a test through `set_meta`/`meta_get` with a spaced name, or restrict to a conservative charset.

### Pitfall 13: Test isolation
`OverlayApp::new` needs a real `TrayHandles`, so the app is not unit-testable; that is why the pure `ConfigState` is essential. Integration tests that spawn `ferrisfilter` set `FERRISFILTER_HOME` and `USERPROFILE` to a temp dir (tests/dynamic_filter.rs:52). Unit tests use `tempfile::tempdir()` and pass paths explicitly. `ConfigState::load(root: &Path)`, `settings::*(root)`, `list_filters(dir)` all take paths.

## Code Examples

### Verified egui 0.36 APIs already used in this repo
```rust
// Colour picker without alpha (egui::color_picker::Alpha::{Opaque, OnlyBlend, BlendOrAdditive} verified in egui-0.36.1/2 source)
let mut hsva = egui::ecolor::Hsva::from_srgba_unmultiplied([r, g, b, 255]);   // or the widgets::hsva_of pattern
let resp = egui::color_picker::color_edit_button_hsva(ui, &mut hsva, egui::color_picker::Alpha::Opaque);
if resp.changed() { let [r, g, b, _] = hsva.to_srgba_unmultiplied(); /* write [r,g,b] */ }
// Source: src/overlay/widgets.rs colour_swatch; egui-0.36.2/src/widgets/color_picker.rs:267

// Dropdown
egui::ComboBox::from_id_salt(("style", name, "msize")).selected_text(label).width(130.0)
    .show_ui(ui, |ui| { if ui.selectable_label(sel, "Large").clicked() { /* record action */ } });
// Source: src/overlay/body.rs minimap_control

// Preview chip painting (new widget): allocate 120x24, then
let (rect, resp) = ui.allocate_exact_size(vec2(120.0, 24.0), Sense::hover());
// checkerboard: loop 6px cells with painter.rect_filled(cell, 0.0, light/dark)
painter.rect(rect, 4.0, bg, Stroke::new(2.0, border), StrokeKind::Inside);   // same call shape as toggle_chip
painter.text(rect.center(), Align2::CENTER_CENTER, "Item Name", FontId::proportional(sz.clamp(12.0, 18.0)), text);
resp.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, format!("Preview of style {name}")));
```
Note: use `Color32::from_rgb` for opaque style colours; do not use premultiplied constructors with translucent values.

### Deferred action enum (sketch)
```rust
enum ConfigAction {
    SelectFilter(String), ConfirmSwitch, CancelSwitch, CreateFilter(String), DiscardNewFilter,
    ToggleStyle(String), AddStyle(String), DiscardNewStyle, SetStyle { name: String, edit: StyleEdit },
    DeleteStyle(String), TierBuffer { tier: String, text: String },
    Apply, Revert, ConfirmRevert, Reload, Overwrite, KeepEditing,
}
```

### Test pattern (pure model)
```rust
let dir = tempfile::tempdir().unwrap();
fs::copy(repo("styles.json"), dir.path().join("styles.json")).unwrap();
fs::copy(repo("tiers.json"),  dir.path().join("tiers.json")).unwrap();
let mut st = ConfigState::load(dir.path()).unwrap();
st.set_tier_text("S", "5");              // S(30) -> 5 : below A(20) -> NotAscending
assert!(st.tier_errors().contains_key("S")); assert!(!st.can_apply());
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `dynamic.json` store, autosave | Filter file is the store; in-memory edits + single Apply writer | Phases 18-19 | CONFIG tab mirrors: in-memory edits, one Apply writer, Revert. |
| Inline `styles` in config | `styles.json` committed, `StyleSet` | Phase 17 | `Config.styles` is `#[serde(skip)]`: reshaping `StyleSet`/`Sound` serialisation cannot alter `config.json` output. |
| Hardcoded `dynamic.filter` | selectable via `ff-settings.json` | this phase | `load_filter` signature change. |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Alpha cannot be supported without changing `Style` (RGB triples) and the emitter; recommend opaque picker | Summary, Pitfall 4 | If the user insists on alpha, a model/emitter/fixture change task is needed (large). |
| A2 | Windows has no practical cross-file atomic rename; rollback from in-memory bytes is acceptable | Pattern 1 | If a stricter guarantee is required, scope grows (not recommended). |
| A3 | `remove_meta` works on disabled blocks | Pattern 5 | Fall back to enabled-only drop. |
| A4 | Style names with spaces survive the `#ff:style` tag round trip | Pitfall 12 | Restrict allowed characters. |
| A5 | Dropping `#ff:style` on *all* unknown tags (including hand typos) matches the user's intent | Pattern 5 | If user wants only deleted-in-session styles dropped, needs session memory of deletions (lost on restart). |
| A6 | A remembered but missing filter should load as empty (STORE-03) rather than fall back to `dynamic.filter` | Pattern 4 | Could surprise the user by re-creating a deleted file on Apply. |

## Open Questions (RESOLVED)

1. **Alpha in the style colour pickers.** RESOLVED (orchestrator): opaque picker, Style stays RGB.
   - Known: CONTEXT/UI-SPEC say picker "with alpha"; `Style` stores `[u8;3]`, emitter writes `255`; `styled::normalize` compares semantic lines; fixtures lock the emitter.
   - Unclear: whether the user wants alpha stored.
   - Recommendation: ship `Alpha::Opaque`, captions `R G B`, note the deviation in the plan and CHANGELOG; defer alpha to a model change phase.
2. **Does the config tab Apply keep `.ffbak`?** RESOLVED: no. Recommendation: no (git-tracked files).
3. **Deleting `kite`/`Recycle`/`exceptional`** breaks `generate`. Recommendation: allow (locked) but show a warning line in the delete confirm when `config.json` references the style (`ClassCatalog` load already reads config.json; extend to expose referenced style names) - or at minimum document in CLAUDE.md. RESOLVED: allow + config.json warning in the delete confirm.
4. **Spec says Revert button disabled when no edits and shows inline confirm only when there are edits** - consistent; no question. **Unit `exalt`/`div` display** read-only from the entry - no question.
5. **Where the dev app root points** RESOLVED: UAT uses a FERRISFILTER_HOME temp copy. (repo root) - recommend manual UAT with `FERRISFILTER_HOME` temp copy and a note in the UAT doc.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | build, tests | yes | cargo 1.100.0-nightly (2026-09-02) | - |
| egui/eframe 0.36.x sources | API verification | yes (0.36.1, 0.36.2 in registry) | - | - |
| Path of Exile 2 game folder | manual UAT only | not required for tests | - | temp dir via `USERPROFILE` |
| python | - | no (not needed) | - | - |

Baseline measured today: `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test --lib` -> **648 passed, 0 failed, about 12 s**. [VERIFIED: ran it]

## Validation Architecture

(`workflow.nyquist_validation` is `false` in `.planning/config.json`; section included because the orchestrator asked for it.)

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` (unit in `src/**` `#[cfg(test)]`; integration in `tests/*.rs`), `insta` snapshots, `assert_cmd`, `tempfile` |
| Config file | `Cargo.toml` (no separate runner config) |
| Quick run command | `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test --lib` |
| Targeted | `... cargo test --lib config_state` / `settings` / `safe_write` / `styled` |
| Full suite command | `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test` |
| Fixture guard | `... cargo test --test cli_generate artifact_guard` |
| Lint/format | `... cargo clippy --all-targets` ; `cargo fmt --check` |

Always set `CARGO_TARGET_DIR` away from `target\`: a running overlay locks `target\debug\ff.exe` and full `cargo test` then fails with "Access is denied" (STATE.md decision).

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CONF-02 | `StyleSet` insert_new rejects empty/dup/case-shadow; remove_custom refuses the 9 built-ins; order preserved | unit | `cargo test --lib models::config` | Wave 0 (new tests) |
| CONF-02 | `render_styles(committed)` re-parses to equal `StyleSet` and validates; custom sound omits/keeps id null consistently | unit | `cargo test --lib render` | Wave 0 |
| CONF-02 | Style edits (colour, font, sound id/custom, minimap, beam, action, drop sound) update the model and dirty count; value-compare clears dirty on manual revert | unit | `cargo test --lib config_state` | Wave 0 |
| CONF-02 | `drop_unknown_style_tags` removes tags for missing styles, keeps actions, idempotent; `style_rewrite_pending` turns true | unit | `cargo test --lib styled` / `rules` | Wave 0 |
| CONF-03 | tier buffer parse (`2.5`, empty, `abc`, `nan`, `-1`), NotAscending mapped to `curr` row, last-good value retained, `can_apply` false | unit | `cargo test --lib config_state::tiers` | Wave 0 |
| CONF-03 | `render_tiers` output re-validates; file order preserved | unit | same | Wave 0 |
| CONF-04 | `write_all_or_none`: both written; guard mismatch on either writes nothing; stage failure writes nothing; second persist failure restores first byte-for-byte; unchanged target skipped | unit | `cargo test --lib safe_write` | Wave 0 |
| CONF-04 | Outside-edit detection (bytes) -> `OutsideEdit`; Overwrite path; Reload discards edits | unit | `cargo test --lib config_state::guard` | Wave 0 |
| STORE-02 | settings round trip, missing -> default, corrupt -> default+warning, traversal names rejected, unknown keys preserved | unit | `cargo test --lib settings` | Wave 0 |
| STORE-02 | `list_filters` returns only `*.filter`, sorted, excludes `.ffbak` | unit | `cargo test --lib settings::list` | Wave 0 |
| STORE-02 | new-filter name validation incl. case-insensitive exists, reserved names | unit | same | Wave 0 |
| CONF-01 | headless `ctx.run_ui` smoke: CONFIG view renders with committed styles/tiers, no panic, footer buttons on screen; id uniqueness (egui debug id-clash warnings absent) | unit (egui) | `cargo test --lib config_view` | Wave 0 |
| Regression | Generator byte-identical | integration | `cargo test --test cli_generate` | exists |
| Regression | Existing overlay behaviour | unit | `cargo test --lib` (648 baseline) | exists |

### Sampling Rate
- **Per task commit:** targeted `cargo test --lib <module>` (seconds).
- **Per wave merge:** `cargo test --lib` then `cargo test --test cli_generate`.
- **Phase gate:** full `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`, and a git check that `git diff --stat -- styles.json tiers.json tests/fixtures` is empty (nothing wrote to committed files).

### Wave 0 Gaps
- [ ] `src/settings.rs` tests, `src/overlay/config_state.rs` tests, `safe_write` multi-file tests, `styled` drop tests, `render_*` round-trip tests.
- [ ] Headless egui smoke test for `config_view` (pattern: `app.rs` tests with `egui::Context::default()`, `set_fonts(FontDefinitions::empty())`, 460x600 `RawInput`, `ctx.run_ui(...)`).
- [ ] Manual UAT doc (`21-MANUAL-VERIFICATION.md`): with `FERRISFILTER_HOME` temp copy: add/edit/delete style, tier error, Apply, outside edit, filter switch guard, relaunch remembers filter, in-game Refresh.

## Security Domain

`security_enforcement` is not set in config (treated as enabled). Local desktop tool, no network in this phase.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2/V3 Auth/Session | no | - |
| V4 Access Control | partial | Filter name from `ff-settings.json` and new-filter input is joined onto the game folder: validate as a bare `*.filter` file name. |
| V5 Input Validation | yes | `StyleSet::validate`, `ValidatedTiers`, `valid_filter_name`, `style_name_ok`; custom sound reject `"`/control chars (already in `validate`) because it is emitted inside quotes in the filter. |
| V6 Cryptography | no | - |
| V12 Files | yes | Writes via same-directory temp + persist; never create the game folder; no symlink following concerns beyond existing code. |

### Known Threat Patterns
| Pattern | STRIPE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via `ff-settings.json` (`..\..\x.filter`) | Tampering | `valid_filter_name` before join; treat invalid as corrupt -> default |
| Filter-injection via style custom sound / name with newline or quote | Tampering | `validate()` and `style_name_ok` before write |
| Partial write leaving mismatched config | Tampering/DoS | Pattern 1 stage+preflight+rollback |
| Clobbering an outside edit | Tampering | byte guard + explicit Overwrite confirm |

## Sources

### Primary (HIGH confidence)
- Repo source read in this session: `src/models/config.rs`, `src/models/tiers.rs`, `src/loader.rs`, `src/compile.rs`, `src/paths.rs`, `src/safe_write.rs`, `src/dynamic.rs`, `src/styled.rs`, `src/install.rs`, `src/overlay/{mod,app,apply,rules,rule_list,body,widgets,text_entry,virtual_list}.rs`, `tests/cli_generate.rs`, `tests/dynamic_filter.rs`, `Cargo.toml`, `.gitignore`, `.planning/config.json`, `styles.json`, `tiers.json`, `config.json`.
- Installed egui 0.36.1/0.36.2 source: `src/widgets/color_picker.rs` (`Alpha::{Opaque, OnlyBlend, BlendOrAdditive}`).
- `cargo test --lib` run (648 passing) on current branch.

### Secondary (MEDIUM)
- Phase CONTEXT/UI-SPEC for 21; STATE.md decisions.

### Tertiary (LOW)
- Windows rename/TxF statements (A2) from general knowledge.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new packages; all APIs read from source.
- Architecture: HIGH - integration seams read directly; tag-drop and write-all pattern are new designs (MEDIUM until tested).
- Pitfalls: HIGH for code-derived ones (1-3, 5-9, 11), MEDIUM for 10, 12 (unverified round trips).

**Research date:** 2026-10-13
**Valid until:** 2026-11-12 (stable; invalidated by changes to `Style`/`safe_write`/`app.rs` on the milestone branch)
