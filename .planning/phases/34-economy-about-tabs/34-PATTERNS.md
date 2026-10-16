# Phase 34: ECONOMY & ABOUT Tabs - Pattern Map

**Mapped:** 2026-10-16
**Files analyzed:** 14 (modified) + 2 new asset/art outputs
**Analogs found:** 14 / 14 (all in-repo; most are "extend the file's own existing code")

Key finding: `shell::economy_tab` and `shell::about_tab` ALREADY EXIST in `src/overlay/shell.rs` (a basic League / Last sync / Sync button page and a Credits-table About page). Phase 34 reshapes them to the UI-SPEC; it does not add new modules. A new view module is optional (see "No Analog / New Modules").

## File Classification

| File | Role | Data Flow | Closest Analog | Match |
|------|------|-----------|----------------|-------|
| `src/overlay/shell.rs` (`economy_tab`, `about_tab`, `last_sync_text`, `tab_markers`, `bar_state`) | view + pure decision fns | request-response (draw -> returned click) | itself (existing `economy_tab`/`about_tab`) + `src/bin/ff-proto.rs` `about_tab` | exact |
| `src/overlay/app.rs` (Economy arm ~2687, bar clicks ~2780, `refresh_last_sync` 1177, `stage_*_economy` 1809-1835, `dispatch_popup` 1750) | controller | event-driven (deferred actions) | itself | exact |
| `src/overlay/config_view.rs` (`show_sections` 295, `show_tiers` 452) | view | CRUD (emit `ConfigAction::TierText`) | itself; re-skin target `shell::economy_tab` rows | role-match |
| `src/overlay/config_state.rs` (`tier_rows`, `has_tier_errors`, private `tiers_dirty_count` 382) | model | CRUD | itself | exact |
| `src/overlay/rule_list.rs` (`economy_footer` 1393, `EconomyFooterAction` 226, `ECONOMY_ROW_HEIGHT`, header `x` at 647-653) | view | event-driven | itself (deletions) | exact |
| `src/overlay/economy.rs` | service (worker) | background thread + mpsc | unchanged (OUT of scope) | n/a |
| `src/skin/textures.rs` (+`ferris_about`) | config/asset loader | file-I/O (embedded) | `ferris_bg` field/`tex!` line | exact |
| `src/lib.rs` or `src/skin/mod.rs` (`REPO_URL` home) | config constant | n/a | `const REPO_URL` in `src/bin/ff-proto.rs:33` | exact |
| `tools/ff-art/src/main.rs` (`Cmd::Ferris`, `ferris()` 402) + `assets/poe2/manifest.toml` `[ferris]` | utility (CLI) | batch / transform (SVG -> PNG) | `ferris()` fn + `FerrisEntry` | exact |
| `src/overlay/screenshot.rs` | utility (debug) | request-response | itself (`--tab` already accepts economy/about; `POPUPS` has `remove-economy`) | exact |
| `src/bin/ff-proto.rs` (`economy_tab` 1038, `about_tab` 1064) | prototype view | request-response | itself | exact |
| `CLAUDE.md` `assets/ferris/` row | docs | n/a | existing row | exact |
| tests in `shell.rs` (`tab_markers`, `bar_state`, `last_sync_text` ~753, 1007-1180) | test | pure fn | existing tests there | exact |

## Pattern Assignments

### `src/overlay/shell.rs` - ECONOMY tab (view, request-response)

**Analog:** current `economy_tab` (lines 427-465). Keep the "draw returns what was clicked" shape; widen its signature (it is called once, `app.rs:2689`). Replace the `(&str,&str)` value pairs with typed rows so the tooltip / WARN colour fit.

