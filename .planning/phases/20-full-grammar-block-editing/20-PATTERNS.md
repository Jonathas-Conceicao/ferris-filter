# Phase 20: Full-Grammar Block Editing - Pattern Map

**Mapped:** 2026-10-13
**Files analyzed:** 14 new/modified (plus tests and docs)
**Analogs found:** 13 / 14 (the line-patch API in `filter` has only a partial analog; value codec has a partial one)

Line numbers refer to the working tree at commit b9632d9. Sources: `20-CONTEXT.md`, `20-RESEARCH.md` (suggested plan decomposition 20-01..20-07), `20-UI-SPEC.md`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/filter/lineedit.rs` (NEW; patch/insert/remove one body line) | model (lossless edit API) | transform (validate then commit) | `src/filter/edit.rs` (`replace_block_lines`, `set_meta`) + `src/filter/structure.rs` `set_continue` | role-match (no single-line patcher exists) |
| `src/filter/edit.rs` (MOD: relax Warning rejection, share `commit_lines` helper) | model | transform | itself, `replace_block_lines` lines 276-344 | exact |
| `src/filter/styled.rs` or `src/dynamic.rs` (NEW `rewrite_styled`) | service | batch transform over Document | `src/filter/section.rs` (`refresh`, `find`) + `Document::set_meta`/`set_block_keyword` | role-match |
| `src/emitter.rs` (MOD: extract `style_action_lines`) | utility | transform | `emit_style_actions` lines 209-262 | exact (refactor) |
| `src/overlay/grammar.rs` (NEW, egui-free: groups, operators, ranges, codec) | utility | transform | `src/overlay/text_entry.rs` (pure, egui-free, test-heavy) + `rules.rs::extract` (token-range reading) | role-match |
| `src/overlay/classes.rs` (NEW `ClassCatalog`) | model | transform | v0.4 `class_groups`, `git show 911666f^:src/overlay/edit.rs` lines 206-216 | exact (restore) |
| `src/overlay/rules.rs` (MOD: `RowAction` variants, `body_view`, pending rows, `StyleState`, `set_styles`, `text_with_styles`) | store / controller | request-response (deferred actions) | itself: `RowAction` 33-43, `apply_action` 410-509, `extract` 147-209, `rebuild` 259-355 | exact |
| `src/overlay/widgets.rs` (MOD: `field_row`, `remove_x_button`, `chip_list`, `number_field`, `colour_swatch`) | component | event-driven (immediate mode) | `circular_toggle` 114-138, `arrow_button` 158-202, tests 204-320 | exact |
| `src/overlay/body.rs` (NEW: condition rows, action rows, pickers, style banner) | component | event-driven | `rule_list.rs::expanded_body` 354-498 + v0.4 `render_classes`/`render_base_types` (git) | exact |
| `src/overlay/rule_list.rs` (MOD: delegate to `body.rs`, flags, `ListUi` fields) | component | event-driven | itself: `ListUi` 21-58, `flags_for` 92-139, `draw_row` 219-348 | exact |
| `src/overlay/app.rs` (MOD: `apply_working` style hook, load `styles.json`/`config.json`, `apply_row_actions` focus/scroll) | controller | request-response, file-I/O | itself: `apply_working` 132-190, `apply_row_actions` 197-243 | exact |
| `src/overlay/text_entry.rs` (reuse unchanged) | utility | transform | itself | exact |
| `tests/filter_roundtrip.rs` (MOD: no-op edit sweep) + `tests/overlay_rules.rs` (MOD) | test | batch | `edit_replace_one_block_filterblade` (line 243), `fixture_edits_stay_fast_and_revert_restores_bytes` | exact |
| `CLAUDE.md`, `CHANGELOG.md`, `REQUIREMENTS.md`, `20-HUMAN-UAT.md` | docs | n/a | Phase 19 docs updates (`19-HUMAN-UAT`-style) | role-match |

## Pattern Assignments

### `src/filter/lineedit.rs` (NEW; model, validate-then-commit transform)

**Analog:** `src/filter/edit.rs` (commit machinery) and `src/filter/structure.rs` (`header_parts`, `set_continue`, prefix-aware ranges).

Nothing in the tree patches one body line. The pattern to copy is "build a candidate `Vec<Line>`, validate in isolation, then `commit`". Reuse, do not reimplement, `commit`/`Expect`/`check_neighbours`. Because `commit` and `Expect` are private to `edit.rs`, the new ops either live in `edit.rs` or `lineedit.rs` is a child using `pub(super)` (as `block_pos` and `fresh_eol` already are, edit.rs 114 and 122).

**Imports / module style** (edit.rs 13-20):
```rust
use thiserror::Error;

