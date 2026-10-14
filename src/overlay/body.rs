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

use egui::{Align, Layout, RichText, vec2};

use crate::filter::keywords::{COLOURS, Keyword, Kind, RARITIES, SHAPES, ValueShape};
use crate::filter::tokenize::Operator;
use crate::filter::{BlockId, BlockKeyword, LinePatch};

use super::classes::ClassCatalog;
use super::grammar::{self, BoolOrName, Effect, EncodeError, Minimap, SoundRef, Value};
use super::rule_list::{ListUi, PendingRow};
use super::rules::{BodyEntry, EntryKind, LineKind, RowAction, RuleRow, StyleState, WorkingFilter};
use super::theme;
use super::widgets;
use crate::models::config::Style;

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
/// operator (`<` `<=` `>` `>=`), so that case drops it in the same commit
/// (`Rarity Normal Magic Rare`). Every other operator is kept as written: a negation
/// (`!=`/`!`) with several rarities means "none of these" (PARSE-04).
pub(crate) fn rarity_edit(id: BlockId, entry: &BodyEntry, selected: &[&str]) -> RarityEdit {
    if selected.is_empty() {
        return RarityEdit::Remove(remove(id, entry));
    }
    let text = selected.join(" ");
    if text == current_values(entry) {
        return RarityEdit::None;
    }
    let ordering = matches!(
        entry.op,
        Some(Operator::Lt | Operator::Le | Operator::Gt | Operator::Ge)
    );
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

pub(crate) fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::LABEL_TAN));
}

pub(crate) fn hint(ui: &mut egui::Ui, text: &str) {
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
    let mut picked = None;
    ui.horizontal(|ui| {
        caption(ui, "CONDITIONS");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let btn = ui.add_enabled(
                ctx.editable,
                egui::Button::new(RichText::new("+ condition").size(theme::ROW_SUMMARY_SIZE)),
            );
            picked = picker_popup(ui, &btn, &picker_entries(""), true, &|_| None);
        });
    });
    if let Some(kw) = picked
        && let Some(a) = pick_condition(id, kw, list)
    {
        actions.push(a);
    }
    ui.spacing_mut().item_spacing.y = 4.0;

    let lines: Vec<&BodyEntry> = ctx
        .entries
        .iter()
        .filter(|e| is_condition_side(e))
        .collect();
    let pending: Vec<PendingRow> = list
        .pending
        .iter()
        .filter(|p| p.block == id)
        .cloned()
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
    if lines.is_empty() && pending.is_empty() {
        ui.label(
            RichText::new(NO_CONDITIONS)
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::LABEL_TAN),
        );
    }
    let real = lines.len();
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
    for (i, p) in pending.iter().enumerate() {
        pending_card(ui, ctx, p, real + i, list, actions);
    }
}

// ---------------------------------------------------------------------------------------------
// Picker and pending rows
// ---------------------------------------------------------------------------------------------

type Groups = Vec<(&'static str, Vec<&'static Keyword>)>;

/// The label a picker row shows: the display name.
pub(crate) fn picker_label(kw: &Keyword) -> String {
    grammar::display_name(kw.name)
}

/// Keep the keywords whose name or display name contains `query` (case-insensitive); empty
/// groups are dropped. An empty query keeps everything.
pub(crate) fn filter_groups(
    groups: &[(&'static str, Vec<&'static Keyword>)],
    query: &str,
) -> Groups {
    let q = query.trim().to_lowercase();
    groups
        .iter()
        .map(|(g, ks)| {
            let kept: Vec<&'static Keyword> = ks
                .iter()
                .filter(|k| {
                    q.is_empty()
                        || k.name.to_lowercase().contains(&q)
                        || grammar::display_name(k.name).to_lowercase().contains(&q)
                })
                .copied()
                .collect();
            (*g, kept)
        })
        .filter(|(_, ks)| !ks.is_empty())
        .collect()
}

/// The `+ condition` picker content for a search query.
pub(crate) fn picker_entries(query: &str) -> Groups {
    filter_groups(&grammar::condition_groups(), query)
}

/// A searchable, grouped keyword picker anchored to `button` (shared with `+ action`). Returns
/// the keyword the user picked this frame. Keywords for which `disabled` gives a reason are shown
/// greyed with that reason as their tooltip.
pub(crate) fn picker_popup(
    ui: &mut egui::Ui,
    button: &egui::Response,
    groups: &[(&'static str, Vec<&'static Keyword>)],
    searchable: bool,
    disabled: &dyn Fn(&Keyword) -> Option<&'static str>,
) -> Option<&'static Keyword> {
    let popup_id = egui::Popup::default_response_id(button);
    let query_id = popup_id.with("query");
    let focus_id = popup_id.with("focused");
    let width = (ui.clip_rect().width() - 16.0).clamp(240.0, 440.0);
    let ctx = ui.ctx().clone();
    let hint_text = match groups.first().and_then(|g| g.1.first()) {
        Some(k) if k.kind == Kind::Action => "Find an action\u{2026}",
        _ => "Find a condition\u{2026}",
    };
    let inner = egui::Popup::from_toggle_button_response(button)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .width(width)
        .show(|ui| {
            let mut query = ui
                .data_mut(|d| d.get_temp::<String>(query_id))
                .unwrap_or_default();
            if searchable {
                let r = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .hint_text(hint_text)
                        .desired_width(f32::INFINITY),
                );
                if !ui.data(|d| d.get_temp::<bool>(focus_id)).unwrap_or(false) {
                    r.request_focus();
                    ui.data_mut(|d| d.insert_temp(focus_id, true));
                }
            }
            ui.data_mut(|d| d.insert_temp(query_id, query.clone()));
            let filtered = filter_groups(groups, &query);
            let searching = !query.trim().is_empty();
            let mut picked: Option<&'static Keyword> = None;
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    if filtered.is_empty() {
                        caption(ui, "Nothing matches");
                    }
                    for (gi, (label, kws)) in filtered.iter().enumerate() {
                        let mut header = egui::CollapsingHeader::new(
                            RichText::new(*label)
                                .size(theme::ROW_SUMMARY_SIZE)
                                .color(theme::LABEL_TAN),
                        )
                        .id_salt((popup_id, *label))
                        .default_open(gi == 0);
                        if searching {
                            header = header.open(Some(true));
                        }
                        header.show(ui, |ui| {
                            for kw in kws {
                                let reason = disabled(kw);
                                let text = RichText::new(picker_label(kw)).size(14.0).color(
                                    if reason.is_some() {
                                        theme::DISABLED_TEXT
                                    } else {
                                        theme::VALUE_TEXT
                                    },
                                );
                                let r = ui
                                    .add_enabled_ui(reason.is_none(), |ui| {
                                        ui.selectable_label(false, text)
                                    })
                                    .inner;
                                match reason {
                                    Some(msg) => {
                                        r.on_disabled_hover_text(msg);
                                    }
                                    None if r.clicked() => picked = Some(*kw),
                                    None => {}
                                }
                            }
                        });
                    }
                });
            if picked.is_some() {
                egui::Popup::close_id(ui.ctx(), popup_id);
            }
            picked
        });
    if !egui::Popup::is_id_open(&ctx, popup_id) {
        ctx.data_mut(|d| {
            d.remove::<String>(query_id);
            d.remove::<bool>(focus_id);
        });
    }
    inner.and_then(|r| r.inner)
}

/// What picking `kw` does: a keyword with a sensible default is added at once; list shapes and
/// Rarity become a pending row and nothing is written until the first entry exists.
pub(crate) fn pick_condition(
    id: BlockId,
    kw: &'static Keyword,
    list: &mut ListUi,
) -> Option<RowAction> {
    if let Some(text) = grammar::default_line(kw) {
        return Some(RowAction::AddLine {
            id,
            kind: LineKind::Condition,
            text,
        });
    }
    let p = PendingRow {
        block: id,
        keyword: kw,
        op: (kw.name == "BaseType").then_some(Operator::EqEq),
        kind: LineKind::Condition,
    };
    // At most one pending row per (block, keyword), whatever its operator: a second one would
    // share the card's egui id, so one chip click would write two lines (WR-01, iteration 3).
    if !has_pending(list, id, kw) {
        list.pending.push(p);
    }
    None
}

/// Whether `block` already has a pending row for `kw`, regardless of the row's operator.
fn has_pending(list: &ListUi, block: BlockId, kw: &'static Keyword) -> bool {
    list.pending
        .iter()
        .any(|q| q.block == block && q.keyword.name == kw.name)
}

/// Queue the refill row from `rarity_refill`. Its operator takes over from any row already
/// pending for the same keyword on the same block, so at most one exists (WR-01, iteration 3).
pub(crate) fn queue_refill(list: &mut ListUi, p: PendingRow) {
    list.pending
        .retain(|q| !(q.block == p.block && q.keyword.name == p.keyword.name));
    list.pending.push(p);
}

/// The pending row queued when the last rarity chip is unchecked. It carries the removed
/// line's operator, so `Rarity != Normal`, uncheck Normal, pick Magic writes
/// `Rarity != Magic` rather than the opposite `Rarity Magic` (WR-01, iteration 2).
pub(crate) fn rarity_refill(id: BlockId, kw: &'static Keyword, entry: &BodyEntry) -> PendingRow {
    PendingRow {
        block: id,
        keyword: kw,
        op: entry.op,
        kind: LineKind::Condition,
    }
}

