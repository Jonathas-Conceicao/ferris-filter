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
use crate::filter::{BlockId, FilterDiagnostic};

use super::body::{self, BodyCtx};
use super::classes::ClassCatalog;

use super::rules::{Dir, EconomyView, LineKind, RowAction, RuleRow, StyleState, WorkingFilter};
use super::theme;
use super::virtual_list::{build_offsets, scroll_offset_for, visible_range};
use super::widgets::{self, ArrowDirection};
use crate::skin::Skin;
use crate::skin::fonts::regular;
use crate::skin::palette as pal;
use crate::skin::widgets::{ff_button_ex, ff_text_field};

const GROUP_BROKEN_LONG: &str = "This block was written as part of one rule spread over several blocks (# ff:group), but those blocks no longer match, so each is shown as a rule of its own. It is left as written until you edit it.";
const CANNOT_ENABLE_LONG: &str = "These lines are not valid filter grammar, so this rule cannot be turned on. You can still move or delete it.";

/// Per-view UI state. Everything keyed by [`BlockId`] must be cleared when ids reset.
#[derive(Debug, Clone)]
pub struct ListUi {
    pub expanded: Option<BlockId>,
    pub confirm_delete: Option<BlockId>,
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
    /// Salt of every row's egui id scope. Card state that lives in egui temp memory (bad-text and
    /// bad-int flags, the `Named` toggle, raw drafts) is keyed by `BlockId` and keyword ordinal,
    /// both of which can be reused by a different line after a re-parse or a removal; bumping the
    /// epoch orphans that state so it never shows up on another card (WR-07).
    pub epoch: u64,
    /// Bumped only when the expanded row changes or ids reset (not on every line add or remove).
    /// Open/closed state of the class groups lives under this instead of [`ListUi::epoch`], so
    /// the first value click, which turns a pending row into a line and bumps `epoch`, does not
    /// close the group being used.
    pub row_epoch: u64,
    /// Blocks whose Defences card was picked from `+ condition` with nothing toggled yet
    /// (nothing is written until a toggle is switched on).
    pub pending_defences: HashSet<BlockId>,
    /// Blocks whose Simple Defences card was picked from `+ condition` with every row on Any
    /// (nothing is written until a row is set to Has or No).
    pub pending_simple_defences: HashSet<BlockId>,
    /// Pending Simple Defences cards whose pick was every combination (which, like no pick,
    /// writes no defence line), so the card keeps showing every chip selected.
    pub simple_all: HashSet<BlockId>,
    /// Blocks that show the Defences card (picked, or rendered once), so zero-based lines written
    /// by it (`BaseArmour >= 1`) never turn it into a Simple Defences card while editing. Carried
    /// across an Apply by row position ([`ListUi::rekey_after_apply`]); cleared by
    /// [`ListUi::clear_ids`] on Revert and RELOAD (the file is classified again), by either card's
    /// remove and when Simple Defences is picked.
    pub advanced_defences: HashSet<BlockId>,
    /// A card to scroll to, focus and briefly highlight: set when a picker entry that is already
    /// on the rule is clicked (UAT3-06). Line-backed keyword cards use `focus_line` instead.
    pub focus_card: Option<(BlockId, FocusTarget)>,
}

/// A card without a line of its own to point at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    /// The pending row of a keyword (by keyword name).
    Pending(&'static str),
    /// The Defences card.
    Defences,
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
            name_drafts: HashMap::new(),
            expanded_h: 180.0,
            pending_scroll: None,
            confirm_economy_remove: false,
            pending: Vec::new(),
            focus_line: None,
            last_expanded: None,
            line_error: None,
            epoch: 0,
            row_epoch: 0,
            pending_defences: HashSet::new(),
            pending_simple_defences: HashSet::new(),
            simple_all: HashSet::new(),
            advanced_defences: HashSet::new(),
            focus_card: None,
        }
    }
}

impl ListUi {
    /// Forget every id-keyed field; called after Revert and RELOAD, and by
    /// [`ListUi::rekey_after_apply`] after the Apply re-parse.
    pub fn clear_ids(&mut self) {
        self.expanded = None;
        self.confirm_delete = None;
        self.name_drafts.clear();
        self.pending_scroll = None;
        self.confirm_economy_remove = false;
        self.pending.clear();
        self.pending_defences.clear();
        self.pending_simple_defences.clear();
        self.simple_all.clear();
        self.advanced_defences.clear();
        self.focus_card = None;
        self.focus_line = None;
        self.last_expanded = None;
        self.line_error = None;
        self.row_epoch = self.row_epoch.wrapping_add(1);
        self.forget_card_state();
    }

