# Phase 31: RULES Tab on the Skin - Pattern Map

**Mapped:** 2026-10-16
**Files analyzed:** 14 (new/modified)
**Analogs found:** 14 / 14 (all in-repo; two are partial, see "Gaps")

Sources: 31-CONTEXT.md, 31-UI-SPEC.md. Line numbers are against the working tree at mapping time.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/filter/structure.rs` `append_rule` (modify: insert at top of file) | model / edit op | CRUD | itself (205-280) + `remove_block` (283-300) | exact |
| `src/overlay/rules.rs` `RowAction::AddRule` arm (modify, maybe keep) | model | CRUD | itself (1097-1103) | exact |
| `src/overlay/app.rs` `apply_row_actions` add/duplicate branch (modify: clear query, expand, scroll 0, new status) | controller | event-driven | itself (618-633) | exact |
| `src/overlay/apply.rs` (replace `added_hidden_message` with add-success const) | utility | transform | itself (216-219, test 844) | exact |
| `src/overlay/rule_list.rs` `search_row` (modify: width arithmetic, tooltip) | component | request-response | itself (804-835) + `ff-proto.rs::rules_tab` (392-405) | exact |
| `src/overlay/rule_list.rs` `draw_row` + `flags_for` + `fit_flags` (rewrite row) | component | event-driven | `ff-proto.rs::rule_row` (429-526) + own current `draw_row` (452-708) | exact |
| `src/overlay/rule_list.rs` `show_list` (swap thin scrollbar for game scrollbar, keep virtualization) | component | streaming (viewport) | `skin::widgets::ff_scroll_area` (1039-1115) + own `show_list` (357-450) | role-match (needs a viewport variant) |
| `src/overlay/rule_list.rs` `expanded_body` (two-column editor shell, ff_text_field NAME) | component | request-response | `ff-proto.rs::rule_body` (528-587) + `skin::widgets::ff_row` (460-502) | role-match |
| `src/overlay/widgets.rs` (wrappers become delegates) | component (legacy wrapper) | request-response | `skin::widgets::{ff_check, ff_slider, ff_dropdown, ff_text_field}` + `skin::editor::ff_segmented` | role-match (signatures differ: skin widgets need `&Skin`) |
| `src/overlay/body.rs` (card chrome, headers, combos, pickers) | component | request-response | itself: `card_frame` (275), `condition_card` (1466), `condition_header` (1544), `action_card` (2560), `style_section` (2458), `picker_popup` (1118) | exact (central seams) |
| `src/skin/icons.rs` (flag icon fixes: link, padlock; 16pt glyph, 1.75 stroke) | component (painter) | transform | itself `paint_flag_icon` (37-110) | exact |
| `src/skin/widgets.rs` / `src/skin/editor.rs` (additive: chip, value field, scroll-viewport, quoted-list helper) | component | request-response | `ff_segmented`, `ff_icon_button`, `ff_card` (editor.rs 25, 307, 361) | role-match |
| `src/overlay/screenshot.rs` + `app.rs::new_screenshot` (extra RULES shots: expanded row, popup) | utility | event-driven | `screenshot.rs` args (45-87) + `app.rs` 805-870 (`--popup delete-rule` seeding at 841-847) | exact |
| `CLAUDE.md`, `CHANGELOG.md` | docs | n/a | existing overlay notes | exact |

## Pattern Assignments

### 1. Add rule at top of file: `src/filter/structure.rs::append_rule` (model, CRUD)

**Analog:** itself. Today it computes `ins` = just above the economy section (or end of file) at 235-246, builds `new` from `self.lines[..ins]`, pads blanks, inserts `lines`, then re-ids via `infos`/`expected`/`commit_structural`.

Current insertion-point code to replace (lines 235-268):

```rust
let sec = section::find(self).map_err(|e| EditError::EconomyDamaged(e.to_string()))?;
let total = self.lines.len();
let ins = match sec {
    Some(s) => { let mut i = s.begin; while i > 0 && is_blank(&self.lines[i - 1]) { i -= 1; } i }
    None => total,
};
let eol = if ins > 0 { self.fresh_eol(ins - 1) } else { self.last_eol() };
let mut new: Vec<Line> = self.lines[..ins].to_vec();
if let Some(last) = new.last_mut() { if last.eol() == Eol::None { *last = Line::new(last.text().to_string(), eol); } }
if new.last().is_some_and(|l| !is_blank(l)) { new.push(Line::new(String::new(), eol)); }
for t in &lines { new.push(Line::new(t.clone(), eol)); }
if sec.is_some_and(|s| ins == s.begin) { new.push(Line::new(String::new(), eol)); }
new.extend_from_slice(&self.lines[ins..]);
```

and the id bookkeeping (269-279), which already handles "insert at position `pos`" generically:

```rust
let infos = self.infos();
let pos = infos.iter().filter(|i| i.header < ins).count();   // becomes 0 for top-of-file
let mut ids: Vec<Option<BlockId>> = infos.iter().map(|i| Some(i.id)).collect();
ids.insert(pos, None);
let mut expected: Vec<Vec<String>> = infos.iter().map(|i| self.span_texts(i.span.clone())).collect();
expected.insert(pos, lines);
let out = self.commit_structural(new, ids, expected)?;
Ok(out[pos])
```

Planning notes (decisions the planner must settle):
- "First block of the file" is not line 0 when the file starts with a leading comment/header (e.g. `# GENERATED...`, `#ff:` meta of block 1, a foreign preamble). The new block must go **before the first block's `lead_start`** (its leading comments/meta belong to that block; see `c.lead_start` use at 218) so the first block keeps its own `# ff:name` / meta lines. Use `self.infos().first()` span/`unit.start` for `ins` rather than 0 when blocks exist; with no blocks (comment-only or empty file) keep the append-at-end behaviour. Add a test for each.
- Keep the economy-damaged check (`section::find(self).map_err(...)`, 235) so `append_rejects_bad_input_and_damaged_economy` (structure.rs:587-610) stays valid; the economy section is no longer needed to compute `ins`.
- Blank-line padding: after the new block insert exactly one blank line before the old first line (mirror the `if sec.is_some_and(|s| ins == s.begin) { push blank }` idiom at 264, now unconditional when `ins` is above existing content). EOL: use `self.fresh_eol(ins)` style helper (`fresh_eol`/`last_eol` already exist).
- **Grouped rules (`# ff:group`)**: inserting above block 1 can never land *inside* a run (the run is consecutive blocks; top-of-file is before all), but a run whose first member is block 1 must be unchanged. Test: file starting with a 2-block `# ff:group x 1/2`,`2/2` run keeps the run intact and the new rule is a separate single block. UI-SPEC "never lands inside a group" is therefore satisfied structurally.
- A disabled (commented) first block: `infos()[0].unit.start` still points at its `# Show` line; the new block goes above it.
- Rename candidate: keep the method name `append_rule` only if the planner wants minimal churn (callers: `rules.rs:1097`, `tests/filter_structure.rs:188,227`, structure.rs tests 531-610). A new `insert_rule_at_top` with `append_rule` removed is cleaner; Phase 32 will "reuse the single insertion point".