/// The `AddLine` that turns a pending row plus its first entries into a real condition.
pub(crate) fn pending_first_entry(p: &PendingRow, items: &[String]) -> Option<RowAction> {
    let value = match p.keyword.value {
        ValueShape::QuotedList => Value::List(items.to_vec()),
        // No count: the line reads "any of these". A bare `1` would be an equality test, so the
        // line would stop matching an item carrying two of the listed mods (WR-05).
        ValueShape::CountThenNames => Value::CountNames {
            count: None,
            names: items.to_vec(),
        },
        ValueShape::Rarity => Value::Rarity(
            items
                .iter()
                .filter_map(|i| RARITIES.iter().find(|r| **r == i.as_str()).copied())
                .collect(),
        ),
        _ => return None,
    };
    let enc = grammar::encode(&value).ok()?;
    let name = p.keyword.name;
    // Same rule as `rarity_edit`: an ordering operator means nothing on a multi-rarity line, so
    // only it is dropped; a negation or equality is kept.
    let op = match p.op {
        Some(Operator::Lt | Operator::Le | Operator::Gt | Operator::Ge) if matches!(&value, Value::Rarity(r) if r.len() >= 2) => {
            None
        }
        o => o,
    };
    let text = match op {
        Some(op) => format!("{name} {} {enc}", op.as_str()),
        None => format!("{name} {enc}"),
    };
    Some(RowAction::AddLine {
        id: p.block,
        kind: p.kind,
        text,
    })
}

/// A not-yet-written condition: keyword label, a discard X and the control for its first entry.
fn pending_card(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    p: &PendingRow,
    idx: usize,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let mut discard = false;
    let mut items: Option<Vec<String>> = None;
    // Set when the first entry could not be written, so the refusal is shown instead of the entry
    // vanishing (WR-06).
    let bad_id = ui.id().with((p.block, p.keyword.name, "pending-bad"));
    let bad = ui.data(|d| d.get_temp::<bool>(bad_id).unwrap_or(false));
    ui.push_id((p.block, p.keyword.name, "pending"), |ui| {
        card_frame(idx).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
            ui.horizontal(|ui| {
                ui.add_sized(
                    [LABEL_W, 24.0],
                    egui::Label::new(
                        RichText::new(grammar::display_name(p.keyword.name))
                            .size(14.0)
                            .color(theme::VALUE_TEXT),
                    )
                    .truncate(),
                )
                .on_hover_text(p.keyword.name);
                // Show the operator the first entry will be written with, so a carried-over
                // negation is never silent (WR-01, iteration 2).
                if p.op.is_some() {
                    ui.label(
                        RichText::new(grammar::operator_label(p.keyword, p.op, false))
                            .size(theme::ROW_SUMMARY_SIZE)
                            .color(theme::LABEL_TAN),
                    );
                }
                discard = widgets::remove_x_button(ui, "Discard this condition").clicked();
            });
            caption(ui, "Add an entry to create this condition");
            let salt: Salt = (p.block, p.keyword.name, usize::MAX, "pending");
            items = match p.keyword.value {
                ValueShape::Rarity => {
                    rarity_chips(ui, &[]).map(|r| r.iter().map(|s| s.to_string()).collect())
                }
                _ => match ctx.classes {
                    Some(cat) if p.keyword.name == "Class" => class_picker(ui, salt, cat, &[]),
                    _ => list_chips(ui, salt, &[], chip_hint(p.keyword)),
                },
            };
            if bad {
                hint(ui, BAD_TEXT_HINT);
            }
        });
    });
    if discard {
        ui.data_mut(|d| d.remove::<bool>(bad_id));
        list.pending.retain(|q| q != p);
    } else if let Some(items) = items.filter(|i| !i.is_empty()) {
        match pending_first_entry(p, &items) {
            Some(a) => {
                ui.data_mut(|d| d.remove::<bool>(bad_id));
                actions.push(a);
                list.pending.retain(|q| q != p);
            }
            None => ui.data_mut(|d| {
                d.insert_temp(bad_id, true);
            }),
        }
    }
}

struct Card<'a> {
    id: BlockId,
    entry: &'a BodyEntry,
    kw: &'static Keyword,
    bad_id: egui::Id,
    num_bad_id: egui::Id,
    classes: Option<&'a ClassCatalog>,
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
            num_bad_id: ui.make_persistent_id("bad-int"),
            classes: ctx.classes,
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
        Value::Ints(xs) => {
            ints_control(ui, card, xs, actions);
            None
        }
        Value::List(names) => {
            list_control(ui, card, names, actions);
            None
        }
        Value::CountNames { count, names } => {
            count_names_control(ui, card, *count, names, actions);
            None
        }
        Value::Sockets {
            count: Some(c),
            colours,
        } => {
            sockets_control(ui, card, *c, colours, actions);
            None
        }
        Value::BoolOrName(b) => {
            bool_or_name_control(ui, card, b, list, actions);
            None
        }
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
                queue_refill(list, rarity_refill(card.id, card.kw, card.entry));
            }
        }
    }
    if current.len() > 1 {
        let note = if matches!(card.entry.op, Some(Operator::Ne | Operator::Bang)) {
            "Several rarities exclude all of them"
        } else {
            "Several rarities match any of them"
        };
        ui.label(
            RichText::new(note)
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

// ---------------------------------------------------------------------------------------------
// List-shaped controls
// ---------------------------------------------------------------------------------------------

type Salt = (BlockId, &'static str, usize, &'static str);

/// Edit for a changed list of quoted names; an empty list removes the line.
pub(crate) fn list_edit(
    id: BlockId,
    entry: &BodyEntry,
    new: &[String],
) -> Result<Option<RowAction>, EncodeError> {
    values_edit(id, entry, &Value::List(new.to_vec()))
}

/// `items` with `class` added at the end (when checked and absent) or removed (when unchecked),
/// keeping the order of everything else.
pub(crate) fn class_toggle(items: &[String], class: &str, checked: bool) -> Vec<String> {
    if checked {
        let mut out = items.to_vec();
        if !out.iter().any(|c| c == class) {
            out.push(class.to_string());
        }
        out
    } else {
        items.iter().filter(|c| *c != class).cloned().collect()
    }
}

/// Edit for a `count "name" ...` line. No names left removes the line.
pub(crate) fn count_names_edit(
    id: BlockId,
    entry: &BodyEntry,
    count: Option<i64>,
    names: &[String],
) -> Result<Option<RowAction>, EncodeError> {
    values_edit(
        id,
        entry,
        &Value::CountNames {
            count,
            names: names.to_vec(),
        },
    )
}

/// Edit for a socket line: a count plus colour slots (`5GRG`).
pub(crate) fn sockets_edit(
    id: BlockId,
    entry: &BodyEntry,
    count: i64,
    colours: &str,
) -> Option<RowAction> {
    values_edit(
        id,
        entry,
        &Value::Sockets {
            count: Some(count),
            colours: colours.to_string(),
        },
    )
    .ok()
    .flatten()
}

/// `colours` with slot `i` set to `choice`; `None` (Any) drops the slot.
pub(crate) fn set_slot(colours: &str, i: usize, choice: Option<char>) -> String {
    let mut v: Vec<char> = colours.chars().collect();
    match (choice, v.get_mut(i)) {
        (Some(c), Some(slot)) => *slot = c,
        (None, Some(_)) => {
            v.remove(i);
        }
        _ => {}
    }
    v.into_iter().collect()
}

/// `Any` writes `True`; `Named` writes the quoted name.
pub(crate) fn bool_or_name_edit(
    id: BlockId,
    entry: &BodyEntry,
    any: bool,
    name: &str,
) -> Result<Option<RowAction>, EncodeError> {
    let v = if any {
        BoolOrName::Bool(true)
    } else {
        BoolOrName::Name(name.to_string())
    };
    values_edit(id, entry, &Value::BoolOrName(v))
}

/// Edit for a multi-value whole-number line (`ItemLevel 60 65`).
pub(crate) fn ints_edit(id: BlockId, entry: &BodyEntry, xs: &[i64]) -> Option<RowAction> {
    values_edit(id, entry, &Value::Ints(xs.to_vec()))
        .ok()
        .flatten()
}

fn chip_hint(kw: &Keyword) -> &'static str {
    if kw.name == "BaseType" {
        "Add a base type\u{2026}"
    } else if kw.name.contains("Mod") {
        "Add a mod\u{2026}"
    } else {
        "Add an entry\u{2026}"
    }
}

/// A removable selected-value chip; true when its X was clicked.
fn selected_chip(ui: &mut egui::Ui, label: &str) -> bool {
    let mut removed = false;
    egui::Frame::new()
        .fill(theme::INPUT_FILL)
        .stroke(egui::Stroke::new(1.0, theme::CONTROL_GLYPH))
        .inner_margin(egui::Margin {
            left: 4,
            right: 0,
            top: 0,
            bottom: 0,
        })
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(label)
                        .size(theme::ROW_SUMMARY_SIZE)
                        .color(theme::VALUE_TEXT),
                );
                removed = widgets::remove_x_button(ui, &format!("Remove {label}")).clicked();
            });
        });
    removed
}

/// A chip list with the quoted-paste add field. Returns the new list when it changed.
fn list_chips(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash,
    items: &[String],
    hint_text: &str,
) -> Option<Vec<String>> {
    let out = widgets::chip_list(ui, salt, items, hint_text, 160.0);
    if let Some(msg) = &out.rejected {
        hint(ui, msg);
    }
    let mut next = items.to_vec();
    let mut changed = false;
    if let Some(i) = out.removed
        && i < next.len()
    {
        next.remove(i);
        changed = true;
    }
    for a in out.added {
        if !next.contains(&a) {
            next.push(a);
            changed = true;
        }
    }
    changed.then_some(next)
}

