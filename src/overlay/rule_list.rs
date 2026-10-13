//! The egui view of the rule list: a thin presentation layer over [`super::rules`].
//!
//! Every mutation is returned as a [`RowAction`] and applied by the caller after the frame; this
//! module never mutates the [`WorkingFilter`]. Rows are virtualized with
//! `ScrollArea::show_viewport` over a cached offset table ([`super::virtual_list`]) because the
//! single expanded row has a variable height (so `show_rows` does not fit).

use std::collections::{HashMap, HashSet};

use egui::{Align, Color32, Layout, RichText, Sense, UiBuilder, vec2};

use crate::filter::keywords::Keyword;
use crate::filter::tokenize::Operator;
use crate::filter::{BlockId, BlockKeyword, FilterDiagnostic};

use super::body::{self, BodyCtx};
use super::classes::ClassCatalog;

use super::rules::{Dir, EconomyView, LineKind, RowAction, RuleRow, StyleState, WorkingFilter};
use super::theme;
use super::virtual_list::{build_offsets, scroll_offset_for, visible_range};
use super::widgets::{self, ArrowDirection};

const CANNOT_ENABLE_LONG: &str = "These lines are not valid filter grammar, so this rule cannot be turned on. You can still move or delete it.";

/// Per-view UI state. Everything keyed by [`BlockId`] must be cleared when ids reset.
#[derive(Debug, Clone)]
pub struct ListUi {
    pub expanded: Option<BlockId>,
    pub confirm_delete: Option<BlockId>,
    pub name_open: HashSet<BlockId>,
    pub name_drafts: HashMap<BlockId, String>,
    /// Last measured height of the expanded row (header plus body).
    pub expanded_h: f32,
    pub pending_scroll: Option<BlockId>,
    pub confirm_economy_remove: bool,
    /// Conditions added from the picker that have no first entry yet (nothing is written).
    pub pending: Vec<PendingRow>,
    /// A just-added line to scroll to and focus.
    pub focus_line: Option<(BlockId, usize)>,
    /// The expanded row of the previous frame, to notice a collapse or switch.
    pub last_expanded: Option<BlockId>,
    /// Refusal of a raw edit, shown under the line it targeted: (block, doc line, message).
    pub line_error: Option<(BlockId, usize, String)>,
}

/// A condition picked from `+ condition` that waits for its first entry (list shapes, Rarity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRow {
    pub block: BlockId,
    pub keyword: &'static Keyword,
    pub op: Option<Operator>,
    pub kind: LineKind,
}

impl Default for ListUi {
    fn default() -> Self {
        ListUi {
            expanded: None,
            confirm_delete: None,
            name_open: HashSet::new(),
            name_drafts: HashMap::new(),
            expanded_h: 180.0,
            pending_scroll: None,
            confirm_economy_remove: false,
            pending: Vec::new(),
            focus_line: None,
            last_expanded: None,
            line_error: None,
        }
    }
}

impl ListUi {
    /// Forget every id-keyed field; called after Revert, Apply re-parse and RELOAD.
    pub fn clear_ids(&mut self) {
        self.expanded = None;
        self.confirm_delete = None;
        self.name_open.clear();
        self.name_drafts.clear();
        self.pending_scroll = None;
        self.confirm_economy_remove = false;
        self.pending.clear();
        self.focus_line = None;
        self.last_expanded = None;
        self.line_error = None;
    }
}

