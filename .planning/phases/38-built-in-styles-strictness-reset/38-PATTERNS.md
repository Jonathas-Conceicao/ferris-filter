# Phase 38: Built-in Styles, Strictness & Reset - Pattern Map

**Mapped:** 2026-10-18
**Files analyzed:** 17 new/modified
**Analogs found:** 17 / 17

Line numbers are from the worktree BEFORE Phase 37 lands. Phase 37 edits `config_view.rs`
(Filter section, `ConfigAction`, `ConfigViewCtx`), `app.rs`, `shell.rs` (37-05), `skin/widgets.rs`,
`tests/cli_*.rs`, `tests/dynamic_filter.rs`, `CLAUDE.md`. Re-read every file before editing; match on
the quoted text, not the numbers.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `styles-default.json` | config (embedded data) | transform | itself (old content) + snapshot `.planning/research/author-styles-2026-10-17.json` | exact |
| `tests/fixtures/seed-styles.json` | test fixture | file-I/O | `tests/fixtures/seed-prices.json` (frozen oracle) | role-match |
| `src/overlay/strictness.rs` (new) | model (pure) | transform | `config_state.rs` `DISPLAY_ORDER` + `badges()` | role-match |
| `src/overlay/config_state.rs` | model | CRUD (pending edit) | its own `edit_style` / `revert_scope` / `tiers_dirty_count` | exact |
| `src/models/config.rs` | model | transform | `committed_for_tests()` (L925-945) | exact |
| `src/skin/editor.rs` (`ff_levels`) | component (skin widget) | request-response | `ff_segmented` (L25-98) | exact |
| `src/overlay/config_view.rs` | view | request-response | `show_styles` (L642), `ConfigAction`, `ConfigUi` | exact |
| `src/overlay/shell.rs` | view + popup layer | request-response | `economy_tab`, `Popup::RemoveEconomy` | exact |
| `src/overlay/app.rs` | controller | event-driven | `Popup::DeleteStyle` / `RemoveEconomy` arms, `handle_config_actions` | exact |
| `src/overlay/screenshot.rs` | utility | request-response | `POPUPS` array | exact |
| `src/overlay/style_lines.rs` | view | request-response | the L610 call site (FIX-01) | exact |
| `src/defaults.rs` | config | - | its own tests | exact |
| Test readers (lib: emitter, compile, loader, models::config, app, config_state, config_view, rule_list, rules, styled, apply_all, body, classes, dynamic) | test | file-I/O | `temp_root()` helpers | exact |
| Test readers (integration: cli_generate, cli_errors, dynamic_filter, filter_roundtrip, sync_prices) | test | file-I/O | `cli_generate.rs` copy loop | exact |
| `CLAUDE.md` | doc | - | existing tables | exact |

## Pattern Assignments

### `styles-default.json` and `tests/fixtures/seed-styles.json` (data swap)

Ordering is the pitfall: (1) `cp styles-default.json tests/fixtures/seed-styles.json` while it still has old bytes,
(2) repoint readers, suite green, (3) replace the root file with snapshot lines S..F, `unchanged`, `hidden`
(drop Recycle/exceptional/Build Items/kite/White bases; no trailing comma on the last kept line; wrap in `{`/`}` + final newline).
The lines are already `loader::render_styles` output, so add a test `render_styles(parse(file)) == file`.
`tiers-default.json` stays unchanged. The existing `embedded_text_equals_the_committed_file` test in `src/defaults.rs:72-78` guards embedded vs file.

### Test readers (copy-with-rename pattern)

**Analog:** `src/overlay/config_state.rs:723-735` and `config_view.rs:1119-1132` `temp_root()`:
```rust
for (src, dst) in [
    ("styles-default.json", "styles.json"),
    ("tiers-default.json", "tiers.json"),
] {
    fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join(src), dir.path().join(dst)).unwrap();
}
```
Change the source of the styles pair to `tests/fixtures/seed-styles.json`. Existing assertions like `style_names().len() == 12` (config_state.rs ~L741) keep working on the fixture.

