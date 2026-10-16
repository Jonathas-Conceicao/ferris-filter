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
use crate::filter::{BlockId, BlockKeyword, LinePatch, group};

use super::classes::ClassCatalog;
use super::conditions::{self, CardPlan, DefencesKind, Part};
use super::grammar::{
    self, BoolOrName, ConditionEntry, Effect, EncodeError, Minimap, SoundRef, Value,
};
use super::rule_list::{FocusTarget, ListUi, PendingRow};
use super::rules::{BodyEntry, EntryKind, LineKind, RowAction, RuleRow, StyleState, WorkingFilter};
use super::theme;
use super::widgets;
#[cfg(test)]
use crate::models::config::Style;
use crate::skin::fonts::regular;
use crate::skin::palette as pal;

const LABEL_W: f32 = 120.0;
const OP_W: f32 = 104.0;

const NO_CONDITIONS: &str = "No conditions: this rule matches every item.";
const BAD_TEXT_HINT: &str = "Values cannot contain quotes or control characters";
const WHOLE_NUMBER_HINT: &str = "Enter a whole number";
const MIN_GT_MAX_HINT: &str = "min > max";

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
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::remove_x_button(ui, "Remove this line").clicked() {
                actions.push(remove(id, entry));
            }
        });
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

/// What clicking a picker entry does. An entry already on the rule (a line, a pending row or a
/// defences card) is never added twice: the existing card is focused instead and nothing is
/// written (UAT3-06). Defences and Simple Defences are one slot: when either card is on the rule,
/// both entries focus it (UAT4-03). An absent keyword behaves as `pick_condition`; an absent
/// Defences or Simple Defences becomes a pending card and writes nothing until a toggle or
/// segment is set (UAT3-05). Picking Defences marks the rule advanced so its card never turns
/// into Simple Defences while editing; picking Simple Defences drops that mark.
pub(crate) fn pick_entry(
    id: BlockId,
    entry: ConditionEntry,
    lines: &[&BodyEntry],
    present: &[ConditionEntry],
    list: &mut ListUi,
) -> Option<RowAction> {
    if present.contains(&entry) {
        match entry {
            ConditionEntry::Defences | ConditionEntry::SimpleDefences => {
                list.focus_card = Some((id, FocusTarget::Defences))
            }
            ConditionEntry::Keyword(kw) => {
                let line = lines
                    .iter()
                    .find(|e| {
                        e.kind == EntryKind::Condition
                            && e.keyword.is_some_and(|k| k.name == kw.name)
                    })
                    .map(|e| e.line);
                match line {
                    Some(l) => list.focus_line = Some((id, l)),
                    None => list.focus_card = Some((id, FocusTarget::Pending(kw.name))),
                }
            }
        }
        return None;
    }
    match entry {
        ConditionEntry::Keyword(kw) => pick_condition(id, kw, list),
        ConditionEntry::Defences => {
            list.pending_defences.insert(id);
            list.advanced_defences.insert(id);
            None
        }
        ConditionEntry::SimpleDefences => {
            list.pending_simple_defences.insert(id);
            list.advanced_defences.remove(&id);
            None
        }
    }
}

/// True when either pending defences card (Defences or Simple Defences) is open on the rule.
fn any_pending_defences(list: &ListUi, id: BlockId) -> bool {
    list.pending_defences.contains(&id) || list.pending_simple_defences.contains(&id)
}

/// How long a card focused from the picker keeps its accent outline.
const FLASH_SECS: f64 = 1.0;

/// Accent outline around a card for about a second after `start`. The start time lives in egui
/// temp data under `key`; no state is kept once the flash is over.
fn focus_flash(ui: &egui::Ui, rect: egui::Rect, key: egui::Id, start: bool) {
    let now = ui.input(|i| i.time);
    if start {
        ui.data_mut(|d| d.insert_temp(key, now));
    }
    if let Some(t0) = ui.data(|d| d.get_temp::<f64>(key)) {
        if now - t0 < FLASH_SECS {
            ui.painter().rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(2.0, theme::WARM_ACCENT_TEXT),
                egui::StrokeKind::Inside,
            );
            ui.ctx().request_repaint();
        } else {
            ui.data_mut(|d| d.remove::<f64>(key));
        }
    }
}

/// The CONDITIONS section of an expanded row.
pub fn conditions_group(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let id = ctx.row.id;
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
    let pending_kws: Vec<&'static Keyword> = pending.iter().map(|p| p.keyword).collect();
    let present = conditions::present_entries(&lines, &pending_kws, any_pending_defences(list, id));
    let mut picked = None;
    ui.horizontal(|ui| {
        caption(ui, "CONDITIONS");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let btn = ui.add_enabled(
                ctx.editable,
                egui::Button::new(RichText::new("+ condition").size(theme::ROW_SUMMARY_SIZE)),
            );
            picked = picker_popup(
                ui,
                &btn,
                &grammar::condition_picker_groups(),
                true,
                &|_| None,
                &|e| present.contains(e),
            );
        });
    });
    if let Some(entry) = picked
        && let Some(a) = pick_entry(id, entry, &lines, &present, list)
    {
        actions.push(a);
    }
    ui.spacing_mut().item_spacing.y = 4.0;

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
    let defence_lines: Vec<&BodyEntry> = lines
        .iter()
        .copied()
        .filter(|e| {
            e.kind == EntryKind::Condition
                && e.keyword.is_some_and(|k| conditions::is_defence(k.name))
        })
        .collect();
    // A grouped rule (several blocks, `# ff:group`) only ever shows the Simple Defences card.
    let grouped = ctx.row.group_size > 1;
    let kind = if grouped {
        None
    } else {
        conditions::defences_kind(&defence_lines, list.advanced_defences.contains(&id))
    };
    let simple = grouped || matches!(kind, Some(DefencesKind::Simple(_)));
    let pending_defences =
        !grouped && list.pending_defences.contains(&id) && defence_lines.is_empty();
    let pending_simple = !pending_defences
        && (grouped || list.pending_simple_defences.contains(&id))
        && defence_lines.is_empty();
    if lines.is_empty() && pending.is_empty() && !pending_defences && !pending_simple {
        ui.label(
            RichText::new("No conditions")
                .font(regular(15.0))
                .color(pal::MUTED),
        )
        .on_hover_text(NO_CONDITIONS);
    }
    let single = |ui: &mut egui::Ui,
                  i: usize,
                  idx: usize,
                  list: &mut ListUi,
                  actions: &mut Vec<RowAction>| {
        let e = lines[i];
        match (e.kind, e.keyword) {
            (EntryKind::Condition, Some(kw)) => {
                let can_between = conditions::sole_line(&lines, i);
                condition_card(ui, ctx, e, kw, idx, can_between, list, actions)
            }
            _ => {
                card_frame(idx).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    raw_row(ui, e, true, list, actions, id);
                });
            }
        }
    };
    let mut idx = 0;
    for card in conditions::card_plan(&lines) {
        match card {
            CardPlan::Part(Part::Single(i)) => {
                single(ui, i, idx, list, actions);
                idx += 1;
            }
            CardPlan::Part(Part::Between { lower, upper }) => {
                if let Some(kw) = lines[lower].keyword {
                    between_card(ui, ctx, lines[lower], lines[upper], kw, idx, list, actions);
                    idx += 1;
                }
            }
            CardPlan::Defences(_) => {
                if simple {
                    simple_defences_card(ui, ctx, true, idx, list, actions)
                } else {
                    // Once shown, the rule keeps the Defences card until a reload (UAT4-03).
                    list.advanced_defences.insert(id);
                    defences_card(ui, ctx, &defence_lines, idx, list, actions);
                }
                idx += 1;
            }
        }
    }
    if pending_defences {
        list.advanced_defences.insert(id);
        defences_card(ui, ctx, &[], idx, list, actions);
        idx += 1;
    } else if pending_simple {
        simple_defences_card(ui, ctx, grouped, idx, list, actions);
        idx += 1;
    }
    for (i, p) in pending.iter().enumerate() {
        pending_card(ui, ctx, p, idx + i, list, actions);
    }
}

/// The one Defences card of a rule: an Armour / Evasion / Energy Shield toggle each, and for every
/// line of an on keyword its own operator dropdown and value control. A toggle only ever adds or
/// removes its own keyword's lines; the other lines keep their bytes (PARSE-04). With no lines
/// (`defence_lines` empty) this is the pending card, which writes nothing until a toggle is
/// switched on.
fn defences_card(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    defence_lines: &[&BodyEntry],
    idx: usize,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let id = ctx.row.id;
    #[cfg(test)]
    probe_card(ui, id, "defences");
    let subs: Vec<(&'static str, Vec<Part>)> = conditions::card_plan(defence_lines)
        .into_iter()
        .find_map(|c| match c {
            CardPlan::Defences(s) => Some(s),
            _ => None,
        })
        .unwrap_or_default();
    let mut first_toggle: Option<egui::Response> = None;
    let framed = ui.push_id((id, "defences-card"), |ui| {
        card_frame(idx).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
            let x = condition_header(
                ui,
                "Defences",
                "Armour, Evasion and Energy Shield",
                |_| {},
                if defence_lines.is_empty() {
                    "Discard this condition"
                } else {
                    "Remove all defence conditions"
                },
            );
            #[cfg(test)]
            probe_rect(ui, ("defences-x", id), x.rect);
            if x.clicked() {
                list.pending_defences.remove(&id);
                list.advanced_defences.remove(&id);
                // Highest line first, so each removal leaves the indices of the rest valid.
                let mut doomed: Vec<&BodyEntry> = defence_lines.to_vec();
                doomed.sort_by_key(|e| std::cmp::Reverse(e.line));
                for e in doomed {
                    actions.push(remove(id, e));
                }
            }
            for (name, label) in conditions::DEFENCES {
                let Some(kw) = crate::filter::keywords::lookup(name) else {
                    continue;
                };
                let own: Vec<&BodyEntry> = defence_lines
                    .iter()
                    .copied()
                    .filter(|e| e.keyword.is_some_and(|k| k.name == name))
                    .collect();
                let mut on = !own.is_empty();
                ui.horizontal(|ui| {
                    let t = widgets::circular_toggle(ui, &mut on);
                    #[cfg(test)]
                    probe_rect(ui, ("defences-toggle", id, name), t.rect);
                    ui.label(RichText::new(label).size(14.0).color(theme::VALUE_TEXT));
                    if t.changed() {
                        if on {
                            if let Some(text) = grammar::default_line(kw) {
                                actions.push(RowAction::AddLine {
                                    id,
                                    kind: LineKind::Condition,
                                    text,
                                });
                            }
                            list.pending_defences.remove(&id);
                        } else {
                            let mut doomed = own.clone();
                            doomed.sort_by_key(|e| std::cmp::Reverse(e.line));
                            for e in doomed {
                                actions.push(remove(id, e));
                            }
                        }
                    }
                    if first_toggle.is_none() {
                        first_toggle = Some(t);
                    }
                });
                let parts = subs
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, p)| p.as_slice())
                    .unwrap_or(&[]);
                for part in parts {
                    match *part {
                        Part::Single(i) => {
                            let can_between = conditions::sole_line(defence_lines, i);
                            defence_line_controls(
                                ui,
                                ctx,
                                defence_lines[i],
                                kw,
                                can_between,
                                list,
                                actions,
                            )
                        }
                        Part::Between { lower, upper } => defence_between(
                            ui,
                            ctx,
                            defence_lines[lower],
                            defence_lines[upper],
                            kw,
                            list,
                            actions,
                        ),
                    }
                }
            }
        })
    });
    let rect = framed.inner.response.rect;
    let focus = list.focus_card == Some((id, FocusTarget::Defences));
    if focus {
        framed.inner.response.scroll_to_me(Some(Align::Center));
        if let Some(t) = first_toggle {
            t.request_focus();
        }
        list.focus_card = None;
    }
    focus_flash(ui, rect, framed.response.id.with("flash"), focus);
}

/// Test probe: which defences card kinds a rule rendered, and where a control was drawn.
#[cfg(test)]
fn probe_card(ui: &egui::Ui, id: BlockId, kind: &'static str) {
    ui.data_mut(|d| {
        d.get_temp_mut_or_default::<Vec<&'static str>>(egui::Id::new(("defences-cards", id)))
            .push(kind)
    });
}

#[cfg(test)]
fn probe_rect(ui: &egui::Ui, key: impl std::hash::Hash + std::fmt::Debug, rect: egui::Rect) {
    ui.data_mut(|d| d.insert_temp(egui::Id::new(key), rect));
}

/// The Simple Defences selection the card shows: the rule's own (lines, or a group's union), else
/// the pick that wrote no lines (none, or every combination).
fn shown_selection(row: &RuleRow, list: &ListUi) -> u8 {
    row.simple_mask
        .unwrap_or(if list.simple_all.contains(&row.id) {
            group::ALL_COMBOS
        } else {
            0
        })
}

/// What clicking chip `chip` of a card showing `mask` does: the one write (the new selection),
/// and the card bookkeeping. A selection that writes no defence line (none, or every combination)
/// keeps the card open as a pending card and remembers which of the two it was.
fn chip_click(id: BlockId, mask: u8, chip: usize, list: &mut ListUi) -> RowAction {
    let new = mask ^ (1 << chip);
    let no_lines = new == 0 || new == group::ALL_COMBOS;
    if no_lines {
        list.pending_simple_defences.insert(id);
    } else {
        list.pending_simple_defences.remove(&id);
    }
    if new == group::ALL_COMBOS {
        list.simple_all.insert(id);
    } else {
        list.simple_all.remove(&id);
    }
    RowAction::SetDefenceSelection { id, mask: new }
}

/// The Simple Defences card of a rule: one wrapped line of seven chips, one per defence base
/// type (Full AR, Full EV, Full ES, AR/EV, AR/ES, EV/ES, AR/EV/ES). The selected chips are exactly
/// what the rule matches; a click writes the new selection, which may need several blocks (a
/// `# ff:group`). With nothing selected the card writes no line (`has_lines` false: the pending
/// card), and the X removes every defence line.
fn simple_defences_card(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    has_lines: bool,
    idx: usize,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let id = ctx.row.id;
    #[cfg(test)]
    probe_card(ui, id, "simple");
    let mask = shown_selection(ctx.row, list);
    let mut first_chip: Option<egui::Response> = None;
    let framed = ui.push_id((id, "simple-defences-card"), |ui| {
        card_frame(idx).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
            let x = condition_header(
                ui,
                "Simple Defences",
                "Defence base types",
                |_| {},
                if has_lines {
                    "Remove all defence conditions"
                } else {
                    "Discard this condition"
                },
            );
            #[cfg(test)]
            probe_rect(ui, ("simple-x", id), x.rect);
            if x.clicked() {
                list.pending_simple_defences.remove(&id);
                list.advanced_defences.remove(&id);
                list.simple_all.remove(&id);
                if has_lines {
                    actions.push(RowAction::SetDefenceSelection { id, mask: 0 });
                }
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for (chip, (label, tip, _)) in group::COMBOS.iter().enumerate() {
                    let mut sel = mask & (1 << chip) != 0;
                    let r = widgets::toggle_chip(ui, label, &mut sel).on_hover_text(*tip);
                    #[cfg(test)]
                    probe_rect(ui, ("simple-chip", id, chip), r.rect);
                    if r.changed() {
                        actions.push(chip_click(id, mask, chip, list));
                    }
                    if first_chip.is_none() {
                        first_chip = Some(r);
                    }
                }
            });
        })
    });
    let rect = framed.inner.response.rect;
    let focus = list.focus_card == Some((id, FocusTarget::Defences));
    if focus {
        framed.inner.response.scroll_to_me(Some(Align::Center));
        if let Some(c) = first_chip {
            c.request_focus();
        }
        list.focus_card = None;
    }
    focus_flash(ui, rect, framed.response.id.with("flash"), focus);
}