    /// Orphan every card's egui temp state (see [`ListUi::epoch`]).
    pub fn forget_card_state(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
    }

    /// The row positions that must survive an Apply re-parse (ids reset, positions do not):
    /// the expanded row and the rows marked [`ListUi::advanced_defences`].
    pub fn kept_positions(&self, working: &WorkingFilter) -> KeptPositions {
        let index = |id: &BlockId| working.row(*id).map(|r| r.index);
        let mut advanced_defences: Vec<usize> =
            self.advanced_defences.iter().filter_map(index).collect();
        advanced_defences.sort_unstable();
        KeptPositions {
            expanded: self.expanded.as_ref().and_then(index),
            advanced_defences,
        }
    }

    /// After a successful Apply: forget the old ids ([`ListUi::clear_ids`]), then re-key the
    /// expanded row and the Defences markers to the re-parsed rows by position. Apply is not a
    /// reload: the open row stays open and a Defences card does not turn into Simple Defences
    /// under the user (only Revert, RELOAD or removing the card classify from the file again).
    pub fn rekey_after_apply(&mut self, kept: KeptPositions, working: Option<&WorkingFilter>) {
        self.clear_ids();
        let Some(working) = working else {
            return;
        };
        let id_at = |i: usize| {
            i.checked_sub(1)
                .and_then(|i| working.rows().get(i))
                .map(|r| r.id)
        };
        self.expanded = kept.expanded.and_then(id_at);
        self.advanced_defences = kept
            .advanced_defences
            .into_iter()
            .filter_map(id_at)
            .collect();
    }
}

/// Row positions (1-based, [`RuleRow::index`]) carried across an Apply re-parse; see
/// [`ListUi::kept_positions`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeptPositions {
    pub expanded: Option<usize>,
    pub advanced_defences: Vec<usize>,
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

/// Height of a row's header line (the part that stays when the row is expanded).
pub fn header_height(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y + 4.0
}

/// Height of the one-line condition summary under a collapsed row's header.
pub const SUMMARY_LINE_H: f32 = 16.0;

/// Left indent of the summary line, lining it up under the rule label (past the toggle).
const SUMMARY_INDENT: f32 = 40.0;

/// Fixed height of a collapsed row: header plus the summary line.
pub fn row_height(ui: &egui::Ui) -> f32 {
    header_height(ui) + SUMMARY_LINE_H
}