Existing skeleton to extend (shell.rs 429-465):
```rust
pub(crate) fn economy_tab(ui: &mut egui::Ui, skin: &Skin, league: &str, last_sync: &str,
    syncing: bool, enabled: bool) -> bool {
    let mut clicked = false;
    w::ff_scroll_area(ui, skin, "economy", |ui| {
        w::ff_section(ui, skin, "Economy");
        for (i, (label, value)) in [("League", league), ("Last sync", last_sync)].into_iter().enumerate() {
            w::ff_row(ui, i, label, |ui| {
                ui.label(egui::RichText::new(value).font(fonts::regular(15.0)).color(pal::VALUE));
            });
        }
        ui.add_space(14.0);   // -> 16.0 per spec
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            let label = if syncing { "Syncing…" } else { "Sync economy" };
            if w::ff_button_ex(ui, skin, label, 180.0, enabled && !syncing, false).clicked() { clicked = true; }
        });
    });
    clicked
}
```
Rows: `ff_row(ui, idx, label, |ui| ...)` (`skin/widgets.rs:460`, index restarts at 0 per section; `row_background` by index). Tooltips: chain `.on_hover_text(..)` on the label response inside the closure. Right-aligned Add/Remove button inside a row: copy `rule_list::economy_footer` (rule_list.rs:1393-1430): `ui.allocate_exact_size(vec2(w, ECONOMY_ROW_HEIGHT), Sense::hover())`, `rect_filled(.., pal::ROW_EVEN)`, `ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(10.0,0.0))).layout(Layout::left_to_right(Align::Center)))`, then `ff_button_ex(&mut row, skin, "Add economy section", 180.0, enabled, false).on_hover_text(TIP)`. For the right-aligned 188pt button use `Layout::right_to_left` (pattern at rule_list.rs:640-646). Danger caption on Remove: last arg of `ff_button_ex` is `danger: bool` (widgets.rs:952-959).

Return type: change `bool` to a small `EconomyClicks { sync, add, remove }` (or `Option` enum) and let `app.rs` act on it, matching the file's "deferred action" rule (app.rs 2602-2603 comment).

**Relative-time fn:** put a pure `relative_sync_text(mtime_secs: Option<u64>, now_secs: u64, utc_offset_minutes: i32) -> String` next to `last_sync_text` (shell.rs:413) and test it like `last_sync_text_formats_local_time` (shell.rs:753-770). Fall back to the exact date via `crate::sync::prices::days_to_ymd(local / 86_400)` (already used at shell.rs:418). Keep `last_sync_text` for the hover tooltip. Clamp `now.saturating_sub(mtime)`.

**Priced items / Price source:** the types are `models::prices::PricesFile { league, fetched_at, items: Vec<PriceItem> }` (src/models/prices.rs:9) and `compile::PriceSource::{Live, Fallback(_), Absent}` (compile.rs:201). Loader: `loader::load_prices_optional(path)` (loader.rs:101). Read once in `refresh_last_sync` (app.rs:1177) - extend it into a `refresh_economy_info` that also stores `priced_items: usize` and `live: bool` on `App` (field `last_sync: String` is at app.rs:81, init 1083, called 1132, 2391, 2822). Thousands separator: no existing helper found; write a tiny pure fn + test.

**Repaint after 60 s:** `ui.ctx().request_repaint_after(std::time::Duration::from_secs(60))` inside the Economy arm.

### `src/overlay/shell.rs` - ABOUT tab (view, request-response)

**Analog 1:** current `shell::about_tab` (lines 468-555) - already has the pinned-footer geometry and divider art. **Analog 2:** `ff-proto.rs::about_tab` (1064-1148) for the hyperlink and the page layout.

Reusable pieces to keep verbatim:
```rust
// footer geometry + divider (shell.rs 518-533)
let foot = Rect::from_min_max(pos2(area.min.x, area.max.y - foot_h), area.max);
crate::skin::slice::paint_image(ui.painter(), &skin.title_underline,
    Rect::from_center_size(pos2(foot.center().x, foot.min.y), vec2(foot.width() * 0.8, 3.0)),
    egui::Color32::from_white_alpha(110));
let mut fui = ui.new_child(UiBuilder::new().max_rect(foot.shrink2(vec2(6.0, 12.0)))
    .layout(Layout::top_down(Align::Center)));
fui.spacing_mut().item_spacing.y = 4.0;
fui.add(egui::Label::new(egui::RichText::new(text).font(fonts::regular(15.0)).color(color))
    .halign(Align::Center).wrap());
```
Hyperlink (ff-proto.rs 1085-1090; `ui.hyperlink_to` takes the RichText + URL):
```rust
ui.hyperlink_to(
    egui::RichText::new("github.com/Jonathas-Conceicao/poe2-myfilter")
        .font(fonts::regular(16.0)).color(pal::GOLD),   // spec: regular(15.0)
    REPO_URL,
);
```
Add `.on_hover_text("Open the repository in your browser.")` on the returned response.

