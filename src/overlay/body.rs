//! The data-driven rule body: one editable card per condition line.
//!
//! Presentation only, collect-then-apply: nothing here mutates the [`WorkingFilter`]. Every control
//! pushes a [`RowAction`] and only on a real user change (`response.changed()` with a value that
//! differs from the current one), so rendering an expanded row with no input emits nothing. The
//! decisions about *what* an edit is live in small pure functions (`int_edit`, `op_edit`,
//! `rarity_edit`, `raw_edit`, `raw_line_edit`, ...) so they are unit-testable without a window.
//!
//! Values are rewritten through [`grammar::encode`] only; an operator changes only when the user
//! picks a different one (PARSE-04). Widget ids and drafts are keyed by
//! `(block id, keyword, ordinal)`, never by line index, because line indices shift as lines are
//! added and removed (Pitfall 5).

use egui::{Align, RichText, vec2};

use crate::filter::keywords::{Keyword, RARITIES, ValueShape};
use crate::filter::tokenize::Operator;
use crate::filter::{BlockId, LinePatch};

use super::classes::ClassCatalog;
use super::grammar::{self, EncodeError, Value};
use super::rule_list::{ListUi, PendingRow};
use super::rules::{BodyEntry, EntryKind, LineKind, RowAction, RuleRow, WorkingFilter};
use super::theme;
use super::widgets;

const LABEL_W: f32 = 120.0;
const OP_W: f32 = 104.0;

const NO_CONDITIONS: &str = "No conditions: this rule matches every item.";
const BAD_TEXT_HINT: &str = "Values cannot contain quotes or control characters";
const WHOLE_NUMBER_HINT: &str = "Enter a whole number";

/// Everything the body needs about the expanded row, borrowed for one frame.
pub struct BodyCtx<'a> {
    pub row: &'a RuleRow,
    pub entries: &'a [BodyEntry],
    pub editable: bool,
    pub classes: Option<&'a ClassCatalog>,
    pub working: &'a WorkingFilter,
}

// ---------------------------------------------------------------------------------------------
// Pure edit construction
// ---------------------------------------------------------------------------------------------

fn edit(id: BlockId, entry: &BodyEntry, patch: LinePatch) -> RowAction {
    RowAction::EditLine {
        id,
        line: entry.line,
        expect: entry.text.clone(),
        patch,
    }
}

fn remove(id: BlockId, entry: &BodyEntry) -> RowAction {
    RowAction::RemoveLine {
        id,
        line: entry.line,
        expect: entry.text.clone(),
    }
}

fn current_values(entry: &BodyEntry) -> String {
    entry.values.join(" ")
}

/// Encode `v` and compare it with the line's current value text. An empty value removes the line
/// (never a bare keyword); text that could break out of its quotes is refused.
fn values_edit(
    id: BlockId,
    entry: &BodyEntry,
    v: &Value,
) -> Result<Option<RowAction>, EncodeError> {
    match grammar::encode(v) {
        Ok(t) => Ok((t != current_values(entry)).then(|| edit(id, entry, LinePatch::Values(t)))),
        Err(EncodeError::Empty) => Ok(Some(remove(id, entry))),
        Err(e) => Err(e),
    }
}

/// `Values` edit for a single whole number; `None` when unchanged.
pub(crate) fn int_edit(id: BlockId, entry: &BodyEntry, new: i64) -> Option<RowAction> {
    values_edit(id, entry, &Value::Ints(vec![new]))
        .ok()
        .flatten()
}

/// `Operator` edit; `None` when the picked operator is already the line's.
pub(crate) fn op_edit(id: BlockId, entry: &BodyEntry, new: Option<Operator>) -> Option<RowAction> {
    (new != entry.op).then(|| edit(id, entry, LinePatch::Operator(new)))
}

/// Result of toggling rarity chips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RarityEdit {
    None,
    Edit(RowAction),
    /// No chip left: the line is removed and the caller keeps a pending row in its place.
    Remove(RowAction),
}

