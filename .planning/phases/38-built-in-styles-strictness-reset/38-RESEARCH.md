# Phase 38: Built-in Styles, Strictness & Reset - Research

**Researched:** 2026-10-18
**Domain:** Rust / egui overlay (FerrisFilter): embedded default data, pending-edit model (`ConfigState`), skinned widgets, popups, test fixtures
**Confidence:** HIGH (everything below was read in the code or measured; the one measured fact that overturns the UI-SPEC is flagged)

## Summary

The phase is four mostly independent pieces: (1) swap `styles-default.json` for the author's nine built-ins and
freeze today's file as a test fixture, (2) a pure `Strictness` model plus a one-line level row in the Styles
section, (3) "Reset to defaults" for styles and tiers with two new popups, (4) remove one duplicated
`on_hover_text` (FIX-01). No new crates. All pending-edit plumbing already exists
(`ConfigState` working vs applied snapshots, dirty counts, `RevertScope`, `ConfigAction`, popup layer).

Three findings the planner must act on:

1. **The fixture swap touches ~100 unit tests plus 5 integration test files, not just the two named in CONTEXT.**
   I ran `cargo test --lib` in a scratch copy of the worktree with the new `styles-default.json`: **103 of 1444 lib
   tests fail** (modules: emitter 27, compile 13, config_view 13, models::config 16, overlay::app 8, config_state 8,
   loader 6, rules 5, styled 4, rule_list 3). Every one reads `styles-default.json` (directly, via `repo_file(...)`,
   via `include_str!`, or via `StyleSet::committed_for_tests()`) and expects the old content (twelve styles,
   `Recycle`/`kite`/`exceptional`, `S` with `uwu.mp3`, no `enableDropSound`). Integration tests were not run but read
   the same file by static grep (`cli_generate`, `cli_errors`, `dynamic_filter`, `filter_roundtrip`, `sync_prices`).
   The new file itself parses and validates (no "failed validation" panic).
2. **The UI-SPEC's strictness layout does not fit.** It assumes about 600pt of content width. The real width at the
   660pt window is about **517-520pt** (frame content 553 wide, minus 2x10 shrink, minus scrollbar 14+4; the
   existing `table_columns(517.0)` test uses the same number). With Custom fixed at 72pt the six levels get 74pt
   each; measured small-caps widths (below) show `Scrounging` is 78.9pt at 14pt, 73.3 at 13pt and 67.6 at 12pt, so
   even 12pt does not fit in 74-8=66pt, and the spec forbids ellipsis. Use content-proportional segment widths
   (below) instead of equal widths.
3. **Built-in style names cannot actually be renamed.** `StyleSet::validate` requires the nine `REQUIRED` names to
   exist and a built-in's id to be exactly `builtin:<name>`; `rename_custom` refuses built-ins. So "matched by id"
   (CONTEXT) and "matched by name" are equivalent for S-F in any loadable file. Match by id via
   `StyleSet::builtin_id(letter)` / `name_of_id` as CONTEXT says; do not write rename-repair code for built-ins.

**Primary recommendation:** Land the data swap first as a no-behaviour-change step (fixture copied from the old
file, every test reader repointed, suite green), then build the pure model (`strictness.rs` + `ConfigState`
methods), the skin widget, and the FIX-01 one-liner in parallel, then wire view/app/shell/popups last.

## User Constraints (from CONTEXT.md)

### Locked Decisions
- `styles-default.json` becomes exactly the built-ins from `.planning/research/author-styles-2026-10-17.json`: `S`, `A`, `B`, `C`, `D`, `E`, `F`, `unchanged`, `hidden`, with their values and ids as in the snapshot, in the snapshot's order. No custom styles ship (`Recycle`, `kite`, `exceptional`, `Build Items`, `White bases` are the author's own). None of the shipped styles uses a custom sound file. CLAUDE.md's style and tier tables are updated to the new values and to "no custom styles ship".
- CLI `generate` is not reworked. Today's `styles-default.json` is frozen as a test fixture (`tests/fixtures/seed-styles.json` or similar) that the generate tests and `artifact_guard` read, so the committed fixtures stay byte-identical and no re-baseline is needed. `config.json` is not changed. Known consequence, documented in CLAUDE.md: `generate` on a checkout with no live `styles.json` reports the unknown style (`Recycle` / `kite` / `exceptional`); the author's own `generate` keeps working through their live `styles.json`.
- A user's own `styles.json` is kept as it is; built-in ids stay `builtin:S`..`builtin:F`.
- Strictness row: top of the Styles section, above the Built-in table. One line is the hard requirement. Seven buttons `Custom | Flat Broke | Scrounging | Getting By | Comfy | Loaded | Div Farmer`; two-line (stacked) captions are fine. Segmented look as Show | Hide. Each level's tooltip names what it hides. Custom is leftmost and display-only (never clickable, dim caption).
- Levels hide nothing, F, E-F, D-F, C-F, B-F of `builtin:S`..`builtin:F` (matched by id). A click changes the pending style set like switch clicks (Config tab ` *`, Revert undoes, Apply writes). Highlight derived from current S-F visibility; any other pattern shows Custom. Non-tier built-ins and custom styles never touched. A tooltip says rules using tier styles (user rules included) are affected. A fresh install starts on Flat Broke.
- Reset to defaults: button at the right of the Styles heading (Config) and of the Tiers heading (Economy); greyed when the pending set equals the defaults. Popups "Reset styles to defaults?" / "Reset tiers to defaults?" with a red Reset and Keep (`ff_modal`; Escape/outside click = Keep). Styles reset: built-ins back to default values and order, customs kept after them, strictness reads Flat Broke. Tiers reset: default cutoffs. Both are one pending edit.
- FIX-01: find why a minimap icon tile's tooltip is drawn twice and keep exactly one, everywhere tiles are drawn.