struct Flag {
    text: String,
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

/// Characters of a style name shown in the collapsed-row flag.
const STYLE_FLAG_CHARS: usize = 12;
/// Width reserved for the `+N` overflow badge.
const BADGE_W: f32 = 24.0;
/// Total width flags may take in a collapsed row.
const FLAG_BUDGET: f32 = 120.0;
/// The label always keeps at least this much.
const LABEL_MIN_W: f32 = 140.0;

/// How many leading flags fit in `budget` (with a `+N` badge when some do not), and how many
/// are hidden.
pub fn fit_flags(widths: &[f32], budget: f32) -> (usize, usize) {
    let total: f32 = widths.iter().sum();
    if total <= budget {
        return (widths.len(), 0);
    }
    let mut shown = 0;
    let mut used = BADGE_W;
    for w in widths {
        if used + w > budget {
            break;
        }
        used += w;
        shown += 1;
    }
    (shown, widths.len() - shown)
}

fn flags_for(row: &RuleRow) -> Vec<Flag> {
    let mut out = Vec::new();
    if !row.enabled && row.cannot_enable {
        out.push(Flag {
            text: "can't enable".to_string(),
            color: theme::WARM_ACCENT_TEXT,
            tip: CANNOT_ENABLE_LONG.to_string(),
        });
        return out;
    }
    if row.has_continue {
        out.push(Flag {
            text: "Continue".to_string(),
            color: theme::LABEL_TAN,
            tip: "Matching goes on to the rules below".to_string(),
        });
    }
    match &row.style {
        StyleState::Known(tag) => {
            let short: String = if tag.chars().count() > STYLE_FLAG_CHARS {
                let head: String = tag.chars().take(STYLE_FLAG_CHARS).collect();
                format!("{head}…")
            } else {
                tag.clone()
            };
            out.push(Flag {
                text: format!("styled: {short}"),
                color: theme::LABEL_TAN,
                tip: format!("Style \"{tag}\" sets this rule's actions on Apply"),
            });
        }
        StyleState::Unknown(_) => out.push(Flag {
            text: "unknown style".to_string(),
            color: theme::WARM_ACCENT_TEXT,
            tip: "No style with this name exists. This rule keeps its current actions, and its style link is removed on the next Apply.".to_string(),
        }),
        StyleState::None | StyleState::Unchecked(_) => {}
    }
    if !row.errors.is_empty() {
        out.push(Flag {
            text: "grammar error".to_string(),
            color: theme::WARM_ACCENT_TEXT,
            tip: diag_tip(&row.errors),
        });
    }
    if !row.warnings.is_empty() {
        out.push(Flag {
            text: "grammar warning".to_string(),
            color: theme::LABEL_TAN,
            tip: diag_tip(&row.warnings),
        });
    }
    if row.group_broken {
        out.push(Flag {
            text: "group broken".to_string(),
            color: theme::WARM_ACCENT_TEXT,
            tip: GROUP_BROKEN_LONG.to_string(),
        });
    }
    if row.moved_across_continue {
        out.push(Flag {
            text: "moved across Continue".to_string(),
            color: theme::WARM_ACCENT_TEXT,
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
        list.pending_defences.retain(|b| Some(*b) == keep);
        list.pending_simple_defences.retain(|b| Some(*b) == keep);
        list.simple_all.retain(|b| Some(*b) == keep);
        list.focus_card = None;
        list.line_error = None;
        egui::Popup::close_all(ui.ctx());
        list.last_expanded = list.expanded;
        list.row_epoch = list.row_epoch.wrapping_add(1);
        list.forget_card_state();
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
                ui.push_id(("rule", row.id, list.epoch), |ui| {
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
    // The virtual list's offsets assume a row spans exactly `row_h` (collapsed) or
    // `list.expanded_h` (expanded). That only holds with no spacing between the stacked
    // allocations, so the row enforces it itself instead of relying on the caller.
    ui.spacing_mut().item_spacing.y = 0.0;
    let is_expanded = list.expanded == Some(row.id);
    let width = ui.available_width();
    let header_h = header_height(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(width, header_h), Sense::hover());
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
    let flags = flags_for(row);
    let font = egui::FontId::proportional(theme::ROW_SUMMARY_SIZE);
    let widths: Vec<f32> = flags
        .iter()
        .map(|f| {
            ui.fonts_mut(|fo| {
                fo.layout_no_wrap(f.text.clone(), font.clone(), Color32::WHITE)
                    .size()
                    .x
            }) + 4.0
        })
        .collect();
    let arrows_w = right.min_rect().width();
    let budget = FLAG_BUDGET.min((inner.width() - arrows_w - 24.0 - 8.0 - LABEL_MIN_W).max(0.0));
    let (shown, hidden) = fit_flags(&widths, budget);
    if hidden > 0 {
        let tip = flags
            .iter()
            .map(|f| format!("{}: {}", f.text, f.tip))
            .collect::<Vec<_>>()
            .join(
                "
",
            );
        right
            .add(egui::Label::new(
                RichText::new(format!("+{hidden}"))
                    .size(theme::ROW_SUMMARY_SIZE)
                    .color(theme::LABEL_TAN),
            ))
            .on_hover_text(tip);
    }
    for flag in flags.into_iter().take(shown).rev() {
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
    left.add_space(8.0);
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

    if !is_expanded {
        // The summary comes from the row (built once at rebuild); only the paint happens here.
        debug_assert!((row_h - header_h - SUMMARY_LINE_H).abs() < 0.5);
        let (srect, _) = ui.allocate_exact_size(vec2(width, SUMMARY_LINE_H), Sense::hover());
        ui.painter().rect_filled(srect, 0.0, fill);
        if !row.summary.is_empty() {
            let color = if row.enabled {
                theme::LABEL_TAN
            } else {
                theme::DISABLED_TEXT
            };
            let mut sui = ui.new_child(
                UiBuilder::new()
                    .max_rect(egui::Rect::from_min_max(
                        egui::pos2(srect.min.x + SUMMARY_INDENT, srect.min.y),
                        egui::pos2(srect.max.x - 8.0, srect.max.y),
                    ))
                    .layout(Layout::left_to_right(Align::Center)),
            );
            sui.add(
                egui::Label::new(
                    RichText::new(&row.summary)
                        .size(theme::ROW_SUMMARY_SIZE)
                        .color(color),
                )
                .truncate()
                .selectable(false),
            );
        }
        // Registered after the label so it is the topmost hit: a click on the text expands the
        // row instead of starting a text selection (same pattern as the header's `row_click`).
        let sresp = ui.interact(
            srect,
            ui.id().with(("ff-summary-click", row.id)),
            Sense::click(),
        );
        if !row.summary.is_empty() {
            sresp.clone().on_hover_text(&row.summary);
        }
        if sresp.clicked() {
            list.expanded = Some(row.id);
            list.confirm_delete = None;
        }
    }

    if is_expanded {
        let body = egui::Frame::new()
            .fill(theme::PANEL_GROUND)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                expanded_body(ui, working, classes, row, list, actions);
            });
        let h = header_h + body.response.rect.height();
        if (h - list.expanded_h).abs() > 0.5 {
            list.expanded_h = h;
            ui.ctx().request_repaint();
        }
    }
}

/// A row-action button at the 13px body-small size (UI-SPEC: 11px is for captions only).
fn small_button(ui: &mut egui::Ui, text: impl Into<RichText>) -> egui::Response {
    ui.add(egui::Button::new(text.into().size(theme::ROW_SUMMARY_SIZE)))
}

fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::LABEL_TAN));
}

/// What the name box emits for its current text: an empty box removes the `# ff:name` line.
fn name_action(id: BlockId, draft: &str) -> RowAction {
    if draft.trim().is_empty() {
        RowAction::ClearName(id)
    } else {
        RowAction::SetName(id, draft.to_string())
    }
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
    // The draft outlives `row.name`: emptying the box clears the meta line, and the box must
    // keep its focus while the author retypes.
    let draft = list
        .name_drafts
        .entry(id)
        .or_insert_with(|| row.name.clone().unwrap_or_default());
    let resp = ui.add(
        egui::TextEdit::singleline(draft)
            .hint_text("name")
            .desired_width(f32::INFINITY),
    );
    if resp.changed() {
        actions.push(name_action(id, draft));
    }
    ui.add_space(8.0);

    let entries = working.body_view(row.id);
    let bctx = BodyCtx {
        row,
        entries: &entries,
        editable: working.body_editable(row.id),
        classes,
        working,
    };

    // CONDITIONS
    body::conditions_group(ui, &bctx, list, actions);
    if row.cannot_enable {
        ui.add(
            egui::Label::new(RichText::new(CANNOT_ENABLE_LONG).color(theme::WARM_ACCENT_TEXT))
                .wrap(),
        );
    }
    for d in &row.errors {
        ui.add(
            egui::Label::new(
                RichText::new(format!("Line {}: {}", d.line, d.message))
                    .color(theme::WARM_ACCENT_TEXT),
            )
            .wrap(),
        );
    }
    ui.add_space(16.0);

    // STYLE (Custom or preset, with its actions), last before Duplicate/Delete
    body::style_section(ui, &bctx, list, actions);
    ui.add_space(8.0);

    // Duplicate / Delete
    ui.horizontal(|ui| {
        if list.confirm_delete == Some(id) {
            ui.label(RichText::new("Delete this rule?").color(theme::LABEL_TAN));
            if small_button(
                ui,
                RichText::new("Yes, delete").color(theme::WARM_ACCENT_TEXT),
            )
            .clicked()
            {
                actions.push(RowAction::Delete(id));
                list.confirm_delete = None;
            }
            if small_button(ui, "Keep").clicked() {
                list.confirm_delete = None;
            }
        } else {
            if small_button(ui, "Duplicate").clicked() {
                actions.push(RowAction::Duplicate(id));
            }
            if small_button(ui, "Delete").clicked() {
                list.confirm_delete = Some(id);
            }
        }
    });
}

/// The search row of the RULES tab: the recessed field (hint names the rule count), a small Clear
/// button while a query is typed, and the `+ Add rule` button. Escape clears the query. Returns
/// true when `+ Add rule` was clicked.
pub fn search_row(
    ui: &mut egui::Ui,
    skin: &Skin,
    query: &mut String,
    add_enabled: bool,
    total: usize,
) -> bool {
    let mut add_clicked = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let reserve = if query.is_empty() { 0.0 } else { 56.0 };
        let hint = format!("Search {total} rules, classes, base types…");
        let width = (ui.available_width() - 132.0 - reserve).max(40.0);
        let resp = ff_text_field(ui, query, &hint, width);
        if (resp.has_focus() || resp.lost_focus())
            && !query.is_empty()
            && ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            query.clear();
        }
        if !query.is_empty() && small_button(ui, "Clear").clicked() {
            query.clear();
        }
        let add = ff_button_ex(ui, skin, "+ Add rule", 124.0, add_enabled, false)
            .on_hover_text(ADD_RULE_TIP);
        add_clicked = add_enabled && add.clicked();
    });
    add_clicked
}

