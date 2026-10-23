# Phase 48: Game-Art Widgets - Research

**Researched:** 2026-10-23
**Domain:** Rust / egui 0.36 overlay skin (`src/skin/`), `tools/ff-art` extraction of PoE2 UI art
**Confidence:** HIGH (every extraction claim was run against the real game install; widget claims read from the code)

## Summary

All four art pieces extract cleanly with the existing manifest format and **no `ff-art` code change**.
A trial run of the full `ff-art` extract (every existing entry plus the 14 new ones) against a scratch
copy of `assets/` produced the 14 new PNGs and left **every existing output byte-identical**
(`diff -rq` of the whole `assets/` tree showed only the new files and `manifest.toml`), including the
minimap atlas, fonts and both Ferris renders. So `cargo ff-art` (extract everything) is safe to run in
the repo; there is no churn and no need for a single-entry mode (ff-art has none).
[VERIFIED: trial run, scratch root `C:\Users\jonat\AppData\Local\Temp\claude\ffart-scratch-48`]

Key manifest finding: `trim` only drops the right/bottom gutter (it crops to `[0,0,max_x,max_y]`), so it
cannot remove the left/top gutters of the toggle plates (4 left, 5 top) or the bar's left cap
(19 left). Use **`crop = [x, y, w, h]` alone** (w/h, not max coordinates) for all new entries. The
UI-SPEC's "trim box" columns are min/max boxes; the manifest wants x, y, **width, height**.

