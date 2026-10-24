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

use egui::{Align, Align2, Layout, RichText, Sense, pos2, vec2};

use crate::filter::keywords::{COLOURS, Keyword, Kind, RARITIES, SHAPES, ValueShape};
use crate::filter::tokenize::Operator;
use crate::filter::{BlockId, BlockKeyword, LinePatch, group};

use super::classes::ClassCatalog;
use super::conditions::{self, CardPlan, DefencesKind, Part};
use super::config_state::ColourSlot;
use super::grammar::{
    self, BoolOrName, ConditionEntry, Effect, EncodeError, Minimap, SoundRef, Value,
};
use super::rule_list::{FocusTarget, ListUi, PendingRow};
use super::rules::{BodyEntry, EntryKind, LineKind, RowAction, RuleRow, StyleState, WorkingFilter};
use super::style_lines::{self, Rgba, StyleIntent, StyleView, Vis};
use super::widgets;
#[cfg(test)]
use crate::models::config::Style;
use crate::models::config::{
    self as mc, Beam, Cancelable, DropSound, FilterColor, MinimapShape, Sound,
};
use crate::skin::fonts::{regular, small_caps};
use crate::skin::palette as pal;
use crate::skin::widgets as widgets_skin;

/// Left inset of a condition / action card: its value controls start in the control column.
const CARD_LEFT: f32 = widgets_skin::ROW_PAD + widgets_skin::LABEL_COL_W + widgets_skin::LABEL_GAP;
const OP_W: f32 = 140.0;

/// A skin dropdown over `(value, label)` choices. Returns `Some(value)` only when the user picks a
/// different entry than `current`. A current value that is not among the choices is appended as an
/// extra option with its own label, so what is shown is exact and nothing is normalised.
fn pick_dropdown<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    current: &T,
    current_label: &str,
    choices: &[(T, String)],
    width: f32,
) -> Option<T> {
    let skin = crate::skin::of(ui.ctx());
    let mut labels: Vec<&str> = choices.iter().map(|(_, l)| l.as_str()).collect();
    let start = match choices.iter().position(|(v, _)| v == current) {
        Some(i) => i,
        None => {
            labels.push(current_label);
            labels.len() - 1
        }
    };
    let mut sel = start;
    let r = widgets_skin::ff_dropdown(ui, &skin, salt, &mut sel, &labels, width);
    if r.changed() && sel != start {
        choices.get(sel).map(|(v, _)| v.clone())
    } else {
        None
    }
}

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
    ui.label(RichText::new(text).small().color(pal::LABEL));
}

pub(crate) fn hint(ui: &mut egui::Ui, text: &str) {
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .size(crate::skin::fonts::SUMMARY_SIZE)
                .color(pal::WARN),
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
    draft_field_ex(ui, tid, current, width, list, key, regular(15.0))
}

/// [`draft_field`] with an explicit font (raw filter rows use 14pt monospace).
fn draft_field_ex(
    ui: &mut egui::Ui,
    tid: egui::Id,
    current: &str,
    width: f32,
    list: &mut ListUi,
    key: (BlockId, usize),
    font: egui::FontId,
) -> (egui::Response, Option<String>) {
    let focused = ui.memory(|m| m.has_focus(tid));
    let stored = ui.data_mut(|d| d.get_temp::<String>(tid));
    let mut draft = match stored {
        Some(s) if focused => s,
        _ => current.to_string(),
    };
    let resp = widgets_skin::ff_text_field_ex(ui, Some(tid), &mut draft, "", width, font);
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
        ui.add(egui::Label::new(RichText::new(&content).color(pal::LABEL)).wrap());
        return;
    }
    let tid = ui.make_persistent_id(("raw", id, entry.ordinal));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let width = (ui.available_width() - 32.0).max(80.0);
        let (_, commit) = draft_field_ex(
            ui,
            tid,
            &content,
            width,
            list,
            (id, entry.line),
            egui::FontId::monospace(14.0),
        );
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

fn ground(idx: usize) -> egui::Color32 {
    if idx.is_multiple_of(2) {
        pal::ROW_EVEN
    } else {
        pal::ROW_ODD
    }
}

/// The ground of a condition or action card: alternating row colour, the left inset puts every
/// control below the header in the control column (the header itself reaches back into it).
/// (`egui::Margin` is `i8`, too small for the 154pt inset, hence two nested frames.)
fn card_frame(idx: usize) -> CardFrame {
    CardFrame(idx)
}

struct CardFrame(usize);

impl CardFrame {
    fn show<R>(
        self,
        ui: &mut egui::Ui,
        add: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        let half = (CARD_LEFT / 2.0) as i8;
        let outer = egui::Frame::new()
            .fill(ground(self.0))
            .inner_margin(egui::Margin {
                left: half,
                right: widgets_skin::ROW_PAD as i8,
                top: 0,
                bottom: 4,
            });
        let r = outer.show(ui, |ui| {
            egui::Frame::NONE
                .inner_margin(egui::Margin {
                    left: half,
                    ..egui::Margin::ZERO
                })
                .show(ui, add)
        });
        egui::InnerResponse::new(r.inner.inner, r.response)
    }
}