/// Operator dropdown and value control of one defence line, indented under its toggle.
fn defence_line_controls(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    entry: &BodyEntry,
    kw: &'static Keyword,
    can_between: bool,
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
            can_between,
        };
        let tokens: Vec<&str> = entry.values.iter().map(String::as_str).collect();
        let value = grammar::decode(kw, &tokens);
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            op_controls(ui, &card, false, actions);
        });
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.vertical(|ui| {
                value_controls(ui, &card, &value, list, actions);
            });
        });
        if let Some(msg) = error_for(list, id, entry.line) {
            hint(ui, msg);
        }
    });
}

// ---------------------------------------------------------------------------------------------
// Between (UAT3-06)
// ---------------------------------------------------------------------------------------------

fn single_int_value(entry: &BodyEntry) -> Option<i64> {
    match entry.values.as_slice() {
        [v] => v.parse().ok(),
        _ => None,
    }
}

/// Actions for switching a single numeric line to Between: the other bound is inserted directly
/// after the line (checked against the line's current text, so it goes first), then the existing
/// line's operator is edited only when the switch needs it (`==` becomes `>=`).
pub(crate) fn to_between_actions(id: BlockId, entry: &BodyEntry, kw: &Keyword) -> Vec<RowAction> {
    let Some(value) = single_int_value(entry) else {
        return Vec::new();
    };
    let sw = conditions::between_switch(kw, entry.op, value);
    let mut out = vec![RowAction::InsertLineAfter {
        id,
        after: entry.line,
        expect: entry.text.clone(),
        text: sw.insert_after,
    }];
    if let Some(op) = sw.new_op_for_existing {
        out.extend(op_edit(id, entry, Some(op)));
    }
    out
}

/// Actions for switching a Between pair to the single operator `pick`: the bound matching the
/// picked direction is kept (edited only when its operator differs), the other line is removed.
pub(crate) fn from_between_actions(
    id: BlockId,
    lower: &BodyEntry,
    upper: &BodyEntry,
    pick: Option<Operator>,
) -> Vec<RowAction> {
    let keep_upper = matches!(pick, Some(Operator::Le | Operator::Lt));
    let (keep, drop) = if keep_upper {
        (upper, lower)
    } else {
        (lower, upper)
    };
    let mut out: Vec<RowAction> = op_edit(id, keep, pick).into_iter().collect();
    out.push(remove(id, drop));
    out
}

/// Operator dropdown of a single numeric line: the usual operators plus Between.
fn numeric_op_combo(ui: &mut egui::Ui, card: &Card) -> Option<conditions::NumericOp> {
    use conditions::NumericOp;
    let entry = card.entry;
    let cur = NumericOp::Single(entry.op);
    let mut picked = None;
    egui::ComboBox::from_id_salt(card.salt("op"))
        .selected_text(grammar::operator_label(card.kw, entry.op, false))
        .width(OP_W)
        .show_ui(ui, |ui| {
            for (c, label) in conditions::numeric_op_choices_for(card.kw, cur, card.can_between) {
                let sel = c == cur;
                if ui.selectable_label(sel, label).clicked() && !sel {
                    picked = Some(c);
                }
            }
        });
    picked
}

/// The operator control of one condition line. Whole-number keywords offer Between; the rest keep
/// the plain dropdown.
fn op_controls(ui: &mut egui::Ui, card: &Card, list_form: bool, actions: &mut Vec<RowAction>) {
    if card.kw.value == ValueShape::Int && single_int_value(card.entry).is_some() {
        match numeric_op_combo(ui, card) {
            Some(conditions::NumericOp::Single(op)) => {
                actions.extend(op_edit(card.id, card.entry, op));
            }
            Some(conditions::NumericOp::Between) if card.can_between => {
                actions.extend(to_between_actions(card.id, card.entry, card.kw));
            }
            Some(conditions::NumericOp::Between) => {}
            None => {}
        }
    } else if let Some(op) = operator_combo(ui, card, list_form)
        && let Some(a) = op_edit(card.id, card.entry, op)
    {
        actions.push(a);
    }
}

/// The dropdown of a Between pair (reads "Between"); a single operator pick leaves Between.
fn between_op_combo(
    ui: &mut egui::Ui,
    id: BlockId,
    kw: &'static Keyword,
    lower: &BodyEntry,
    upper: &BodyEntry,
    actions: &mut Vec<RowAction>,
) {
    use conditions::NumericOp;
    let mut picked = None;
    egui::ComboBox::from_id_salt((id, kw.name, lower.ordinal, "op"))
        .selected_text("Between")
        .width(OP_W)
        .show_ui(ui, |ui| {
            for (c, label) in conditions::numeric_op_choices(kw, NumericOp::Between) {
                let sel = c == NumericOp::Between;
                if ui.selectable_label(sel, label).clicked() && !sel {
                    picked = Some(c);
                }
            }
        });
    if let Some(NumericOp::Single(op)) = picked {
        actions.extend(from_between_actions(id, lower, upper, op));
    }
}

/// One bound's slider and number field. The new value is clamped to `clamp` (the other bound), so
/// an edit can never produce min > max; an untouched loaded pair is left alone.
fn bound_control(
    ui: &mut egui::Ui,
    id: BlockId,
    kw: &'static Keyword,
    entry: &BodyEntry,
    current: i64,
    clamp: (i64, i64),
    actions: &mut Vec<RowAction>,
) {
    let mut v = current;
    let mut invalid = false;
    let mut changed = false;
    ui.horizontal(|ui| {
        let (min, max, log) = grammar::slider_range(kw.name);
        let s = widgets::themed_slider(ui, &mut v, min, max, log);
        let n = widgets::number_field(
            ui,
            (id, kw.name, entry.ordinal, "num"),
            &mut v,
            widgets::NUMBER_FIELD_W,
        );
        invalid = n.invalid;
        changed = s.changed() || n.response.changed();
    });
    if invalid {
        hint(ui, WHOLE_NUMBER_HINT);
    }
    if changed && let Some(v) = clamped_bound(current, v, clamp) {
        actions.extend(int_edit(id, entry, v));
    }
}

/// The value to write for a bound edited from `current` to `new`, kept inside `clamp` (the other
/// bound); `None` when nothing would change.
pub(crate) fn clamped_bound(current: i64, new: i64, clamp: (i64, i64)) -> Option<i64> {
    let v = new.clamp(clamp.0, clamp.1.max(clamp.0));
    (v != current).then_some(v)
}

/// The min and max controls of a Between pair, each editing only its own line.
fn between_sliders(
    ui: &mut egui::Ui,
    id: BlockId,
    kw: &'static Keyword,
    lower: &BodyEntry,
    upper: &BodyEntry,
    actions: &mut Vec<RowAction>,
) {
    let (Some(lo), Some(hi)) = (single_int_value(lower), single_int_value(upper)) else {
        return;
    };
    // Strict `>` / `<` bounds exclude their own value, so the clamps and the hint account for them.
    let (lo_clamp, hi_clamp) = conditions::bound_clamps(lower.op, lo, upper.op, hi);
    ui.horizontal(|ui| {
        caption(ui, "min");
        bound_control(ui, id, kw, lower, lo, lo_clamp, actions);
    });
    ui.horizontal(|ui| {
        caption(ui, "max");
        bound_control(ui, id, kw, upper, hi, hi_clamp, actions);
    });
    if !conditions::between_ok(lower.op, lo, upper.op, hi) {
        hint(ui, MIN_GT_MAX_HINT);
    }
}

/// One Between card: a lower and an upper bound line of the same keyword.
#[allow(clippy::too_many_arguments)]
fn between_card(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    lower: &BodyEntry,
    upper: &BodyEntry,
    kw: &'static Keyword,
    idx: usize,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let id = ctx.row.id;
    ui.push_id((id, kw.name, lower.ordinal, "between"), |ui| {
        let framed = card_frame(idx).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
            let x = condition_header(
                ui,
                &grammar::display_name(kw.name),
                kw.name,
                |ui| between_op_combo(ui, id, kw, lower, upper, actions),
                "Remove this condition",
            );
            if x.clicked() {
                let mut both = [lower, upper];
                both.sort_by_key(|e| std::cmp::Reverse(e.line));
                for e in both {
                    actions.push(remove(id, e));
                }
            }
            between_sliders(ui, id, kw, lower, upper, actions);
            for e in [lower, upper] {
                if let Some(msg) = error_for(list, id, e.line) {
                    hint(ui, msg);
                }
            }
        });
        let focus = [lower.line, upper.line]
            .iter()
            .any(|l| list.focus_line == Some((id, *l)));
        if focus {
            framed.response.scroll_to_me(Some(Align::Center));
            list.focus_line = None;
        }
        focus_flash(ui, framed.response.rect, ui.id().with("flash"), focus);
    });
}

/// A Between pair inside the Defences card, indented under its toggle.
fn defence_between(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    lower: &BodyEntry,
    upper: &BodyEntry,
    kw: &'static Keyword,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let id = ctx.row.id;
    ui.push_id((id, kw.name, lower.ordinal, "between"), |ui| {
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            between_op_combo(ui, id, kw, lower, upper, actions);
        });
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.vertical(|ui| between_sliders(ui, id, kw, lower, upper, actions));
        });
        for e in [lower, upper] {
            if let Some(msg) = error_for(list, id, e.line) {
                hint(ui, msg);
            }
        }
    });
}

// ---------------------------------------------------------------------------------------------
// Picker and pending rows
// ---------------------------------------------------------------------------------------------