use super::Document;
use super::block::{BlockId, BlockKeyword, DisabledState, Item};
use super::diag::{FilterDiagnostic, Severity};
use super::disabled;
use super::lines::{Eol, Line};
use super::parse;
```

**Error enum pattern** (edit.rs 22-58): extend `EditError` with `thiserror` variants, one sentence each, e.g. `#[error("replacement lines must not contain line breaks")] EmbeddedNewline`. Add variants for a stale line (`StaleLine`) and a bad value; the overlay surfaces `e.to_string()` as status text (rules.rs `map_err(|e| e.to_string())`).

**Core pattern to generalize: per-line EOL preserved, token-range patch.** `Line::new(text, eol)` is `pub(crate)` (lines.rs 30). `replace_block_lines` currently forces one EOL (edit.rs 290-304, the thing to NOT copy):
```rust
let eol = self.fresh_eol(header);
let last_unterminated =
    owned.end == self.lines.len() && self.lines[owned.end - 1].eol() == Eol::None;
let cand: Vec<Line> = new_lines.into_iter().enumerate().map(|(i, t)| {
    let e = if i + 1 == n && last_unterminated { Eol::None } else { eol };
    Line::new(t, e)
}).collect();
```
The new ops should instead copy the existing `Line`s and rebuild only the touched one, like `set_enabled` (edit.rs 257-266) and `set_meta` (edit.rs 374-384):
```rust
let mut new_lines = self.lines.clone();
new_lines[i] = Line::new(text, self.lines[i].eol());   // keep this line's own terminator
// insert: lines.insert(header, Line::new(text, self.fresh_eol(header)));
```

**Isolated one-block validation** (edit.rs 306-327) is the model for "reject only NEW diagnostics". The research change is to compare the candidate's Error/Warning diagnostics against the block's existing ones instead of rejecting any:
```rust
let mut scratch = 0u64;
let (items, diags) = parse::build(&cand, &mut scratch);
let one = items.len() == 1 && matches!(&items[0], Item::Block(c) if c.header == 0 && c.end == n);
if !one { return Err(EditError::NotOneBlock); }
...
.filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
```

**Commit with expectations** (edit.rs 332-343; copy the `Expect` shape, with `owned_len` = new owned length and the `Fail` mapping):
```rust
let ex = Expect { header, owned_len: n, enabled: None, reject_errors: false, enableable: false };
self.commit(lines, pos, ex).map_err(|f| match f {
    Fail::Structure => EditError::NotOneBlock,
    Fail::Neighbours => EditError::WouldReattributeLines,
    Fail::Errors(diagnostics) => EditError::InvalidBlock { diagnostics },
})
```
Note `reject_errors: false` means `commit` does not re-check diagnostics; the isolated check above does. For a disabled block use `enableable: true` semantics only where the block was clean (see `set_enabled` edit.rs 244-249).

**Disabled-block prefix awareness** (structure.rs 145-151 in `header_parts`, and 185-197 in `set_continue`). Token ranges are relative to the text after the first `#`:
```rust
let off = if b.is_enabled() { 0 } else {
    let inner = disabled::uncomment_once(text).ok_or(EditError::CompactHeader)?;
    text.len() - inner.len()
};
let t = tokenize(&text[off..]).map_err(|_| EditError::CompactHeader)?;
```
New-line indentation is copied from the first body line (structure.rs 198-202):
```rust
let line = match first {
    Some((prefix, indent)) => format!("{prefix}{indent}Continue"),
    None if enabled => "  Continue".to_string(),
    None => "#  Continue".to_string(),
};
```