/// The ground of a card that is one full-width raw line.
fn raw_card_frame(idx: usize) -> egui::Frame {
    egui::Frame::new()
        .fill(ground(idx))
        .inner_margin(egui::Margin::symmetric(widgets_skin::ROW_PAD as i8, 4))
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
                egui::Stroke::new(2.0, pal::WARN),
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
    let skin = crate::skin::of(ui.ctx());
    widgets_skin::ff_field_row(ui, 0, "Conditions", None, |ui| {
        let btn = widgets_skin::ff_button_ex(ui, &skin, "+ condition", 132.0, ctx.editable, false);
        picked = picker_popup(
            ui,
            &btn,
            &grammar::condition_picker_groups(),
            true,
            &|_| None,
            &|e| present.contains(e),
        );
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
                ui.add(egui::Label::new(RichText::new(line).color(pal::LABEL)).wrap());
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
                raw_card_frame(idx).show(ui, |ui| {
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
                idx,
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
            crate::skin::editor::ff_card(ui, "", |ui| {
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
                        ui.label(RichText::new(label).size(14.0).color(pal::VALUE));
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
            });
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
                idx,
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
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
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
    let choices: Vec<(conditions::NumericOp, String)> =
        conditions::numeric_op_choices_for(card.kw, cur, card.can_between)
            .into_iter()
            .map(|(c, l)| (c, l.to_string()))
            .collect();
    pick_dropdown(
        ui,
        card.salt("op"),
        &cur,
        grammar::operator_label(card.kw, entry.op, false),
        &choices,
        OP_W,
    )
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
    let choices: Vec<(NumericOp, String)> = conditions::numeric_op_choices(kw, NumericOp::Between)
        .into_iter()
        .map(|(c, l)| (c, l.to_string()))
        .collect();
    let picked = pick_dropdown(
        ui,
        (id, kw.name, lower.ordinal, "op"),
        &NumericOp::Between,
        "Between",
        &choices,
        OP_W,
    );
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

/// The `Min` / `Max` label that starts a Between slider line, in a fixed 36 pt slot.
fn bound_label(ui: &mut egui::Ui, text: &str) {
    ui.allocate_ui_with_layout(
        vec2(36.0, 30.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.label(RichText::new(text).font(regular(15.0)).color(pal::MUTED));
        },
    );
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
        bound_label(ui, "Min");
        bound_control(ui, id, kw, lower, lo, lo_clamp, actions);
    });
    ui.horizontal(|ui| {
        bound_label(ui, "Max");
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
                idx,
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

/// Tooltip on a picker entry that is already on the rule.
const ALREADY_ON_RULE: &str = "Already on this rule";

/// Picker sections start closed (UAT3-07); only a search opens the matching ones.
const PICKER_SECTION_DEFAULT_OPEN: bool = false;

/// A forced open state for every picker section: open while searching, otherwise the user's own
/// toggling stands.
fn picker_section_override(searching: bool) -> Option<bool> {
    searching.then_some(true)
}

/// Row height of a picker / class-picker line.
const PICK_ROW_H: f32 = 28.0;
/// The picker list never grows beyond this; it scrolls instead.
const PICKER_MAX_H: f32 = 320.0;

/// A collapsible section head: a small triangle and a small-caps label on a 28pt line.
fn section_head(ui: &mut egui::Ui, label: &str, open: bool) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(vec2(ui.available_width(), PICK_ROW_H), Sense::click());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    if r.hovered() {
        ui.painter().rect_filled(rect, 0.0, pal::ROW_HOVER);
    }
    let c = pos2(rect.min.x + 12.0, rect.center().y);
    let pts = if open {
        vec![
            c + vec2(-4.5, -2.5),
            c + vec2(4.5, -2.5),
            c + vec2(0.0, 3.5),
        ]
    } else {
        vec![
            c + vec2(-2.5, -4.5),
            c + vec2(3.5, 0.0),
            c + vec2(-2.5, 4.5),
        ]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        pts,
        pal::LABEL,
        egui::Stroke::NONE,
    ));
    ui.painter().text(
        pos2(rect.min.x + 26.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        small_caps(17.0),
        pal::LABEL,
    );
    r
}

/// One full-width 28pt picker entry; greyed entries use the disabled colour. `clickable` is false
/// for entries that cannot be picked at all.
fn pick_row(ui: &mut egui::Ui, text: &str, greyed: bool, clickable: bool) -> egui::Response {
    let sense = if clickable {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, r) = ui.allocate_exact_size(vec2(ui.available_width(), PICK_ROW_H), sense);
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, clickable, text));
    if clickable && r.hovered() {
        ui.painter().rect_filled(rect, 0.0, pal::ROW_HOVER);
    }
    let color = if greyed { pal::DISABLED } else { pal::VALUE };
    ui.painter().text(
        pos2(rect.min.x + 26.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        regular(15.0),
        color,
    );
    r
}

/// A searchable, grouped picker anchored to `button` (the `+ condition` picker). Returns the entry
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
        .gap(2.0)
        .frame(egui::Frame::NONE.inner_margin(6))
        .show(|ui| {
            let skin = crate::skin::of(ui.ctx());
            let bg = ui.painter().add(egui::Shape::Noop);
            ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
            let mut query = ui
                .data_mut(|d| d.get_temp::<String>(query_id))
                .unwrap_or_default();
            if searchable {
                let w = ui.available_width();
                let r = widgets_skin::ff_search_field(
                    ui,
                    &skin,
                    popup_id.with("query-edit"),
                    &mut query,
                    hint_text,
                    w,
                );
                if !ui.data(|d| d.get_temp::<bool>(focus_id)).unwrap_or(false) {
                    r.request_focus();
                    ui.data_mut(|d| d.insert_temp(focus_id, true));
                }
            }
            ui.data_mut(|d| d.insert_temp(query_id, query.clone()));
            let filtered = filter_groups(groups, &query);
            let searching = !query.trim().is_empty();
            let forced = picker_section_override(searching);
            let open_id = |label: &str| popup_id.with(("section-open", label.to_string()));
            let is_open = |ui: &egui::Ui, label: &str| {
                forced.unwrap_or_else(|| {
                    ui.data(|d| d.get_temp::<bool>(open_id(label)))
                        .unwrap_or(PICKER_SECTION_DEFAULT_OPEN)
                })
            };
            let rows: usize = if filtered.is_empty() {
                1
            } else {
                filtered
                    .iter()
                    .map(|(l, es)| 1 + if is_open(ui, l) { es.len() } else { 0 })
                    .sum()
            };
            let list_h = (rows as f32 * PICK_ROW_H).min(PICKER_MAX_H);
            let mut picked: Option<ConditionEntry> = None;
            ui.allocate_ui(vec2(ui.available_width(), list_h), |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                widgets_skin::ff_scroll_area(ui, &skin, (popup_id, "list"), |ui| {
                    if filtered.is_empty() {
                        let (r, _) = ui.allocate_exact_size(
                            vec2(ui.available_width(), PICK_ROW_H),
                            Sense::hover(),
                        );
                        ui.painter().text(
                            pos2(r.min.x + 8.0, r.center().y),
                            Align2::LEFT_CENTER,
                            "Nothing matches",
                            regular(15.0),
                            pal::MUTED,
                        );
                    }
                    for (label, kws) in filtered.iter() {
                        let open = is_open(ui, label);
                        if section_head(ui, label, open).clicked() && forced.is_none() {
                            ui.data_mut(|d| d.insert_temp(open_id(label), !open));
                            ui.ctx().request_repaint();
                        }
                        if !open {
                            continue;
                        }
                        for entry in kws {
                            let reason = disabled(entry);
                            let already = reason.is_none() && present(entry);
                            let greyed = reason.is_some() || already;
                            let r = pick_row(ui, &entry.label(), greyed, reason.is_none());
                            match reason {
                                Some(msg) => {
                                    r.on_hover_text(msg);
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
                    }
                });
            });
            if picked.is_some() {
                egui::Popup::close_id(ui.ctx(), popup_id);
            }
            crate::skin::popup::paint_box_behind(ui, &skin, bg);
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
                idx,
                &grammar::display_name(p.keyword.name),
                p.keyword.name,
                |ui| {
                    // The operator the first entry will be written with is a dropdown from the
                    // start, so a carried-over negation is never silent (WR-01, iteration 2) and
                    // the author can pick it before any value exists.
                    let choices = grammar::operator_choices(p.keyword, p.op, false);
                    if !choices.is_empty() {
                        let opts: Vec<(Option<Operator>, String)> = choices
                            .iter()
                            .map(|c| (c.op, c.label.to_string()))
                            .collect();
                        if let Some(op) = pick_dropdown(
                            ui,
                            (p.block, p.keyword.name, "pending-op"),
                            &p.op,
                            grammar::operator_label(p.keyword, p.op, false),
                            &opts,
                            OP_W,
                        ) {
                            picked_op = Some(op);
                        }
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
                idx,
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

/// A two-column card header: `label_text` in the 128pt label column, `add_control` from the label
/// end + 16 to the right edge. The line reaches back over the card's left inset so its label sits
/// at the row's left edge; the cursor advances by its height only.
fn card_header<R>(
    ui: &mut egui::Ui,
    idx: usize,
    label_text: &str,
    label_hover: &str,
    add_control: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let avail = ui.available_rect_before_wrap();
    let outer = egui::Rect::from_min_max(
        egui::pos2(avail.min.x - CARD_LEFT, avail.min.y),
        egui::pos2(avail.max.x + widgets_skin::ROW_PAD, avail.max.y),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(outer)
            .layout(Layout::top_down(Align::Min)),
    );
    let inner =
        widgets_skin::ff_field_row(&mut child, idx, label_text, Some(label_hover), add_control)
            .inner;
    ui.allocate_space(vec2(0.0, child.min_rect().height()));
    inner
}

/// One condition header row: the keyword label column, the `middle` control, and the remove X
/// flush against the right edge so the X buttons line up across rows. Returns the X response.
fn condition_header(
    ui: &mut egui::Ui,
    idx: usize,
    label_text: &str,
    label_hover: &str,
    middle: impl FnOnce(&mut egui::Ui),
    remove_tip: &str,
) -> egui::Response {
    card_header(ui, idx, label_text, label_hover, |ui| {
        middle(ui);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            widgets::remove_x_button(ui, remove_tip)
        })
        .inner
    })
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
    let skin = crate::skin::of(ui.ctx());
    let mut idx = if effective { 0 } else { 1 };
    let r = crate::skin::editor::ff_segmented(ui, &skin, &mut idx, &["Yes", "No"], 132.0)
        .on_hover_text("Remove the condition to match either.");
    (r.changed() && (idx == 0) != effective).then_some(idx == 0)
}

/// The operator dropdown; `Some(new)` only when the user picked a different operator.
fn operator_combo(ui: &mut egui::Ui, card: &Card, list_form: bool) -> Option<Option<Operator>> {
    let entry = card.entry;
    let choices = grammar::operator_choices(card.kw, entry.op, list_form);
    let current = grammar::operator_label(card.kw, entry.op, list_form);
    let opts: Vec<(Option<Operator>, String)> = choices
        .iter()
        .map(|c| (c.op, c.label.to_string()))
        .collect();
    pick_dropdown(ui, card.salt("op"), &entry.op, current, &opts, OP_W)
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
        let opts: Vec<(String, String)> = options
            .iter()
            .map(|o| (o.to_string(), o.to_string()))
            .collect();
        if let Some(p) = pick_dropdown(ui, card.salt("enum"), only, only.as_str(), &opts, 160.0) {
            let res = values_edit(card.id, card.entry, &Value::Enum(vec![p]));
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
                .size(crate::skin::fonts::SUMMARY_SIZE)
                .color(pal::LABEL),
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
        "Type a base type, press Enter"
    } else if kw.name.contains("Mod") {
        "Type a mod, press Enter"
    } else {
        "Type a name, press Enter"
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
        if section_head(ui, &format!("{label} ({n})"), open).clicked() {
            ui.data_mut(|d| d.insert_temp(open_id, !open));
        }
        if open {
            for c in classes {
                let mut on = selected.contains(c);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let toggled = widgets::circular_toggle(ui, &mut on).changed();
                    ui.label(
                        RichText::new(c.as_str())
                            .font(regular(15.0))
                            .color(pal::VALUE),
                    );
                    if toggled {
                        result = Some(class_toggle(selected, c, on));
                    }
                });
            }
        }
    }
    let unknown = unknown_classes(selected, catalog);
    if !unknown.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for c in unknown {
                ui.label(
                    RichText::new(c.as_str())
                        .font(regular(15.0))
                        .color(pal::MUTED),
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
        "Type a whole number, press Enter",
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
            ui.label(RichText::new(lead).size(14.0).color(pal::VALUE));
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
            ui.label(RichText::new("of these:").size(14.0).color(pal::VALUE));
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
    let skin = crate::skin::of(ui.ctx());
    let mut sel = if named {
        1
    } else if matches!(current, BoolOrName::Bool(true)) {
        0
    } else {
        2
    };
    let changed =
        crate::skin::editor::ff_segmented(ui, &skin, &mut sel, &["Any", "Named"], 132.0).changed();
    let (any_changed, nm_changed) = (changed && sel == 0, changed && sel == 1);
    let (any, nm) = (sel == 0, sel == 1);
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

/// Colour edit. `had_alpha` is the line's form when the picker opened (4 tokens), not its
/// current form: a 3-token line keeps its 3-token form while the alpha stays at the keyword's
/// bare alpha (240 for `SetBackgroundColor`, else 255), even after a live alpha edit wrote 4
/// tokens, so Escape restores the original bytes.
pub(crate) fn rgba_edit(
    id: BlockId,
    entry: &BodyEntry,
    new: [u8; 4],
    had_alpha: bool,
) -> Option<RowAction> {
    values_edit(
        id,
        entry,
        &Value::Rgba {
            rgba: new,
            had_alpha,
            bare_alpha: grammar::bare_alpha(entry.keyword.map_or("", |k| k.name)),
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

/// The hint under a font size outside the range the game renders readably.
pub(crate) fn font_hint(size: i64) -> Option<&'static str> {
    (!(18..=45).contains(&size)).then_some("Readable in game: 18-45")
}

// ---------------------------------------------------------------------------------------------
// The Custom STYLE lines: the rule backend of `style_lines`
// ---------------------------------------------------------------------------------------------
//
// A Custom rule's STYLE section draws the same lines as the CONFIG style editor
// (`style_lines::show`), over the rule's own action lines, losslessly:
//
// * Binding. A later action line of the same kind overrides an earlier one in game, so each style
//   line is bound to the LAST action line of its keyword family, the one the game uses: one
//   family per colour keyword (`SetTextColor`, `SetBorderColor`, `SetBackgroundColor`),
//   `SetFontSize`, `MinimapIcon`, `PlayEffect`, Sound (`PlayAlertSound`,
//   `PlayAlertSoundPositional`, `CustomAlertSound`, `CustomAlertSoundOptional`: one alert sound
//   per item) and Drop sound (`DisableDropSound`, `EnableDropSound`).
// * Exact or not. The bound line is shown exactly when the editor can represent it: a decoded
//   colour; a font size within 1-45; `MinimapIcon` with size 0-2 and a known colour and shape,
//   or the bare cancel value `MinimapIcon -1`; `PlayEffect` with a known colour or `None`;
//   `PlayAlertSound` / `PlayAlertSoundPositional` 1-16 or `CustomAlertSound` with a volume absent
//   or within 0-300, or the bare `PlayAlertSound None`; `DisableDropSound` / `EnableDropSound`
//   (bare or `True`). Otherwise the line shows the nearest honest state (the cancel value for
//   `MinimapIcon -1 <colour> <shape>`, `PlayAlertSound None <volume>` or
//   `PlayAlertSoundPositional None`; off for an unreadable line or another named sound; a font
//   size or volume clamped into the slider's range; an optional sound as its plain kind; a
//   `... False` drop-sound line as the opposite state) and is ALSO listed as a leftover row, so
//   its exact text stays visible. Both places edit the same line.
// * Leftovers. Every other action line (`DisableDropSoundIfAlertSound`, and every earlier line of
//   a family, which the game overrides) is kept as written and shown under the lines as the usual
//   action card (or a read-only raw row).
// * Edits. An intent rewrites only its bound line (`EditLine` with the line's text as `expect`),
//   inserts one line after the last action line only when the family has no line at all (so a
//   family never gets a second line), or removes the bound line when it is cleared (an earlier
//   line of the family then shows through, as it would in game). A 3-value colour stays 3-value
//   while its alpha is 255; a volume the line did not write is only written when it is changed;
//   a sound kind switch rewrites the keyword in place and keeps a trailing comment. Rendering
//   with no input emits nothing, so an untouched rule is byte-identical on Apply.

/// The action line each style line edits (`None` = the slot is absent).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StyleBinding {
    pub keyword: BlockKeyword,
    pub text: Option<BodyEntry>,
    pub border: Option<BodyEntry>,
    pub background: Option<BodyEntry>,
    pub font: Option<BodyEntry>,
    pub minimap: Option<BodyEntry>,
    pub beam: Option<BodyEntry>,
    pub sound: Option<BodyEntry>,
    pub drop: Option<BodyEntry>,
    /// The bound sound line's written volume (`None` when the line has none).
    pub sound_volume: Option<i64>,
    /// The volume the Volume line shows (the written one clamped to 0-300, or the game default
    /// for the kind).
    pub sound_shown: u16,
    /// Bound lines the editor cannot show exactly (also listed as leftover rows).
    pub inexact: Vec<usize>,
}

impl StyleBinding {
    fn slots(&self) -> [&Option<BodyEntry>; 8] {
        [
            &self.text,
            &self.border,
            &self.background,
            &self.font,
            &self.minimap,
            &self.beam,
            &self.sound,
            &self.drop,
        ]
    }

    /// Whether document line `line` is bound to a style line.
    pub(crate) fn binds(&self, line: usize) -> bool {
        self.slots()
            .iter()
            .any(|s| s.as_ref().is_some_and(|e| e.line == line))
    }

    /// Whether document line `line` is bound and shown exactly by its style line.
    fn shows_exactly(&self, line: usize) -> bool {
        self.binds(line) && !self.inexact.contains(&line)
    }

    fn colour(&self, slot: ColourSlot) -> &Option<BodyEntry> {
        match slot {
            ColourSlot::Text => &self.text,
            ColourSlot::Border => &self.border,
            ColourSlot::Background => &self.background,
        }
    }
}

/// Game default volumes of a sound line written without one.
const ALERT_DEFAULT_VOLUME: u16 = 50;
const CUSTOM_DEFAULT_VOLUME: u16 = 100;

fn volume_ok(volume: Option<i64>) -> bool {
    volume.is_none_or(|v| (grammar::VOLUME_RANGE.0..=grammar::VOLUME_RANGE.1).contains(&v))
}

/// A written volume clamped into the Volume slider's range.
fn volume_shown(volume: i64) -> u16 {
    volume.clamp(grammar::VOLUME_RANGE.0, grammar::VOLUME_RANGE.1) as u16
}

/// The style line (slot) whose keyword family `name` belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Colour(ColourSlot),
    Font,
    Minimap,
    Beam,
    Sound,
    Drop,
}

fn family_of(name: &str) -> Option<Family> {
    Some(match name {
        "SetTextColor" => Family::Colour(ColourSlot::Text),
        "SetBorderColor" => Family::Colour(ColourSlot::Border),
        "SetBackgroundColor" => Family::Colour(ColourSlot::Background),
        "SetFontSize" => Family::Font,
        "MinimapIcon" => Family::Minimap,
        "PlayEffect" => Family::Beam,
        "PlayAlertSound"
        | "PlayAlertSoundPositional"
        | "CustomAlertSound"
        | "CustomAlertSoundOptional" => Family::Sound,
        "DisableDropSound" | "EnableDropSound" => Family::Drop,
        _ => return None,
    })
}

fn decoded(e: &BodyEntry) -> Option<(&'static str, Value)> {
    let kw = e.keyword?;
    let tokens: Vec<&str> = e.values.iter().map(String::as_str).collect();
    Some((kw.name, grammar::decode(kw, &tokens)))
}

/// The colour and shape written after the `-1` of a `MinimapIcon -1 <colour> <shape>` line
/// (`tokens` starts at `-1`), as a Large icon; `None` when they are absent or not valid names.
fn cancelled_minimap_hint(tokens: &[String]) -> Option<mc::Minimap> {
    let [_, colour, shape] = tokens else {
        return None;
    };
    Some(mc::Minimap {
        size: 0,
        color: FilterColor::from_name(colour)?,
        shape: MinimapShape::from_name(shape)?,
    })
}

/// The style view of a rule's action lines and the line each style line is bound to (the last
/// line of its family, see the binding rule above).
pub(crate) fn rule_style_view(
    entries: &[BodyEntry],
    keyword: BlockKeyword,
) -> (StyleView, StyleBinding) {
    let mut view = StyleView {
        visibility: match keyword {
            BlockKeyword::Show => Vis::Show,
            BlockKeyword::Hide => Vis::Hide,
            BlockKeyword::Minimal => Vis::Minimal,
        },
        text: None,
        border: None,
        background: None,
        font: None,
        minimap: None,
        minimap_hint: None,
        beam: None,
        sound: None,
        sound_volume_written: false,
        drop_sound: None,
    };
    let mut b = StyleBinding {
        keyword,
        text: None,
        border: None,
        background: None,
        font: None,
        minimap: None,
        beam: None,
        sound: None,
        drop: None,
        sound_volume: None,
        sound_shown: ALERT_DEFAULT_VOLUME,
        inexact: Vec::new(),
    };
    // The last line of each family wins.
    for e in entries.iter().filter(|e| e.kind == EntryKind::Action) {
        let Some(family) = e.keyword.and_then(|k| family_of(k.name)) else {
            continue;
        };
        let slot = match family {
            Family::Colour(ColourSlot::Text) => &mut b.text,
            Family::Colour(ColourSlot::Border) => &mut b.border,
            Family::Colour(ColourSlot::Background) => &mut b.background,
            Family::Font => &mut b.font,
            Family::Minimap => &mut b.minimap,
            Family::Beam => &mut b.beam,
            Family::Sound => &mut b.sound,
            Family::Drop => &mut b.drop,
        };
        *slot = Some(e.clone());
    }
    let mut inexact = Vec::new();
    for (slot, shown) in [
        (&b.text, &mut view.text),
        (&b.border, &mut view.border),
        (&b.background, &mut view.background),
    ] {
        if let Some(e) = slot {
            match decoded(e) {
                Some((
                    _,
                    Value::Rgba {
                        rgba, had_alpha, ..
                    },
                )) => {
                    *shown = Some(Rgba { rgba, had_alpha });
                }
                _ => inexact.push(e.line),
            }
        }
    }
    if let Some(e) = &b.font {
        let (lo, hi) = grammar::FONT_SIZE_RANGE;
        match decoded(e) {
            Some((_, Value::FontSize(n))) => {
                view.font = Some(n.clamp(lo, hi));
                if !(lo..=hi).contains(&n) {
                    inexact.push(e.line);
                }
            }
            _ => inexact.push(e.line),
        }
    }
    if let Some(e) = &b.minimap {
        let (shown, exact) = match decoded(e) {
            Some((
                _,
                Value::Minimap(Minimap::Icon {
                    size,
                    colour,
                    shape,
                }),
            )) => match (
                u8::try_from(size),
                FilterColor::from_name(&colour),
                MinimapShape::from_name(&shape),
            ) {
                (Ok(size @ 0..=2), Some(color), Some(shape)) => (
                    Some(Cancelable::Set(mc::Minimap { size, color, shape })),
                    true,
                ),
                _ => (None, false),
            },
            // `-1` cancels the icon; trailing colour/shape tokens are kept in the file and only
            // seed what the pickers show, so leaving `/` brings them back (review IN-04).
            Some((_, Value::Minimap(Minimap::Hidden(t)))) => {
                view.minimap_hint = cancelled_minimap_hint(&t);
                (Some(Cancelable::Cancel), t.len() == 1)
            }
            _ => (None, false),
        };
        if !exact {
            inexact.push(e.line);
        }
        view.minimap = shown;
    }
    if let Some(e) = &b.beam {
        let shown = match decoded(e) {
            Some((_, Value::Effect(Effect::Colour { colour, temp }))) => {
                FilterColor::from_name(&colour).map(|color| Cancelable::Set(Beam { color, temp }))
            }
            Some((_, Value::Effect(Effect::None))) => Some(Cancelable::Cancel),
            _ => None,
        };
        if shown.is_none() {
            inexact.push(e.line);
        }
        view.beam = shown;
    }
    if let Some(e) = &b.sound {
        let mut exact = false;
        match decoded(e) {
            Some((
                name,
                Value::Sound {
                    id: SoundRef::Id(k),
                    volume,
                },
            )) => {
                if let Ok(k @ 1..=16) = u8::try_from(k) {
                    let vol = volume.map_or(ALERT_DEFAULT_VOLUME, volume_shown);
                    b.sound_volume = volume;
                    b.sound_shown = vol;
                    view.sound = Some(Cancelable::Set(Sound {
                        id: Some(k),
                        custom: None,
                        vol,
                        positional: name == "PlayAlertSoundPositional",
                    }));
                    view.sound_volume_written = volume.is_some();
                    exact = volume_ok(volume);
                }
            }
            Some((
                name,
                Value::Sound {
                    id: SoundRef::Named(n),
                    volume,
                },
            )) if n == "None" => {
                view.sound = Some(Cancelable::Cancel);
                exact = name == "PlayAlertSound" && volume.is_none();
            }
            Some((name, Value::CustomSound { file, volume })) => {
                let vol = volume.map_or(CUSTOM_DEFAULT_VOLUME, volume_shown);
                b.sound_volume = volume;
                b.sound_shown = vol;
                view.sound = Some(Cancelable::Set(Sound {
                    id: None,
                    custom: Some(file),
                    vol,
                    positional: false,
                }));
                view.sound_volume_written = volume.is_some();
                exact = name == "CustomAlertSound" && volume_ok(volume);
            }
            _ => {}
        }
        if !exact {
            inexact.push(e.line);
        }
    }
    if let Some(e) = &b.drop {
        match drop_state(e) {
            Some((state, exact)) => {
                view.drop_sound = Some(state);
                if !exact {
                    inexact.push(e.line);
                }
            }
            None => inexact.push(e.line),
        }
    }
    b.inexact = inexact;
    (view, b)
}

/// What a drop-sound line means and whether it is the bare (or `True`) form the editor writes:
/// `DisableDropSound` is Silent, `EnableDropSound` Normal, and a `False` value the opposite.
fn drop_state(e: &BodyEntry) -> Option<(DropSound, bool)> {
    match decoded(e) {
        Some(("DisableDropSound", Value::OptionalBool(on))) => Some(if on == Some(false) {
            (DropSound::Normal, false)
        } else {
            (DropSound::Silent, true)
        }),
        Some(("EnableDropSound", Value::OptionalBool(on))) => Some(if on == Some(false) {
            (DropSound::Silent, false)
        } else {
            (DropSound::Normal, true)
        }),
        _ => None,
    }
}

/// The action lines no style line shows exactly, in file order: the earlier lines of a family,
/// the actions outside every family, and a bound line the editor cannot show exactly.
pub(crate) fn style_leftovers<'a>(
    entries: &'a [BodyEntry],
    binding: &StyleBinding,
) -> Vec<&'a BodyEntry> {
    entries
        .iter()
        .filter(|e| e.kind == EntryKind::Action && !binding.shows_exactly(e.line))
        .collect()
}

fn add_action(id: BlockId, text: String) -> RowAction {
    RowAction::AddLine {
        id,
        kind: LineKind::Action,
        text,
    }
}

fn colour_keyword(slot: ColourSlot) -> &'static str {
    match slot {
        ColourSlot::Text => "SetTextColor",
        ColourSlot::Border => "SetBorderColor",
        ColourSlot::Background => "SetBackgroundColor",
    }
}

fn icon_of(m: &mc::Minimap) -> Minimap {
    Minimap::Icon {
        size: i64::from(m.size),
        colour: m.color.as_str().to_string(),
        shape: m.shape.as_str().to_string(),
    }
}

fn effect_of(b: &Beam) -> Effect {
    Effect::Colour {
        colour: b.color.as_str().to_string(),
        temp: b.temp,
    }
}

/// ` <v>` when a volume is written, else nothing.
fn volume_suffix(volume: Option<i64>) -> String {
    volume.map(|v| format!(" {v}")).unwrap_or_default()
}

/// The minimal line edits one style-line intent stands for (see the binding rule above).
pub(crate) fn rule_intent_actions(
    id: BlockId,
    binding: &StyleBinding,
    intent: StyleIntent,
) -> Vec<RowAction> {
    let one = |a: Option<RowAction>| a.into_iter().collect::<Vec<_>>();
    match intent {
        StyleIntent::Visibility(v) => {
            let kw = match v {
                Vis::Hide => BlockKeyword::Hide,
                Vis::Show | Vis::Minimal => BlockKeyword::Show,
            };
            one((kw != binding.keyword).then_some(RowAction::SetKeyword(id, kw)))
        }
        StyleIntent::Colour {
            slot,
            value,
            opened_had_alpha,
        } => one(match (binding.colour(slot), value) {
            (Some(e), Some(c)) => rgba_edit(id, e, c, opened_had_alpha),
            (Some(e), None) => Some(remove(id, e)),
            (None, Some([r, g, b, a])) => Some(add_action(
                id,
                format!("{} {r} {g} {b} {a}", colour_keyword(slot)),
            )),
            (None, None) => None,
        }),
        StyleIntent::Font(f) => one(match (&binding.font, f) {
            (Some(e), Some(n)) => font_edit(id, e, n),
            (Some(e), None) => Some(remove(id, e)),
            (None, Some(n)) => Some(add_action(id, format!("SetFontSize {n}"))),
            (None, None) => None,
        }),
        StyleIntent::Minimap(m) => one(match (&binding.minimap, m) {
            (Some(e), Some(Cancelable::Set(m))) => minimap_edit(id, e, icon_of(&m)),
            (Some(e), Some(Cancelable::Cancel)) => {
                minimap_edit(id, e, Minimap::Hidden(vec!["-1".into()]))
            }
            (Some(e), None) => Some(remove(id, e)),
            (None, Some(Cancelable::Set(m))) => Some(add_action(
                id,
                format!(
                    "MinimapIcon {}",
                    grammar::encode_tokens(&Value::Minimap(icon_of(&m))).join(" ")
                ),
            )),
            (None, Some(Cancelable::Cancel)) => Some(add_action(id, "MinimapIcon -1".into())),
            (None, None) => None,
        }),
        StyleIntent::Beam(b) => one(match (&binding.beam, b) {
            (Some(e), Some(Cancelable::Set(b))) => effect_edit(id, e, effect_of(&b)),
            (Some(e), Some(Cancelable::Cancel)) => effect_edit(id, e, Effect::None),
            (Some(e), None) => Some(remove(id, e)),
            (None, Some(Cancelable::Set(b))) => Some(add_action(
                id,
                format!(
                    "PlayEffect {}",
                    grammar::encode_tokens(&Value::Effect(effect_of(&b))).join(" ")
                ),
            )),
            (None, Some(Cancelable::Cancel)) => Some(add_action(id, "PlayEffect None".into())),
            (None, None) => None,
        }),
        StyleIntent::Sound(s) => one(sound_intent(id, binding, s)),
        StyleIntent::DropSound(d) => one(drop_sound_intent(id, binding, d)),
    }
}

/// The drop sound switch. The bound line is the last `DisableDropSound` / `EnableDropSound`
/// line: it is kept when it already means the requested state, else rewritten in place to the
/// bare keyword (never followed by a contradicting line); off removes it.
fn drop_sound_intent(
    id: BlockId,
    binding: &StyleBinding,
    want: Option<DropSound>,
) -> Option<RowAction> {
    let keyword = |d: DropSound| match d {
        DropSound::Silent => "DisableDropSound",
        DropSound::Normal => "EnableDropSound",
    };
    match (&binding.drop, want) {
        (Some(e), None) => Some(remove(id, e)),
        (None, None) => None,
        (None, Some(d)) => Some(add_action(id, keyword(d).to_string())),
        (Some(e), Some(d)) => (drop_state(e).map(|s| s.0) != Some(d))
            .then(|| edit(id, e, LinePatch::Content(keyword(d).to_string()))),
    }
}

fn alert_keyword(positional: bool) -> &'static str {
    if positional {
        "PlayAlertSoundPositional"
    } else {
        "PlayAlertSound"
    }
}

fn sound_intent(
    id: BlockId,
    binding: &StyleBinding,
    sound: Option<Cancelable<Sound>>,
) -> Option<RowAction> {
    let s = match sound {
        None => return binding.sound.as_ref().map(|e| remove(id, e)),
        Some(Cancelable::Cancel) => {
            return match &binding.sound {
                None => Some(add_action(id, "PlayAlertSound None".to_string())),
                Some(e) => match decoded(e) {
                    Some((
                        _,
                        Value::Sound {
                            id: SoundRef::Named(n),
                            ..
                        },
                    )) if n == "None" => None,
                    _ => Some(edit(
                        id,
                        e,
                        LinePatch::Content("PlayAlertSound None".to_string()),
                    )),
                },
            };
        }
        Some(Cancelable::Set(s)) => s,
    };
    let file = s.custom.as_deref().map(str::trim);
    if file.is_some_and(|f| f.is_empty() || !style_lines::safe_file_name(f)) {
        return None;
    }
    // A file has no positional form.
    let positional = s.positional && file.is_none();
    let alert = i64::from(s.id.unwrap_or(1));
    let Some(e) = &binding.sound else {
        let text = match file {
            Some(f) => format!("CustomAlertSound \"{f}\" {}", s.vol),
            None => format!("{} {alert} {}", alert_keyword(positional), s.vol),
        };
        return Some(add_action(id, text));
    };
    let bound = decoded(e);
    let bound_alert = matches!(
        bound,
        Some((
            _,
            Value::Sound {
                id: SoundRef::Id(_),
                ..
            }
        ))
    );
    let bound_custom = matches!(bound, Some((_, Value::CustomSound { .. })));
    if !bound_alert && !bound_custom {
        // Leaving the cancel value (or an unreadable line): the full line, volume written.
        let text = match file {
            Some(f) => format!("CustomAlertSound \"{f}\" {}", s.vol),
            None => format!("{} {alert} {}", alert_keyword(positional), s.vol),
        };
        return Some(edit(id, e, LinePatch::Content(text)));
    }
    // A volume is written only once it is changed: a line without one keeps none, and a volume
    // the slider shows clamped (outside 0-300) is kept as written until the slider moves. A switch
    // between an alert and a file changes the game default (50 / 100), so a line without a volume
    // gets the shown one written unless it is the new kind's default: the loudness stays.
    let kind_switch = bound_custom != file.is_some();
    let new_default = if file.is_some() {
        CUSTOM_DEFAULT_VOLUME
    } else {
        ALERT_DEFAULT_VOLUME
    };
    let volume = if s.vol != binding.sound_shown || (kind_switch && s.vol != new_default) {
        Some(i64::from(s.vol))
    } else {
        binding.sound_volume
    };
    let bound_positional = e
        .keyword
        .is_some_and(|k| k.name == "PlayAlertSoundPositional");
    // `CustomAlertSoundOptional` keeps its keyword while the kind stays a file; a kind or
    // positional switch rewrites the bound line's keyword and values in place, keeping a trailing
    // comment.
    match (bound_custom, file) {
        (false, None) if bound_positional == positional => {
            sound_edit(id, e, SoundRef::Id(alert), volume)
        }
        (true, Some(f)) => custom_sound_edit(id, e, f, volume).ok().flatten(),
        (false, Some(f)) => Some(edit(
            id,
            e,
            LinePatch::Content(format!("CustomAlertSound \"{f}\"{}", volume_suffix(volume))),
        )),
        (_, None) => Some(edit(
            id,
            e,
            LinePatch::Content(format!(
                "{} {alert}{}",
                alert_keyword(positional),
                volume_suffix(volume)
            )),
        )),
    }
}

/// The Custom STYLE lines plus the action lines they cannot represent.
fn custom_style_lines(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
    row_idx: &mut usize,
) {
    let id = ctx.row.id;
    let (view, binding) = rule_style_view(ctx.entries, ctx.row.keyword);
    // A line the editor just added is already in view on its style line.
    if let Some((b, line)) = list.focus_line
        && b == id
        && binding.binds(line)
    {
        list.focus_line = None;
    }
    let memo = egui::Id::new(("rule-style-last", id));
    // A scope independent of the row's card epoch, so a picker stays open when its first change
    // adds the slot's line (adding a line resets the per-card state).
    let intents = ui
        .scope_builder(
            egui::UiBuilder::new().id(egui::Id::new(("rule-style-lines", id))),
            |ui| {
                ui.add_enabled_ui(ctx.editable, |ui| {
                    style_lines::show(ui, &view, memo, row_idx)
                })
                .inner
            },
        )
        .inner;
    if ctx.editable {
        for intent in intents {
            actions.extend(rule_intent_actions(id, &binding, intent));
        }
    }
    for (idx, e) in style_leftovers(ctx.entries, &binding)
        .into_iter()
        .enumerate()
    {
        style_lines::probe("ed-leftover", egui::Rect::NOTHING);
        match e.keyword {
            Some(kw) if ctx.editable => {
                action_card(ui, ctx, e, kw, idx, None, list, actions);
            }
            _ => {
                raw_card_frame(idx).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    raw_row(ui, e, false, list, actions, id);
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The ACTIONS group
// ---------------------------------------------------------------------------------------------

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
    let (colour, tip) = match state {
        StyleState::None => (None, None),
        StyleState::Known(_) => (None, Some(STYLE_KNOWN_TIP)),
        StyleState::Unknown(_) => (Some(pal::WARN), Some(STYLE_UNKNOWN_TIP)),
        StyleState::Unchecked(_) => (Some(pal::WARN), Some(STYLE_UNCHECKED_TIP)),
    };
    let names = ctx.working.style_names();
    let mut labels: Vec<&str> = vec!["Custom"];
    labels.extend(names.iter().map(String::as_str));
    let start = match tag {
        None => 0,
        Some(t) => match names.iter().position(|n| n == t) {
            Some(i) => i + 1,
            None => {
                labels.push(t);
                labels.len() - 1
            }
        },
    };
    let mut sel = start;
    let skin = crate::skin::of(ui.ctx());
    let r = widgets_skin::ff_dropdown_ex(
        ui,
        &skin,
        (id, "style-pick"),
        &mut sel,
        &labels,
        180.0,
        colour,
    );
    let r = if let Some(t) = tip {
        r.on_hover_text(t)
    } else {
        r
    };
    if r.changed() && sel != start {
        if sel == 0 {
            return Some(StylePick::Custom);
        }
        return names.get(sel - 1).map(|n| StylePick::Preset(n.clone()));
    }
    None
}

/// The STYLE section, last in the rule body: the Custom/preset picker, then (Custom only) the
/// style editor's lines and the action lines they cannot represent, then `Continue` on its own
/// last line.
pub fn style_section(
    ui: &mut egui::Ui,
    ctx: &BodyCtx,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let id = ctx.row.id;
    let show_rows = shows_action_rows(&ctx.row.style);
    let skin = crate::skin::of(ui.ctx());
    let mut row_idx = 0;
    widgets_skin::ff_field_row(ui, row_idx, "Style", None, |ui| {
        ui.add_enabled_ui(ctx.editable, |ui| {
            if let Some(p) = style_picker(ui, ctx)
                && let Some(a) = style_pick_action(id, &ctx.row.style, p)
            {
                actions.push(a);
            }
        });
    });
    row_idx += 1;
    ui.spacing_mut().item_spacing.y = 0.0;
    if show_rows {
        custom_style_lines(ui, ctx, list, actions, &mut row_idx);
    }
    widgets_skin::ff_field_row(ui, row_idx, "", None, |ui| {
        ui.add_enabled_ui(ctx.editable, |ui| {
            // A style never owns `Continue`, so the check stays editable on a styled rule.
            let mut cont = ctx.row.has_continue;
            let tip = "Matching goes on to the rules below";
            let changed = widgets_skin::ff_check(ui, &skin, &mut cont, "Continue to later rules")
                .on_hover_text(tip)
                .changed();
            ui.label(
                RichText::new("Continue to later rules")
                    .font(crate::skin::fonts::small_caps(17.0))
                    .color(pal::LABEL),
            )
            .on_hover_text(tip);
            if changed {
                actions.push(RowAction::SetContinue(id, cont));
            }
        });
    });
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
            card_header(ui, idx, &grammar::display_name(kw.name), kw.name, |ui| {
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
                        let x = ui
                            .with_layout(Layout::right_to_left(Align::Center), |ui| {
                                widgets::remove_x_button(ui, "Remove this action")
                            })
                            .inner;
                        if x.clicked() {
                            actions.push(remove(id, entry));
                        }
                    }
                }
            });
            if lock.is_some() {
                ui.label(
                    RichText::new(current_values(entry))
                        .size(14.0)
                        .color(pal::DISABLED),
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
        Value::Rgba {
            rgba, had_alpha, ..
        } => {
            if let Some((n, opened_had_alpha)) = widgets::colour_swatch(ui, *rgba, *had_alpha)
                && let Some(a) = rgba_edit(card.id, card.entry, n, opened_had_alpha)
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
                .size(crate::skin::fonts::SUMMARY_SIZE)
                .color(pal::HEADING),
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
    dot(ui, filter_colour(current));
    let mut names: Vec<String> = COLOURS.iter().map(|c| c.to_string()).collect();
    if allow_none {
        names.push("None".to_string());
    }
    let opts: Vec<(String, String)> = names.into_iter().map(|n| (n.clone(), n)).collect();
    pick_dropdown(ui, salt, &current.to_string(), current, &opts, 120.0)
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
        let size_opts: Vec<(i64, String)> = SIZE_CHOICES
            .iter()
            .map(|(l, v)| (*v, (*l).to_string()))
            .collect();
        if let Some(v) = pick_dropdown(ui, card.salt("msize"), &size, &label, &size_opts, 120.0) {
            next = Some(if v == -1 {
                Minimap::Hidden(vec!["-1".to_string()])
            } else {
                icon(v, &colour, &shape)
            });
        }
        if hidden {
            return;
        }
        if let Some(c) = colour_combo(ui, card.salt("mcol"), &colour, false) {
            next = Some(icon(size, &c, &shape));
        }
        let shape_opts: Vec<(String, String)> = SHAPES
            .iter()
            .map(|s| (s.to_string(), s.to_string()))
            .collect();
        if let Some(sh) = pick_dropdown(ui, card.salt("mshape"), &shape, &shape, &shape_opts, 120.0)
        {
            next = Some(icon(size, &colour, &sh));
        }
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
                ui.label(RichText::new("Temporary").size(14.0).color(pal::VALUE));
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
        ui.label(RichText::new("On").size(14.0).color(pal::VALUE));
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
        let visible: Vec<crate::overlay::rules::ListEntry> = (0..wf.rows().len())
            .map(crate::overlay::rules::ListEntry::Rule)
            .collect();
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
            &format!("{dir}/tests/fixtures/seed-styles.json"),
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
    fn condition_control_starts_after_label_column() {
        let ctx = headless_ctx();
        let mut start = 0.0;
        let mut inner_left = 0.0;
        ctx.run_ui(input(vec![]), |ui| {
            let area = Rect::from_min_size(Pos2::new(200.0, 0.0), vec2(500.0, 200.0));
            let mut child = ui.new_child(UiBuilder::new().max_rect(area));
            inner_left = child.max_rect().left();
            condition_header(
                &mut child,
                0,
                "Label",
                "hover",
                |ui| {
                    start = ui.min_rect().left().max(ui.cursor().left());
                },
                "Remove",
            );
        })
        .drop_without_applying_deltas();
        // Label at row left + 10, control at label column end + 16: the card's inner left.
        assert!((start - inner_left).abs() < 1.0, "{start} vs {inner_left}");
        assert_eq!(CARD_LEFT, 154.0);
    }

    #[test]
    fn defences_card_is_an_ff_card_with_three_lines() {
        // Normalised so a CRLF checkout (core.autocrlf) still finds the function end.
        let src = include_str!("body.rs").replace("\r\n", "\n");
        let start = src.find("fn defences_card(").unwrap();
        let f = &src[start..start + src[start..].find("\n}\n").unwrap()];
        assert!(f.contains("ff_card("));
        let wf = WorkingFilter::from_text("Show\n  BaseArmour > 300\n");
        assert!(
            wf.rows()[0]
                .conditions
                .iter()
                .any(|c| c.contains("BaseArmour"))
        );
        for (name, label) in conditions::DEFENCES {
            assert!(
                ["Armour", "Evasion", "Energy Shield"].contains(&label),
                "{name}"
            );
        }
    }

    #[test]
    fn section_heads_use_skin_buttons() {
        let src = include_str!("body.rs");
        let prod = &src[..src.find("mod tests").unwrap()];
        assert!(prod.contains("\"+ condition\", 132.0"));
        assert!(!prod.contains("\"+ style\""));
        assert!(!prod.contains("caption(ui, \"CONDITIONS\")"));
        assert!(!prod.contains("caption(ui, \"STYLE\")"));
        assert!(!prod.contains("egui::Checkbox::new"));
    }

    #[test]
    fn bool_segment_keeps_not_equal_operator() {
        let (_, id, e) = entries_of("Show\n    Corrupted != True\n");
        // `!= True` reads as No; picking Yes writes False and keeps the `!=`.
        assert_eq!(bool_effective(e[0].op, true), Some(false));
        let a = bool_edit(id, &e[0], true);
        assert_eq!(values_of(a), Some("False".to_string()));
        assert_eq!(bool_edit(id, &e[0], false), None);
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
            let r1 = condition_header(&mut child, 0, "Short", "a", |_| {}, "Remove");
            let r2 = condition_header(
                &mut child,
                1,
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
    fn custom_rules_have_no_add_style_button() {
        let production = include_str!("body.rs").split("mod tests {").next().unwrap();
        assert!(!production.contains(&["\"+ ", "style\""].concat()));
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
        assert!(production.contains("greyed, reason.is_none())"));
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
        scan_clicks_at(src, pending, &[170.0, 178.0, 186.0])
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
            for x in (150..300).step_by(8) {
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
        assert_eq!(rgba_edit(id, &e[0], [255, 121, 198, 255], false), None);
        assert_eq!(
            values_of(rgba_edit(id, &e[0], [255, 0, 198, 255], false)).as_deref(),
            Some("255 0 198")
        );
        assert_eq!(
            values_of(rgba_edit(id, &e[0], [255, 121, 198, 128], false)).as_deref(),
            Some("255 121 198 128")
        );
        let (_, id, e) = entries_of("Show\n    SetTextColor 255 121 198 255\n");
        assert_eq!(
            values_of(rgba_edit(id, &e[0], [255, 0, 198, 255], true)).as_deref(),
            Some("255 0 198 255")
        );
    }

    /// A bare 3-token background is alpha 240 in game: the Custom editor shows 240, keeps the
    /// line 3 tokens while the alpha stays 240 and writes 4 tokens for any other alpha. Text and
    /// border keep 255 as their 3-token meaning.
    #[test]
    fn three_token_background_reads_and_writes_alpha_240() {
        let src = "Show
    SetTextColor 1 2 3
    SetBackgroundColor 40 42 54
";
        let (mut wf, id, e) = entries_of(src);
        let (view, b) = rule_style_view(&e, BlockKeyword::Show);
        assert_eq!(
            view.background,
            Some(Rgba {
                rgba: [40, 42, 54, 240],
                had_alpha: false
            })
        );
        assert_eq!(view.text.map(|c| c.rgba), Some([1, 2, 3, 255]));
        assert!(style_leftovers(&e, &b).is_empty(), "the line shows exactly");
        let bg = &e[1];
        assert_eq!(
            values_of(rgba_edit(id, bg, [9, 42, 54, 240], false)).as_deref(),
            Some("9 42 54")
        );
        assert_eq!(
            values_of(rgba_edit(id, bg, [40, 42, 54, 255], false)).as_deref(),
            Some("40 42 54 255")
        );
        assert_eq!(rgba_edit(id, bg, [40, 42, 54, 240], false), None);
        apply_all(
            &mut wf,
            vec![rgba_edit(id, bg, [40, 42, 54, 128], false).unwrap()],
        );
        assert!(wf.text().contains(
            "SetBackgroundColor 40 42 54 128
"
        ));
        let e = wf.body_view(id);
        apply_all(
            &mut wf,
            vec![rgba_edit(id, &e[1], [40, 42, 54, 240], false).unwrap()],
        );
        assert_eq!(wf.text(), src);
    }

    /// CR-02: a 3-token line, a live alpha edit (written as 4 tokens), then Escape restoring the
    /// opening colour with the opening form gives back the original bytes.
    #[test]
    fn escape_after_an_alpha_edit_restores_a_three_token_line() {
        let src = "Show\n    SetTextColor 1 2 3\n";
        let (mut wf, id, e) = entries_of(src);
        let opened_had_alpha = e[0].values.len() >= 4;
        assert!(!opened_had_alpha);
        apply_all(
            &mut wf,
            vec![rgba_edit(id, &e[0], [1, 2, 3, 128], opened_had_alpha).unwrap()],
        );
        assert_eq!(wf.text(), "Show\n    SetTextColor 1 2 3 128\n");
        let e = wf.body_view(id);
        apply_all(
            &mut wf,
            vec![rgba_edit(id, &e[0], [1, 2, 3, 255], opened_had_alpha).unwrap()],
        );
        assert_eq!(wf.text(), src);
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
    fn absent_slots_add_one_line_in_the_default_form() {
        let (_, id, e) = entries_of("Show\n    Class \"Rings\"\n");
        let (_, b) = rule_style_view(&e, BlockKeyword::Show);
        assert_eq!(
            rule_intent_actions(
                id,
                &b,
                StyleIntent::Colour {
                    slot: ColourSlot::Background,
                    value: Some([40, 42, 54, 255]),
                    opened_had_alpha: false,
                }
            ),
            [RowAction::AddLine {
                id,
                kind: LineKind::Action,
                text: "SetBackgroundColor 40 42 54 255".into()
            }]
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
    fn rule_without_actions_renders_the_style_lines_idle() {
        let prod = include_str!("body.rs").split("mod tests {").next().unwrap();
        assert!(!prod.contains("No actions"));
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
            "/tests/fixtures/seed-styles.json"
        ))
        .unwrap()));
        wf
    }

    #[test]
    fn style_owned_follows_style_output() {
        let set = crate::loader::load_styles(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/seed-styles.json"
        ))
        .unwrap();
        assert!(style_owned(set.get("S").unwrap(), "SetFontSize"));
        assert!(style_owned(set.get("S").unwrap(), "PlayEffect"));
        for kw in crate::filter::keywords::KEYWORDS
            .iter()
            .filter(|k| k.kind == Kind::Action)
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

    /// Runs `draw` for a few frames, clicking at `click_at` on frame 1, and returns the texts
    /// painted below y = 34 (the open popup's rows) from the last frame, in paint order.
    fn popup_texts(click_at: Pos2, draw: &dyn Fn(&mut egui::Ui)) -> Vec<String> {
        let ctx = headless_ctx();
        let mut texts = Vec::new();
        for frame in 0..6 {
            let events = if frame == 1 {
                click_events(click_at)
            } else {
                vec![]
            };
            let mut raw = input(events);
            raw.time = Some(f64::from(frame) * 0.1);
            let mut out = ctx.run_ui(raw, |ui| draw(ui));
            out.textures_delta.clear();
            texts = out
                .shapes
                .iter()
                .filter_map(|cs| match &cs.shape {
                    egui::Shape::Text(t) if t.pos.y > 34.0 => Some(t.galley.text().to_string()),
                    _ => None,
                })
                .collect();
        }
        texts
    }

    #[test]
    fn foreign_operator_is_shown_and_not_normalised() {
        // `=` is not among the offered choices: it is appended, shown, and nothing is emitted.
        let choices = vec![
            (Some(Operator::EqEq), "is exactly".to_string()),
            (Some(Operator::Ne), "is not".to_string()),
        ];
        let cur = Some(Operator::Eq);
        let texts = popup_texts(Pos2::new(50.0, 16.0), &|ui| {
            assert_eq!(
                pick_dropdown(ui, "op", &cur, "equals", &choices, 104.0),
                None
            );
        });
        assert_eq!(texts, vec!["is exactly", "is not", "equals"]);
    }

    #[test]
    fn style_picker_lists_custom_then_presets_in_file_order() {
        let wf = styled(
            "Show
    Class \"Rings\"
",
        );
        let id = wf.rows()[0].id;
        let entries = wf.body_view(id);
        let names = wf.style_names();
        assert!(!names.is_empty());
        let texts = popup_texts(Pos2::new(50.0, 16.0), &|ui| {
            let bctx = BodyCtx {
                row: &wf.rows()[0],
                entries: &entries,
                editable: true,
                classes: None,
                working: &wf,
            };
            assert_eq!(style_picker(ui, &bctx), None);
        });
        let mut want = vec!["Custom".to_string()];
        want.extend(names);
        assert_eq!(texts, want);
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
    fn visibility_writes_show_or_hide_only_when_it_differs() {
        let (_, id, e) = entries_of("Minimal\n    Rarity Rare\n");
        let (view, b) = rule_style_view(&e, BlockKeyword::Minimal);
        assert_eq!(view.visibility, Vis::Minimal);
        assert_eq!(
            rule_intent_actions(id, &b, StyleIntent::Visibility(Vis::Show)),
            [RowAction::SetKeyword(id, BlockKeyword::Show)]
        );
        assert_eq!(
            rule_intent_actions(id, &b, StyleIntent::Visibility(Vis::Hide)),
            [RowAction::SetKeyword(id, BlockKeyword::Hide)]
        );
        let (_, b) = rule_style_view(&e, BlockKeyword::Hide);
        assert!(rule_intent_actions(id, &b, StyleIntent::Visibility(Vis::Hide)).is_empty());
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
    /// The `list.expanded` / `pending_scroll` resets in app.rs only fire for AddRule (adds a rule at the top of the file), Duplicate,
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

    const CLASS_HEADER_X: f32 = 170.0;

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
        'a: for y in (194..300).step_by(4) {
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
        'c: for hy in (218..300).step_by(4) {
            for (cx, cy) in [166.0f32, 220.0, 260.0]
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
        assert!(production.contains(".unwrap_or(PICKER_SECTION_DEFAULT_OPEN)"));
    }

    #[test]
    fn rules_editor_has_no_egui_default_widgets() {
        // Phase 31 contract: only the colour picker keeps its legacy look. Doc and line comments
        // are skipped so prose may name egui types.
        let strip = |src: &'static str| -> String {
            src.split("\nmod tests")
                .next()
                .unwrap()
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let banned = [
            "egui::ComboBox::from_id_salt",
            "egui::TextEdit::singleline",
            "egui::Button::new",
            "egui::Checkbox::new",
            "egui::CollapsingHeader::new",
            "small_button(",
            "ui.button(",
            "egui::Slider::new",
            "ui.checkbox(",
            "selectable_label(",
            "egui::DragValue",
            "text_edit_singleline",
            "ScrollArea::vertical",
        ];
        for (name, src) in [
            ("body.rs", strip(include_str!("body.rs"))),
            ("widgets.rs", strip(include_str!("widgets.rs"))),
        ] {
            for b in banned {
                assert!(!src.contains(b), "{name} still uses {b}");
            }
        }
    }

    #[test]
    fn picker_popup_uses_skin_search_and_sections() {
        // Normalised so a CRLF checkout (core.autocrlf) still finds the function end.
        let src = include_str!("body.rs").replace("\r\n", "\n");
        let production = src.split("mod tests {").next().unwrap();
        let start = production.find("pub(crate) fn picker_popup").unwrap();
        let end = production[start..].find("\n}\n").unwrap() + start;
        let body = &production[start..end];
        assert!(body.contains("ff_search_field("));
        assert!(body.contains("section_head("));
        assert!(body.contains("paint_box_behind("));
        assert!(!production.contains("CollapsingHeader"));
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
        'scan: for hy in (218..300).step_by(4) {
            for cx in [166.0f32, 190.0, 220.0, 260.0] {
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
        let visible: Vec<crate::overlay::rules::ListEntry> = (0..wf.rows().len())
            .map(crate::overlay::rules::ListEntry::Rule)
            .collect();
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
    fn between_card_labels_min_and_max() {
        let (ctx, wf, mut list) = defences_session(PAIR, false);
        let mut seen = String::new();
        for _ in 0..3 {
            seen = rendered_text(&ctx, &wf, &mut list);
        }
        let min = seen.find("Min").expect("Min label");
        let max = seen.find("Max").expect("Max label");
        assert!(min < max, "{seen}");
    }

    #[test]
    fn int_control_number_field_right_of_slider() {
        let ctx = headless_ctx();
        let (mut slider, mut field) = (egui::Rect::NOTHING, egui::Rect::NOTHING);
        let mut v = 30_i64;
        ctx.run_ui(input(vec![]), |ui| {
            ui.set_max_width(360.0);
            ui.horizontal(|ui| {
                slider = widgets::themed_slider(ui, &mut v, 1, 100, false).rect;
                field = widgets::number_field(ui, "n", &mut v, widgets::NUMBER_FIELD_W)
                    .response
                    .rect;
            });
        })
        .drop_without_applying_deltas();
        assert!(field.min.x >= slider.max.x, "{slider:?} {field:?}");
        assert!(field.max.x <= 360.0 + 1.0, "{field:?}");
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

    // ---- Custom STYLE lines: the lossless rule backend (quick 261016-qi7) ----

    /// The text after applying `intent`'s line edits to the first rule of `src`.
    fn intent_text(src: &str, intent: StyleIntent) -> String {
        let (mut wf, id, e) = entries_of(src);
        let (_, b) = rule_style_view(&e, wf.rows()[0].keyword);
        let acts = rule_intent_actions(id, &b, intent);
        apply_all(&mut wf, acts);
        wf.text().to_string()
    }

    fn leftover_texts(src: &str) -> Vec<String> {
        let (wf, _, e) = entries_of(src);
        let (_, b) = rule_style_view(&e, wf.rows()[0].keyword);
        style_leftovers(&e, &b)
            .iter()
            .map(|e| e.text.trim().to_string())
            .collect()
    }

    fn colour(slot: ColourSlot, value: Option<[u8; 4]>, had: bool) -> StyleIntent {
        StyleIntent::Colour {
            slot,
            value,
            opened_had_alpha: had,
        }
    }

    const LEFTOVERS: &str = "Show\n    SetFontSize 30\n    PlayAlertSoundPositional 3 100\n    CustomAlertSoundOptional \"x.mp3\"\n    EnableDropSound\n    PlayEffect None\n    PlayAlertSound ShAlchemy\n    MinimapIcon -1\n    FutureKw 3\n    SetFontSize 40\n";

    #[test]
    fn rule_style_view_binds_the_last_line_of_each_family() {
        let src = "Hide\n    SetTextColor 1 2 3\n    SetTextColor 4 5 6 7\n    SetBackgroundColor 9 9 9 128\n    PlayAlertSoundPositional 2 50\n    PlayAlertSound 5\n    CustomAlertSound \"a.mp3\" 80\n    MinimapIcon -1\n    MinimapIcon 2 Blue Star\n    PlayEffect None\n    PlayEffect Green Temp\n    DisableDropSound False\n    SetFontSize 60\n";
        let (_, _, e) = entries_of(src);
        let (view, b) = rule_style_view(&e, BlockKeyword::Hide);
        assert_eq!(view.visibility, Vis::Hide);
        assert_eq!(view.background.map(|c| c.rgba), Some([9, 9, 9, 128]));
        assert_eq!(
            view.text,
            Some(Rgba {
                rgba: [4, 5, 6, 7],
                had_alpha: true
            })
        );
        assert_eq!(view.border, None);
        assert_eq!(
            view.background,
            Some(Rgba {
                rgba: [9, 9, 9, 128],
                had_alpha: true
            })
        );
        assert_eq!(view.font, Some(45), "60 shows clamped into 1-45");
        assert_eq!(
            view.minimap,
            Some(Cancelable::Set(mc::Minimap {
                size: 2,
                color: FilterColor::Blue,
                shape: MinimapShape::Star
            }))
        );
        assert_eq!(
            view.beam,
            Some(Cancelable::Set(Beam {
                color: FilterColor::Green,
                temp: true
            }))
        );
        assert_eq!(
            view.sound,
            Some(Cancelable::Set(Sound {
                id: None,
                custom: Some("a.mp3".to_string()),
                vol: 80,
                positional: false
            })),
            "one sound family: the last of the four keywords"
        );
        assert!(view.sound_volume_written);
        assert_eq!(
            view.drop_sound,
            Some(DropSound::Normal),
            "DisableDropSound False reads as Normal"
        );
        assert!(b.drop.is_some(), "it binds");
        assert_eq!(
            style_leftovers(&e, &b)
                .iter()
                .map(|e| e.text.trim())
                .collect::<Vec<_>>(),
            [
                "SetTextColor 1 2 3",
                "PlayAlertSoundPositional 2 50",
                "PlayAlertSound 5",
                "MinimapIcon -1",
                "PlayEffect None",
                "DisableDropSound False",
                "SetFontSize 60",
            ],
            "earlier lines of a family, and the bound lines it shows inexactly"
        );
    }

    #[test]
    fn an_unrepresentable_last_line_shows_its_nearest_state_and_stays_listed() {
        let src = "Show\n    MinimapIcon 1 Blue Star\n    MinimapIcon -1\n    PlayEffect Red\n    PlayEffect None\n    PlayAlertSound 3\n    PlayAlertSound ShAlchemy\n    DisableDropSound\n    EnableDropSound\n    SetFontSize 30\n    SetFontSize 0\n";
        let (_, _, e) = entries_of(src);
        let (view, b) = rule_style_view(&e, BlockKeyword::Show);
        assert_eq!(
            view.minimap,
            Some(Cancelable::Cancel),
            "MinimapIcon -1 is the cancel value"
        );
        assert_eq!(
            view.beam,
            Some(Cancelable::Cancel),
            "PlayEffect None is the cancel value"
        );
        assert_eq!(view.sound, None, "a named sound has no dropdown entry");
        assert_eq!(
            view.drop_sound,
            Some(DropSound::Normal),
            "EnableDropSound wins"
        );
        assert_eq!(view.font, Some(1));
        for (slot, line) in [
            (&b.minimap, "MinimapIcon -1"),
            (&b.beam, "PlayEffect None"),
            (&b.sound, "PlayAlertSound ShAlchemy"),
            (&b.drop, "EnableDropSound"),
            (&b.font, "SetFontSize 0"),
        ] {
            assert_eq!(slot.as_ref().map(|e| e.text.trim()), Some(line));
        }
        // Listed: the earlier lines, and the bound ones it cannot show (the named sound and the
        // clamped font); the cancel values and EnableDropSound are shown exactly.
        assert_eq!(
            style_leftovers(&e, &b)
                .iter()
                .map(|e| e.text.trim())
                .collect::<Vec<_>>(),
            [
                "MinimapIcon 1 Blue Star",
                "PlayEffect Red",
                "PlayAlertSound 3",
                "PlayAlertSound ShAlchemy",
                "DisableDropSound",
                "SetFontSize 30",
                "SetFontSize 0",
            ]
        );
        // A positional alert is shown exactly with its toggle on.
        let (_, _, e) = entries_of("Show\n    PlayAlertSoundPositional 4 120\n");
        let (view, b) = rule_style_view(&e, BlockKeyword::Show);
        assert_eq!(
            view.sound,
            Some(Cancelable::Set(Sound {
                id: Some(4),
                custom: None,
                vol: 120,
                positional: true
            }))
        );
        assert!(style_leftovers(&e, &b).is_empty());
        // An optional sound shows as its plain kind and is listed too.
        let (_, _, e) = entries_of("Show\n    CustomAlertSoundOptional \"x.mp3\"\n");
        let (view, b) = rule_style_view(&e, BlockKeyword::Show);
        assert_eq!(
            view.sound,
            Some(Cancelable::Set(Sound {
                id: None,
                custom: Some("x.mp3".to_string()),
                vol: 100,
                positional: false
            }))
        );
        assert_eq!(style_leftovers(&e, &b).len(), 1);
        // `MinimapIcon -1 Red Star` and `PlayAlertSoundPositional None` are the cancel value,
        // shown inexactly (listed too).
        let (_, _, e) =
            entries_of("Show\n    MinimapIcon -1 Red Star\n    PlayAlertSoundPositional None\n");
        let (view, b) = rule_style_view(&e, BlockKeyword::Show);
        assert_eq!(view.minimap, Some(Cancelable::Cancel));
        // Its colour and shape are what leaving `/` brings back (IN-04).
        assert_eq!(
            view.minimap_hint,
            Some(mc::Minimap {
                size: 0,
                color: FilterColor::Red,
                shape: MinimapShape::Star,
            })
        );
        assert_eq!(view.sound, Some(Cancelable::Cancel));
        assert_eq!(style_leftovers(&e, &b).len(), 2);
        // A bare `-1` or unknown names give no hint.
        for src in [
            "Show\n    MinimapIcon -1\n",
            "Show\n    MinimapIcon -1 Mauve Star\n",
            "Show\n    MinimapIcon -1 Red Blob\n",
            "Show\n    MinimapIcon -1 Red\n",
        ] {
            let (_, _, e) = entries_of(src);
            let (view, _) = rule_style_view(&e, BlockKeyword::Show);
            assert_eq!(view.minimap, Some(Cancelable::Cancel), "{src}");
            assert_eq!(view.minimap_hint, None, "{src}");
        }
        // The bare `PlayAlertSound None` is exact, and so is `DisableDropSound`.
        let (_, _, e) = entries_of("Show\n    PlayAlertSound None\n    DisableDropSound\n");
        let (view, b) = rule_style_view(&e, BlockKeyword::Show);
        assert_eq!(view.sound, Some(Cancelable::Cancel));
        assert_eq!(view.drop_sound, Some(DropSound::Silent));
        assert!(style_leftovers(&e, &b).is_empty());
        assert!(!view.sound_volume_written);
    }

    #[test]
    fn an_edit_rewrites_the_last_line_and_never_adds_a_second_one() {
        let pick = mc::Minimap {
            size: 0,
            color: FilterColor::White,
            shape: MinimapShape::Circle,
        };
        assert_eq!(
            intent_text(
                "Show\n    Rarity Rare\n    MinimapIcon -1\n",
                StyleIntent::Minimap(Some(Cancelable::Set(pick.clone())))
            ),
            "Show\n    Rarity Rare\n    MinimapIcon 0 White Circle\n"
        );
        assert_eq!(
            intent_text(
                "Show\n    MinimapIcon -1 # hidden\n    SetFontSize 30\n",
                StyleIntent::Minimap(Some(Cancelable::Set(pick)))
            ),
            "Show\n    MinimapIcon 0 White Circle # hidden\n    SetFontSize 30\n"
        );
        let two_fonts = "Show\n    SetFontSize 30\n    SetFontSize 40\n";
        let (_, _, e) = entries_of(two_fonts);
        assert_eq!(rule_style_view(&e, BlockKeyword::Show).0.font, Some(40));
        assert_eq!(
            intent_text(two_fonts, StyleIntent::Font(Some(35))),
            "Show\n    SetFontSize 30\n    SetFontSize 35\n"
        );
        assert_eq!(
            intent_text(two_fonts, StyleIntent::Font(None)),
            "Show\n    SetFontSize 30\n",
            "clearing removes the effective line; the earlier one then shows through"
        );
        let two_colours = "Show\n    SetTextColor 1 2 3\n    SetTextColor 4 5 6\n";
        assert_eq!(
            intent_text(
                two_colours,
                colour(ColourSlot::Text, Some([7, 8, 9, 255]), false)
            ),
            "Show\n    SetTextColor 1 2 3\n    SetTextColor 7 8 9\n"
        );
        assert_eq!(
            intent_text(
                "Show\n    PlayEffect None\n",
                StyleIntent::Beam(Some(Cancelable::Set(Beam {
                    color: FilterColor::Red,
                    temp: false
                })))
            ),
            "Show\n    PlayEffect Red\n"
        );
        assert_eq!(
            intent_text("Show\n    SetFontSize 50\n", StyleIntent::Font(Some(44))),
            "Show\n    SetFontSize 44\n"
        );
        assert_eq!(
            intent_text(
                "Show\n    PlayAlertSound ShAlchemy\n",
                StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    id: Some(2),
                    custom: None,
                    vol: 100,
                    positional: false
                })))
            ),
            "Show\n    PlayAlertSound 2 100\n"
        );
        // A different sound keyword is the same family: no second alert sound.
        assert_eq!(
            intent_text(
                "Show\n    CustomAlertSoundOptional \"x.mp3\"\n",
                StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    id: Some(2),
                    custom: None,
                    vol: 100,
                    positional: false
                })))
            ),
            // The file's default 100 is written so the loudness stays (WR-02).
            "Show\n    PlayAlertSound 2 100\n"
        );
    }

    #[test]
    fn colour_edits_keep_the_line_form() {
        let src = "Show\n    SetTextColor 10 20 30\n    SetFontSize 30\n";
        assert_eq!(
            intent_text(src, colour(ColourSlot::Text, Some([1, 2, 3, 255]), false)),
            "Show\n    SetTextColor 1 2 3\n    SetFontSize 30\n"
        );
        assert_eq!(
            intent_text(
                src,
                colour(ColourSlot::Text, Some([10, 20, 30, 128]), false)
            ),
            "Show\n    SetTextColor 10 20 30 128\n    SetFontSize 30\n"
        );
        assert_eq!(
            intent_text(src, colour(ColourSlot::Text, None, false)),
            "Show\n    SetFontSize 30\n"
        );
        assert_eq!(
            intent_text(src, colour(ColourSlot::Border, Some([5, 6, 7, 255]), false)),
            "Show\n    SetTextColor 10 20 30\n    SetFontSize 30\n    SetBorderColor 5 6 7 255\n"
        );
    }

    #[test]
    fn minimap_none_removes_only_the_bound_line() {
        let src =
            "Show\n    MinimapIcon -1\n    MinimapIcon 1 Blue Star\n    MinimapIcon 0 Red Circle\n";
        assert_eq!(
            intent_text(src, StyleIntent::Minimap(None)),
            "Show\n    MinimapIcon -1\n    MinimapIcon 1 Blue Star\n"
        );
        let pink = mc::Minimap {
            size: 1,
            color: FilterColor::Pink,
            shape: MinimapShape::Star,
        };
        assert_eq!(
            intent_text(src, StyleIntent::Minimap(Some(Cancelable::Set(pink)))),
            "Show\n    MinimapIcon -1\n    MinimapIcon 1 Blue Star\n    MinimapIcon 1 Pink Star\n"
        );
        assert_eq!(
            leftover_texts(src),
            ["MinimapIcon -1", "MinimapIcon 1 Blue Star"]
        );
        let small = mc::Minimap {
            size: 2,
            color: FilterColor::White,
            shape: MinimapShape::Circle,
        };
        assert_eq!(
            intent_text(
                "Show\n    Rarity Rare\n",
                StyleIntent::Minimap(Some(Cancelable::Set(small)))
            ),
            "Show\n    Rarity Rare\n    MinimapIcon 2 White Circle\n"
        );
    }

    #[test]
    fn unrepresentable_lines_survive_other_edits_as_leftovers() {
        assert_eq!(
            leftover_texts(LEFTOVERS),
            [
                "SetFontSize 30",
                "PlayAlertSoundPositional 3 100",
                "CustomAlertSoundOptional \"x.mp3\"",
                "PlayAlertSound ShAlchemy",
            ]
        );
        assert_eq!(
            intent_text(LEFTOVERS, StyleIntent::Font(Some(35))),
            LEFTOVERS.replacen("SetFontSize 40", "SetFontSize 35", 1)
        );
        // The bound `EnableDropSound` is rewritten in place, never contradicted by a new line.
        assert_eq!(
            intent_text(LEFTOVERS, StyleIntent::DropSound(Some(DropSound::Silent))),
            LEFTOVERS.replacen("    EnableDropSound\n", "    DisableDropSound\n", 1)
        );
    }

    #[test]
    fn drop_sound_follows_the_last_disable_or_enable_line() {
        let off = "Show\n    DisableDropSound\n    EnableDropSound False\n";
        let (_, _, e) = entries_of(off);
        assert_eq!(
            rule_style_view(&e, BlockKeyword::Show).0.drop_sound,
            Some(DropSound::Silent)
        );
        assert_eq!(
            intent_text(off, StyleIntent::DropSound(Some(DropSound::Normal))),
            "Show\n    DisableDropSound\n    EnableDropSound\n"
        );
        assert_eq!(
            intent_text(off, StyleIntent::DropSound(Some(DropSound::Silent))),
            off
        );
        let on = "Show\n    EnableDropSound # loud\n";
        assert_eq!(
            intent_text(on, StyleIntent::DropSound(Some(DropSound::Silent))),
            "Show\n    DisableDropSound # loud\n"
        );
        assert_eq!(
            intent_text(on, StyleIntent::DropSound(Some(DropSound::Normal))),
            on
        );
        assert_eq!(intent_text(on, StyleIntent::DropSound(None)), "Show\n");
    }

    #[test]
    fn sound_edits_keep_the_line_and_its_volume_form() {
        let custom = |vol| Sound {
            id: None,
            custom: Some("uwu.mp3".to_string()),
            vol,
            positional: false,
        };
        let alert = |k, vol| Sound {
            id: Some(k),
            custom: None,
            vol,
            positional: false,
        };
        let src = "Show\n    PlayAlertSound 3 200\n    SetFontSize 30\n";
        assert_eq!(
            intent_text(src, StyleIntent::Sound(Some(Cancelable::Set(custom(200))))),
            "Show\n    CustomAlertSound \"uwu.mp3\" 200\n    SetFontSize 30\n"
        );
        assert_eq!(
            intent_text(src, StyleIntent::Sound(None)),
            "Show\n    SetFontSize 30\n"
        );
        let bare = "Show\n    PlayAlertSound 5\n";
        assert_eq!(
            intent_text(
                bare,
                StyleIntent::Sound(Some(Cancelable::Set(alert(5, 120))))
            ),
            "Show\n    PlayAlertSound 5 120\n"
        );
        assert_eq!(
            intent_text(
                bare,
                StyleIntent::Sound(Some(Cancelable::Set(alert(7, 50))))
            ),
            "Show\n    PlayAlertSound 7\n"
        );
        assert_eq!(
            intent_text(bare, StyleIntent::Sound(Some(Cancelable::Set(custom(50))))),
            "Show\n    CustomAlertSound \"uwu.mp3\" 50\n",
            "WR-02: the alert's default 50 is written, a bare file line would play at 100"
        );
        // Switching kind on a volumeless line writes the old kind's default, keeping the loudness.
        assert_eq!(
            intent_text(
                "Show\n    PlayAlertSound 6\n",
                StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    id: None,
                    custom: Some("f".to_string()),
                    vol: 50,
                    positional: false,
                })))
            ),
            "Show\n    CustomAlertSound \"f\" 50\n"
        );
        assert_eq!(
            intent_text(
                "Show\n    CustomAlertSound \"a.mp3\"\n",
                StyleIntent::Sound(Some(Cancelable::Set(alert(4, 100))))
            ),
            "Show\n    PlayAlertSound 4 100\n"
        );
        // A same-kind pick on a volumeless line still writes no volume.
        assert_eq!(
            intent_text(
                "Show\n    CustomAlertSound \"a.mp3\"\n",
                StyleIntent::Sound(Some(Cancelable::Set(custom(100))))
            ),
            "Show\n    CustomAlertSound \"uwu.mp3\"\n"
        );
        assert_eq!(
            intent_text(
                "Show\n    Rarity Rare\n",
                StyleIntent::Sound(Some(Cancelable::Set(alert(2, 100))))
            ),
            "Show\n    Rarity Rare\n    PlayAlertSound 2 100\n"
        );
        // A bad file name writes nothing.
        let bad = Sound {
            id: None,
            custom: Some("a\"b.mp3".to_string()),
            vol: 200,
            positional: false,
        };
        assert_eq!(
            intent_text(src, StyleIntent::Sound(Some(Cancelable::Set(bad)))),
            src
        );
    }

    /// IN-01: a picker opened on an empty slot reports the 4-token form for the whole session
    /// (`colour_slot_rgba`), so the line it added keeps 4 tokens on every later change.
    #[test]
    fn a_colour_added_to_an_empty_slot_stays_four_valued() {
        let added = intent_text(
            "Show\n    Rarity Rare\n",
            colour(ColourSlot::Border, Some([5, 6, 7, 255]), true),
        );
        assert_eq!(
            added,
            "Show\n    Rarity Rare\n    SetBorderColor 5 6 7 255\n"
        );
        assert_eq!(
            intent_text(
                &added,
                colour(ColourSlot::Border, Some([8, 9, 10, 255]), true)
            ),
            "Show\n    Rarity Rare\n    SetBorderColor 8 9 10 255\n"
        );
    }

    #[test]
    fn sound_kind_switch_keeps_the_inline_comment() {
        let custom = Sound {
            id: None,
            custom: Some("uwu.mp3".to_string()),
            vol: 200,
            positional: false,
        };
        assert_eq!(
            intent_text(
                "Show\n    PlayAlertSound 3 200 # loud\n",
                StyleIntent::Sound(Some(Cancelable::Set(custom)))
            ),
            "Show\n    CustomAlertSound \"uwu.mp3\" 200 # loud\n"
        );
        assert_eq!(
            intent_text(
                "Show\n    CustomAlertSound \"a.mp3\"  # mine\n",
                StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    id: Some(4),
                    custom: None,
                    vol: 100,
                    positional: false
                })))
            ),
            "Show\n    PlayAlertSound 4 100  # mine\n"
        );
    }

    #[test]
    fn positional_optional_and_out_of_range_sound_lines_keep_their_form() {
        // Same kind: the keyword stays.
        let alert = |id, vol, positional| {
            StyleIntent::Sound(Some(Cancelable::Set(Sound {
                id: Some(id),
                custom: None,
                vol,
                positional,
            })))
        };
        assert_eq!(
            intent_text(
                "Show\n    PlayAlertSoundPositional 3 100\n",
                alert(5, 100, true)
            ),
            "Show\n    PlayAlertSoundPositional 5 100\n"
        );
        // The Positional toggle flips the keyword in place, keeping the volume form and a comment.
        assert_eq!(
            intent_text(
                "Show\n    PlayAlertSoundPositional 3 100 # a\n",
                alert(3, 100, false)
            ),
            "Show\n    PlayAlertSound 3 100 # a\n"
        );
        assert_eq!(
            intent_text("Show\n    PlayAlertSound 3 # a\n", alert(3, 50, true)),
            "Show\n    PlayAlertSoundPositional 3 # a\n"
        );
        assert_eq!(
            intent_text("Show\n    Rarity Rare\n", alert(6, 200, true)),
            "Show\n    Rarity Rare\n    PlayAlertSoundPositional 6 200\n"
        );
        // A file never takes the positional form.
        assert_eq!(
            intent_text(
                "Show\n    PlayAlertSoundPositional 3 100\n",
                StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    id: None,
                    custom: Some("a.mp3".to_string()),
                    vol: 100,
                    positional: true,
                })))
            ),
            "Show\n    CustomAlertSound \"a.mp3\" 100\n"
        );
        assert_eq!(
            intent_text(
                "Show\n    CustomAlertSoundOptional \"x.mp3\" 90\n",
                StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    id: None,
                    custom: Some("y.mp3".to_string()),
                    vol: 90,
                    positional: false
                })))
            ),
            "Show\n    CustomAlertSoundOptional \"y.mp3\" 90\n"
        );
        // A volume shown clamped is kept as written until the slider moves.
        let loud = "Show\n    PlayAlertSound 3 400\n";
        let (_, _, e) = entries_of(loud);
        assert_eq!(
            rule_style_view(&e, BlockKeyword::Show)
                .0
                .sound
                .and_then(|s| s.value().map(|s| s.vol)),
            Some(300)
        );
        assert_eq!(
            intent_text(
                loud,
                StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    id: Some(6),
                    custom: None,
                    vol: 300,
                    positional: false
                })))
            ),
            "Show\n    PlayAlertSound 6 400\n"
        );
        assert_eq!(
            intent_text(
                loud,
                StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    id: Some(3),
                    custom: None,
                    vol: 250,
                    positional: false
                })))
            ),
            "Show\n    PlayAlertSound 3 250\n"
        );
    }

    #[test]
    fn font_and_drop_sound_add_and_remove_one_line() {
        assert_eq!(
            intent_text("Show\n    SetFontSize 30\n", StyleIntent::Font(None)),
            "Show\n"
        );
        assert_eq!(
            intent_text("Show\n    Rarity Rare\n", StyleIntent::Font(Some(32))),
            "Show\n    Rarity Rare\n    SetFontSize 32\n"
        );
        assert_eq!(
            intent_text(
                "Show\n    Rarity Rare\n",
                StyleIntent::DropSound(Some(DropSound::Silent))
            ),
            "Show\n    Rarity Rare\n    DisableDropSound\n"
        );
        assert_eq!(
            intent_text(
                "Show\n    Rarity Rare\n",
                StyleIntent::DropSound(Some(DropSound::Normal))
            ),
            "Show\n    Rarity Rare\n    EnableDropSound\n"
        );
        for line in [
            "DisableDropSound",
            "DisableDropSound True",
            "EnableDropSound",
        ] {
            assert_eq!(
                intent_text(&format!("Show\n    {line}\n"), StyleIntent::DropSound(None)),
                "Show\n"
            );
        }
        // `DisableDropSound False` means Normal: Silent rewrites it to the bare keyword, Normal
        // keeps it as written.
        let off = "Show\n    DisableDropSound False\n";
        assert_eq!(
            intent_text(off, StyleIntent::DropSound(Some(DropSound::Silent))),
            "Show\n    DisableDropSound\n"
        );
        assert_eq!(
            intent_text(off, StyleIntent::DropSound(Some(DropSound::Normal))),
            off
        );
        assert_eq!(
            intent_text(
                "Show\n    DisableDropSound\n",
                StyleIntent::DropSound(Some(DropSound::Normal))
            ),
            "Show\n    EnableDropSound\n"
        );
    }

    fn style_probes(
        ctx: &egui::Context,
        wf: &WorkingFilter,
        list: &mut ListUi,
        tag: &str,
    ) -> Vec<Rect> {
        style_lines::drain_probes();
        frame_of(ctx, input(vec![]), wf, list, None);
        style_lines::drain_probes()
            .into_iter()
            .filter(|(t, _)| *t == tag)
            .map(|(_, r)| r)
            .collect()
    }

    #[test]
    fn untouched_custom_rule_renders_idle_and_byte_identical() {
        let all = "Show\n    SetTextColor 1 2 3\n    PlayAlertSound 5\n    DisableDropSound\n    MinimapIcon 1 Blue Star\n    PlayEffect Red\n    SetBackgroundColor 0 0 0 128\n";
        let clamped = "Show\n    SetFontSize 60\n    PlayAlertSound 3 400\n    MinimapIcon -1\n    EnableDropSound False\n    SetTextColor 1 2 3\n    SetTextColor 4 5 6 7\n";
        let cancel = "Show\n    MinimapIcon -1 Red Star\n    PlayEffect None\n    PlayAlertSound None\n    EnableDropSound\n";
        let positional = "Show\n    PlayAlertSoundPositional 6 # loud\n    CustomAlertSound \"x.mp3\"\n    CustomAlertSound \"gone.mp3\" 90\n";
        for src in [all, LEFTOVERS, clamped, cancel, positional] {
            let wf = WorkingFilter::from_text(src);
            let ctx = headless_ctx();
            let mut list = ListUi {
                expanded: Some(wf.rows()[0].id),
                ..Default::default()
            };
            for _ in 0..3 {
                let a = frame_of(&ctx, input(vec![]), &wf, &mut list, None);
                assert!(a.is_empty(), "idle frame emitted {a:?}");
            }
            assert_eq!(wf.text(), src);
        }
    }

    #[test]
    fn custom_rules_get_the_style_lines_presets_do_not() {
        let ctx = headless_ctx();
        let custom = WorkingFilter::from_text("Show\n    Class \"Rings\"\n");
        let mut list = ListUi {
            expanded: Some(custom.rows()[0].id),
            ..Default::default()
        };
        assert_eq!(
            style_probes(&ctx, &custom, &mut list, "ed-visibility").len(),
            1
        );
        let preset = styled("# ff:style S\nShow\n    Class \"Rings\"\n    SetFontSize 37\n");
        let mut list = ListUi {
            expanded: Some(preset.rows()[0].id),
            ..Default::default()
        };
        assert!(style_probes(&ctx, &preset, &mut list, "ed-visibility").is_empty());
        assert!(style_probes(&ctx, &preset, &mut list, "ed-drop-check").is_empty());
    }

    #[test]
    fn clicking_a_style_line_writes_its_line() {
        let mut wf = WorkingFilter::from_text("Show\n    Rarity Rare\n");
        let ctx = headless_ctx();
        let mut list = ListUi {
            expanded: Some(wf.rows()[0].id),
            ..Default::default()
        };
        let r = style_probes(&ctx, &wf, &mut list, "ed-drop-check")[0];
        let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
        mirror_apply(&mut wf, &mut list, a);
        assert_eq!(wf.text(), "Show\n    Rarity Rare\n    DisableDropSound\n");
        // The added line is bound to its style line, so the focus request is dropped.
        frame_of(&ctx, input(vec![]), &wf, &mut list, None);
        assert_eq!(list.focus_line, None);
        let r = style_probes(&ctx, &wf, &mut list, "ed-font-check")[0];
        let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
        mirror_apply(&mut wf, &mut list, a);
        assert_eq!(
            wf.text(),
            "Show\n    Rarity Rare\n    DisableDropSound\n    SetFontSize 32\n"
        );
    }

    #[test]
    fn picking_a_colour_over_a_hidden_icon_rewrites_that_line() {
        let mut wf = WorkingFilter::from_text("Show\n    MinimapIcon -1\n");
        let ctx = headless_ctx();
        let mut list = ListUi {
            expanded: Some(wf.rows()[0].id),
            ..Default::default()
        };
        // `MinimapIcon -1` is the cancel value: the size switch is drawn but inert.
        let r = style_probes(&ctx, &wf, &mut list, "ed-mm-size")[0];
        let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
        assert!(a.is_empty(), "{a:?}");
        let pink = crate::skin::minimap::index_of(&crate::skin::minimap::COLOURS, "Pink").unwrap();
        let r = style_probes(&ctx, &wf, &mut list, "ed-mm-colour")[pink];
        let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
        mirror_apply(&mut wf, &mut list, a);
        assert_eq!(wf.text(), "Show\n    MinimapIcon 0 Pink Circle\n");
        // The / tile writes the cancel value back on the same line.
        let r = style_probes(&ctx, &wf, &mut list, "ed-mm-cancel")[0];
        let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
        mirror_apply(&mut wf, &mut list, a);
        assert_eq!(wf.text(), "Show\n    MinimapIcon -1\n");
    }

    #[test]
    fn leaving_a_hidden_icon_keeps_the_colour_and_shape_it_names() {
        let mut wf = WorkingFilter::from_text("Show\n    MinimapIcon -1 Red Star\n");
        let ctx = headless_ctx();
        let mut list = ListUi {
            expanded: Some(wf.rows()[0].id),
            ..Default::default()
        };
        // Under `/` the shapes are tinted with the line's colour: picking a colour keeps its shape.
        let pink = crate::skin::minimap::index_of(&crate::skin::minimap::COLOURS, "Pink").unwrap();
        let r = style_probes(&ctx, &wf, &mut list, "ed-mm-colour")[pink];
        let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
        mirror_apply(&mut wf, &mut list, a);
        assert_eq!(wf.text(), "Show\n    MinimapIcon 0 Pink Star\n");
        // Its own colour leaves `/` as written.
        let mut wf = WorkingFilter::from_text("Show\n    MinimapIcon -1 Red Star\n");
        let mut list = ListUi {
            expanded: Some(wf.rows()[0].id),
            ..Default::default()
        };
        let ctx = headless_ctx();
        let red = crate::skin::minimap::index_of(&crate::skin::minimap::COLOURS, "Red").unwrap();
        let r = style_probes(&ctx, &wf, &mut list, "ed-mm-colour")[red];
        let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
        mirror_apply(&mut wf, &mut list, a);
        assert_eq!(wf.text(), "Show\n    MinimapIcon 0 Red Star\n");
        // Off and on again brings back a bare `-1`, but leaving `/` still keeps the Star (WR-03).
        let mut wf = WorkingFilter::from_text("Show\n    MinimapIcon -1 Red Star\n");
        let mut list = ListUi {
            expanded: Some(wf.rows()[0].id),
            ..Default::default()
        };
        let ctx = headless_ctx();
        for _ in 0..2 {
            let r = style_probes(&ctx, &wf, &mut list, "ed-mm-check")[0];
            let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
            mirror_apply(&mut wf, &mut list, a);
            frame_of(&ctx, input(vec![]), &wf, &mut list, None);
        }
        assert_eq!(wf.text(), "Show\n  MinimapIcon -1\n");
        let r = style_probes(&ctx, &wf, &mut list, "ed-mm-colour")[pink];
        let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
        mirror_apply(&mut wf, &mut list, a);
        assert_eq!(wf.text(), "Show\n  MinimapIcon 0 Pink Star\n");
    }

    #[test]
    fn rule_group_checks_add_and_remove_one_line() {
        let mut wf = WorkingFilter::from_text("Show\n    Rarity Rare\n");
        let ctx = headless_ctx();
        let mut list = ListUi {
            expanded: Some(wf.rows()[0].id),
            ..Default::default()
        };
        for tag in ["ed-sound-check", "ed-beam-check"] {
            let r = style_probes(&ctx, &wf, &mut list, tag)[0];
            let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
            mirror_apply(&mut wf, &mut list, a);
            frame_of(&ctx, input(vec![]), &wf, &mut list, None);
        }
        assert_eq!(
            wf.text(),
            "Show\n    Rarity Rare\n    PlayAlertSound 1 100\n    PlayEffect White\n"
        );
        let r = style_probes(&ctx, &wf, &mut list, "ed-sound-positional")[0];
        let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
        mirror_apply(&mut wf, &mut list, a);
        assert_eq!(
            wf.text(),
            "Show\n    Rarity Rare\n    PlayAlertSoundPositional 1 100\n    PlayEffect White\n"
        );
        assert_eq!(leftover_rows(&ctx, &wf), 0);
        let r = style_probes(&ctx, &wf, &mut list, "ed-sound-check")[0];
        let a = frame_of(&ctx, input(click_events(r.center())), &wf, &mut list, None);
        mirror_apply(&mut wf, &mut list, a);
        assert_eq!(wf.text(), "Show\n    Rarity Rare\n    PlayEffect White\n");
    }

    #[test]
    fn cancel_values_rewrite_the_bound_line_or_add_one() {
        let star = "Show\n    MinimapIcon 1 Blue Star # icon\n    PlayEffect Red Temp\n    PlayAlertSound 3 100\n";
        assert_eq!(
            intent_text(star, StyleIntent::Minimap(Some(Cancelable::Cancel))),
            "Show\n    MinimapIcon -1 # icon\n    PlayEffect Red Temp\n    PlayAlertSound 3 100\n"
        );
        assert_eq!(
            intent_text(star, StyleIntent::Beam(Some(Cancelable::Cancel))),
            "Show\n    MinimapIcon 1 Blue Star # icon\n    PlayEffect None\n    PlayAlertSound 3 100\n"
        );
        assert_eq!(
            intent_text(star, StyleIntent::Sound(Some(Cancelable::Cancel))),
            "Show\n    MinimapIcon 1 Blue Star # icon\n    PlayEffect Red Temp\n    PlayAlertSound None\n"
        );
        let bare = "Show\n    Rarity Rare\n";
        for (intent, line) in [
            (
                StyleIntent::Minimap(Some(Cancelable::Cancel)),
                "MinimapIcon -1",
            ),
            (
                StyleIntent::Beam(Some(Cancelable::Cancel)),
                "PlayEffect None",
            ),
            (
                StyleIntent::Sound(Some(Cancelable::Cancel)),
                "PlayAlertSound None",
            ),
        ] {
            assert_eq!(intent_text(bare, intent), format!("{bare}    {line}\n"));
        }
        // Already the cancel value: nothing to write.
        for (src, intent) in [
            (
                "Show\n    MinimapIcon -1\n",
                StyleIntent::Minimap(Some(Cancelable::Cancel)),
            ),
            (
                "Show\n    PlayEffect None\n",
                StyleIntent::Beam(Some(Cancelable::Cancel)),
            ),
            (
                "Show\n    PlayAlertSound None\n",
                StyleIntent::Sound(Some(Cancelable::Cancel)),
            ),
        ] {
            assert_eq!(intent_text(src, intent), src);
        }
        // Leaving the cancel value writes the full line, volume included.
        assert_eq!(
            intent_text(
                "Show\n    PlayAlertSound None\n",
                StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    id: Some(6),
                    custom: None,
                    vol: 200,
                    positional: false,
                })))
            ),
            "Show\n    PlayAlertSound 6 200\n"
        );
        assert_eq!(
            intent_text(
                "Show\n    PlayEffect None\n",
                StyleIntent::Beam(Some(Cancelable::Set(Beam {
                    color: FilterColor::Pink,
                    temp: false,
                })))
            ),
            "Show\n    PlayEffect Pink\n"
        );
        // Off removes the line, cancel value or not.
        assert_eq!(
            intent_text("Show\n    PlayAlertSound None\n", StyleIntent::Sound(None)),
            "Show\n"
        );
    }

    #[test]
    fn named_and_optional_sounds_stay_leftover_rows() {
        assert_eq!(
            leftover_texts("Show\n    PlayAlertSound ShAlchemy\n"),
            ["PlayAlertSound ShAlchemy"]
        );
        assert_eq!(
            leftover_texts("Show\n    CustomAlertSoundOptional \"x.mp3\"\n"),
            ["CustomAlertSoundOptional \"x.mp3\""]
        );
        // A file edit on an optional line keeps its keyword (never writes a new Optional line).
        assert_eq!(
            intent_text(
                "Show\n    CustomAlertSoundOptional \"x.mp3\"\n",
                StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    id: None,
                    custom: Some("y.mp3".to_string()),
                    vol: 100,
                    positional: false,
                })))
            ),
            "Show\n    CustomAlertSoundOptional \"y.mp3\"\n"
        );
    }

    /// The leftover action cards drawn for the expanded first rule of `wf`.
    fn leftover_rows(ctx: &egui::Context, wf: &WorkingFilter) -> usize {
        let mut list = ListUi {
            expanded: Some(wf.rows()[0].id),
            ..Default::default()
        };
        style_probes(ctx, wf, &mut list, "ed-leftover").len()
    }

    /// D-06: a Custom rule with one alert-sound line draws it on the Sound line only, never as an
    /// extra leftover action row; nor does a sound the editor itself just added.
    #[test]
    fn single_alert_sound_line_has_no_leftover_row() {
        let ctx = headless_ctx();
        for src in [
            "Show\n    PlayAlertSound 6 200\n",
            "Show\n    PlayAlertSoundPositional 6 200\n",
            "Show\n    PlayAlertSound None\n",
            "Show\n    CustomAlertSound \"uwu.mp3\" 300\n",
            "Show\n    PlayAlertSound 6\n",
            "Show\n    CustomAlertSound \"uwu.mp3\"\n",
        ] {
            let (wf, _, e) = entries_of(src);
            let (_, b) = rule_style_view(&e, BlockKeyword::Show);
            assert!(style_leftovers(&e, &b).is_empty(), "{src}");
            assert_eq!(leftover_rows(&ctx, &wf), 0, "{src}");
        }
        // `CustomAlertSoundOptional` is inexact by design: exactly one leftover row, its own.
        let (wf, _, _) = entries_of("Show\n    CustomAlertSoundOptional \"x.mp3\"\n");
        assert_eq!(leftover_rows(&ctx, &wf), 1);
        for positional in [false, true] {
            let mut wf = WorkingFilter::from_text("Show\n    Rarity Rare\n");
            let id = wf.rows()[0].id;
            let (_, b) = rule_style_view(&wf.body_view(id), BlockKeyword::Show);
            let acts = rule_intent_actions(
                id,
                &b,
                StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    id: Some(6),
                    custom: None,
                    vol: 200,
                    positional,
                }))),
            );
            apply_all(&mut wf, acts);
            let e = wf.body_view(id);
            let sounds = e
                .iter()
                .filter(|e| {
                    e.keyword
                        .is_some_and(|k| family_of(k.name) == Some(Family::Sound))
                })
                .count();
            assert_eq!(sounds, 1);
            let (_, b) = rule_style_view(&e, BlockKeyword::Show);
            assert!(style_leftovers(&e, &b).is_empty());
            assert_eq!(leftover_rows(&ctx, &wf), 0);
        }
        // The probe does count a real leftover.
        let wf = WorkingFilter::from_text("Show\n    PlayAlertSound 3\n    PlayAlertSound 6 200\n");
        assert_eq!(leftover_rows(&ctx, &wf), 1);
    }
}