The painting side needs one new pure helper (UV for a visible sub-box), one new widget
(`ff_search_field`), a rewrite of the body of `draw_section_header`, and a small change to
`ff_reload_button` (plate fills the rect). The existing `slice::three_h_rects` + `paint_three` already
do exactly the bar three-slice (caps keep aspect at the bar height: 132x75 and 151x75 at 36pt give
63.4pt and 72.5pt caps, the spec's 63 / 72).

**Primary recommendation:** add the 14 manifest entries below (crop only), run `cargo ff-art` once, embed the PNGs as new `Skin` fields, then build `box_uv`/`paint_visible` in `slice.rs`, `ff_search_field` in `widgets.rs`, redraw the header, and fix `ff_reload_button` to paint the visible plate into the full row-height rect.

## Architectural Responsibility Map

Not a multi-tier app (single local egui desktop process). Ownership by module:

| Capability | Owner | Rationale |
|------------|-------|-----------|
| Art extraction / PNG names | `tools/ff-art` + `assets/poe2/manifest.toml` | only writer of `assets/poe2/` |
| Texture embedding | `src/skin/textures.rs` (`Skin`) | `tex!` + `include_bytes!` |
| Visible-plate / slice geometry (pure, tested) | `src/skin/slice.rs` | house style: pure geometry + painter |
| Search field (glyph, X, text inset) | `src/skin/widgets.rs` (`ff_search_field`) | shared by two call sites |
| Section header bar | `src/overlay/rule_list.rs` (`draw_section_header`) | list-specific layout |
| Reload button placement | `src/skin/widgets.rs` (`ff_reload_button`) | one widget, two call sites, no per-site nudges |

## User Constraints (from CONTEXT.md)

### Locked Decisions
- Extract the Social-panel toggle plate (and its hover state if the game has one), the stone bar, the round clear X and the magnifying glass with `ff-art` into `assets/poe2/` (manifest entries). The PoE2 install is at the default Steam path on this machine.
- If any of these is **not found**, paint a substitute matching the reference crops and **record it for the author's review** (list which piece was substituted and why) — in the phase SUMMARY and the end-of-run review list. (All four were found; nothing is substituted.)
- References: `.planning/research/ui-refs/social-*.png`, ROADMAP backlog 999.23-999.25.
- Section headers: 36pt bar (unchanged height): toggle plate at the far left — **chevron up above a dot = expanded**, **dot above a chevron down = collapsed** — then the stone bar with its bronze frame. Title stays **left-aligned**; `{n} rules`, rename pencil, up / down arrows, remove x stay on the right. The whole bar stays clickable to toggle. The economy header gets the same look and keeps its padlock.
- Search fields (RULES search, the + condition picker, and any other search box found): the round clear X inside the right end while the field holds text — a click empties it and keeps focus (same effect as Escape in the RULES toolbar); the magnifying glass inside the left end, with the placeholder / text shifted right.
- Art margins (ART-04): game and custom button art carries transparent margins (left and bottom gutters). Position every new button by its visible plate, compensating those margins. Fix the existing offset of the **Config → Filter refresh button**; check the other uses of the same art (e.g. the Economy league refresh).

### Claude's Discretion
- Slicing / painting approach (three-slice like the tabs), hover / pressed visuals for the plate.

### Deferred Ideas (OUT OF SCOPE)
None.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ART-01 | Section headers use the game's Social-panel style (plate + stone bar) | manifest entries `toggle_*` + `bar_*`; `three_h_rects` + `paint_three`; `draw_section_header` rewrite (below) |
| ART-02 | Search fields show the round clear X | `clear_*` entries; `ff_search_field`; refocus via `ctx.memory_mut(request_focus)` |
| ART-03 | Search fields show the magnifier | `search_*` entries; text rect inset 32pt left |
| ART-04 | Button art positioned by its visible plate; fix the reload button offset | new art cropped at extraction; `box_uv` helper for legacy PNGs; `ff_reload_button` fix |
</phase_requirements>

## Standard Stack

No new dependencies (egui/eframe 0.36.1, image 0.25 already in use; `ff-art` unchanged). Package Legitimacy Audit: not applicable, no external packages are installed. [VERIFIED: Cargo.toml / Cargo.lock]

## Manifest entries to add (verified)

Append to `assets/poe2/manifest.toml` (e.g. after the generic-button block, before the minimap section). `prefer_4k = true` resolved the `4K/` sibling for every one. Output sizes and opaque boxes were confirmed with `ff-art info` on the trial output (`opaque=[0,0,w,h]` for all, so no residual gutter).

```toml
# Social-panel toggle plate: "collapse" is the game's name for the plate shown while the group is
# open (chevron up over a dot); "expand" is shown while closed. Left 4 / top 5 transparent pixels
# are cropped (trim only drops right/bottom), so the PNG is exactly the 54x64 plate.
[[image]]
path = "art/textures/interface/2d/2dart/uiimages/common/buttoncollapsenormal.dds"
out = "common/toggle_collapse_normal.png"
prefer_4k = true
crop = [4, 5, 54, 64]
# ... same for buttoncollapsehover -> toggle_collapse_hover.png, buttoncollapsepressed -> toggle_collapse_pressed.png
# ... and buttonexpandnormal/hover/pressed -> toggle_expand_normal/hover/pressed.png (same crop)

# Stone bar (npcmenubar): left cap 132x75 (19px left gutter + 24px bottom gutter cropped),
# middle 111x75, right cap 151x75.
[[image]]
path = "art/textures/interface/2d/2dart/uiimages/ingame/npcmenubarleft.dds"
out = "common/bar_left.png"
prefer_4k = true
crop = [19, 0, 132, 75]
[[image]]
path = "art/textures/interface/2d/2dart/uiimages/ingame/npcmenubarmiddle.dds"
out = "common/bar_middle.png"
prefer_4k = true
crop = [0, 0, 111, 75]
[[image]]
path = "art/textures/interface/2d/2dart/uiimages/ingame/npcmenubarright.dds"
out = "common/bar_right.png"
prefer_4k = true
crop = [0, 0, 151, 75]

# Round clear X (cream disc, dark X): the disc is 36x36 (the 37th row/column is transparent).
[[image]]
path = "art/textures/interface/2d/2dart/uiimages/common/buttoncleartext.dds"
out = "common/clear_normal.png"
prefer_4k = true
crop = [0, 0, 36, 36]
# ... buttoncleartexthover -> clear_hover.png, buttoncleartextpressed -> clear_pressed.png

# Magnifier glyph (no plate); hover cropped to the same box so the two align.
[[image]]
path = "art/textures/interface/2d/2dart/uiimages/common/buttonsearchnormal.dds"
out = "common/search_normal.png"
prefer_4k = true
crop = [6, 4, 48, 49]
# ... buttonsearchhover -> search_hover.png (same crop)
```

(The abbreviated "same for" lines must be written out as full `[[image]]` entries: 6 toggle, 3 bar, 3 clear, 2 search = 14.)

Verified output sizes: toggle 54x64 (x6), `bar_left` 132x75, `bar_middle` 111x75, `bar_right` 151x75, clear **36x36** (x3), search 48x49 (x2).

**Deviation from UI-SPEC:** the clear X crop is `[0,0,36,36]`, not 37x37 (`ff-art info` on a 37x37 crop shows the opaque box is 36x36 with a transparent last row and column). 36 art px x 0.5 = the spec's 18pt disc. Painting the 36x36 PNG into an 18x18pt rect gives exactly the intended disc.
[VERIFIED: `ff-art info`]

The reference `social-*.png` crops are untouched; the planner should still compare screenshots to them (`.planning/research/ui-refs/`).

### Running ff-art safely
- Build/run with `CARGO_TARGET_DIR` outside the repo, e.g. `C:\Users\jonat\AppData\Local\Temp\claude\ffart-target` (a built `release/ff-art.exe` already exists there). Default game path `C:\Program Files (x86)\Steam\steamapps\common\Path of Exile 2` exists on this machine. [VERIFIED]
- `ff-art --root <dir>` writes to `<dir>/assets/poe2` and `<dir>/assets/ui`; needs `<dir>/assets/poe2/manifest.toml` and `<dir>/assets/ferris/*.svg` (a full copy of `assets/`). Use it for a dry run; for the real run plain `cargo ff-art` from the repo (writes only `assets/`).
- Afterwards `git status` must show only the 14 new PNGs and `manifest.toml` modified. Any other changed PNG means a game patch changed the art: stop and report.

## Package/Code Architecture

### Embedding (`src/skin/textures.rs`)
- `tex!(ctx, "poe2/common/x.png")` = `load(ctx, path, include_bytes!("../../assets/<path>"))`. Paths are `poe2/...` (manifest `out` is relative to `assets/poe2/`).
- `Skin` is a plain struct of `TextureHandle`s, `ThreeSlice { left, middle, right }`, arrays for states (`close: [_;3]`, `icon_button: [_;3]`, `button: [ThreeSlice;3]`, `combo_arrow: [_;2]`). Add, mirroring that style:
  - `toggle_collapse: [TextureHandle; 3]` and `toggle_expand: [TextureHandle; 3]` (normal, hover, pressed)
  - `bar: ThreeSlice`
  - `clear: [TextureHandle; 3]`, `search: [TextureHandle; 2]`
- Tests build the skin through `Skin::load` (real decode, in `every_embedded_texture_decodes`) or `textures::load_light` (2x2 stand-ins; layout tests only). Extend `every_embedded_texture_decodes` with `assert_eq!(skin.toggle_collapse[0].size(), [54, 64])`, `bar.left.size() == [132, 75]`, etc., so a bad re-extract fails a test. Light textures are 2x2, so any new code that derives size from `tex.size()` (e.g. `three_h_rects(.., tex.size())`) behaves differently in light tests: keep geometry in pure functions fed with the real constants, not texture size, where a test must assert numbers.
- No test iterates `manifest.toml` or checks that every `out` exists; `include_bytes!` already fails the build when a PNG is missing. README credits need no change (existing GGG art bullet covers `assets/poe2/`). [VERIFIED: grep of `src/` and `tests/`]

### Slice helpers (`src/skin/slice.rs`)
- `snap`, `snap_rect(rect, ppp)`, `full_uv(size)` (half-texel inset), `paint_image(painter, tex, dest, tint)` (snaps, whole texture), `three_h_rects(dest, left_size, right_size, ppp) -> [Rect;3]` (caps keep aspect at dest height, middle stretched, shrink proportionally if too narrow via `cuts`), `paint_three(painter, [&l,&m,&r], rects, tint)`, `nine_slice`/`paint_nine`.
- **Bar:** `let rects = three_h_rects(bar_rect, bar.left.size(), bar.right.size(), ppp); paint_three(&painter, [&bar.left,&bar.middle,&bar.right], rects, WHITE)`. No new helper needed. The three cuts are snapped to physical pixels (no seams at 100-200% DPI).
- **New pure helper for "paint by visible plate"** (legacy PNGs that keep their gutter), house style with unit tests:
  ```rust
  /// UV of the texel box [x0,y0,x1,y1) of a texture of `size`, inset half a texel on each side.
  pub fn box_uv(size: [usize; 2], b: [u32; 4]) -> Rect
  pub fn paint_visible(painter: &Painter, tex: &TextureHandle, opaque: [u32; 4], dest: Rect, tint: Color32)
  ```
  `paint_visible` = `painter.image(tex.id(), snap_rect(dest, ppp), box_uv(tex.size(), opaque), tint)`. For the icon button plate `opaque = [0,0,84,84]` of 92x92. Light (2x2) textures make `box_uv` meaningless in headless tests, so test `box_uv` with explicit sizes.
- **Check boxes (`ff_check`, widgets.rs ~603):** `check_on/off` are 54x56, opaque `[0,0,54,54]`. Today `draw` is 22 x 22.8 so the visible plate is 22x22 at the rect's top-left and the gutter hangs below the allocated 22x22 rect. Switching to `paint_visible(.., [0,0,54,54], rect, tint)` is the ART-04-correct form (visible plate = the 22x22 rect, 0.8pt shorter art than now, nothing moves). Low risk; include it. `close_normal`, `combo_arrow` and `ff_arrow`: no offsetting gutter, leave as is.

### `ff_reload_button` (widgets.rs 1208-1275; call sites `config_view.rs:444`, `shell.rs:708`)
- Rect is `ROW_HEIGHT x ROW_HEIGHT` (34). Plate painted with `slice::paint_image(.., &skin.icon_button[state], rect, tint)`: the whole 92px texture goes into the rect, so the visible 84px plate is 31.3pt and sits up-left (the offset the author sees, glyph maths already assume `plate = ROW_HEIGHT * PLATE_VISIBLE` anchored at `rect.min`).
- Fix (matches UI-SPEC): plate via `paint_visible(.., [0,0,84,84], rect, tint)`; delete `PLATE_VISIBLE`; `plate = ROW_HEIGHT`; ring centre `rect.center() + vec2(0.0, 1.0 + pressed)` (was `rect.min + plate/2 + ...`); `side = plate * RING_OF_PLATE / RING_OF_GLYPH`, `dest = from_min_size(ring_centre - RING_CENTRE*side, splat(side))`, mirror and `-28` degrees unchanged.
- `PLATE_VISIBLE` is referenced only inside `ff_reload_button`. Existing tests (`reload_button_is_row_height_square`, `_clicks_only_when_enabled`, `_is_labelled_and_reports_enabled`) test rect/click/accessibility only, unaffected. `style_lines::probe("league-refresh", refresh.rect)` uses the unchanged rect.
- Both call sites go through this one function: Filter refresh (`ff_field_row`, `row_h`, an `egui::ComboBox` of `combo_w = available - row_h - gap`) and Economy league refresh (`ff_dropdown_ex` width `CONTROL_WIDTH - ROW_HEIGHT - FIELD_GAP`, then the button). No other user of `icon_button` art (`ed::ff_icon_button` is the minimap tile, different art). Verify both with screenshots (below).

### Text field internals (`src/skin/widgets.rs` ~1340-1410)
- `ff_text_field(ui, text, hint, width)` -> `ff_text_field_ex(ui, None, ...)`; `ff_text_field_id(.., id, ..)` -> `Some(id)`; `_ex` takes `font`.
- `_ex` allocates a 30pt-high rect (`Sense::hover`), reserves a `bg` shape slot, then `ui.put(rect.shrink2(vec2(10.0, 3.0)), TextEdit::singleline(..).frame(Frame::NONE).hint_text(RichText.color(DISABLED)).font(font).text_color(VALUE).vertical_align(Center))`, then `advance_cursor_after_rect(rect)` + `FIELD_GAP` extra spacing in horizontal layouts, then paints fill (`INPUT_FILL`) and 1pt border (`INPUT_BORDER` / `INPUT_BORDER_HOT` when the edit is hovered or focused) into the reserved slot. `Frame::NONE` means the TextEdit margin is 0, so text starts exactly at the put-rect's left edge (+10 today). The hint is painted inside the same rect, so shifting the rect shifts text and hint together.
- **Recommended structure:** refactor `_ex` to take a small `FieldInsets { left, right }` (default `10.0, 10.0`) or split a private `text_field_inner(ui, id, text, hint, width, font, insets) -> (Response /*edit*/, Rect /*field*/)`; `ff_search_field(ui, id, text, hint, width) -> Response` calls it with `left = 32.0, right = 32.0`, then draws glyph and X on top. Keep all existing callers (`ff_text_field*`) byte-for-byte equivalent (insets 10/10) so layout tests pass unchanged. Return the **edit** response (callers use `has_focus`/`lost_focus`/`request_focus`/`id`).
- "Hot" today = `resp.hovered() || resp.has_focus()` of the edit. For the magnifier hover art use `ui.rect_contains_pointer(field) || resp.has_focus()` (the glyph zone is outside the edit rect).
- **Glyph:** 16x16pt at `field.min.x + 10`, vertically centred; `paint_image(painter, &skin.search[usize::from(hot)], rect, WHITE)` (art already trimmed to the glyph, 48x49 aspect: paint into a 16x16 box, or 16 x 16.3, centre it; spec says 16x16). Its click zone: `ui.interact(zone, id.with("glyph"), Sense::click())`; on click `ctx.memory_mut(|m| m.request_focus(id))`. The zone must not overlap the edit rect (edit starts at +32, zone ends at +26..32).
- **Clear X:** shown while `!text.is_empty()`. 18x18pt disc, centre `field.max.x - 17`, hit rect 24x24 via `ui.interact(hit, id.with("clear"), Sense::click())`, created **after** the TextEdit so it is on top. States: `resp.is_pointer_button_down_on()` -> `clear[2]`, `hovered()` -> `clear[1]` (+ `ui.set_cursor_icon(PointingHand)`), else `clear[0]`. `.on_hover_text("Clear search")` and `widget_info(labeled(Button, true, "Clear search"))`. Not a tab stop: `ui.interact` with `Sense::click()` is focusable by default in egui 0.36, so use `Sense::click()` only if acceptable, otherwise use `Sense { focusable: false, ..Sense::click() }` (the spec says not a tab stop; verify the `Sense` field name in egui 0.36.1 before relying on it, it exists as `Sense::focusable` in recent versions [ASSUMED]).
- **Keep focus after clearing:** on click do `text.clear(); ui.ctx().memory_mut(|m| m.request_focus(id));`. The X runs after the edit in the same frame, so its `request_focus` wins over any surrender triggered by the click landing elsewhere. egui clamps the stale cursor range to the (now empty) galley on the next frame, so no `TextEditState` edit is strictly required; if the caret looks wrong in a screenshot, `TextEdit::load_state(ctx, id)` -> `st.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(0))))` -> `st.store(ctx, id)` (this exact pattern is already used for the section rename box, `rule_list.rs` ~786). Test with the existing click harness (`click_events`/`click_at`) plus a second frame and `ctx.memory(|m| m.has_focus(id))`.
- Reserve X space always: edit right inset 32 regardless of text (spec judgement 8).

### RULES toolbar search (`rule_list.rs` 1426-1536)
- `search_row` -> `search_row_rects(ui, skin, query, add_enabled, total)` returns `(click, field, add, section)`; row is a 34pt-high `left_to_right(Center)` child, `item_spacing.x = 8`; field `width = available - add_w - sec_w - 8 - FIELD_GAP`.
- Today: `ff_text_field(ui, query, &hint, width)`; `field` is reconstructed from `start_x`/`resp.rect.center().y` (30pt high); Escape (`resp.has_focus() || resp.lost_focus()` and `key_pressed(Escape)` and non-empty query) clears; a painted `Clear` text link (interact id `ui.id().with("ff-search-clear")`, text "Clear", right-aligned in the last 52pt) clears on click. **Delete that link block**, swap the call to `ff_search_field(ui, search_id, query, &hint, width)` with a stable id (e.g. `ui.id().with("rules-search")`), keep the Escape block, keep the `field` rect construction (the width/height contract is unchanged, the test `search_field_and_add_button_do_not_touch` asserts gap == `FIELD_GAP`, field width == `avail - add - section - 18`). No test references the "Clear" link or `ff-search-clear` (grep: only the production code).
- Escape-then-keep-focus: Escape handler leaves focus as today; unchanged.

### + condition picker search (`body.rs` `picker_popup` ~1290-1320)
- Inside `egui::Popup::from_toggle_button_response(button)...show(|ui| ...)`: `query` lives in `ui.data` under `query_id = popup_id.with("query")`; first-frame focus is requested via `r.request_focus()` guarded by `focus_id = popup_id.with("focused")`. Call: `widgets_skin::ff_text_field(ui, &mut query, hint_text, ui.available_width())` when `searchable`.
- Replace with `ff_search_field(ui, popup_id.with("query-edit"), &mut query, hint_text, w)` (an id **different** from `query_id`, which is a data key) and keep the focus-once logic on the returned response. The X clears `query` (then `ui.data_mut(insert_temp(query_id, query))` already follows). Escape still closes the popup (unchanged); the popup uses `CloseOnClickOutside`, and clicks on the X are inside the popup so they do not close it.
- **Test to update:** `body.rs` `picker_popup_uses_skin_search_and_sections` (~6239) asserts `body.contains("ff_text_field(")`; change to `"ff_search_field("`. The adjacent banned-widget test (`text_edit_singleline`, `selectable_label(` ...) scans `body.rs` and `widgets.rs` production source: do not introduce any banned token in the new code.
- Not search fields (verified by grep of every `ff_text_field*` call and every `hint`): rule Name, section rename, style name, filter-draft name, tier cutoffs, colour Hex/RGB, quoted-list `+ Add` fields (`overlay/widgets.rs:272,591`), raw rows (`body.rs:250`). `body.rs` `section_head` (`+ condition` picker's own group header with a painted triangle) is **not** a rule-list section header and is out of scope for ART-01.

### `draw_section_header` (`rule_list.rs` 627-845)
Current structure (what changes and what must not):
- `allocate_exact_size(vec2(width, 36), Sense::click())` gives `rect, resp` (whole bar is the click target; accessible name from `section_accessible_name`, which has no test dependency on the chevron). Keep.
- Paints: flat `pal::PANEL`/`ROW_HOVER` ground, 1pt `INPUT_BORDER` top and bottom lines, 1pt `INPUT_BORDER_HOT` focus stroke. Replace per UI-SPEC: bar art (three-slice) from `rect.min.x + 35` to `rect.max.x`, hover = `ROW_HOVER` wash clipped to the bar, focus stroke inside the bar rect.
- Right cluster: a child `Ui` (`id_salt("ff-section-right")`, `max_rect(inner)`, right-to-left, `item_spacing.x = 4`) built **first**; economy = padlock via `flag_icon(.., Padlock, MUTED, "locked", ECONOMY_TIP)`; user sections = `ff_x_button` (if `can_remove`), `ff_arrow` down, `ff_arrow` up (with "Clear the search to reorder" tooltips when `!reorder_enabled`), `icon_button(Rename)`. `used_left = right.min_rect().left()`. For the spec's `bar.max.x - 12` set `inner = Rect::from_min_max(pos2(bar.min.x, rect.min.y), pos2(bar.max.x - 12.0, rect.max.y))` (it is `rect.shrink2(vec2(ROW_PAD, 0))` today).
- Left: a second click target `chev` (`ui.interact(chev_rect, ui.id().with("chevron"), Sense::click())`, accessible name "chevron", tooltip Expand/Collapse) painted with `FlagIcon::ChevronRight/Down`. Replace by the plate: `plate_rect = Rect::from_min_size(pos2(rect.min.x + 4.0, rect.min.y + 2.0), vec2(27.0, 32.0))`, keep the interact + `widget_info("chevron")` + tooltip, paint `skin.toggle_expand[state]` when `collapsed`, else `skin.toggle_collapse[state]` with `state` = 2 if `resp.is_pointer_button_down_on()` (and the pointer is on the row), 1 if `chev.hovered() || resp.has_focus() || chev.has_focus()`, else 0, via `slice::paint_image`. `chev_clicked` still ORs into the toggle condition (`(resp.clicked() || chev_clicked) && !was_editing && matches.is_none()`).
- Title: `name_x = chev_rect.max.x + 8` today; becomes `bar.min.x + 60`. `slot_w`, `cut_galley` (small caps 17, `HEADING`), the count (`regular(15)`, `MUTED`, right-aligned at `used_left - 8`) and the inline rename `ff_text_field_id(.. "Section name" ..)` field (30pt, up to 220 wide, at `name_x`) keep working with the new `name_x`.
- The decorative rule line between title and count (`line_segment`, `INPUT_BORDER`, drawn when `line_to - line_from >= 16` and name not cut) is not covered by the UI-SPEC. **Recommendation: remove it** (the art is busy, the bar carries the frame); it is pure paint, no test asserts it. Flag in the SUMMARY.
- Painting order matters: bar art and hover wash first (under the right cluster and title), then count/name. The right cluster child Ui is created before the paint in the current code (so widgets interact first), but its widgets paint immediately; paint the bar **before** creating the right-cluster child, or use the `ui.painter().add(Shape::Noop)` reserve-then-set pattern the widgets already use (`bg` slot in `ff_text_field_ex`), otherwise the art covers the arrows/pencil. (Easiest: compute `bar` from `rect`, paint bar + wash + focus first, then build the right cluster, then plate and title.)
- Economy header: same function (`sec.economy`), padlock stays; the economy `collapsed` flag/default is passed in as `collapsed`.
- Constants: `SECTION_HEADER_HEIGHT = 36.0` asserted in a test (`rule_list.rs:2879`) and used by `entry_height` / virtualisation, so it must not change. Header click tests (`click_at(.. Pos2::new(200.0, 70.0)..)` at ~2899-2908, 3049, 3137) click mid-row at x=200 (the bar), unaffected by the plate; the economy-header tests at 3049/3137 keep working if the whole-bar `resp` stays.
- Pure, testable geometry (house style, see `tab_layout`): add `fn header_layout(rect: Rect, ppp: f32) -> HeaderLayout { plate, bar, title_x }` with unit tests (plate 27x32 at (+4,+2), bar from +35 to right edge, title at bar.min.x + 60, 36pt high, rects on physical pixels at ppp 1.0/1.25/1.5/2.0), and `fn search_layout(field: Rect, has_text: bool) -> SearchLayout { glyph, edit, clear_disc, clear_hit }` (glyph x +10..+26, edit `+32 .. max-32`, disc centre `max.x-17`, hit 24x24).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Header bar three-slice | custom stretch painting | `slice::three_h_rects` + `paint_three` |
| Pixel snapping at 100-200% DPI | manual rounding | `slice::snap_rect` / `snap` |
| Sub-box UVs | ad-hoc UV maths in each widget | one `slice::box_uv` / `paint_visible` |
| Substitute art | painted shapes | not needed, all four pieces exist |
| Stateful click-and-focus | custom focus tracking | `ctx.memory_mut(request_focus(id))` + explicit TextEdit id |

## Common Pitfalls

### Pitfall 1: `trim` does not remove left/top gutters
**What goes wrong:** `trim = true` on `buttoncollapse*` / `npcmenubarleft` leaves 4-5px / 19px of transparent margin at the top-left, so the "visible plate" is still offset.
**How to avoid:** `crop` only, with w/h; verify with `ff-art info` (`opaque=[0,0,w,h]`).

### Pitfall 2: crop values in the spec are min/max boxes
**What goes wrong:** `crop = [4,5,58,69]` would be read as w=58, h=69 and fail ("crop outside") or include gutter. Use `[4,5,54,64]`, `[19,0,132,75]`, `[6,4,48,49]`.

### Pitfall 3: Painting the bar over the header controls
Right-cluster widgets paint as they are laid out, so the bar and hover wash must be painted before the cluster is built (see above).

### Pitfall 4: Light textures in tests
`load_light` textures are 2x2; any size-derived geometry (cap widths from `tex.size()`) is wrong in those tests. Assert geometry through pure functions with real constants.

### Pitfall 5: Click on X loses focus
A click anywhere outside a TextEdit surrenders its focus; the X must `request_focus(id)` after the edit has been shown that frame (X drawn after the edit). Needs an explicit, stable id: the toolbar must not rebuild the id from a changing path, and the picker's edit id must differ from its `query_id` data key.

### Pitfall 6: Text reflow / hint overlap
The edit's right edge is `field.max.x - 32` always; the hint text is clipped inside the edit rect, so a long placeholder (`Search N rules, classes, base types…`) truncates earlier than today (22pt less width): acceptable at the 660pt window, but check the screenshot.

### Pitfall 7: Existing source-text tests
Update `body.rs` `picker_popup_uses_skin_search_and_sections` (`ff_text_field(` -> `ff_search_field(`). Check `body.rs` banned-token scan and `section_heads_use_skin_buttons` still pass.

### Pitfall 8: `ff_text_field_ex` refactor must not shift other fields
Seven other fields share it (names, tiers, hex, entry fields). Keep the default insets 10/10 and the `FIELD_GAP` logic untouched; the existing tests at `widgets.rs` ~1777 and ~2180 cover field rect and id.

## Code Examples

### Visible-box UV (new, in `slice.rs`)
```rust
pub fn box_uv(size: [usize; 2], b: [u32; 4]) -> Rect {
    let (w, h) = (size[0].max(1) as f32, size[1].max(1) as f32);
    Rect::from_min_max(
        pos2((b[0] as f32 + 0.5) / w, (b[1] as f32 + 0.5) / h),
        pos2((b[2] as f32 - 0.5) / w, (b[3] as f32 - 0.5) / h),
    )
}
```
`box_uv([92,92],[0,0,84,84])` = ((0.5/92, 0.5/92), (83.5/92, 83.5/92)); with a zero origin it equals `full_uv` on the min side.

### Bar painting (reuses existing helpers)
```rust
let ppp = painter.pixels_per_point();
let rects = slice::three_h_rects(bar, skin.bar.left.size(), skin.bar.right.size(), ppp);
slice::paint_three(&painter, [&skin.bar.left, &skin.bar.middle, &skin.bar.right], rects, Color32::WHITE);
```

## State of the Art

Nothing version-sensitive: egui 0.36.1 (locked), `TextEdit::frame(Frame::NONE)` gives zero margin (verified in `egui-0.36.1/src/widgets/text_edit/builder.rs`: `frame.unwrap_or_else(|| Frame::new().inner_margin(margin))`).

## Runtime State Inventory
Not a rename/refactor phase. None.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `Sense { focusable: false, .. }` exists in egui 0.36.1 for a non-tab-stop X | Text field internals | Compile error; fall back to `Sense::click()` (X becomes a tab stop) or `ui.interact` + `ui.memory_mut(|m| m.set_focus_lock_filter..)`. Check `Sense` in the registry source first |
| A2 | The header's rule line should be removed | draw_section_header | Cosmetic only; author review |

## Open Questions (RESOLVED)

1. **Rule line between title and count** — not in the UI-SPEC. Recommendation: remove; list in SUMMARY for the author's review. RESOLVED: removed (48-02 Task 2 step 5); recorded as a judgement call in the deferred list (48-04).
2. **ff_check visible-plate change** — spec lists it ("use the helper"); effect is 0.8pt less art height only. Recommendation: include with a unit test on `box_uv`. RESOLVED: included (48-01 Task 3) with a `box_uv` unit test.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| PoE2 install (Steam default path) | `cargo ff-art` | yes | - | none needed |
| Rust nightly | build, tests, ff-art | yes | cargo 1.100.0-nightly (2026-09-02) | - |
| Prebuilt `ff-art.exe` | extraction | yes | `...\Temp\claude\ffart-target\release\ff-art.exe` | `cargo ff-art` with `CARGO_TARGET_DIR` outside the repo |
| Prebuilt debug `ff.exe` (baseline, pre-change) | screenshots | yes | `...\Temp\claude\ffart-target\debug\ff.exe` (rebuild after changes, ~4 min cold, incremental faster) | - |

## Validation (workflow.nyquist_validation = false: brief, not a full section)

- Tests: `cargo test --lib` (plus `cargo test` for integration, `tests/isolation_guard.rs` requires env pinning on spawns). Add: pure-geometry unit tests (`box_uv`, `header_layout`, `search_layout`, reload plate/ring placement: ring centre = rect centre + (0,1)), a focus-after-clear test, the updated source-text test, decode-size asserts in `every_embedded_texture_decodes`.
- Visual check with debug screenshot mode (compiled out of release). Both env dirs must be under the temp folder; use literal Windows paths:
  ```
  FERRISFILTER_HOME=C:\Users\jonat\AppData\Local\Temp\claude\ffshot48\home
  FERRISFILTER_GAME_DIR=C:\Users\jonat\AppData\Local\Temp\claude\ffshot48\game
  ff.exe --screenshot <png> --tab config                       # Filter refresh button vs dropdown
  ff.exe --screenshot <png> --tab economy --leagues "A,B" --league A   # league refresh
  ff.exe --screenshot <png> --tab rules --filter tests/ui/test_ff.filter --search ring   # headers + X + glyph
  ff.exe --screenshot <png> --tab rules --filter tests/ui/test_ff.filter --expand 0 --picker rule ...  (+ condition popup via --expand)
  ```
  I ran the first and third form successfully against the current debug build (both PNGs written to `...\ffshot48\`). Use `ff-art zoom <png> x y w h out --zoom 4` to inspect up close; compare to `.planning/research/ui-refs/social-*.png`. Test at ppp 1.0 and 2.0 if the harness supports it (`window` DPI), watching for seams between bar slices.

## Security Domain

Local desktop UI, no network/auth/input-to-system paths touched. ASVS V5 (input validation): the search query is only used for in-memory filtering; the X only clears it. No new file/path inputs. No new dependencies. Nothing applicable beyond existing practice.

## Sources

### Primary (HIGH)
- Codebase: `tools/ff-art/src/main.rs` (ImageEntry fields, `extract`), `assets/poe2/manifest.toml`, `src/skin/{textures,slice,widgets,icons}.rs`, `src/overlay/{rule_list,body,config_view,shell}.rs`, `CLAUDE.md`, README Credits.
- Trial run of `ff-art` against the installed game (this session) and `ff-art info` output.
- `egui-0.36.1/src/widgets/text_edit/builder.rs` (margin/frame behaviour, focus on click).

### Secondary / Tertiary
None.

## Metadata

**Confidence breakdown:**
- Manifest entries / churn: HIGH (executed)
- Widget structure and test impact: HIGH (read code, grepped tests)
- egui focus-after-clear and non-tab-stop `Sense`: MEDIUM (A1; verify in a test)

**Research date:** 2026-10-23
**Valid until:** until the game patches the art or the skin files change (30 days)
