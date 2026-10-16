# Phase 33: Style Table & Line-by-Line Style Editor - Pattern Map

**Mapped:** 2026-10-16
**Files analyzed:** 9 (1 new, 8 modified)
**Analogs found:** 9 / 9
**Line numbers** are against the working tree on branch `milestone/v0.6-new-poe2-based-tool-ui` at HEAD `8f2c88e`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/overlay/config_view.rs` (rewrite `show_styles`, `styles_table_header`, `style_row`, `style_draft_row`, `style_editor` + `sound/minimap/beam_editor`) | component (egui view) | request-response (immediate-mode, returns `Vec<ConfigAction>`) | itself (current) + `src/bin/ff-proto.rs` `style_row`/`style_editor` (L694-1003) + `src/overlay/body.rs` `ff_field_row` action cards | exact (structure) / role-match (widgets) |
| `src/overlay/config_state.rs` (little or none: `StyleEdit`/`edit_style` stay) | model | CRUD in memory | itself | exact |
| `src/skin/colour_picker.rs` (NEW) `ff_colour_picker`, `PickerMode`, `PickerResult` | component (skin widget, popup) | event-driven / live edit | `ff_dropdown_ex` popup (`src/skin/widgets.rs` L787-881) + `body::picker_popup` (L1306-1426) + `widgets::colour_swatch` contract (`overlay/widgets.rs` L395-471) | role-match (no colour picker exists in skin) |
| `src/skin/mod.rs` (add `pub mod colour_picker;`) | config | n/a | existing `pub mod editor;` lines | exact |
| `src/skin/popup.rs` (add `pub fn paint_box_behind`, moved from `body.rs` L1287) | utility | transform (paints mesh) | `body::paint_box_behind` (L1285-1300), identical code in `ff_modal` L85-96 | exact |
| `src/skin/editor.rs` (drop `ff_card` use in editor; maybe add `ff_colour_card` open-ring, per-state tweaks) | component | request-response | itself | exact |
| `src/skin/minimap.rs` (add a Dracula quick-pick table, maybe `NAMES`) | utility/config | transform | itself (`SWATCHES`, `index_of`) | exact |
| `src/overlay/widgets.rs` (`colour_swatch`, `colour_swatch_rgb` re-implemented over the new popup; `hsva_of` to `pub(crate)`; retire `style_preview_chip`/`zebra_row` users) | component (wrapper) | request-response | itself L395-471 | exact |
| `src/overlay/body.rs` (`action_controls` L2896-2904 swatch call; `paint_box_behind` removed/re-pointed to skin) | component | request-response | itself | exact |
| `src/overlay/screenshot.rs` + `src/overlay/app.rs` `new_for_screenshot` (~L966-998) (new flags e.g. `--expand-style <name>`, `--picker <text\|border\|background\|rule>`) | utility (debug CLI) | batch (one frame) | `--expand` / `--rename-section` plumbing | exact |
| `src/overlay/app.rs` `handle_config_actions` (L1912-2060) | controller | event-driven | itself | exact (probably unchanged) |

## Pattern Assignments

### `src/overlay/config_view.rs` (component, request-response)

**Analog for layering:** itself. Module contract (L1-7): all model mutations are returned as `ConfigAction`s; the view never mutates `ConfigState` and never touches the filesystem. Keep it. The `edit()` helper (L956-961) is the single way an editor control emits a change:

```rust
fn edit(actions: &mut Vec<ConfigAction>, name: &str, edit: StyleEdit) {
    actions.push(ConfigAction::EditStyle {
        name: name.to_string(),
        edit,
    });
}
```

**Imports to add** (current imports L9-26; the skin side follows `body.rs`):
```rust
use crate::skin::editor as ed;                    // ff_segmented, ff_swatch, ff_colour_card, ff_item_label, paint_minimap_icon, ff_icon_button, ff_chip, LabelLook
use crate::skin::minimap::{self, COLOURS, SHAPES, SWATCHES};
use crate::skin::widgets as w;                    // ff_field_row, ff_check, ff_slider, ff_dropdown, ff_button(_ex), ff_arrow, ff_text_field, row_background, ROW_HEIGHT, ROW_PAD
```
Get the skin the way `body.rs` does, not from the `Option<&Skin>` in ctx: `let skin = crate::skin::of(ui.ctx());` (`skin/mod.rs` L57; loads a light skin in tests, so headless tests keep working with `skin: None`).

**Table cells** (replace `cell`/`cell_right`/`cell_with` L556-591 fixed-width machinery and `COL_*` L33-37): the existing pattern allocates a rect and a child Ui; keep that mechanism but with absolute x-starts from the UI-SPEC table (64/flex/32/160/56, gap 8, computed from `ui.available_width()`):
```rust
fn cell_with<R>(ui, salt, width, layout, add) -> (egui::Rect, R) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 24.0), egui::Sense::hover());
    let mut child = ui.new_child(egui::UiBuilder::new().id_salt(salt).max_rect(rect).layout(layout));
    let out = add(&mut child);
    (rect, out)
}
```
Keep the test `probe(tag, rect)` hook (L540-553, `#[cfg(test)]` thread-local) and add tags `head-icon`, `row-icon`; the layout tests depend on it. Existing probe tags: `head-vis head-name head-preview head-order row-vis row-name row-preview row-order row-up row-down builtin-heading custom-heading add-style`.