type Groups = Vec<(&'static str, Vec<ConditionEntry>)>;

/// The label a picker row shows: the display name.
#[cfg(test)]
pub(crate) fn picker_label(kw: &Keyword) -> String {
    grammar::display_name(kw.name)
}

/// Keep the entries whose name or display name contains `query` (case-insensitive); empty
/// groups are dropped. An empty query keeps everything.
pub(crate) fn filter_groups(groups: &[(&'static str, Vec<ConditionEntry>)], query: &str) -> Groups {
    let q = query.trim().to_lowercase();
    groups
        .iter()
        .map(|(g, es)| {
            let kept: Vec<ConditionEntry> = es
                .iter()
                .filter(|e| q.is_empty() || e.search_text().contains(&q))
                .copied()
                .collect();
            (*g, kept)
        })
        .filter(|(_, es)| !es.is_empty())
        .collect()
}

/// The `+ condition` picker content for a search query.
#[cfg(test)]
pub(crate) fn picker_entries(query: &str) -> Groups {
    filter_groups(&grammar::condition_picker_groups(), query)
}

/// Keyword groups as picker entries (the action picker's input).
fn keyword_entries(groups: Vec<(&'static str, Vec<&'static Keyword>)>) -> Groups {
    groups
        .into_iter()
        .map(|(g, ks)| (g, ks.into_iter().map(ConditionEntry::Keyword).collect()))
        .collect()
}

/// Tooltip on a picker entry that is already on the rule.
const ALREADY_ON_RULE: &str = "Already on this rule";

/// Picker sections start closed (UAT3-07); only a search opens the matching ones.
const PICKER_SECTION_DEFAULT_OPEN: bool = false;

/// A forced open state for every picker section: open while searching, otherwise the user's own
/// toggling stands.
fn picker_section_override(searching: bool) -> Option<bool> {
    searching.then_some(true)
}

/// A searchable, grouped picker anchored to `button` (shared with `+ style`). Returns the entry
/// the user picked this frame. Entries for which `disabled` gives a reason are shown greyed with
/// that reason as their tooltip and cannot be clicked. Entries `present` on the rule are also
/// greyed but stay clickable (the caller focuses the existing card instead of adding another).
pub(crate) fn picker_popup(
    ui: &mut egui::Ui,
    button: &egui::Response,
    groups: &[(&'static str, Vec<ConditionEntry>)],
    searchable: bool,
    disabled: &dyn Fn(&ConditionEntry) -> Option<&'static str>,
    present: &dyn Fn(&ConditionEntry) -> bool,
) -> Option<ConditionEntry> {
    let popup_id = egui::Popup::default_response_id(button);
    let query_id = popup_id.with("query");
    let focus_id = popup_id.with("focused");
    let width = (ui.clip_rect().width() - 16.0).clamp(240.0, 440.0);
    let ctx = ui.ctx().clone();
    let hint_text = match groups.first().and_then(|g| g.1.first()) {
        Some(ConditionEntry::Keyword(k)) if k.kind == Kind::Action => "Find an action\u{2026}",
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
            let mut picked: Option<ConditionEntry> = None;
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    if filtered.is_empty() {
                        caption(ui, "Nothing matches");
                    }
                    for (label, kws) in filtered.iter() {
                        let mut header = egui::CollapsingHeader::new(
                            RichText::new(*label)
                                .size(theme::ROW_SUMMARY_SIZE)
                                .color(theme::LABEL_TAN),
                        )
                        .id_salt((popup_id, *label))
                        .default_open(PICKER_SECTION_DEFAULT_OPEN);
                        if let Some(open) = picker_section_override(searching) {
                            header = header.open(Some(open));
                        }
                        header.show(ui, |ui| {
                            for entry in kws {
                                let reason = disabled(entry);
                                let already = reason.is_none() && present(entry);
                                let text = RichText::new(entry.label()).size(14.0).color(
                                    if reason.is_some() || already {
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
                                    None => {
                                        let r = if already {
                                            r.on_hover_text(ALREADY_ON_RULE)
                                        } else {
                                            r
                                        };
                                        if r.clicked() {
                                            picked = Some(*entry);
                                        }
                                    }
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
    let mut picked_op: Option<Option<Operator>> = None;
    let mut items: Option<Vec<String>> = None;
    // Set when the first entry could not be written, so the refusal is shown instead of the entry
    // vanishing (WR-06).
    let bad_id = ui.id().with((p.block, p.keyword.name, "pending-bad"));
    let bad = ui.data(|d| d.get_temp::<bool>(bad_id).unwrap_or(false));
    ui.push_id((p.block, p.keyword.name, "pending"), |ui| {
        let framed = card_frame(idx).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
            discard = condition_header(
                ui,
                &grammar::display_name(p.keyword.name),
                p.keyword.name,
                |ui| {
                    // The operator the first entry will be written with is a dropdown from the
                    // start, so a carried-over negation is never silent (WR-01, iteration 2) and
                    // the author can pick it before any value exists.
                    let choices = grammar::operator_choices(p.keyword, p.op, false);
                    if !choices.is_empty() {
                        egui::ComboBox::from_id_salt((p.block, p.keyword.name, "pending-op"))
                            .selected_text(grammar::operator_label(p.keyword, p.op, false))
                            .width(OP_W)
                            .show_ui(ui, |ui| {
                                for c in &choices {
                                    if ui.selectable_label(c.op == p.op, c.label).clicked()
                                        && c.op != p.op
                                    {
                                        picked_op = Some(c.op);
                                    }
                                }
                            });
                    }
                },
                "Discard this condition",
            )
            .clicked();
            // The salt the real card will use once this row gets its first line: the new line's
            // keyword ordinal, and the same tag, so state keyed on it (the open class group)
            // carries over from the pending card to the real one.
            let ordinal = ctx
                .entries
                .iter()
                .filter(|e| e.keyword.is_some_and(|k| k.name == p.keyword.name))
                .count();
            let salt: Salt = (p.block, p.keyword.name, ordinal, "class");
            items = match p.keyword.value {
                ValueShape::Rarity => {
                    rarity_chips(ui, &[]).map(|r| r.iter().map(|s| s.to_string()).collect())
                }
                _ => match ctx.classes {
                    Some(cat) if p.keyword.name == "Class" => {
                        class_picker(ui, salt, list.row_epoch, cat, &[])
                    }
                    _ => quoted_list(ui, salt, &[], chip_hint(p.keyword)),
                },
            };
            if bad {
                hint(ui, BAD_TEXT_HINT);
            }
        });
        let focus = list.focus_card == Some((p.block, FocusTarget::Pending(p.keyword.name)));
        if focus {
            framed.response.scroll_to_me(Some(Align::Center));
            list.focus_card = None;
        }
        focus_flash(ui, framed.response.rect, ui.id().with("flash"), focus);
        framed
            .response
            .on_hover_text("Add an entry to create this condition");
    });
    // A picked operator is kept on the pending row (matched by block + keyword, whatever its
    // current operator) and used by this frame's first entry; nothing is written for it.
    let mut eff = p.clone();
    if let Some(op) = picked_op {
        eff.op = op;
        if let Some(row) = list
            .pending
            .iter_mut()
            .find(|q| q.block == p.block && q.keyword.name == p.keyword.name)
        {
            row.op = op;
        }
    }
    if discard {
        ui.data_mut(|d| d.remove::<bool>(bad_id));
        list.pending.retain(|q| q != p && q != &eff);
    } else if let Some(items) = items.filter(|i| !i.is_empty()) {
        match pending_first_entry(&eff, &items) {
            Some(a) => {
                ui.data_mut(|d| d.remove::<bool>(bad_id));
                actions.push(a);
                list.pending.retain(|q| q != p && q != &eff);
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
    /// The line is its keyword's only line, so the operator dropdown may offer Between (WR-03).
    can_between: bool,
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

#[allow(clippy::too_many_arguments)]
fn condition_card(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    entry: &BodyEntry,
    kw: &'static Keyword,
    idx: usize,
    can_between: bool,
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
            can_between,
        };
        let tokens: Vec<&str> = entry.values.iter().map(String::as_str).collect();
        let value = grammar::decode(kw, &tokens);
        let list_form = matches!(&value, Value::Rarity(r) if r.len() > 1);
        let mut primary: Option<egui::Response> = None;
        let framed = card_frame(idx).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
            let compact_bool = match (&kw.value, &value) {
                (ValueShape::Bool, Value::Bool { value: v }) => bool_effective(entry.op, *v),
                _ => None,
            };
            let x = condition_header(
                ui,
                &grammar::display_name(kw.name),
                kw.name,
                |ui| {
                    if let Some(eff) = compact_bool {
                        if let Some(want) = bool_yes_no(ui, eff) {
                            actions.extend(bool_edit(id, entry, want));
                        }
                    } else {
                        op_controls(ui, &card, list_form, actions);
                    }
                },
                "Remove this condition",
            );
            if x.clicked() {
                actions.push(remove(id, entry));
            }
            if compact_bool.is_none() {
                primary = value_controls(ui, &card, &value, list, actions);
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
        let focus = list.focus_line == Some((id, entry.line));
        if focus {
            framed.response.scroll_to_me(Some(Align::Center));
            if let Some(p) = primary {
                p.request_focus();
            }
            list.focus_line = None;
        }
        // A card focused from the picker (or just added) gets a brief accent outline.
        focus_flash(ui, framed.response.rect, ui.id().with("flash"), focus);
    });
}

/// One condition header row: fixed-width keyword label, the `middle` control, and the remove X
/// flush against the right edge so the X buttons line up across rows. Returns the X response.
fn condition_header(
    ui: &mut egui::Ui,
    label_text: &str,
    label_hover: &str,
    middle: impl FnOnce(&mut egui::Ui),
    remove_tip: &str,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add_sized(
            [LABEL_W, 24.0],
            egui::Label::new(
                RichText::new(label_text)
                    .size(14.0)
                    .color(theme::VALUE_TEXT),
            )
            .truncate(),
        )
        .on_hover_text(label_hover);
        middle(ui);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            widgets::remove_x_button(ui, remove_tip)
        })
        .inner
    })
    .inner
}

/// The Yes/No a boolean line reads as: negating operators flip the written word; ordering
/// operators have no Yes/No reading (`None`).
pub(crate) fn bool_effective(op: Option<Operator>, value: bool) -> Option<bool> {
    match op {
        None | Some(Operator::Eq) | Some(Operator::EqEq) => Some(value),
        Some(Operator::Ne) | Some(Operator::Bang) => Some(!value),
        _ => None,
    }
}

/// Set a boolean line to Yes (`want == true`) or No, keeping its operator and rewriting only the
/// value word. `None` when the line already reads that way or is not a plain boolean.
pub(crate) fn bool_edit(id: BlockId, entry: &BodyEntry, want: bool) -> Option<RowAction> {
    let value = match entry.values.as_slice() {
        [v] if v == "True" => true,
        [v] if v == "False" => false,
        _ => return None,
    };
    if bool_effective(entry.op, value)? == want {
        return None;
    }
    let negated = matches!(entry.op, Some(Operator::Ne) | Some(Operator::Bang));
    values_edit(
        id,
        entry,
        &Value::Bool {
            value: want ^ negated,
        },
    )
    .ok()
    .flatten()
}

/// The compact `Yes | No` control; `Some(want)` when the unselected side was clicked.
fn bool_yes_no(ui: &mut egui::Ui, effective: bool) -> Option<bool> {
    let mut picked = None;
    ui.spacing_mut().item_spacing.x = 2.0;
    for (label, val) in [("Yes", true), ("No", false)] {
        let mut sel = effective == val;
        let r = widgets::toggle_chip(ui, label, &mut sel);
        if r.changed() && val != effective {
            picked = Some(val);
        }
        r.on_hover_text("Remove the condition to match either.");
    }
    picked
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
            list_control(ui, card, names, list.row_epoch, actions);
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
        let n = widgets::number_field(ui, card.salt("num"), &mut v, widgets::NUMBER_FIELD_W);
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

/// Edit for a socket line: the count, with any colour letters (`5GRG`) kept as given.
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

/// A vertical list of quoted names with the quoted-paste add field, capped at 7 rows. Returns
/// the new list when it changed.
fn quoted_list(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash,
    items: &[String],
    hint_text: &str,
) -> Option<Vec<String>> {
    let out = widgets::string_list(ui, salt, items, hint_text, 7);
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

/// Selected classes the catalogue does not list; kept on write-back and surfaced on one line.
pub(crate) fn unknown_classes<'a>(
    selected: &'a [String],
    catalog: &ClassCatalog,
) -> Vec<&'a String> {
    selected.iter().filter(|c| !catalog.contains(c)).collect()
}

/// The grouped class picker: one collapsible group per catalogue group (labels and members come from config.json). Natural height, no inner scroll
/// area (Pitfall 8). Returns the new class list when the user changed it.
fn class_picker(
    ui: &mut egui::Ui,
    salt: Salt,
    row_epoch: u64,
    catalog: &ClassCatalog,
    selected: &[String],
) -> Option<Vec<String>> {
    let mut result = None;
    for (label, classes) in catalog.groups() {
        let n = classes.iter().filter(|c| selected.contains(c)).count();
        // Open/closed lives under an absolute id: the header's own id sits under the row's
        // epoch-scoped `push_id`, which the first value click changes (it adds a line), and under
        // a different `push_id` for the pending and the real card. Either would close the group
        // in use.
        let open_id = egui::Id::new(("ff-class-group-open", row_epoch, salt, *label));
        let open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        let out = egui::CollapsingHeader::new(
            RichText::new(format!("{label} ({n})"))
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::LABEL_TAN),
        )
        .id_salt((salt, *label))
        .open(Some(open))
        .show(ui, |ui| {
            for c in classes {
                let mut on = selected.contains(c);
                if ui.checkbox(&mut on, c.as_str()).changed() {
                    result = Some(class_toggle(selected, c, on));
                }
            }
        });
        if out.header_response.clicked() {
            ui.data_mut(|d| d.insert_temp(open_id, !open));
        }
    }
    let unknown = unknown_classes(selected, catalog);
    if !unknown.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for c in unknown {
                ui.label(
                    RichText::new(c.as_str())
                        .size(theme::ROW_SUMMARY_SIZE)
                        .color(theme::LABEL_TAN),
                )
                .on_hover_text("Not in config.json's class list; kept as written.");
                if widgets::remove_x_button(ui, &format!("Remove {c}")).clicked() {
                    result = Some(class_toggle(selected, c, false));
                }
            }
        });
    }
    result
}

fn list_control(
    ui: &mut egui::Ui,
    card: &Card,
    names: &[String],
    row_epoch: u64,
    actions: &mut Vec<RowAction>,
) {
    let next = match card.classes {
        Some(cat) if card.kw.name == "Class" => {
            class_picker(ui, card.salt("class"), row_epoch, cat, names)
        }
        _ => quoted_list(ui, card.salt("chips"), names, chip_hint(card.kw)),
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
            let (lo, hi, _) = grammar::slider_range("Sockets");
            // Leave room for the trailing "of these:" label.
            let (s, n) = ui
                .scope(|ui| {
                    ui.set_max_width((ui.available_width() - 80.0).max(180.0));
                    let s = widgets::themed_slider(ui, &mut v, lo, hi, false);
                    let n = widgets::number_field(
                        ui,
                        card.salt("count"),
                        &mut v,
                        widgets::NUMBER_FIELD_W,
                    );
                    (s, n)
                })
                .inner;
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
    let next = quoted_list(ui, card.salt("names"), names, chip_hint(card.kw));
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

fn sockets_control(
    ui: &mut egui::Ui,
    card: &Card,
    count: i64,
    colours: &str,
    actions: &mut Vec<RowAction>,
) {
    // Count only: colour letters on a foreign line pass through verbatim; the UI never offers them.
    let mut v = count;
    let mut changed = false;
    ui.horizontal(|ui| {
        let (lo, hi, _) = grammar::slider_range("Sockets");
        let s = widgets::themed_slider(ui, &mut v, lo, hi, false);
        let n = widgets::number_field(ui, card.salt("count"), &mut v, widgets::NUMBER_FIELD_W);
        changed = s.changed() || n.response.changed();
    });
    if changed
        && v != count
        && let Some(a) = sockets_edit(card.id, card.entry, v, colours)
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
#[cfg(test)]
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

/// The dispositions the editor offers. `Minimal` is a real header but is never offered; a foreign
/// Minimal block is displayed as such and left alone until the user picks Show or Hide.
pub(crate) const DISPOSITION_CHOICES: [BlockKeyword; 2] = [BlockKeyword::Show, BlockKeyword::Hide];

fn disposition_word(kw: BlockKeyword) -> &'static str {
    match kw {
        BlockKeyword::Show => "Show",
        BlockKeyword::Hide => "Hide",
        BlockKeyword::Minimal => "Minimal",
    }
}

/// Segmented Show/Hide control, drawn only for Custom rules (a preset's action decides it).
fn disposition_control(ui: &mut egui::Ui, ctx: &BodyCtx, actions: &mut Vec<RowAction>) {
    let row = ctx.row;
    let id = row.id;
    if row.keyword == BlockKeyword::Minimal {
        ui.label(
            RichText::new(disposition_word(BlockKeyword::Minimal)).color(theme::SECTION_CREAM),
        );
    }
    for kw in DISPOSITION_CHOICES {
        let selected = row.keyword == kw;
        let (text_color, fill) = if selected {
            (theme::SECTION_CREAM, theme::HEADER_PLATE)
        } else {
            (theme::LABEL_TAN, theme::ROW_BASE)
        };
        let btn =
            egui::Button::new(RichText::new(disposition_word(kw)).color(text_color)).fill(fill);
        if ui.add(btn).clicked() && !selected {
            actions.push(RowAction::SetKeyword(id, kw));
        }
    }
}

/// What the style picker can choose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StylePick {
    Custom,
    Preset(String),
}

/// The tag a rule carries, whatever its state.
fn style_tag(state: &StyleState) -> Option<&str> {
    match state {
        StyleState::None => None,
        StyleState::Known(n) | StyleState::Unknown(n) | StyleState::Unchecked(n) => Some(n),
    }
}

/// What a picker choice does: Custom drops the tag and keeps the actions; a preset sets the tag
/// and applies its actions. Re-picking the current choice does nothing.
pub(crate) fn style_pick_action(
    id: BlockId,
    state: &StyleState,
    pick: StylePick,
) -> Option<RowAction> {
    match pick {
        StylePick::Custom => style_tag(state).map(|_| RowAction::RemoveStyle(id)),
        StylePick::Preset(n) => (!matches!(state, StyleState::Known(k) if *k == n))
            .then_some(RowAction::ApplyStyle(id, n)),
    }
}

/// A preset owns the actions; every other state lets the author edit them.
pub(crate) fn shows_action_rows(state: &StyleState) -> bool {
    !matches!(state, StyleState::Known(_))
}

const STYLE_UNKNOWN_TIP: &str = "No style with this name exists. This rule keeps its current actions, and its style link is removed on the next Apply.";
const STYLE_UNCHECKED_TIP: &str =
    "The styles could not be read; this style is not applied until they load.";
const STYLE_KNOWN_TIP: &str = "Actions come from this style; Apply rewrites them.";

/// The picker: Custom first, then every preset with a swatch of its text colour.
fn style_picker(ui: &mut egui::Ui, ctx: &BodyCtx) -> Option<StylePick> {
    let id = ctx.row.id;
    let state = &ctx.row.style;
    let tag = style_tag(state);
    let (shown, colour, tip) = match state {
        StyleState::None => ("Custom".to_string(), theme::VALUE_TEXT, None),
        StyleState::Known(n) => (n.clone(), theme::VALUE_TEXT, Some(STYLE_KNOWN_TIP)),
        StyleState::Unknown(n) => (n.clone(), theme::WARM_ACCENT_TEXT, Some(STYLE_UNKNOWN_TIP)),
        StyleState::Unchecked(n) => (
            n.clone(),
            theme::WARM_ACCENT_TEXT,
            Some(STYLE_UNCHECKED_TIP),
        ),
    };
    let mut picked = None;
    let combo = egui::ComboBox::from_id_salt((id, "style-pick"))
        .selected_text(RichText::new(shown).color(colour))
        .width(140.0)
        .show_ui(ui, |ui| {
            if ui.selectable_label(tag.is_none(), "Custom").clicked() {
                picked = Some(StylePick::Custom);
            }
            for name in ctx.working.style_names() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if let Some(t) = ctx.working.style(&name).and_then(|s| s.text) {
                        dot(ui, egui::Color32::from_rgb(t[0], t[1], t[2]));
                    }
                    if ui
                        .selectable_label(tag == Some(name.as_str()), &name)
                        .clicked()
                    {
                        picked = Some(StylePick::Preset(name.clone()));
                    }
                });
            }
        });
    if let Some(t) = tip {
        combo.response.on_hover_text(t);
    }
    picked
}

/// The STYLE section, last in the rule body: the Custom/preset picker, then (Custom only)
/// Show/Hide and the action rows, then `Continue` on its own last line.
pub fn style_section(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let id = ctx.row.id;
    let added = added_actions(ctx.entries);
    let show_rows = shows_action_rows(&ctx.row.style);
    let mut picked_action = None;
    ui.horizontal(|ui| {
        caption(ui, "STYLE");
        if show_rows {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let btn = ui.add_enabled(
                    ctx.editable,
                    egui::Button::new(RichText::new("+ style").size(theme::ROW_SUMMARY_SIZE)),
                );
                let disabled = |e: &ConditionEntry| match e {
                    ConditionEntry::Keyword(k) => added.contains(&k.name).then_some("(added)"),
                    ConditionEntry::Defences | ConditionEntry::SimpleDefences => None,
                };
                picked_action = picker_popup(
                    ui,
                    &btn,
                    &keyword_entries(grammar::action_groups()),
                    false,
                    &disabled,
                    &|_| false,
                )
                .and_then(|e| match e {
                    ConditionEntry::Keyword(k) => Some(k),
                    ConditionEntry::Defences | ConditionEntry::SimpleDefences => None,
                });
            });
        }
    });
    if let Some(kw) = picked_action
        && let Some(a) = pick_action(id, kw)
    {
        actions.push(a);
    }
    ui.spacing_mut().item_spacing.y = 4.0;
    ui.add_enabled_ui(ctx.editable, |ui| {
        if let Some(p) = style_picker(ui, ctx)
            && let Some(a) = style_pick_action(id, &ctx.row.style, p)
        {
            actions.push(a);
        }
        if show_rows {
            ui.horizontal(|ui| disposition_control(ui, ctx, actions));
        }
    });
    if show_rows {
        action_rows(ui, ctx, list, actions);
    }
    ui.add_enabled_ui(ctx.editable, |ui| {
        // A style never owns `Continue`, so the checkbox stays editable on a styled rule.
        let mut cont = ctx.row.has_continue;
        let cb = egui::Checkbox::new(
            &mut cont,
            RichText::new("Continue to later rules").color(theme::LABEL_TAN),
        );
        if ui.add(cb).changed() {
            actions.push(RowAction::SetContinue(id, cont));
        }
    });
}