Changes vs. today: drop `ff_scroll_area` and the `Credits` `ff_section`/`ff_row` table (spec: non-scrolling, no rows); reorder to name (`small_caps(21.0)`, HEADING, already there) -> description (`regular(15)`, LABEL) -> Ferris image -> `Version {env!("CARGO_PKG_VERSION")}` (`regular(14)`, MUTED) -> link -> divider -> empty -> footer block. Footer becomes a max-480 centred column: disclaimer 1 (LABEL 15), disclaimer 2 (MUTED 15), 16pt gap, three credit lines (MUTED 14, 4pt apart). Compute `foot_h` from content or bottom-anchor with `Layout::bottom_up` instead of the fixed `80.0` (credits make it taller). The divider is drawn at the top of the footer region today; spec wants divider directly under the link (draw it in the flow, 16pt after the link) and the footer pinned separately.

**Ferris image:** draw with `slice::paint_image(painter, &skin.ferris_about, rect, Color32::WHITE)` (`skin/slice.rs:175`: `pub fn paint_image(painter: &Painter, tex: &TextureHandle, dest: Rect, tint: Color32)`), with `rect` from `ui.allocate_exact_size(vec2(112.0, 112.0 * h/w), Sense::hover())`. Aspect from `skin.ferris_about.size()` (see `ferris_bg.size()` use at `skin/widgets.rs:30`). Accessible name: `resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, "Ferris the crab"))` (pattern at rule_list.rs:607).

### `src/overlay/shell.rs` - `tab_markers` / `bar_state` (pure decision fns)

**Analog:** the functions themselves (lines 78-85, 336-399), both unit-tested at ~1007-1180.

Current signature `tab_markers(active, rules_dirty, config_dirty) -> [bool; 4]` marks index 2 (Economy) `false`. New: add `tiers_dirty: bool`; Economy marker = `active != Economy && tiers_dirty`; Config marker = `active != Config && config_dirty_without_tiers`. Caller (app.rs 2560, 2645) must split `config.is_dirty()` into styles-only and tiers-only: `tiers_dirty_count` is private (`config_state.rs:382`), so make it `pub` and add a `styles_dirty()` sibling next to `is_dirty` (config_state.rs:483). The tab tooltip is `apply::OTHER_TAB_DIRTY_TIP` (app.rs:2653); reuse unless the copy says "Config".

`bar_state` line 352 `ShellTab::Economy | ShellTab::About => false` for Revert: change to Economy => `(actions.revert || config.dirty-for-tiers/filter staging) && !needs_reload`; keep About `false`. The enabled/tooltip chain at 368-384 (`config.tier_errors` -> `config_state::APPLY_BLOCKED_TOOLTIP`) already handles tier gating; for Economy use `config.apply_enabled` like `ShellTab::Config` at line 369 so tier errors gate Apply from this tab. In app.rs the bar-click match (2780-2787) needs `ShellTab::Economy => self.config_ui.revert_armed = true` when only tiers are dirty, or `self.revert()` when only filter staging is dirty (do both when both).

### `src/overlay/app.rs` - wiring (controller, event-driven)

**Analog:** itself. Follow the "collect into locals, apply after render" pattern.

- Locals declared at 2604-2609 (`economy_action`, `sync_clicked`, `config_actions`): replace `economy_action: Option<rule_list::EconomyFooterAction>` with the new ECONOMY-tab click result; delete the `rules_content(.., economy_action)` parameter (2130-2136) and the footer block (2147-2162, 2237-2254) once `economy_footer` is removed.
- Economy arm (2687-2697): becomes
  `ShellTab::Economy => { clicks = shell::economy_tab(&mut cui, &skin, <info struct>, ...); tier_actions = <tiers editor via config_view>; }` - tiers `ConfigAction`s go into the existing `config_actions` vec so `handle_config_actions` (1921-2007, `ConfigAction::TierText` arm 2002) needs no change.