/// Edit for the chosen rarities (in table order). Several rarities cannot keep an ordering
/// operator, so that case drops it in the same commit (`Rarity Normal Magic Rare`).
pub(crate) fn rarity_edit(id: BlockId, entry: &BodyEntry, selected: &[&str]) -> RarityEdit {
    if selected.is_empty() {
        return RarityEdit::Remove(remove(id, entry));
    }
    let text = selected.join(" ");
    if text == current_values(entry) {
        return RarityEdit::None;
    }
    let ordering = !matches!(entry.op, None | Some(Operator::Eq) | Some(Operator::EqEq));
    let patch = if selected.len() >= 2 && ordering {
        LinePatch::Both(None, text)
    } else {
        LinePatch::Values(text)
    };
    RarityEdit::Edit(edit(id, entry, patch))
}

/// Value edit for the raw text field of a line the codec cannot structure.
pub(crate) fn raw_edit(id: BlockId, entry: &BodyEntry, text: &str) -> Option<RowAction> {
    let t = text.trim();
    if t.is_empty() || t == current_values(entry) || t.contains(['\n', '\r']) {
        return None;
    }
    Some(edit(id, entry, LinePatch::Values(t.to_string())))
}

/// The line after its indent (and a disabled block's `#`): what a raw row shows and edits.
fn line_content(text: &str) -> &str {
    let t = text.trim_start();
    t.strip_prefix('#').unwrap_or(t).trim_start()
}

/// Whole-line edit for an opaque or unknown-keyword line.
pub(crate) fn raw_line_edit(id: BlockId, entry: &BodyEntry, draft: &str) -> Option<RowAction> {
    let t = draft.trim();
    if t.is_empty() || t == line_content(&entry.text).trim_end() || t.contains(['\n', '\r']) {
        return None;
    }
    Some(edit(id, entry, LinePatch::Whole(t.to_string())))
}

// ---------------------------------------------------------------------------------------------
// Small UI helpers
// ---------------------------------------------------------------------------------------------

fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::LABEL_TAN));
}

fn hint(ui: &mut egui::Ui, text: &str) {
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::WARM_ACCENT_TEXT),
        )
        .wrap(),
    );
}

fn error_for<'a>(list: &'a ListUi, id: BlockId, line: usize) -> Option<&'a str> {
    list.line_error
        .as_ref()
        .filter(|(b, l, _)| *b == id && *l == line)
        .map(|(_, _, m)| m.as_str())
}

/// A single-line text field that holds its draft in egui temp memory while focused and commits
/// the trimmed text on Enter or blur. Escape abandons the draft. Returns the field's response and
/// the committed text when it is non-empty and differs from `current`.
fn draft_field(
    ui: &mut egui::Ui,
    tid: egui::Id,
    current: &str,
    width: f32,
    list: &mut ListUi,
    key: (BlockId, usize),
) -> (egui::Response, Option<String>) {
    let focused = ui.memory(|m| m.has_focus(tid));
    let stored = ui.data_mut(|d| d.get_temp::<String>(tid));
    let mut draft = match stored {
        Some(s) if focused => s,
        _ => current.to_string(),
    };
    let resp = ui.add(
        egui::TextEdit::singleline(&mut draft)
            .id(tid)
            .desired_width(width),
    );
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    if resp.changed()
        && list
            .line_error
            .as_ref()
            .is_some_and(|(b, l, _)| (*b, *l) == key)
    {
        list.line_error = None;
    }
    let mut commit = None;
    if resp.lost_focus() && !escape {
        let t = draft.trim();
        if !t.is_empty() && t != current.trim() {
            commit = Some(t.to_string());
        }
    }
    if resp.has_focus() {
        ui.data_mut(|d| d.insert_temp(tid, draft));
    } else {
        ui.data_mut(|d| d.remove::<String>(tid));
    }
    (resp, commit)
}