/// One card per action line (Custom / unstyled rules).
fn action_rows(ui: &mut egui::Ui, ctx: &BodyCtx, list: &mut ListUi, actions: &mut Vec<RowAction>) {
    let id = ctx.row.id;
    let lines: Vec<&BodyEntry> = ctx
        .entries
        .iter()
        .filter(|e| e.kind == EntryKind::Action)
        .collect();
    if lines.is_empty() {
        ui.label(
            RichText::new("No actions")
                .font(regular(15.0))
                .color(pal::MUTED),
        )
        .on_hover_text(NO_ACTIONS);
        return;
    }
    for (idx, e) in lines.into_iter().enumerate() {
        match e.keyword {
            Some(kw) if ctx.editable => {
                action_card(ui, ctx, e, kw, idx, None, list, actions);
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
            can_between: false,
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
    let n = widgets::number_field(ui, salt, &mut v, widgets::NUMBER_FIELD_W);
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
        let (n, r) = slider_pair(
            ui,
            card.salt("num"),
            current,
            grammar::FONT_SIZE_RANGE,
            &mut invalid,
        );
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
        let (n, r) = slider_pair(
            ui,
            card.salt("sid"),
            sound,
            grammar::SOUND_ID_RANGE,
            &mut invalid,
        );
        new_id = n;
        primary = Some(r);
    });
    ui.horizontal(|ui| {
        caption(ui, "Volume");
        let shown = volume.unwrap_or(100);
        new_vol = slider_pair(
            ui,
            card.salt("svol"),
            shown,
            grammar::VOLUME_RANGE,
            &mut invalid,
        )
        .0;
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
        resp = Some(r.on_hover_text("The file must be in the game folder."));
        commit = c;
    });
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
            grammar::VOLUME_RANGE,
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
                // The popup's ScrollArea auto-shrinks to its content width and its floating scroll
                // bar takes no room, painting over the content's right edge. A plain `horizontal`
                // row is only as wide as its dot + label, so the area shrank to the longest name and
                // the bar covered it. The label fills the rest of the row (justified, like the
                // shape combo's rows), so the content spans the whole popup and the bar sits at
                // the popup edge.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    dot(ui, filter_colour(c));
                    let label = ui
                        .with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                            ui.selectable_label(c == current, c)
                        })
                        .inner;
                    if label.clicked() && c != current {
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
        crate::skin::fonts::install(&ctx);
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
    fn removed_poe1_keyword_is_a_raw_row() {
        let (wf, _, e) = entries_of("Show\n    LinkedSockets 6\n");
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].kind, EntryKind::Opaque);
        assert_eq!(e[0].text, "    LinkedSockets 6");
        assert!(e[0].keyword.is_none());
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[0].id);
        for _ in 0..3 {
            let a = frame_of(&ctx, input(vec![]), &wf, &mut list, None);
            assert!(a.is_empty(), "idle frame emitted {a:?}");
        }
    }

    #[test]
    fn removed_exceptional_is_a_raw_row() {
        let (wf, _, e) = entries_of("Show\n    Exceptional True\n");
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].kind, EntryKind::Opaque);
        assert_eq!(e[0].text, "    Exceptional True");
        assert!(e[0].keyword.is_none());
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[0].id);
        for _ in 0..3 {
            let a = frame_of(&ctx, input(vec![]), &wf, &mut list, None);
            assert!(a.is_empty(), "idle frame emitted {a:?}");
        }
    }

    #[test]
    fn gate_ignores_removed_exceptional() {
        let doc = crate::filter::parse(b"Show\n    Exceptional True\n").unwrap();
        assert!(crate::dynamic::blocking_errors(&doc).is_empty());
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
            &format!("{dir}/styles-default.json"),
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
    fn base_type_list_is_vertical_and_idle() {
        let wf =
            WorkingFilter::from_text("Show\n    BaseType == \"Heavy Belt\" \"Utility Belt\"\n");
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[0].id);
        for _ in 0..3 {
            let a = frame_of(&ctx, input(vec![]), &wf, &mut list, None);
            assert!(a.is_empty(), "idle frame emitted {a:?}");
        }
        let src = include_str!("body.rs");
        assert!(src.contains(&["widgets::string", "_list("].concat()));
        assert!(!src.contains(&["fn list", "_chips"].concat()));
    }

    #[test]
    fn base_type_remove_writes_only_that_line() {
        let (_, id, e) = entries_of("Show\n    BaseType == \"Heavy Belt\" \"Utility Belt\"\n");
        let r = list_edit(id, &e[0], &["Utility Belt".to_string()]).unwrap();
        assert_eq!(values_of(r.clone()), Some("\"Utility Belt\"".to_string()));
        assert!(matches!(
            r,
            Some(RowAction::EditLine {
                patch: LinePatch::Values(_),
                ..
            })
        ));
    }

    #[test]
    fn class_picker_preserves_unknown_classes() {
        let cat = committed_catalog();
        let sel = vec!["Amulets".to_string(), "Custom Thing".to_string()];
        assert_eq!(unknown_classes(&sel, &cat), ["Custom Thing"]);
        let next = class_toggle(&sel, "Rings", true);
        assert!(next.contains(&"Custom Thing".to_string()));
        let (_, id, e) = entries_of("Show\n    Class \"Amulets\" \"Custom Thing\"\n");
        let v = values_of(list_edit(id, &e[0], &next).unwrap()).unwrap();
        assert!(v.contains("\"Custom Thing\""), "{v}");
    }

    #[test]
    fn bool_edit_writes_only_the_value_and_keeps_the_operator() {
        let (_, id, e) = entries_of("Show\n    Corrupted == True\n");
        let r = bool_edit(id, &e[0], false).unwrap();
        assert_eq!(
            r,
            RowAction::EditLine {
                id,
                line: 1,
                expect: e[0].text.clone(),
                patch: LinePatch::Values("False".into()),
            }
        );
        assert_eq!(bool_edit(id, &e[0], true), None);
    }

    #[test]
    fn bool_effective_reads_negations() {
        assert_eq!(bool_effective(Some(Operator::Ne), true), Some(false));
        assert_eq!(bool_effective(Some(Operator::Bang), true), Some(false));
        assert_eq!(bool_effective(None, true), Some(true));
        assert_eq!(bool_effective(Some(Operator::EqEq), true), Some(true));
        assert_eq!(bool_effective(Some(Operator::Ne), false), Some(true));
        assert_eq!(bool_effective(Some(Operator::Gt), true), None);
    }

    #[test]
    fn negated_bool_flip_keeps_bang_eq() {
        let (_, id, e) = entries_of("Show\n    Corrupted != True\n");
        assert_eq!(bool_edit(id, &e[0], false), None);
        let r = bool_edit(id, &e[0], true).unwrap();
        assert_eq!(values_of(Some(r)), Some("False".to_string()));
    }

    #[test]
    fn bool_with_ordering_operator_stays_raw() {
        let (_, id, e) = entries_of("Show\n    Corrupted > True\n");
        assert_eq!(bool_edit(id, &e[0], false), None);
        assert_eq!(bool_edit(id, &e[0], true), None);
        let wf = WorkingFilter::from_text("Show\n    Corrupted > True\n");
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[0].id);
        for _ in 0..3 {
            assert!(frame_of(&ctx, input(vec![]), &wf, &mut list, None).is_empty());
        }
    }

    #[test]
    fn expanded_bool_conditions_are_idle() {
        let wf = WorkingFilter::from_text(
            "Show\n    Corrupted == True\n    Identified False\n    Mirrored != True\n",
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
    fn condition_x_buttons_line_up() {
        let ctx = headless_ctx();
        let mut rights = Vec::new();
        let mut edge = 0.0;
        ctx.run_ui(input(vec![]), |ui| {
            let area = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 200.0));
            let mut child = ui.new_child(UiBuilder::new().max_rect(area));
            edge = child.max_rect().right();
            let r1 = condition_header(&mut child, "Short", "a", |_| {}, "Remove");
            let r2 = condition_header(
                &mut child,
                "A longer label",
                "b",
                |ui| {
                    ui.label("middle control");
                },
                "Remove",
            );
            rights.push(r1.rect.right());
            rights.push(r2.rect.right());
        })
        .drop_without_applying_deltas();
        assert!((rights[0] - rights[1]).abs() < 0.5, "{rights:?}");
        assert!((rights[0] - edge).abs() < 0.5, "{rights:?} vs {edge}");
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
    fn sockets_slider_keeps_colour_letters() {
        let (wf, id, e) = entries_of("Show\n    Sockets >= 5GGG\n");
        assert_eq!(e[0].values, ["5GGG"]);
        let a = sockets_edit(id, &e[0], 3, "GGG");
        assert_eq!(values_of(a.clone()), Some("3GGG".to_string()));
        let mut wf = wf;
        wf.apply_action(a.unwrap());
        assert_eq!(wf.text(), "Show\n    Sockets >= 3GGG\n");
        let (wf, id, e) = entries_of("Show\n    Sockets >= 5GGG\n");
        assert_eq!(sockets_edit(id, &e[0], 5, "GGG"), None);
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(id);
        for _ in 0..3 {
            let a = frame_of(&ctx, input(vec![]), &wf, &mut list, None);
            assert!(a.is_empty(), "idle frame emitted {a:?}");
        }
        let production = include_str!("body.rs").split("mod tests {").next().unwrap();
        assert!(!production.contains(&["\"+ ", "socket\""].concat()));
        assert!(!production.contains("SLOT_CHOICES"));
    }

    /// Topmost y at which a click emits an action matching `pred` (scan over the body).
    fn first_click_y(wf: &WorkingFilter, pred: impl Fn(&RowAction) -> bool) -> Option<f32> {
        let id = wf.rows()[0].id;
        for y in (20..700).step_by(4) {
            for x in (8..456).step_by(8) {
                let ctx = headless_ctx();
                let mut list = ListUi::default();
                list.expanded = Some(id);
                frame_of(&ctx, input(vec![]), wf, &mut list, None);
                let a = frame_of(
                    &ctx,
                    input(click_events(Pos2::new(x as f32, y as f32))),
                    wf,
                    &mut list,
                    None,
                );
                if a.iter().any(&pred) {
                    return Some(y as f32);
                }
            }
        }
        None
    }

    #[test]
    fn continue_is_the_last_body_line() {
        let wf = WorkingFilter::from_text("Show\n    Class \"Rings\"\n    SetFontSize 37\n");
        let cont = first_click_y(&wf, |a| matches!(a, RowAction::SetContinue(..)))
            .expect("Continue checkbox clickable");
        let disp = first_click_y(&wf, |a| matches!(a, RowAction::SetKeyword(..)))
            .expect("Show/Hide clickable");
        let remove = first_click_y(&wf, |a| matches!(a, RowAction::RemoveLine { line: 2, .. }))
            .expect("action row clickable");
        assert!(
            cont > disp,
            "Continue ({cont}) not below Show/Hide ({disp})"
        );
        assert!(
            cont > remove,
            "Continue ({cont}) not below actions ({remove})"
        );
    }

    #[test]
    fn custom_add_button_reads_plus_style() {
        let production = include_str!("body.rs").split("mod tests {").next().unwrap();
        assert!(production.contains(&["\"+ ", "style\""].concat()));
        assert!(!production.contains(&["\"+ ", "action\""].concat()));
    }

    #[test]
    fn disposition_hidden_for_presets() {
        let preset = styled("#ff:style S\nShow\n    Class \"Rings\"\n    SetFontSize 37\n");
        assert!(first_click_y(&preset, |a| matches!(a, RowAction::SetKeyword(..))).is_none());
        let minimal_preset = styled("#ff:style S\nMinimal\n    Class \"Rings\"\n");
        assert!(
            first_click_y(&minimal_preset, |a| matches!(a, RowAction::SetKeyword(..))).is_none()
        );
        let custom = WorkingFilter::from_text("Show\n    Class \"Rings\"\n");
        assert!(first_click_y(&custom, |a| matches!(a, RowAction::SetKeyword(..))).is_some());
        let minimal = WorkingFilter::from_text("Minimal\n    Class \"Rings\"\n");
        assert!(
            first_click_y(&minimal, |a| matches!(
                a,
                RowAction::SetKeyword(_, BlockKeyword::Show)
            ))
            .is_some()
        );
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

    fn names_of(g: &Groups) -> Vec<String> {
        g.iter()
            .flat_map(|(_, es)| es.iter().map(|e| e.label()))
            .collect()
    }

    #[test]
    fn picker_entries_filter_case_insensitively_and_drop_empty_groups() {
        let all = picker_entries("");
        assert_eq!(all.len(), grammar::condition_picker_groups().len());
        let level = picker_entries("LEVEL");
        let names = names_of(&level);
        for n in ["Item Level", "Drop Level", "Area Level", "Gem Level"] {
            assert!(names.iter().any(|x| x == n), "{n} missing from {names:?}");
        }
        assert!(level.iter().all(|(_, es)| !es.is_empty()));
        assert!(!names.iter().any(|x| x == "Class"));
        let spaced = picker_entries("item lev");
        assert_eq!(names_of(&spaced), ["Item Level"]);
        assert!(picker_entries("zzzz").is_empty());
    }

    #[test]
    fn picker_search_finds_defences_by_any_of_its_words() {
        for q in ["armour", "energy", "evasion", "defences"] {
            assert!(
                names_of(&picker_entries(q)).iter().any(|n| n == "Defences"),
                "{q}"
            );
        }
        let names = names_of(&picker_entries(""));
        for gone in ["Base Armour", "Base Evasion", "Base Energy Shield"] {
            assert!(!names.iter().any(|n| n == gone), "{gone}");
        }
    }

    #[test]
    fn removed_poe1_keywords_are_not_offered() {
        assert!(picker_entries("linked").is_empty());
        let mirage = picker_entries("mirage");
        assert_eq!(names_of(&mirage), ["Mirage Map"]);
        let kw = crate::filter::keywords::lookup("MirageMap").unwrap();
        assert_eq!(picker_label(kw), "Mirage Map");
        let item = crate::filter::keywords::lookup("ItemLevel").unwrap();
        assert_eq!(picker_label(item), "Item Level");
    }

    fn kw_entry(name: &str) -> ConditionEntry {
        ConditionEntry::Keyword(crate::filter::keywords::lookup(name).unwrap())
    }

    /// Run `pick_entry` the way `conditions_group` does for a rule parsed from `src`.
    fn pick_on(
        src: &str,
        list: &mut ListUi,
        entry: ConditionEntry,
    ) -> (BlockId, Vec<BodyEntry>, Option<RowAction>) {
        let (_, id, entries) = entries_of(src);
        let lines: Vec<&BodyEntry> = entries.iter().filter(|e| is_condition_side(e)).collect();
        let pending_kws: Vec<&'static Keyword> = list
            .pending
            .iter()
            .filter(|p| p.block == id)
            .map(|p| p.keyword)
            .collect();
        let present =
            conditions::present_entries(&lines, &pending_kws, any_pending_defences(list, id));
        let a = pick_entry(id, entry, &lines, &present, list);
        (id, entries.clone(), a)
    }

    #[test]
    fn picking_a_present_class_focuses_it_and_emits_nothing() {
        let mut list = ListUi::default();
        let (id, entries, a) = pick_on(
            "Show\n  ItemLevel >= 70\n  Class \"Rings\"\n",
            &mut list,
            kw_entry("Class"),
        );
        assert_eq!(a, None);
        let class_line = entries
            .iter()
            .find(|e| e.keyword.is_some_and(|k| k.name == "Class"))
            .unwrap()
            .line;
        assert_eq!(list.focus_line, Some((id, class_line)));
        assert!(list.pending.is_empty());
        assert!(list.pending_defences.is_empty());
    }

    #[test]
    fn picking_a_pending_class_does_not_add_a_second_pending_row() {
        let mut list = ListUi::default();
        let (id, _, _) = pick_on("Show\n", &mut list, kw_entry("Class"));
        assert_eq!(list.pending.len(), 1);
        let (_, _, a) = pick_on("Show\n", &mut list, kw_entry("Class"));
        assert_eq!(a, None);
        assert_eq!(list.pending.len(), 1);
        assert_eq!(list.focus_card, Some((id, FocusTarget::Pending("Class"))));
    }

    #[test]
    fn defences_present_through_a_loaded_line_focuses_the_card() {
        let mut list = ListUi::default();
        let (id, _, a) = pick_on(
            "Show\n  BaseEvasion > 5\n",
            &mut list,
            ConditionEntry::Defences,
        );
        assert_eq!(a, None);
        assert_eq!(list.focus_card, Some((id, FocusTarget::Defences)));
        assert!(list.pending_defences.is_empty());
    }

    #[test]
    fn picking_absent_defences_writes_nothing_and_becomes_pending() {
        let mut list = ListUi::default();
        let (id, _, a) = pick_on("Show\n  Rarity Rare\n", &mut list, ConditionEntry::Defences);
        assert_eq!(a, None);
        assert!(list.pending_defences.contains(&id));
        // Picking it again focuses the pending card instead of stacking another.
        let (_, _, a) = pick_on("Show\n  Rarity Rare\n", &mut list, ConditionEntry::Defences);
        assert_eq!(a, None);
        assert_eq!(list.focus_card, Some((id, FocusTarget::Defences)));
        assert_eq!(list.pending_defences.len(), 1);
    }

    #[test]
    fn picker_search_finds_simple_defences() {
        for q in ["simple", "evasion", "armour"] {
            assert!(
                names_of(&picker_entries(q))
                    .iter()
                    .any(|n| n == "Simple Defences"),
                "{q}"
            );
        }
    }

    #[test]
    fn picking_absent_simple_defences_becomes_pending_and_drops_the_advanced_mark() {
        let mut list = ListUi::default();
        let (id, _, _) = pick_on("Show\n  Rarity Rare\n", &mut list, ConditionEntry::Defences);
        assert!(list.advanced_defences.contains(&id));
        // Discard the pending Defences card the way its X does, then pick Simple Defences.
        list.pending_defences.remove(&id);
        let (_, _, a) = pick_on(
            "Show\n  Rarity Rare\n",
            &mut list,
            ConditionEntry::SimpleDefences,
        );
        assert_eq!(a, None);
        assert!(list.pending_simple_defences.contains(&id));
        assert!(!list.advanced_defences.contains(&id));
        assert!(list.pending_defences.is_empty());
    }

    #[test]
    fn picking_absent_defences_marks_the_rule_advanced() {
        let mut list = ListUi::default();
        let (id, _, a) = pick_on("Show\n  Rarity Rare\n", &mut list, ConditionEntry::Defences);
        assert_eq!(a, None);
        assert!(list.pending_defences.contains(&id));
        assert!(list.advanced_defences.contains(&id));
        assert!(list.pending_simple_defences.is_empty());
    }

    #[test]
    fn either_defences_entry_focuses_a_loaded_defence_line() {
        for entry in [ConditionEntry::Defences, ConditionEntry::SimpleDefences] {
            let mut list = ListUi::default();
            let (id, _, a) = pick_on("Show\n  BaseArmour > 0\n", &mut list, entry);
            assert_eq!(a, None);
            assert_eq!(list.focus_card, Some((id, FocusTarget::Defences)));
            assert!(list.pending_defences.is_empty());
            assert!(list.pending_simple_defences.is_empty());
            assert!(list.advanced_defences.is_empty());
        }
    }

    #[test]
    fn a_pending_defences_card_of_either_kind_blocks_the_other() {
        let mut list = ListUi::default();
        let (id, _, _) = pick_on(
            "Show\n  Rarity Rare\n",
            &mut list,
            ConditionEntry::SimpleDefences,
        );
        let (_, _, a) = pick_on("Show\n  Rarity Rare\n", &mut list, ConditionEntry::Defences);
        assert_eq!(a, None);
        assert_eq!(list.focus_card, Some((id, FocusTarget::Defences)));
        assert!(list.pending_defences.is_empty());
        assert!(!list.advanced_defences.contains(&id));

        let mut list = ListUi::default();
        let (id, _, _) = pick_on("Show\n  Rarity Rare\n", &mut list, ConditionEntry::Defences);
        let (_, _, a) = pick_on(
            "Show\n  Rarity Rare\n",
            &mut list,
            ConditionEntry::SimpleDefences,
        );
        assert_eq!(a, None);
        assert_eq!(list.focus_card, Some((id, FocusTarget::Defences)));
        assert!(list.pending_simple_defences.is_empty());
        assert!(list.advanced_defences.contains(&id));
    }

    #[test]
    fn picker_is_greyed_but_clickable_for_present_entries() {
        let production = include_str!("body.rs").split("mod tests {").next().unwrap();
        assert!(production.contains("Already on this rule"));
        // Present entries are only greyed: only a `disabled` reason blocks the click.
        assert!(production.contains("add_enabled_ui(reason.is_none()"));
    }

    // -- the Defences card --------------------------------------------------------------------

    const DEF_SRC: &str = "Show\n  BaseArmour >= 100\n  Rarity Rare\n  BaseEvasion > 5\n";

    fn defences_session(src: &str, pending: bool) -> (egui::Context, WorkingFilter, ListUi) {
        let wf = WorkingFilter::from_text(src);
        let id = wf.rows()[0].id;
        let mut list = ListUi::default();
        list.expanded = Some(id);
        if pending {
            list.pending_defences.insert(id);
        }
        (headless_ctx(), wf, list)
    }

    /// Frames until the card settles, then one frame with a click at `at`.
    fn click_after_settle(
        cat: &ClassCatalog,
        src: &str,
        pending: bool,
        at: Pos2,
    ) -> (WorkingFilter, ListUi, Vec<RowAction>) {
        let (ctx, mut wf, mut list) = defences_session(src, pending);
        for _ in 0..3 {
            click_step(&ctx, &mut wf, &mut list, cat, None);
        }
        let a = click_step(&ctx, &mut wf, &mut list, cat, Some(at));
        (wf, list, a)
    }

    fn scan_clicks(src: &str, pending: bool) -> Vec<(Pos2, Vec<RowAction>, WorkingFilter)> {
        scan_clicks_at(src, pending, &[30.0, 40.0, 52.0])
    }

    fn scan_clicks_at(
        src: &str,
        pending: bool,
        xs: &[f32],
    ) -> Vec<(Pos2, Vec<RowAction>, WorkingFilter)> {
        let cat = committed_catalog();
        let mut out = Vec::new();
        for y in (100..420).step_by(4) {
            for x in xs.iter().copied() {
                let at = Pos2::new(x, y as f32);
                let (wf, _, a) = click_after_settle(&cat, src, pending, at);
                if !a.is_empty() {
                    out.push((at, a, wf));
                }
            }
        }
        out
    }

    #[test]
    fn loaded_defence_lines_are_byte_identical_after_idle_frames() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = defences_session(DEF_SRC, false);
        let before = wf.text();
        for _ in 0..4 {
            assert!(click_step(&ctx, &mut wf, &mut list, &cat, None).is_empty());
        }
        assert_eq!(wf.text(), before);
        assert!(
            before.contains("BaseEvasion > 5"),
            "the `>` is not normalized"
        );
    }

    #[test]
    fn one_defences_card_at_the_first_defence_and_rarity_separate() {
        let (_, _, e) = entries_of(DEF_SRC);
        let lines: Vec<&BodyEntry> = e.iter().filter(|x| is_condition_side(x)).collect();
        let plan = conditions::card_plan(&lines);
        assert_eq!(plan.len(), 2);
        assert!(matches!(&plan[0], CardPlan::Defences(s) if s.len() == 2));
        assert!(matches!(plan[1], CardPlan::Part(Part::Single(1))));
    }

    #[test]
    fn two_armour_lines_stay_in_one_defences_card() {
        let src = "Show\n  BaseArmour >= 10\n  BaseArmour <= 90\n";
        let (_, _, e) = entries_of(src);
        let lines: Vec<&BodyEntry> = e.iter().filter(|x| is_condition_side(x)).collect();
        let plan = conditions::card_plan(&lines);
        assert_eq!(plan.len(), 1);
        assert!(matches!(&plan[0], CardPlan::Defences(s) if s.len() == 1));
        // Both lines survive idle frames untouched.
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = defences_session(src, false);
        let before = wf.text();
        assert!(click_step(&ctx, &mut wf, &mut list, &cat, None).is_empty());
        assert_eq!(wf.text(), before);
    }

    #[test]
    fn no_defence_lines_and_no_pending_means_no_card() {
        let (_, _, e) = entries_of("Show\n  Rarity Rare\n");
        let lines: Vec<&BodyEntry> = e.iter().filter(|x| is_condition_side(x)).collect();
        assert!(
            !conditions::card_plan(&lines)
                .iter()
                .any(|c| matches!(c, CardPlan::Defences(_)))
        );
    }

    #[test]
    fn toggling_energy_shield_on_adds_only_its_default_line() {
        let found = scan_clicks(DEF_SRC, false);
        let hit = found.iter().find(|(_, a, _)| {
            a.iter().any(
                |x| matches!(x, RowAction::AddLine { text, .. } if text == "BaseEnergyShield >= 1"),
            )
        });
        let (_, a, wf) = hit.expect("the Energy Shield toggle is clickable");
        assert_eq!(a.len(), 1, "{a:?}");
        let t = wf.text();
        assert!(t.contains("BaseEnergyShield >= 1"));
        assert!(t.contains("BaseArmour >= 100") && t.contains("BaseEvasion > 5"));
    }

    #[test]
    fn toggling_armour_off_removes_only_the_armour_line() {
        let found = scan_clicks(DEF_SRC, false);
        let hit = found.iter().find(|(_, a, _)| {
            a.iter().any(|x| {
                matches!(x, RowAction::RemoveLine { expect, .. } if expect.contains("BaseArmour"))
            })
        });
        let (_, a, wf) = hit.expect("the Armour toggle is clickable");
        assert_eq!(a.len(), 1, "{a:?}");
        let t = wf.text();
        assert!(!t.contains("BaseArmour"));
        assert!(t.contains("BaseEvasion > 5") && t.contains("Rarity Rare"));
    }

    #[test]
    fn defence_edits_touch_only_their_own_line() {
        // The card hands each control its own line's entry, so a slider or operator edit is an
        // EditLine on that line alone; the sibling defence line keeps its bytes.
        let src = "Show\n  BaseArmour >= 100\n  BaseEvasion > 5\n";
        let (mut wf, id, e) = entries_of(src);
        let armour = &e[0];
        let value = int_edit(id, armour, 250).unwrap();
        assert!(matches!(&value, RowAction::EditLine { line, .. } if *line == armour.line));
        assert!(wf.apply_action(value).changed);
        let op = op_edit(id, &wf.body_view(id)[0], Some(Operator::Le)).unwrap();
        assert!(matches!(&op, RowAction::EditLine { line, .. } if *line == armour.line));
        assert!(wf.apply_action(op).changed);
        let t = wf.text();
        assert!(t.contains("BaseArmour <= 250"), "{t}");
        assert!(t.contains("BaseEvasion > 5"), "{t}");
    }

    // -- the Simple Defences card ---------------------------------------------------------------

    const SIMPLE_SRC: &str = "Show\n  BaseArmour >= 1\n  BaseEvasion == 0\n";

    /// Chip order: Full AR, Full EV, Full ES, AR/EV, AR/ES, EV/ES, AR/EV/ES.
    const FULL_AR: usize = 0;
    const FULL_EV: usize = 1;
    const FULL_ES: usize = 2;

    fn simple_session(src: &str, pending: bool) -> (egui::Context, WorkingFilter, ListUi) {
        let (ctx, wf, mut list) = defences_session(src, false);
        if pending {
            list.pending_simple_defences.insert(wf.rows()[0].id);
        }
        (ctx, wf, list)
    }

    /// One `click_step`, returning its actions and the defences card kinds it rendered.
    fn step_cards(
        ctx: &egui::Context,
        wf: &mut WorkingFilter,
        list: &mut ListUi,
        cat: &ClassCatalog,
        at: Option<Pos2>,
    ) -> (Vec<RowAction>, Vec<&'static str>) {
        let id = wf.rows()[0].id;
        let key = egui::Id::new(("defences-cards", id));
        ctx.data_mut(|d| d.remove::<Vec<&'static str>>(key));
        let a = click_step(ctx, wf, list, cat, at);
        let kinds = ctx
            .data(|d| d.get_temp::<Vec<&'static str>>(key))
            .unwrap_or_default();
        (a, kinds)
    }

    fn probed(ctx: &egui::Context, key: impl std::hash::Hash + std::fmt::Debug) -> Rect {
        ctx.data(|d| d.get_temp::<Rect>(egui::Id::new(key)))
            .expect("control was drawn")
    }

    fn chip_at(ctx: &egui::Context, id: BlockId, chip: usize) -> Pos2 {
        probed(ctx, ("simple-chip", id, chip)).center()
    }

    /// Click chip `chip` of the first rule's card (settled first); returns the frame's actions.
    fn click_chip(
        ctx: &egui::Context,
        wf: &mut WorkingFilter,
        list: &mut ListUi,
        cat: &ClassCatalog,
        chip: usize,
    ) -> Vec<RowAction> {
        for _ in 0..2 {
            click_step(ctx, wf, list, cat, None);
        }
        let id = wf.rows()[0].id;
        let at = chip_at(ctx, id, chip);
        let (a, kinds) = step_cards(ctx, wf, list, cat, Some(at));
        assert_eq!(kinds, ["simple"]);
        a
    }

    fn shown(wf: &WorkingFilter, list: &ListUi) -> u8 {
        shown_selection(&wf.rows()[0], list)
    }

    #[test]
    fn zero_based_lines_load_as_one_simple_card_and_stay_byte_identical() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = simple_session(SIMPLE_SRC, false);
        let before = wf.text();
        for _ in 0..4 {
            let (a, kinds) = step_cards(&ctx, &mut wf, &mut list, &cat, None);
            assert!(a.is_empty(), "idle frame emitted {a:?}");
            assert_eq!(kinds, ["simple"]);
        }
        assert_eq!(wf.text(), before);
        // Has AR, no EV: Full AR and AR/ES.
        assert_eq!(shown(&wf, &list), 0b001_0001);
        for chip in 0..7 {
            probed(&ctx, ("simple-chip", wf.rows()[0].id, chip));
        }
    }

    #[test]
    fn clicking_a_chip_writes_the_new_selection() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = simple_session(SIMPLE_SRC, false);
        let id = wf.rows()[0].id;
        // Add Full ES: Full AR, Full ES, AR/ES = "no Evasion", one line.
        let a = click_chip(&ctx, &mut wf, &mut list, &cat, FULL_ES);
        assert_eq!(
            a,
            [RowAction::SetDefenceSelection {
                id,
                mask: 0b001_0101
            }]
        );
        assert_eq!(wf.text(), "Show\n  BaseEvasion 0\n");
        assert_eq!(shown(&wf, &list), 0b001_0101);
    }

    #[test]
    fn a_selection_one_block_cannot_express_becomes_one_grouped_rule() {
        let cat = committed_catalog();
        let src = "Show\n  Rarity Rare\n";
        let (ctx, mut wf, mut list) = simple_session(src, true);
        click_chip(&ctx, &mut wf, &mut list, &cat, FULL_AR);
        assert_eq!(
            wf.text(),
            "Show\n  Rarity Rare\n  BaseEvasion 0\n  BaseEnergyShield 0\n"
        );
        assert!(list.pending_simple_defences.is_empty());
        click_chip(&ctx, &mut wf, &mut list, &cat, FULL_EV);
        assert_eq!(wf.rows().len(), 1);
        assert_eq!(wf.rows()[0].group_size, 2);
        assert_eq!(shown(&wf, &list), 0b011);
        assert_eq!(wf.text().matches("# ff:group ").count(), 2);
        // Still exactly one card, idle.
        let (a, kinds) = step_cards(&ctx, &mut wf, &mut list, &cat, None);
        assert!(a.is_empty(), "{a:?}");
        assert_eq!(kinds, ["simple"]);
        // Deselect Full AR: one block again, no group line.
        click_chip(&ctx, &mut wf, &mut list, &cat, FULL_AR);
        assert_eq!(wf.rows()[0].group_size, 1);
        assert_eq!(
            wf.text(),
            "Show\n  Rarity Rare\n  BaseArmour 0\n  BaseEnergyShield 0\n"
        );
    }

    #[test]
    fn non_zero_based_defences_render_the_defences_card() {
        let cat = committed_catalog();
        for src in [
            "Show\n  BaseArmour >= 100\n",
            "Show\n  BaseArmour > 0\n  BaseArmour <= 50\n",
            "Show\n  BaseArmour > 0\n  BaseEvasion >= 30\n",
        ] {
            let (ctx, mut wf, mut list) = simple_session(src, false);
            for _ in 0..2 {
                let (a, kinds) = step_cards(&ctx, &mut wf, &mut list, &cat, None);
                assert!(a.is_empty(), "{src}: {a:?}");
                assert_eq!(kinds, ["defences"], "{src}");
            }
            assert_eq!(wf.text(), src);
        }
    }

    #[test]
    fn pending_simple_card_writes_nothing_until_a_chip_is_picked() {
        let cat = committed_catalog();
        let src = "Show\n  Rarity Rare\n";
        let (ctx, mut wf, mut list) = simple_session(src, true);
        for _ in 0..3 {
            let (a, kinds) = step_cards(&ctx, &mut wf, &mut list, &cat, None);
            assert!(a.is_empty());
            assert_eq!(kinds, ["simple"]);
        }
        assert_eq!(wf.text(), src);
        assert_eq!(shown(&wf, &list), 0);
    }

    #[test]
    fn every_or_no_chip_writes_no_line_and_keeps_the_card() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = simple_session("Show\n  Rarity Rare\n", true);
        let id = wf.rows()[0].id;
        for chip in 0..7 {
            click_chip(&ctx, &mut wf, &mut list, &cat, chip);
        }
        // Every combination is no defence line at all, yet the card shows them all selected.
        assert_eq!(wf.text(), "Show\n  Rarity Rare\n");
        assert!(list.pending_simple_defences.contains(&id));
        assert_eq!(shown(&wf, &list), group::ALL_COMBOS);
        // Deselect one again: lines come back.
        click_chip(&ctx, &mut wf, &mut list, &cat, 6);
        assert_eq!(shown(&wf, &list), 0b011_1111);
        assert!(!list.pending_simple_defences.contains(&id));
        // Down to the last chip, then off: no line, the card stays open and empty.
        for chip in 1..6 {
            click_chip(&ctx, &mut wf, &mut list, &cat, chip);
        }
        assert_eq!(shown(&wf, &list), 0b000_0001);
        click_chip(&ctx, &mut wf, &mut list, &cat, FULL_AR);
        assert_eq!(wf.text(), "Show\n  Rarity Rare\n");
        assert!(list.pending_simple_defences.contains(&id));
        assert_eq!(shown(&wf, &list), 0);
        let (_, kinds) = step_cards(&ctx, &mut wf, &mut list, &cat, None);
        assert_eq!(kinds, ["simple"]);
    }

    #[test]
    fn simple_card_x_removes_every_defence_line_and_collapses_a_group() {
        let cat = committed_catalog();
        let src = "# ff:group abc 1/2\nShow\n  BaseArmour > 0\n  Rarity Rare\n  BaseEvasion 0\n  BaseEnergyShield 0\n\n# ff:group abc 2/2\nShow\n  BaseArmour 0\n  Rarity Rare\n  BaseEvasion > 0\n  BaseEnergyShield 0\n";
        let (ctx, mut wf, mut list) = simple_session(src, false);
        assert_eq!(wf.rows()[0].group_size, 2);
        for _ in 0..3 {
            click_step(&ctx, &mut wf, &mut list, &cat, None);
        }
        let id = wf.rows()[0].id;
        list.pending_simple_defences.insert(id);
        let x = probed(&ctx, ("simple-x", id)).center();
        let a = click_step(&ctx, &mut wf, &mut list, &cat, Some(x));
        assert_eq!(a, [RowAction::SetDefenceSelection { id, mask: 0 }]);
        assert!(list.pending_simple_defences.is_empty());
        assert!(list.advanced_defences.is_empty());
        assert_eq!(wf.text(), "Show\n  Rarity Rare\n");
    }

    #[test]
    fn focus_on_a_simple_card_is_consumed() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = simple_session(SIMPLE_SRC, false);
        click_step(&ctx, &mut wf, &mut list, &cat, None);
        let id = wf.rows()[0].id;
        list.focus_card = Some((id, FocusTarget::Defences));
        let (a, kinds) = step_cards(&ctx, &mut wf, &mut list, &cat, None);
        assert!(a.is_empty());
        assert_eq!(kinds, ["simple"]);
        assert_eq!(list.focus_card, None);
    }

    #[test]
    fn a_loaded_group_shows_one_card_with_the_union_and_stays_byte_identical() {
        let src = "# ff:name Pure\n# ff:group abc 1/3\nShow\n  Rarity Rare\n  BaseEvasion 0\n  BaseEnergyShield 0\n\n# ff:name Pure\n# ff:group abc 2/3\nShow\n  Rarity Rare\n  BaseArmour 0\n  BaseEnergyShield 0\n\n# ff:name Pure\n# ff:group abc 3/3\nShow\n  Rarity Rare\n  BaseArmour 0\n  BaseEvasion 0\n";
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = simple_session(src, false);
        assert_eq!(wf.rows().len(), 1);
        for _ in 0..3 {
            let (a, kinds) = step_cards(&ctx, &mut wf, &mut list, &cat, None);
            assert!(a.is_empty(), "{a:?}");
            assert_eq!(kinds, ["simple"]);
        }
        assert_eq!(shown(&wf, &list), 0b111);
        assert_eq!(wf.text(), src);
        // The picker offers no second defences card on a group.
        let id = wf.rows()[0].id;
        assert_eq!(pick_in(&wf, &mut list, ConditionEntry::Defences), None);
        assert_eq!(list.focus_card, Some((id, FocusTarget::Defences)));
        assert!(list.pending_defences.is_empty());
    }

    // -- card stability and exclusivity through the UI -------------------------------------------

    /// `pick_entry` the way `conditions_group` runs it, on the session's current text.
    fn pick_in(wf: &WorkingFilter, list: &mut ListUi, entry: ConditionEntry) -> Option<RowAction> {
        let id = wf.rows()[0].id;
        let entries = wf.body_view(id);
        let lines: Vec<&BodyEntry> = entries.iter().filter(|e| is_condition_side(e)).collect();
        let present = conditions::present_entries(&lines, &[], any_pending_defences(list, id));
        pick_entry(id, entry, &lines, &present, list)
    }

    fn settle(
        ctx: &egui::Context,
        wf: &mut WorkingFilter,
        list: &mut ListUi,
        cat: &ClassCatalog,
    ) -> Vec<&'static str> {
        let mut kinds = Vec::new();
        for _ in 0..2 {
            let (a, k) = step_cards(ctx, wf, list, cat, None);
            assert!(a.is_empty(), "idle frame emitted {a:?}");
            kinds = k;
        }
        kinds
    }

    #[test]
    fn defences_card_does_not_flip_to_simple_after_toggle() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = defences_session("Show\n  Rarity Rare\n", false);
        let id = wf.rows()[0].id;
        assert_eq!(pick_in(&wf, &mut list, ConditionEntry::Defences), None);
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["defences"]);
        let at = probed(&ctx, ("defences-toggle", id, "BaseArmour")).center();
        let (a, _) = step_cards(&ctx, &mut wf, &mut list, &cat, Some(at));
        assert!(
            matches!(&a[..], [RowAction::AddLine { text, .. }] if text == "BaseArmour >= 1"),
            "{a:?}"
        );
        assert!(wf.text().contains("BaseArmour >= 1"));
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["defences"]);
        assert!(list.advanced_defences.contains(&id));
    }

    #[test]
    fn loaded_defences_card_edited_to_zero_based_stays_defences() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = defences_session("Show\n  BaseArmour >= 100\n", false);
        let id = wf.rows()[0].id;
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["defences"]);
        let view = wf.body_view(id);
        let a = int_edit(id, &view[0], 1).unwrap();
        assert!(wf.apply_action(a).changed);
        assert!(wf.text().contains("BaseArmour >= 1"));
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["defences"]);

        // The Apply re-parse is not a reload: the open card stays Defences (review WR-01).
        let id = reparse_as_apply(&mut wf, &mut list);
        assert_eq!(list.expanded, Some(id));
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["defences"]);

        // A reload (clear_ids, run after Revert and RELOAD) classifies from the file again: the
        // same text now loads as Simple Defences.
        list.clear_ids();
        list.expanded = Some(id);
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["simple"]);
        assert_eq!(wf.text(), "Show\n  BaseArmour >= 1\n");
    }

    /// What `run_apply` does to the list after a successful Apply: the document is re-parsed (ids
    /// reset) and the list is re-keyed by row position. Returns the first row's new id.
    fn reparse_as_apply(wf: &mut WorkingFilter, list: &mut ListUi) -> BlockId {
        let kept = list.kept_positions(wf);
        *wf = WorkingFilter::from_text(&wf.text());
        list.rekey_after_apply(kept, Some(wf));
        wf.rows()[0].id
    }

    #[test]
    fn a_defences_card_toggled_on_stays_defences_after_apply() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = defences_session("Show\n  Rarity Rare\n", false);
        let id = wf.rows()[0].id;
        assert_eq!(pick_in(&wf, &mut list, ConditionEntry::Defences), None);
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["defences"]);
        let at = probed(&ctx, ("defences-toggle", id, "BaseArmour")).center();
        let (a, _) = step_cards(&ctx, &mut wf, &mut list, &cat, Some(at));
        assert_eq!(a.len(), 1, "{a:?}");
        assert_eq!(wf.text(), "Show\n  Rarity Rare\n  BaseArmour >= 1\n");

        let id = reparse_as_apply(&mut wf, &mut list);
        assert_eq!(list.expanded, Some(id));
        assert!(list.advanced_defences.contains(&id));
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["defences"]);
    }

    #[test]
    fn a_simple_classified_rule_stays_simple_after_apply() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = simple_session("Show\n  BaseArmour > 0\n", false);
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["simple"]);
        let id = reparse_as_apply(&mut wf, &mut list);
        assert!(list.advanced_defences.is_empty());
        assert_eq!(list.expanded, Some(id));
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["simple"]);
    }

    #[test]
    fn simple_card_edits_keep_the_rule_simple() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = simple_session("Show\n  Rarity Rare\n", true);
        let id = wf.rows()[0].id;
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["simple"]);
        // + Full AR, + Full EV, + AR/EV, - Full AR, + AR/EV/ES, - Full EV.
        for chip in [FULL_AR, FULL_EV, 3, FULL_AR, 6, FULL_EV] {
            let at = chip_at(&ctx, id, chip);
            let (a, kinds) = step_cards(&ctx, &mut wf, &mut list, &cat, Some(at));
            assert_eq!(a.len(), 1, "{chip}: {a:?}");
            assert_eq!(kinds, ["simple"]);
            assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["simple"]);
            assert_eq!(wf.rows().len(), 1);
        }
        // AR/EV and AR/EV/ES: has Armour and Evasion.
        assert_eq!(
            wf.text(),
            "Show\n  Rarity Rare\n  BaseArmour > 0\n  BaseEvasion > 0\n"
        );
        assert!(list.advanced_defences.is_empty());
    }

    #[test]
    fn removing_the_defences_card_clears_the_mark_for_a_simple_pick() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = defences_session("Show\n  BaseArmour >= 100\n", false);
        let id = wf.rows()[0].id;
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["defences"]);
        assert!(list.advanced_defences.contains(&id));
        let x = probed(&ctx, ("defences-x", id)).center();
        let (a, _) = step_cards(&ctx, &mut wf, &mut list, &cat, Some(x));
        assert!(matches!(&a[..], [RowAction::RemoveLine { .. }]), "{a:?}");
        assert!(!list.advanced_defences.contains(&id));
        assert_eq!(wf.text(), "Show\n");
        assert_eq!(
            pick_in(&wf, &mut list, ConditionEntry::SimpleDefences),
            None
        );
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["simple"]);
    }

    #[test]
    fn never_both_defences_cards() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = simple_session("Show\n  Rarity Rare\n", true);
        let id = wf.rows()[0].id;
        assert_eq!(pick_in(&wf, &mut list, ConditionEntry::Defences), None);
        assert_eq!(list.focus_card, Some((id, FocusTarget::Defences)));
        assert!(list.pending_defences.is_empty());
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["simple"]);
        assert_eq!(list.focus_card, None, "the Simple card took the focus");

        let (ctx, mut wf, mut list) = defences_session("Show\n  Rarity Rare\n", true);
        assert_eq!(
            pick_in(&wf, &mut list, ConditionEntry::SimpleDefences),
            None
        );
        assert!(list.pending_simple_defences.is_empty());
        assert_eq!(settle(&ctx, &mut wf, &mut list, &cat), ["defences"]);
        assert_eq!(list.focus_card, None, "the Defences card took the focus");
    }

    #[test]
    fn pending_defences_writes_nothing_and_clears_on_first_toggle() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = defences_session("Show\n  Rarity Rare\n", true);
        let before = wf.text();
        for _ in 0..4 {
            assert!(click_step(&ctx, &mut wf, &mut list, &cat, None).is_empty());
        }
        assert_eq!(wf.text(), before);
        let id = wf.rows()[0].id;
        assert!(list.pending_defences.contains(&id));

        let found = scan_clicks("Show\n  Rarity Rare\n", true);
        let (_, a, wf2) = found
            .iter()
            .find(|(_, a, _)| a.iter().any(|x| matches!(x, RowAction::AddLine { .. })))
            .expect("a pending toggle is clickable");
        assert_eq!(a.len(), 1);
        let t = wf2.text();
        assert!(
            t.contains("BaseArmour >= 1")
                || t.contains("BaseEvasion >= 1")
                || t.contains("BaseEnergyShield >= 1")
        );
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
            "/styles-default.json"
        ))
        .unwrap()));
        wf
    }

    #[test]
    fn style_owned_follows_style_output() {
        let set =
            crate::loader::load_styles(concat!(env!("CARGO_MANIFEST_DIR"), "/styles-default.json"))
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
    fn style_pick_action_maps_custom_and_presets() {
        let id = BlockId(1);
        let known = StyleState::Known("S".into());
        assert_eq!(
            style_pick_action(id, &known, StylePick::Custom),
            Some(RowAction::RemoveStyle(id))
        );
        assert_eq!(
            style_pick_action(id, &StyleState::None, StylePick::Custom),
            None
        );
        assert_eq!(
            style_pick_action(id, &StyleState::None, StylePick::Preset("S".into())),
            Some(RowAction::ApplyStyle(id, "S".into()))
        );
        assert_eq!(
            style_pick_action(id, &known, StylePick::Preset("S".into())),
            None
        );
        assert_eq!(
            style_pick_action(id, &StyleState::Unknown("X".into()), StylePick::Custom),
            Some(RowAction::RemoveStyle(id))
        );
    }

    #[test]
    fn preset_hides_action_rows_custom_shows_them() {
        assert!(!shows_action_rows(&StyleState::Known("S".into())));
        assert!(shows_action_rows(&StyleState::None));
        assert!(shows_action_rows(&StyleState::Unknown("X".into())));
        assert!(shows_action_rows(&StyleState::Unchecked("X".into())));
    }

    #[test]
    fn preset_to_custom_keeps_actions_and_drops_tag() {
        let mut wf = styled("#ff:style S\nShow\n  Class \"Rings\"\n  SetFontSize 37\n");
        let row = &wf.rows()[0];
        let id = row.id;
        let a = style_pick_action(id, &row.style, StylePick::Custom).unwrap();
        wf.apply_action(a);
        assert_eq!(wf.text(), "Show\n  Class \"Rings\"\n  SetFontSize 37\n");
    }

    #[test]
    fn disposition_offers_show_and_hide_only() {
        assert_eq!(
            DISPOSITION_CHOICES,
            [BlockKeyword::Show, BlockKeyword::Hide]
        );
    }

    #[test]
    fn minimal_block_renders_idle_and_untouched() {
        let src = "Minimal\n    Rarity Rare\n";
        let wf = WorkingFilter::from_text(src);
        let id = wf.rows()[0].id;
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(id);
        for _ in 0..3 {
            let a = frame_of(&ctx, input(vec![]), &wf, &mut list, None);
            assert!(a.is_empty(), "idle frame emitted {a:?}");
        }
        assert_eq!(wf.text(), src);
        let mut found = false;
        'scan: for y in (20..600).step_by(6) {
            for x in (8..456).step_by(8) {
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
                if a.contains(&RowAction::SetKeyword(id, BlockKeyword::Show)) {
                    found = true;
                    break 'scan;
                }
            }
        }
        assert!(found, "no click position emitted SetKeyword(Show)");
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

    /// Mirrors `app::apply_row_actions` for the actions a value click emits. Every `ListUi`/
    /// `WorkingFilter` call it makes, in app.rs order:
    /// 1. `working.apply_action(action)` for each action;
    /// 2. on `changed` + `AddLine`: `list.focus_line = Some((block, outcome.line))`;
    /// 3. on `changed` + (`AddLine` | `RemoveLine`): `list.forget_card_state()` (WR-07).
    ///
    /// The `list.expanded` / `pending_scroll` resets in app.rs only fire for AddRule, Duplicate,
    /// Move and Delete, none of which a value click emits.
    fn mirror_apply(wf: &mut WorkingFilter, list: &mut ListUi, actions: Vec<RowAction>) {
        for action in actions {
            let shifts = matches!(
                action,
                RowAction::AddLine { .. }
                    | RowAction::RemoveLine { .. }
                    | RowAction::InsertLineAfter { .. }
                    | RowAction::SetDefenceSelection { .. }
            );
            let added_in = match &action {
                RowAction::AddLine { id, .. } => Some(*id),
                _ => None,
            };
            let outcome = wf.apply_action(action);
            if outcome.changed {
                if let (Some(b), Some(line)) = (added_in, outcome.line) {
                    list.focus_line = Some((b, line));
                }
                if shifts {
                    list.forget_card_state();
                }
            }
        }
    }

    /// One frame of the list, a click at `at` when given, then the app.rs bookkeeping.
    fn click_step(
        ctx: &egui::Context,
        wf: &mut WorkingFilter,
        list: &mut ListUi,
        cat: &ClassCatalog,
        at: Option<Pos2>,
    ) -> Vec<RowAction> {
        let raw = match at {
            Some(p) => input(click_events(p)),
            None => input(vec![]),
        };
        let actions = frame_of(ctx, raw, wf, list, Some(cat));
        mirror_apply(wf, list, actions.clone());
        actions
    }

    const CLASS_HEADER_X: f32 = 40.0;

    /// A fresh session with an expanded rule and a pending Class row, after the group header at
    /// `header` was clicked and (when given) the checkbox at `box_at` was clicked.
    fn pending_class_session(
        cat: &ClassCatalog,
        header: f32,
        box_at: Option<Pos2>,
    ) -> (
        egui::Context,
        WorkingFilter,
        ListUi,
        BlockId,
        Vec<RowAction>,
    ) {
        let mut wf = WorkingFilter::from_text("Show\n    ItemLevel >= 70\n");
        let id = wf.rows()[0].id;
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(id);
        let class = crate::filter::keywords::lookup("Class").unwrap();
        assert_eq!(pick_condition(id, class, &mut list), None);
        click_step(&ctx, &mut wf, &mut list, cat, None);
        click_step(&ctx, &mut wf, &mut list, cat, None);
        click_step(
            &ctx,
            &mut wf,
            &mut list,
            cat,
            Some(Pos2::new(CLASS_HEADER_X, header)),
        );
        // The group opens over a few frames (egui animates it).
        for _ in 0..3 {
            click_step(&ctx, &mut wf, &mut list, cat, None);
        }
        let mut got = Vec::new();
        if let Some(p) = box_at {
            got = click_step(&ctx, &mut wf, &mut list, cat, Some(p));
            for _ in 0..3 {
                click_step(&ctx, &mut wf, &mut list, cat, None);
            }
        }
        (ctx, wf, list, id, got)
    }

    /// A fresh expanded rule with a pending row for `kw`, settled for a few frames.
    fn pending_session(
        cat: &ClassCatalog,
        kw: &str,
    ) -> (egui::Context, WorkingFilter, ListUi, BlockId) {
        let mut wf = WorkingFilter::from_text("Show\n    ItemLevel >= 70\n");
        let id = wf.rows()[0].id;
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(id);
        let kw = crate::filter::keywords::lookup(kw).unwrap();
        assert_eq!(pick_condition(id, kw, &mut list), None);
        for _ in 0..3 {
            click_step(&ctx, &mut wf, &mut list, cat, None);
        }
        (ctx, wf, list, id)
    }

    /// Click each point in turn, three idle frames after each; the click-frame actions.
    fn play(
        ctx: &egui::Context,
        wf: &mut WorkingFilter,
        list: &mut ListUi,
        cat: &ClassCatalog,
        clicks: &[Pos2],
    ) -> Vec<RowAction> {
        let mut out = Vec::new();
        for p in clicks {
            out.extend(click_step(ctx, wf, list, cat, Some(*p)));
            for _ in 0..3 {
                click_step(ctx, wf, list, cat, None);
            }
        }
        out
    }

    #[test]
    fn pending_class_operator_is_chosen_before_a_value_and_used_by_it() {
        let cat = committed_catalog();
        // The operator dropdown is on the pending card: some click opens a popup, and the popup
        // offers "Not" (other popups, such as the style picker, do not).
        let mut found = None;
        'a: for y in (194..230).step_by(4) {
            for x in (150..440).step_by(10) {
                let (ctx, mut wf, mut list, _) = pending_session(&cat, "Class");
                let p = Pos2::new(x as f32, y as f32);
                play(&ctx, &mut wf, &mut list, &cat, &[p]);
                if !egui::Popup::is_any_open(&ctx) {
                    continue;
                }
                // Choosing "Not" is stored on the pending row and writes nothing.
                for iy in ((y + 8)..(y + 160)).step_by(4) {
                    for dx in [-60.0f32, -40.0, -20.0, 0.0, 20.0, 40.0] {
                        let (ctx, mut wf, mut list, _) = pending_session(&cat, "Class");
                        let before = wf.text();
                        let q = Pos2::new(p.x + dx, iy as f32);
                        let a = play(&ctx, &mut wf, &mut list, &cat, &[p, q]);
                        if list.pending.len() == 1 && list.pending[0].op == Some(Operator::Ne) {
                            assert!(a.is_empty(), "choosing an operator writes nothing: {a:?}");
                            assert_eq!(wf.text(), before);
                            found = Some((p, q));
                            break 'a;
                        }
                    }
                }
            }
        }
        let (combo, item) = found.expect("a pending Class row offers the operator Not");

        // An idle frame with the operator set still writes nothing.
        let (ctx, mut wf, mut list, _) = pending_session(&cat, "Class");
        let before = wf.text();
        play(&ctx, &mut wf, &mut list, &cat, &[combo, item]);
        assert!(click_step(&ctx, &mut wf, &mut list, &cat, None).is_empty());
        assert_eq!(wf.text(), before);

        // The first class then lands with that operator.
        let mut text = None;
        'c: for hy in (218..242).step_by(4) {
            for (cx, cy) in [36.0f32, 90.0, 130.0]
                .into_iter()
                .flat_map(|x| ((hy + 16)..(hy + 120)).step_by(4).map(move |y| (x, y)))
            {
                let (ctx, mut wf, mut list, _) = pending_session(&cat, "Class");
                let a = play(
                    &ctx,
                    &mut wf,
                    &mut list,
                    &cat,
                    &[
                        combo,
                        item,
                        Pos2::new(CLASS_HEADER_X, hy as f32),
                        Pos2::new(cx, cy as f32),
                    ],
                );
                for act in a {
                    if let RowAction::AddLine { text: t, .. } = act {
                        text = Some(t);
                        break 'c;
                    }
                }
            }
        }
        let text = text.expect("a class click writes the line");
        assert!(text.starts_with("Class != \""), "{text}");
    }

    #[test]
    fn pending_base_type_starts_exact_and_shows_its_operator() {
        let cat = committed_catalog();
        let (_, _, list, _) = pending_session(&cat, "BaseType");
        assert_eq!(list.pending[0].op, Some(Operator::EqEq));
        let p = list.pending[0].clone();
        let a = pending_first_entry(&p, &["Heavy Belt".to_string()]).unwrap();
        assert!(
            matches!(&a, RowAction::AddLine { text, .. } if text == "BaseType == \"Heavy Belt\"")
        );
        assert!(!grammar::operator_choices(p.keyword, p.op, false).is_empty());
    }

    #[test]
    fn pending_rarity_operator_applies_to_one_chip_but_not_to_two() {
        let cat = committed_catalog();
        let (_, _, mut list, id) = pending_session(&cat, "Rarity");
        assert!(!grammar::operator_choices(list.pending[0].keyword, None, false).is_empty());
        list.pending[0].op = Some(Operator::Ge);
        let p = list.pending[0].clone();
        let one = pending_first_entry(&p, &["Magic".to_string()]).unwrap();
        assert!(
            matches!(&one, RowAction::AddLine { id: i, text, .. } if *i == id && text == "Rarity >= Magic")
        );
        let two = pending_first_entry(&p, &["Magic".to_string(), "Rare".to_string()]).unwrap();
        assert!(matches!(&two, RowAction::AddLine { text, .. } if text == "Rarity Magic Rare"));
    }

    #[test]
    fn idle_pending_rows_write_nothing_for_any_keyword() {
        let cat = committed_catalog();
        for kw in ["Class", "BaseType", "Rarity"] {
            let (ctx, mut wf, mut list, _) = pending_session(&cat, kw);
            let before = wf.text();
            for _ in 0..3 {
                assert!(click_step(&ctx, &mut wf, &mut list, &cat, None).is_empty());
            }
            assert_eq!(wf.text(), before, "{kw}");
            assert_eq!(list.pending.len(), 1, "{kw}");
        }
    }

    #[test]
    fn picker_sections_start_collapsed_and_a_search_opens_them() {
        // Nothing opens by itself (this was `gi == 0`, which opened "Item").
        assert!(!PICKER_SECTION_DEFAULT_OPEN);
        assert_eq!(picker_section_override(false), None);
        assert_eq!(picker_section_override(true), Some(true));
        let production = include_str!("body.rs").split("mod tests {").next().unwrap();
        assert!(!production.contains(&["default_open(gi", " == 0)"].concat()));
        assert!(production.contains("default_open(PICKER_SECTION_DEFAULT_OPEN)"));
    }

    #[test]
    fn first_rarity_chip_click_keeps_the_rule_expanded() {
        let cat = committed_catalog();
        let mut wrote = false;
        'scan: for y in (100..400).step_by(4) {
            for x in (8..456).step_by(8) {
                let mut wf = WorkingFilter::from_text("Show\n    ItemLevel >= 70\n");
                let id = wf.rows()[0].id;
                let ctx = headless_ctx();
                let mut list = ListUi::default();
                list.expanded = Some(id);
                let rarity = crate::filter::keywords::lookup("Rarity").unwrap();
                assert_eq!(pick_condition(id, rarity, &mut list), None);
                for _ in 0..3 {
                    click_step(&ctx, &mut wf, &mut list, &cat, None);
                }
                let a = click_step(
                    &ctx,
                    &mut wf,
                    &mut list,
                    &cat,
                    Some(Pos2::new(x as f32, y as f32)),
                );
                if a.iter().any(|a| matches!(a, RowAction::AddLine { .. })) {
                    for _ in 0..3 {
                        click_step(&ctx, &mut wf, &mut list, &cat, None);
                    }
                    assert_eq!(list.expanded, Some(id), "rarity chip collapsed the rule");
                    wrote = true;
                    break 'scan;
                }
            }
        }
        assert!(wrote, "a rarity chip is clickable");
    }

    #[test]
    fn first_class_click_keeps_rule_and_group_open() {
        let cat = committed_catalog();
        // Find a group header y and a checkbox position below it whose click writes the first
        // class.
        let mut found = None;
        'scan: for hy in (218..242).step_by(4) {
            for cx in [36.0f32, 60.0, 90.0, 130.0] {
                for cy in ((hy + 16)..(hy + 120)).step_by(4) {
                    let at = Pos2::new(cx, cy as f32);
                    let (_, _, _, _, got) = pending_class_session(&cat, hy as f32, Some(at));
                    if got.iter().any(|a| matches!(a, RowAction::AddLine { .. })) {
                        found = Some((hy as f32, at));
                        break 'scan;
                    }
                }
            }
        }
        let (hy, at) = found.expect("a pending Class group checkbox is clickable");

        // After the AddLine the rule is still the expanded one.
        let (_, wf, list, id, _) = pending_class_session(&cat, hy, Some(at));
        assert!(wf.text().contains("Class"), "the line was written");
        assert_eq!(list.expanded, Some(id), "the rule row stayed expanded");

        // And the group the click happened in is still open: some click in the body now toggles
        // a second class checkbox (an EditLine that rewrites the Class values).
        let mut reopened = false;
        'second: for y in (40..700).step_by(4) {
            let (ctx, mut wf, mut list, _, _) = pending_class_session(&cat, hy, Some(at));
            let a = click_step(
                &ctx,
                &mut wf,
                &mut list,
                &cat,
                Some(Pos2::new(at.x, y as f32)),
            );
            if a.iter().any(|a| {
                matches!(
                    a,
                    RowAction::EditLine {
                        patch: LinePatch::Values(t),
                        ..
                    } if t.contains('"')
                )
            }) {
                reopened = true;
                break 'second;
            }
        }
        assert!(
            reopened,
            "the class group closed when the first class turned the pending row into a line"
        );
    }

    // -- Between (UAT3-06) ----------------------------------------------------------------------

    fn apply_all(wf: &mut WorkingFilter, actions: Vec<RowAction>) {
        for a in actions {
            let o = wf.apply_action(a);
            assert!(o.changed, "refused: {:?}", o.status);
        }
    }

    fn rendered_text(ctx: &egui::Context, wf: &WorkingFilter, list: &mut ListUi) -> String {
        let visible: Vec<usize> = (0..wf.rows().len()).collect();
        let cat = committed_catalog();
        let mut out = String::new();
        let full = ctx.run_ui(input(vec![]), |ui| {
            let area = Rect::from_min_size(Pos2::ZERO, vec2(460.0, 900.0));
            let mut child = ui.new_child(UiBuilder::new().max_rect(area));
            show_list(&mut child, wf, &visible, list, true, Some(&cat));
        });
        for clipped in &full.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                out.push_str(t.galley.text());
                out.push('\n');
            }
        }
        full.drop_without_applying_deltas();
        out
    }

    const PAIR: &str = "Show\n  ItemLevel > 60\n  ItemLevel <= 80\n";

    #[test]
    fn loaded_bound_pair_is_one_between_card_and_byte_identical() {
        let (_, _, e) = entries_of(PAIR);
        let lines: Vec<&BodyEntry> = e.iter().filter(|x| is_condition_side(x)).collect();
        assert_eq!(
            conditions::card_plan(&lines),
            [CardPlan::Part(Part::Between { lower: 0, upper: 1 })]
        );
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = defences_session(PAIR, false);
        let before = wf.text();
        for _ in 0..4 {
            assert!(click_step(&ctx, &mut wf, &mut list, &cat, None).is_empty());
        }
        assert_eq!(wf.text(), before);
        assert!(before.contains("ItemLevel > 60") && before.contains("ItemLevel <= 80"));
    }

    #[test]
    fn between_card_renders_between_and_no_impossible_flag_for_a_good_pair() {
        let (ctx, wf, mut list) = defences_session(PAIR, false);
        let mut seen = String::new();
        for _ in 0..3 {
            seen = rendered_text(&ctx, &wf, &mut list);
        }
        assert!(seen.contains("Between"), "{seen}");
        assert!(!seen.contains("min > max"), "{seen}");
    }

    #[test]
    fn loaded_impossible_pair_is_flagged_and_not_rewritten() {
        let src = "Show\n  ItemLevel >= 80\n  ItemLevel <= 10\n";
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = defences_session(src, false);
        let before = wf.text();
        let mut seen = String::new();
        for _ in 0..3 {
            assert!(click_step(&ctx, &mut wf, &mut list, &cat, None).is_empty());
            seen = rendered_text(&ctx, &wf, &mut list);
        }
        assert!(seen.contains("min > max"), "{seen}");
        assert_eq!(wf.text(), before);
    }

    #[test]
    fn editing_the_min_touches_only_the_min_line() {
        let (mut wf, id, e) = entries_of(PAIR);
        let a = int_edit(id, &e[0], 65).expect("changed");
        apply_all(&mut wf, vec![a]);
        assert_eq!(wf.text(), "Show\n  ItemLevel > 65\n  ItemLevel <= 80\n");
        let e = wf.body_view(id);
        let a = int_edit(id, &e[1], 90).expect("changed");
        apply_all(&mut wf, vec![a]);
        assert_eq!(wf.text(), "Show\n  ItemLevel > 65\n  ItemLevel <= 90\n");
    }

    #[test]
    fn bounds_are_clamped_so_min_never_exceeds_max() {
        // min slider dragged past a max of 80
        assert_eq!(clamped_bound(60, 95, (i64::MIN, 80)), Some(80));
        // max slider dragged below a min of 60
        assert_eq!(clamped_bound(80, 10, (60, i64::MAX)), Some(60));
        // already at the limit: nothing to write
        assert_eq!(clamped_bound(80, 95, (i64::MIN, 80)), None);
        // a loaded impossible pair is repaired only by an edit, to the other bound
        assert_eq!(clamped_bound(80, 70, (i64::MIN, 10)), Some(10));
    }

    #[test]
    fn other_duplicates_stay_separate_cards() {
        for src in [
            "Show\n  ItemLevel >= 60\n  ItemLevel >= 70\n",
            "Show\n  ItemLevel == 60\n  ItemLevel >= 70\n",
            "Show\n  ItemLevel >= 60\n  ItemLevel <= 70\n  ItemLevel <= 90\n",
        ] {
            let (_, _, e) = entries_of(src);
            let lines: Vec<&BodyEntry> = e.iter().filter(|x| is_condition_side(x)).collect();
            let plan = conditions::card_plan(&lines);
            assert!(
                plan.iter()
                    .all(|c| matches!(c, CardPlan::Part(Part::Single(_)))),
                "{src}"
            );
            let (ctx, wf, mut list) = defences_session(src, false);
            let seen = rendered_text(&ctx, &wf, &mut list);
            assert!(!seen.contains("Between"), "{src}: {seen}");
        }
    }

    #[test]
    fn only_whole_number_conditions_offer_between() {
        use conditions::NumericOp;
        for (name, offered) in [
            ("ItemLevel", true),
            ("BaseArmour", true),
            ("Class", false),
            ("Rarity", false),
            ("Corrupted", false),
        ] {
            let kw = crate::filter::keywords::lookup(name).unwrap();
            let has = conditions::numeric_op_choices(kw, NumericOp::Single(None))
                .iter()
                .any(|(c, _)| *c == NumericOp::Between);
            assert_eq!(has, offered, "{name}");
        }
    }

    fn switch_to_between(src: &str) -> (WorkingFilter, Vec<RowAction>) {
        let (mut wf, id, e) = entries_of(src);
        let kw = e[0].keyword.expect("keyword");
        let actions = to_between_actions(id, &e[0], kw);
        apply_all(&mut wf, actions.clone());
        (wf, actions)
    }

    #[test]
    fn switching_a_lower_bound_to_between_only_inserts_the_upper() {
        let (wf, a) = switch_to_between("Show\n  ItemLevel >= 60\n  Rarity Rare\n");
        assert_eq!(a.len(), 1, "{a:?}");
        assert!(matches!(
            &a[0],
            RowAction::InsertLineAfter { after: 1, expect, text, .. }
                if expect == "  ItemLevel >= 60" && text == "ItemLevel <= 100"
        ));
        assert_eq!(
            wf.text(),
            "Show\n  ItemLevel >= 60\n  ItemLevel <= 100\n  Rarity Rare\n"
        );
    }

    #[test]
    fn switching_an_upper_bound_to_between_inserts_the_lower_after_it() {
        let (wf, a) = switch_to_between("Show\n  ItemLevel <= 80\n");
        assert_eq!(a.len(), 1, "{a:?}");
        assert!(matches!(
            &a[0],
            RowAction::InsertLineAfter { text, .. } if text == "ItemLevel >= 1"
        ));
        assert_eq!(wf.text(), "Show\n  ItemLevel <= 80\n  ItemLevel >= 1\n");
    }

    #[test]
    fn switching_equals_to_between_makes_it_the_lower_bound() {
        let (wf, a) = switch_to_between("Show\n  ItemLevel == 70\n");
        assert_eq!(a.len(), 2, "{a:?}");
        assert!(
            matches!(&a[0], RowAction::InsertLineAfter { text, .. } if text == "ItemLevel <= 70")
        );
        assert!(matches!(&a[1], RowAction::EditLine { .. }));
        assert_eq!(wf.text(), "Show\n  ItemLevel >= 70\n  ItemLevel <= 70\n");
    }

    #[test]
    fn the_next_frame_after_a_switch_is_a_between_card_and_the_rule_stays_open() {
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = defences_session("Show\n  ItemLevel >= 60\n", false);
        let id = wf.rows()[0].id;
        let e = wf.body_view(id);
        let acts = to_between_actions(id, &e[0], e[0].keyword.unwrap());
        mirror_apply(&mut wf, &mut list, acts);
        for _ in 0..3 {
            assert!(click_step(&ctx, &mut wf, &mut list, &cat, None).is_empty());
        }
        assert_eq!(list.expanded, Some(id));
        let seen = rendered_text(&ctx, &wf, &mut list);
        assert!(seen.contains("Between"), "{seen}");
    }

    fn switch_away(src: &str, pick: Option<Operator>) -> (WorkingFilter, Vec<RowAction>) {
        let (mut wf, id, e) = entries_of(src);
        let lines: Vec<&BodyEntry> = e.iter().filter(|x| is_condition_side(x)).collect();
        let Some(CardPlan::Part(Part::Between { lower, upper })) =
            conditions::card_plan(&lines).into_iter().next()
        else {
            panic!("not a between pair");
        };
        let actions = from_between_actions(id, lines[lower], lines[upper], pick);
        apply_all(&mut wf, actions.clone());
        (wf, actions)
    }

    #[test]
    fn leaving_between_keeps_the_matching_bound_and_removes_the_other() {
        let (wf, a) = switch_away(PAIR, Some(Operator::Gt));
        assert_eq!(a.len(), 1, "keeps `>` untouched: {a:?}");
        assert_eq!(wf.text(), "Show\n  ItemLevel > 60\n");
        let (wf, a) = switch_away(PAIR, Some(Operator::Ge));
        assert_eq!(a.len(), 2, "{a:?}");
        assert_eq!(wf.text(), "Show\n  ItemLevel >= 60\n");
        let (wf, a) = switch_away(PAIR, Some(Operator::Le));
        assert_eq!(a.len(), 1, "{a:?}");
        assert_eq!(wf.text(), "Show\n  ItemLevel <= 80\n");
        let (wf, _) = switch_away(
            "Show\n  ItemLevel <= 80\n  ItemLevel >= 60\n",
            Some(Operator::Ge),
        );
        assert_eq!(wf.text(), "Show\n  ItemLevel >= 60\n");
        let (wf, _) = switch_away(PAIR, Some(Operator::EqEq));
        assert_eq!(wf.text(), "Show\n  ItemLevel == 60\n");
    }

    #[test]
    fn defences_show_a_between_pair_as_one_sub_row_and_stay_idle() {
        let src = "Show\n  BaseArmour >= 100\n  BaseArmour <= 500\n  Rarity Rare\n";
        let cat = committed_catalog();
        let (ctx, mut wf, mut list) = defences_session(src, false);
        let before = wf.text();
        for _ in 0..3 {
            assert!(click_step(&ctx, &mut wf, &mut list, &cat, None).is_empty());
        }
        assert_eq!(wf.text(), before);
        let seen = rendered_text(&ctx, &wf, &mut list);
        assert!(seen.contains("Between"), "{seen}");
        let (_, _, e) = entries_of(src);
        let lines: Vec<&BodyEntry> = e.iter().filter(|x| is_condition_side(x)).collect();
        let plan = conditions::card_plan(&lines);
        assert!(matches!(
            &plan[0],
            CardPlan::Defences(s) if s[0].1 == vec![Part::Between { lower: 0, upper: 1 }]
        ));
    }

    #[test]
    fn defences_switch_to_between_inserts_right_after_the_armour_line() {
        let (mut wf, id, e) = entries_of("Show\n  BaseArmour >= 100\n  BaseEvasion > 5\n");
        let a = to_between_actions(id, &e[0], e[0].keyword.unwrap());
        apply_all(&mut wf, a);
        assert_eq!(
            wf.text(),
            "Show\n  BaseArmour >= 100\n  BaseArmour <= 1000\n  BaseEvasion > 5\n"
        );
    }
}