/// What one frame of [`show_list`] produced.
#[derive(Debug, Default)]
pub struct ListFrame {
    pub actions: Vec<RowAction>,
    pub drawn: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EconomyFooterAction {
    Add,
    Remove,
}

/// Fixed height of a collapsed row.
pub fn row_height(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y + 4.0
}

struct Flag {
    text: &'static str,
    color: Color32,
    tip: String,
}

fn diag_tip(diags: &[FilterDiagnostic]) -> String {
    diags
        .iter()
        .map(|d| format!("Line {}: {}", d.line, d.message))
        .collect::<Vec<_>>()
        .join("\n")
}

fn flags_for(row: &RuleRow) -> Vec<Flag> {
    let mut out = Vec::new();
    if !row.enabled && row.cannot_enable {
        out.push(Flag {
            text: "can't enable",
            color: theme::WARM_ACCENT,
            tip: CANNOT_ENABLE_LONG.to_string(),
        });
        return out;
    }
    if row.has_continue {
        out.push(Flag {
            text: "Continue",
            color: theme::LABEL_TAN,
            tip: "Matching goes on to the rules below".to_string(),
        });
    }
    if let StyleState::Known(tag) = &row.style {
        out.push(Flag {
            text: "style wins",
            color: theme::LABEL_TAN,
            tip: format!("Style \"{tag}\" sets this rule's Show/Hide on Apply"),
        });
    }
    if !row.errors.is_empty() {
        out.push(Flag {
            text: "grammar error",
            color: theme::WARM_ACCENT,
            tip: diag_tip(&row.errors),
        });
    }
    if !row.warnings.is_empty() {
        out.push(Flag {
            text: "grammar warning",
            color: theme::LABEL_TAN,
            tip: diag_tip(&row.warnings),
        });
    }
    if row.moved_across_continue {
        out.push(Flag {
            text: "moved across Continue",
            color: theme::WARM_ACCENT,
            tip: "This move changes which rules still apply after a Continue. Check the order."
                .to_string(),
        });
    }
    out
}

/// Draw the virtualized list. `visible` holds indices into `working.rows()`.
pub fn show_list(
    ui: &mut egui::Ui,
    working: &WorkingFilter,
    visible: &[usize],
    list: &mut ListUi,
    reorder_enabled: bool,
    classes: Option<&ClassCatalog>,
) -> ListFrame {
    let row_h = row_height(ui);
    let rows = working.rows();
    let pos_of = |id: BlockId| {
        visible
            .iter()
            .position(|&i| rows.get(i).is_some_and(|r| r.id == id))
    };
    let expanded_vis = list.expanded.and_then(pos_of);
    if list.expanded.is_some() && expanded_vis.is_none() {
        list.expanded = None;
    }
    // A delete confirm only lives while its row is the expanded one; re-opening a row later must
    // not show it already armed.
    if list.confirm_delete.is_some() && list.confirm_delete != list.expanded {
        list.confirm_delete = None;
    }
    // Collapsing or switching rows drops the old row's unwritten conditions, any open picker
    // and its stale refusal message.
    if list.last_expanded != list.expanded {
        let keep = list.expanded;
        list.pending.retain(|p| Some(p.block) == keep);
        list.line_error = None;
        egui::Popup::close_all(ui.ctx());
        list.last_expanded = list.expanded;
    }
    let offsets = build_offsets(
        visible.len(),
        row_h,
        expanded_vis.map(|i| (i, list.expanded_h)),
    );
    let total = offsets.last().copied().unwrap_or(0.0);

    let mut area = egui::ScrollArea::vertical()
        .id_salt("ff-rule-list")
        .auto_shrink([false, false]);
    if let Some(target) = list.pending_scroll.take()
        && let Some(vi) = pos_of(target)
    {
        area = area.vertical_scroll_offset(scroll_offset_for(&offsets, vi, ui.available_height()));
    }

    let mut frame = ListFrame::default();
    area.show_viewport(ui, |ui, viewport| {
        ui.set_height(total);
        let range = visible_range(&offsets, viewport.min.y, viewport.max.y);
        if range.is_empty() {
            return;
        }
        let origin = ui.min_rect().min;
        let width = ui.available_width();
        let rect = egui::Rect::from_min_size(
            egui::pos2(origin.x, origin.y + offsets[range.start]),
            vec2(width, offsets[range.end] - offsets[range.start]),
        );
        ui.skip_ahead_auto_ids(range.start);
        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for vi in range.clone() {
                let Some(row) = visible.get(vi).and_then(|&i| rows.get(i)) else {
                    continue;
                };
                ui.push_id(("rule", row.id), |ui| {
                    draw_row(
                        ui,
                        working,
                        classes,
                        row,
                        vi,
                        row_h,
                        list,
                        reorder_enabled,
                        &mut frame.actions,
                    );
                });
                frame.drawn += 1;
            }
        });
    });
    frame
}