**Byte-range patch** (research code example; tokenizer ranges: `TokenizedLine { indent, keyword, operator: Option<(Range, Operator)>, values: Vec<Range>, comment }`, tokenize.rs 54-60). Patch `operator` range and the span `values.first().start .. values.last().end`; keep indent, keyword text, spacing and trailing comment verbatim. Removing the last value removes the line (never emit a bare keyword; mirrors v0.4 `set_class` lesson).

**Newline guard** (edit.rs 281-283, copy verbatim): `if new_lines.iter().any(|s| s.contains(['\n', '\r'])) { return Err(EditError::EmbeddedNewline); }`

**Tests to copy:** edit.rs unit tests (line 428+): `fn p(s) -> Document`, `fn ids(d)`, `fn s(d)`, and CRLF case `disable_then_enable_crlf` (line 444). Add mixed-EOL and warning-bearing-block cases.

---

### `src/filter/edit.rs` (MOD)

**Analog:** itself.
Change 1: factor the body of `replace_block_lines` (276-344) into `commit_lines(id, Vec<(String, Eol)>)` shared with the new ops.
Change 2: replace the blanket Warning rejection at 318-325 with "no new Error/Warning relative to the block's current diagnostics". `rules.rs` already shows how to select a block's diagnostics by range (rules.rs 304-314):
```rust
let owned = b.owned();
for d in self.doc.diagnostics() {
    if d.line > owned.start && d.line <= owned.end { /* d.severity Error | Warning | Info */ }
}
```
Do not loosen `set_enabled`'s `reject_errors: enabled` path (edit.rs 254): enabling a block must still produce fully valid grammar.

---

### `src/emitter.rs` (MOD: `style_action_lines`) (utility, transform)

**Analog:** `emit_style_actions`, emitter.rs 209-262 (excerpted). Refactor to a pure `pub fn style_action_lines(style: &Style) -> Vec<String>` returning lines with no indent, in the existing fixed order (PlayEffect, MinimapIcon, SetTextColor, SetBackgroundColor, SetBorderColor, SetFontSize, Custom/PlayAlertSound, DisableDropSound). Then:
```rust
fn emit_style_actions(out: &mut String, style: &Style) {
    for l in style_action_lines(style) { writeln!(out, "  {l}").unwrap(); }
}
```
Line formats to keep byte-identical:
```rust
format!("PlayEffect {}{}", beam.color.as_filter_str(), if beam.temp { " Temp" } else { "" })
format!("MinimapIcon {} {} {}", mm.size, mm.color.as_filter_str(), mm.shape.as_filter_str())
format!("SetTextColor {} {} {} 255", rgb[0], rgb[1], rgb[2])
format!("CustomAlertSound \"{}\" {}", custom, sound.vol)   // else PlayAlertSound {id} {vol}
```
Guard: `tests/fixtures/seed-universal.filter` and `tests/snapshots/cli_generate__generate_snapshot.snap` must stay byte-identical (`artifact_guard`, `generate_snapshot`).

`Style` fields (models/config.rs 91-102): `text/border/background: Option<[u8;3]>`, `font: Option<u8>`, `minimap`, `beam`, `sound`, `action: StyleAction`, `disable_drop_sound: bool`. `StyleSet::get(&str) -> Option<&Style>`, `names()`, `len()` (config.rs 219-247).

---

### `rewrite_styled` (NEW; `src/filter/styled.rs` or `src/dynamic.rs`) (service, batch transform)

**Analog:** `src/filter/section.rs` (library pass over `Document` text with typed errors) and the `set_meta`/`remove_meta`/`set_block_keyword` calls used from `rules.rs`.