/// UAT4-02: a colour dropdown leaves its scroll bar room like the shape dropdown. Driven headless
/// with real fonts; the bar and the item names are read from the painted output.
#[cfg(test)]
mod colour_combo_tests {
    use crate::filter::keywords::{COLOURS, SHAPES};
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, vec2};

    const BUTTON: Pos2 = Pos2::new(72.0, 9.0);
    const IN_POPUP: Pos2 = Pos2::new(60.0, 100.0);

    fn raw(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(460.0, 900.0))),
            events,
            ..Default::default()
        }
    }

    fn click(pos: Pos2) -> Vec<Event> {
        let button = |pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::default(),
        };
        vec![Event::PointerMoved(pos), button(true), button(false)]
    }

    /// The reference: the minimap shape dropdown's rows (plain selectable labels), laid out the
    /// same way as a colour dropdown (dot, then a 104px combo).
    fn shape_combo(ui: &mut egui::Ui) {
        super::dot(ui, egui::Color32::WHITE);
        egui::ComboBox::from_id_salt("shape")
            .selected_text("Circle")
            .width(104.0)
            .show_ui(ui, |ui| {
                for s in SHAPES {
                    let _ = ui.selectable_label(*s == "Circle", *s);
                }
            });
    }

    struct Popup {
        /// The scroll bar's painted rect (background track), if one was drawn.
        bar: Option<Rect>,
        /// Painted item names and their rects, below the combo button.
        items: Vec<(String, Rect)>,
        bar_width: f32,
        picked: Vec<Option<String>>,
    }

    /// Opens the dropdown drawn by `draw`, hovers the popup until the floating bar settles, and
    /// returns what was painted. `then` events are sent on the last frame (e.g. a click).
    fn open(
        draw: &dyn Fn(&mut egui::Ui) -> Option<String>,
        then: impl Fn(&Popup) -> Vec<Event>,
    ) -> Popup {
        let ctx = egui::Context::default();
        crate::overlay::theme::install_fonts(&ctx);
        let mut popup = Popup {
            bar: None,
            items: Vec::new(),
            bar_width: 0.0,
            picked: Vec::new(),
        };
        let frames = 16;
        for frame in 0..frames {
            let events = match frame {
                1 => click(BUTTON),
                f if f == frames - 1 => then(&popup),
                f if f >= 2 => vec![Event::PointerMoved(IN_POPUP)],
                _ => Vec::new(),
            };
            let mut raw = raw(events);
            raw.time = Some(f64::from(frame) * 0.1);
            let mut picked = None;
            let mut out = ctx.run_ui(raw, |ui| {
                popup.bar_width = ui.spacing().scroll.bar_width;
                ui.horizontal(|ui| picked = draw(ui));
            });
            out.textures_delta.clear();
            popup.picked.push(picked);
            popup.bar = None;
            popup.items.clear();
            for cs in &out.shapes {
                match &cs.shape {
                    Shape::Rect(r)
                        if r.rect.width() <= popup.bar_width + 0.5 && r.rect.height() > 100.0 =>
                    {
                        let bar = popup.bar.get_or_insert(r.rect);
                        *bar = bar.union(r.rect);
                    }
                    Shape::Text(t) if t.pos.y > 20.0 => {
                        let rect = t.galley.rect.translate(t.pos.to_vec2());
                        popup.items.push((t.galley.text().to_string(), rect));
                    }
                    _ => {}
                }
            }
        }
        popup
    }

    fn assert_bar_clears_items(p: &Popup, names: &[&str], what: &str) {
        let bar = p
            .bar
            .unwrap_or_else(|| panic!("{what}: no scroll bar painted"));
        // At full (hovered) width the bar spans bar_width left of its right edge.
        let bar_left = bar.right() - p.bar_width;
        let shown: Vec<_> = p
            .items
            .iter()
            .filter(|(t, _)| names.contains(&t.as_str()))
            .collect();
        assert!(shown.len() >= 5, "{what}: items painted {:?}", p.items);
        for (t, r) in shown {
            assert!(
                r.right() <= bar_left,
                "{what}: {t:?} ends at {} under the bar {bar:?}",
                r.right()
            );
        }
    }

    #[test]
    fn colour_combo_rows_leave_scroll_bar_room_like_the_shape_combo() {
        let shape = open(
            &|ui| {
                shape_combo(ui);
                None
            },
            |_| Vec::new(),
        );
        assert_bar_clears_items(&shape, SHAPES, "shape combo");

        let colour = open(&|ui| super::colour_combo(ui, "c", "Red", false), |_| {
            Vec::new()
        });
        assert_bar_clears_items(&colour, COLOURS, "colour combo");
    }

    #[test]
    fn colour_combo_picks_and_offers_none() {
        let at = |name: &'static str| {
            move |p: &Popup| {
                let r = p
                    .items
                    .iter()
                    .find(|(t, _)| t == name)
                    .map(|(_, r)| *r)
                    .unwrap_or_else(|| panic!("{name} not painted"));
                click(r.center())
            }
        };
        let p = open(&|ui| super::colour_combo(ui, "c", "Red", false), at("Blue"));
        assert_eq!(p.picked.last().unwrap().as_deref(), Some("Blue"));
        let p = open(&|ui| super::colour_combo(ui, "c", "Red", false), at("Red"));
        assert_eq!(p.picked.last().unwrap(), &None);
        // A taller popup so the last entry is painted without scrolling.
        let tall = |allow_none: bool| {
            move |ui: &mut egui::Ui| {
                ui.spacing_mut().combo_height = 600.0;
                super::colour_combo(ui, "c", "Red", allow_none)
            }
        };
        let p = open(&tall(true), |_| Vec::new());
        assert_eq!(p.items.last().map(|(t, _)| t.as_str()), Some("None"));
        let p = open(&tall(false), |_| Vec::new());
        assert!(!p.items.iter().any(|(t, _)| t == "None"));
        assert!(p.items.iter().any(|(t, _)| t == "Purple"));
    }
}