### Claude's Discretion
- Widget composition of the strictness row (caption wrapping, font size), where reset helpers live in `config_state.rs`, the frozen fixture's file name.

### Deferred Ideas (OUT OF SCOPE)
- Reworking CLI `generate` / `config.json` against the new defaults (author: later).

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BSTY-01 | Built-ins are the author's reworked set; no custom style ships; `tiers-default.json` unchanged | New `styles-default.json` = snapshot lines S..F, unchanged, hidden (see Data section); validates today; fixture swap list |
| BSTY-02 | No built-in uses a custom sound file | Unit test over the embedded default: every `sound` has `custom == None`; snapshot uses `id` 6/2 only |
| BSTY-03 | User's `styles.json` kept; ids stay `builtin:S`..`F` | Resolver keeps live file over embedded default (`read_effective`); test: a temp home with an old-style `styles.json` loads unchanged and `prepare_write` keeps its bytes; default ids asserted |
| STRICT-01 | One exclusive button row with the seven captions | `skin::editor::ff_levels` (new) + `show_styles` insertion; proportional widths; one-row test at 517pt |
| STRICT-02 | Click sets S-F visibility as one pending edit | `ConfigAction::SetStrictness` -> `ConfigState::set_strictness` (loop of `edit_style(Action)`); dirty count/Revert already work |
| STRICT-03 | Highlight derived from S-F visibility; Custom display-only | Pure `strictness_of(&StyleSet)`; Custom segment senses hover only |
| STRICT-04 | Non-tier built-ins and customs untouched; tooltip names affected rules | Model touches only the six ids; tooltip text from UI-SPEC |
| RESET-01 | Styles reset button + popup, one pending edit | `ConfigState::reset_styles`, `styles_at_defaults`, `Popup::ResetStyles` |
| RESET-02 | Tiers reset button + popup | `ConfigState::reset_tiers`, `tiers_at_defaults`, button in `shell.rs::economy_tab`, `Popup::ResetTiers` |
| FIX-01 | Minimap tooltip once | Source found: `style_lines.rs:610` (see Pitfall 1) |
</phase_requirements>

## Architectural Responsibility Map

Single-process desktop app (egui); tiers map to layers of the crate.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Default style/tier data | Embedded data (`src/defaults.rs`, repo-root JSON) | Test fixtures (`tests/fixtures/`) | The embedded copy is `include_str!` of the repo-root file; tests must stop reading that file for old content |
| Strictness derive/apply, reset, "at defaults" | Pure model (`overlay/strictness.rs` new + `config_state.rs`) | — | Layer rule: no egui types in model files; provable with temp dirs |
| Strictness row, Reset buttons | View (`config_view.rs`, `shell.rs`) | Skin (`skin/editor.rs` new widget) | Views return `ConfigAction`s / click flags, never mutate state |
| Popups and action dispatch | App (`app.rs`, `shell.rs` popup layer) | `screenshot.rs` | `dispatch_popup` applies results; `popup_spec` owns text |
| Persistence | Existing Apply path (`prepare_write`) | — | Reset/strictness are ordinary pending edits; nothing new is written directly |

## Standard Stack

No new dependencies. Everything uses what is already in `Cargo.toml`: egui/eframe 0.36.1 [VERIFIED: Cargo.lock], `serde_json`, `indexmap` (inside `StyleSet`), `tempfile` (tests), `insta` (snapshot tests), `assert_cmd` (CLI tests).

**Package Legitimacy Audit:** not applicable, this phase installs no external packages. [VERIFIED: no Cargo.toml change is needed by any requirement]

## Architecture Patterns

### Data flow

```
repo-root styles-default.json --include_str!--> defaults::STYLES_DEFAULT  (embedded; the app's default + reset source)
tests/fixtures/seed-styles.json (frozen old file) --> generate tests, artifact_guard, unit-test temp homes

ConfigState (pending StyleSet, applied snapshot)
   ^  edit_style / set_strictness / reset_styles / reset_tiers  (called by app.handle_config_actions)
   |
ConfigAction  <---- config_view::show_styles (strictness row, Styles heading Reset button)
              <---- shell::economy_tab EconomyClicks.reset_tiers (Tiers heading Reset button)
   |
config_ui.reset_armed = Some(RevertScope::Styles|Tiers)  -> PopupFlags -> ff_modal -> dispatch_popup(choice 0) -> ConfigAction::ResetStyles/ResetTiers
```