fn draw_row(
    ui: &mut egui::Ui,
    working: &WorkingFilter,
    classes: Option<&ClassCatalog>,
    row: &RuleRow,
    vi: usize,
    row_h: f32,
    list: &mut ListUi,
    reorder_enabled: bool,
    actions: &mut Vec<RowAction>,
) {
    let is_expanded = list.expanded == Some(row.id);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, row_h), Sense::hover());
    let fill = if vi % 2 == 0 {
        theme::ROW_BASE
    } else {
        theme::ROW_ALT
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    let inner = rect.shrink2(vec2(8.0, 0.0));

    // Right side first: arrows, then flags, so the label can take what remains.
    let mut right = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::right_to_left(Align::Center)),
    );
    let down = widgets::arrow_button(
        &mut right,
        ArrowDirection::Down,
        reorder_enabled && row.can_move_down,
    );
    let up = widgets::arrow_button(
        &mut right,
        ArrowDirection::Up,
        reorder_enabled && row.can_move_up,
    );
    if !reorder_enabled {
        down.on_hover_text("Clear the search to reorder");
        up.on_hover_text("Clear the search to reorder");
    } else {
        if down.clicked() {
            actions.push(RowAction::Move(row.id, Dir::Down));
            list.pending_scroll = Some(row.id);
        }
        if up.clicked() {
            actions.push(RowAction::Move(row.id, Dir::Up));
            list.pending_scroll = Some(row.id);
        }
    }
    right.spacing_mut().item_spacing.x = 4.0;
    for flag in flags_for(row).into_iter().rev() {
        right
            .add(egui::Label::new(
                RichText::new(flag.text)
                    .size(theme::ROW_SUMMARY_SIZE)
                    .color(flag.color),
            ))
            .on_hover_text(flag.tip);
    }
    let used_left = right.min_rect().left();

    let left_rect = egui::Rect::from_min_max(
        inner.min,
        egui::pos2((used_left - 4.0).max(inner.min.x), inner.max.y),
    );
    let mut left = ui.new_child(
        UiBuilder::new()
            .max_rect(left_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let mut enabled = row.enabled;
    let toggle = widgets::circular_toggle(&mut left, &mut enabled);
    if toggle.changed() {
        actions.push(RowAction::SetEnabled(row.id, enabled));
    }
    if !row.enabled && row.cannot_enable {
        toggle.on_hover_text("Can't enable: the commented lines are not valid filter grammar.");
    } else {
        toggle.on_hover_text(if row.enabled { "Enabled" } else { "Disabled" });
    }
    left.add_space(6.0);
    let color = if is_expanded {
        theme::SECTION_CREAM
    } else if row.enabled {
        theme::TAB_ACTIVE
    } else {
        theme::DISABLED_TEXT
    };
    let label = left.add(
        egui::Label::new(
            RichText::new(&row.label)
                .size(theme::ROW_SUMMARY_SIZE)
                .color(color),
        )
        .truncate()
        .sense(Sense::click()),
    );
    // The whole space from the label to the flags expands the row, not just the text extent.
    let click_rect = egui::Rect::from_min_max(
        egui::pos2(label.rect.min.x, inner.min.y),
        egui::pos2(left_rect.max.x.max(label.rect.max.x), inner.max.y),
    );
    let row_click = ui.interact(
        click_rect,
        ui.id().with(("ff-row-click", row.id)),
        Sense::click(),
    );
    if label.clicked() || row_click.clicked() {
        list.expanded = if is_expanded { None } else { Some(row.id) };
        list.confirm_delete = None;
    }
    label.on_hover_text(&row.label);
    // The click area sits on top of the label, so it carries the full-label tooltip too.
    row_click.on_hover_text(&row.label);

    if is_expanded {
        let body = egui::Frame::new()
            .fill(theme::PANEL_GROUND)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                expanded_body(ui, working, classes, row, list, actions);
            });
        let h = row_h + body.response.rect.height();
        if (h - list.expanded_h).abs() > 0.5 {
            list.expanded_h = h;
            ui.ctx().request_repaint();
        }
    }
}

fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::LABEL_TAN));
}

fn expanded_body(
    ui: &mut egui::Ui,
    working: &WorkingFilter,
    classes: Option<&ClassCatalog>,
    row: &RuleRow,
    list: &mut ListUi,
    actions: &mut Vec<RowAction>,
) {
    let id = row.id;

    // NAME
    caption(ui, "NAME");
    let mut name_on = row.name.is_some() || list.name_open.contains(&id);
    ui.horizontal(|ui| {
        let toggle = widgets::circular_toggle(ui, &mut name_on);
        let changed = toggle.changed();
        toggle.on_hover_text(if name_on {
            "Remove this rule's name"
        } else {
            "Name this rule"
        });
        if changed {
            if name_on {
                list.name_open.insert(id);
            } else {
                actions.push(RowAction::ClearName(id));
                list.name_open.remove(&id);
                list.name_drafts.remove(&id);
            }
        }
        if !name_on {
            return;
        }
        // Keep the field open while typing: emptying a loaded name clears `row.name`, which
        // would otherwise collapse the editor under the cursor.
        list.name_open.insert(id);
        let draft = list
            .name_drafts
            .entry(id)
            .or_insert_with(|| row.name.clone().unwrap_or_default());
        let resp = ui.add(egui::TextEdit::singleline(draft).desired_width(f32::INFINITY));
        if resp.changed() {
            actions.push(RowAction::SetName(id, draft.clone()));
        }
    });
    ui.add_space(8.0);

    // ACTION
    caption(ui, "ACTION");
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
    let mut cont = row.has_continue;
    let cb = egui::Checkbox::new(
        &mut cont,
        RichText::new("Continue to later rules").color(if locked {
            theme::DISABLED_TEXT
        } else {
            theme::LABEL_TAN
        }),
    );
    if ui.add_enabled(!locked, cb).changed() {
        actions.push(RowAction::SetContinue(id, cont));
    }
    if let StyleState::Known(tag) = &row.style {
        ui.label(
            RichText::new(format!(
                "Style \"{tag}\" decides Show or Hide when you press Apply. Remove the style tag to choose here."
            ))
            .size(theme::ROW_SUMMARY_SIZE)
            .color(theme::TAB_ACTIVE),
        );
    }
    ui.add_space(8.0);

    // CONDITIONS
    let entries = working.body_view(row.id);
    let bctx = BodyCtx {
        row,
        entries: &entries,
        editable: working.body_editable(row.id),
        classes,
        working,
    };
    body::conditions_group(ui, &bctx, list, actions);
    if row.cannot_enable {
        ui.add(
            egui::Label::new(RichText::new(CANNOT_ENABLE_LONG).color(theme::WARM_ACCENT)).wrap(),
        );
    }
    for d in &row.errors {
        ui.add(
            egui::Label::new(
                RichText::new(format!("Line {}: {}", d.line, d.message)).color(theme::WARM_ACCENT),
            )
            .wrap(),
        );
    }
    ui.add_space(8.0);

    // Duplicate / Delete
    ui.horizontal(|ui| {
        if list.confirm_delete == Some(id) {
            ui.label(RichText::new("Delete this rule?").color(theme::LABEL_TAN));
            if ui
                .small_button(RichText::new("Yes, delete").color(theme::WARM_ACCENT))
                .clicked()
            {
                actions.push(RowAction::Delete(id));
                list.confirm_delete = None;
            }
            if ui.small_button("Keep").clicked() {
                list.confirm_delete = None;
            }
        } else {
            if ui.small_button("Duplicate").clicked() {
                actions.push(RowAction::Duplicate(id));
            }
            if ui.small_button("Delete").clicked() {
                list.confirm_delete = Some(id);
            }
        }
    });
}

/// Full-width search field with a Clear button; true when the query changed this frame.
pub fn search_bar(ui: &mut egui::Ui, query: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let reserve = if query.is_empty() { 0.0 } else { 56.0 };
        let resp = ui.add(
            egui::TextEdit::singleline(query)
                .hint_text("Search rules, classes, base types\u{2026}")
                .desired_width((ui.available_width() - reserve).max(40.0)),
        );
        if resp.changed() {
            changed = true;
        }
        if (resp.has_focus() || resp.lost_focus())
            && !query.is_empty()
            && ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            query.clear();
            changed = true;
        }
        if !query.is_empty() && ui.small_button("Clear").clicked() {
            query.clear();
            changed = true;
        }
    });
    changed
}