**Integration analog:** `tests/cli_generate.rs:33-39`:
```rust
for (src, dst) in [
    ("config.json", "config.json"),
    ("tiers-default.json", "tiers.json"),
    ("styles-default.json", "styles.json"),
] { fs::copy(manifest.join(src), dir.join(dst)) ... }
```
and `tests/cli_errors.rs:40-43` `valid_styles()` (`fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/styles-default.json"))`). Point at `/tests/fixtures/seed-styles.json`. For copy loops whose destination is literally `styles-default.json` (cli_generate ~L746, config_state ~L822, app ~L3761) keep the dst name, change the src (use `(src, dst)` pairs).
Exception: tests asserting the fallback to the EMBEDDED default via `InputPaths::in_home` (`default_files_in_the_folder_are_ignored`, `load_falls_back_to_defaults_and_creates_nothing`, app ~L3792-3884) must compare against `crate::defaults::STYLES_DEFAULT.text`.

### `src/models/config.rs`

**Analog:** `committed_for_tests` (L935-945):
```rust
#[cfg(test)]
pub(crate) fn committed_for_tests() -> StyleSet {
    let mut set: StyleSet = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"), "/styles-default.json"))).unwrap_or_else(...);
    set.assign_missing_ids();
    set.validate().unwrap_or_else(...);
    set
}
```
Change the path to `/tests/fixtures/seed-styles.json`. Add a NON-test `StyleSet::shipped_defaults()` (parse `crate::defaults::STYLES_DEFAULT.text`, `assign_missing_ids`, cache in `OnceLock`) as the reset source.
Helpers to reuse: `StyleSet::builtin_id(name)` (L795; takes a name such as "S", returns `builtin:S`), `name_of_id` (L805), `from_pairs` (L543, pub), `is_builtin`, `REQUIRED` (L523), `iter()`, `get_mut()`.

### `src/overlay/strictness.rs` (new, pure, no egui types)

**Analog for shape:** `config_state.rs` L28 `const DISPLAY_ORDER: [&str; 7] = ["S","A","B","C","D","E","F"];` and `badges()` (L204-210):
```rust
hide: matches!(now, Some(s) if s.action == StyleAction::Hide),
```
Implement `Strictness` enum, `hidden()` sets, `strictness_of(&StyleSet)`: for each letter, `set.name_of_id(&StyleSet::builtin_id(letter))` then `get(name).action == Hide`; compare hidden-letter set with each level; missing id or unmatched pattern -> `Custom`. Also hold captions (stacked lines) and tooltip text (UI-SPEC table + shared note "Affects every rule that uses these tier styles, yours included."). Register `pub mod strictness;` in `src/overlay/mod.rs`.

### `src/overlay/config_state.rs` (new methods)

**Core edit pattern** - `edit_style` (L225-251) returns bool; strictness reuses it:
```rust
pub fn edit_style(&mut self, name: &str, edit: StyleEdit) -> bool { ... StyleEdit::Action(a) => style.action = a, ... }
```
`set_strictness(level) -> bool`: loop the six letters, resolve name via `self.styles.name_of_id(&StyleSet::builtin_id(l))`, skip absent, call `edit_style(name, StyleEdit::Action(Hide|Show))`, return any-changed. No `apply_state` code needed (per-frame `combined_dirty` in app.rs).