### Recommended file changes
```
styles-default.json                      # replaced (9 built-ins, render_styles-exact bytes)
tests/fixtures/seed-styles.json          # NEW: byte copy of today's styles-default.json (copy BEFORE replacing)
src/overlay/strictness.rs                # NEW pure: Strictness enum, hidden sets, strictness_of, captions/tooltips
src/overlay/config_state.rs              # + set_strictness, strictness, reset_styles, styles_at_defaults, reset_tiers, tiers_at_defaults; unit-aware tier dirty
src/models/config.rs                     # + StyleSet::shipped_defaults() (non-test), committed_for_tests() -> fixture
src/skin/editor.rs                       # + ff_levels widget + pure width/caption helpers
src/overlay/config_view.rs               # ConfigAction variants, ConfigUi.reset_armed, strictness row, Styles heading button
src/overlay/shell.rs                     # Tiers heading button, EconomyClicks/EconomyTabInputs fields, Popup/PopupFlags/PopupData/popup_spec
src/overlay/app.rs                       # popup_flags, popup_spec_for, dispatch_popup, handle_config_actions, economy click handling, screenshot popup names
src/overlay/screenshot.rs                # POPUPS += "reset-styles", "reset-tiers"
src/overlay/style_lines.rs               # FIX-01: drop the redundant .on_hover_text at the ff_icon_button call
CLAUDE.md                                # style/tier tables, repo-layout row, popups "The eight" -> ten, generate consequence
```

### Pattern 1: Strictness model (pure)

Tier letters in order `S A B C D E F`. Levels (index = button order after Custom): `FlatBroke` hides none,
`Scrounging` {F}, `GettingBy` {E,F}, `Comfy` {D,E,F}, `Loaded` {C,D,E,F}, `DivFarmer` {B,C,D,E,F}.

```rust
// src/overlay/strictness.rs (no egui types)
pub enum Strictness { Custom, FlatBroke, Scrounging, GettingBy, Comfy, Loaded, DivFarmer }
pub const TIERS: [&str; 7] = ["S","A","B","C","D","E","F"];
impl Strictness {
    pub const LEVELS: [Strictness; 6] = [/* FlatBroke..DivFarmer */];
    pub fn hidden(self) -> &'static [&'static str] { /* F, E-F, ... ; Custom => &[] */ }
}
/// Visibility of builtin:S..builtin:F -> level. A missing tier id, or any non-listed pattern, is Custom.
pub fn strictness_of(set: &StyleSet) -> Strictness {
    // for each letter: name = set.name_of_id(&StyleSet::builtin_id(letter)); hide = style.action == Hide
    // find the level whose hidden() set equals the set of hidden letters
}
```
`ConfigState::set_strictness(level)` loops the six letters and calls the existing `edit_style(name, StyleEdit::Action(Hide|Show))`, skipping a letter whose id is absent (UI-SPEC choice 8). It is automatically one pending edit as far as the user sees it: `styles_dirty_count` counts changed styles, Revert restores the snapshot, the tab label shows ` *`, and the apply state flips via the per-frame `combined_dirty` check in `app.rs` (~L2935-2943), so `handle_config_actions` needs no `apply_state` code for it (unlike `RenameStyle`/`MoveStyle` which set it explicitly).

Clicking the highlighted level, or Custom, must emit no action (UI-SPEC). `edit_style` returns bool; `set_strictness` should return whether anything changed so the app clears `config_message` only then.

### Pattern 2: Reset (pure, in `ConfigState`)