**Group heading + Add style** (current L644-667; swap `caption_text`/`small_button` for `small_caps(15)` `pal::LABEL` and `w::ff_button(ui, &skin, "+ Add style", 124.0)`; ff-proto analog `style_group_heading` L644-663):
```rust
ui.horizontal(|ui| {
    ui.set_min_height(34.0);
    ui.add_space(6.0);
    ui.label(egui::RichText::new(title).font(fonts::small_caps(16.0)).color(pal::GOLD));
    if add {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if w::ff_button(ui, skin, "+ Add style", 130.0).clicked() { self.add_style(); }
        });
    }
});
```
In config_view the click must keep the existing body: `if add.clicked() && cfg.style_draft.is_none() { cfg.style_draft = Some(String::new()); cfg.focus_draft = true; }` (L662-665).

**Row skeleton** (`style_row` L754-887). Preserve, verbatim in behaviour:
- `ui.push_id(("style", name, epoch), ...)` L768 (epoch salt orphans per-row egui state).
- click target = row rect minus switch, arrows and (custom) name field: computed `name_left` / `arrows_left` L809-851, then `ui.interact(row_rect, ui.id().with(("style-row", name)), Sense::click()).on_hover_cursor(PointingHand)`.
- accessible name L859-866: `WidgetInfo::selected(WidgetType::CollapsingHeader, true, is_expanded, name)`.
- `scroll_to` handling L867-870, and `if row.clicked() { cfg.set_expanded(name); egui::Popup::close_all(ui.ctx()); }` L871-874 (close_all is also what the colour picker needs so a picker never survives a row change).
- Switch cell L776-787: `widgets::slide_switch(ui, &mut on).on_hover_text(SWITCH_TIP)` then `edit(actions, name, StyleEdit::Action(..))` (kept per UI-SPEC; no skin switch exists).

Replace: `widgets::zebra_row(ui, i, ..)` with `w::row_background(ui, i, rect)` (skin/widgets.rs L447-456; pattern in ff-proto L695-696: `let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click()); w::row_background(ui, i, rect);`); `widgets::style_preview_chip` with `ed::ff_item_label(ui.painter(), rect_160x26, "Item Name", look)`; `widgets::arrow_button` with `w::ff_arrow(ui, &skin, up, enabled)`. Built-in rows: call `ff_arrow(.., enabled=false)` for both (disabled arrows ignore clicks, skin/widgets.rs L998-1034, `Sense::hover()` when disabled) and add tooltip `Built-in styles keep their order`. Expanded gold outline: ff-proto L699-704 `ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, pal::INPUT_BORDER_HOT), StrokeKind::Inside)` with `pal::GOLD` per UI-SPEC.

**Icon + label look in the row** (ff-proto L741-758, direct template):
```rust
let mut look = s.look();            // here: build LabelLook from the model Style (see below)
look.font_size = Some(26.0);        // UI-SPEC: font_size None in the table so the row does not grow
ed::ff_item_label(ui.painter(), chip, "Item", look);
...
if let Some(size) = s.mm_size {
    ed::paint_minimap_icon(ui.painter(), skin, icon.center(), s.mm_shape, s.mm_colour,
        minimap::icon_points(size, 28.0), Color32::WHITE);
}
```
Model mapping needed (no helper exists in config_view): `Style` (crate::models::config) has `text/border/background: Option<[u8;3]>`, `font: Option<u8>`, `minimap: Option<Minimap{size:u8,color:FilterColor,shape:MinimapShape}>`, `beam: Option<Beam{color,temp}>`, `sound: Option<Sound{id:Option<u8>,custom:Option<String>,vol:u16}>`, `disable_drop_sound`, `action: StyleAction`. Convert with `minimap::index_of(&minimap::SHAPES, m.shape.as_str())` / `index_of(&COLOURS, m.color.as_str())`; `LabelLook { text, border, background, font_size, hidden: style.action == StyleAction::Hide }` with `Color32::from_rgb(r,g,b)`. Put this as a small pure `fn label_look(&Style, font: bool) -> LabelLook` so it is unit-testable.

**Rename field** (custom Name cell; `rename_field` L691-741). Keep logic (draft keyed by `state.style_id(name)`, Enter/blur commit via `lost_focus()`, Escape restores, `rename_hint` red outline `INVALID_RED`, tooltip = hint or `config_warning(name)`), but swap `egui::TextEdit::singleline(..).id_salt("rename").desired_width(..)` for `w::ff_text_field_id(ui, id, &mut text, "Style name", width)` (skin/widgets.rs L1059-1069; 30pt tall). Escape/commit lines to copy unchanged:
```rust
let escaped = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape));
if escaped { text = name.to_string(); }
else if resp.lost_focus() && hint.is_none() && text.trim() != name {
    actions.push(ConfigAction::RenameStyle { from: name.to_string(), to: text.trim().to_string() });
}
```
Note: a 30pt field in a 34pt row; the old test `escape_restores_the_name` / `enter_commits_a_valid_rename...` / `built_in_names_are_not_editable` (config_view tests ~L1900-2060) drive this by typing into the field, so keep its egui id stable or update those tests to find it.