// ---------------------------------------------------------------------------------------------
// Raw rows (unknown keywords, opaque lines)
// ---------------------------------------------------------------------------------------------

/// A line shown as plain text: an inline single-line edit plus Remove when `editable`, read-only
/// text otherwise (the invalid-disabled exception).
pub(crate) fn raw_row(
    ui: &mut egui::Ui,
    entry: &BodyEntry,
    editable: bool,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
    id: BlockId,
) {
    let content = line_content(&entry.text).trim_end().to_string();
    if !editable {
        ui.add(egui::Label::new(RichText::new(&content).color(theme::LABEL_TAN)).wrap());
        return;
    }
    let tid = ui.make_persistent_id(("raw", id, entry.ordinal));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let width = (ui.available_width() - 32.0).max(80.0);
        let (_, commit) = draft_field(ui, tid, &content, width, list, (id, entry.line));
        if let Some(text) = commit
            && let Some(a) = raw_line_edit(id, entry, &text)
        {
            actions.push(a);
        }
        if widgets::remove_x_button(ui, "Remove this line").clicked() {
            actions.push(remove(id, entry));
        }
    });
    if let Some(msg) = error_for(list, id, entry.line) {
        hint(ui, msg);
    }
}

// ---------------------------------------------------------------------------------------------
// The CONDITIONS group
// ---------------------------------------------------------------------------------------------

fn card_frame(idx: usize) -> egui::Frame {
    egui::Frame::new()
        .fill(if idx.is_multiple_of(2) {
            theme::ROW_BASE
        } else {
            theme::ROW_ALT
        })
        .inner_margin(egui::Margin::same(8))
}

fn is_condition_side(e: &BodyEntry) -> bool {
    matches!(e.kind, EntryKind::Condition | EntryKind::Opaque)
}

/// The CONDITIONS section of an expanded row.
pub fn conditions_group(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let id = ctx.row.id;
    caption(ui, "CONDITIONS");
    ui.spacing_mut().item_spacing.y = 4.0;

    let lines: Vec<&BodyEntry> = ctx
        .entries
        .iter()
        .filter(|e| is_condition_side(e))
        .collect();
    if !ctx.editable {
        if lines.is_empty() {
            for line in &ctx.row.conditions {
                ui.add(egui::Label::new(RichText::new(line).color(theme::LABEL_TAN)).wrap());
            }
        }
        for e in lines {
            raw_row(ui, e, false, list, actions, id);
        }
        return;
    }
    if lines.is_empty() {
        ui.label(
            RichText::new(NO_CONDITIONS)
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::LABEL_TAN),
        );
    }
    for (idx, e) in lines.into_iter().enumerate() {
        match (e.kind, e.keyword) {
            (EntryKind::Condition, Some(kw)) => condition_card(ui, ctx, e, kw, idx, list, actions),
            _ => {
                card_frame(idx).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    raw_row(ui, e, true, list, actions, id);
                });
            }
        }
    }
}

struct Card<'a> {
    id: BlockId,
    entry: &'a BodyEntry,
    kw: &'static Keyword,
    bad_id: egui::Id,
}

impl Card<'_> {
    fn salt(&self, tag: &'static str) -> (BlockId, &'static str, usize, &'static str) {
        (self.id, self.kw.name, self.entry.ordinal, tag)
    }

    /// Push an edit result; text that cannot be encoded raises the inline hint instead.
    fn commit(
        &self,
        ui: &egui::Ui,
        res: Result<Option<RowAction>, EncodeError>,
        actions: &mut Vec<RowAction>,
    ) {
        match res {
            Ok(Some(a)) => {
                ui.data_mut(|d| d.insert_temp(self.bad_id, false));
                actions.push(a);
            }
            Ok(None) => {}
            Err(_) => {
                ui.data_mut(|d| d.insert_temp(self.bad_id, true));
            }
        }
    }
}