Contract from RESEARCH: `rewrite_styled(doc: &mut Document, styles: &StyleSet) -> StyleReport { rewritten: usize, unknown: Vec<(rule_index, name)> }`, called on a CLONE of the document in `apply_working`. Skip economy-section blocks using the same predicate `rebuild` uses (rules.rs 260-266):
```rust
let sec: Result<Option<SectionSpan>, _> = section::find(&self.doc);
let in_economy = |h: usize| span.is_some_and(|s| h > s.begin && h < s.end);
```
Reading the tag: `doc.meta_get(id, "style")` (mod.rs; returns the first `#ff:style`, trimmed). Keyword flip: `doc.set_block_keyword(id, style.action.into())` (structure.rs 160, touches only the keyword). Idempotence: compare semantically (keyword plus token values), not textually, so a second Apply is `Unchanged`.

Error handling: unknown style is NOT an error; it is counted in the report and the block is untouched (CONTEXT).

---

### `src/overlay/grammar.rs` (NEW; egui-free utility, transform)

**Analog:** `src/overlay/text_entry.rs` (module doc stating "Pure string handling, no egui and no model types, so it stays unit-testable without a window (PITFALLS.md section 8)" and a block of small named unit tests) and `rules.rs::extract` for reading values out of token ranges.

**Module header pattern** (text_entry.rs 1-4):
```rust
//! Text-entry helpers shared by the rule editor ...
//!
//! Pure string handling, no egui and no model types, so it stays unit-testable without a window
//! (`PITFALLS.md` section 8).
```

**Reading token values from a (possibly disabled) line** (rules.rs 161-178, copy for `decode`):
```rust
let base = if b.is_enabled() { text } else {
    match text.find('#') { Some(i) => &text[i + 1..], None => text }
};
let values: Vec<String> = tok.values.iter()
    .filter_map(|r| base.get(r.clone()))
    .map(|v| unquote(v).to_string())
    .collect();
```
`fn unquote(s) -> &str { s.trim_matches('"') }` (rules.rs 143).

**Source of truth for shapes:** `crate::filter::keywords::{KEYWORDS, ValueShape, Kind, Ops, COLOURS, SHAPES, RARITIES}` (keywords.rs 9-80). `Operator::as_str` (tokenize.rs 24) gives operator text. Drive control choice by `match kw.value { ValueShape::Int => .., ValueShape::Rarity => .., ... }`.

**Quoted-list entry parsing:** reuse `text_entry::parse_base_type_entry` unchanged (text_entry.rs 18-48).

**Required tests** (RESEARCH): every condition keyword in `KEYWORDS` reachable from a group or `Other`, every grouped name exists in `KEYWORDS` (same spirit as the "never hardcode groups" test, PITFALLS section 10); `encode(decode(x)) == original` or raw fallback for every body line of `tests/fixtures/parser/filterblade.filter` (loaded as in `tests/overlay_rules.rs` 11-17 via `env!("CARGO_MANIFEST_DIR")`).

---

### `src/overlay/classes.rs` (NEW; model, transform)

**Analog (restore, do not invent):** v0.4 `class_groups`, `git show 911666f^:src/overlay/edit.rs` lines 206-216:
```rust
pub fn class_groups(config: &Config) -> Vec<(&'static str, &[String])> {
    [
        ("WEAPONS", config.weapon_classes.as_slice()),
        ("ARMOUR", config.armour_classes.as_slice()),
        ("JEWELLERY", config.jewellery_classes.as_slice()),
        ("FLASKS & CHARMS", config.flask_charm_classes.as_slice()),
    ]
    .into_iter()
    .filter(|(_, classes)| !classes.is_empty())
    .collect()
}
```
Config fields: `weapon_classes`, `armour_classes`, `flask_charm_classes`, `jewellery_classes` (models/config.rs 676-687). Load via `loader::load_config(path: &str, styles_path: &str) -> Result<Config, LoadError>` (loader.rs 132). The overlay currently never loads `config.json`; if it fails, degrade to the generic chip list and show the error. Port the v0.4 tests (`class_groups_are_borrowed_from_the_config_in_order`, `jewellery_is_offered_and_comes_from_the_config_not_a_literal`).