**Editor lines** (replace `style_editor` L963-1092 and `sound_editor`/`minimap_editor`/`beam_editor`). Each line = one `w::ff_field_row(ui, index, label, tip, |ui| ...)` (skin/widgets.rs L513-573: 128pt label col, control at +154, wraps with `with_main_wrap(true)`, row grows past 34 when the control wraps). The `index` argument drives zebra: pass a running counter. The row is drawn by the editor directly under the style row, with no `egui::Frame` (delete the `Frame::new().fill(theme::ROW_BASE).inner_margin(8)` wrapper at L875-884). Concrete control recipes already in ff-proto:

Visibility (ff-proto L836-841):
```rust
let mut sel = usize::from(!s.show);
if ed::ff_segmented(ui, skin, &mut sel, &["Show", "Hide"], 128.0).changed() { /* emit StyleEdit::Action */ }
```
Colour cards, Font size check + slider (ff-proto L850-884; spec range is 1-45 here, ff-proto used 18-45; `ff_slider` returns a `Response` with `changed()`, takes `&mut f64`):
```rust
for (k, name) in ["Text", "Border", "Background"].iter().enumerate() {
    if ed::ff_colour_card(ui, name, s.colors[k], card_w).clicked() { /* open picker */ }
}
let mut custom = s.font.is_some();
if w::ff_check(ui, skin, &mut custom, "Font size").changed() { /* StyleEdit::Font(custom.then_some(32)) as today L1032 */ }
w::ff_slider(ui, skin, f, 1.0..=45.0, 1.0, "Font size", |v| format!("{v:.0}"));
```
Convert slider `f64` to `i64` for `StyleEdit::Font(Some(v))` (note `config_state::edit_style` L235-243 rejects values that do not fit `u8`). Keep `body::font_hint(i64)` (body.rs L2517) as a tooltip on the slider.

Minimap size/colour/shape (ff-proto L911-952): `ff_segmented(&["None","Large","Medium","Small"])`, `ff_swatch(ui, Some(SWATCHES[c]), sel, vec2(24,24), name)`, `ff_icon_button(ui, skin, shape, colour, sel, 24.0)`. Today's model writes (config_view L1203-1209 seed and L1233-1255 clones) are the edit pattern:
```rust
let mut next = m.clone(); next.size = size; edit(actions, name, StyleEdit::Minimap(Some(next)));
```
Seed when switching from None: `Minimap { size: 2, color: FilterColor::White, shape: MinimapShape::Circle }` today; UI-SPEC wants size 0 (Large). Look up `FilterColor::from_name(&str)` and `MinimapShape::ALL` / `.as_str()` for index conversions.

Beam (ff-proto L956-987: `ff_swatch(ui, None, ..)` as the none tile, then 11 swatches, `ff_chip` for Temp; UI-SPEC says `ff_chip("Temp")`). Current model write L1270-1290: `StyleEdit::Beam(Some(Beam { color, temp }))`.

Sound (ff-proto L992: `w::ff_dropdown(ui, skin, ("sound", i), &mut s.sound, &sound_options, 190.0)` with `&[&str]`; options `None`, `Alert 1..16`, `Custom file`). Model writes to copy from `sound_editor` L1106-1119 (defaults `Sound{id:Some(1),custom:None,vol:100}` / `Sound{id:None,custom:Some("uwu.mp3".into()),vol:100}`) and L1126-1148 (custom file text; `vol: sound.vol`), volume `u16::try_from(v).ok()` pattern L1178-1188 (values that do not fit are dropped, never wrapped).

Drop sound: `w::ff_check` -> `StyleEdit::DisableDropSound(on)` (existing L1058-1068).

Load error line L1073-1082 stays (colour `pal::WARN` instead of `theme::WARM_ACCENT_TEXT`).

Delete line (custom only): currently L1084-1091:
```rust
if delete_visible(state, name) {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if ui.add(small_button("Delete style")).clicked() { cfg.arm_delete(name); }
    });
}
```
Swap for `w::ff_button_ex(ui, skin, "Delete style", 128.0, true, true)` (skin/widgets.rs L929; args `(ui, skin, text, min_width, enabled, danger)`). `arm_delete` sets `cfg.delete_armed`, which `app.rs` turns into `Popup::DeleteStyle`.

**Draft row** (`style_draft_row` L889-954): keep commit/discard state machine (Enter commit when `hint.is_none()`, Escape discard, `cfg.expanded = Some(name); cfg.scroll_to = Some(name); cfg.style_draft = None`), re-skin the field with `w::ff_text_field`/`ff_text_field_id` and the two buttons with `w::ff_button(.., 112.0)` / `(.., 168.0)`; keep `body::hint(ui, msg)` for the hint text.