/// The footer line for an economy state.
pub fn economy_footer_text(view: &EconomyView) -> String {
    match view {
        EconomyView::Present { blocks: 1 } => "+ 1 locked rule block (economy)".to_string(),
        EconomyView::Present { blocks } => format!("+ {blocks} locked rule blocks (economy)"),
        EconomyView::Absent => "No economy section".to_string(),
        EconomyView::Damaged(_) => "Economy markers are damaged".to_string(),
    }
}

/// The pinned economy footer row. `enabled` gates the buttons.
pub fn economy_footer(
    ui: &mut egui::Ui,
    view: &EconomyView,
    list: &mut ListUi,
    enabled: bool,
) -> Option<EconomyFooterAction> {
    let mut result = None;
    let text = economy_footer_text(view);
    egui::Frame::new()
        .fill(theme::HEADER_PLATE)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            match view {
                EconomyView::Present { .. } => {
                    ui.horizontal(|ui| {
                        if list.confirm_economy_remove {
                            ui.label(
                                RichText::new("Remove the economy section?")
                                    .color(theme::LABEL_TAN),
                            );
                            if ui
                                .small_button(RichText::new("Yes, remove").color(theme::WARM_ACCENT))
                                .clicked()
                            {
                                result = Some(EconomyFooterAction::Remove);
                                list.confirm_economy_remove = false;
                            }
                            if ui.small_button("Keep").clicked() {
                                list.confirm_economy_remove = false;
                            }
                        } else {
                            ui.label(RichText::new(&text).color(theme::LABEL_TAN))
                                .on_hover_text(
                                    "Generated from prices.json by SYNC ECONOMY and APPLY. Not editable here.",
                                );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add_enabled(enabled, egui::Button::new("Remove").small())
                                    .clicked()
                                {
                                    list.confirm_economy_remove = true;
                                }
                            });
                        }
                    });
                    if list.confirm_economy_remove {
                        ui.label(
                            RichText::new("Your own rules are kept. Add the section back any time.")
                                .size(theme::ROW_SUMMARY_SIZE)
                                .color(theme::TAB_ACTIVE),
                        );
                    }
                }
                EconomyView::Absent => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&text).color(theme::DISABLED_TEXT));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add_enabled(enabled, egui::Button::new("Add economy section").small())
                                .clicked()
                            {
                                result = Some(EconomyFooterAction::Add);
                            }
                        });
                    });
                }
                EconomyView::Damaged(msg) => {
                    ui.label(RichText::new(&text).color(theme::WARM_ACCENT))
                        .on_hover_text(msg);
                }
            }
        });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

    fn headless_ctx() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.set_fonts(egui::FontDefinitions::empty());
        ctx
    }

    fn input(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(460.0, 600.0))),
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

    fn filter_of(n: usize) -> WorkingFilter {
        WorkingFilter::from_text(&"Show\n    Class \"Rings\"\n\n".repeat(n))
    }

    fn run_frame(
        ctx: &egui::Context,
        raw: RawInput,
        wf: &WorkingFilter,
        list: &mut ListUi,
        reorder: bool,
    ) -> (ListFrame, f32) {
        let visible: Vec<usize> = (0..wf.rows().len()).collect();
        let mut out = (ListFrame::default(), 0.0);
        ctx.run_ui(raw, |ui| {
            let h = row_height(ui);
            let area = Rect::from_min_size(Pos2::ZERO, vec2(460.0, 600.0));
            let mut child = ui.new_child(UiBuilder::new().max_rect(area));
            out = (show_list(&mut child, wf, &visible, list, reorder, None), h);
        })
        .drop_without_applying_deltas();
        out
    }

    #[test]
    fn only_visible_rows_are_drawn_for_511_rules() {
        let wf = filter_of(511);
        assert_eq!(wf.rows().len(), 511);
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        let (frame, row_h) = run_frame(&ctx, input(vec![]), &wf, &mut list, true);
        assert!(frame.drawn > 0);
        assert!(
            frame.drawn <= (600.0 / row_h).ceil() as usize + 2,
            "drew {} rows",
            frame.drawn
        );
    }

    #[test]
    fn arrows_do_nothing_when_reorder_is_disabled_and_work_when_enabled() {
        let wf = filter_of(5);
        let sweep = |reorder: bool| -> bool {
            let ctx = headless_ctx();
            let mut list = ListUi::default();
            let (_, row_h) = run_frame(&ctx, input(vec![]), &wf, &mut list, reorder);
            let y = row_h / 2.0;
            let mut any_move = false;
            let mut x = 300.0;
            while x < 458.0 {
                let (frame, _) = run_frame(
                    &ctx,
                    input(click_events(Pos2::new(x, y))),
                    &wf,
                    &mut list,
                    reorder,
                );
                any_move |= frame
                    .actions
                    .iter()
                    .any(|a| matches!(a, RowAction::Move(..)));
                x += 3.0;
            }
            any_move
        };
        assert!(!sweep(false), "disabled reorder must not emit Move");
        assert!(
            sweep(true),
            "enabled reorder must emit Move somewhere on the row"
        );
    }

    #[test]
    fn clear_ids_resets_every_id_keyed_field() {
        let wf = filter_of(1);
        let id = wf.rows()[0].id;
        let mut list = ListUi {
            expanded: Some(id),
            confirm_delete: Some(id),
            pending_scroll: Some(id),
            confirm_economy_remove: true,
            ..Default::default()
        };
        list.pending.push(PendingRow {
            block: id,
            keyword: crate::filter::keywords::lookup("Class").unwrap(),
            op: None,
            kind: LineKind::Condition,
        });
        list.focus_line = Some((id, 1));
        list.last_expanded = Some(id);
        list.line_error = Some((id, 1, "x".into()));
        list.name_open.insert(id);
        list.name_drafts.insert(id, "x".into());
        list.clear_ids();
        assert!(list.expanded.is_none());
        assert!(list.confirm_delete.is_none());
        assert!(list.name_open.is_empty());
        assert!(list.name_drafts.is_empty());
        assert!(list.pending_scroll.is_none());
        assert!(!list.confirm_economy_remove);
        assert!(list.pending.is_empty());
        assert!(list.focus_line.is_none());
        assert!(list.last_expanded.is_none());
        assert!(list.line_error.is_none());
    }

    #[test]
    fn economy_footer_text_strings() {
        assert_eq!(
            economy_footer_text(&EconomyView::Present { blocks: 1 }),
            "+ 1 locked rule block (economy)"
        );
        assert_eq!(
            economy_footer_text(&EconomyView::Present { blocks: 12 }),
            "+ 12 locked rule blocks (economy)"
        );
        assert_eq!(
            economy_footer_text(&EconomyView::Absent),
            "No economy section"
        );
        assert_eq!(
            economy_footer_text(&EconomyView::Damaged("x".into())),
            "Economy markers are damaged"
        );
    }

    #[test]
    fn expanded_style_tagged_row_renders_without_actions() {
        let wf = WorkingFilter::from_text(
            "Show # x\n    #ff:style S\n    Class \"Rings\"\n\nShow\n    Class \"Belts\"\n\n",
        );
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[0].id);
        for _ in 0..2 {
            let (frame, _) = run_frame(&ctx, input(vec![]), &wf, &mut list, true);
            assert!(
                !frame
                    .actions
                    .iter()
                    .any(|a| matches!(a, RowAction::SetKeyword(..)))
            );
        }
    }

    #[test]
    fn economy_footer_renders_every_state() {
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        for view in [
            EconomyView::Absent,
            EconomyView::Present { blocks: 3 },
            EconomyView::Damaged("bad".into()),
        ] {
            ctx.run_ui(input(vec![]), |ui| {
                assert!(economy_footer(ui, &view, &mut list, true).is_none());
            })
            .drop_without_applying_deltas();
        }
    }
}