---

### `src/overlay/rules.rs` (MOD; store/controller, deferred actions)

**Analog:** itself.

**RowAction vocabulary** (rules.rs 31-43): add variants per RESEARCH Pattern 1, following the existing tuple/struct style:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowAction {
    SetEnabled(BlockId, bool), Move(BlockId, Dir), Delete(BlockId), Duplicate(BlockId),
    SetKeyword(BlockId, BlockKeyword), SetContinue(BlockId, bool),
    SetName(BlockId, String), ClearName(BlockId), AddRule,
    // NEW
    EditLine { id: BlockId, line: usize, expect: String, patch: LinePatch },
    AddLine { id: BlockId, kind: LineKind, text: String },
    RemoveLine { id: BlockId, line: usize, expect: String },
    ApplyStyle(BlockId, String),
    RemoveStyle(BlockId),
}
```

**Dispatch pattern** (rules.rs 410-509): each arm yields `Result<(), String>` via `.map_err(|e| e.to_string())`; success calls `self.rebuild()` and returns `ActionOutcome { changed: true, status, focus }`; failure returns `ActionOutcome { changed: false, status: Some(msg), focus: None }`. Early-return refusal copy for a model-level rule (e.g. `AddLine(Action)` on a Known-styled block, stale `expect`) follows the `SetEnabled` arm (lines 414-421):
```rust
if on && self.row(id).is_some_and(|r| r.cannot_enable) {
    return ActionOutcome { changed: false, status: Some(CANNOT_ENABLE_STATUS.to_string()), focus: None };
}
```
`SetName`/`ClearName` arms (484-492) are the template for the `ApplyStyle`/`RemoveStyle` meta writes (`set_meta`/`remove_meta`; trim and reject empty or control-character style names at the call site because `set_meta` trims, meta.rs).

**Duplicate keeps the tag:** the `Duplicate` arm (438-452) already copies the leading run; add a test that `#ff:style` is copied (RESEARCH A2).

**Row projection:** `RuleRow.style_tag: Option<String>` (line 79, built at 286) becomes a `StyleState { None, Known(name), Unknown(name) }` computed in `rebuild` against a `StyleSet` stored on `WorkingFilter` (`set_styles` calls `rebuild`). Keep `RuleRow` small; build the typed per-line `body_view(id)` lazily for the one expanded row only (rows are rebuilt once per mutation, module doc lines 3-6), cached by `generation` (line 542).

**Pending-row state:** put it in `ListUi` (see rule_list.rs) and clear it in `clear_ids`.

**Test pattern:** headless model tests construct `WorkingFilter::from_text(...)`, call `apply_action`, assert on `text()`/`rows()`; perf pattern in `tests/overlay_rules.rs` (fixture load plus 100 edits under a loose budget).

---

### `src/overlay/widgets.rs` (MOD; component, immediate-mode)

**Analog:** `circular_toggle` and `arrow_button` (widgets.rs 114-202), the painted-control recipe: `allocate_exact_size` then `ui.interact`/`Sense`, then `ui.painter()` shapes, and never a Unicode glyph (Cinzel has none; module header 1-17).

**`remove_x_button` (24x24 target, 16px paint)** copy the header close control (widgets.rs 46-78):
```rust
let close_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 20.0, rect.center().y), egui::vec2(close_size, close_size));
let close_response = ui.interact(close_rect, ui.id().with("header-close"), egui::Sense::click());
let stroke_color = if close_response.hovered() { theme::SECTION_CREAM } else { theme::LABEL_TAN };
let stroke = egui::Stroke::new(2.0, stroke_color);
painter.line_segment([close_rect.left_top() + vec2(pad, pad), close_rect.right_bottom() - vec2(pad, pad)], stroke);
painter.line_segment([close_rect.right_top() + vec2(-pad, pad), close_rect.left_bottom() + vec2(pad, -pad)], stroke);
```
Add `response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Remove this condition"))` per UI-SPEC accessibility.