**Keyboard / focus:** `ff_swatch`, `ff_segmented` and `ff_slider` already set `widget_info`; `ff_slider` locks left/right arrows only while focused (L671-682, `EventFilter`). Reuse as is.

**Cross-file constants to remove or retarget:** `COL_VISIBILITY/COL_NAME/COL_PREVIEW/COL_ORDER` (L33-37), `RENAME_TIP` (L38; text moves to the built-in Name tooltip), `NOT_SET` (L29), `WHOLE_NUMBER` (L28; only if slider_pair goes away), `segmented` (L280, still used elsewhere? check TIERS/filter before deleting), `caption_text` (L536, used by test `styles_header_captions_fit_their_columns`, which must be rewritten with the new columns).

---

### `src/skin/colour_picker.rs` (NEW; component, event-driven live edit)

**No direct analog**. Compose from three existing pieces.

1. Popup mechanism and frame - `ff_dropdown_ex` (`src/skin/widgets.rs` L835-873), and `body::picker_popup` (L1323-1418). Copy this skeleton:
```rust
egui::Popup::from_toggle_button_response(&resp)
    .id(id)
    .width(width)
    .gap(2.0)
    .frame(egui::Frame::NONE.inner_margin(6))
    .show(|ui| {
        let bg = ui.painter().add(Shape::Noop);      // reserve the layer slot first
        ui.spacing_mut().item_spacing.y = 0.0;
        /* ... widgets ... */
        let frame_rect = ui.min_rect().expand(6.0);
        let mut mesh = egui::Mesh::with_texture(skin.box_9slice.id());
        for p in slice::nine_slice(frame_rect, skin.box_9slice.size(),
                Margins::new(8.0, 8.0, 8.0, 8.0), 1.0, ui.painter().pixels_per_point()) {
            mesh.add_rect_with_uv(p.dest, p.uv, Color32::WHITE);
        }
        ui.painter().set(bg, Shape::mesh(mesh));
    });
```
`picker_popup` additionally uses `.close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)` (body L1324) - the picker wants this (UI-SPEC: click outside keeps the value and closes). Closing programmatically: `egui::Popup::close_id(ui.ctx(), popup_id)` (body L1414); open state: `egui::Popup::is_id_open(ctx, id)` (widgets L805); `egui::Popup::default_response_id(button)` (body L1314). Other open popups: `egui::Popup::close_all(ctx)` (config_view L873).

   The 9-slice painter is duplicated three times (popup.rs L85-96, widgets.rs L861-872, body.rs L1287-1300): the UI-SPEC says to move body's `paint_box_behind(ui, skin, bg)` into `skin::popup` as `pub fn` and call it from the picker. Use inner padding 16 (`Frame::NONE.inner_margin(16)`) and content width 268; the 8pt box margin in `paint_box_behind` is `expand(6.0)` today, so check it against the spec's "8pt margins".

2. Temp memory for open state - same idiom as `picker_popup`'s query (body L1332-1343):
```rust
let mut query = ui.data_mut(|d| d.get_temp::<String>(query_id)).unwrap_or_default();
ui.data_mut(|d| d.insert_temp(query_id, query.clone()));
...
if !egui::Popup::is_id_open(&ctx, popup_id) { ctx.data_mut(|d| { d.remove::<String>(query_id); d.remove::<bool>(focus_id); }); }
```
Apply to the `Hsva` (needs `Clone + Send + Sync + 'static`; `Hsva` qualifies) and the "value the picker opened with" (for Escape restore).

3. Colour model + no-op guard - `src/overlay/widgets.rs` L395-436:
```rust
fn hsva_of(rgba: [u8; 4]) -> egui::ecolor::Hsva {
    use egui::ecolor::linear_f32_from_gamma_u8 as lin;
    egui::ecolor::Hsva::from_rgba_unmultiplied(lin(rgba[0]), lin(rgba[1]), lin(rgba[2]), f32::from(rgba[3]) / 255.0)
}
...
if response.changed() {
    let now = colour.to_srgba_unmultiplied();
    if now != rgba { result = Some(now); }   // opening/idle never emits
}
```
Make `hsva_of` `pub(crate)` or (better, since it is a skin widget now) re-home it in `colour_picker.rs` and `use crate::skin::colour_picker::hsva_of` from `overlay/widgets.rs`; existing test `colour_model_keeps_rgb_at_zero_and_low_alpha` (widgets.rs L1258-1277) must keep passing wherever it lands.

4. Widgets inside the picker: `ff_text_field(ui, &mut String, hint, width)` for Hex/R/G/B (fixed 30pt tall, so the R/G/B 44pt boxes with `ff_text_field` are tight: they must hold 3 chars at `regular(15)` with 10pt side padding - verify), `ff_slider(ui, skin, &mut f64, 0.0..=255.0, 1.0, "Alpha", |v| format!("{v:.0}"))`, `ff_swatch` for the 9 Dracula quick picks and the None tile (`ed::ff_swatch(ui, None, selected, vec2(24,24), "No colour")`), `ff_button(ui, skin, "Use colour", 112.0)`. Square/hue-bar drawing: use `ui.allocate_exact_size(.., Sense::click_and_drag())`, paint with an `egui::Mesh` of coloured vertices (no existing gradient helper; `ff_slider` L655-669 shows the pointer-to-value pattern: `resp.dragged() || resp.clicked()` then `resp.interact_pointer_pos()`). Keyboard on square/bar: copy the focus-lock pattern from `ff_slider` L671-703 (`ui.memory_mut(|m| m.set_focus_lock_filter(resp.id, EventFilter{ horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false }))`).

