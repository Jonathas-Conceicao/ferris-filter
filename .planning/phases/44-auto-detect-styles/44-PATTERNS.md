# Phase 44: Auto Detect Styles - Pattern Map

**Mapped:** 2026-10-23
**Files analyzed:** 8 (1 new, 7 modified)
**Analogs found:** 8 / 8

Line numbers drift; re-grep before editing. All paths under `src/`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `overlay/style_detect.rs` (new) | pure model/planner | transform | `overlay/strictness.rs` (pure model + table tests); logic wraps `body.rs::rule_style_view` | role-match |
| `overlay/style_lines.rs` (+`StyleView::to_style`) | utility | transform | same file, `StyleView::from_style` (inverse) | exact |
| `overlay/config_state.rs` (+`add_detected_styles`) | model | CRUD | same file, `add_style` (l.219) | exact |
| `overlay/rules.rs` (+`detect_inputs`, `tag_styles`) | model | CRUD batch | `RowAction::ApplyStyle` arm (~l.1816) and group `dispatch` snapshot/restore (~l.1575) | role-match |
| `overlay/config_view.rs` (+button, `ConfigAction::AutoDetect`, `can_auto_detect`) | component | request-response | same file, `+ Add style` (l.840) and Reset button (l.800) | exact |
| `overlay/app.rs` (handler, ctx field, clear-events list) | controller | event-driven | same file, `ConfigAction::AddStyle` arm (l.3046), `sync_working_styles` (l.326) | exact |
| `styled.rs` (code comment only) | service | transform | `rewrite_block` `splice_body` call (~l.140) | exact |
| `overlay/mod.rs` | config | - | existing `pub mod` list | exact |

## Pattern Assignments

### `overlay/style_lines.rs`: `StyleView::to_style` (utility, transform)

**Analog:** `StyleView::from_style` (style_lines.rs ~72-92). Put `to_style` right below it as its inverse.
```rust
impl StyleView {
    pub fn from_style(style: &Style) -> StyleView {
        let rgba = |c: StyleColour| Rgba { rgba: c.rgba(), had_alpha: true };
        StyleView {
            visibility: match style.action { StyleAction::Show => Vis::Show, StyleAction::Hide => Vis::Hide },
            text: style.text.map(rgba), border: style.border.map(rgba), background: style.background.map(rgba),
            font: style.font.map(i64::from),
            minimap: style.minimap.clone(), beam: style.beam.clone(), sound: style.sound.clone(),
            drop_sound: style.drop_sound(), ..
```
Copy the `to_style` skeleton from RESEARCH.md "Code Examples" (Minimal -> `None`; background `!had_alpha` -> 240; `Style::blank_show()`, `s.set_drop_sound(self.drop_sound)`, `id: None`). Reject a custom sound file name containing `"` (StyleSet::validate).

### `overlay/style_detect.rs` (pure planner)

**Analog inputs (body.rs ~2646, ~2859), reuse as-is:**
```rust
pub(crate) fn rule_style_view(entries: &[BodyEntry], keyword: BlockKeyword) -> (StyleView, StyleBinding)
pub(crate) fn style_leftovers<'a>(entries: &'a [BodyEntry], binding: &StyleBinding) -> Vec<&'a BodyEntry>
```
Exact rule = `style_leftovers(..).is_empty()` and `to_style().is_some()`. Compare with id cleared (`Style` derives `PartialEq` including `id`). Match order: `StyleSet::is_builtin(name)` first, then customs, then already-planned. Name: smallest free `Custom Style N`, `eq_ignore_ascii_case` against pending + planned names. Output `{ new_styles: Vec<(String, Style)>, tags: Vec<(BlockId, String)> }`, plus pure `status_text`. Inputs are `(BlockId, Vec<BodyEntry>, BlockKeyword)` so the planner has no egui/doc dependency. Tests: table-driven from the RESEARCH equality table, building bodies via `WorkingFilter::from_text` + `body_view` (as `body.rs` tests do).

### `overlay/config_state.rs`: `add_detected_styles`

**Analog:** `add_style` (l.219-223)
```rust
pub fn add_style(&mut self, input: &str) -> Result<String, StyleNameError> {
    let trimmed = input.trim();
    self.styles.insert_new(trimmed, Style::blank_show())?;
    Ok(trimmed.to_string())
}
```
New method loops `self.styles.insert_new(&name, style)` (fresh id per call, config.rs:793); on `Err` skip and return the names actually added so the caller does not tag a missing style. No other bookkeeping (dirtiness is `PartialEq` vs `styles_applied`). Test next to `add_style_trims_and_rejects_bad_names` (l.1225).

### `overlay/rules.rs`: `detect_inputs`, `tag_styles`

