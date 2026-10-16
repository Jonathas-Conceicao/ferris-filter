# Phase 30: Skinned Overlay Shell - Pattern Map

**Mapped:** 2026-10-16
**Files analyzed:** 17 (new/modified)
**Analogs found:** 16 / 17 (one partial: monitor-aware Win32 dock)

Sources: 30-CONTEXT.md, 30-UI-SPEC.md. Line numbers are against the working tree at mapping time.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/overlay/app.rs` (modify: eframe `ui()` becomes the shell composition) | component / app | event-driven + request-response | `src/bin/ff-proto.rs` `Proto::ui` (317-362) + own current `ui()` (1647-2314) | exact (composition) |
| `src/overlay/shell.rs` (new, optional: `header_art`, `status_for` priority fn, bottom bar, tab labels) | component / utility | transform | `ff-proto.rs::bottom_bar` (366-386) + `app.rs::outcome_lines` (1571-1601) | role-match |
| `src/overlay/mod.rs` (modify `launch()`: viewport + `skin::install`) | config / entry | request-response | itself (36-112) + `ff-proto.rs::main` (88-109) | exact |
| `src/overlay/win.rs` (modify `work_area` to per-monitor, add monitor/DPI fns) | utility (Win32) | request-response | `win::work_area` (166-187) + `win::find_game_window` (298-313) | role-match (new APIs) |
| `Cargo.toml` (modify windows-sys features) | config | n/a | line 52 | exact |
| `src/skin/window.rs` (generalise `primary_work_area_px`, keep `left_dock_rect` pure) | utility | transform | itself (12-82) | exact |
| `src/skin/widgets.rs::ff_button` (add `enabled`, `danger`) | component | request-response | `ff_button` (748-780) + `ff_arrow` disabled handling (784-813) | exact |
| `src/skin/widgets.rs::ff_tabs` (add `tips`) | component | request-response | `ff_tabs` (247-353) + `app.rs::tab_strip` tooltip (293-297) | exact |
| `src/skin/widgets.rs::apply_style` (extend restyle) | config | transform | `apply_style` (1012-1033) + `overlay/theme.rs::poe2_visuals` (58-75) | exact |
| `src/skin/popup.rs` (new: `ff_modal`) | component | event-driven | `app.rs` quit modal (2259-2308) + `ff_dropdown` popup frame (689-736) | role-match |
| `src/skin/icons.rs` (new: `paint_flag_icon`, `FlagIcon`) | component (painter) | transform | `ff_arrow` (784-813) + `skin/minimap.rs` + `ff_header` painter use (135-169) | partial |
| `src/skin/mod.rs` (add `pub mod icons; pub mod popup;`, fix module doc) | config | n/a | itself (24-41) | exact |
| `src/skin/fonts.rs` / `src/skin/widgets.rs` (16 -> 15 size change) | utility | transform | `regular(16.0)` sites at 675, 717, 827 | exact |
| `src/overlay/theme.rs` (re-point constants) | config | transform | itself (18-49, 162) | exact |
| `src/overlay/rule_list.rs` (flags to icons, search row, economy footer, popup flags) | component | event-driven | itself: `flags_for` (259-327), `fit_flags` (242-257), `economy_footer` (784-857), delete confirm (719-742) | exact |
| `src/overlay/config_view.rs` (section headers, captions to tooltips, confirms to popups) | component | event-driven | itself: delete frame (1105-1136), `RELOAD_TOGGLE_CAPTION` (44, 428), `helper` (268) | exact |
| `src/overlay/apply.rs` (message strings to mixed case; tooltip rename) | utility | transform | itself: consts 193-202, 402-406, 460-469 | exact |
| ECONOMY / ABOUT placeholder tabs (in shell.rs or app.rs) | component | request-response | `ff-proto.rs::economy_tab` (1038-1062), `about_tab` (1064-1147) | exact |
| `CLAUDE.md`, `CHANGELOG.md`, verification checklist | docs | n/a | existing overlay notes in CLAUDE.md | exact |
| debug-only `--screenshot` path for `ff` | utility | event-driven | `ff-proto.rs::screenshot` (276-307) | exact |

## Pattern Assignments

### Shell composition in `src/overlay/app.rs` `ui()` (component, layered paint)

**Analog:** `src/bin/ff-proto.rs` `Proto::ui` (317-362). Copy the seven-layer order verbatim; UI-SPEC "Layers" matches it.

```rust
let win = ui.max_rect();
let skin = self.skin.clone();
let painter = ui.painter().clone();
// 1. Static background: never moves with the content.
w::ff_background(&painter, &skin, win);
// 2. Frame below the header; the header overlaps the frame's top strip.
let header_h = w::header_height(&skin, win.width());
let frame_outer = Rect::from_min_max(pos2(win.min.x, win.min.y + header_h - 14.0), win.max);
let frame = w::ff_frame(&painter, &skin, frame_outer);
// 3. Header art, title and close button.
let header = Rect::from_min_size(win.min, vec2(win.width(), header_h));
if w::ff_header(ui, &skin, header, Some(&skin.title_bar), "FerrisFilter").clicked() { /* proto: Close */ }
// 4. Tabs
w::ff_tabs(ui, &skin, w::tab_strip(&frame), &TABS, &mut self.tab);
// 5. Content, clipped to the area inside the gold line.
let content = frame.content.shrink2(vec2(10.0, 8.0));
let mut cui = ui.new_child(UiBuilder::new().max_rect(content).layout(Layout::top_down(Align::Min)));
cui.set_clip_rect(frame.content);
// 6. Bottom bar
self.bottom_bar(ui, &skin, frame.button_bar);
```

Adaptations required (the overlay differs from the proto):
- Replace the proto's `ViewportCommand::Close` on header close with the overlay's hide, copied from the current close handler (`app.rs` 1926-1932): `if header.close_clicked && self.visibility != Visibility::Hidden { if let Some(window) = &self.window { win::conceal(window); } self.visibility = Visibility::Hidden; }`. Never send Close here; `app.rs` 2310-2313 ("the only Close send site") stays the only one.
- Replace `egui::CentralPanel::default().frame(panel_frame).show(ui, ...)` (1920-1922) with the layered painting above; keep everything before line 1856 (hotkey, tray, instance poll, `visibility::advance`, conceal message clearing, reload poll, `request_repaint_after(50ms)`) untouched. Those are the "behaves exactly as v0.5" guarantees.
- Keep the deferred-action rule: `row_actions`, `economy_action`, `jump_to`, `config_actions`, `picked_tab` are collected during render and applied after (declared 1912-1918, applied 2225-2257). The new status line click (grammar-gate / unknown-style jump) feeds the existing `jump_to` (applied 2227-2233: clears query if row hidden, sets `list.expanded` and `list.pending_scroll`).
- Tab labels: keep `apply::tab_label(name, marker)` (apply.rs:460) and markers from the existing call (1934-1939): RULES marker = `self.tab == Tab::Config && rules_dirty`, CONFIG marker = `self.tab == Tab::Rules && config_dirty`. The UI-SPEC says the marker is "other tab dirty" regardless of which tab is selected; the existing code only shows it on the non-selected tab's label via `self.tab ==` guards. Preserve the existing guard semantics (it is the v0.5 behaviour) and add ECONOMY/ABOUT with no marker. Existing `Tab` enum is at app.rs:196 and needs two more variants (or a separate shell tab index).
- `picked_tab` handling: existing code path (disarm confirms, refresh filter list when switching to CONFIG) stays; `ff_tabs` returns a `Response` with `changed()` so map `*selected` change to the same path.

**Pinned-footer deletion:** the old `egui::Panel::bottom("ff-footer")` block (1950-2100+) with indicator line, `+ ADD RULE` / RELOAD / OVERWRITE / SYNC buttons and `outcome_area` is removed; its enablement logic is the source for the bottom bar:
- `actions` (`apply::footer_actions_combined`, app.rs 1891-1898) and `config_footer.apply_enabled` (1899-1910) drive Revert/Reload/Apply enable.
- OVERWRITE call: `let egui_ctx = ui.ctx().clone(); self.run_apply(&egui_ctx, true);` (app.rs 2010-2011).
- RELOAD call: `self.reload_pressed();` (2000; defined 1265).

---

### Bottom bar and status line (component, request-response)

**Analog:** `ff-proto.rs::bottom_bar` (366-386)

```rust
let inner = bar.shrink2(vec2(10.0, 0.0));
let mut bui = ui.new_child(UiBuilder::new().max_rect(inner).layout(Layout::right_to_left(Align::Center)));
bui.spacing_mut().item_spacing.x = 6.0;            // UI-SPEC snaps to 8.0
if w::ff_button(&mut bui, skin, "Apply", 84.0).clicked() { ... }
if w::ff_button(&mut bui, skin, "Reload", 84.0).clicked() { ... }
if w::ff_button(&mut bui, skin, "Revert", 84.0).clicked() { ... }
let status_rect = Rect::from_min_max(inner.min, pos2(bui.min_rect().min.x - 10.0, inner.max.y));
w::ff_status(ui, status_rect, &self.status.text, self.status.kind);
```

Status widget already exists: `skin::widgets::ff_status` (898-923), `StatusKind` + `color()` (857-878), `status_job` (882-894; single row, `overflow_character: Some('…')`, hover tooltip only when `galley.elided`). Reuse as is. Extension needed: click-to-jump with `CursorIcon::PointingHand` (pattern from `app.rs::outcome_area` 1629-1638):

```rust
if line.jump.is_some() { label = label.sense(egui::Sense::click()); }
let resp = ui.add(label);
if let Some(target) = line.jump && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { jump = Some(target); }
```
`ff_status` currently uses `Sense::hover()` (899); make the sense depend on a `jump` flag or return the response and let the caller decide, since the proto call site discards the response.

**Status priority function (pure, table-driven test):** replace `outcome_lines` (app.rs 1571-1601; `OutcomeLine { text, accent, jump }`, tests at `outcome_lines_order_and_jump_targets` 2398) with a pure `fn status_for(...) -> (String, StatusKind, Option<BlockId>)` over the 9-row table. Copy its signature style: inputs are already-computed options, no egui types:

```rust
pub(crate) fn outcome_lines(
    list_status: Option<String>,
    apply_message: Option<(String, bool)>,   // (text, accent)
    sync_status: Option<(String, bool)>,
    grammar_gate: Option<(String, BlockId)>,
    unknown_summary: Option<(String, BlockId)>,
) -> Vec<OutcomeLine>
```
Source strings to reuse unchanged (test-pinned): `apply::invalid_blocks_message` (205), `apply::DAMAGED_MARKERS_MESSAGE` (256), `apply::unknown_styles_message` (305), `apply::indicator` (283), `apply::filter_state_message` (260), `apply::REVERTED_MESSAGE` (199). Retire `OUTCOME_MAX_HEIGHT` (1567) and the tests `outcome_area_is_capped_and_keeps_apply_on_screen` (2422), `the_apply_button_stays_on_screen_however_long_the_rule_list_is` (2381), `the_config_footer_keeps_apply_on_screen_with_a_long_styles_list` (2461), `a_511_block_filter_keeps_the_footer_on_screen...` (2756); replace with a shell layout test that the bar buttons stay inside `frame.button_bar`.

**Message lifetime:** keep v0.5 behaviour of `apply_message = None` on edit (app.rs 2245) and the conceal rule (1821-1830: `apply_message_after_conceal` keeps only an in-flight reload line; `economy.clear_message()`). The status line reads those same fields.

---

### `ff_button(enabled, danger)` (component) in `src/skin/widgets.rs`

**Analog:** `ff_button` (748-780) for art/caption; disabled pattern from `ff_arrow` (784-813):

```rust
let sense = if enabled { Sense::click() } else { Sense::hover() };
...
let tint = if enabled { Color32::WHITE } else { Color32::from_white_alpha(70) };
```
and the pressed/hover state selection already in `ff_button`:
```rust
let state = if resp.is_pointer_button_down_on() { 2 } else if resp.hovered() || resp.has_focus() { 1 } else { 0 };
let art = &skin.button[state];
slice::paint_three(painter, [&art.left, &art.middle, &art.right], slice::three_h_rects(rect, art.left.size(), art.right.size(), painter.pixels_per_point()), Color32::WHITE);
```
Per UI-SPEC: disabled tint `from_white_alpha(90)`, caption `pal::DISABLED`, `danger` caption `pal::ERROR` (the existing caption uses `pal::BUTTON_TEXT` at 752/778). `widget_info` already passes `true` for enabled (755): change to the `enabled` param. Keep `ff_button(ui, skin, text, min_width)` as a thin wrapper calling the new function because `ff-proto` calls the 4-arg form (many sites, e.g. 374-380, 1057). Existing in-file tests (1036+) use `setup()` (1062) / `click_at` (1040): copy those for a disabled-button-never-clicks test.

### `ff_tabs(tips)` (component) in `src/skin/widgets.rs`

**Analog:** `ff_tabs` (247-353). The per-tab response is created at 288 (`let resp = ui.interact(hit, ui.id().with(("ff_tab", i)), Sense::click());`) and stored in `resps`; attach `on_hover_text` there. Existing tooltip pattern to port, `app.rs::tab_strip` (293-297):

```rust
let resp = if marker { resp.on_hover_text(apply::OTHER_TAB_DIRTY_TIP) } else { resp };
```
Keyboard Left/Right (kept per UI-SPEC) pattern at `tab_strip` 304-316 uses `resp.has_focus()` + `ui.input(|i| i.key_pressed(egui::Key::ArrowLeft))`; `ff_tabs` has no focus handling yet (it imports `EventFilter`/`Key` at line 12-13, check use before adding). Keep a 5-arg wrapper for ff-proto (343, test at 1123). Labels are `&[&str]`; the dynamic ` *` label means passing owned strings (`Vec<String>` -> `Vec<&str>`).

---

### `apply_style` extension (config) in `src/skin/widgets.rs`

**Analog:** `apply_style` (1012-1033, already sets visuals, selection, widget strokes, `ScrollStyle::thin()`, `bar_width = 6`, `item_spacing = (8, 2)`). Extend in the same `ctx.global_style_mut(|style| { ... })` closure with `text_styles`, `faint_bg_color`, `hyperlink_color`, `warn_fg_color`, `error_fg_color`, tooltip/popup stroke, `button_padding`, `interact_size.y`. UI-SPEC says `item_spacing` (8, 4) but the current value is (8, 2): ff-proto will shift; verify the proto screenshot after. `ctx.global_style_mut` (not `style_mut`) is the egui API in use; `egui::Visuals::dark()` is the base.

`skin::install` (`skin/mod.rs` 37-41) is the one-call replacement for `theme::install_fonts` + `set_visuals(theme::poe2_visuals())`:

```rust
pub fn install(ctx: &egui::Context) -> Skin {
    fonts::install(ctx);
    widgets::apply_style(ctx);
    Skin::load(ctx)
}
```
Call it in `overlay::launch()` where `mod.rs` 98-102 now calls `theme::install_fonts(&cc.egui_ctx); cc.egui_ctx.set_visuals(theme::poe2_visuals());`, and pass the returned `Skin` into `OverlayApp::new(...)` (new param or field; `new` is at app.rs:824). Do not call `theme::install_fonts` too: `ctx.set_fonts` would overwrite the skin fonts (Cinzel is already registered by `skin::fonts::install`, constant `fonts::CINZEL`).

Size change 16 -> 15: hard-coded `regular(16.0)` at widgets.rs 675 (dropdown value), 717 (dropdown option), 827 (text field); also the slider value site (grep `regular(16.0)` again when editing). ff-proto has its own `fonts::regular(16.0)` at 1050/1088/1103 (cosmetic).

### `overlay::theme` constants re-point (config)

**Analog:** `theme.rs` constants 18-49 and `ROW_SUMMARY_SIZE` 162. Value-only edits per the UI-SPEC table, example:

```rust
pub const LABEL_TAN: Color32 = Color32::from_rgb(0xE3, 0xCD, 0x97);   // becomes pal::LABEL
```
Use `use crate::skin::palette as pal;` and `pub const LABEL_TAN: Color32 = pal::LABEL;` (palette values are `const Color32`; confirm in `skin/palette.rs` 61 lines). The unit tests at theme.rs 177-207 pin the old hex values (`palette_constants_match_the_d24_hex_table`, `poe2_visuals_*`, `the_row_summary_stays_readable_and_subordinate` asserts `ROW_SUMMARY_SIZE < 14.0`): they must be updated or deleted in the same change (ROW_SUMMARY_SIZE becomes 15). `poe2_visuals`/`install_fonts`/`section_header`/`row_label` stay until Phase 35 but must stop being installed; `row_label` (129-142) is still used by `tab_strip` (275) which is deleted.

---

### `skin::popup::ff_modal` (component, event-driven) — new `src/skin/popup.rs`

**Analogs:**
1. Current quit modal, `app.rs` 2259-2308 — the dispatch contract to keep (choice enum, Escape/outside-click = Cancel, deferred quit):

```rust
let modal = egui::Modal::new(egui::Id::new("quit_confirm")).show(ui.ctx(), |ui| {
    ui.heading(apply::QUIT_CONFIRM_TITLE);
    ui.horizontal(|ui| {
        if ui.button(apply::QUIT_APPLY_LABEL).clicked() { choice = Some(apply::QuitChoice::ApplyAndQuit); }
        if ui.button(apply::QUIT_DISCARD_LABEL).on_hover_text(apply::QUIT_DISCARD_TIP).clicked() { choice = Some(apply::QuitChoice::QuitWithoutApplying); }
        if ui.button(apply::QUIT_CANCEL_LABEL).clicked() { choice = Some(apply::QuitChoice::Cancel); }
    });
});
// Escape or a click outside counts as Cancel.
if choice.is_none() && modal.should_close() { choice = Some(apply::QuitChoice::Cancel); }
if let Some(choice) = choice {
    let dirty = self.quit_dirty();
    match apply::quit_step(dirty, true, apply::QuitEvent::Choice(choice)) { ... }
}
```
Keep the `match` body (2284-2307) untouched; only the widget block above it changes to `ff_modal(...) -> Option<usize>` mapped to `QuitChoice`.
2. The game box art as a nine-slice, `ff_dropdown` (724-735) — copy exactly for the modal box background, with a `Shape::Noop` placeholder painted first then `painter().set(bg, Shape::mesh(mesh))`:

```rust
let bg = ui.painter().add(Shape::Noop);
...
let frame_rect = ui.min_rect().expand(6.0);
let mut mesh = egui::Mesh::with_texture(skin.box_9slice.id());
for p in slice::nine_slice(frame_rect, skin.box_9slice.size(), Margins::new(8.0, 8.0, 8.0, 8.0), 1.0, ui.painter().pixels_per_point()) {
    mesh.add_rect_with_uv(p.dest, p.uv, Color32::WHITE);
}
ui.painter().set(bg, Shape::mesh(mesh));
```
(UI-SPEC says inner padding 24, so expand by 24 with `Frame::NONE.inner_margin(24)`; `ff_dropdown` uses `.frame(egui::Frame::NONE.inner_margin(6))`, line 694.)
3. Title with underline: `ff_section` (359-385), reuse its rule painting (`skin.title_underline` at 150 alpha); body text via `fonts::regular(15.0)` + `pal::VALUE`; buttons via `ff_button`.

`egui::Modal` provides the dim backdrop, swallow-input and `should_close()` (Escape / click outside); set the backdrop colour to `Color32::from_black_alpha(160)` via `Modal::backdrop_color` if available in the pinned egui, otherwise paint on an `Area` with `Order::Foreground`. Add the Escape key check via `ui.input(|i| i.key_pressed(egui::Key::Escape))` if `should_close` is not enough. Stacking priority (Quit > Changed-on-disk > rest) belongs in an app-level `fn active_popup(&self) -> Option<Popup>` (pure, unit-testable) deciding from the existing flags.

**State flags reused as "popup open" (no new state):**
- `quit_confirm: bool` (app.rs:91)
- `changed_on_disk: Vec<DiskFile>` (116; set in `run_apply` at 1080-1082, cleared 984/1153; `reload_pressed` 1265, `reload_changed` 1275)
- `list.confirm_delete: Option<BlockId>` (rule_list.rs:31; reset 122, 351-352, 573, 620) and `list.confirm_economy_remove: bool` (36, 125)
- `cfg.delete_armed: Option<String>` / `cfg.revert_armed: bool` (config_view.rs 98-99, `arm_delete` 122, reset 117/127-128)
- `ctx.pending_switch: Option<&str>` (config_view.rs:147, used at 384)

**Delete confirm conversion (rule_list.rs 719-742):** the inline `if list.confirm_delete == Some(id) { label + "Yes, delete" + "Keep" } else { Duplicate / Delete }` collapses to: Delete button sets `list.confirm_delete = Some(id)`; the modal (drawn once per frame from the app, not per row) pushes `RowAction::Delete(id)` and clears the flag on choice 0. Same for economy footer (800-835) -> `EconomyFooterAction::Remove`, and CONFIG delete style (config_view.rs 1105-1128) -> `ConfigAction::DeleteStyle(name.to_string())`. The modal's body text for style deletion is `config_view::delete_prompt(name, ctx)` (228) and for revert `revert_prompt(n)` (239).

---

### Flag icons — new `src/skin/icons.rs` (component, painter) and `rule_list.rs`

**Analog for painted shapes:** no icon module exists. Closest: `ff_header` placeholder plate (139-152) using `painter.rect(plate, 2.0, fill, Stroke::new(1.0, c), StrokeKind::Inside)`, `ff_arrow` (784-813) for "allocate a small square, `widget_info`, paint", and `skin/minimap.rs` for icon-shape lookups. Use `egui::Shape::{circle_stroke, line, convex_polygon, Path}` with `Stroke::new(1.5, colour)`; see `Shape::rect_stroke(rect, CornerRadius::same(1), Stroke::new(..), StrokeKind::Inside)` used in `input_box` / `ff_text_field` (832-847). Hit-rect pattern with accessible name and tooltip:

```rust
let (rect, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
resp.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, name));
paint_flag_icon(ui.painter(), rect.shrink(2.0), icon, colour);
resp.on_hover_text(tip)
```

**`flags_for` (rule_list.rs 259-327) stays the single source of truth** and returns the same flag set (test requirement). Change `struct Flag { text: String, color: Color32, tip: String }` (217-221) to carry a `FlagIcon` kind (`Padlock`, `Continue`(word), `Styled(String)`, `UnknownStyle`, `GrammarError`, `GrammarWarning`, `GroupBroken`, `MovedAcrossContinue`). Keep the early-return for `can't enable` (261-268) and `diag_tip` (223-229) as is. Shorten `CANNOT_ENABLE_LONG`/`GROUP_BROKEN_LONG` tips at the flag site only (consts at lines 24-25 are still used for the toggle tooltip and 699). Keep `STYLE_FLAG_CHARS` truncation, but UI-SPEC wants 10 chars (current 12, line 232).