/// The grouped class picker: chips for what is selected, then one collapsible group per
/// catalogue group (labels and members come from config.json). Natural height, no inner scroll
/// area (Pitfall 8). Returns the new class list when the user changed it.
fn class_picker(
    ui: &mut egui::Ui,
    salt: Salt,
    catalog: &ClassCatalog,
    selected: &[String],
) -> Option<Vec<String>> {
    let mut result = None;
    let unknown: Vec<&String> = selected.iter().filter(|c| !catalog.contains(c)).collect();
    let known: Vec<&String> = selected.iter().filter(|c| catalog.contains(c)).collect();
    if !selected.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for c in unknown.iter().chain(known.iter().take(6)) {
                if selected_chip(ui, c) {
                    result = Some(class_toggle(selected, c, false));
                }
            }
        });
        if known.len() > 6 {
            caption(ui, &format!("+{} more", known.len() - 6));
        }
    }
    for (label, classes) in catalog.groups() {
        let n = classes.iter().filter(|c| selected.contains(c)).count();
        egui::CollapsingHeader::new(
            RichText::new(format!("{label} ({n})"))
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::LABEL_TAN),
        )
        .id_salt((salt, *label))
        .default_open(false)
        .show(ui, |ui| {
            for c in classes {
                let mut on = selected.contains(c);
                if ui.checkbox(&mut on, c.as_str()).changed() {
                    result = Some(class_toggle(selected, c, on));
                }
            }
        });
    }
    result
}

fn list_control(ui: &mut egui::Ui, card: &Card, names: &[String], actions: &mut Vec<RowAction>) {
    let next = match card.classes {
        Some(cat) if card.kw.name == "Class" => class_picker(ui, card.salt("class"), cat, names),
        _ => list_chips(ui, card.salt("chips"), names, chip_hint(card.kw)),
    };
    if let Some(next) = next {
        let res = list_edit(card.id, card.entry, &next);
        card.commit(ui, res, actions);
    }
}

fn ints_control(ui: &mut egui::Ui, card: &Card, xs: &[i64], actions: &mut Vec<RowAction>) {
    let items: Vec<String> = xs.iter().map(i64::to_string).collect();
    let out = widgets::chip_list(
        ui,
        card.salt("ints"),
        &items,
        "Add a whole number\u{2026}",
        160.0,
    );
    let mut next = xs.to_vec();
    let mut changed = false;
    let mut bad = ui
        .data(|d| d.get_temp::<bool>(card.num_bad_id))
        .unwrap_or(false);
    if let Some(i) = out.removed
        && i < next.len()
    {
        next.remove(i);
        changed = true;
    }
    for a in out.added {
        match a.trim().parse::<i64>() {
            Ok(n) => {
                next.push(n);
                changed = true;
                bad = false;
            }
            Err(_) => bad = true,
        }
    }
    ui.data_mut(|d| d.insert_temp(card.num_bad_id, bad));
    if bad {
        hint(ui, WHOLE_NUMBER_HINT);
    }
    if changed && let Some(a) = ints_edit(card.id, card.entry, &next) {
        actions.push(a);
    }
}

fn count_names_control(
    ui: &mut egui::Ui,
    card: &Card,
    count: Option<i64>,
    names: &[String],
    actions: &mut Vec<RowAction>,
) {
    let mut new_count = count;
    if let Some(c) = count {
        let mut v = c;
        let mut changed = false;
        ui.horizontal(|ui| {
            let lead = if card.entry.op == Some(Operator::Ge) {
                "At least"
            } else {
                "Count"
            };
            ui.label(RichText::new(lead).size(14.0).color(theme::VALUE_TEXT));
            let s = widgets::themed_slider(ui, &mut v, 0, 6, false);
            let n = widgets::number_field(ui, card.salt("count"), &mut v, 40.0);
            changed = s.changed() || n.response.changed();
            ui.label(
                RichText::new("of these:")
                    .size(14.0)
                    .color(theme::VALUE_TEXT),
            );
        });
        if changed && v != c {
            new_count = Some(v);
        }
    } else {
        caption(ui, "Any of these:");
    }
    let next = list_chips(ui, card.salt("names"), names, chip_hint(card.kw));
    if next.is_none() && new_count == count {
        return;
    }
    let res = count_names_edit(
        card.id,
        card.entry,
        new_count,
        next.as_deref().unwrap_or(names),
    );
    card.commit(ui, res, actions);
}

const SLOT_CHOICES: [(&str, Option<char>); 7] = [
    ("Any", None),
    ("R", Some('R')),
    ("G", Some('G')),
    ("B", Some('B')),
    ("W", Some('W')),
    ("A", Some('A')),
    ("D", Some('D')),
];

fn sockets_control(
    ui: &mut egui::Ui,
    card: &Card,
    count: i64,
    colours: &str,
    actions: &mut Vec<RowAction>,
) {
    let mut v = count;
    let mut new_colours = colours.to_string();
    let mut changed = false;
    ui.horizontal(|ui| {
        let s = widgets::themed_slider(ui, &mut v, 0, 6, false);
        let n = widgets::number_field(ui, card.salt("count"), &mut v, 40.0);
        changed = s.changed() || n.response.changed();
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for (i, ch) in colours.chars().enumerate() {
            let mut picked: Option<Option<char>> = None;
            egui::ComboBox::from_id_salt((card.salt("slot"), i))
                .selected_text(ch.to_string())
                .width(56.0)
                .show_ui(ui, |ui| {
                    for (label, c) in SLOT_CHOICES {
                        if ui.selectable_label(c == Some(ch), label).clicked() && c != Some(ch) {
                            picked = Some(c);
                        }
                    }
                });
            if let Some(c) = picked {
                new_colours = set_slot(&new_colours, i, c);
                changed = true;
            }
        }
        let small = |t: &str| egui::Button::new(RichText::new(t).size(theme::ROW_SUMMARY_SIZE));
        if colours.chars().count() < 6 && ui.add(small("+ socket")).clicked() {
            new_colours.push('R');
            changed = true;
        }
        if !colours.is_empty() && ui.add(small("Remove")).clicked() {
            new_colours.pop();
            changed = true;
        }
    });
    if changed
        && (v != count || new_colours != colours)
        && let Some(a) = sockets_edit(card.id, card.entry, v, &new_colours)
    {
        actions.push(a);
    }
}

fn bool_or_name_control(
    ui: &mut egui::Ui,
    card: &Card,
    current: &BoolOrName,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let named_id = ui.make_persistent_id(card.salt("named"));
    let is_name = matches!(current, BoolOrName::Name(_));
    let named = ui.data(|d| d.get_temp::<bool>(named_id)).unwrap_or(is_name);
    let mut any = !named && matches!(current, BoolOrName::Bool(true));
    let mut nm = named;
    let (mut any_changed, mut nm_changed) = (false, false);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        any_changed = widgets::toggle_chip(ui, "Any", &mut any).changed();
        nm_changed = widgets::toggle_chip(ui, "Named", &mut nm).changed();
    });
    let mut show_name = named;
    if any_changed && any {
        ui.data_mut(|d| d.insert_temp(named_id, false));
        show_name = false;
        let res = bool_or_name_edit(card.id, card.entry, true, "");
        card.commit(ui, res, actions);
    } else if nm_changed && nm {
        ui.data_mut(|d| d.insert_temp(named_id, true));
        show_name = true;
    }
    if show_name {
        let tid = ui.make_persistent_id(card.salt("name"));
        let cur = match current {
            BoolOrName::Name(n) => n.as_str(),
            BoolOrName::Bool(_) => "",
        };
        let width = ui.available_width().max(80.0);
        let (_, commit) = draft_field(ui, tid, cur, width, list, (card.id, card.entry.line));
        if let Some(text) = commit {
            let res = bool_or_name_edit(card.id, card.entry, false, &text);
            card.commit(ui, res, actions);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Pure action edits
// ---------------------------------------------------------------------------------------------

/// Colour edit. A 3-token line keeps its 3-token form while the alpha stays 255.
pub(crate) fn rgba_edit(id: BlockId, entry: &BodyEntry, new: [u8; 4]) -> Option<RowAction> {
    values_edit(
        id,
        entry,
        &Value::Rgba {
            rgba: new,
            had_alpha: entry.values.len() >= 4,
        },
    )
    .ok()
    .flatten()
}

/// Font size edit; `None` when unchanged.
pub(crate) fn font_edit(id: BlockId, entry: &BodyEntry, new: i64) -> Option<RowAction> {
    values_edit(id, entry, &Value::FontSize(new)).ok().flatten()
}

/// Minimap icon edit; `None` when the encoded value is unchanged.
pub(crate) fn minimap_edit(id: BlockId, entry: &BodyEntry, m: Minimap) -> Option<RowAction> {
    values_edit(id, entry, &Value::Minimap(m)).ok().flatten()
}

/// Beam edit; `None` when unchanged.
pub(crate) fn effect_edit(id: BlockId, entry: &BodyEntry, e: Effect) -> Option<RowAction> {
    values_edit(id, entry, &Value::Effect(e)).ok().flatten()
}

/// Built-in alert sound edit. A line written without a volume keeps none until it is touched.
pub(crate) fn sound_edit(
    id: BlockId,
    entry: &BodyEntry,
    sound: SoundRef,
    volume: Option<i64>,
) -> Option<RowAction> {
    values_edit(id, entry, &Value::Sound { id: sound, volume })
        .ok()
        .flatten()
}

/// Custom sound edit. A file name with quotes or control characters is refused with the hint.
pub(crate) fn custom_sound_edit(
    id: BlockId,
    entry: &BodyEntry,
    file: &str,
    volume: Option<i64>,
) -> Result<Option<RowAction>, String> {
    let file = file.trim();
    if file.is_empty() {
        return Ok(None);
    }
    values_edit(
        id,
        entry,
        &Value::CustomSound {
            file: file.to_string(),
            volume,
        },
    )
    .map_err(|_| BAD_TEXT_HINT.to_string())
}

/// Whether an optional-boolean action line currently reads as on (bare or `True`).
fn optional_bool_on(entry: &BodyEntry) -> bool {
    !matches!(entry.values.first().map(String::as_str), Some("False"))
}

/// Toggle edit for `DisableDropSound`-style lines. A bare line is never given an explicit `True`
/// unless the user changes it.
pub(crate) fn optional_bool_edit(id: BlockId, entry: &BodyEntry, on: bool) -> Option<RowAction> {
    (on != optional_bool_on(entry)).then(|| {
        edit(
            id,
            entry,
            LinePatch::Values(if on { "True" } else { "False" }.to_string()),
        )
    })
}

/// Whether `style` writes the `keyword` action line.
pub(crate) fn style_owned(style: &Style, keyword: &str) -> bool {
    crate::emitter::style_action_lines(style)
        .iter()
        .any(|l| l.split_whitespace().next() == Some(keyword))
}

/// The action keywords already present in the block (each is single-instance).
pub(crate) fn added_actions(entries: &[BodyEntry]) -> Vec<&'static str> {
    entries
        .iter()
        .filter(|e| e.kind == EntryKind::Action)
        .filter_map(|e| e.keyword.map(|k| k.name))
        .collect()
}

/// The hint under a font size outside the range the game renders readably.
pub(crate) fn font_hint(size: i64) -> Option<&'static str> {
    (!(18..=45).contains(&size)).then_some("Readable in game: 18-45")
}