/// Tooltip on `+ Add rule` (it replaces the standing empty-state paragraph).
pub const ADD_RULE_TIP: &str = "Blocks you add are saved with the filter when you press Apply.";

/// The footer line for an economy state.
pub fn economy_footer_text(view: &EconomyView) -> String {
    match view {
        EconomyView::Present { blocks: 1 } => "+ 1 locked rule block (economy)".to_string(),
        EconomyView::Present { blocks } => format!("+ {blocks} locked rule blocks (economy)"),
        EconomyView::Absent => "No economy section".to_string(),
        EconomyView::Damaged(_) => "Economy markers are damaged".to_string(),
    }
}

/// Height of the pinned economy footer row.
pub const ECONOMY_ROW_HEIGHT: f32 = 34.0;

/// The pinned economy footer row (34pt, the row ground). `enabled` gates the buttons.
pub fn economy_footer(
    ui: &mut egui::Ui,
    skin: &Skin,
    view: &EconomyView,
    list: &mut ListUi,
    enabled: bool,
) -> Option<EconomyFooterAction> {
    let mut result = None;
    let text = economy_footer_text(view);
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), ECONOMY_ROW_HEIGHT),
        Sense::hover(),
    );
    ui.painter().rect_filled(rect, 0.0, pal::ROW_EVEN);
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink2(vec2(10.0, 0.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    let label = |text: &str, color: Color32| RichText::new(text).font(regular(15.0)).color(color);
    match view {
        EconomyView::Present { .. } => {
            if list.confirm_economy_remove {
                row.label(label("Remove the economy section?", pal::LABEL));
                if small_button(&mut row, RichText::new("Yes, remove").color(pal::ERROR)).clicked()
                {
                    result = Some(EconomyFooterAction::Remove);
                    list.confirm_economy_remove = false;
                }
                if small_button(&mut row, "Keep").clicked() {
                    list.confirm_economy_remove = false;
                }
            } else {
                row.label(label(&text, pal::MUTED)).on_hover_text(
                    "Generated from prices.json by Sync economy and Apply. Not editable here.",
                );
                row.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ff_button_ex(ui, skin, "Remove", 84.0, enabled, false).clicked() {
                        list.confirm_economy_remove = true;
                    }
                });
            }
        }
        EconomyView::Absent => {
            row.label(label(&text, pal::DISABLED));
            row.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ff_button_ex(ui, skin, "Add economy section", 160.0, enabled, false).clicked() {
                    result = Some(EconomyFooterAction::Add);
                }
            });
        }
        EconomyView::Damaged(msg) => {
            row.label(label(&text, pal::ERROR)).on_hover_text(msg);
        }
    }
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

    fn styled_filter(tag: &str) -> WorkingFilter {
        let src = format!(
            "#ff:style {tag}
Show # x
    Class \"Rings\"
    Continue

Show
    Class \"Belts\"

"
        );
        let mut wf = WorkingFilter::from_text(&src);
        wf.set_styles(Ok(crate::loader::load_styles(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/styles-default.json"
        ))
        .unwrap()));
        wf
    }

    #[test]
    fn broken_group_blocks_carry_the_flag() {
        let wf = WorkingFilter::from_text(
            "# ff:group abc 1/2\nShow\n  BaseArmour > 0\n\n# ff:group abc 2/2\nHide\n  BaseArmour 0\n\nShow\n",
        );
        assert_eq!(wf.rows().len(), 3);
        for r in &wf.rows()[..2] {
            let f = flags_for(r);
            let g = f.iter().find(|f| f.text == "group broken").expect("flag");
            assert_eq!(g.color, theme::WARM_ACCENT_TEXT);
            assert!(g.tip.contains("# ff:group"));
        }
        assert!(
            !flags_for(&wf.rows()[2])
                .iter()
                .any(|f| f.text == "group broken")
        );
    }

    #[test]
    fn style_flags_known_truncated_unknown_and_none() {
        let wf = styled_filter("S");
        let f = flags_for(&wf.rows()[0]);
        assert!(f.iter().any(|f| f.text == "styled: S"));
        let wf = styled_filter("Nope");
        let f = flags_for(&wf.rows()[0]);
        let u = f.iter().find(|f| f.text == "unknown style").unwrap();
        assert_eq!(u.color, theme::WARM_ACCENT_TEXT);
        assert!(u.tip.starts_with("No style with this name exists"));
        assert!(
            !flags_for(&wf.rows()[1])
                .iter()
                .any(|f| f.text.contains("style"))
        );
        let mut row = wf.rows()[0].clone();
        row.style = StyleState::Known("Exalted Showcase".into());
        let f = flags_for(&row);
        let k = f.iter().find(|f| f.text.starts_with("styled:")).unwrap();
        assert_eq!(k.text, "styled: Exalted Show…");
        assert!(k.tip.contains("Exalted Showcase"));
        row.style = StyleState::Unchecked("S".into());
        assert!(!flags_for(&row).iter().any(|f| f.text.contains("style")));
    }

    #[test]
    fn fit_flags_collapses_overflow_into_a_badge() {
        assert_eq!(fit_flags(&[50.0, 50.0, 50.0], 120.0), (1, 2));
        assert_eq!(fit_flags(&[40.0, 40.0], 120.0), (2, 0));
    }

    #[test]
    fn crowded_row_renders_with_overflow_badge() {
        let mut wf = styled_filter("Nope");
        let _ = &mut wf;
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        for _ in 0..2 {
            let (frame, _) = run_frame(&ctx, input(vec![]), &wf, &mut list, true);
            assert!(frame.drawn >= 1);
        }
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
    fn collapsed_row_height_includes_summary_line() {
        let ctx = headless_ctx();
        let mut hs = (0.0, 0.0);
        ctx.run_ui(input(vec![]), |ui| {
            hs = (row_height(ui), header_height(ui));
        })
        .drop_without_applying_deltas();
        assert_eq!(hs.0, hs.1 + SUMMARY_LINE_H);
    }

    #[test]
    fn expanded_row_height_excludes_summary() {
        let wf = filter_of(3);
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[1].id);
        let mut header = 0.0;
        for _ in 0..3 {
            ctx.run_ui(input(vec![]), |ui| {
                header = header_height(ui);
            })
            .drop_without_applying_deltas();
            run_frame(&ctx, input(vec![]), &wf, &mut list, true);
        }
        assert!(list.expanded_h > header, "{} <= {header}", list.expanded_h);
        let (_, row_h) = run_frame(&ctx, input(vec![]), &wf, &mut list, true);
        let offsets = build_offsets(3, row_h, Some((1, list.expanded_h)));
        assert_eq!(
            offsets.last().copied().unwrap(),
            2.0 * row_h + list.expanded_h
        );
    }

    /// WR-03 (review 24): the height `draw_row` actually allocates matches what the virtual list
    /// assumes: `row_height` collapsed, `list.expanded_h` expanded. Drawn in a plain UI with the
    /// default (non-zero) item spacing, so it does not lean on `show_list` zeroing it.
    #[test]
    fn painted_row_heights_match_the_virtual_list() {
        let wf = filter_of(2);
        let ctx = headless_ctx();
        let mut list = ListUi {
            expanded: Some(wf.rows()[1].id),
            ..Default::default()
        };
        let measure = |list: &mut ListUi, i: usize| -> (f32, f32) {
            let mut out = (0.0, 0.0);
            ctx.run_ui(input(vec![]), |ui| {
                assert!(ui.spacing().item_spacing.y > 0.0, "test needs real spacing");
                let row_h = row_height(ui);
                let mut actions = Vec::new();
                let drawn = ui.push_id(("rule", i), |ui| {
                    draw_row(
                        ui,
                        &wf,
                        None,
                        &wf.rows()[i],
                        i,
                        row_h,
                        list,
                        true,
                        &mut actions,
                    );
                });
                out = (drawn.response.rect.height(), row_h);
            })
            .drop_without_applying_deltas();
            out
        };
        let (collapsed, row_h) = measure(&mut list, 0);
        assert!((collapsed - row_h).abs() < 0.5, "{collapsed} vs {row_h}");
        // The expanded height settles after a frame or two (it is measured, then stored).
        for _ in 0..3 {
            measure(&mut list, 1);
        }
        let (expanded, _) = measure(&mut list, 1);
        assert!(list.expanded_h > row_h);
        assert!(
            (expanded - list.expanded_h).abs() < 0.5,
            "{expanded} vs {}",
            list.expanded_h
        );
    }

    /// WR-01 (review 24): a click on the summary *text* expands the row. The label is laid out
    /// with real fonts (not `headless_ctx`'s empty set) so the text actually covers the point.
    #[test]
    fn clicking_the_summary_text_expands_the_row() {
        let wf = filter_of(3);
        assert!(!wf.rows()[0].summary.is_empty());
        let ctx = egui::Context::default();
        let mut list = ListUi::default();
        let (_, row_h) = run_frame(&ctx, input(vec![]), &wf, &mut list, true);
        let mut header = 0.0;
        ctx.run_ui(input(vec![]), |ui| header = header_height(ui))
            .drop_without_applying_deltas();
        let click = |list: &mut ListUi, pos: Pos2| {
            // Hover first so the press lands on last frame's hit-test, then press and release.
            for events in click_events(pos).into_iter().map(|e| vec![e]) {
                run_frame(&ctx, input(events), &wf, list, true);
            }
        };
        let y = row_h + header + SUMMARY_LINE_H / 2.0;
        // Control: the blank area far past the text has always expanded the row.
        click(&mut list, Pos2::new(440.0, y));
        assert_eq!(list.expanded, Some(wf.rows()[1].id), "blank-area click");
        list.expanded = None;
        run_frame(&ctx, input(vec![]), &wf, &mut list, true);
        // Second row, a few pixels into the summary text (past the indent).
        click(&mut list, Pos2::new(SUMMARY_INDENT + 12.0, y));
        assert_eq!(list.expanded, Some(wf.rows()[1].id), "summary-text click");
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

    /// WR-07: collapsing or switching the expanded row orphans the old row's card state, so a
    /// stale hint or `Named` mode does not come back when it is reopened.
    #[test]
    fn collapsing_a_row_orphans_card_state() {
        let wf = filter_of(2);
        let ctx = egui::Context::default();
        ctx.set_fonts(egui::FontDefinitions::empty());
        let mut list = ListUi {
            expanded: Some(wf.rows()[0].id),
            ..Default::default()
        };
        let visible: Vec<usize> = (0..wf.rows().len()).collect();
        let frame = |list: &mut ListUi| {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                show_list(ui, &wf, &visible, list, true, None);
            })
            .drop_without_applying_deltas();
        };
        frame(&mut list);
        let open = list.epoch;
        frame(&mut list);
        assert_eq!(list.epoch, open, "an idle frame keeps card state");
        list.expanded = None;
        frame(&mut list);
        assert_ne!(list.epoch, open);
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
        list.name_drafts.insert(id, "x".into());
        list.pending_defences.insert(id);
        list.pending_simple_defences.insert(id);
        list.advanced_defences.insert(id);
        let epoch = list.epoch;
        list.clear_ids();
        assert_ne!(
            list.epoch, epoch,
            "card state keyed by reused ids is orphaned"
        );
        assert!(list.expanded.is_none());
        assert!(list.confirm_delete.is_none());
        assert!(list.name_drafts.is_empty());
        assert!(list.pending_scroll.is_none());
        assert!(!list.confirm_economy_remove);
        assert!(list.pending.is_empty());
        assert!(list.focus_line.is_none());
        assert!(list.last_expanded.is_none());
        assert!(list.line_error.is_none());
        assert!(list.pending_defences.is_empty());
        assert!(list.pending_simple_defences.is_empty());
        assert!(list.advanced_defences.is_empty());
    }

    #[test]
    fn rekey_after_apply_carries_expanded_and_defences_markers_by_position() {
        let text = "Show\n  Rarity Rare\n\nShow\n  BaseArmour >= 1\n\nShow\n  BaseEvasion 0\n";
        let old = WorkingFilter::from_text(text);
        let ids: Vec<BlockId> = old.rows().iter().map(|r| r.id).collect();
        let mut list = ListUi::default();
        list.expanded = Some(ids[1]);
        list.advanced_defences.insert(ids[1]);
        list.pending_defences.insert(ids[0]);
        let kept = list.kept_positions(&old);
        assert_eq!(
            kept,
            KeptPositions {
                expanded: Some(2),
                advanced_defences: vec![2],
            }
        );
        let new = WorkingFilter::from_text(text);
        let epoch = list.epoch;
        list.rekey_after_apply(kept.clone(), Some(&new));
        let new_ids: Vec<BlockId> = new.rows().iter().map(|r| r.id).collect();
        assert_eq!(list.expanded, Some(new_ids[1]));
        assert_eq!(list.advanced_defences, HashSet::from([new_ids[1]]));
        // Everything else is still forgotten, as by clear_ids.
        assert!(list.pending_defences.is_empty());
        assert_ne!(list.epoch, epoch);

        // A position past the end (or no document) keeps nothing.
        list.rekey_after_apply(
            KeptPositions {
                expanded: Some(9),
                advanced_defences: vec![9],
            },
            Some(&new),
        );
        assert!(list.expanded.is_none() && list.advanced_defences.is_empty());
        list.rekey_after_apply(kept, None);
        assert!(list.expanded.is_none() && list.advanced_defences.is_empty());
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
    fn expanded_body_orders_name_conditions_style() {
        let src = include_str!("rule_list.rs");
        let start = src.find("fn expanded_body(").unwrap();
        // `\n}` (not `\n}\n`) so a CRLF checkout (core.autocrlf) still finds the function end.
        let end = start + src[start..].find("\n}").unwrap();
        let f = &src[start..end];
        let name = f.find("caption(ui, \"NAME\")").unwrap();
        let cond = f.find("body::conditions_group").unwrap();
        let style = f.find("body::style_section").unwrap();
        assert!(name < cond && cond < style);
        assert!(!f.contains("body::style_group") && !f.contains("body::actions_group"));
    }

    /// Focus the name box by clicking down its right edge, then send `events`; returns the actions
    /// of the first scan position whose frame emitted any.
    fn type_in_name_box(wf: &WorkingFilter, events: Vec<Event>) -> (Vec<RowAction>, ListUi) {
        let id = wf.rows()[0].id;
        for y in (20..220).step_by(3) {
            let ctx = headless_ctx();
            let mut list = ListUi::default();
            list.expanded = Some(id);
            run_frame(&ctx, input(vec![]), wf, &mut list, true);
            run_frame(
                &ctx,
                input(click_events(Pos2::new(440.0, y as f32))),
                wf,
                &mut list,
                true,
            );
            let (f, _) = run_frame(&ctx, input(events.clone()), wf, &mut list, true);
            if f.actions
                .iter()
                .any(|a| matches!(a, RowAction::SetName(..) | RowAction::ClearName(..)))
            {
                return (f.actions, list);
            }
        }
        panic!("name box never produced a name action");
    }

    #[test]
    fn empty_name_box_writes_no_meta() {
        let mut wf = WorkingFilter::from_text("Show\n    Class \"Rings\"\n");
        let id = wf.rows()[0].id;
        // Typing adds the meta line.
        let (actions, list) = type_in_name_box(&wf, vec![Event::Text("Boots".into())]);
        assert!(list.name_drafts.contains_key(&id));
        for a in actions {
            wf.apply_action(a);
        }
        assert!(wf.text().contains("# ff:name Boots"), "{}", wf.text());
        // Clearing the box removes it; the draft is kept so the box keeps focus.
        assert_eq!(name_action(id, ""), RowAction::ClearName(id));
        assert_eq!(name_action(id, "  "), RowAction::ClearName(id));
        assert_eq!(name_action(id, "x"), RowAction::SetName(id, "x".into()));
        wf.apply_action(name_action(id, ""));
        assert!(!wf.text().contains("ff:name"), "{}", wf.text());
        // No toggle any more.
        let src = include_str!("rule_list.rs");
        let body = &src[src.find("fn expanded_body(").unwrap()..];
        let body = &body[..body.find("body::conditions_group").unwrap()];
        assert!(!body.contains("circular_toggle") && !body.contains("name_open"));
    }

    #[test]
    fn expanded_style_tagged_row_renders_without_actions() {
        for src in [
            "Show # x
    Class \"Rings\"

Show
    Class \"Belts\"

",
            "Show # x
    #ff:style S
    Class \"Rings\"
    SetFontSize 37

Show
    Class \"Belts\"

",
            "Show # x
    #ff:style Nope
    Class \"Rings\"
    SetFontSize 20

Show
    Class \"Belts\"

",
        ] {
            let mut wf = WorkingFilter::from_text(src);
            wf.set_styles(Ok(crate::loader::load_styles(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/styles-default.json"
            ))
            .unwrap()));
            let ctx = headless_ctx();
            let mut list = ListUi::default();
            list.expanded = Some(wf.rows()[0].id);
            for _ in 0..2 {
                let (frame, _) = run_frame(&ctx, input(vec![]), &wf, &mut list, true);
                assert!(frame.actions.is_empty(), "{src}: {:?}", frame.actions);
            }
        }
    }

    #[test]
    fn economy_footer_renders_every_state() {
        let ctx = headless_ctx();
        let skin = crate::skin::install(&ctx);
        let mut list = ListUi::default();
        for view in [
            EconomyView::Absent,
            EconomyView::Present { blocks: 3 },
            EconomyView::Damaged("bad".into()),
        ] {
            ctx.run_ui(input(vec![]), |ui| {
                assert!(economy_footer(ui, &skin, &view, &mut list, true).is_none());
            })
            .drop_without_applying_deltas();
        }
    }
}