`fit_flags(widths, budget) -> (shown, hidden)` (242-257) is unchanged; feed it 18.0-per-icon widths (the "styled" and "Continue" flags are wider: pass measured widths). Existing test: `fit_flags_collapses_overflow_into_a_badge` (985). Budget math at line 495: `FLAG_BUDGET.min((inner.width() - arrows_w - 24.0 - 8.0 - LABEL_MIN_W).max(0.0))`. The spec orders icons right to left (errors closest to arrows), whereas `flags_for` pushes Continue/styled first; reverse for painting or reorder pushes without changing the returned set.

---

### Caption -> tooltip conversions (config_view.rs / rule_list.rs / app.rs / body.rs)

**Analog:** the existing tooltip idiom `ui.label(..).on_hover_text(..)` (rule_list.rs 815-818, economy footer label) and `resp.on_hover_text(apply::OTHER_TAB_DIRTY_TIP)`. Concrete edits per UI-SPEC section A:
- #2: config_view.rs 428 `helper(ui, RELOAD_TOGGLE_CAPTION);` removed; attach `RELOAD_TOGGLE_CAPTION + "\n\n" + RELOAD_TOGGLE_TIP` to the toggle response. Unit tests asserting these consts (config_view.rs 1516-1523) keep passing since the consts are unchanged.
- #1/#3/#4/#5: `helper(ui, text)` (268) callers become `on_hover_text` on the control's response.
- #6/#7: `centered_notice(ui, heading, body)` (app.rs 680-689; call sites 2174-2199) -> heading only; paragraph becomes the tooltip on `+ Add rule`.
- #9/#10: `body::NO_CONDITIONS`/`NO_ACTIONS` consts (body.rs) rendered as short `No conditions`/`No actions` with the long text as the tooltip.
- #11: header title plate tooltip = filter path. `ff_header` paints text via the painter, not a widget, so add a `ui.interact(plate_rect, id, Sense::hover())` response over the plate or have `ff_header` accept an optional tooltip.
- `rule_count_text` (app.rs 670) and its test (`rule_count_copy` 2746) go away; hint text becomes `format!("Search {N} rules, classes, base types…")`. `rule_list::search_bar` (746-771) is replaced by `ff_text_field` (816-850) + `ff_button("+ Add rule", 124)` mirroring `ff-proto.rs::rules_tab` (392-400: `ui.horizontal` with `item_spacing.x = 8`, `field_w = ui.available_width() - 132.0`). Keep the Escape-to-clear and the Clear button logic (755-768) when swapping.
- `apply::APPLY_BLOCKED_TOOLTIP` (apply.rs:202, currently `Fix or disable the rules marked "grammar error" first.`) -> `Fix or disable the rules marked with the red ! first.` (update the test that pins it).
- Mixed-case retirements: strings containing `APPLY`/`RELOAD`/`SYNC ECONOMY` in apply.rs / config_view.rs / app.rs (grep `press APPLY`, `Press + ADD RULE`, `Fix the file and press RELOAD`) plus their tests.