/// What picking an action does: it is added at once with its default value.
pub(crate) fn pick_action(id: BlockId, kw: &'static Keyword) -> Option<RowAction> {
    grammar::default_line(kw).map(|text| RowAction::AddLine {
        id,
        kind: LineKind::Action,
        text,
    })
}

// ---------------------------------------------------------------------------------------------
// The ACTIONS group
// ---------------------------------------------------------------------------------------------

const NO_ACTIONS: &str = "No actions: matching items keep the game's look.";
pub(crate) const SIZE_CHOICES: [(&str, i64); 4] =
    [("Large", 0), ("Medium", 1), ("Small", 2), ("Hide icon", -1)];

pub(crate) fn filter_colour(name: &str) -> egui::Color32 {
    let (r, g, b) = match name {
        "Red" => (255, 85, 85),
        "Green" => (80, 250, 123),
        "Blue" => (98, 140, 255),
        "Brown" => (160, 100, 50),
        "White" => (248, 248, 242),
        "Yellow" => (241, 250, 140),
        "Cyan" => (139, 233, 253),
        "Grey" => (150, 150, 150),
        "Orange" => (255, 184, 108),
        "Pink" => (255, 121, 198),
        "Purple" => (189, 147, 249),
        _ => (136, 133, 110),
    };
    egui::Color32::from_rgb(r, g, b)
}

pub(crate) fn dot(ui: &mut egui::Ui, colour: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 6.0, colour);
}

/// Segmented Show/Hide/Minimal selector (locked on a known style) plus `Continue to later rules`
/// (always editable: no style owns `Continue`).
fn disposition_line(ui: &mut egui::Ui, ctx: &BodyCtx, actions: &mut Vec<RowAction>) {
    let row = ctx.row;
    let id = row.id;
    let locked = matches!(row.style, StyleState::Known(_));
    ui.horizontal(|ui| {
        for (kw, word) in [
            (BlockKeyword::Show, "Show"),
            (BlockKeyword::Hide, "Hide"),
            (BlockKeyword::Minimal, "Minimal"),
        ] {
            let selected = row.keyword == kw;
            let (text_color, fill) = if locked {
                (theme::DISABLED_TEXT, theme::ROW_BASE)
            } else if selected {
                (theme::SECTION_CREAM, theme::HEADER_PLATE)
            } else {
                (theme::LABEL_TAN, theme::ROW_BASE)
            };
            let fill = if locked && selected {
                theme::HEADER_PLATE
            } else {
                fill
            };
            let btn = egui::Button::new(RichText::new(word).color(text_color)).fill(fill);
            if ui.add_enabled(!locked, btn).clicked() && !selected {
                actions.push(RowAction::SetKeyword(id, kw));
            }
        }
    });
    // A style never owns `Continue` (style_action_lines never emits it and the rewrite keeps it),
    // so the checkbox stays editable on a styled rule (WR-01).
    let mut cont = row.has_continue;
    let cb = egui::Checkbox::new(
        &mut cont,
        RichText::new("Continue to later rules").color(theme::LABEL_TAN),
    );
    if ui.add(cb).changed() {
        actions.push(RowAction::SetContinue(id, cont));
    }
    if locked {
        ui.add(
            egui::Label::new(
                RichText::new("The style decides Show or Hide. Remove the style to choose here.")
                    .size(theme::ROW_SUMMARY_SIZE)
                    .color(theme::TAB_ACTIVE),
            )
            .wrap(),
        );
    }
}

/// The ACTIONS section of an expanded row: disposition first, then one card per action line.
pub fn actions_group(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let id = ctx.row.id;
    let known = match &ctx.row.style {
        StyleState::Known(n) => Some(n.clone()),
        _ => None,
    };
    let added = added_actions(ctx.entries);
    let mut picked = None;
    ui.horizontal(|ui| {
        caption(ui, "ACTIONS");
        if known.is_none() {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let btn = ui.add_enabled(
                    ctx.editable,
                    egui::Button::new(RichText::new("+ action").size(theme::ROW_SUMMARY_SIZE)),
                );
                let disabled = |k: &Keyword| added.contains(&k.name).then_some("(added)");
                picked = picker_popup(ui, &btn, &grammar::action_groups(), false, &disabled);
            });
        }
    });
    if let Some(kw) = picked
        && let Some(a) = pick_action(id, kw)
    {
        actions.push(a);
    }
    ui.spacing_mut().item_spacing.y = 4.0;
    disposition_line(ui, ctx, actions);

    let lines: Vec<&BodyEntry> = ctx
        .entries
        .iter()
        .filter(|e| e.kind == EntryKind::Action)
        .collect();
    if lines.is_empty() {
        ui.label(
            RichText::new(NO_ACTIONS)
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::LABEL_TAN),
        );
        return;
    }
    let style = known.as_deref().and_then(|n| ctx.working.style(n));
    for (idx, e) in lines.into_iter().enumerate() {
        match e.keyword {
            Some(kw) if ctx.editable => {
                let lock = known.as_deref().map(|n| {
                    let owned = style.is_some_and(|s| style_owned(s, kw.name));
                    (n, owned)
                });
                action_card(ui, ctx, e, kw, idx, lock, list, actions);
            }
            _ => {
                card_frame(idx).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    raw_row(ui, e, false, list, actions, id);
                });
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn action_card(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    entry: &BodyEntry,
    kw: &'static Keyword,
    idx: usize,
    lock: Option<(&str, bool)>,
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
            num_bad_id: ui.make_persistent_id("bad-int"),
            classes: ctx.classes,
        };
        let tokens: Vec<&str> = entry.values.iter().map(String::as_str).collect();
        let value = grammar::decode(kw, &tokens);
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
                            .color(if lock.is_some() {
                                theme::DISABLED_TEXT
                            } else {
                                theme::VALUE_TEXT
                            }),
                    )
                    .truncate(),
                )
                .on_hover_text(kw.name);
                match lock {
                    Some((name, true)) => {
                        widgets::badge(
                            ui,
                            "from style",
                            &format!("Set by style \"{name}\". Remove the style to edit it."),
                        );
                    }
                    Some((name, false)) => {
                        widgets::badge(
                            ui,
                            "locked",
                            &format!(
                                "Style \"{name}\" is applied, so edits here would be overwritten. Remove the style to edit it."
                            ),
                        );
                    }
                    None => {
                        if widgets::remove_x_button(ui, "Remove this action").clicked() {
                            actions.push(remove(id, entry));
                        }
                    }
                }
            });
            if lock.is_some() {
                ui.label(
                    RichText::new(current_values(entry))
                        .size(14.0)
                        .color(theme::DISABLED_TEXT),
                );
            } else {
                primary = action_controls(ui, &card, &value, list, actions);
            }
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

fn action_controls(
    ui: &mut egui::Ui,
    card: &Card,
    value: &Value,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) -> Option<egui::Response> {
    match value {
        Value::Rgba { rgba, .. } => {
            if let Some(n) = widgets::colour_swatch(ui, *rgba)
                && let Some(a) = rgba_edit(card.id, card.entry, n)
            {
                actions.push(a);
            }
            None
        }
        Value::FontSize(n) => font_control(ui, card, *n, actions),
        Value::Sound {
            id: SoundRef::Id(i),
            volume,
        } => sound_control(ui, card, *i, *volume, actions),
        Value::CustomSound { file, volume } => {
            custom_sound_control(ui, card, file, *volume, list, actions)
        }
        Value::Minimap(m) => {
            minimap_control(ui, card, m, actions);
            None
        }
        Value::Effect(e) => {
            effect_control(ui, card, e, actions);
            None
        }
        Value::OptionalBool(_) => {
            optional_bool_control(ui, card, actions);
            None
        }
        _ => Some(raw_value_control(ui, card, list, actions)),
    }
}