**`circular_toggle` mutate-in-place plus `mark_changed`** (widgets.rs 114-121) is the contract every new control follows: callers branch on `.changed()`, and writes happen only on `changed()`.

**Slider styling:** reuse v0.4 `themed_slider` (`git show 911666f^:src/overlay/rule_editor.rs` lines 639-680): scoped `spacing_mut().slider_width`, `slider_rail_height`, `HandleShape::Circle`, `selection.bg_fill = theme::CONTROL_GLYPH`, `egui::Slider::new(value, range).integer().trailing_fill(true)`. Per RESEARCH, add `.show_value(false)` plus an adjacent `number_field` and `.clamping(SliderClamping::Never)`.

**Chip list / add field** restore from v0.4 `render_base_types` (rule_editor.rs 365-500): owned clone of items, record `remove_at: Option<usize>` and apply after the loop; add field with `TextEdit::singleline(&mut buf).hint_text(..)`, buffer in egui temp memory `ui.data_mut(|d| d.get_temp_mut_or_default::<String>(buf_id))`, commit only on Enter (`response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))`) or `+ ADD`, then `parse_base_type_entry(&buf)` and `response.request_focus()`. The right-to-left row recipe with explicit `row_size` (`allocate_ui_with_layout`, comment at rule_editor.rs ~395-420) is load-bearing: do not use `with_layout` for the outer row (it produced the one-row-per-screen bug).

**Tests** (widgets.rs 204-320): copy `click_at`, `headless_ctx` (`ctx.set_fonts(FontDefinitions::empty())`), and the two-frame technique (frame 1 learns the rect, frame 2 clicks its center, `.drop_without_applying_deltas()`). Add "idle frame emits no change" tests for every new control (colour round-trip drift guard).

---

### `src/overlay/body.rs` (NEW) and `src/overlay/rule_list.rs` (MOD) (component)

**Analog:** `rule_list.rs::expanded_body` (354-498) for structure, plus v0.4 `render_classes` for the grouped class picker.

**Collect-then-apply discipline** (rule_list.rs module doc 3-6 and every control): controls push `RowAction`s into `actions: &mut Vec<RowAction>`; nothing mutates `WorkingFilter` during render. Signature to keep:
```rust
fn expanded_body(ui: &mut egui::Ui, row: &RuleRow, list: &mut ListUi, actions: &mut Vec<RowAction>)
```

**NAME block stays as is** (rule_list.rs 362-397): circular toggle plus draft in `list.name_drafts`, write on `resp.changed()`, `list.name_open` keeps the field open while the draft is empty. Same draft-in-`ListUi` approach applies to the pending-row state and add-entry buffers, but key by `(BlockId, kind, keyword occurrence ordinal)` rather than line index (RESEARCH pitfall 5). Id-keyed state must be cleared in `clear_ids` (rule_list.rs 50-57) and tests extended (`clear_ids_resets_every_id_keyed_field`, ~line 735).

**Locked-control pattern** (rule_list.rs 399-447): `let locked = ...; ui.add_enabled(!locked, btn)` with `DISABLED_TEXT`/`HEADER_PLATE` fill and a note label (`RichText::new(..).size(theme::ROW_SUMMARY_SIZE).color(theme::TAB_ACTIVE)`). Reuse for style-owned action rows; the lock must depend on `StyleState::Known` only. Replaces the Phase 19 note at 439-447, the "Editing conditions arrives in a later update." text at 455-459, and the read-only `row.conditions` loop at 452-454.

**Caption helper** (rule_list.rs 350-352): `ui.label(RichText::new(text).small().color(theme::LABEL_TAN))`.

**Segmented Show/Hide/Minimal** (rule_list.rs 403-426) stays as the first line of the ACTIONS group.

**Flags** (rule_list.rs 78-139): rename `style wins` (line 111) to `styled: <name>` (12-char truncation, tooltip full) and add the non-blocking `unknown style` flag using `theme::WARM_ACCENT`, following the `Flag { text, color, tip }` struct and `flags_for` shape. `Flag.text` is `&'static str` today; it must become `String`/`Cow` for `styled: <name>`.