/// The colour dropdown is a skin dropdown (Phase 31): it picks, offers None on request, and leaves
/// the current colour alone. Driven headless; the item names are read from the painted output.
#[cfg(test)]
mod colour_combo_tests {
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, vec2};

    const BUTTON: Pos2 = Pos2::new(72.0, 20.0);

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

    struct Popup {
        /// Painted item names and their rects, below the dropdown box.
        items: Vec<(String, Rect)>,
        picked: Vec<Option<String>>,
    }

    /// Opens the dropdown drawn by `draw` and returns what was painted. `then` events are sent on
    /// the last frame (e.g. a click on an entry).
    fn open(
        draw: &dyn Fn(&mut egui::Ui) -> Option<String>,
        then: impl Fn(&Popup) -> Vec<Event>,
    ) -> Popup {
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        let mut popup = Popup {
            items: Vec::new(),
            picked: Vec::new(),
        };
        let frames = 8;
        for frame in 0..frames {
            let events = match frame {
                1 => click(BUTTON),
                f if f == frames - 1 => then(&popup),
                _ => Vec::new(),
            };
            let mut raw = raw(events);
            raw.time = Some(f64::from(frame) * 0.1);
            let mut picked = None;
            let mut out = ctx.run_ui(raw, |ui| {
                ui.horizontal(|ui| picked = draw(ui));
            });
            out.textures_delta.clear();
            popup.picked.push(picked);
            popup.items.clear();
            for cs in &out.shapes {
                if let Shape::Text(t) = &cs.shape
                    && t.pos.y > 34.0
                {
                    let rect = t.galley.rect.translate(t.pos.to_vec2());
                    popup.items.push((t.galley.text().to_string(), rect));
                }
            }
        }
        popup
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
        let p = open(&|ui| super::colour_combo(ui, "c", "Red", true), |_| {
            Vec::new()
        });
        assert_eq!(p.items.last().map(|(t, _)| t.as_str()), Some("None"));
        let p = open(&|ui| super::colour_combo(ui, "c", "Red", false), |_| {
            Vec::new()
        });
        assert!(!p.items.iter().any(|(t, _)| t == "None"));
        assert!(p.items.iter().any(|(t, _)| t == "Purple"));
    }
}