/// Slider plus exact number field. Returns the new value when the user changed it.
pub(crate) fn slider_pair(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash,
    current: i64,
    range: (i64, i64),
    invalid: &mut bool,
) -> (Option<i64>, egui::Response) {
    let mut v = current;
    let s = widgets::themed_slider(ui, &mut v, range.0, range.1, false);
    let n = widgets::number_field(ui, salt, &mut v, 56.0);
    *invalid |= n.invalid;
    let changed = (s.changed() || n.response.changed()) && v != current;
    (changed.then_some(v), n.response)
}

fn font_control(
    ui: &mut egui::Ui,
    card: &Card,
    current: i64,
    actions: &mut Vec<RowAction>,
) -> Option<egui::Response> {
    let mut invalid = false;
    let mut new = None;
    let mut primary = None;
    ui.horizontal(|ui| {
        let (n, r) = slider_pair(ui, card.salt("num"), current, (1, 45), &mut invalid);
        new = n;
        primary = Some(r);
    });
    if invalid {
        hint(ui, WHOLE_NUMBER_HINT);
    }
    if let Some(msg) = font_hint(new.unwrap_or(current)) {
        ui.label(
            RichText::new(msg)
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::TAB_ACTIVE),
        );
    }
    if let Some(n) = new
        && let Some(a) = font_edit(card.id, card.entry, n)
    {
        actions.push(a);
    }
    primary
}

fn sound_control(
    ui: &mut egui::Ui,
    card: &Card,
    sound: i64,
    volume: Option<i64>,
    actions: &mut Vec<RowAction>,
) -> Option<egui::Response> {
    let mut invalid = false;
    let (mut new_id, mut new_vol) = (None, None);
    let mut primary = None;
    ui.horizontal(|ui| {
        caption(ui, "Sound");
        let (n, r) = slider_pair(ui, card.salt("sid"), sound, (1, 16), &mut invalid);
        new_id = n;
        primary = Some(r);
    });
    ui.horizontal(|ui| {
        caption(ui, "Volume");
        let shown = volume.unwrap_or(100);
        new_vol = slider_pair(ui, card.salt("svol"), shown, (0, 300), &mut invalid).0;
    });
    if invalid {
        hint(ui, WHOLE_NUMBER_HINT);
    }
    if (new_id.is_some() || new_vol.is_some())
        && let Some(a) = sound_edit(
            card.id,
            card.entry,
            SoundRef::Id(new_id.unwrap_or(sound)),
            new_vol.or(volume),
        )
    {
        actions.push(a);
    }
    primary
}

fn custom_sound_control(
    ui: &mut egui::Ui,
    card: &Card,
    file: &str,
    volume: Option<i64>,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) -> Option<egui::Response> {
    let tid = ui.make_persistent_id(card.salt("file"));
    let key = (card.id, card.entry.line);
    let mut resp = None;
    let mut commit = None;
    ui.horizontal(|ui| {
        caption(ui, "File");
        let (r, c) = draft_field(ui, tid, file, 160.0, list, key);
        resp = Some(r);
        commit = c;
    });
    caption(ui, "The file must be in the game folder.");
    if let Some(text) = commit {
        match custom_sound_edit(card.id, card.entry, &text, volume) {
            Ok(Some(a)) => {
                ui.data_mut(|d| d.insert_temp(card.bad_id, false));
                actions.push(a);
            }
            Ok(None) => {}
            Err(_) => {
                ui.data_mut(|d| d.insert_temp(card.bad_id, true));
            }
        }
    }
    let mut invalid = false;
    let mut new_vol = None;
    ui.horizontal(|ui| {
        caption(ui, "Volume");
        new_vol = slider_pair(
            ui,
            card.salt("cvol"),
            volume.unwrap_or(100),
            (0, 300),
            &mut invalid,
        )
        .0;
    });
    if invalid {
        hint(ui, WHOLE_NUMBER_HINT);
    }
    if let Some(v) = new_vol
        && let Ok(Some(a)) = custom_sound_edit(card.id, card.entry, file, Some(v))
    {
        actions.push(a);
    }
    resp
}

pub(crate) fn colour_combo(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    current: &str,
    allow_none: bool,
) -> Option<String> {
    let mut picked = None;
    dot(ui, filter_colour(current));
    egui::ComboBox::from_id_salt(salt)
        .selected_text(current)
        .width(104.0)
        .show_ui(ui, |ui| {
            let mut names: Vec<&str> = COLOURS.to_vec();
            if allow_none {
                names.push("None");
            }
            for c in names {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    dot(ui, filter_colour(c));
                    if ui.selectable_label(c == current, c).clicked() && c != current {
                        picked = Some(c.to_string());
                    }
                });
            }
        });
    picked
}

fn minimap_control(ui: &mut egui::Ui, card: &Card, m: &Minimap, actions: &mut Vec<RowAction>) {
    let (size, colour, shape) = match m {
        Minimap::Icon {
            size,
            colour,
            shape,
        } => (*size, colour.clone(), shape.clone()),
        Minimap::Hidden(_) => (-1, "White".to_string(), "Circle".to_string()),
    };
    let hidden = matches!(m, Minimap::Hidden(_));
    let mut next: Option<Minimap> = None;
    let icon = |size: i64, colour: &str, shape: &str| Minimap::Icon {
        size,
        colour: colour.to_string(),
        shape: shape.to_string(),
    };
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
        let label = SIZE_CHOICES
            .iter()
            .find(|(_, v)| *v == size)
            .map_or_else(|| size.to_string(), |(l, _)| (*l).to_string());
        egui::ComboBox::from_id_salt(card.salt("msize"))
            .selected_text(label)
            .width(104.0)
            .show_ui(ui, |ui| {
                for (l, v) in SIZE_CHOICES {
                    if ui.selectable_label(v == size, l).clicked() && v != size {
                        next = Some(if v == -1 {
                            Minimap::Hidden(vec!["-1".to_string()])
                        } else {
                            icon(v, &colour, &shape)
                        });
                    }
                }
            });
        if hidden {
            return;
        }
        if let Some(c) = colour_combo(ui, card.salt("mcol"), &colour, false) {
            next = Some(icon(size, &c, &shape));
        }
        egui::ComboBox::from_id_salt(card.salt("mshape"))
            .selected_text(shape.as_str())
            .width(104.0)
            .show_ui(ui, |ui| {
                for s in SHAPES {
                    if ui.selectable_label(*s == shape, *s).clicked() && *s != shape {
                        next = Some(icon(size, &colour, s));
                    }
                }
            });
    });
    if let Some(n) = next
        && let Some(a) = minimap_edit(card.id, card.entry, n)
    {
        actions.push(a);
    }
}

fn effect_control(ui: &mut egui::Ui, card: &Card, e: &Effect, actions: &mut Vec<RowAction>) {
    let (colour, temp) = match e {
        Effect::None => ("None".to_string(), false),
        Effect::Colour { colour, temp } => (colour.clone(), *temp),
    };
    let mut next: Option<Effect> = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
        if let Some(c) = colour_combo(ui, card.salt("ecol"), &colour, true) {
            next = Some(if c == "None" {
                Effect::None
            } else {
                Effect::Colour { colour: c, temp }
            });
        }
        let mut t = temp;
        let none = matches!(e, Effect::None);
        let changed = ui
            .add_enabled_ui(!none, |ui| {
                let c = widgets::circular_toggle(ui, &mut t).changed();
                ui.label(
                    RichText::new("Temporary")
                        .size(14.0)
                        .color(theme::VALUE_TEXT),
                );
                c
            })
            .inner;
        if changed && t != temp {
            next = Some(Effect::Colour {
                colour: colour.clone(),
                temp: t,
            });
        }
    });
    if let Some(n) = next
        && let Some(a) = effect_edit(card.id, card.entry, n)
    {
        actions.push(a);
    }
}

fn optional_bool_control(ui: &mut egui::Ui, card: &Card, actions: &mut Vec<RowAction>) {
    let current = optional_bool_on(card.entry);
    let mut on = current;
    let mut changed = false;
    ui.horizontal(|ui| {
        changed = widgets::circular_toggle(ui, &mut on).changed();
        ui.label(RichText::new("On").size(14.0).color(theme::VALUE_TEXT));
    });
    if changed && let Some(a) = optional_bool_edit(card.id, card.entry, on) {
        actions.push(a);
    }
}

// ---------------------------------------------------------------------------------------------
// The STYLE group
// ---------------------------------------------------------------------------------------------

const STYLE_NONE_HELP: &str = "Apply a style to set this rule's look from your Styles tab.";
const STYLE_KNOWN_EXPLAIN: &str = "This rule's look comes from the style. Edits to the style reach it when you press Apply. Hand edits here are overwritten.";
const STYLE_UNKNOWN_EXPLAIN: &str = "No style with this name exists. This rule keeps its current actions, and its style link is removed on the next Apply.";
const STYLE_UNCHECKED_EXPLAIN: &str =
    "styles.json could not be read; this style is not applied until it loads.";

/// A dropdown of every style (with a swatch of its text colour). Returns the picked name.
fn style_combo(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    salt: impl std::hash::Hash + std::fmt::Debug,
    current: Option<&str>,
    placeholder: &str,
) -> Option<String> {
    let mut picked = None;
    egui::ComboBox::from_id_salt(salt)
        .selected_text(current.unwrap_or(placeholder))
        .width(160.0)
        .show_ui(ui, |ui| {
            if current.is_none() {
                let _ = ui.selectable_label(true, "(none)");
            }
            for name in ctx.working.style_names() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if let Some(t) = ctx.working.style(&name).and_then(|s| s.text) {
                        dot(ui, egui::Color32::from_rgb(t[0], t[1], t[2]));
                    }
                    if ui
                        .selectable_label(Some(name.as_str()) == current, &name)
                        .clicked()
                        && Some(name.as_str()) != current
                    {
                        picked = Some(name.clone());
                    }
                });
            }
        });
    picked
}