- Add/Remove handling next to the sync start (2803-2812): Add -> `self.stage_add_economy()` (1809, already guards root/working); Remove -> `self.list.confirm_economy_remove = true` (this flag drives `PopupFlags.remove_economy`, app.rs:1611, and `dispatch_popup` 1750-1755 calls `stage_remove_economy`). Keep that flag name; do not add a second popup path.
- Enabled gate: copy `can_edit_economy = self.root.is_ok() && self.changed_on_disk.is_empty()` (app.rs:2170) and add "not damaged / filter loaded" from `FilterState` (`state` computed at 2554; `FilterState::Damaged`, `HasSection`, `NoSection`, `Absent` at 1356-1362). Current section state: `working.economy()` -> `EconomyView::{Absent, Present{blocks}, Damaged(String)}` (rules.rs:286).
- Sync start (2803-2812) stays as is. Sync button enable flag today is `actions.sync` (apply.rs:156/242, `sync: root_ok`).
- Tab switch hook (2815-2824): the `ShellTab::Economy => self.refresh_last_sync()` arm is where to also refresh priced-items/source. Also re-run after a sync finishes (the call at app.rs:2391).
- Combined dirty (2556-2566, 1765-1773 `quit_dirty`, 1804): unchanged semantically; only the tab-marker split changes (see above).
- Screenshot hook: `Some("remove-economy") => self.list.confirm_economy_remove = true` (app.rs:1034) already exists - no change.

### `src/overlay/config_view.rs` + `config_state.rs` - TIERS editor move (view, CRUD)

**Analog:** `show_tiers` (config_view.rs:452-529) and `show_sections` (295-310).

Move step: delete lines 306-308 from `show_sections` (`section_title(ui, ctx, "Tiers"); show_tiers(..)`), make `show_tiers` `pub` (or add `pub fn show_tiers_section(ui, state, ctx) -> Vec<ConfigAction>` wrapping it) and call it from the ECONOMY arm inside the `ff_scroll_area("economy")` after the section toggle row. The `section_title(ui, ctx, "Tiers")` helper needs a `ConfigViewCtx` (skin is `Option<&Skin>`); in the economy tab call `w::ff_section(ui, skin, "Tiers")` directly. `show_load_error` (used in app.rs:2122) is the pattern when `self.config` is `Err`: show it in the Tiers section instead of the editor.

Re-skin note: `show_tiers` still uses legacy `theme::*` / `widgets::zebra_row` / `egui::TextEdit` (lines 460-509). UI-SPEC allows "moved unchanged in behaviour"; re-skin to spec lines/gaps only if cheap. Closest skinned analogs: `ff_field_row` (widgets.rs:513, `(ui, index, label, tip, add_control)`) for a TIER label + control, and `ff_text_field_id` (widgets.rs:1082) for the input with a stable id. Keep: `ConfigAction::TierText { tier, text }` emitted on `edit.changed()`, `row.error` hint, `row.edge` marker bar, `UNIT_TIP`. Spec asks `INVALID_RED` outline; the existing marker uses `theme::WARM_ACCENT_TEXT` (comment at 519-521) - confirm palette const before swapping.

`helper()` captions (453-456, 524-528) are standing paragraphs; spec ("Overlay UI not texty", no captions on ECONOMY) says remove them or move to tooltips. Model needs no change except exposing tier-only dirtiness (`tiers_dirty_count` private at config_state.rs:382).

### `src/overlay/rule_list.rs` - removals (view)

**Analog:** itself (deletion only).