**Row frame/margins** (rule_list.rs 334-347): body `Frame::new().fill(PANEL_GROUND).inner_margin(..)`, measured height cache `list.expanded_h` updated with a one-frame `request_repaint`. Keep this; UI-SPEC wants `Margin::same(8)` and spacing snapped to 4/8.

**Class picker (grouped, from catalogue)**: restore v0.4 `render_classes` shape (rule_editor.rs 275-350): one `egui::CollapsingHeader::new(format!("{group_label} ({selected_count})")).id_salt((id, group_label)).default_open(false)` per `ClassCatalog` group, each class a `ui.checkbox(&mut selected, class.as_str())`; on `.changed()` push an edit. Prefer natural-height groups to a capped inner scroller (RESEARCH pitfall 8). Classes in the file but absent from the catalogue render as removable chips above the picker.

**Popups** (`+ condition`/`+ action`): egui 0.36 `egui::Popup::from_toggle_button_response(&btn).close_behavior(PopupCloseBehavior::CloseOnClickOutside)`; `popup_below_widget` no longer exists. Close popups when `list.expanded` changes and in `clear_ids`.

**Test pattern** (rule_list.rs 640-799): `run_frame(ctx, raw, wf, list, reorder) -> (ListFrame, f32)` lays `show_list` in a 460x600 child; style-tagged rows render with `list.expanded = Some(wf.rows()[0].id)` for two frames asserting no spurious actions (`expanded_style_tagged_row_renders_without_actions` at ~766 is to be rewritten for the Known/Unknown states).

---

### `src/overlay/app.rs` (MOD; controller, file-I/O)

**Analog:** `apply_working` (132-190). Single Apply path; hook the style rewrite before the writer:
```rust
match dynamic::apply_edited(loaded, &working.text(), op) {
    Ok(report) => {
        working.on_applied(loaded.text());
        if report.migrated { extras.push_str(" Converted from v0.4."); }
        ...
```
Change to compute `let (text, style_report) = working.text_with_styles(&styles)` (rewrite on a clone, Pattern 4) and pass `&text`. Append `" Updated N styled rules."` to `extras` (omit at 0), same `extras.push_str(..)` style as the fallback-price and `Converted from v0.4.` sentences. Load `styles.json` fresh at this point via `loader::load_styles(&str)` (loader.rs 190); on `LoadError` return `(ApplyOutcome::Failed(err.to_string()), extras, false)` like the `economy_body` error arm (lines 143-145), refusing Apply with the styles error.

**`apply_row_actions`** (197-243): after `AddLine`, set `list.pending_scroll`/focus like the `is_add || is_duplicate` branch; extend the pre-dispatch classification block (`is_add`, `is_duplicate`, `is_move`, `deleted`) rather than restructuring.

**Tests** (app.rs ~1067-1130): `loaded_in(&dir, text) -> (PathBuf, LoadedFilter)` with `tempfile::tempdir()`; assert `ApplyOutcome::Written`/`Unchanged`, `std::fs::read_to_string(&path)`, and that the second `apply_working` returns `Unchanged`. For styled rewrite write a `styles.json` into the tempdir (root) before calling.

---

### Tests: `tests/filter_roundtrip.rs`, `tests/overlay_rules.rs`

**Analog:** `tests/filter_roundtrip.rs` `edit_replace_one_block_filterblade` (line 243) and helpers `fixture()` (232), `enabled_ids` (236), `byte_offset` (224): load the FilterBlade fixture (CRLF, `-text`, byte-protected), apply an edit, assert only the edited block's bytes changed. New sweep: for each condition/action line, patch with its own value and assert `doc.to_bytes()` identical; and `Class "x"` operator stays bare after a values-only edit (diff-line count == 1).

---

## Shared Patterns