fn body_text(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .size(theme::ROW_SUMMARY_SIZE)
                .color(color),
        )
        .wrap(),
    );
}

/// The STYLE section: pick a named style, or see which style a rule carries and remove it.
pub fn style_group(ui: &mut egui::Ui, ctx: &BodyCtx, actions: &mut Vec<RowAction>) {
    let id = ctx.row.id;
    caption(ui, "STYLE");
    ui.add_enabled_ui(ctx.editable, |ui| {
        let (title, tag, explain, colour) = match &ctx.row.style {
            StyleState::None => {
                if let Some(n) = style_combo(ui, ctx, (id, "style-pick"), None, "(none)") {
                    actions.push(RowAction::ApplyStyle(id, n));
                }
                body_text(ui, STYLE_NONE_HELP, theme::TAB_ACTIVE);
                return;
            }
            StyleState::Known(n) => (
                format!("Style: {n}"),
                Some(n.clone()),
                STYLE_KNOWN_EXPLAIN,
                theme::SECTION_CREAM,
            ),
            StyleState::Unknown(n) => (
                format!("Unknown style \"{n}\""),
                None,
                STYLE_UNKNOWN_EXPLAIN,
                theme::WARM_ACCENT_TEXT,
            ),
            StyleState::Unchecked(n) => (
                format!("Style: {n}"),
                None,
                STYLE_UNCHECKED_EXPLAIN,
                theme::SECTION_CREAM,
            ),
        };
        egui::Frame::new()
            .fill(theme::HEADER_PLATE)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).size(14.0).color(colour));
                    let placeholder = "Pick a style\u{2026}";
                    if let Some(n) =
                        style_combo(ui, ctx, (id, "style-change"), tag.as_deref(), placeholder)
                    {
                        actions.push(RowAction::ApplyStyle(id, n));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button(RichText::new("Remove style").size(theme::ROW_SUMMARY_SIZE))
                            .clicked()
                        {
                            actions.push(RowAction::RemoveStyle(id));
                        }
                    });
                });
                body_text(ui, explain, theme::TAB_ACTIVE);
            });
    });
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
            "Show\n    ItemLevel >= 70\n    Corrupted True\n    Rarity Rare\n    Identified False\n    FutureKw 3\n",
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

    /// CR-01: a negated rarity line keeps its negation when a chip is added; only ordering
    /// operators are dropped for a multi-rarity line.
    #[test]
    fn rarity_edit_keeps_a_negation() {
        let patch_of = |r: RarityEdit| match r {
            RarityEdit::Edit(RowAction::EditLine { patch, .. }) => Some(patch),
            _ => None,
        };
        for src in [
            "Show\n    Rarity != Normal\n",
            "Show\n    Rarity ! Normal\n",
        ] {
            let (_, id, e) = entries_of(src);
            assert_eq!(
                patch_of(rarity_edit(id, &e[0], &["Normal", "Magic"])),
                Some(LinePatch::Values("Normal Magic".into())),
                "{src}"
            );
        }
        let (_, id, e) = entries_of("Show\n    Rarity == Rare\n");
        assert_eq!(
            patch_of(rarity_edit(id, &e[0], &["Magic", "Rare"])),
            Some(LinePatch::Values("Magic Rare".into()))
        );
        let (_, id, e) = entries_of("Show\n    Rarity < Rare\n");
        assert_eq!(
            patch_of(rarity_edit(id, &e[0], &["Normal", "Rare"])),
            Some(LinePatch::Both(None, "Normal Rare".into()))
        );
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

    fn values_of(a: Option<RowAction>) -> Option<String> {
        match a {
            Some(RowAction::EditLine {
                patch: LinePatch::Values(t),
                ..
            }) => Some(t),
            _ => None,
        }
    }

    fn committed_catalog() -> ClassCatalog {
        let dir = env!("CARGO_MANIFEST_DIR");
        let cfg = crate::loader::load_config(
            &format!("{dir}/config.json"),
            &format!("{dir}/styles.json"),
        )
        .unwrap();
        ClassCatalog::from_config(&cfg)
    }

    #[test]
    fn list_edits_remove_the_line_when_emptied() {
        let (_, id, e) = entries_of("Show\n    Class \"Amulets\"\n");
        let two = ["Amulets".to_string(), "Rings".to_string()];
        assert_eq!(
            values_of(list_edit(id, &e[0], &two).unwrap()),
            Some("\"Amulets\" \"Rings\"".to_string())
        );
        assert!(matches!(
            list_edit(id, &e[0], &[]).unwrap(),
            Some(RowAction::RemoveLine { .. })
        ));
        assert_eq!(
            list_edit(id, &e[0], &["Amulets".to_string()]).unwrap(),
            None
        );
    }

    #[test]
    fn class_toggle_appends_and_removes_in_order() {
        let items = vec!["Amulets".to_string(), "Rings".to_string()];
        assert_eq!(
            class_toggle(&items, "Belts", true),
            ["Amulets", "Rings", "Belts"]
        );
        assert_eq!(class_toggle(&items, "Amulets", false), ["Rings"]);
        assert_eq!(class_toggle(&items, "Rings", true), items);
    }

    #[test]
    fn class_card_with_a_catalogue_is_idle_and_keeps_unknown_classes() {
        let wf = WorkingFilter::from_text(
            "Show\n    Class \"Amulets\" \"Custom Thing\"\n    BaseType == \"Heavy Belt\"\n    HasExplicitMod >=1 \"Tyrannical\"\n    Sockets >= 5GGG\n    ItemLevel 60 65\n    Corrupted True\n",
        );
        let cat = committed_catalog();
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[0].id);
        for _ in 0..3 {
            let a = frame_of(&ctx, input(vec![]), &wf, &mut list, Some(&cat));
            assert!(a.is_empty(), "idle frame emitted {a:?}");
        }
        // And without a catalogue the Class card falls back to the chip list.
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[0].id);
        assert!(frame_of(&headless_ctx(), input(vec![]), &wf, &mut list, None).is_empty());
    }

    #[test]
    fn base_type_chip_commit_appends_both_entries() {
        let (_, id, e) = entries_of("Show\n    BaseType == \"Utility Belt\"\n");
        let items = vec![
            "Utility Belt".to_string(),
            "Heavy Belt".to_string(),
            "Armoured Cap".to_string(),
        ];
        assert_eq!(
            values_of(list_edit(id, &e[0], &items).unwrap()),
            Some("\"Utility Belt\" \"Heavy Belt\" \"Armoured Cap\"".to_string())
        );
    }

    #[test]
    fn count_names_edits_keep_the_operator_and_remove_when_empty() {
        let (_, id, e) = entries_of("Show\n    HasExplicitMod >=1 \"Tyrannical\"\n");
        assert_eq!(e[0].op, Some(Operator::Ge));
        let r = count_names_edit(id, &e[0], Some(2), &["Tyrannical".to_string()]).unwrap();
        assert_eq!(values_of(r), Some("2 \"Tyrannical\"".to_string()));
        let r = count_names_edit(id, &e[0], Some(1), &[]).unwrap();
        assert!(matches!(r, Some(RowAction::RemoveLine { .. })));
        let r = count_names_edit(id, &e[0], Some(1), &["Tyrannical".to_string()]).unwrap();
        assert_eq!(r, None);
    }

    #[test]
    fn sockets_edits_and_slots() {
        let (_, id, e) = entries_of("Show\n    Sockets >= 5GGG\n");
        assert_eq!(e[0].values, ["5GGG"]);
        let colours = set_slot("GGG", 1, Some('R'));
        assert_eq!(colours, "GRG");
        assert_eq!(
            values_of(sockets_edit(id, &e[0], 5, &colours)),
            Some("5GRG".to_string())
        );
        assert_eq!(set_slot("GRG", 1, None), "GG");
        assert_eq!(
            values_of(sockets_edit(id, &e[0], 5, "")),
            Some("5".to_string())
        );
        assert_eq!(sockets_edit(id, &e[0], 5, "GGG"), None);
    }

    #[test]
    fn bool_or_name_any_and_named() {
        // No keyword uses BoolOrName today; the edit helper is keyword-independent, so any
        // entry exercises it.
        let (_, id, e) = entries_of("Show\n    Corrupted False\n");
        assert_eq!(
            values_of(bool_or_name_edit(id, &e[0], true, "").unwrap()),
            Some("True".to_string())
        );
        assert_eq!(
            values_of(bool_or_name_edit(id, &e[0], false, "Arc of Surging").unwrap()),
            Some("\"Arc of Surging\"".to_string())
        );
        assert_eq!(
            bool_or_name_edit(id, &e[0], false, "a\"b"),
            Err(EncodeError::BadText)
        );
    }

    #[test]
    fn multi_int_chip_add_appends() {
        let (_, id, e) = entries_of("Show\n    ItemLevel 60 65\n");
        assert_eq!(
            values_of(ints_edit(id, &e[0], &[60, 65, 70])),
            Some("60 65 70".to_string())
        );
    }

    fn names_of(g: &Groups) -> Vec<&'static str> {
        g.iter()
            .flat_map(|(_, ks)| ks.iter().map(|k| k.name))
            .collect()
    }

    #[test]
    fn picker_entries_filter_case_insensitively_and_drop_empty_groups() {
        let all = picker_entries("");
        assert_eq!(all.len(), grammar::condition_groups().len());
        let level = picker_entries("LEVEL");
        let names = names_of(&level);
        for n in ["ItemLevel", "DropLevel", "AreaLevel", "GemLevel"] {
            assert!(names.contains(&n), "{n} missing from {names:?}");
        }
        assert!(level.iter().all(|(_, ks)| !ks.is_empty()));
        assert!(!names.contains(&"Class"));
        let spaced = picker_entries("item lev");
        assert_eq!(names_of(&spaced), ["ItemLevel"]);
        assert!(picker_entries("zzzz").is_empty());
    }

    #[test]
    fn removed_poe1_keywords_are_not_offered() {
        assert!(picker_entries("linked").is_empty());
        let mirage = picker_entries("mirage");
        assert_eq!(names_of(&mirage), ["MirageMap"]);
        let kw = crate::filter::keywords::lookup("MirageMap").unwrap();
        assert_eq!(picker_label(kw), "Mirage Map");
        let item = crate::filter::keywords::lookup("ItemLevel").unwrap();
        assert_eq!(picker_label(item), "Item Level");
    }

    #[test]
    fn picking_a_default_keyword_adds_a_line_and_repeats_are_allowed() {
        let (_, id, _) = entries_of("Show\n    ItemLevel >= 70\n");
        let kw = crate::filter::keywords::lookup("ItemLevel").unwrap();
        let mut list = ListUi::default();
        for _ in 0..2 {
            assert_eq!(
                pick_condition(id, kw, &mut list),
                Some(RowAction::AddLine {
                    id,
                    kind: LineKind::Condition,
                    text: "ItemLevel >= 65".into(),
                })
            );
        }
        assert!(list.pending.is_empty());
    }

    #[test]
    fn list_keywords_become_pending_rows_with_the_right_operator() {
        let (_, id, _) = entries_of("Show\n");
        let mut list = ListUi::default();
        let base = crate::filter::keywords::lookup("BaseType").unwrap();
        let class = crate::filter::keywords::lookup("Class").unwrap();
        let rarity = crate::filter::keywords::lookup("Rarity").unwrap();
        for kw in [base, class, rarity, base] {
            assert_eq!(pick_condition(id, kw, &mut list), None);
        }
        assert_eq!(list.pending.len(), 3, "the repeat pick is not duplicated");
        assert_eq!(list.pending[0].op, Some(Operator::EqEq));
        assert_eq!(list.pending[1].op, None);
        assert_eq!(list.pending[2].op, None);
        assert!(list.pending.iter().all(|p| p.kind == LineKind::Condition));
    }

    /// WR-01 (iteration 2): unchecking the last rarity chip and then picking another keeps a
    /// negation or equality; an ordering operator survives a single pick and is dropped only
    /// when two or more rarities are picked.
    #[test]
    fn rarity_refill_keeps_the_removed_operator() {
        let text_of = |a: &Option<RowAction>| match a {
            Some(RowAction::AddLine { text, .. }) => Some(text.clone()),
            _ => None,
        };
        let rarity = crate::filter::keywords::lookup("Rarity").unwrap();
        let cases: [(&str, &[&str], &str); 6] = [
            (
                "Show\n    Rarity != Normal\n",
                &["Magic"],
                "Rarity != Magic",
            ),
            (
                "Show\n    Rarity ! Normal\n",
                &["Magic", "Rare"],
                "Rarity ! Magic Rare",
            ),
            ("Show\n    Rarity == Rare\n", &["Magic"], "Rarity == Magic"),
            ("Show\n    Rarity >= Rare\n", &["Magic"], "Rarity >= Magic"),
            (
                "Show\n    Rarity >= Rare\n",
                &["Magic", "Rare"],
                "Rarity Magic Rare",
            ),
            ("Show\n    Rarity Rare\n", &["Magic"], "Rarity Magic"),
        ];
        for (src, pick, want) in cases {
            let (mut wf, id, e) = entries_of(src);
            let RarityEdit::Remove(rm) = rarity_edit(id, &e[0], &[]) else {
                panic!("{src}: unchecking the last chip removes the line");
            };
            let p = rarity_refill(id, rarity, &e[0]);
            assert!(wf.apply_action(rm).changed, "{src}");
            let picked: Vec<String> = pick.iter().map(|s| s.to_string()).collect();
            let a = pending_first_entry(&p, &picked);
            assert_eq!(text_of(&a).as_deref(), Some(want), "{src}");
            assert!(
                wf.apply_action(a.unwrap()).changed,
                "{src}: the line is accepted"
            );
        }
    }

    /// WR-01 (iteration 3): a block never holds two pending rows for one keyword, so no two
    /// pending cards share an egui id. A refill replaces any pending row; a pick keeps it.
    #[test]
    fn one_pending_row_per_keyword_and_block() {
        let rarity = crate::filter::keywords::lookup("Rarity").unwrap();
        let class = crate::filter::keywords::lookup("Class").unwrap();

        // `Rarity != Normal`, uncheck Normal, then pick Rarity again from `+ condition`.
        let (_, id, e) = entries_of("Show\n    Rarity != Normal\n");
        let mut list = ListUi::default();
        queue_refill(&mut list, rarity_refill(id, rarity, &e[0]));
        assert_eq!(pick_condition(id, rarity, &mut list), None);
        assert_eq!(list.pending.len(), 1, "{:?}", list.pending);
        assert_eq!(
            list.pending[0].op,
            Some(Operator::Ne),
            "the negation is kept"
        );

        // A range rule: unchecking the last chip on both lines leaves one row, the latest.
        let (_, id, e) = entries_of("Show\n    Rarity >= Magic\n    Rarity <= Rare\n");
        let mut list = ListUi::default();
        assert_eq!(pick_condition(id, class, &mut list), None);
        queue_refill(&mut list, rarity_refill(id, rarity, &e[0]));
        queue_refill(&mut list, rarity_refill(id, rarity, &e[1]));
        let rows: Vec<_> = list
            .pending
            .iter()
            .filter(|p| p.keyword.name == "Rarity")
            .collect();
        assert_eq!(rows.len(), 1, "{:?}", list.pending);
        assert_eq!(rows[0].op, Some(Operator::Le));
        assert_eq!(list.pending.len(), 2, "the Class row is untouched");

        // Picking Rarity first and then refilling leaves one row, with the refill's operator.
        let mut list = ListUi::default();
        assert_eq!(pick_condition(id, rarity, &mut list), None);
        queue_refill(&mut list, rarity_refill(id, rarity, &e[0]));
        assert_eq!(list.pending.len(), 1);
        assert_eq!(list.pending[0].op, Some(Operator::Ge));
    }

    #[test]
    fn pending_first_entry_builds_the_full_line() {
        let (_, id, _) = entries_of("Show\n");
        let pend = |name: &str, op| PendingRow {
            block: id,
            keyword: crate::filter::keywords::lookup(name).unwrap(),
            op,
            kind: LineKind::Condition,
        };
        let text_of = |a: Option<RowAction>| match a {
            Some(RowAction::AddLine { text, .. }) => Some(text),
            _ => None,
        };
        let items = vec!["Heavy Belt".to_string()];
        assert_eq!(
            text_of(pending_first_entry(
                &pend("BaseType", Some(Operator::EqEq)),
                &items
            )),
            Some("BaseType == \"Heavy Belt\"".to_string())
        );
        assert_eq!(
            text_of(pending_first_entry(
                &pend("Class", None),
                &["Rings".to_string()]
            )),
            Some("Class \"Rings\"".to_string())
        );
        assert_eq!(
            text_of(pending_first_entry(
                &pend("Rarity", None),
                &["Magic".to_string(), "Rare".to_string()]
            )),
            Some("Rarity Magic Rare".to_string())
        );
        assert_eq!(
            text_of(pending_first_entry(
                &pend("HasExplicitMod", None),
                &["Tyrannical".to_string()]
            )),
            Some("HasExplicitMod \"Tyrannical\"".to_string())
        );
        let mut wf = WorkingFilter::from_text("Show\n");
        let a = pending_first_entry(
            &PendingRow {
                block: wf.rows()[0].id,
                ..pend("HasExplicitMod", None)
            },
            &["Tyrannical".to_string()],
        )
        .unwrap();
        assert!(wf.apply_action(a).changed, "the count-less line is valid");
        assert_eq!(pending_first_entry(&pend("Class", None), &[]), None);
        assert_eq!(
            pending_first_entry(&pend("BaseType", None), &["a\"b".to_string()]),
            None
        );
    }

    #[test]
    fn pending_rows_render_idle_and_are_dropped_when_the_row_collapses() {
        let wf = WorkingFilter::from_text("Show\n    ItemLevel >= 70\n\nShow\n");
        let id = wf.rows()[0].id;
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(id);
        let class = crate::filter::keywords::lookup("Class").unwrap();
        assert_eq!(pick_condition(id, class, &mut list), None);
        let cat = committed_catalog();
        for _ in 0..2 {
            let a = frame_of(&ctx, input(vec![]), &wf, &mut list, Some(&cat));
            assert!(a.is_empty(), "{a:?}");
        }
        assert_eq!(list.pending.len(), 1, "pending survives while expanded");
        list.expanded = Some(wf.rows()[1].id);
        frame_of(&ctx, input(vec![]), &wf, &mut list, Some(&cat));
        assert!(list.pending.is_empty(), "switching rows drops pending");
        list.pending.push(PendingRow {
            block: wf.rows()[1].id,
            keyword: class,
            op: None,
            kind: LineKind::Condition,
        });
        list.expanded = None;
        frame_of(&ctx, input(vec![]), &wf, &mut list, Some(&cat));
        assert!(list.pending.is_empty(), "collapsing drops pending");
    }

    #[test]
    fn plus_condition_opens_the_picker() {
        let wf = WorkingFilter::from_text("Show\n    ItemLevel >= 70\n");
        let id = wf.rows()[0].id;
        let mut opened = false;
        'scan: for y in (20..120).step_by(8) {
            for x in (300..456).step_by(8) {
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
                assert!(a.is_empty(), "opening the picker writes nothing: {a:?}");
                frame_of(&ctx, input(vec![]), &wf, &mut list, None);
                if egui::Popup::is_any_open(&ctx) {
                    opened = true;
                    // Esc closes it again.
                    let esc = Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::default(),
                    };
                    frame_of(&ctx, input(vec![esc]), &wf, &mut list, None);
                    frame_of(&ctx, input(vec![]), &wf, &mut list, None);
                    assert!(!egui::Popup::is_any_open(&ctx));
                    break 'scan;
                }
            }
        }
        assert!(opened, "no click position opened the + condition picker");
    }

    #[test]
    fn refused_edit_error_is_found_for_its_line_only() {
        let (_, id, _) = entries_of("Show\n    FutureKw 3\n");
        let mut list = ListUi::default();
        list.line_error = Some((id, 1, "nope".into()));
        assert_eq!(error_for(&list, id, 1), Some("nope"));
        assert_eq!(error_for(&list, id, 2), None);
    }

    // ----- 20-08: actions -----

    const ALL_ACTIONS: &str = "Show\n    SetTextColor 255 121 198 128\n    SetFontSize 37\n    MinimapIcon 0 Pink Circle\n    PlayEffect Pink Temp\n    PlayAlertSound 6 300\n    CustomAlertSound \"uwu.mp3\" 300\n    DisableDropSound\n";

    #[test]
    fn rgba_edits_keep_form() {
        let (_, id, e) = entries_of("Show\n    SetTextColor 255 121 198\n");
        assert_eq!(rgba_edit(id, &e[0], [255, 121, 198, 255]), None);
        assert_eq!(
            values_of(rgba_edit(id, &e[0], [255, 0, 198, 255])).as_deref(),
            Some("255 0 198")
        );
        assert_eq!(
            values_of(rgba_edit(id, &e[0], [255, 121, 198, 128])).as_deref(),
            Some("255 121 198 128")
        );
        let (_, id, e) = entries_of("Show\n    SetTextColor 255 121 198 255\n");
        assert_eq!(
            values_of(rgba_edit(id, &e[0], [255, 0, 198, 255])).as_deref(),
            Some("255 0 198 255")
        );
    }

    #[test]
    fn minimap_effect_sound_edits() {
        let (_, id, e) = entries_of("Show\n    MinimapIcon 0 Pink Circle\n");
        let small = Minimap::Icon {
            size: 2,
            colour: "Pink".into(),
            shape: "Circle".into(),
        };
        assert_eq!(
            values_of(minimap_edit(id, &e[0], small)).as_deref(),
            Some("2 Pink Circle")
        );
        assert_eq!(
            values_of(minimap_edit(id, &e[0], Minimap::Hidden(vec!["-1".into()]))).as_deref(),
            Some("-1")
        );
        let (_, id, e) = entries_of("Show\n    PlayEffect Red\n");
        assert_eq!(
            values_of(effect_edit(
                id,
                &e[0],
                Effect::Colour {
                    colour: "Red".into(),
                    temp: true
                }
            ))
            .as_deref(),
            Some("Red Temp")
        );
        let (_, id, e) = entries_of("Show\n    PlayAlertSound 6 300\n");
        assert_eq!(
            values_of(sound_edit(id, &e[0], SoundRef::Id(6), Some(200))).as_deref(),
            Some("6 200")
        );
        let (_, id, e) = entries_of("Show\n    PlayAlertSound 6\n");
        assert_eq!(sound_edit(id, &e[0], SoundRef::Id(6), None), None);
    }

    #[test]
    fn custom_sound_edits() {
        let (_, id, e) = entries_of("Show\n    CustomAlertSound \"uwu.mp3\" 300\n");
        assert_eq!(
            custom_sound_edit(id, &e[0], "bad\"name.mp3", Some(300)),
            Err("Values cannot contain quotes or control characters".to_string())
        );
        assert_eq!(
            values_of(custom_sound_edit(id, &e[0], "alert.mp3", Some(300)).unwrap()).as_deref(),
            Some("\"alert.mp3\" 300")
        );
    }

    #[test]
    fn optional_bool_edits() {
        let (_, id, e) = entries_of("Show\n    DisableDropSound\n");
        assert_eq!(optional_bool_edit(id, &e[0], true), None);
        assert_eq!(
            values_of(optional_bool_edit(id, &e[0], false)).as_deref(),
            Some("False")
        );
        let (_, id, e) = entries_of("Show\n    DisableDropSound False\n");
        assert_eq!(optional_bool_edit(id, &e[0], false), None);
        assert_eq!(
            values_of(optional_bool_edit(id, &e[0], true)).as_deref(),
            Some("True")
        );
    }

    #[test]
    fn added_actions_and_picking() {
        let (_, id, e) =
            entries_of("Show\n    SetFontSize 30\n    PlayEffect Red\n    Class \"Rings\"\n");
        assert_eq!(added_actions(&e), vec!["SetFontSize", "PlayEffect"]);
        let kw = crate::filter::keywords::lookup("SetBackgroundColor").unwrap();
        assert_eq!(
            pick_action(id, kw),
            Some(RowAction::AddLine {
                id,
                kind: LineKind::Action,
                text: "SetBackgroundColor 40 42 54 255".into()
            })
        );
    }

    #[test]
    fn font_hint_range() {
        assert_eq!(font_hint(12), Some("Readable in game: 18-45"));
        assert_eq!(font_hint(30), None);
    }

    #[test]
    fn expanded_actions_are_idle() {
        let wf = WorkingFilter::from_text(ALL_ACTIONS);
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[0].id);
        for _ in 0..3 {
            let a = frame_of(&ctx, input(vec![]), &wf, &mut list, None);
            assert!(a.is_empty(), "idle frame emitted {a:?}");
        }
    }

    #[test]
    fn empty_actions_state_exists() {
        assert!(include_str!("body.rs").contains(NO_ACTIONS));
        let wf = WorkingFilter::from_text("Show\n    ItemLevel >= 70\n");
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[0].id);
        let a = frame_of(&ctx, input(vec![]), &wf, &mut list, None);
        assert!(a.is_empty());
    }

    // ----- 20-08: styles -----

    fn styled(src: &str) -> WorkingFilter {
        let mut wf = WorkingFilter::from_text(src);
        wf.set_styles(Ok(crate::loader::load_styles(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/styles.json"
        ))
        .unwrap()));
        wf
    }

    #[test]
    fn style_owned_follows_style_output() {
        let set = crate::loader::load_styles(concat!(env!("CARGO_MANIFEST_DIR"), "/styles.json"))
            .unwrap();
        assert!(style_owned(set.get("S").unwrap(), "SetFontSize"));
        assert!(style_owned(set.get("S").unwrap(), "PlayEffect"));
        for (_, kw) in grammar::action_groups()
            .iter()
            .flat_map(|(g, ks)| ks.iter().map(move |k| (*g, k)))
        {
            assert!(!style_owned(set.get("unchanged").unwrap(), kw.name));
        }
    }

    #[test]
    fn styled_rows_render_idle_in_every_state() {
        for src in [
            "Show\n    Class \"Rings\"\n    SetFontSize 20\n",
            "#ff:style S\nShow\n    Class \"Rings\"\n    SetFontSize 37\n    SetTextColor 255 121 198 255\n",
            "#ff:style Nope\nShow\n    Class \"Rings\"\n    SetFontSize 20\n",
        ] {
            let wf = styled(src);
            let ctx = headless_ctx();
            let mut list = ListUi::default();
            list.expanded = Some(wf.rows()[0].id);
            for _ in 0..2 {
                let a = frame_of(&ctx, input(vec![]), &wf, &mut list, None);
                assert!(a.is_empty(), "{src}: idle frame emitted {a:?}");
            }
        }
        let mut wf = WorkingFilter::from_text("#ff:style S\nShow\n    Class \"Rings\"\n");
        wf.set_styles(Err("unreadable".into()));
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[0].id);
        assert!(frame_of(&ctx, input(vec![]), &wf, &mut list, None).is_empty());
    }

    #[test]
    fn known_style_row_offers_remove_style() {
        let wf = styled("#ff:style S\nShow\n    Class \"Rings\"\n    SetFontSize 37\n");
        assert!(matches!(
            wf.rows()[0].style,
            super::super::rules::StyleState::Known(_)
        ));
        let id = wf.rows()[0].id;
        let mut found = false;
        'scan: for y in (20..400).step_by(8) {
            for x in (300..456).step_by(8) {
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
                if a.contains(&RowAction::RemoveStyle(id)) {
                    found = true;
                    break 'scan;
                }
            }
        }
        assert!(found, "no click position emitted RemoveStyle");
    }

    /// WR-01: `Continue` is not style-owned, so a known-styled rule still lets it be toggled.
    #[test]
    fn known_style_row_keeps_continue_editable() {
        let wf = styled("#ff:style S\nShow\n    Class \"Rings\"\n    SetFontSize 37\n");
        assert!(matches!(
            wf.rows()[0].style,
            super::super::rules::StyleState::Known(_)
        ));
        let id = wf.rows()[0].id;
        let mut found = false;
        'scan: for y in (20..600).step_by(6) {
            for x in (8..200).step_by(8) {
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
                if a.contains(&RowAction::SetContinue(id, true)) {
                    found = true;
                    break 'scan;
                }
            }
        }
        assert!(
            found,
            "no click position emitted SetContinue on a styled rule"
        );
    }
}