5. Escape: `ff_slider` explicitly leaves `escape: false` so the popup can see it; read `ui.input(|i| i.key_pressed(Key::Escape))` inside the popup closure and emit one final edit with the opening value (UI-SPEC "Semantics"). Do not rely on `PopupCloseBehavior` for Escape (egui closes the popup on Escape by itself; the revert must happen on that frame).

**Public surface to export** (from UI-SPEC): `ff_colour_picker(ui, skin, id, &mut Hsva, mode: PickerMode) -> PickerResult`, `PickerMode { alpha: bool, allow_none: bool }`. Result should carry `changed: Option<Option<[u8;4]>>`-like data so wrappers keep the no-op guard.

**Tests (pattern):** inline `#[cfg(test)] mod tests` in the same file; a headless Context via `Context::default(); super::fonts::install(&ctx); let skin = Skin::load(&ctx);` (see `skin/editor.rs` tests L448-485). Click events built from `Event::PointerMoved`, `PointerButton{pressed:true}`, `PointerButton{pressed:false}`, run in two `ctx.run_ui` calls (frame 1 lays out, frame 2 delivers the click), each followed by `.drop_without_applying_deltas()`:
```rust
let pos = pos2(rect.max.x - 20.0, rect.center().y);
let click = RawInput { events: vec![Event::PointerMoved(pos),
    Event::PointerButton { pos, button: PointerButton::Primary, pressed: true,  modifiers: Modifiers::default() },
    Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::default() }], ..Default::default() };
```
Plus pure-function tests (hex parse `#FF79C6` <-> `[255,121,198]`, 3-digit hex, invalid hex returns None, hsva round trip) in the style of `colour_model_keeps_rgb_at_zero_and_low_alpha`.

---

### `src/overlay/widgets.rs` `colour_swatch` / `colour_swatch_rgb` (wrapper, request-response)

**Analog:** itself L408-471. Keep the signatures and `Option<new>` contract; replace the `egui::color_picker::color_edit_button_hsva(ui, &mut colour, Alpha::{OnlyBlend|Opaque})` call with a skin swatch trigger + `colour_picker::ff_colour_picker`. Per UI-SPEC the RULES trigger is a 34x22 chip with the caption demoted to a tooltip (`rgba_caption`, L391, stays, test at L1279-1285). Keep existing test `colour_swatch_idle_frames_return_none_even_when_translucent` (L1245-1256) green: it runs two idle frames for opaque/translucent/transparent colours and asserts `None`. Add a third thin `colour_slot(ui, skin, Option<[u8;3]>, seed) -> SlotResult` for the CONFIG cards.

---

### `src/overlay/body.rs` (RULES colour action)

**Analog:** `action_controls` L2896-2904:
```rust
Value::Rgba { rgba, .. } => {
    if let Some(n) = widgets::colour_swatch(ui, *rgba)
        && let Some(a) = rgba_edit(card.id, card.entry, n)
    {
        actions.push(a);
    }
    None
}
```
No change needed here if the wrapper keeps its signature. `rgba_edit` (L2420-2431) keeps the 3-token form while alpha stays 255 (`had_alpha: entry.values.len() >= 4`); its test `rgba_edits_keep_form` (L5001). `paint_box_behind` (L1287) moves to `skin::popup`; update the call at L1416 and the source-text test at L5670-5679 that asserts `body.contains("paint_box_behind(")` inside `picker_popup` (it reads the source via `find("pub(crate) fn picker_popup")`, so keep the call text `paint_box_behind(` in that function, e.g. `crate::skin::popup::paint_box_behind(`).
Not touched this phase: `colour_combo` (L3087), `minimap_control`, `effect_control`, `SIZE_CHOICES` (L2535), `slider_pair` (L2930). `config_view` currently imports `body::{caption, hint, slider_pair, colour_combo, SIZE_CHOICES, font_hint}`; after the rewrite only `hint` and `font_hint` should remain in use there.

---

### `src/skin/editor.rs` / `src/skin/minimap.rs` (component / utility)