---

### Monitor-aware dock (utility, request-response) — `win.rs` + `skin/window.rs`

**Current (to replace):** `app.rs` 1648-1666 (`!self.docked` once, `win::work_area()` primary-only + logical/physical mix via `layout::dock_rect_in(area, layout::STRIPE_WIDTH)`) and `win::work_area` (win.rs 166-187):

```rust
let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut rect as *mut RECT).cast(), 0) };
if ok == 0 { return None; }
let area = egui::Rect::from_min_max(egui::pos2(rect.left as f32, rect.top as f32), egui::pos2(rect.right as f32, rect.bottom as f32));
(area.width() > 1.0 && area.height() > 1.0).then_some(area)
```

**Pure geometry to keep and extend:** `skin::window::left_dock_rect(work_area_px, pixels_per_point, width)` (window.rs 16-26) with tests at 64-81. Add tests for a second-monitor origin (e.g. work area `x = 1920..3840`, and negative `x = -1920..0`) at 1.0/1.5/2.0. The UI-SPEC wants `SetWindowPos` in physical pixels with scale from `GetDpiForMonitor`, so add a second pure fn (e.g. `dock_rect_px(work_px, scale, width_pt) -> RectPx`, physical width = `round(660 * scale)`) beside `left_dock_rect` and unit-test it; leave `left_dock_rect` for the ff-proto fallback.