**Tests that must be rewritten (they assert append-at-end/before-economy):**
- `src/filter/structure.rs`: `append_basic_and_fresh_id` (531-538, expects `"Show\n    Class \"Rings\"\n\nShow\n"` and `ids == [before[0], id]`), `append_before_economy_section` (541-568, two sub-cases), `append_after_trailing_comment_and_edge_inputs` (571-584: `"Show\n# note\n"`, empty file, CRLF `"Show\r\nHide"` cases), `append_rejects_bad_input_and_damaged_economy` (587-610; stays valid, only strings/ordering check).
- `tests/filter_structure.rs`: `append_leaves_existing_bytes_alone` (183-197: asserts `d.blocks().last()` is the new id, `appended[..bytes.len()] == bytes` untouched prefix, and after `remove_block` bytes start with the original). Both invert: suffix is now untouched and the new block is `blocks().next()`. `v04_dynamic_economy_is_never_touched` (212-240): asserts the new rule's header is before the economy begin and "appended rule is now the last rule... cannot move into the economy" (228-235); the last-rule/move-into-economy assertions need rework (the new rule is now first, so `move_neighbour(new, true)` is the boundary case instead).
- `src/overlay/rules.rs`: `add_rule_appends_before_economy` (1977-1987) asserts `row.label == "2  Show"`; becomes `"1  Show"` and rename the test. The `with_economy()` fixture and the `EconomyView::Present { blocks: 2 }` assertion remain valid. Also `rules.rs:2114` (revert/on_applied test using `AddRule`) should keep passing unchanged.
- Callers that only call `AddRule` to make the document dirty (app.rs 3291, 3389, 3418, 3445, 3489, 3501, 3540, 3559, 3619, 3774, 3797, 4028) stay valid but check any that assert text equality against a tail position.

---

### 2. App-side add behaviour: `src/overlay/app.rs::apply_row_actions` (controller, event-driven)

**Analog:** itself, 541-640. Current add branch:

```rust
if let Some(focus) = outcome.focus {
    let hidden = is_add && !query.trim().is_empty() && !working.row_matches(focus, query);
    if hidden {
        if let Some(row) = working.row(focus) { status = Some(apply::added_hidden_message(row.index)); }
    } else if is_add || is_duplicate {
        list.expanded = Some(focus);
        list.pending_scroll = Some(focus);
    } else if is_move {
        list.pending_scroll = Some(focus);
    }
}
```

Change for UI-SPEC "Add Rule": `apply_row_actions` currently takes `query: &str` (immutable). Make it `&mut String` (or return a "clear query" flag; the existing `jump_to` path clears the query at app.rs ~2227-2233, copy that). On add: clear the query, `list.expanded = Some(focus)`, `list.pending_scroll = Some(focus)` (index 0 so `scroll_offset_for` clamps to 0), set a one-shot "focus NAME field" flag on `ListUi` (analog: `list.focus_line` at 603 / `focus_card`), and set `status = Some(apply::RULE_ADDED_MESSAGE)`. Remove the `hidden` branch.

Status const home: `src/overlay/apply.rs` next to `added_hidden_message` (216-219). Replace with `pub const RULE_ADDED_MESSAGE: &str = "Rule added at the top of the file. Press Apply to save it.";` and update the unit test at apply.rs:843-846.

**Tests to rewrite:** `app.rs::adding_a_rule_the_search_hides_says_so_and_does_not_expand_it` (3661-3672; expects status `"Rule added at position 3 (hidden by the current search)"`, `expanded == None`, `pending_scroll == None`). New expectation: query cleared, `expanded == Some(new id)`, status = new message. Also `app.rs:550` (`is_add`) and the `body.rs:5035` doc comment (mentions AddRule resets; update wording only). `apply.rs:729/749` assert `add_rule` button enablement and are unaffected. `app.rs:4028` (config_message cleared on row actions) unaffected.