Already contain every control the editor needs (`ff_segmented` L25, `ff_swatch` L112, `ff_colour_card` L168, `ff_item_label` L241, `paint_minimap_icon` L283, `ff_icon_button` L307, `ff_chip` L394, `ff_card` L361 which the new editor must NOT use). Likely edits:
- `ff_colour_card`: add an `open: bool` (or separate fn) that draws the 2pt `pal::HEADING` ring when its picker is open (existing hover/normal stroke code L192-204); keep the old signature working for ff-proto (`ff-proto.rs` L851 calls `ed::ff_colour_card(ui, name, s.colors[k], card_w)`).
- `ff_swatch(.., selected, size, label)` L112-154: UI-SPEC wants the selected ring colour `pal::HEADING`: already so (L133).
- `skin/editor.rs` module doc L5-6 says "There is deliberately no colour picker here yet"; update it.
- `minimap.rs`: add `pub const DRACULA: [(&str, Color32); 9]` (names + RGB per UI-SPEC) next to `SWATCHES` L35-47, with a unit test in the existing `mod tests` (L81-103) style (e.g. exact-RGB assertions).
- `ff-proto.rs` also uses `ff_card` (L836-989); leave it working (it is not in scope), so do not delete `ff_card`.

---

### `src/overlay/screenshot.rs` + `src/overlay/app.rs::new_for_screenshot` (debug flags)

**Analog:** `--expand` / `--rename-section` end to end.

1. `ShotArgs` field + doc (screenshot.rs L40-45):
```rust
/// Expand the n-th rule (1-based) of the RULES tab.
pub expand: Option<usize>,
...
pub rename_section: Option<usize>,
```
2. Parse arm with validation (L83-100):
```rust
"--expand" => {
    let n = value("--expand")?;
    expand = match n.parse::<usize>() {
        Ok(v) if v >= 1 => Some(v),
        _ => return Err(format!("--expand needs a rule number >= 1, got `{n}`")),
    };
}
```
   For styles a name is friendlier: `"--expand-style" => expand_style = Some(value("--expand-style")?)`, plus `"--picker"` validated against a `const PICKERS: [&str; N]` array like `POPUPS` (L17-25, error `format!("unknown popup `{p}` ({})", POPUPS.join("|"))`). Add the new fields to the final `ShotArgs { .. }` struct literal (L105-114) and to the module doc line 1-2.
3. Application in `app.rs` `new_for_screenshot` (L966-998). Mirror:
```rust
if let Some(n) = shot.rename_section { ... app.list.editing_section = Some(sec.ordinal); ... }
```
   New: `if let Some(name) = &shot.expand_style { app.config_ui.set_expanded(name); app.config_ui.scroll_to = Some(name.clone()); }` (`ConfigUi::set_expanded` L112-120 bumps `epoch`; `scroll_to` is consumed in `style_row` L867-870). For the picker flag the open state must be forced before the first frame: set the same temp-memory flag / `Popup` id that `ff_colour_picker` uses (derive the id from a stable salt like `("cfg-style", name, "text")` so the screenshot code can open it via `egui::Popup::open_id(ctx, id)` or a debug-only `ConfigUi` field). Also `if shot.tab == ShellTab::Config { app.refresh_filters(); }` already exists (L968).
4. `--popup delete-style` already works via `open_shot_popup` (L1021-1029; picks the first custom style or `"kite"`); nothing to add.
5. Parse tests: extend the `a` full-args test (screenshot.rs ~L277-298) and the error list (L311-321), same style `assert!(p(&["--screenshot","o.png","--expand-style"]).is_err());`.
6. Everything here is `#[cfg(debug_assertions)]`-gated at the use site in app.rs; copy the `#[cfg(debug_assertions)]` guards used on `open_shot_popup` (L1003).
7. Remember the module doc comment in CLAUDE.md "Repository layout" `ff` row lists the screenshot flags; update it with the new flags.

---

### `src/overlay/app.rs` (controller; ConfigAction -> pending style edits -> Apply/Revert)

**Flow (read-only reference, probably no code change):**

1. View: `app.rs` `config_content` builds `ConfigViewCtx` (L2088-2101) with `rule_users` (from `working.style_users()`), `config_refs`, `skin: Some(skin)`, then inside `w::ff_scroll_area(ui, skin, "config", ..)` calls `config_view::show_sections(ui, state, &mut self.config_ui, &ctx)` (L2102-2110) and collects `Vec<ConfigAction>` (`state` is `&ConfigState`, immutable).
2. Dispatch: the collected actions go to `handle_config_actions(ctx, actions)` (L1912). It first calls `self.clear_events_on_edit()` if any is `AddStyle | EditStyle | DeleteStyle | RenameStyle | MoveStyle | TierText | Revert` (L1914-1928).
3. `EditStyle` (L1944-1950):
```rust
ConfigAction::EditStyle { name, edit } => {
    if let Ok(state) = &mut self.config
        && state.edit_style(&name, edit)
    {
        self.config_message = None;
    }
}
```
   `ConfigState::edit_style` (config_state.rs L234-260) mutates the pending `StyleSet` in memory (`Font` is range-checked to `u8`; everything else stored verbatim). `ConfigState::is_dirty/dirty_count/can_apply` (L340, L483, L530) compare pending against baselines by value, so "open then close the picker with no change" leaves Apply disabled only if the view emits no action for an unchanged colour (the no-op guard above).