**Analog 1, tag text and meta (ApplyStyle arm ~1816-1850):**
```rust
let (style, tag) = match set.get(&name) {
    Some(st) if styled::style_name_ok(&name) => (st, set.tag_for(&name).unwrap_or_else(|| name.clone())),
    _ => return refused(format!("There is no style named \"{name}\".")),
};
let mut clone = self.doc.clone();
let r = clone.set_meta(id, "style", &tag) ...
```
Use `set_meta` only; do NOT call `styled::rewrite_block` (Apply does it).

**Analog 2, group-wide edit with snapshot restore (~1575-1621):**
```rust
let snapshot = self.doc.clone();
let r = members.iter().try_for_each(|m| self.doc.set_enabled(*m, on));
...
if !out.changed { self.doc = snapshot; }
```
`tag_styles`: for each `(id, name)`, re-check eligibility, `self.groups.members(id)` -> `set_meta(m, "style", &tag)` on every member, restore snapshot on error, one `self.rebuild()` at end, return count of rules (not blocks). `detect_inputs`: iterate `rows()` (excludes economy and group followers); keep `style == StyleState::None`, `!cannot_enable`, `errors.is_empty()`, `body_editable(id)`; attach `body_view(id)` and keyword.

### `overlay/config_view.rs`: button + action

**Analog:** Custom heading row (l.840-851) and Reset button tip handling (l.800-818)
```rust
ui.horizontal(|ui| {
    let heading = group_heading(ui, "Custom");
    probe("custom-heading", heading.rect);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let add = crate::skin::widgets::ff_button(ui, &skin, "+ Add style", 124.0);
        probe("add-style", add.rect);
        ...
    });
});
```
Add after the `+ Add style` block (right_to_left, so it lands left of it): `ui.add_space(8.0)`, `ff_button_ex(ui, &skin, "Auto Detect", 112.0, ctx.can_auto_detect, false)`, then `r.on_hover_text(tip).on_disabled_hover_text(tip)` (Reset precedent: `ff_button_ex` greys inside an enabled Ui), `probe("auto-detect", r.rect)`, click and enabled -> `actions.push(ConfigAction::AutoDetect)`. Tips per UI-SPEC. Add `can_auto_detect: bool` to `ConfigViewCtx` (l.171; constructed at app.rs ~3272 and config_view tests ~1325). Update (do not replace) tests counting `add-style` probes (~2052, ~2215).

### `overlay/app.rs`: handler

**Analog (AddStyle arm l.3046) and sync (l.326-337):**
```rust
ConfigAction::AddStyle(input) => {
    if let Ok(state) = &mut self.config {
        match state.add_style(&input) { Ok(name) => { ...; self.config_message = None; } Err(err) => self.config_message = Some((err.to_string(), true)) }
    }
}
fn sync_working_styles(config: &ConfigState, working: &mut WorkingFilter) -> bool { ... working.set_styles_over(config.pending_styles().clone(), config.applied_styles()); true }
```
Handler order: `clear_events_on_edit()` (or add to the `matches!` list at ~l.3026), build inputs, `style_detect::plan`, empty plan -> `self.apply_message = Some((text, StatusKind::Info))` and stop; else `add_detected_styles`, `sync_working_styles(config, working)` (explicit, before tagging), `tag_styles`, `self.apply_message = Some((status_text, w::StatusKind::Ok))` (type seen at app.rs:2086). Do not set `apply_state` by hand. Tests: `rig(..)` / `rig_at` (app.rs ~4700) with `apply_everything`; AddStyle test at ~6635 shows driving `handle_config_actions(&ctx, vec![..])`.

### `styled.rs`: comment

At the `doc.splice_body(id, &action_lines, after, &targets)` call in `rewrite_block` (after the `Ok(false)` early return): add the author-requested comment that dropping trailing comments on action lines is intentional and Auto Detect (DETECT-04) relies on them not blocking a match. Add a styled.rs test pinning the drop.

## Shared Patterns

- **Pending edits, no disk:** dirtiness derived (`styles_dirty`, `rules_edited`, `combined_dirty`); never add new flags.
- **Tag canonical form:** `StyleSet::tag_for(name)` only (`"<id> <name>"`).
- **Status:** `apply_message: Option<(String, StatusKind)>` carries Ok and Info (`config_message` cannot).
- **Test isolation:** pin `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR`; use the temp rigs (isolation_guard).

## No Analog Found

None. Closest to new: a batch tagger (`tag_styles`); no existing batch-tag method, so compose from the two rules.rs analogs above.

## Metadata

**Analog search scope:** `src/overlay/*`, `src/models/config.rs`, `src/styled.rs`
**Pattern extraction date:** 2026-10-23