fn condition_card(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    entry: &BodyEntry,
    kw: &'static Keyword,
    idx: usize,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let id = ctx.row.id;
    ui.push_id((id, kw.name, entry.ordinal), |ui| {
        let card = Card {
            id,
            entry,
            kw,
            bad_id: ui.make_persistent_id("bad-text"),
        };
        let tokens: Vec<&str> = entry.values.iter().map(String::as_str).collect();
        let value = grammar::decode(kw, &tokens);
        let list_form = matches!(&value, Value::Rarity(r) if r.len() > 1);
        let mut primary: Option<egui::Response> = None;
        let framed = card_frame(idx).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
            ui.horizontal(|ui| {
                ui.add_sized(
                    [LABEL_W, 24.0],
                    egui::Label::new(
                        RichText::new(grammar::display_name(kw.name))
                            .size(14.0)
                            .color(theme::VALUE_TEXT),
                    )
                    .truncate(),
                )
                .on_hover_text(kw.name);
                if let Some(op) = operator_combo(ui, &card, list_form)
                    && let Some(a) = op_edit(id, entry, op)
                {
                    actions.push(a);
                }
                if widgets::remove_x_button(ui, "Remove this condition").clicked() {
                    actions.push(remove(id, entry));
                }
            });
            primary = value_controls(ui, &card, &value, list, actions);
            if ui
                .data(|d| d.get_temp::<bool>(card.bad_id))
                .unwrap_or(false)
            {
                hint(ui, BAD_TEXT_HINT);
            }
            if let Some(msg) = error_for(list, id, entry.line) {
                hint(ui, msg);
            }
        });
        if list.focus_line == Some((id, entry.line)) {
            framed.response.scroll_to_me(Some(Align::Center));
            if let Some(p) = primary {
                p.request_focus();
            }
            list.focus_line = None;
        }
    });
}

/// The operator dropdown; `Some(new)` only when the user picked a different operator.
fn operator_combo(ui: &mut egui::Ui, card: &Card, list_form: bool) -> Option<Option<Operator>> {
    let entry = card.entry;
    let choices = grammar::operator_choices(card.kw, entry.op, list_form);
    let current = grammar::operator_label(card.kw, entry.op, list_form);
    let mut picked = None;
    egui::ComboBox::from_id_salt(card.salt("op"))
        .selected_text(current)
        .width(OP_W)
        .show_ui(ui, |ui| {
            for c in &choices {
                if ui.selectable_label(c.op == entry.op, c.label).clicked() && c.op != entry.op {
                    picked = Some(c.op);
                }
            }
        });
    picked
}

/// Dispatch on the decoded value. Returns the primary control, for focus-after-add.
fn value_controls(
    ui: &mut egui::Ui,
    card: &Card,
    value: &Value,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) -> Option<egui::Response> {
    match value {
        Value::Ints(xs) if xs.len() == 1 => int_control(ui, card, xs[0], actions),
        Value::Bool { value } => {
            bool_control(ui, card, *value, actions);
            None
        }
        Value::Enum(vs) => {
            enum_control(ui, card, vs, actions);
            None
        }
        Value::Rarity(rs) => {
            rarity_control(ui, card, rs, list, actions);
            None
        }
        _ => Some(raw_value_control(ui, card, list, actions)),
    }
}

fn int_control(
    ui: &mut egui::Ui,
    card: &Card,
    current: i64,
    actions: &mut Vec<RowAction>,
) -> Option<egui::Response> {
    let mut v = current;
    let mut invalid = false;
    let mut changed = false;
    let mut primary = None;
    ui.horizontal(|ui| {
        let (min, max, log) = grammar::slider_range(card.kw.name);
        let s = widgets::themed_slider(ui, &mut v, min, max, log);
        let n = widgets::number_field(ui, card.salt("num"), &mut v, 64.0);
        invalid = n.invalid;
        changed = s.changed() || n.response.changed();
        primary = Some(n.response);
    });
    if invalid {
        hint(ui, WHOLE_NUMBER_HINT);
    }
    if changed
        && v != current
        && let Some(a) = int_edit(card.id, card.entry, v)
    {
        actions.push(a);
    }
    primary
}