**Win32 pattern to copy for the new calls:** the SAFETY-comment style and null/zero checks in `win.rs` (`work_area` 166-187, `reveal`/`conceal` 143-158 which use the cached `OverlayWindow(HWND)`), HWND newtype with `unsafe impl Send` (121-129 and `GameWindow` 225-229), and `find_game_window()` (298-313) returning `Option<GameWindow>` to feed `MonitorFromWindow(game.0, MONITOR_DEFAULTTONEAREST)`. Non-Windows stubs live in the `#[cfg(not(windows))]` module (563-650: `work_area() -> None` at 573); add stubs for every new pub fn there. Smoke-test style: `the_work_area_is_a_plausible_desktop` (win.rs 715-736) returns early on `None`; copy it for the new monitor fn.

**Cargo.toml gap (important):** `windows-sys` features at Cargo.toml:52 are `Win32_Foundation, Win32_Security, Win32_System_Threading, Win32_UI_WindowsAndMessaging, Win32_UI_Input_KeyboardAndMouse, Win32_System_DataExchange, Win32_System_Memory`. `MonitorFromWindow` / `MonitorFromPoint` / `GetMonitorInfoW` / `MONITORINFO` need `Win32_Graphics_Gdi`; `GetDpiForMonitor` / `MDT_EFFECTIVE_DPI` need `Win32_UI_HiDpi`. Add both. (`skin/window.rs` module doc claims "no Win32 feature beyond the ones the crate already enables" — update that comment.) `SetWindowPos`, `SWP_*`, `HWND_TOPMOST` are in `Win32_UI_WindowsAndMessaging` already.