- Remove the `x` on the expanded economy header: lines 647-653 (`if !collapsed && matches.is_none() && economy_edit { ... ff_x_button ... list.confirm_economy_remove = true; }`). Then the `economy_edit` parameter of that header fn (signature near line 600) and `can_edit_economy` threaded through `show_list` become unused; remove them and fix call sites in app.rs 2206, 2223 and the tests (rule_list.rs ~2228, 2352).
- Remove `economy_footer`, `economy_footer_text`, `ECONOMY_ROW_HEIGHT`, `ADD_ECONOMY_TIP`, `EconomyFooterAction` (rule_list.rs:226, 1377-1430) or keep `ECONOMY_ROW_HEIGHT`-style 34pt constants only if reused. `ListUi.confirm_economy_remove` (rule_list.rs:45, 120, 149) STAYS (the popup flag).
- Tooltip on the divider: extend `ECONOMY_TIP` (referenced at rule_list.rs:659) with `Add or remove it in the ECONOMY tab.`
- The `+ N locked rule blocks (economy)` summary text and the `Absent`/`Damaged` states: the Damaged-markers message previously came from the footer (rule_list 1423-1426); it still reaches the user through `shell::status_for` (filter_error built at app.rs:2737-2744), so nothing to port.
- Tests to delete/adjust: `economy_footer_text_strings` (2068), `economy_footer_draws_only_for_absent_and_damaged` (2228), and the Remove-x click scan (~2352).

### `src/skin/textures.rs` + `src/skin/widgets.rs` (asset loader)

**Analog:** `ferris_bg`.
```rust
// field (textures.rs:99-100)
/// The static background drawing (Ferris as pale line art, 2x resolution).
pub ferris_bg: TextureHandle,
// load (textures.rs:170)
ferris_bg: tex!(ctx, "ui/ferris-bg.png"),
```
Add `pub ferris_about: TextureHandle` and `ferris_about: tex!(ctx, "ui/ferris-about.png"),`. `tex!` does `include_bytes!(concat!("../../assets/", $path))` with mipmaps + LINEAR (`OPTIONS`, line 10), exactly what the spec wants. Test hook: `every_embedded_texture_decodes` (textures.rs:181) - add an assertion that `ferris_about.size()[0] == 336`. `load_light` (44) needs no change.

### `REPO_URL` shared constant

**Analog:** `src/bin/ff-proto.rs:33` `const REPO_URL: &str = "https://github.com/Jonathas-Conceicao/poe2-myfilter";`. Move to the library (e.g. `pub const REPO_URL` in `src/skin/mod.rs` or `src/overlay/mod.rs`) and `use ferrisfilter::...::REPO_URL` in ff-proto (ff-proto already imports `ferrisfilter::skin::slice::paint_image` this way, ff-proto.rs 1112). Link text constant can live next to it.

### `tools/ff-art/src/main.rs` + `manifest.toml` (CLI, batch transform)

**Analog:** `ferris()` (main.rs:402-485), `FerrisEntry` (131-148), manifest `[ferris]` (manifest.toml:251-259), `Cmd::Ferris` (46, 191), `Cmd::Extract` calls `ferris(&manifest, &root)` after `extract` (177), `save_png` (517) with `CompressionType::Best`.

Reusable front half (rendering, 406-417):
```rust
let svg = std::fs::read(root.join(&f.svg)).with_context(|| format!("reading {}", f.svg))?;
let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default())?;
let size = tree.size();
let scale = crab_w as f32 / size.width();
let crab_h = (size.height() * scale).round() as u32;
let mut pix = resvg::tiny_skia::Pixmap::new(crab_w, crab_h).context("pixmap")?;
resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
```
For the colour variant: add `ferris_about: Option<FerrisAboutEntry { svg, out, width }>` to `Manifest` (field `ferris: Option<FerrisEntry>` at line 81; new `[ferris_about]` table in the manifest, `width = 336`), a new fn `ferris_about()` that renders at `width`, skips the Sobel/thicken block, converts tiny-skia premultiplied RGBA to straight alpha (`pix.data()` is premultiplied - demultiply before building `RgbaImage`, or use `pix.take_demultiplied()` if available), tight-crops to the non-transparent bounding box (the `info` subcommand already computes a bbox; see `Cmd::Info` implementation for the loop), and `save_png`. Wire it in as `Cmd::Ferris` (render both) or a `FerrisColour` subcommand, and call it from `Cmd::Extract` after `ferris(..)`. Note the `Cmd::Ferris` doc says "Render only the Ferris background"; update it. Check output < 100 KB.

### `src/overlay/screenshot.rs` (debug utility)

**Analog:** itself. `--tab economy|about` already parses via `ShellTab::from_arg` (shell.rs:65; test at screenshot.rs:330) and `POPUPS` already includes `remove-economy` (line 24). Spec's extra shots (section present/absent, syncing, fallback prices, invalid tier) are produced by `--filter <path>` staging and temp-home contents, no new flags needed. If a "syncing" shot is wanted add a flag following the `--status` pattern (screenshot.rs 91, app.rs 2733-2736).