Re-key by identity: `ListUi` fields are keyed by `BlockId` (`expanded`, `pending_scroll`, `name_drafts`, `confirm_delete`), so shifting indices needs no work; only `rekey_after_apply`/`kept_positions` (rule_list.rs 150-196) are index-based and run after Apply. Verify `kept_positions` still behaves (test `rekey_after_apply_carries_expanded_and_defences_markers_by_position`, rule_list.rs:1324).

---

### 3. Toolbar: `rule_list.rs::search_row` (component, request-response)

**Analog:** itself (804-832) and `ff-proto.rs::rules_tab` (392-405). Current code reserves `132.0` (button 124 + gap 8) but the observed bug (30-UI-REVIEW #2) is that the field and button touch; the horizontal's default `item_spacing` is overridden to 8 inside the closure but the `ff_text_field` `allocate_exact_size` happens before `ff_button_ex` and `Clear` adds 56 when typing. Excerpt to keep/adjust:

```rust
ui.horizontal(|ui| {
    ui.spacing_mut().item_spacing.x = 8.0;
    let reserve = if query.is_empty() { 0.0 } else { 56.0 };
    let hint = format!("Search {total} rules, classes, base types…");
    let width = (ui.available_width() - 132.0 - reserve).max(40.0);
    let resp = ff_text_field(ui, query, &hint, width);
    ...
    let add = ff_button_ex(ui, skin, "+ Add rule", 124.0, add_enabled, false).on_hover_text(ADD_RULE_TIP);
```

Changes: spec says shared 34pt row (field 30, button 34 art) with explicit centring (`ui.allocate_ui_with_layout(vec2(w, 34.0), Layout::left_to_right(Align::Center), ..)`), `Clear` rendered *inside* the field's right edge (overlay on the field rect, like `ff_text_field`'s own `ui.put`), tooltip `ADD_RULE_TIP = "Adds a new rule at the top of the file."` (rule_list.rs:835; no test pins the old text, grep confirmed only line 828). Caller: `app.rs::rules_content` 1909-1914 (`ui.add_space(6.0)` after → 8.0 per spec).

---

### 4. Collapsed row: `rule_list.rs::draw_row`, `flags_for`, `fit_flags` (component, event-driven)

**Analog (look):** `ff-proto.rs::rule_row` (429-526). **Analog (behaviour, keep):** current `draw_row` (452-708).

Prototype layout to copy (painted text inside one allocated click rect; no nested labels per cell):

```rust
let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
w::row_background(ui, i, rect);                     // ROW_EVEN / ROW_ODD / ROW_HOVER (skin::widgets 447)
if expanded { ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, pal::INPUT_BORDER_HOT), egui::StrokeKind::Inside); }
let mut row = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(w::ROW_PAD, 0.0))).layout(Layout::left_to_right(Align::Center)));
w::ff_check(&mut row, skin, &mut rule.enabled, "Enabled");
let mut right = row.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(w::ROW_PAD, 0.0))).layout(Layout::right_to_left(Align::Center)));
right.spacing_mut().item_spacing.x = 4.0;
w::ff_arrow(&mut right, skin, false, can_down); w::ff_arrow(&mut right, skin, true, can_up);
let p = ui.painter().with_clip_rect(Rect::from_min_max(pos2(text_left, rect.min.y), pos2(text_right, rect.max.y)));
p.text(pos2(x, l1), Align2::LEFT_CENTER, name, fonts::small_caps(16.0), fade(pal::LABEL));
```

Adaptations (UI-SPEC wins over the prototype): row height **52** (const replacing `header_height + SUMMARY_LINE_H` in `row_height`, rule_list.rs 205-218; `debug_assert!` at 651 and the `row_height(ui)` signature taking `&Ui` can drop the ui arg or keep it); 28pt right-aligned index slot; 44pt Show/Hide slot in `small_caps(17)` `pal::LABEL`; name `small_caps(17)` `pal::HEADING`; summary `regular(15)` `pal::MUTED` at row top + 38, indent = index end + 8 (replaces `SUMMARY_INDENT = 40.0`, line 213); disabled row uses `pal::DISABLED`.

Behaviour to carry over unchanged from current `draw_row`:
- Toggle semantics + tooltips (605-613): `widgets::circular_toggle(&mut left, &mut enabled)` → `ff_check`; `RowAction::SetEnabled(row.id, enabled)`; blocked tooltip `"Can't enable: the commented lines are not valid filter grammar."`.
- Arrow disabled-while-searching (485-507): `reorder_enabled && row.can_move_down/up`, hover text `"Clear the search to reorder"`, and `list.pending_scroll = Some(row.id)` after a move.
- Click toggles expansion; clears `list.confirm_delete` (641-644). Current code has THREE click sinks (label, `row_click`, summary `sresp`) because labels swallow clicks; with a single painted `Sense::click()` row rect only one is needed. Test `clicking_the_summary_text_expands_the_row` (rule_list.rs ~1193-1215) uses `SUMMARY_INDENT` and `row_h + header + SUMMARY_LINE_H / 2.0` for the click point; update to the new constants.
- Expanded-height measurement (694-706): `body.response.rect.height()` + header height cached in `list.expanded_h`, `request_repaint` on change. Keep: the expanded row = 52 header + measured editor height. Frame fill `theme::PANEL_GROUND` + `Margin::same(8)` becomes the spec's `ROW_EVEN` ground with 10pt side padding.
- Per-row ID: `ui.push_id(("rule", row.id, list.epoch), ..)` (show_list 432).