**Dirty/revert** (already works): `styles_dirty_count` (L337-370) keys by id; `revert_scope` (L600-608):
```rust
RevertScope::Styles => self.styles = self.styles_applied.clone(),
RevertScope::Tiers => { self.tiers = self.tiers_applied.clone(); self.resync_buffers(); }
```
**`reset_styles()`**: `StyleSet::from_pairs(default builtins in default order, then pending customs in current relative order)` (customs = `!StyleSet::is_builtin(name)`).
**`styles_at_defaults()`**: first nine names equal default names in order and each `Style ==` default.
**`reset_tiers()`**: write `min`/`unit` into existing `self.tiers` entries by tier name (do not replace the Vec), then `self.resync_buffers()` (clears buffers/errors). `tiers_at_defaults()`: min/unit equal, buffer == `format!("{}", min)`, no errors.
**Landmine fix** in `tiers_dirty_count` (L373-387):
```rust
buffer_differs || e.min != base.min      // add:  || e.unit != base.unit
```
**Test pattern:** `loaded()` helper (L738-742) -> `ConfigState::load(dir.path())`; assert `dirty_count()`, `revert_scope`, `tier_rows()` buffers (`["30","20","12","8","2","0.5","0.1"]` in `load_reads_styles_and_tiers`). Tests must use temp dirs only (isolation_guard).

### `src/skin/editor.rs` - `ff_levels`

**Analog:** `ff_segmented` (L25-98). Copy its structure and keep it untouched:
```rust
let (rect, mut resp) = ui.allocate_exact_size(vec2(width, SEGMENT_HEIGHT), Sense::hover());
let paint_art = |p: &Painter, state: usize, tint: Color32| {
    let art = &skin.button[state];
    slice::paint_three(p, [&art.left,&art.middle,&art.right],
        slice::three_h_rects(rect, art.left.size(), art.right.size(), ppp), tint);
};
paint_art(&painter, 0, Color32::from_gray(170));
...
let r = ui.interact(seg, resp.id.with(i), Sense::click());
r.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, true, is_sel, *opt));
if is_sel { paint_art(&painter.with_clip_rect(seg), 1, Color32::WHITE); }
if i > 0 { painter.line_segment([pos2(seg.min.x, seg.min.y+7.0), pos2(seg.min.x, seg.max.y-7.0)],
                                Stroke::new(1.0, Color32::from_black_alpha(160))); }
let color = if is_sel { pal::HEADING } else if r.hovered() { pal::LABEL } else { pal::DISABLED };
painter.text(seg.center()+vec2(0.0,1.0), Align2::CENTER_CENTER, *opt, small_caps(16.0), color);
```
Differences: height 40, variable `seg_w` from a pure `levels_layout(total, naturals) -> Vec<f32>` (natural + 16 pad + equal slack share), `small_caps(14.0)` stacked two-line captions (14pt step), Custom uses `Sense::hover()` and `WidgetInfo::selected(RadioButton, false, ..)`, one `on_hover_text` per segment, returns `(Response, Option<usize>)`, never reporting the selected or non-clickable segment. Put it in `editor.rs`, NOT `widgets.rs` (Phase 37-02 edits it). Test pattern: headless `egui::Context::default()` + `fonts::install(&ctx)` + `ctx.run_ui(RawInput::default(), ...)` as in editor.rs L497-503; assert layout at 517/660/460 widths.

### `src/overlay/config_view.rs`

**ConfigAction** (L46-78): add `SetStrictness(Strictness)`, `ResetStyles`, `ResetTiers` (derives `Debug, Clone, PartialEq`, so `Strictness` needs those plus `Copy`).
**ConfigUi** (L82-106, `Default`): add `pub reset_armed: Option<RevertScope>`; clear in `disarm_all` (L134-138):
```rust
pub fn disarm_all(&mut self) { self.delete_armed = None; self.revert_armed = false; self.revert_scope = RevertScope::Styles; }
```
**section_title** (L266-275) currently discards the response; change to return `egui::Response` (the `None` branch returns the `ui.label` response):
```rust
Some(skin) => { crate::skin::widgets::ff_section(ui, skin, title); }
```
**show_styles** (L642-706): insert directly after `section_title(ui, ctx, "Styles")` but only after the "No styles loaded" early return check. Heading button pattern from RESEARCH Code Examples (child ui over `heading.rect`, `Layout::right_to_left(Align::Center)`, `ff_button_ex(.., "Reset to defaults", 150.0, enabled, false)`, `on_disabled_hover_text`). Existing button-on-heading analogs: `show_styles` L674-685:
```rust
ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
    let add = crate::skin::widgets::ff_button(ui, &skin, "+ Add style", 124.0);
    probe("add-style", add.rect);
    if add.clicked() && cfg.style_draft.is_none() { cfg.style_draft = Some(String::new()); cfg.focus_draft = true; }
});
```
Row: `ui.add_space(4.0)`, `ff_levels`, push `ConfigAction::SetStrictness(level)` on `Some(i)`, `ui.add_space(8.0)`, then the existing `group_heading(ui, "Built-in")`. Headless (no skin) fallback: `small_button`. Do NOT touch `show_filter`, `show_sections`, `ConfigViewCtx`, `view_ctx`.
**Test pattern:** `headless()` (L1140) and `temp_root()` (L1119, repoint to fixture).