### `src/bin/ff-proto.rs` (prototype)

**Analog:** itself (`economy_tab` 1038-1062, `about_tab` 1064-1148). Update to the same layout/sizes as the real tabs (drop the 36pt name, 16/13.5 sizes; use `ferris_about`; add the Add/Remove and Priced items / Price source fake rows) and replace the local `REPO_URL` with the shared one. Purely cosmetic; lowest priority.

### Tests

Copy existing patterns: pure-fn tests in `shell.rs` (`last_sync_text_formats_local_time` 753; `bar_state`/`tab_markers` tests 1007-1180 with `ConfigBarInputs { .. }` literals) and `app.rs` flow tests (`stage_remove_economy` at 4655, popup dispatch at 4819: `app.dispatch_popup(&ctx, Popup::RemoveEconomy, 1)`). Existing tests that assert Economy is never marked (`shell.rs` ~1024 loop over `[Economy, About]`) must be updated for the new marker rule.

## Shared Patterns

### Deferred-action rule
**Source:** `src/overlay/app.rs` 2602-2609 and 2814-2848. Widgets return clicks / push `ConfigAction`s; app applies them after both panels render.

### Skin widgets only, sizes 14/15/17/21
**Source:** `src/skin/widgets.rs` (`ff_scroll_area` 1242, `ff_section` 412, `ff_row` 460, `ff_field_row` 513, `ff_button_ex` 952 with `danger` last arg, `ff_text_field_id` 1082) and `skin::fonts::{regular, small_caps}`. Colours via `crate::skin::palette as pal` (`HEADING, LABEL, VALUE, MUTED, GOLD, WARN, ERROR, ROW_EVEN/ODD/HOVER`).
```rust
use crate::skin::Skin;
use crate::skin::fonts;
use crate::skin::palette as pal;
use crate::skin::widgets::{self as w, StatusKind};
```

### Popups
**Source:** `shell.rs` `PopupData::RemoveEconomy` (649, 732-736) and `app.rs` `popup_flags` (1604-1615) / `dispatch_popup` (1750-1755). Reuse unchanged; open with `self.list.confirm_economy_remove = true`.

### In-memory staging, written by shared Apply
**Source:** `app.rs` `stage_add_economy` (1809-1824) / `stage_remove_economy` (1827-1835); both set `apply_state = ApplyState::Unapplied` and call `clear_events_on_edit()`. Add/Remove UI must call these, never write files.

### Status line carries results
**Source:** `shell::event_from` (212-238) + `status_for`; sync results already flow via `self.economy.state()` (app.rs 2720-2725). No inline result text on the tab.

### Disabled-with-tooltip
**Source:** `ff_button_ex(..., enabled, ...)` then `.on_hover_text(..)` on the response (rule_list.rs 1344-1349). Use `Fix the filter first.` for the damaged / no-filter case.

## No Analog / New Modules

| Item | Reason |
|------|--------|
| Thousands-separated integer formatter | none in repo; tiny pure fn in `shell.rs` with a unit test |
| Relative-time text | none exists (`last_sync_text` is absolute); new pure fn beside it |
| Demultiply + tight-crop colour render in ff-art | no colour SVG render path exists; start from `ferris()` render half and the `Info` bbox loop |
| Separate `economy_view.rs` / `about_view.rs` modules | optional; existing precedent keeps both tabs in `shell.rs`. If `shell.rs` (1328 lines) grows too much, split into `src/overlay/economy_view.rs` modelled on `config_view.rs` (pub fns taking `&mut Ui`, `&Skin`, state structs, returning actions), and register in `src/overlay/mod.rs` |

## Metadata

**Analog search scope:** `src/overlay/`, `src/skin/`, `src/bin/ff-proto.rs`, `src/compile.rs`, `src/models/`, `tools/ff-art/src/`, `assets/poe2/manifest.toml`
**Files scanned:** ~20 read (targeted ranges), ~15 grepped
**Pattern extraction date:** 2026-10-16