### Optimistic line addressing (every body edit)
**Source:** RESEARCH Pattern 1 plus `apply_row_actions` (app.rs 197) which applies several actions per frame sequentially, so earlier removals shift later line numbers. Every `EditLine`/`RemoveLine` carries `(id, line, expect_text)` and the model refuses with a status if `doc.lines()[line].text() != expect` (`Document::lines()` is already used at rules.rs 161).
**Apply to:** `rules.rs` dispatch, `body.rs` emitters.

### Deferred mutation via `RowAction`
**Source:** `rule_list.rs` module doc 1-6 and `actions.push(RowAction::...)` throughout; `rules.rs::apply_action`.
**Apply to:** all new controls. Write only on `response.changed()` with a value different from the current one.

### Never normalize; never emit empty
**Source:** CONTEXT "Editing rules"; v0.4 `set_class` collapsing an empty list to `None` (`git show 911666f^:src/overlay/edit.rs` lines 225-246). Patch operator and value ranges only; removing the last list entry removes the line (encode returns `None` for empty).
**Apply to:** `lineedit.rs`, `grammar.rs`, list controls.

### Error to status string
**Source:** `.map_err(|e| e.to_string())` in `rules.rs` 422-492; `EditError` thiserror messages are the user-visible copy.
**Apply to:** all new `EditError` variants (write them as UI-ready sentences).

### Theme tokens
**Source:** `src/overlay/theme.rs` (20-45): `PANEL_GROUND`, `ROW_BASE/ROW_ALT`, `INPUT_FILL`, `HEADER_PLATE`, `LABEL_TAN`, `SECTION_CREAM`, `VALUE_TEXT`, `CONTROL_GLYPH`, `TAB_ACTIVE`, `DISABLED_TEXT`, `WARM_ACCENT`. `WARM_ACCENT_TEXT` (#BF8548) does not exist yet and must be added for text uses. Accent only for the reserved list in UI-SPEC.
**Apply to:** all overlay files.

### Colour handling
**Source:** RESEARCH pitfall 3. Build swatches with `Color32::from_rgba_unmultiplied`, read back with `to_srgba_unmultiplied()`, `color_edit_button_srgba(ui, &mut c, Alpha::OnlyBlend)`, write only on `changed()`; keep the 3-token form unless alpha was edited.
**Apply to:** colour action rows.

### Verification commands
`CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test` (the running `ff.exe` locks the default target dir), then `cargo fmt --check` and `cargo build --bins` warning-free; protected artifacts: `tests/fixtures/seed-universal.filter`, `tests/snapshots/cli_generate__generate_snapshot.snap`, `tests/fixtures/parser/filterblade.filter`.

## No Analog Found

| File / piece | Role | Data Flow | Reason |
|---|---|---|---|
| Single-line `LinePatch` op, `insert_body_line`, `remove_body_line` in `src/filter/lineedit.rs` | model | transform | No existing single-line editor; closest are `replace_block_lines`/`set_continue`, which rebuild the whole body and re-terminate every line. Use the commit machinery but not the whole-body rebuild. |
| Structured value codec (Sockets `5GGG`, `CountThenNames`, Rgba/Sound/MinimapIcon decode and encode) | utility | transform | No parser of value semantics exists (`validate_shape` only checks token shape). Follow RESEARCH "Value codec" with round-trip-or-raw fallback; sockets semantics are [ASSUMED] (A3). |
| `number_field` (TextEdit with draft, commits on parse), colour swatch, popup pickers | component | event-driven | No such widgets in the tree; v0.4 had slider and checkbox only. Build per RESEARCH "Standard Stack" egui APIs (read from egui 0.36.1 source). |
| Pending (unwritten) condition rows | store | event-driven | New concept (RESEARCH Pattern 3); closest is `ListUi.name_open`/`name_drafts`, which already hold "visible but not written" state for NAME. |

## Metadata

**Analog search scope:** `src/filter/`, `src/overlay/`, `src/emitter.rs`, `src/loader.rs`, `src/models/config.rs`, `tests/`, and git history `911666f^` for the deleted v0.4 `edit.rs`/`rule_editor.rs`.
**Files scanned:** about 20 read (full or targeted ranges); no source modified.
**Pattern extraction date:** 2026-10-13