### `src/overlay/shell.rs`

**Tiers heading** (`economy_tab` L681): `w::ff_section(ui, skin, "Tiers");` returns the heading response; draw the button via `ui.new_child(UiBuilder::new().max_rect(..).layout(Layout::right_to_left(Align::Center)))` exactly like L650-673 (economy-section button):
```rust
let mut right = ui.new_child(UiBuilder::new().max_rect(inner).layout(Layout::right_to_left(Align::Center)));
let mut r = w::ff_button_ex(&mut right, skin, section.label(), 188.0, section.enabled, section.danger());
if let Some(tip) = section.tip() { r = r.on_hover_text(tip); }
if r.clicked() { ... clicks.add = true ... }
```
Add `reset_tiers: bool` to `EconomyClicks` (L537, derives `Default, PartialEq` - existing tests compare `EconomyClicks::default()`) and `tiers_reset_enabled: bool` to `EconomyTabInputs` (L544; tests build it literally).
**Popups:** add `Popup::ResetStyles`/`ResetTiers` (L693), `PopupFlags.reset_styles/reset_tiers` (L706), entries in `active_popup` (L719-732), `pub(crate) const RESET: &str = "Reset";` beside `KEEP` (L738), `PopupData::{ResetStyles, ResetTiers}` (L759), and `popup_spec` arms modelled on `RemoveEconomy` (L888-892):
```rust
PopupData::RemoveEconomy => ("Remove economy section?",
    "Your own rules are kept. Add the section back any time.".to_string(),
    vec![(REMOVE, true, None), (KEEP, false, None)]),
```
Titles/bodies from UI-SPEC. Update the doc comment "The eight". Tests at ~L1504-1514 (`active_popup`), ~L1596 `popup_specs_match_the_contract`, ~L1739 (all-data loop) need the new variants/fields.

### `src/overlay/app.rs`

- `popup_flags` (L1809-1821): add `reset_styles: matches!(self.config_ui.reset_armed, Some(RevertScope::Styles))`, `reset_tiers` likewise.
- `popup_spec_for` (L1859+): `Popup::RemoveEconomy => shell::popup_spec(PopupData::RemoveEconomy),` -> add the two arms.
- `dispatch_popup` (L1943+), pattern from `DeleteStyle`/`RemoveEconomy` (L2003-2016):
```rust
Popup::DeleteStyle => {
    if let Some(name) = self.config_ui.delete_armed.take() && choice == 0 {
        self.handle_config_actions(ctx, vec![ConfigAction::DeleteStyle(name)]);
    }
}
```
  New: `self.config_ui.reset_armed = None;` (always) then `if choice == 0 { handle_config_actions(ctx, vec![ConfigAction::ResetStyles]) }`.