fn bool_control(ui: &mut egui::Ui, card: &Card, current: bool, actions: &mut Vec<RowAction>) {
    let mut b = current;
    let mut changed = false;
    ui.horizontal(|ui| {
        changed = widgets::circular_toggle(ui, &mut b).changed();
        ui.label(
            RichText::new(if b { "Yes" } else { "No" })
                .size(14.0)
                .color(theme::VALUE_TEXT),
        );
    });
    if changed && b != current {
        let res = values_edit(card.id, card.entry, &Value::Bool { value: b });
        card.commit(ui, res, actions);
    }
}

fn enum_control(ui: &mut egui::Ui, card: &Card, current: &[String], actions: &mut Vec<RowAction>) {
    let ValueShape::Enum(options) = card.kw.value else {
        return;
    };
    if let [only] = current {
        let mut picked: Option<&str> = None;
        egui::ComboBox::from_id_salt(card.salt("enum"))
            .selected_text(only.as_str())
            .width(160.0)
            .show_ui(ui, |ui| {
                for o in options {
                    if ui.selectable_label(*o == only, *o).clicked() && *o != only {
                        picked = Some(o);
                    }
                }
            });
        if let Some(p) = picked {
            let res = values_edit(card.id, card.entry, &Value::Enum(vec![p.to_string()]));
            card.commit(ui, res, actions);
        }
        return;
    }
    let mut flags: Vec<bool> = options
        .iter()
        .map(|o| current.iter().any(|c| c == o))
        .collect();
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for (o, f) in options.iter().zip(flags.iter_mut()) {
            changed |= widgets::toggle_chip(ui, o, f).changed();
        }
    });
    if changed {
        let sel: Vec<String> = options
            .iter()
            .zip(&flags)
            .filter(|(_, f)| **f)
            .map(|(o, _)| o.to_string())
            .collect();
        let res = values_edit(card.id, card.entry, &Value::Enum(sel));
        card.commit(ui, res, actions);
    }
}

/// Four rarity chips; returns the new selection (table order) when the user toggled one.
fn rarity_chips(ui: &mut egui::Ui, selected: &[&str]) -> Option<Vec<&'static str>> {
    let mut flags: Vec<bool> = RARITIES.iter().map(|r| selected.contains(r)).collect();
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for (r, f) in RARITIES.iter().zip(flags.iter_mut()) {
            changed |= widgets::toggle_chip(ui, r, f).changed();
        }
    });
    changed.then(|| {
        RARITIES
            .iter()
            .zip(flags)
            .filter(|(_, f)| *f)
            .map(|(r, _)| *r)
            .collect()
    })
}

fn rarity_control(
    ui: &mut egui::Ui,
    card: &Card,
    current: &[&'static str],
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    if let Some(sel) = rarity_chips(ui, current) {
        match rarity_edit(card.id, card.entry, &sel) {
            RarityEdit::None => {}
            RarityEdit::Edit(a) => actions.push(a),
            RarityEdit::Remove(a) => {
                actions.push(a);
                let p = PendingRow {
                    block: card.id,
                    keyword: card.kw,
                    op: None,
                    kind: LineKind::Condition,
                };
                if !list.pending.contains(&p) {
                    list.pending.push(p);
                }
            }
        }
    }
    if current.len() > 1 {
        ui.label(
            RichText::new("Several rarities match any of them")
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::LABEL_TAN),
        );
    }
}