Flag cluster (`FlagKind`, `Flag`, `flags_for` 221-354; `fit_flags` 258-275). `flags_for` output must stay identical per spec ("Keep flags_for output identical") except the Styled chip colour. Existing excerpt for the flag the spec edits:

```rust
if row.has_continue { out.push(Flag { text: "Continue".into(), kind: FlagKind::ContinueWord, color: pal::GOLD, tip: "Matching goes on to the rules below".into() }); }
if let StyleState::Known(tag) = &row.style { ... out.push(Flag { text: format!("styled: {short}"), kind: FlagKind::Styled(short), color: pal::GOLD, tip: format!("Style \"{tag}\" sets this rule's actions on Apply") }); }
```

- Order change: spec wants styled chip LEFT of the Continue tag. Because the cluster paints right-to-left with index 0 nearest the arrows, push order must become `... moved-across-Continue, Continue, styled` (already the present order: Continue pushed before styled) so styled ends up to the left. Verify; the existing test `flags_carry_kinds_tips_and_paint_order` (rule_list.rs 1032-1077) pins this order and the `color`/`FlagKind` values; update only the Styled colour (`pal::LABEL`) and keep text order. Add an 8pt gap between the Continue word and the styled chip (currently `right.spacing_mut().item_spacing.x = 4.0` at 508 applies uniformly).
- Styled chip currently paints a polygon diamond `DIAMOND_W` 14 (561-572); spec says use the `skin.slider_thumb` texture tinted `pal::LABEL`. This needs the `&Skin` to reach `draw_row` (it is not passed today: `show_list(ui, working, visible, list, reorder, classes)`). Add a `skin: &Skin` parameter (callers: `app.rs::rules_content` 1950, the rule_list tests' `run_frame` 940-953, and `painted_row_heights_match_the_virtual_list` which calls `draw_row` directly at ~1159). Headless tests have no Skin: `config_view` solved this with `skin: Option<&Skin>` (config_view.rs:164, `section_title` 255-266). Copy that `Option<&Skin>` + painted fallback pattern for the row so existing headless tests keep running; also `Skin::load(&ctx)` works headless (skin/editor.rs test 330-345 does `fonts::install(&ctx); Skin::load(&ctx)`).
- Budget arithmetic at 532-534 (`budget = FLAG_BUDGET.min((inner.width() - arrows_w - 24.0 - 8.0 - LABEL_MIN_W).max(0.0))`) must account for the new left slots (check 22 + index 28 + Show/Hide 44 + 3x8 gaps). Extract into one pure fn (spec: "one tested fn") so it is testable without egui. `fit_flags(widths, budget)` itself is pure and its test `fit_flags_collapses_overflow_into_a_badge` (1024-1030) uses 4pt-free widths; spec adds gaps between icons: either bake the 4pt gap into each width passed in (as `ContinueWord` already adds `+ 4.0` at 519) or add a `gap` param and update the 4 assertions.
- Icon fixes live in `skin/icons.rs` (section 8 below). `flag_icon` call: `flag_icon(&mut right, *icon, flag.color, &flag.text, &flag.tip)` (547).

**Existing tests touching row layout (need review/updating):** `fit_flags_collapses_overflow_into_a_badge`, `flags_carry_kinds_tips_and_paint_order`, `crowded_row_renders_with_overflow_badge` (1080-1089), `only_visible_rows_are_drawn_for_511_rules` (1092-1105, uses `row_h` from `row_height(ui)`), `collapsed_row_height_includes_summary_line` (1107-1115: asserts `row_height == header_height + SUMMARY_LINE_H`; delete or replace with `== 52.0`), `expanded_row_height_excludes_summary` (1118-1142), `painted_row_heights_match_the_virtual_list` (1144-1190; keep, it is the key virtual-list invariant), `clicking_the_summary_text_expands_the_row` (1193-1218), `arrows_do_nothing_when_reorder_is_disabled_and_work_when_enabled` (1220-1253; click coordinates assume the arrow positions, uses `460x600` screen), `style_flags_known_truncated_unknown_and_none` (998-1018, STYLE_FLAG_CHARS 10), `broken_group_blocks_carry_the_flag` (979-996), `expanded_style_tagged_row_renders_without_actions` (1449+), plus the source-text assertions `expanded_body_orders_name_conditions_style` (1384-1397: greps `caption(ui, "NAME")`, `body::conditions_group`, `body::style_section` in the source of `fn expanded_body`, so renaming/replacing `caption(ui, "NAME")` with a two-column label call breaks it, update the needle) and the "no circular_toggle / name_open" greps at 1442-1446. Test harness width is `460`, narrower than the 535 content; consider widening to 535 in `input()`/`run_frame` since the budget arithmetic is width-sensitive.

---

### 5. Virtual list + scrollbar: `rule_list.rs::show_list` (component, streaming/viewport)

**Analogs:** own `show_list` (357-450, `ScrollArea::vertical().id_salt("ff-rule-list").auto_shrink([false,false]).show_viewport`) and `skin::widgets::ff_scroll_area` (1039-1115, closure-based, `ScrollBarVisibility::AlwaysHidden`, paints the thumb from `out.state.offset` / `out.content_size`).

Gap: `ff_scroll_area` calls `.show(&mut child, add_contents)`, which gives no viewport rect, but virtualization needs `show_viewport`. Add a sibling `ff_scroll_area_viewport(ui, skin, id_salt, |ui, viewport: Rect| ..)` in `skin/widgets.rs` (additive; copy the body of `ff_scroll_area` changing `.show` to `.show_viewport` and keeping the track/thumb paint block lines 1066-1114 verbatim, ideally factor the paint into a shared private fn). Also allow a programmatic offset (`.vertical_scroll_offset(..)` as at rule_list.rs 406-410) for `pending_scroll`. Content width after the scrollbar is `outer.max.x - SCROLLBAR_WIDTH - 4.0` (1048), so `show_list`'s `let width = ui.available_width()` (420) automatically measures inside it.

Keep untouched: `virtual_list.rs` (`build_offsets`, `visible_range`, `scroll_offset_for`) and its tests; the `ui.skip_ahead_auto_ids(range.start)` + `scope_builder` trick at 425-427; `list.expanded_h` bookkeeping. `only_visible_rows_are_drawn_for_511_rules` is the regression guard for responsiveness.

Persist scroll across tabs/hide: `id_salt` already fixes the egui scroll-state id; no extra work unless the planner wants it explicit.

Callers: `app.rs::rules_content` 1933-1961 creates the list child ui; the Config tab already wraps in `w::ff_scroll_area(ui, skin, "config", ..)` (app.rs:1883) and Economy/About do too (shell.rs:430, 473): copy that call shape.

---

### 6. Expanded editor shell: `rule_list.rs::expanded_body` + `body.rs` (component, request-response)

**Analog (look):** `ff-proto.rs::rule_body` (528-587) using `w::ff_row(&mut body, idx, label, |ui| ..)`; `skin::widgets::ff_row` (460-502) is label-left/control-right but its control column is a fixed right-aligned `CONTROL_WIDTH` (252) strip. The spec wants the control to START at `label_end(128) + 16` and fill the remaining width, so `ff_row` itself does not fit. Create a sibling `ff_field_row(ui, index, label, |ui|)` (or `ff_row_ex` with a mode) in `skin/widgets.rs`, copying the `ff_row` skeleton:

```rust
let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
row_background(ui, index, rect);
// label: small_caps(17) pal::LABEL clipped at control_min - 4, vertically centred
let control = Rect::from_min_max(pos2(rect.min.x + ROW_PAD + 128.0 + 16.0, rect.min.y), pos2(rect.max.x - ROW_PAD, rect.max.y));
let mut child = ui.new_child(UiBuilder::new().max_rect(control).layout(Layout::left_to_right(Align::Center)));
```

Caveat: `ff_row` allocates a fixed 34pt row, but several editor controls are taller (quoted lists ~7 rows, Between card, Simple Defences wrapped line, class picker). Provide a variant that allocates the row AFTER measuring (use `ui.scope`/`new_child` with `max_rect` of unbounded height, then `allocate_rect` the used rect and paint `row_background` behind it, exactly the trick in `ff-proto.rs::rule_body` 581-586 for the body height: `let used = body.min_rect(); ui.allocate_rect(..)`; paint the ground with a `painter.add(Shape::Noop)` placeholder then `.set(..)` as `ff_text_field` does, skin/widgets.rs 906-925).

**Central seams in `body.rs` (introduce the two-column grid here, not per control):**

1. Label+control line for every condition: `condition_header` (1544-1570):
```rust
ui.horizontal(|ui| {
    ui.add_sized([LABEL_W, 24.0], egui::Label::new(RichText::new(label_text).size(14.0).color(theme::VALUE_TEXT)).truncate()).on_hover_text(label_hover);
    middle(ui);                                   // operator controls
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| widgets::remove_x_button(ui, remove_tip)).inner
}).inner
```
   `LABEL_W` is already a fixed label width constant (body.rs top; value to be set to 128). Convert to the two-column row: label at left (small_caps 17, `pal::LABEL`), control column right, remove X right-aligned in the control column. Called from `condition_card` (1466-1541) and the Between/Defences paths.
2. Card chrome: `card_frame(idx)` (275-282) is the single frame (fill `ROW_BASE/ROW_ALT`, `Margin::same(8)`), used by `condition_card`, `action_card`, `action_rows` raw rows (2548), `defences_card`, `between_card`, `simple_defences_card`, `pending_card`. Change this one fn to `pal::ROW_EVEN/ROW_ODD` + 10pt side padding.
3. Action cards: `action_card` (2560-2652) has the same label/X header (2582-2623) → same row helper. Locked-style badge variants (`widgets::badge`, 2597-2616) are legacy; keep (presets hide actions per CLAUDE.md, but "from style" locks exist).
4. Section heads: `conditions_group` (370-515) draws `caption(ui, "CONDITIONS")` + right-aligned `+ condition` `egui::Button`; `style_section` (2458-2526) draws `caption(ui, "STYLE")` + `+ style`. Replace buttons with `ff_button_ex(.., "+ condition", 132.0, ctx.editable, false)` (signature: `ff_button_ex(ui, skin, text, min_width, enabled, danger)`, skin/widgets.rs 807). `picker_popup` takes `&egui::Response` of the button and uses `egui::Popup::from_toggle_button_response(button)` (1135), which works unchanged with the `ff_button_ex` response (same pattern as `ff_dropdown` 743).
5. `Continue to later rules` is `egui::Checkbox` (style_section 2501-2512) → `ff_check` + label (`ff_check(ui, skin, &mut cont, "Continue to later rules")`; tooltip `Matching goes on to the rules below`). Test `continue_is_the_last_body_line` (body.rs:3578) asserts ordering by click y; will need re-tuning when positions change.
6. Delete: `expanded_body` already only sets `list.confirm_delete = Some(id)` (rule_list.rs 795-797) and the skinned "Delete this rule?" popup is already wired (app.rs 1381-1425, 1508-1511). CONTEXT said "inline confirm may become the popup": it already is; only the `small_button` Duplicate/Delete (711-713, `egui::Button`) need to become `ff_button(.., "Duplicate", 112.0)` / `ff_button_ex(.., "Delete", 112.0, true, true)`.

Skin access: `BodyCtx` (rule_list.rs 757-763: `{ row, entries, editable, classes, working }`) and `Card` have no skin. Add `skin: Option<&'a Skin>` to `BodyCtx`/`Card` (same optional-skin pattern as `ConfigViewCtx.skin`, config_view.rs:164) and to `ListUi`-free call chain `show_list → draw_row → expanded_body`. Alternative that avoids threading: store the `Skin` in egui memory once (`ctx.data_mut(|d| d.insert_temp(Id::NULL, skin.clone()))`) and fetch it inside the wrappers; the spec requires "same signatures, same call sites", which makes this the lower-churn route for `themed_slider`/`toggle_chip`/`circular_toggle` (none of which take a `&Skin` today). `Skin` is `Clone` (`self.skin.clone()` at app.rs:2331). Decide this once; it also fixes headless tests (no skin set → fall back to the legacy painted look, or `Skin::load` in the test ctx).

---

### 7. Legacy wrappers → skin delegates: `src/overlay/widgets.rs` (component)

**Analog target signatures** (skin/widgets.rs):
- `ff_check(ui, skin, &mut bool, label) -> Response` (509): replaces `circular_toggle(ui, &mut bool)` (115-146). Both flip the bool and `mark_changed()` on click. Wrapper: `pub fn circular_toggle(ui, checked) -> Response { ff_check(ui, &skin_of(ui), checked, "Enabled") }`. The old one expands the hit rect by 4; `ff_check` is a 22pt exact `Sense::click` rect (no expansion), so tests that click near the old 16pt circle (`body.rs` `effect_control` 2972, `optional_bool_control` 3000, `defences_card` 574) may need position nudges.
- `ff_slider(ui, skin, &mut f64, range, step, label, format) -> Response` (563): replaces `themed_slider(ui, &mut i64, min, max, log)` (372-409) which uses `egui::Slider` with `clamping(Never)` and `logarithmic(log)`. Adapt: copy value into `f64`, call `ff_slider`, write back `as i64` on `changed()`. Differences to handle: (a) `ff_slider` snaps/clamps to `range` (snap_value clamps) so a hand-written out-of-range value is clamped only on interaction, not on display (display uses `t.clamp`), but `Home/End/arrow` and clicks clamp; acceptable but the doc on `themed_slider` ("value is never clamped to min..=max (a hand-written out-of-range value must survive)") must hold: only write back when `resp.changed()`. (b) no `log` mode in `ff_slider`: `grammar::slider_range(name)` returns `(min,max,log)` (used at body.rs 1693, 934); if `log` is needed (e.g. ItemLevel? StackSize), map through `ln` before/after or add a `log` option. (c) `ff_slider` paints its own value text at the right ("format(v)", 673-679), so `number_field` beside it duplicates the number. UI-SPEC wants "the exact number as a value field right-aligned at the slider end (44pt, editable on click)". Add a `format`-less variant or pass `|_| String::new()` and keep `widgets::number_field` (429-468) restyled with `regular(15)`/`pal::VALUE` and the `ff_text_field` input-box look. (d) `ff_slider` takes `ui.available_width().max(120.0)`; inside `ui.horizontal` it will swallow the whole line, so callers (`int_control` 1683-1711, `bound_control` 920-947, `slider_pair` 2696-2709, `count_names_control` 2096, `sockets_control` 2136) must reserve the number field width first (allocate the slider in a child with `max_rect` = remaining minus 44+8).
- `ff_dropdown(ui, skin, id_salt, &mut usize, &[&str], width) -> Response` (703): replaces every `egui::ComboBox::from_id_salt(..).selected_text(..).show_ui(.., |ui| selectable_label..)`. 10 call sites in body.rs: 856 `numeric_op_combo`, 902 `between_op_combo`, 1348 pending op, 1625 `operator_combo`, 1719 `enum_control`, 2428 `style_picker`, 2861 `colour_combo`, 2915 minimap size, 2935 minimap shape, plus sound control. Pattern per site: map choices to a `Vec<&str>` labels + current index, call `ff_dropdown`, translate index back. Current example to convert (body.rs 1620-1637, `operator_combo`) returns `Option<Option<Operator>>` only when a different operator is picked; keep that return contract so `op_controls` and tests do not change. Beware `ff_dropdown` requires `options: &[&str]`, and `style_picker` (2413-2456) has an "unknown style" case flagged on the picker with a tooltip (render text `pal::WARN` per spec: needs an optional colour param or a wrapper that overpaints; additive change).
- `ff_text_field(ui, &mut String, hint, width)` (901): replaces bare `egui::TextEdit::singleline` at rule_list.rs 746-750 (NAME), body.rs `draft_field` (188-235), `picker_popup` search (1143-1147), `quoted_list` / `string_list` add field (widgets.rs 780+), `custom_sound_control` (2799). `ff_text_field` fixes height 30 and `put`s the edit inside `rect.shrink2(10, 3)`; it returns the TextEdit `Response`, so `.changed()`, `.lost_focus()`, `has_focus()` all still work (used the same way in `search_row` 817-823). Gotcha: it uses auto ids from `ui.put`; callers that set `.id(..)` for focus (`number_field` uses `.id(id)`, widgets.rs 444-448) need an id-aware variant (add `ff_text_field_id` or make the id a param).
- `ff_segmented(ui, skin, &mut usize, &[&str], width)` (editor.rs 25): replaces `bool_yes_no` (body.rs 1605-1618, two `toggle_chip`s with `r.on_hover_text("Remove the condition to match either.")`) and `disposition_control` Show/Hide (2350-2379), `bool_or_name_control` Any/Named (2163-2164), CONFIG `segmented` helper (config_view.rs 266-280). Wrapper keeps the `Option<bool>` return contract: map `effective` to index (Yes=0, No=1), call, translate on `changed()`.
- `toggle_chip` (305-349, 24pt chip, `FontId::proportional`): stays as the multi-select pill used by Rarity chips (`rarity_chips` 1759), Simple Defences (736) and enum multi (1743). Restyle in place to spec chip: 28pt high, `small_caps(17)`, pad 12, off = `pal::INPUT_FILL`/`INPUT_BORDER`, on = `pal::GOLD` 1pt border + `GOLD` alpha 60 fill, text `pal::HEADING`. No signature change; this is the natural "thin delegate" case needing no Skin.
- `slide_switch` (152-193): used by CONFIG style visibility switches (config_view.rs, 12 matches of wrappers there). Spec only lists it for Yes/No or Show/Hide pairs; CONFIG is Phase 34, so leave `slide_switch` alone or delegate to `ff_segmented` only where RULES uses it (grep shows none in body.rs, only config_view).
- `remove_x_button` (268-281): spec says `ff_icon_button` x, but `skin::editor::ff_icon_button` is the minimap shape tile (takes `shape, colour`), NOT an X button. There is no skinned X today: restyle `paint_x` (283-301) with `pal` colours (`pal::MUTED` → `pal::HEADING` on hover), keeping the 24x24 hit rect and the "right-aligned" alignment test `condition_x_buttons_line_up` (body.rs 3492-3517). Treat as additive painting change, not a delegate.
- `arrow_button` (213-264): used by rule_list.rs:485-494; replace with `ff_arrow(ui, skin, up, enabled)` (skin/widgets.rs 869). Note argument order `(up: bool, enabled: bool)`.

**Skin colour constants:** wrappers still use `theme::*` (overlay/theme.rs), which per CLAUDE.md "now point at the skin palette until Phase 35 removes that module". New code should import `crate::skin::palette as pal` directly (as `rule_list.rs` already does) and `crate::skin::fonts::{regular, small_caps}`.

---

### 8. Icons: `src/skin/icons.rs::paint_flag_icon` (component, painter)

**Analog:** itself, 37-110. Spec asks: all glyphs at 16pt inside the 18pt hit rect, 1.75pt stroke (`STROKE` const, line 17: 1.5 → 1.75; the glyph size is `r = rect.width().min(rect.height()).min(14.0)/2.0` at line 39 → raise cap to 16.0; `flag_icon` passes `rect.shrink(2.0)` at 119 = 14pt, so shrink by 1.0 for a 16pt glyph). Group-broken currently draws two rounded rects `a`/`b` touching at ±0.55r (88-91): move them apart (gap) and keep the diagonal slash. Padlock (41-55): body filled `pal::WARN` alpha 90 (`Color32::from_rgba_unmultiplied`/`gamma_multiply`) + outline, shackle arc open-topped (the `pts` arc already starts open at the left; ensure it does not close). Tests to keep green: `every_icon_paints_something` (icons.rs ~150-163) and `flag_icon_is_18_square_and_accessible` (166-187).

---

### 9. Debug screenshot mode: `src/overlay/screenshot.rs` + `app.rs` (utility, event-driven)

**Analog:** itself. `ShotArgs` parse (screenshot.rs 45-87) already supports `--tab rules|config|economy|about`, `--popup changed|switch|quit|delete-rule|...`, `--status`, `--filter`. `app.rs::new_screenshot` (805-870) seeds state: e.g. the popup arm:

```rust
Some("delete-rule") => { ... self.list.confirm_delete = id; }   // app.rs 841-847
```

Needed for this phase (UI-SPEC step 6): shots of rules collapsed, expanded with each card, popup. Add `--expand <rule index|n>` (and optionally `--search <text>`) that sets `self.list.expanded = Some(rows[n].id)` and `self.query`, parsed next to `--tab` in `screenshot::parse`, with a unit test next to `every_flag_is_read` (screenshot.rs 245-268) and `bad_input_is_an_error` (270-276). Keep the compiled-out-of-release guard pattern already used (`#[cfg(debug_assertions)]` in app.rs 76, 221-224). Also `ff-proto` must still build: it is a separate bin with its own `Args` (ff-proto.rs 54-86); skin additions are additive.

---

### 10. Pickers: `body.rs::picker_popup` (1118-1223) (component, request-response)

**Analog:** itself plus `ff_dropdown` popup art (skin/widgets.rs 742-789). Current: `egui::Popup::from_toggle_button_response(button).close_behavior(CloseOnClickOutside).width(width).show(..)` with a raw `egui::TextEdit` search and `egui::CollapsingHeader`s inside a `ScrollArea::vertical().max_height(320.0)`. Skin it by: popup `.frame(egui::Frame::NONE.inner_margin(6))` and the `bg = ui.painter().add(Shape::Noop)` ... `nine_slice(frame_rect, skin.box_9slice.size(), Margins::new(8.0,8.0,8.0,8.0), 1.0, ppp)` mesh block (ff_dropdown 747-789, copy verbatim), `ff_text_field` for the search, keep `CollapsingHeader` logic (`picker_section_override`, `PICKER_SECTION_DEFAULT_OPEN`, tests at body.rs 3651-3860 are pure functions and unaffected), items `regular(15)` rows with `pal::DISABLED` for present/disabled entries, and keep the `ALREADY_ON_RULE` focus behaviour (`pick_entry` 296-338, tested by `picking_a_present_class_focuses_it_and_emits_nothing` 3723). Header text `small_caps(17)` `pal::LABEL`.

---

## Shared Patterns

### Optional skin, headless-test friendly
**Source:** `src/overlay/config_view.rs` 164, 255-266
**Apply to:** `show_list`/`draw_row`/`BodyCtx`/`Card` if the skin is threaded explicitly.
```rust
pub skin: Option<&'a crate::skin::Skin>,
...
match ctx.skin { Some(skin) => { crate::skin::widgets::ff_section(ui, skin, title); } None => { ui.label(RichText::new(title).size(21.0)); } }
```
Headless tests that need fonts: `crate::skin::fonts::install(&ctx)` (rule_list.rs:904-906); tests that need the Skin: `let skin = Skin::load(&ctx);` (skin/editor.rs test).

### Deferred actions
**Source:** `RowAction` collection (`actions: &mut Vec<RowAction>`), applied once after the frame by `apply_row_actions` (app.rs 541-640) via `rules::coalesce_edits`. Every new skinned control must keep pushing `RowAction`s and never mutate `WorkingFilter` directly during render (the editor tests in body.rs assert exact `RowAction` values).

### Click-test harness
**Source:** `rule_list.rs` tests 899-955 (`headless_ctx`, `input`, `click_events`, `run_frame`) and body.rs `scan_clicks`/`click_after_settle` (3870-3920). Position-scanning tests tolerate layout changes (they scan for the first click that emits the wanted action) but fixed-coordinate tests do not: `arrows_do_nothing_when_reorder_is_disabled_and_work_when_enabled`, `clicking_the_summary_text_expands_the_row`, and the `chip_at`/`probed` tests (body.rs 4054-4064) that use `probe_rect` keys rather than coordinates (stable if probes are kept).

### Source-grep tests (fragile to refactors)
`rule_list.rs::expanded_body_orders_name_conditions_style` (1384), `empty_name_box_writes_no_meta` (1442-1446), `body.rs::base_type_list_is_vertical_and_idle` (3388-3400, asserts a removed `fn list_chips` string is absent). Any rename of `caption(ui, "NAME")`, `circular_toggle`, `name_open`, `body::conditions_group`, `body::style_section` must update these `include_str!` checks.

### Palette / fonts
`use crate::skin::palette as pal; use crate::skin::fonts::{regular, small_caps};` sizes only 14/15/17/21. Row ground helper `skin::widgets::row_background(ui, index, rect)` (447) and constants `ROW_HEIGHT 34`, `CONTROL_WIDTH 252`, `ROW_PAD 10` (440-444).

## No Analog Found / Gaps

| Item | Role | Data Flow | Reason |
|------|------|-----------|--------|
| Viewport-aware game scroll area | component | streaming | `ff_scroll_area` is closure-only (no `show_viewport`, no programmatic offset); needs an additive variant (section 5) |
| Variable-height two-column row (label 128 + control fills) | component | request-response | `ff_row` is fixed 34pt with a right-aligned 252pt control column (section 6) |
| Skinned X remove button | component | request-response | `skin::editor::ff_icon_button` is the minimap tile, not an X; restyle `overlay::widgets::remove_x_button` instead |
| Log-scale skin slider | component | request-response | `ff_slider` is linear only; `themed_slider(.., log)` has callers via `grammar::slider_range` |
| Quoted-list framed scroll (7 rows, 28pt, remove x, Enter-add, paste split) | component | CRUD | Behaviour exists in `overlay::widgets::string_list`/`chip_list` (670-780); only the skin restyle is new, reuse `ff_scroll_area`-style track and `ff_text_field` |
| `--expand`/`--search` screenshot flags | utility | event-driven | extend `ShotArgs`; no existing flag expands a rule |

## Metadata

**Analog search scope:** `src/overlay/`, `src/skin/`, `src/filter/`, `src/bin/ff-proto.rs`, `tests/`
**Files scanned:** about 25 (targeted reads of rule_list.rs, widgets.rs, body.rs seams, structure.rs, rules.rs, app.rs, skin/widgets.rs, skin/editor.rs, skin/icons.rs, ff-proto.rs, virtual_list.rs, screenshot.rs)
**Pattern extraction date:** 2026-10-16