4. Apply (shared bottom bar): `ConfigAction::Apply` -> `run_apply(ctx, false)` (L2009); `validate()` / `prepare_write()` / `adopt()` in config_state.rs (L538, L558, L586) write `styles.json` all-or-none with the filter.
5. Revert: `ConfigAction::Revert` -> `state.revert()` + `self.config_ui.reset_for_new_styles()` + message `"Reverted config changes."` (L1998-2003). `reset_for_new_styles` (config_view L135-138) must also drop any picker temp state if the new editor keeps per-row memory (e.g. last-values-for-switching-back from UI-SPEC: key them under `cfg.epoch` or clear via `ctx.data_mut(|d| d.clear())` is too broad; prefer keying by `(id_salt, name, epoch)` so epoch bumps orphan them as `style_row` does).
6. Delete: editor button -> `cfg.arm_delete(name)` -> `app.rs` builds `Popup::DeleteStyle` prompt (L1651-1660: `config_state::delete_confirm_text(name, users)` + `config_warning(name)` when `config_refs.contains(name)`) -> `dispatch_popup` (L1734-1740): choice 0 -> `ConfigAction::DeleteStyle(name)`; `delete_armed.take()` runs for any choice. No change this phase.
7. Rename / Move set `ApplyState::Unapplied` explicitly (L1974-1976, L1987-1989) because they do not go through `edit_style`; plain `EditStyle` relies on `ConfigState::is_dirty`. Any new action (none planned) must follow the same rule.

If the picker holds a live value while a Revert / Reload happens, it must close: call `egui::Popup::close_all(ctx)` in `reset_for_new_styles` call sites or key picker memory by `epoch`.

---

## Shared Patterns

### Immediate-mode view returns actions, never mutates the model
**Source:** `src/overlay/config_view.rs` L1-7 and `ConfigAction` L54-84.
**Apply to:** every new/changed control in `config_view.rs` and the three colour wrappers: widgets return `Option<new value>`, the caller wraps in `ConfigAction::EditStyle` / `RowAction`; `ConfigState` / `Working` apply after the frame.

### Skin acquisition
**Source:** `body.rs` L1329 `let skin = crate::skin::of(ui.ctx());`; `skin/mod.rs` L57-70 (`of` falls back to `textures::load_light` under `#[cfg(test)]`).
**Apply to:** all new skin-widget call sites in `config_view.rs` (do not require `ConfigViewCtx.skin`; tests construct `skin: None`).

### Field row (editor line)
**Source:** `src/skin/widgets.rs` `ff_field_row` L513-573 (+ `LABEL_COL_W` 128, `LABEL_GAP` 16, `ROW_PAD` 10, `ROW_HEIGHT` 34). RULES precedent in `body.rs` action cards.
**Apply to:** all 11-12 editor lines; control column is `517 - 154 - 10 = 353pt` at the real window width.

### Row ground and zebra
**Source:** `skin/widgets.rs` `row_background` L447-456 (`ROW_HOVER` when pointer inside, else `ROW_EVEN`/`ROW_ODD` by index).
**Apply to:** table rows (index per group; each table restarts at 0 as `zebra_row(ui, i, ..)` does today) and `ff_field_row` index inside the editor.

### Popup frame art
**Source:** `skin/widgets.rs` L861-872 / `skin/popup.rs` L85-96 / `body.rs` L1287-1300 (same nine-slice mesh with `Margins::new(8,8,8,8)`).
**Apply to:** `colour_picker.rs`; consolidate into `skin::popup::paint_box_behind` (UI-SPEC).

### Tooltips instead of captions
**Source:** `ff_field_row(.., tip: Option<&str>, ..)` L513-571; memory note "Overlay UI not texty".
**Apply to:** Font size check, Drop sound, built-in arrows, built-in Name, picker None tile, rule colour chip (`rgba_caption`).

### Error/no-op guards in colour paths
**Source:** `overlay/widgets.rs` L408-436 (return `Some` only if `now != rgba`), WR-04 `hsva_of` L395-406, `body::rgba_edit` L2420.
**Apply to:** `colour_picker` + both wrappers + `colour_slot`; opening a picker must never emit an action or rewrite a 3-token line.

---

## Test Patterns (how config_view / config_state tests are written today)

**Fixtures** (identical helpers in both files; copy, do not share):
```rust
fn temp_root() -> tempfile::TempDir { /* copies styles-default.json -> styles.json, tiers-default.json -> tiers.json from env!("CARGO_MANIFEST_DIR") */ }
fn loaded() -> (tempfile::TempDir, ConfigState) { let dir = temp_root(); let state = ConfigState::load(dir.path()).unwrap(); (dir, state) }
```
(config_state.rs L692-716; config_view.rs L1337-1356.) `config_state.rs` adds `full_root()` (L1020) for save tests and `defaults_root()` (L750) for fallback tests.

**Headless egui** (`config_view.rs` L1358-1390): `headless()` returns `(Context, RawInput)` with `crate::skin::fonts::install(&ctx)` and a 460x600 `screen_rect`; `tall_input()` (L1661) is 460x1400 so nothing scrolls out. Note 460pt is narrower than the real 517pt content, so width-dependent columns (flexible Name = `available - ...`, min 110) get their minimum in tests; assert against the probe rects, not hard-coded 517 numbers. `view_ctx(filters, &users, &refs)` builds a `ConfigViewCtx` with `skin: None`. Always end a frame with `.drop_without_applying_deltas()`.