/// A text field for a value the codec cannot structure; commits `Values(text)`.
fn raw_value_control(
    ui: &mut egui::Ui,
    card: &Card,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) -> egui::Response {
    let tid = ui.make_persistent_id(card.salt("rawv"));
    let current = current_values(card.entry);
    let width = ui.available_width().max(80.0);
    let (resp, commit) = draft_field(ui, tid, &current, width, list, (card.id, card.entry.line));
    if let Some(text) = commit
        && let Some(a) = raw_edit(card.id, card.entry, &text)
    {
        actions.push(a);
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::rule_list::show_list;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect, UiBuilder};

    fn entries_of(src: &str) -> (WorkingFilter, BlockId, Vec<BodyEntry>) {
        let wf = WorkingFilter::from_text(src);
        let id = wf.rows()[0].id;
        let e = wf.body_view(id);
        (wf, id, e)
    }

    fn headless_ctx() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.set_fonts(egui::FontDefinitions::empty());
        ctx
    }

    fn input(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(460.0, 900.0))),
            events,
            ..Default::default()
        }
    }

    fn click_events(pos: Pos2) -> Vec<Event> {
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::default(),
            },
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::default(),
            },
        ]
    }

    fn frame_of(
        ctx: &egui::Context,
        raw: RawInput,
        wf: &WorkingFilter,
        list: &mut ListUi,
        classes: Option<&ClassCatalog>,
    ) -> Vec<RowAction> {
        let visible: Vec<usize> = (0..wf.rows().len()).collect();
        let mut actions = Vec::new();
        ctx.run_ui(raw, |ui| {
            let area = Rect::from_min_size(Pos2::ZERO, vec2(460.0, 900.0));
            let mut child = ui.new_child(UiBuilder::new().max_rect(area));
            actions = show_list(&mut child, wf, &visible, list, true, classes).actions;
        })
        .drop_without_applying_deltas();
        actions
    }

    #[test]
    fn expanded_scalar_conditions_are_idle() {
        let wf = WorkingFilter::from_text(
            "Show\n    ItemLevel >= 70\n    Corrupted True\n    Rarity Rare\n    HasInfluence Shaper\n    FutureKw 3\n",
        );
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[0].id);
        for _ in 0..3 {
            let a = frame_of(&ctx, input(vec![]), &wf, &mut list, None);
            assert!(a.is_empty(), "idle frame emitted {a:?}");
        }
    }

    #[test]
    fn placeholder_text_is_gone_and_empty_state_exists() {
        let src = include_str!("rule_list.rs");
        let needle = ["Editing conditions arrives", " in a later update."].concat();
        assert!(!src.contains(&needle));
        assert!(include_str!("body.rs").contains(NO_CONDITIONS));
    }

    #[test]
    fn remove_x_of_item_level_card_emits_remove_line() {
        let wf = WorkingFilter::from_text("Show\n    ItemLevel >= 70\n");
        let id = wf.rows()[0].id;
        let mut found = None;
        'scan: for y in (20..300).step_by(8) {
            for x in (180..456).step_by(8) {
                let ctx = headless_ctx();
                let mut list = ListUi::default();
                list.expanded = Some(id);
                frame_of(&ctx, input(vec![]), &wf, &mut list, None);
                let a = frame_of(
                    &ctx,
                    input(click_events(Pos2::new(x as f32, y as f32))),
                    &wf,
                    &mut list,
                    None,
                );
                if let Some(r) = a
                    .into_iter()
                    .find(|a| matches!(a, RowAction::RemoveLine { .. }))
                {
                    found = Some(r);
                    break 'scan;
                }
            }
        }
        match found {
            Some(RowAction::RemoveLine {
                id: got,
                line,
                expect,
            }) => {
                assert_eq!(got, id);
                assert_eq!(line, 1);
                assert_eq!(expect, "    ItemLevel >= 70");
            }
            other => panic!("no RemoveLine emitted: {other:?}"),
        }
    }

    #[test]
    fn int_and_operator_edits() {
        let (_, id, e) = entries_of("Show\n    ItemLevel >= 70\n");
        assert_eq!(
            int_edit(id, &e[0], 75),
            Some(RowAction::EditLine {
                id,
                line: 1,
                expect: e[0].text.clone(),
                patch: LinePatch::Values("75".into()),
            })
        );
        assert_eq!(int_edit(id, &e[0], 70), None);
        assert_eq!(
            op_edit(id, &e[0], Some(Operator::Le)),
            Some(RowAction::EditLine {
                id,
                line: 1,
                expect: e[0].text.clone(),
                patch: LinePatch::Operator(Some(Operator::Le)),
            })
        );
        assert_eq!(op_edit(id, &e[0], Some(Operator::Ge)), None);
    }

    #[test]
    fn rarity_edits() {
        let (_, id, e) = entries_of("Show\n    Rarity Rare\n");
        let patch_of = |r: RarityEdit| match r {
            RarityEdit::Edit(RowAction::EditLine { patch, .. }) => Some(patch),
            _ => None,
        };
        assert_eq!(
            patch_of(rarity_edit(id, &e[0], &["Magic"])),
            Some(LinePatch::Values("Magic".into()))
        );
        assert_eq!(rarity_edit(id, &e[0], &["Rare"]), RarityEdit::None);
        let (_, id, e) = entries_of("Show\n    Rarity >= Magic\n");
        assert_eq!(
            patch_of(rarity_edit(id, &e[0], &["Magic", "Rare"])),
            Some(LinePatch::Both(None, "Magic Rare".into()))
        );
        let (_, id, e) = entries_of("Show\n    Rarity Magic\n");
        assert_eq!(
            patch_of(rarity_edit(id, &e[0], &["Normal", "Magic", "Rare"])),
            Some(LinePatch::Values("Normal Magic Rare".into()))
        );
        assert!(matches!(
            rarity_edit(id, &e[0], &[]),
            RarityEdit::Remove(RowAction::RemoveLine { .. })
        ));
    }

    #[test]
    fn raw_value_edit_commits_only_a_changed_nonempty_text() {
        let (_, id, e) = entries_of("Show\n    Corrupted true\n");
        assert_eq!(e[0].values, ["true"]);
        assert!(matches!(
            raw_edit(id, &e[0], "True"),
            Some(RowAction::EditLine {
                patch: LinePatch::Values(ref t),
                ..
            }) if t == "True"
        ));
        assert_eq!(raw_edit(id, &e[0], "true"), None);
        assert_eq!(raw_edit(id, &e[0], "   "), None);
    }

    #[test]
    fn opaque_line_is_editable() {
        let (wf, id, e) = entries_of("Show\n    FutureKw 3\n");
        assert_eq!(e[0].kind, EntryKind::Opaque);
        assert!(wf.body_editable(id));
        assert_eq!(
            raw_line_edit(id, &e[0], "FutureKw 4"),
            Some(RowAction::EditLine {
                id,
                line: 1,
                expect: e[0].text.clone(),
                patch: LinePatch::Whole("FutureKw 4".into()),
            })
        );
        assert_eq!(raw_line_edit(id, &e[0], "FutureKw 3"), None);
        assert_eq!(raw_line_edit(id, &e[0], "   "), None);
        assert_eq!(raw_line_edit(id, &e[0], "A\nB"), None);
    }

    #[test]
    fn uneditable_block_renders_read_only_without_actions() {
        // An invalid commented-out block cannot be enabled, so its lines stay read-only.
        let wf =
            WorkingFilter::from_text("#Show\n#    Rarity Banana\n\nShow\n    Class \"Rings\"\n");
        let id = wf.rows()[0].id;
        assert!(!wf.body_editable(id));
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(id);
        for _ in 0..2 {
            assert!(frame_of(&ctx, input(vec![]), &wf, &mut list, None).is_empty());
        }
    }

    #[test]
    fn refused_edit_error_is_found_for_its_line_only() {
        let (_, id, _) = entries_of("Show\n    FutureKw 3\n");
        let mut list = ListUi::default();
        list.line_error = Some((id, 1, "nope".into()));
        assert_eq!(error_for(&list, id, 1), Some("nope"));
        assert_eq!(error_for(&list, id, 2), None);
    }
}