- `handle_config_actions` (L2307+): add the three actions to the `matches!` clear-events list (L2310-2320); handlers mirror `Revert` (L2392-2400): `ResetStyles` -> `state.reset_styles(); self.config_ui.reset_for_new_styles(); self.config_message = None;`; `ResetTiers` -> `state.reset_tiers(); self.config_message = None;`; `SetStrictness(l)` -> `if state.set_strictness(l) { self.config_message = None; }` (mirrors `EditStyle` L2352-2359).
- Economy click consumer (L3255-3260): after `if economy_clicks.remove { self.list.confirm_economy_remove = true; }` add `if economy_clicks.reset_tiers { self.config_ui.reset_armed = Some(RevertScope::Tiers); }`; compute `tiers_reset_enabled` where `EconomyTabInputs` is built (~L3082).
- `open_shot_popup` (L1171+): add `Some("reset-styles") => self.config_ui.reset_armed = Some(RevertScope::Styles)`, `"reset-tiers"` likewise.
- Tests use `app.dispatch_popup(&ctx, Popup::RemoveEconomy, 1/0)` (L5493-5500, 5769): copy for `ResetStyles` (choice 0 resets, 1 keeps).

### `src/overlay/screenshot.rs`

`pub(crate) const POPUPS: [&str; 8]` (L19-28) -> `[&str; 10]`, append `"reset-styles"`, `"reset-tiers"`. Check the parser test at ~L410-416 that loops `POPUPS`.

### `src/overlay/style_lines.rs` (FIX-01)

L610 (verified): `let r = ed::ff_icon_button(ui, skin, i, colour, lit, tile).on_hover_text(*label);`
`ed::ff_icon_button` already ends with `resp.on_hover_text(name)` (editor.rs:406), `name = minimap::SHAPES[shape]`. Fix: drop `.on_hover_text(*label)` at the call site; keep `let r = ed::ff_icon_button(...)`. Other tile paths (`rule_list.rs:909, 1168`, `ff_swatch` + `style_lines.rs:588`, editor.rs:241 chain) are single already. Test: headless hover counting text shapes (MEDIUM confidence), fallback source-scan guard test.

### `CLAUDE.md`

Edit the existing tables: style table (new built-in values, no Recycle/kite/exceptional), tier colour table (S/A background `248 248 242`, Star minimap, built-in alerts), "`styles-default.json` + `tiers-default.json`" row and the `artifact_guard` sentence (fixture `tests/fixtures/seed-styles.json`), "Popups: The eight" -> ten, Config/Economy notes (strictness row, Reset buttons), documented `generate` consequence (unknown style on a checkout without live `styles.json`). Phase 37-06 also edits it: re-read first. Do not touch CHANGELOG.md / README.md.

## Shared Patterns

### Pending edit, never direct write
**Source:** `config_state.rs` (`edit_style`, `revert_scope`, dirty counts) + app per-frame `combined_dirty`. All of strictness/reset are in-memory edits; Apply (`prepare_write`) is the only writer.

### Skinned popup
**Source:** `shell.rs` `Popup` / `popup_spec` / `PopupFlags` + `app.rs` `dispatch_popup`. Last button is the safe dismiss (Escape/backdrop); destructive button has `true` (red).

### Probes
Views call `probe("name", rect)` (e.g. `probe("add-style", add.rect)`); add `probe("reset-styles", ..)`, `probe("reset-tiers", ..)`, and one for the levels row.

### Test isolation
Use `tempfile::tempdir()` + `ConfigState::load(dir.path())`; never `locations::from_env` or env mutation (`tests/isolation_guard.rs`); spawned binaries pin `FERRISFILTER_HOME` and `USERPROFILE`.

## No Analog Found

| File | Role | Reason |
|---|---|---|
| Content-proportional `levels_layout` | pure layout fn | `ff_segmented` is equal-width; only `skin::widgets::tab_layout` (widgets.rs:214, pure tab layout) is a loose structural analog for a pure, unit-tested layout function |
| FIX-01 hover-count test | test | no existing egui hover/tooltip test; use RESEARCH's `tooltip_delay = 0` recipe or the source-scan fallback |

## Metadata

**Analog search scope:** `src/skin/`, `src/overlay/`, `src/models/`, `src/defaults.rs`, `tests/`
**Files scanned:** about 14 read in part
**Pattern extraction date:** 2026-10-18