**Idle-frame assertion** (L1478-1503): render each expanded style (`S`, `unchanged`, `hidden`, `kite`) via `show_sections` and `assert_eq!(actions, Some(vec![]))` - this is the guard that every new control (picker trigger, sliders, text fields) emits nothing without input. Extend the list to cover a custom style and a style with a minimap, beam and sound.

**Layout probes** (L540-553 + tests L1738-1818): `drain_probes()` + `of(tag)` closures compare x-starts across header and rows; rewrite `styles_table_header_and_columns` and `styles_layout_runs_heading_then_header_then_rows_per_group` for the new columns (Icon added, Order present in both groups). Rewrite `styles_header_captions_fit_their_columns` (L2177-2193) against the new column widths (it references the removed `COL_*` constants and `caption_text`).

**Click simulation** (`click_styles` L2060-2091): frame 1 settles hover/layout, frame 2 delivers `PointerMoved` + press + release at `target`; collects `Vec<ConfigAction>` from `show_styles(ui, state, cfg, &vctx, &mut actions)`. `style_switch_click_emits_hide` (L1553-1617) scans a grid of positions because rects were not probed; prefer probing a rect and clicking its centre (as `custom_arrows_reorder_and_builtins_have_none` L2101-2164 does with `rects("row-up")`). Note it asserts disabled arrows emit nothing and "arrows only on custom rows" (`ups.len() == custom.len()`): that assertion changes (built-in rows now draw disabled arrows).

**Text input in tests:** `type_into_first_custom` (~L1903-1960) injects `Event::Text` into the rename field; tests `enter_commits_a_valid_rename_and_the_draft_survives_typing`, `an_invalid_rename_is_not_emitted_and_the_draft_is_kept`, `escape_restores_the_name`, `built_in_names_are_not_editable`. Keep the field id / salt reachable or adapt the helper.

**Pure-state tests** (`config_state.rs` L906-1018): `add_style_trims_and_rejects_bad_names`, `font_out_of_u8_is_refused_and_out_of_range_blocks_apply`, `other_style_edits_are_stored_verbatim` (every `StyleEdit` variant), `revert_after_save_returns_to_saved_values`. Phase 33 does not change `StyleEdit`, so add only new cases if a new variant is introduced.

**App-level tests** (`app.rs` L4633-4710): `app.handle_config_actions(&ctx, vec![ConfigAction::Revert])`, `app.dispatch_popup(&ctx, Popup::DeleteStyle, 1)` with `popup_app(RULES)`; assertions on `app.config_ui.delete_armed`. Use for any new Revert/close-picker behaviour.

**Screenshot parse tests:** `screenshot.rs` L263-321 (`p(&[..])` helper, one big happy-path test + a list of `is_err()` cases).

**Skin widget tests:** `skin/editor.rs` L434-485 (headless `Context`, `Skin::load(&ctx)`, two frames, `ff_segmented(..).changed()`); `skin/minimap.rs` L81-103 (pure table/UV tests).

**Verification in this repo:** `cargo test` (nightly, workspace root builds only `ff`), plus the manual walk in 33-UI-SPEC.md with `cargo run --bin ff -- --screenshot <png> --tab config ...` under a temp `FERRISFILTER_HOME`/`USERPROFILE`.

---

## No Analog Found

| File / piece | Role | Data Flow | Reason |
|--------------|------|-----------|--------|
| HSV saturation/brightness square, hue bar, checkerboard alpha strip (inside `colour_picker.rs`) | widget | drag input | No gradient-painting widget exists in `src/skin/`; the old picker was egui's built-in `color_edit_button_hsva`. Use egui `Mesh` vertices; pointer mapping pattern from `ff_slider` (`slider_value_at` L607-619, `resp.interact_pointer_pos()`). `style_preview_chip` (`overlay/widgets.rs` L485+) draws a checkerboard already (background over a checkerboard) - read it for the strip. |
| Hex / R / G / B text parsing | utility | transform | None exists; write as pure fns with unit tests. |
| Skin on/off slide switch for Visibility | widget | n/a | `overlay::widgets::slide_switch` (32x16) is kept per UI-SPEC. |
| Remembering last Minimap/Beam/Sound values per row for switch-back | state | temp memory | No precedent for per-row restore; use `ui.data_mut(|d| d.insert_temp(id, v))` keyed by `(name, field, cfg.epoch)` like `picker_popup`'s `query_id`. |

## Metadata

**Analog search scope:** `src/overlay/{config_view,config_state,body,widgets,app,screenshot}.rs`, `src/skin/{editor,minimap,widgets,popup,mod}.rs`, `src/bin/ff-proto.rs`.
**Files scanned:** ~12 (read targeted ranges; `body.rs`, `app.rs`, `widgets.rs` read by grep + offset).
**Pattern extraction date:** 2026-10-16