- Default sets: parse `crate::defaults::STYLES_DEFAULT.text` / `TIERS_DEFAULT.text` once (`std::sync::OnceLock`), `assign_missing_ids()` (no-op, ids are in the file). Do not read repo-root files at runtime (Phase 36 contract: only the embedded copy exists in a release build).
- `reset_styles()`: new set = `StyleSet::from_pairs(default builtins in default order, then the pending customs in their current relative order)` (`from_pairs` is `pub`, keeps ids as given). Then the app calls `config_ui.reset_for_new_styles()` (collapses expanded row, clears rename drafts, bumps epochs). Dirty counting already handles value change, a changed name (impossible for built-ins) and `reordered`.
- `styles_at_defaults()` (button enabled = `!styles_at_defaults()`): the first nine names of the pending set equal the default names in order AND every one of them `==` the default `Style`. Customs ignored (UI-SPEC choice 10). Note the author's own live file has `Recycle` between `F` and `unchanged`, so the button is enabled for them until they reset and Apply; correct per the contract.
- `reset_tiers()`: **do not replace `self.tiers` wholesale.** Write `min`/`unit` into the existing entries by tier name (keeps the user's file order, so `tiers_equal` positional comparison and `render_tiers` stay consistent), then `resync_buffers()` (clears buffers/errors). `tiers_at_defaults()`: for each default entry, the pending entry of that tier has equal `min` and `unit` and its buffer text equals `format!("{}", min)` and no tier errors exist.
- **Landmine (unit-blind dirty count):** `tiers_dirty_count` compares only `min` and the buffer text, not `unit`. A user whose `tiers.json` uses another unit (the fallback pair uses `exalt`) would see Reset change units with `dirty_count() == 0`, so `can_apply` stays false and Apply is disabled. Extend `tiers_dirty_count` with `|| e.unit != base.unit` (small, safe; `Unit` derives `PartialEq`).

### Pattern 3: Reset button on a heading
`widgets::ff_section` returns the heading `Response` (`resp.rect` is the full-width heading strip, ~37pt high). Draw the button in `ui.new_child(UiBuilder::new().max_rect(resp.rect).layout(Layout::right_to_left(Align::Center)))` after the call, as `shell.rs` already does for the economy-section button (~L650: `new_child` + `Layout::right_to_left`). The button is 34pt high so it fits the strip; keep it 8pt from the right edge (`rect.shrink2(vec2(8.0, 0.0))`). `config_view::section_title` currently returns `()` and discards the response; change it to return `egui::Response` (callers use it as a statement, so Phase 37's `section_title(ui, ctx, "Filter")` call keeps compiling).

### Pattern 4: Popups
Add `Popup::ResetStyles`, `Popup::ResetTiers`, matching `PopupFlags` fields, `PopupData::{ResetStyles, ResetTiers}`, `popup_spec` arms (`("Reset", true, None), ("Keep", false, None)`; add a `RESET` constant next to `KEEP`), `popup_spec_for`, `dispatch_popup` arms. State lives in `ConfigUi`: `reset_armed: Option<RevertScope>` (reuse the enum: `Styles` / `Tiers`), cleared in `disarm_all`. Dispatch choice `0` -> `handle_config_actions(vec![ConfigAction::ResetStyles])` / `ResetTiers`; any choice clears `reset_armed`. `active_popup` order is irrelevant for these (only one can be armed). Tests in `shell.rs` (~L1485, L1506) and `app.rs` that build `PopupFlags` literally need the new fields (`..Default::default()` if they do not already use it).

The styles button sets `cfg.reset_armed` directly inside `show_styles` (like `cfg.style_draft`); the tiers button is in `shell::economy_tab`, which has no `ConfigUi`, so add `reset_tiers: bool` to `EconomyClicks` and `tiers_reset_enabled: bool` to `EconomyTabInputs`, and let `app.rs` (the `economy_clicks` consumer next to `clicks.remove`) set `config_ui.reset_armed = Some(RevertScope::Tiers)`. Existing shell tests assert `EconomyClicks::default()` equality and build `EconomyTabInputs` literally: update them.

`ConfigAction::ResetStyles` / `ResetTiers` / `SetStrictness(Strictness)` go into the "clear stale messages" `matches!` list at the top of `handle_config_actions` (`app.rs` ~L2309). `ResetStyles` handler: `state.reset_styles()`, `config_ui.reset_for_new_styles()`, `config_message = None`. `ResetTiers`: `state.reset_tiers()`, `config_message = None`. `RevertScoped` already undoes both and `Popup::DiscardConfig` already counts them via `scope_dirty_count`.

Screenshot mode: add `"reset-styles"`, `"reset-tiers"` to `screenshot.rs::POPUPS` (array length 8 -> 10) and arms in `app.rs::open_shot_popup` that set `config_ui.reset_armed`. Check the argument-parser test that enumerates `POPUPS`.

### Pattern 5: The levels widget (UI-SPEC deviation, measured)

Measured Fontin SmallCaps widths in this build (egui 0.36.1, `fonts_mut(|f| f.layout_no_wrap(...))`) [VERIFIED: measured in a scratch copy]:

| Caption | 14pt | 13pt | 12pt |
|---------|------|------|------|
| Custom | 49.5 | 45.9 | 42.4 |
| Flat Broke (one line) | 67.8 | 63.0 | 58.1 |
| Flat / Broke (stacked, widest word) | 37.4 | 34.7 | 32.0 |
| Scrounging | 78.9 | 73.3 | 67.6 |
| Getting By (one line) | 68.3 | 63.5 | 58.6 |
| Getting (stacked widest) | 50.5 | 46.9 | 43.3 |
| Comfy | 40.5 | 37.6 | 34.7 |
| Loaded | 45.5 | 42.2 | 39.0 |
| Div Farmer (one line) | 72.0 | 66.9 | 61.8 |
| Farmer (stacked widest) | 46.6 | 43.3 | 40.0 |

Real content width is about 517-520pt. Recommended layout: segment width `i` = `natural_i + 16` (8pt padding each side) plus an equal share of the slack, where `natural_i` is the widest line of that caption at 14pt (Custom 49.5, Flat Broke 37.4 stacked, Scrounging 78.9, Getting By 50.5 stacked, Comfy 40.5, Loaded 45.5, Div Farmer 46.6 stacked). Sum of naturals is 348.9, plus 7x16 = 461, so about 56pt of slack at 517pt: every caption fits at 14pt with no shrink and no ellipsis. Keep the UI-SPEC shrink rule (14 -> 13 -> 12, then padding 8 -> 4) only as the fallback for narrower widths (headless tests run at 460pt). Make the layout a pure function `levels_layout(total_width, naturals) -> Vec<f32>` and unit-test it at 517 and 660 and 460 widths. Keep stacked two-word captions (spec) at 14pt, line step 14, row height 40.

Widget API suggestion (in `skin/editor.rs`, same art/dividers/hover/`WidgetInfo` radio semantics as `ff_segmented`, which must stay untouched):

```rust
pub struct Level<'a> { pub lines: &'a [&'a str], pub tip: &'a str, pub clickable: bool }
/// Returns the index clicked this frame (never a non-clickable one, never the already-selected one).
pub fn ff_levels(ui: &mut Ui, skin: &Skin, selected: usize, levels: &[Level]) -> (Response, Option<usize>)
```
Custom: `Sense::hover()`, `WidgetInfo::selected(RadioButton, false /*enabled*/, is_sel, "Custom")`, caption `DISABLED` always unless selected (then art state 1 + `HEADING`). Tooltips via `on_hover_text` on each segment's interact response (a single call per segment: see FIX-01). Put the widget in `skin/editor.rs`, not `skin/widgets.rs`: Phase 37-02 edits `widgets.rs`/`textures.rs`.

### Anti-patterns
- Reading `styles-default.json` from disk in app code to reset (use the embedded copy).
- Replacing `ConfigState.tiers` wholesale on reset (breaks file order and the unit-blind dirty count, above).
- Adding a `ConfigViewCtx` field for the reset buttons (Phase 37 is growing that struct; compute from `state` instead and avoid a merge conflict).
- A "Strictness" caption or explanatory paragraph (memory: overlay UI is not texty; tooltips only).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Dirty/revert for strictness and reset | A new snapshot or undo stack | `ConfigState` pending vs `styles_applied` + `revert_scope` | Already counts value, membership and order changes |
| Default sets | A second copy of the defaults in Rust | `defaults::STYLES_DEFAULT` / `TIERS_DEFAULT` parsed once | Single source; `embedded_text_equals_the_committed_file` guards the bytes |
| Confirm dialogs | New modal code | `skin::popup::ff_modal` via `Popup`/`popup_spec` | Escape/backdrop = last button, stacking rules, hide/summon persistence |
| Writing the new default file by hand | Hand-formatted JSON | Take the 9 snapshot lines verbatim (they are already `loader::render_styles` output) | Keeps an unedited default byte-identical to what Apply would write |
| Test isolation | Setting env vars | `ConfigState::load(<tempdir>)`, `AppDirs::at(<tempdir>)`; spawned CLI/`ff` runs pin `FERRISFILTER_HOME` and `USERPROFILE` | `no_env_mutation` and `spawns_that_resolve_the_folder_set_home` in `tests/isolation_guard.rs` fail otherwise; `locations::from_env` panics under `cfg(test)` |

## Data: the new `styles-default.json`

Source lines: `author-styles-2026-10-17.json` lines 2-8 (S..F), 10 (`unchanged`), 11 (`hidden`); drop `Recycle`, `exceptional`, `Build Items`, `kite`, `White bases`; last kept line without trailing comma; file wrapped in `{` / `}` with a trailing newline. Those lines are exactly `loader::render_styles` output (compact object, struct field order, `enableDropSound` only on S/A/B), so add a test `render_styles(parse(file)) == file` [VERIFIED by reading `render_styles` and the snapshot]. Values: S/A background `248 248 242`, S size-0 Pink Star, A size-1 Purple Star, B/C/D size-2 Star, sound ids 6/6/2/2, `enableDropSound` on S/A/B, C/D/E/F per CONTEXT. `tiers-default.json` unchanged.

I verified in a scratch copy that this file loads: `StyleSet::validate` passes (required nine present, ids `builtin:<name>`), and 1341 of 1444 lib tests pass with it; the 103 failures are all old-content expectations.

### Fixture swap: the full reader list (the likely landmine)

Rule: **a test that needs "a valid style set with customs / the old values" reads `tests/fixtures/seed-styles.json`; a test about the shipped default or the embedded fallback reads the real file / `defaults::STYLES_DEFAULT`.** Do the swap while `styles-default.json` still has the old bytes (step 1: copy the file to the fixture; step 2: repoint readers; run suite green; step 3: replace `styles-default.json`).

Failing lib tests by module (all in the scratch run): `compile::tests` (13: `load_inputs_*`, `force_fallback_*`, `compile_produces_filter_text_from_loaded_inputs`), `emitter::tests` (27, via `committed_for_tests`-style load at `emitter.rs:698`), `loader::tests` (6, `loader.rs:381-415,493`), `models::config` (`committed_for_tests` L935, L1516, L2141; 16 incl. `style_id_tests`), `overlay::app::tests` (8; helper `config_root()` ~L3671 and L4470, L5170, L5243, L5265, L5461), `overlay::config_state::tests` (8; L727 and the `-default.json` loops L822+), `overlay::config_view::tests` (13; `temp_root()` L1119-1130), `overlay::rule_list::tests` (3; L1913, 2045, 2060, 2730), `overlay::rules::tests` (5; L2336), `styled::tests` (4). Also repoint without having failed in the scratch run (verify by running): `overlay/apply_all.rs` L461/469/1066/1170, `overlay/body.rs` L4049/5689/5698, `overlay/classes.rs:50`, `dynamic.rs` L455/567/617.

Integration tests (static; not run): `tests/cli_generate.rs` L36, 74, 228, 298-314 (explicit `--styles styles-default.json` run in the repo root), 515, 650, 691, 746 (+ L786, 829 text-edit/remove of the copied `styles-default.json`), `tests/cli_errors.rs:42` (`valid_styles()`), `tests/dynamic_filter.rs` L41, 444, `tests/filter_roundtrip.rs:173`, `tests/sync_prices.rs:668`. For copy loops of the form `for f in ["config.json","styles-default.json",...] { copy(manifest.join(f), dir.join(f)) }` (cli_generate L746, config_state L822, app L3761) the destination name stays `styles-default.json` and the source becomes the fixture, so use `(src, dst)` pairs. Tests that assert fallback to the **embedded** default through `InputPaths::in_home` (`config_state::default_files_in_the_folder_are_ignored`, `load_falls_back_to_defaults_and_creates_nothing`, `app.rs` ~L3792-3884) must compare against `defaults::STYLES_DEFAULT.text`, not the copied fixture, because `in_home` resolves the embedded default and ignores `*-default.json` in the folder. Tests on the `generate` path (`InputPaths::from dir`) use `DefaultSource::File(dir/styles-default.json)`, so the fixture copied under that name works there.

Cheap lever: change `StyleSet::committed_for_tests()` (`models/config.rs:935`) to `include_str!` the fixture and add a non-test `StyleSet::shipped_defaults()` (embedded, parsed, ids assigned) for the new code and new default tests. That alone fixes most `committed_for_tests` callers.

Phase 37 overlap: 37-07 also edits `tests/cli_errors.rs` and `tests/cli_generate.rs`; 37-03 edits `tests/dynamic_filter.rs`. Phase 38 runs after 37: re-read those files at execution time, do not rely on the line numbers above.

`generate` landmine check (answered): `config.json` references `Recycle` (2), `exceptional` (3), `kite` (2), `hidden` (3), `unchanged` (11) [VERIFIED: grep]. `compile.rs:413` calls `styles.check_rules(...)`, so `ferrisfilter generate` against the new `styles-default.json` fails with the unknown-style error. Only `generate` calls it; the overlay does not (it uses `config.json` only for the class catalog and `config_style_refs` for delete warnings). `generate` does not use the embedded default (it uses `DefaultSource::File(<cwd>/styles-default.json)`), so tests that spawn `generate` in a temp dir with the fixture copied are safe, and `generate_without_any_styles_file_names_both` (removes the copied default, expects an error naming both files) keeps passing.

Other stale references: `tests/ui/test_ff.filter` case 3 and its README table use the `kite` preset (a fresh install no longer has it; the rule shows `unknown style`). The manual UI README is not CHANGELOG/README.md, so a one-line note is allowed but optional; `screenshot.rs` `delete-style` already falls back to the text `kite` when no custom style exists.

## FIX-01: the duplicate tooltip

Found: `src/overlay/style_lines.rs:610`
```rust
let r = ed::ff_icon_button(ui, skin, i, colour, lit, tile).on_hover_text(*label);
```
`skin::editor::ff_icon_button` (`editor.rs:357-407`) already ends with `resp.on_hover_text(name)` where `name = minimap::SHAPES[shape]`, the same text as `*label` (iterating `minimap::SHAPES`). Two `on_hover_text` calls on one response stack two identical tooltips (egui offsets additional tooltips of the same widget below the first), which is the "one on top of the other" in FIX-01. `ff_icon_button` has exactly one caller. Fix: delete `.on_hover_text(*label)` at the call site (ownership rule from the UI-SPEC: the painting widget owns its tooltip). This one site covers both the CONFIG style editor and a Custom rule's STYLE lines (both draw the shared `style_lines` component). The other tile paths are single already: colour swatches (`ff_swatch` has no own tooltip, `style_lines.rs:588` adds one), rule chips (`rule_list.rs:909, 1168` call `resp.on_hover_text(tip)` once each), the Config Icon column paints without a tooltip. The `editor.rs:241` `on_hover_text(&text).on_disabled_hover_text(text)` chain is not a double (the second only fires on a disabled response); leave it.

Test: a headless frame run with the pointer over a shape tile, tooltip delay 0 (`ctx.style_mut(|s| s.interaction.tooltip_delay = 0.0)` and still-only off), a few frames, then count text shapes equal to the shape name in the output (expect 1; fails with 2 before the fix). I could not run an egui hover test here, so treat the exact knobs as MEDIUM confidence; fallback guard if the harness proves flaky: a source-scan unit test asserting the `ff_icon_button(` call in `style_lines.rs` is not followed by `.on_hover_text`.

## Common Pitfalls

### Pitfall 1: Equal-width strictness segments overflow
See Summary item 2 and the measurement table. Use proportional widths; verify at 517pt in a test and in a `--screenshot --tab config` run.

### Pitfall 2: Swapping the default before repointing tests
The suite goes red in 100+ places at once and hides real regressions. Order: fixture copy -> repoint -> green -> swap file.

### Pitfall 3: Tests resolving the real folders
Spawned `ff`, `apply` or `sync` without `.env("FERRISFILTER_HOME")` and `.env("USERPROFILE")` fail `isolation_guard`. New unit tests use `ConfigState::load(tempdir)`; never call `locations::from_env` outside the allowed files. Screenshot runs for UAT use a temp home and `FERRISFILTER_GAME_DIR` under `%TEMP%` (the mode refuses otherwise).

### Pitfall 4: Strictness vs "Hide" tiers and the economy section
Hiding a tier style hides every block using it (user rules and the generated economy blocks) on the next Apply; this is the intended effect and is what the tooltip says. The Rules tab shows pending Show/Hide via `sync_working_styles`, so no extra wiring is needed.

### Pitfall 5: Reset when a built-in is missing
`StyleSet::validate` rejects a set missing any built-in at load, so `reset_styles` always has all nine to replace. Still build from defaults + customs rather than patching entries in place.

### Pitfall 6: Phase 37 compose points
Phase 37 (running in parallel) edits `config_view.rs` `show_filter`/`filter_draft_row`/`ConfigViewCtx`/`ConfigAction` (adds `BrowseFolder`, `RefreshLists`) and the Filter-section tests, `app.rs`, `shell.rs` (37-05), `skin/widgets.rs` (adds `widgets::ff_icon_button`, a different function from `editor::ff_icon_button`; do not confuse them when grepping), `CLAUDE.md` (37-06). Phase 38 touches in `config_view.rs` only: `ConfigAction` (add three variants), `ConfigUi` (+`reset_armed`, `disarm_all`), `section_title` (return value), `show_styles` (heading button + row inserted right after `section_title(ui, ctx, "Styles")`, before the "No styles loaded" early return check is fine only if the row is skipped when the lists are empty), and a new helper fn for the row; test helpers `temp_root()` (fixture) plus new tests. It must not edit `show_filter`, `show_sections`, `ConfigViewCtx` or `view_ctx`. Re-read the file after 37 lands; merge conflicts are most likely in the `ConfigAction` enum, `app.rs` `popup_flags`/`handle_config_actions`, and `CLAUDE.md`.

### Pitfall 7: Slow suite
`cargo test --lib` took about 15 minutes in the scratch run (cold target, 1444 tests). Plans should use filtered runs per task (`cargo test --lib -- overlay::config_state`) and the full suite once at the phase gate.

## Code Examples

Reset button on a heading (pattern already used in `shell.rs`):
```rust
// Source: src/overlay/shell.rs economy section button (~L650) + src/skin/widgets.rs ff_section/ff_button_ex
let heading = crate::skin::widgets::ff_section(ui, skin, "Styles");
let mut right = ui.new_child(
    egui::UiBuilder::new()
        .max_rect(heading.rect.shrink2(vec2(8.0, 0.0)))
        .layout(Layout::right_to_left(Align::Center)),
);
let enabled = !state.styles_at_defaults();
let mut r = crate::skin::widgets::ff_button_ex(&mut right, skin, "Reset to defaults", 150.0, enabled, false);
r = if enabled { r.on_hover_text("Restore the built-in styles. Your custom styles are kept.") }
    else { r.on_disabled_hover_text("Already at the defaults.") };
probe("reset-styles", r.rect);
if r.clicked() { cfg.reset_armed = Some(RevertScope::Styles); }
```
(`ff_button_ex` disabled uses `Sense::hover()`, so `on_disabled_hover_text` is the right tooltip call. Headless tests have `ctx.skin == None`; fall back to `small_button` like the other views.)

## State of the Art

| Old | Current | Impact |
|-----|---------|--------|
| `styles-default.json` is also the byte-for-byte generate spec | Same file is only the app default; the generate spec is the frozen fixture | CLAUDE.md "Repository layout" and the generate prose must say so |
| Repo-root `config.json` rules use author styles | Unchanged, still used by CLI `generate` only | `generate` on a bare checkout reports unknown style; documented, deferred |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | egui stacks a second `on_hover_text` on the same response as a visible second tooltip (explains FIX-01) | FIX-01 | Low: the call-site duplication is verified in code; if egui merged them, the user-visible bug would be elsewhere and the planner should reproduce via the hover test first |
| A2 | The headless hover test can be driven with `tooltip_delay = 0` over a few frames | FIX-01 | Medium: fallback source-scan test given |
| A3 | Real content width at a 660pt window is about 517-520pt | Pattern 5 | Low: derived from `frame_layout` (content 553 at 655), 10pt shrink each side and `SCROLLBAR_WIDTH + 4`; the existing `table_columns(517.0)` test agrees; confirm with a `--screenshot --tab config` run |
| A4 | Integration tests (`cli_*`, `dynamic_filter`, `filter_roundtrip`, `sync_prices`) will fail on the new default | Fixture swap | Low: static grep only; repointing them is harmless either way |

## Open Questions (RESOLVED)

1. **Does the author want the Reset button also greyed while the styles equal defaults but customs exist?** RESOLVED: follow the UI-SPEC (customs never count); recorded as a choice made in the author's place.
2. **Strictness `Custom` on a fresh install where a built-in is hidden by the default?** RESOLVED: none of the nine default styles is `Hide` except `hidden` (not a tier), so a fresh install reads Flat Broke. No action.

## Project Constraints (from CLAUDE.md)

- Do not edit `CHANGELOG.md` or `README.md` in this phase's plans (orchestrator rule); CLAUDE.md edits are in scope: update the style table, tier table (S/A background now pale `248 248 242`, alerts are built-in sounds, Star minimap icons, `enableDropSound`), the "styles-default.json + tiers-default.json" row (no longer the generate spec; the fixture is), the `artifact_guard` sentence, "Popups: The eight" to ten, the strictness row and Reset buttons in the Config/Economy notes, and the documented `generate` consequence.
- Keep `[Unreleased]` handling to the orchestrator.
- Tests must not touch the real `%APPDATA%\FerrisFilter` or game folder (see Pitfall 3).
- Overlay UI avoids walls of text; explanations are tooltips.
- Layer rule: model files (`config_state.rs`, new `strictness.rs`) hold no egui types; views return actions.
- Git: milestone branch workflow; the orchestrator commits.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust nightly toolchain + cargo | build/tests | yes | `rust-toolchain.toml` nightly | — |
| Windows desktop (egui headless tests need no GPU) | unit tests | yes | — | — |
| GPU/window for `--screenshot` UAT | visual checks of the strictness row and popups | not verified here | — | Unit tests at 517pt for fit; screenshot is a manual UAT step |
| Python | — | no (Microsoft Store stub only) | — | not needed |

## Validation Architecture

(`workflow.nyquist_validation` is `false` in `.planning/config.json`; this section is included because the orchestrator asked for it.)

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in tests; `insta` (snapshots), `assert_cmd` + `predicates` (CLI), `tempfile` |
| Config file | none (`Cargo.toml`); nightly via `rust-toolchain.toml` |
| Quick run command | `cargo test --lib -- <module filter>` e.g. `overlay::config_state`, `overlay::config_view`, `overlay::strictness`, `skin::editor`, `defaults` |
| Full suite command | `cargo test` (slow: lib alone ~15 min cold) |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BSTY-01 | Embedded default = nine built-ins in snapshot order/values; `render_styles(parse(file)) == file`; `tiers-default.json` bytes unchanged | unit | `cargo test --lib -- defaults::tests models::config` | extend `defaults.rs` tests, Wave 0 |
| BSTY-01 | Generate output stays byte-identical on the fixture | integration | `cargo test --test cli_generate` (incl. `artifact_guard`, `generate_snapshot`) | yes, repoint to fixture |
| BSTY-02 | No default style has `sound.custom` | unit | `cargo test --lib -- defaults::tests` | Wave 0 |
| BSTY-03 | A temp home with an old `styles.json` loads unchanged, `prepare_write` keeps its bytes; default ids are `builtin:*` | unit | `cargo test --lib -- overlay::config_state` | Wave 0 |
| STRICT-01 | Seven captions drawn on one row at 517pt; widest fits at 14pt | unit (headless egui) | `cargo test --lib -- skin::editor overlay::config_view` | Wave 0 |
| STRICT-02 | Each level sets S-F exactly (hide none/F/E-F/D-F/C-F/B-F); one pending edit; Revert undoes | unit | `cargo test --lib -- overlay::strictness overlay::config_state` | Wave 0 |
| STRICT-03 | `strictness_of` for every pattern incl. S hidden, a gap, missing id -> Custom; fresh default -> FlatBroke; Custom/active click emits nothing | unit | same | Wave 0 |
| STRICT-04 | `set_strictness` leaves `unchanged`, `hidden` and customs byte-equal; tooltip text contains "yours included" | unit | same | Wave 0 |
| RESET-01 | `reset_styles`: builtins default + first, customs kept in order, strictness FlatBroke, renamed-custom ids kept; `styles_at_defaults` false/true; dirty count; Revert undoes; popup answers (choice 0 resets, 1 keeps) | unit + app test | `cargo test --lib -- overlay::config_state overlay::app overlay::shell` | Wave 0 |
| RESET-02 | `reset_tiers` writes defaults in file order, clears errors/buffers, unit-aware dirty; button state; popup | unit | same | Wave 0 |
| FIX-01 | One tooltip per shape tile | unit (headless hover) or source guard | `cargo test --lib -- overlay::style_lines skin::editor` | Wave 0 |
| (isolation) | no env mutation / pinned spawns | integration | `cargo test --test isolation_guard` | yes |

### Sampling Rate
- **Per task commit:** the quick filtered command for the touched module.
- **Per wave merge:** `cargo test --lib` plus `cargo test --test cli_generate --test cli_errors --test isolation_guard`.
- **Phase gate:** full `cargo test`, `cargo clippy`, and `cargo fmt --check`, green before `/gsd-verify-work`; manual UAT: `ff --screenshot` with temp `FERRISFILTER_HOME`/`USERPROFILE`/game folder for the Config tab (strictness row), Economy tab (Tiers button), and the two popups.

### Wave 0 Gaps
- [ ] `tests/fixtures/seed-styles.json` (byte copy of the old default) and repointed readers.
- [ ] New default-file tests in `defaults.rs` / `models/config.rs`.
- [ ] `src/overlay/strictness.rs` tests, `ConfigState` reset tests, `ff_levels` layout tests, FIX-01 hover test.

## Security Domain

`security_enforcement` is not set in config (treated as enabled); the phase adds no network, process, or parsing surface beyond the already-validated embedded JSON.

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | yes (small) | Embedded defaults parsed through the existing `StyleSet` / `ValidatedTiers` validators; tier text keeps `parse_cutoff` |
| V2/V3/V4/V6 | no | — |

| Threat | STRIDE | Mitigation |
|--------|--------|-----------|
| Reset or strictness writing a file nobody confirmed | Tampering | Both are in-memory pending edits; only the existing guarded all-or-none Apply writes |
| Tests overwriting the developer's live files | Tampering | `locations::from_env` panics under `cfg(test)`; `isolation_guard` source scan; temp homes |

## Sources

### Primary (HIGH confidence, read in this session)
- `src/overlay/config_state.rs`, `config_view.rs`, `app.rs`, `shell.rs`, `style_lines.rs`, `rule_list.rs`, `screenshot.rs`; `src/skin/editor.rs`, `widgets.rs`, `popup.rs`, `mod.rs`; `src/models/config.rs` (`StyleSet`, `Style`, `REQUIRED`), `src/models/tiers.rs`, `src/loader.rs` (`render_styles`), `src/defaults.rs`, `src/locations.rs`, `src/compile.rs`; `tests/isolation_guard.rs`, `tests/cli_generate.rs`, `tests/cli_errors.rs`; `config.json` (style refs); `Cargo.lock` (egui 0.36.1).
- Measurements (scratch copy of the worktree built in `%TEMP%`, since deleted): lib test run with the new default (1341 pass, 103 fail), Fontin SmallCaps caption widths.
- `.planning/phases/37-game-folder-resolver-picker/*-PLAN.md` for file overlap.

### Secondary / Tertiary
- None. No external documentation was needed; no web lookups.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (no new packages)
- Architecture: HIGH (every integration point read in code)
- Fixture swap scope: HIGH for lib tests (executed), MEDIUM for integration tests (static)
- FIX-01 root cause: HIGH in code, MEDIUM for the egui stacking behaviour and the hover-test harness
- Strictness layout: HIGH (measured widths), real content width MEDIUM-HIGH (derived; confirm by screenshot)

**Research date:** 2026-10-18
**Valid until:** until Phase 37 merges (line numbers in `config_view.rs`, `app.rs`, `shell.rs`, `tests/cli_*.rs` will move); the design findings do not expire.