**Re-dock on every summon:** the reveal site is `visibility::effect_for(self.visibility, next)` -> `Effect::Reveal` handled at app.rs 1799-1806:

```rust
if let Some(effect) = visibility::effect_for(self.visibility, next) {
    if let Some(window) = &self.window {
        match effect {
            visibility::Effect::Reveal => win::reveal(window),   // dock here, before reveal
            visibility::Effect::Conceal => win::conceal(window),
        }
    }
}
```
Put `win::dock_on_game_monitor(window, WIDTH)` immediately before `win::reveal(window)` so first open, hotkey, tray toggle/Show, second-launch (`take_show_request` 1785-1791) all go through it. The close button path (1926-1932) calls `win::conceal` directly and needs no dock. `visibility.rs` itself stays pure (no changes; its tests 117-199 pin the state machine). The first-frame dock replaces `!self.docked` block; ff-proto's `dock()` (255-274) shows the wait-for-`native_pixels_per_point` retry (`let Some(ppp) = ppp else { return; }`) — the physical-pixel `SetWindowPos` approach avoids needing it, but egui still needs a frame to pick up the new scale.

**Viewport builder** (`overlay/mod.rs` 80-87) keeps `.with_always_on_top().with_decorations(false).with_resizable(false).with_taskbar(false).with_active(false)`; change `with_inner_size([layout::STRIPE_WIDTH, 1000.0])` to `[660.0, ...]`; do not add transparency / click-through (comment 73-79). `ff-proto` uses `.with_visible(false)` then shows after docking (88-96), but the overlay must keep its current show-without-activate flow (`SW_SHOWNOACTIVATE`, `win::reveal` doc 141-142 says egui's own Visible command is not safe across repeat summons). `layout.rs` (`STRIPE_WIDTH` 460, `dock_rect`, `dock_rect_in`) becomes dead except tests; retire in Phase 35 or delete now with its `work_area_tests`.

---

### ECONOMY / ABOUT placeholders (component, request-response)

**Analog:** `ff-proto.rs::economy_tab` (1038-1062) and `about_tab` (1064-1147), copy and trim per UI-SPEC.

```rust
w::ff_scroll_area(ui, skin, "economy", |ui| {
    w::ff_section(ui, skin, "Economy");
    for (i, (label, value)) in rows.iter().enumerate() {
        w::ff_row(ui, i, label, |ui| { ui.label(egui::RichText::new(*value).font(fonts::regular(16.0)).color(pal::VALUE)); });
    }
    ui.add_space(14.0);
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        if w::ff_button(ui, skin, "Sync economy", 180.0).clicked() { ... }
    });
});
```
Real data: `League` -> `sync::OVERLAY_LEAGUE`; `Last sync` -> `prices.json` mtime under the app root (reuse root resolution from `self.root`); drop the proto's `Priced items` row and REPO_URL hyperlink (about_tab 1085-1090). Sync trigger: the existing SYNC button code in the deleted footer (`app.rs` ~2021-2060: `let syncing = self.economy.is_running(); label "SYNCING…" else "SYNC ECONOMY"; add_enabled(...)`, then the call into `self.economy`); read lines 2021-2075 when planning to lift the exact start call and the `footer_actions.sync` gate. Sync result text comes from `self.economy` (`economy.rs`, `poll()` at app.rs 1716, `clear_message()` at 1829) and goes to the status line (priority 2/5). ABOUT pinned disclaimer at the bottom: copy `about_tab` 1109-1147 (`foot = Rect::from_min_max(pos2(area.min.x, area.max.y - 80.0), area.max)`, `slice::paint_image(painter, &skin.title_underline, ...)`, a `new_child` with `Layout::top_down(Align::Center)`, `egui::Label::new(...).halign(Align::Center).wrap()`); size 15 per the spec (proto uses 13.5, title 36 vs spec 21).

CONFIG scroll wrap: `ff_scroll_area(ui, &skin, "config", |ui| ...)` around the existing `config_body` (app.rs 1467) minus its footer; `ff_section(ui, &skin, "Filter"/"Styles"/"Tiers")` replaces `theme::section_header` call sites in config_view.rs (the `LayoutJob` fn at theme.rs 111-124). RULES keeps `show_list`'s own `ScrollArea` (virtual_list.rs).

---

### Debug-only screenshot path for `ff` (utility, event-driven)

**Analog:** `ff-proto.rs`: `Args.screenshot` (47-52, `--screenshot` parse at 64), `Proto { docked, frames, shot_requested }` (171-178), and `screenshot()` (276-307):

```rust
fn screenshot(&mut self, ui: &mut egui::Ui) {
    let Some(path) = self.args.screenshot.clone() else { return; };
    ui.ctx().request_repaint();
    let image = ui.ctx().input(|i| i.raw.events.iter().find_map(|e| match e {
        egui::Event::Screenshot { image, .. } => Some(image.clone()), _ => None }));
    if let Some(image) = image {
        let [w, h] = image.size;
        let mut buf = Vec::with_capacity(w * h * 4);
        for c in &image.pixels { buf.extend_from_slice(&c.to_srgba_unmultiplied()); }
        match image::RgbaImage::from_raw(w as u32, h as u32, buf).map(|img| img.save(&path)) {
            Some(Ok(())) => eprintln!("saved {}", path.display()),
            Some(Err(e)) => eprintln!("could not save {}: {e}", path.display()),
            None => eprintln!("screenshot had an unexpected size"),
        }
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    } else if self.docked && self.frames >= 20 && !self.shot_requested {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        self.shot_requested = true;
    } else if self.frames > 600 { eprintln!("no screenshot arrived; exiting"); ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close); }
}
```
Notes for the `ff` port: (1) gate with `#[cfg(debug_assertions)]` so release has no extra code or flag (matches CONTEXT "debug-only"); (2) the overlay's only legal `ViewportCommand::Close` is at app.rs 2312 ("the only Close send site", asserted by a comment; check any grep-based test before adding another), so route the screenshot exit through the existing `quit_requested`/`begin_shutdown` path or the same site; (3) the stripe is hidden-until-revealed and hides on focus loss, so the screenshot mode must force `Visibility::ShownFocused`/skip `visibility::advance` hide and must not run against the user's live `dynamic.filter` (use `FERRISFILTER_HOME` + `USERPROFILE` temp copy per the CLAUDE.md development warning); (4) the `image` crate is already a dependency of the binary target (ff-proto uses `image::RgbaImage`) — confirm it is in the library's `[dependencies]` rather than a bin-only/dev dependency before using it from `src/overlay`; (5) the `ff` binary entry is `ferrisfilter::overlay::launch()`, so arg parsing (`std::env::args`, same loop as ff-proto 61-85, `--tab rules|config|economy|about`) belongs in `launch()`/`OverlayApp::new`, because there is no `Args` plumbed today; also skip `instance::acquire()` (mod.rs:46) and the hotkey/tray in screenshot mode so it cannot collide with a running `ff`.

---

## Shared Patterns

### Deferred actions (collect during render, apply after)
**Source:** `app.rs` 1912-1918 and 2225-2257. **Apply to:** shell bar buttons, all modal choices, status-line jump, economy tab sync button. No state mutation of `self.working`/`self.config` while those are borrowed by a render closure; push `RowAction` / `ConfigAction` / `QuitChoice` and handle once after (`apply_row_actions` app.rs 567, `handle_config_actions` 1307).

### Pure decision fn + thin egui view (PITFALLS section 8)
**Source:** `visibility::advance/effect_for` (visibility.rs 70-115), `apply::footer_actions_combined` (apply.rs 410), `apply::quit_step` (383), `left_dock_rect`. **Apply to:** the new `status_for` priority fn, `active_popup` stacking fn, popup button-set per trigger (const slices of labels in order, test the order/labels), `dock_rect_px`. Tests live in the same file's `mod tests`.

### Widget shape: allocate -> interact -> paint -> `widget_info`
**Source:** `ff_button` 748-780, `ff_arrow` 784-813, `ff_check` 456-480. **Apply to:** `skin::icons` hit rects, `ff_modal` buttons, any new control. Always call `resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label))`; take values by `&mut`; `mark_changed()` on change.

### Headless egui widget tests
**Source:** `skin/widgets.rs` tests (1035-1188): `click_at(pos) -> RawInput` (1040), `setup() -> (Context, Skin)` (1062), `tabs_are_centred_evenly_spaced...` (1123), `long_status_is_cut_to_one_row...` (1153). Overlay-side equivalents: `app.rs` `stripe_ctx()` / `stripe_input()` (2320-2336), `rule_list.rs` `headless_ctx()` (864). **Apply to:** disabled `ff_button`, tab tooltips, `ff_modal` returns, status click-to-jump.

### Palette / fonts
**Source:** `skin/palette.rs` (`pal::GROUND, PANEL, ROW_EVEN/ODD/HOVER, INPUT_FILL, INPUT_BORDER(_HOT), GOLD, HEADING, LABEL, VALUE, MUTED, DISABLED, BUTTON_TEXT, ON_GOLD, OK, WARN, ERROR`) and `skin/fonts.rs` (`small_caps(size)`, `regular(size)`, `install(ctx)`). Allowed text sizes 14, 15, 17, 21 (+ art-derived header/tab sizes). **Apply to:** every new draw call; no `theme::` constants in new code.

### Single outcome surface
**Source:** `OutcomeLine` + `outcome_area` (app.rs 1560-1644). **Apply to:** replace with status line; delete `OUTCOME_MAX_HEIGHT` and `theme::` use there (`WARM_ACCENT_TEXT`, `TAB_ACTIVE`).

### Hidden-window state survives
Popup flags, scroll ids (`ff_scroll_area(..., id_salt)` with per-tab ids), `self.tab`, edits live on `OverlayApp`, not in egui temp memory; `Visibility::Hidden` only calls `win::conceal` (ShowWindow SW_HIDE), nothing else is reset. Keep it that way: do not reset popup flags on conceal. The conceal hook (app.rs 1821-1830) is the only place messages are cleared.

## No Analog Found

| File / Piece | Role | Data Flow | Reason |
|---|---|---|---|
| Per-monitor work area + `GetDpiForMonitor` Win32 calls (`win.rs`) | utility | request-response | Only `SPI_GETWORKAREA` (primary) is used today; `Win32_Graphics_Gdi` and `Win32_UI_HiDpi` windows-sys features are not enabled. Copy SAFETY-comment/handle-newtype style only; API shape from RESEARCH/Win32 docs. |
| Painted vector flag icons (`skin/icons.rs`) | component | transform | No existing painter-primitive icons; only texture-based icons (`skin/minimap.rs`, `combo_arrow`). Build with `egui::Shape` primitives. |
| Modal dim layer with art box (`ff_modal`) | component | event-driven | Only the plain `egui::Modal` (app.rs 2261) and the dropdown's `Popup` with 9-slice (widgets.rs 689-736) exist; combine them. |
| Click-through status line (`ff_status` with jump) | component | event-driven | `ff_status` is hover-only today; `outcome_area` (1604-1644) has the click/pointing-hand pattern but on a Label. |

## Metadata

**Analog search scope:** `src/bin/ff-proto.rs`, `src/skin/*`, `src/overlay/{app,mod,win,visibility,theme,layout,rule_list,config_view,widgets,apply}.rs`, `Cargo.toml`.
**Files scanned:** about 20 (targeted reads and greps; app.rs 3763, body.rs 5812, config_view.rs 2362 were read by section only; body.rs `NO_CONDITIONS`/`NO_ACTIONS` and the app.rs sync-button lines 2021-2100 were not read and need a targeted read when planning).
**Pattern extraction date:** 2026-10-16
