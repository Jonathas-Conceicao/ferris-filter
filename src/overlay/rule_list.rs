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

use super::drag_reorder::{self, SlotEntry, SlotTarget};
use super::rules::{
    Dir, LineKind, ListEntry, RowAction, RuleRow, SectionRow, StyleState, WorkingFilter,
};
use super::virtual_list::{build_offsets_mixed, scroll_offset_for, visible_range};
use crate::filter::MoveDest;
use crate::skin::Skin;
use crate::skin::fonts::{regular, small_caps};
use crate::skin::icons::{FLAG_ICON_SIZE, FlagIcon, flag_icon, paint_flag_icon};
use crate::skin::palette as pal;
use crate::skin::slice;
use crate::skin::widgets::{
    ROW_PAD, ff_arrow, ff_button_ex, ff_check, ff_drag_handle, ff_scroll_area_viewport,
    ff_search_field, ff_text_field_id, ff_x_button, row_background,
};

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
    /// One-shot request to focus the NAME field of that rule (set when a rule is added).
    pub focus_name: Option<BlockId>,
    /// The section (banner ordinal) whose name is being edited in its header.
    pub editing_section: Option<usize>,
    /// The text of the section name box.
    pub section_draft: String,
    /// One-shot: focus the section name box and select its text.
    pub section_focus: bool,
    /// One-shot: scroll the list to the top (set when a section is added at the top).
    pub scroll_top: bool,
    /// The rule drag in progress, if any.
    pub drag: Option<DragState>,
}

/// A list-level drag of a rule row. `handle` is the egui id captured at `drag_started`: the row's
/// id scope includes [`ListUi::epoch`], which changes when the expanded row collapses, so the
/// handle's own id is not stable but the captured one is what egui keeps dragging.
/// `injected` is a pointer position set only by the debug screenshot mode (None
/// otherwise); when set the egui drag checks are skipped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragState {
    pub src: BlockId,
    pub handle: egui::Id,
    /// The window had focus when the drag started (losing it afterwards cancels the drag).
    pub had_focus: bool,
    pub injected: Option<egui::Pos2>,
    /// The boundary a release would drop on right now (valid and not a no-op), if any.
    pub last_slot: Option<usize>,
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
            focus_name: None,
            editing_section: None,
            section_draft: String::new(),
            section_focus: false,
            scroll_top: false,
            drag: None,
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
        self.focus_name = None;
        self.editing_section = None;
        self.section_focus = false;
        self.focus_line = None;
        self.last_expanded = None;
        self.line_error = None;
        self.drag = None;
        self.row_epoch = self.row_epoch.wrapping_add(1);
        self.forget_card_state();
    }

    /// Drop a drag in progress (window concealed, focus lost, Reload): nothing is changed. Also
    /// tells egui to stop dragging the handle.
    pub fn cancel_drag(&mut self, ctx: &egui::Context) {
        if self.drag.take().is_some() {
            ctx.stop_dragging();
        }
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
    /// Section headers clicked this frame: (section name, economy, now collapsed). The caller
    /// saves the flag; it never touches the filter.
    pub toggled: Vec<(String, bool, bool)>,
    /// The list's scroll offset this frame (content space), for the drag tests.
    pub scroll_y: f32,
}

/// Fixed height of a collapsed row (and of an expanded row's header): two lines.
pub const ROW_H: f32 = 52.0;

/// Fixed height of a collapsed row.
pub fn row_height() -> f32 {
    ROW_H
}

/// Width of the round check, and the slots after it: index, Show/Hide.
const CHECK_W: f32 = 22.0;
const INDEX_W: f32 = 28.0;
const SHOW_W: f32 = 44.0;
/// Gap between flags.
const FLAG_GAP: f32 = 4.0;

/// How a flag is drawn in a collapsed row.
#[derive(Debug, Clone, PartialEq, Eq)]
enum FlagKind {
    /// A painted icon with a tooltip.
    Icon(FlagIcon),
    /// The word `Continue`, kept as text.
    ContinueWord,
    /// The style chip: the minimap icon (shape, colour indices into `skin::minimap`), drawn
    /// from the game atlas in its colour, then the (truncated) style name. A known style without
    /// a minimap icon has a name only; an untagged rule with a `MinimapIcon` has an icon only.
    Styled {
        name: Option<String>,
        icon: Option<(usize, usize)>,
    },
}

struct Flag {
    /// Accessible name and test handle.
    text: String,
    kind: FlagKind,
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
const STYLE_FLAG_CHARS: usize = 10;
/// Width reserved for the `+N` overflow badge.
const BADGE_W: f32 = 24.0;
/// Total width flags may take in a collapsed row.
const FLAG_BUDGET: f32 = 120.0;
/// The label always keeps at least this much.
const LABEL_MIN_W: f32 = 140.0;
/// Gap between the style chip's icon and its name.
const CHIP_ICON_GAP: f32 = 4.0;

/// Width of a style chip: the icon, the gap when both are present, and the name.
fn chip_width(icon: bool, name_w: Option<f32>) -> f32 {
    let icon_w = if icon { FLAG_ICON_SIZE } else { 0.0 };
    let gap = if icon && name_w.is_some() {
        CHIP_ICON_GAP
    } else {
        0.0
    };
    icon_w + gap + name_w.unwrap_or(0.0)
}

/// Width the flags may take in a row `content_w` wide whose arrows (with their gap) take
/// `arrows_w`: [`FLAG_BUDGET`], less whatever the label would otherwise lose below
/// [`LABEL_MIN_W`]. Left to right the row holds the padding, check, index, Show/Hide, three 8pt
/// gaps, the label, an 8pt gap, the flags, the arrows and the padding.
pub fn flag_budget(content_w: f32, arrows_w: f32) -> f32 {
    let fixed = 2.0 * ROW_PAD + CHECK_W + INDEX_W + SHOW_W + 3.0 * 8.0 + arrows_w + 8.0;
    (content_w - fixed - LABEL_MIN_W).clamp(0.0, FLAG_BUDGET)
}

/// How many leading flags fit in `budget` (with a `+N` badge when some do not), and how many
/// are hidden. `gap` is the space between two shown items.
pub fn fit_flags(widths: &[f32], budget: f32, gap: f32) -> (usize, usize) {
    let total: f32 = widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32;
    if total <= budget {
        return (widths.len(), 0);
    }
    let mut shown = 0;
    let mut used = BADGE_W;
    for w in widths {
        if used + gap + w > budget {
            break;
        }
        used += gap + w;
        shown += 1;
    }
    (shown, widths.len() - shown)
}

/// The flags of a row in painting order, right to left: the first entry sits closest to the
/// reorder arrows, so errors are never the ones folded into the `+N` badge.
fn flags_for(row: &RuleRow) -> Vec<Flag> {
    let mut out = Vec::new();
    if !row.enabled && row.cannot_enable {
        out.push(Flag {
            text: "can't enable".to_string(),
            kind: FlagKind::Icon(FlagIcon::Padlock),
            color: pal::WARN,
            tip: "Can't enable: these lines are not valid filter grammar. You can still move or delete it.".to_string(),
        });
        return out;
    }
    if !row.errors.is_empty() {
        out.push(Flag {
            text: "grammar error".to_string(),
            kind: FlagKind::Icon(FlagIcon::GrammarError),
            color: pal::ERROR,
            tip: diag_tip(&row.errors),
        });
    }
    if !row.warnings.is_empty() {
        out.push(Flag {
            text: "grammar warning".to_string(),
            kind: FlagKind::Icon(FlagIcon::GrammarWarning),
            color: pal::WARN,
            tip: diag_tip(&row.warnings),
        });
    }
    if matches!(row.style, StyleState::Unknown(_)) {
        out.push(Flag {
            text: "unknown style".to_string(),
            kind: FlagKind::Icon(FlagIcon::UnknownStyle),
            color: pal::WARN,
            tip: "No style with this name exists. This rule keeps its current actions, and its style link is removed on the next Apply.".to_string(),
        });
    }
    if row.group_broken {
        out.push(Flag {
            text: "group broken".to_string(),
            kind: FlagKind::Icon(FlagIcon::GroupBroken),
            color: pal::WARN,
            tip: "Group broken: these blocks were one rule (# ff:group) but no longer match, so each is shown on its own. Left as written until you edit it.".to_string(),
        });
    }
    if row.moved_across_continue {
        out.push(Flag {
            text: "moved across Continue".to_string(),
            kind: FlagKind::Icon(FlagIcon::MovedAcrossContinue),
            color: pal::WARN,
            tip: "This move changes which rules still apply after a Continue. Check the order."
                .to_string(),
        });
    }
    if row.has_continue {
        out.push(Flag {
            text: "Continue".to_string(),
            kind: FlagKind::ContinueWord,
            color: pal::GOLD,
            tip: "Matching goes on to the rules below".to_string(),
        });
    }
    if let StyleState::Known(tag) = &row.style {
        let short: String = if tag.chars().count() > STYLE_FLAG_CHARS {
            let head: String = tag.chars().take(STYLE_FLAG_CHARS).collect();
            format!("{head}…")
        } else {
            tag.clone()
        };
        out.push(Flag {
            text: format!("styled: {short}"),
            kind: FlagKind::Styled {
                name: Some(short),
                icon: row.chip_icon,
            },
            color: pal::LABEL,
            tip: format!("Style \"{tag}\" sets this rule's actions on Apply"),
        });
    } else if matches!(row.style, StyleState::None)
        && let Some(icon) = row.chip_icon
    {
        out.push(Flag {
            text: "minimap icon".to_string(),
            kind: FlagKind::Styled {
                name: None,
                icon: Some(icon),
            },
            color: pal::LABEL,
            tip: format!(
                "Minimap icon: {}",
                row.chip_icon_desc.as_deref().unwrap_or_default()
            ),
        });
    }
    out
}

/// Height of a section header row.
pub const SECTION_HEADER_HEIGHT: f32 = 36.0;

/// Tooltip of the Economy padlock and of the economy rows.
const ECONOMY_TIP: &str = "Generated by FerrisFilter. Read-only: click a rule to inspect it. Refreshed by Apply; add or remove it in the ECONOMY tab.";
/// Tooltip of the WR-02 marker.
const MATCHES_EVERYTHING_TIP: &str =
    "This rule has no conditions, so it matches every item and the rules below it never apply.";

/// Height of one displayed entry when nothing is expanded.
pub fn entry_height(entry: &ListEntry) -> f32 {
    match entry {
        ListEntry::Header { .. } => SECTION_HEADER_HEIGHT,
        ListEntry::Rule(_) | ListEntry::EconomyRule(_) => row_height(),
    }
}

/// Total height of `entries` with nothing expanded.
pub fn entries_height(entries: &[ListEntry]) -> f32 {
    entries.iter().map(entry_height).sum()
}

/// `3 rules`, `1 rule`, or during a search `2 of 12 rules`.
pub fn section_count_text(rules: usize, matches: Option<usize>) -> String {
    match matches {
        Some(m) => format!("{m} of {rules} rules"),
        None if rules == 1 => "1 rule".to_string(),
        None => format!("{rules} rules"),
    }
}

/// Accessible name of a section header.
pub fn section_accessible_name(name: &str, rules: usize, collapsed: bool) -> String {
    format!(
        "Section {name}, {}, {}",
        section_count_text(rules, None),
        if collapsed { "collapsed" } else { "expanded" }
    )
}

/// Draw the virtualized list. `entries` is the displayed list ([`WorkingFilter::entries`]).
pub fn show_list(
    ui: &mut egui::Ui,
    working: &WorkingFilter,
    entries: &[ListEntry],
    list: &mut ListUi,
    reorder_enabled: bool,
    classes: Option<&ClassCatalog>,
) -> ListFrame {
    let skin = crate::skin::of(ui.ctx());
    let rows = working.rows();
    let econ_rows = working.economy_rows();
    // The expanded row may be a user rule or an economy rule (shown read-only).
    let pos_of = |id: BlockId| {
        entries.iter().position(|e| match e {
            ListEntry::Rule(i) => rows.get(*i).is_some_and(|r| r.id == id),
            ListEntry::EconomyRule(k) => econ_rows.get(*k).is_some_and(|r| r.id == id),
            ListEntry::Header { .. } => false,
        })
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
    let heights: Vec<f32> = entries.iter().map(entry_height).collect();
    let offsets = build_offsets_mixed(&heights, expanded_vis.map(|i| (i, list.expanded_h)));
    let total = offsets.last().copied().unwrap_or(0.0);
    // Rule rows alternate by their place among the rule rows; a header does not count.
    let mut rule_no = Vec::with_capacity(entries.len());
    let mut n = 0usize;
    for e in entries {
        rule_no.push(n);
        if !matches!(e, ListEntry::Header { .. }) {
            n += 1;
        }
    }

    let mut offset = None;
    if std::mem::take(&mut list.scroll_top) {
        offset = Some(0.0);
    }
    if let Some(target) = list.pending_scroll.take()
        && let Some(vi) = pos_of(target)
    {
        offset = Some(scroll_offset_for(&offsets, vi, ui.available_height()));
    }

    let mut frame = ListFrame::default();
    let ctx = ui.ctx().clone();
    let list_rect = ui.available_rect_before_wrap();
    let slot_entries = slot_entries_of(working, entries);
    // Debug screenshot mode injects `Pos2 { x: NaN, y: boundary }`: the pointer then sits on that
    // boundary of the list at its current scroll offset.
    if let Some(d) = list.drag.as_mut()
        && let Some(p) = d.injected
        && p.x.is_nan()
    {
        let y = offsets.get(p.y as usize).copied().unwrap_or(0.0);
        d.injected = Some(egui::pos2(
            list_rect.center().x,
            list_rect.top() + y - offset.unwrap_or(0.0),
        ));
    }
    let src_index = list.drag.and_then(|d| {
        entries.iter().position(|e| match e {
            ListEntry::Rule(i) => rows.get(*i).is_some_and(|r| r.id == d.src),
            _ => false,
        })
    });
    // Finish or cancel a drag begun on an earlier frame, before any row is drawn.
    if let Some(d) = list.drag {
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if escape {
            // Consumed so nothing else (a rename box, the search box) reacts to the same press.
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            list.cancel_drag(&ctx);
        } else if src_index.is_none() || (!reorder_enabled && d.injected.is_none()) {
            // The source is gone, or reordering was switched off mid-drag (a search typed into a
            // still-focused box): the filtered view's neighbours are not the file's, so never
            // commit from it.
            list.cancel_drag(&ctx);
        } else if d.injected.is_none()
            && d.had_focus
            && !ctx.input(|i| i.viewport().focused.unwrap_or(false))
        {
            list.cancel_drag(&ctx);
        } else if d.injected.is_none() && ctx.dragged_id() != Some(d.handle) {
            // The drag ended (or egui dropped it): commit only on a real release over the list.
            let released = ctx.input(|i| i.pointer.primary_released())
                && ctx.drag_stopped_id() == Some(d.handle);
            let inside = ctx
                .input(|i| i.pointer.latest_pos())
                .is_some_and(|p| list_rect.contains(p));
            let dest = (released && inside)
                .then_some(d.last_slot)
                .flatten()
                .zip(src_index)
                .filter(|&(b, src)| !drag_reorder::is_noop(&slot_entries, src, b))
                .and_then(|(b, _)| drag_reorder::slot_target(&slot_entries, b))
                .and_then(|t| move_dest(t, rows));
            list.drag = None;
            if let Some(dest) = dest {
                frame.actions.push(RowAction::MoveTo { id: d.src, dest });
            }
        }
    }
    let dragging = list.drag;
    let mut content_w = list_rect.width();
    // The gold drop line (screen rect, clip rect), painted above the shadow after it is shown.
    let mut marker_line: Option<(egui::Rect, egui::Rect)> = None;
    ff_scroll_area_viewport(ui, &skin, "ff-rule-list", offset, |ui, viewport| {
        ui.set_height(total);
        frame.scroll_y = viewport.min.y;
        let range = visible_range(&offsets, viewport.min.y, viewport.max.y);
        if range.is_empty() {
            return;
        }
        let origin = ui.min_rect().min;
        let width = ui.available_width();
        content_w = width;
        let mut marker_y = None;
        if let (Some(d), Some(src)) = (dragging, src_index) {
            let pointer = d.injected.or_else(|| ctx.input(|i| i.pointer.latest_pos()));
            let py = pointer.map_or(list_rect.center().y, |p| p.y);
            let (wheel, dt) = ctx.input(|i| (i.smooth_scroll_delta.y, i.stable_dt.min(0.1)));
            let auto = drag_reorder::autoscroll_speed(
                py,
                list_rect.top(),
                list_rect.bottom(),
                drag_reorder::EDGE_ZONE,
                drag_reorder::AUTOSCROLL_MAX,
            );
            // egui's scroll area ignores the wheel during a drag, so forward it, and scroll at
            // the edges.
            ui.scroll_with_delta_animation(
                vec2(0.0, wheel - auto * dt),
                egui::style::ScrollAnimation::none(),
            );
            ctx.request_repaint();
            let top = drag_reorder::shadow_top(py, ROW_H, list_rect.top(), list_rect.bottom());
            let centre = top + ROW_H / 2.0 - origin.y;
            // The nearest boundary that is a place at all; when that is the source's own
            // neighbourhood the drop is a no-op and there is no slot (and no marker).
            let slot = drag_reorder::nearest_slot(&offsets, centre, |b| {
                drag_reorder::slot_target(&slot_entries, b).is_some()
            })
            .filter(|&b| !drag_reorder::is_noop(&slot_entries, src, b));
            let inside = pointer.is_some_and(|p| list_rect.contains(p));
            marker_y = slot.filter(|_| inside).map(|b| (b, origin.y + offsets[b]));
        }
        if let Some(d) = list.drag.as_mut() {
            d.last_slot = marker_y.map(|(b, _)| b);
        }
        let rect = egui::Rect::from_min_size(
            egui::pos2(origin.x, origin.y + offsets[range.start]),
            vec2(width, offsets[range.end] - offsets[range.start]),
        );
        ui.skip_ahead_auto_ids(range.start);
        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for vi in range.clone() {
                match entries.get(vi) {
                    Some(&ListEntry::Header {
                        section,
                        collapsed,
                        matches,
                    }) => {
                        let Some(sec) = working.sections().get(section) else {
                            continue;
                        };
                        ui.push_id(("section", sec.ordinal, sec.economy, list.epoch), |ui| {
                            draw_section_header(
                                ui,
                                &skin,
                                sec,
                                collapsed,
                                matches,
                                list,
                                reorder_enabled,
                                &mut frame,
                            );
                        });
                    }
                    Some(&ListEntry::Rule(i)) => {
                        let Some(row) = rows.get(i) else {
                            continue;
                        };
                        ui.push_id(("rule", row.id, list.epoch), |ui| {
                            draw_row(
                                ui,
                                &skin,
                                working,
                                classes,
                                row,
                                rule_no[vi],
                                list,
                                reorder_enabled,
                                &mut frame.actions,
                            );
                        });
                    }
                    Some(&ListEntry::EconomyRule(k)) => {
                        let Some(row) = econ_rows.get(k) else {
                            continue;
                        };
                        ui.push_id(("economy-rule", row.id, list.epoch), |ui| {
                            // Read-only: whatever the body's editor functions recorded is
                            // dropped here, never returned to the caller.
                            let _discarded = draw_economy_row(
                                ui,
                                working.economy_source(),
                                classes,
                                row,
                                rule_no[vi],
                                list,
                            );
                        });
                    }
                    None => continue,
                }
                frame.drawn += 1;
            }
        });
        if let Some((_, y)) = marker_y {
            let t = drag_reorder::MARKER_THICKNESS;
            let top = (y - t / 2.0).max(origin.y);
            let line = egui::Rect::from_min_size(egui::pos2(origin.x, top), vec2(width, t));
            marker_line = Some((
                slice::snap_rect(line, ui.ctx().pixels_per_point()),
                ui.clip_rect(),
            ));
        }
    });
    // The shadow: a translucent copy of the dragged row under the pointer, aligned to the list's
    // left edge, on an opaque ground so the rows under it do not show through its text. It
    // never takes input.
    let shadow_id = egui::Id::new(("ff-drag-shadow", "rules"));
    if let (Some(d), Some(src)) = (list.drag, src_index)
        && let ListEntry::Rule(i) = entries[src]
        && let Some(row) = rows.get(i)
    {
        let pointer = d.injected.or_else(|| ctx.input(|i| i.pointer.latest_pos()));
        let py = pointer.map_or(list_rect.center().y, |p| p.y);
        let top = drag_reorder::shadow_top(py, ROW_H, list_rect.top(), list_rect.bottom());
        egui::Area::new(shadow_id)
            .order(egui::Order::Tooltip)
            .fixed_pos(egui::pos2(list_rect.left(), top))
            .interactable(false)
            .constrain(false)
            .show(&ctx, |ui| {
                ui.set_width(content_w);
                let ground = egui::Rect::from_min_size(ui.min_rect().min, vec2(content_w, ROW_H));
                ui.painter().rect_filled(ground, 0.0, pal::PANEL);
                ui.disable();
                ui.set_opacity(0.6);
                let mut throwaway = ListUi::default();
                let mut discarded = Vec::new();
                draw_row(
                    ui,
                    &skin,
                    working,
                    classes,
                    row,
                    rule_no[src],
                    &mut throwaway,
                    false,
                    &mut discarded,
                );
            });
    }
    // The drop line goes on the shadow's own layer, after the shadow, at full opacity: the
    // shadow is centred on the pointer and the slot is the boundary nearest its centre, so the
    // line always falls inside the shadow and would otherwise be painted under it.
    if let Some((line, clip)) = marker_line {
        ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, shadow_id))
            .with_clip_rect(clip)
            .rect_filled(line, 0.0, pal::GOLD);
    }
    frame
}

/// The list-agnostic view of `entries` for [`drag_reorder`].
fn slot_entries_of(working: &WorkingFilter, entries: &[ListEntry]) -> Vec<SlotEntry> {
    entries
        .iter()
        .map(|e| match e {
            ListEntry::Rule(i) => SlotEntry::Rule(*i),
            ListEntry::EconomyRule(_) => SlotEntry::EconomyRule,
            ListEntry::Header {
                section, collapsed, ..
            } => match working.sections().get(*section) {
                Some(sec) if !sec.economy => SlotEntry::UserHeader {
                    ordinal: sec.ordinal,
                    collapsed: *collapsed,
                },
                _ => SlotEntry::EconomyHeader,
            },
        })
        .collect()
}

/// A drop target in document terms. `None` when a rule target no longer exists.
fn move_dest(target: SlotTarget, rows: &[RuleRow]) -> Option<MoveDest> {
    Some(match target {
        SlotTarget::BeforeRule(i) => MoveDest::Before(rows.get(i)?.id),
        SlotTarget::AfterBanner(k) => MoveDest::AfterBanner(k),
        SlotTarget::BeforeBanner(k) => MoveDest::BeforeBanner(k),
        SlotTarget::End => MoveDest::End,
    })
}

/// A clickable 18x18 painted icon (hover tinted) with an accessible name and a tooltip.
fn icon_button(ui: &mut egui::Ui, icon: FlagIcon, name: &str, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(FLAG_ICON_SIZE, FLAG_ICON_SIZE), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    let colour = if resp.hovered() || resp.has_focus() {
        pal::HEADING
    } else {
        pal::LABEL
    };
    paint_flag_icon(ui.painter(), rect.shrink(1.0), icon, colour);
    resp.on_hover_text(tip)
}

/// Opens the inline name box of `sec`.
fn start_rename(list: &mut ListUi, sec: &SectionRow) {
    list.editing_section = Some(sec.ordinal);
    list.section_draft = sec.name.clone();
    list.section_focus = true;
}

/// Left inset of the toggle plate and its gap to the bar.
const HEADER_PLATE_INSET: f32 = 4.0;
/// Toggle plate size (the art's 54:64 aspect).
const HEADER_PLATE_SIZE: egui::Vec2 = vec2(27.0, 32.0);
/// Plate offset from the top of the row.
const HEADER_PLATE_TOP: f32 = 2.0;
/// Title offset from the bar's left edge (clears the left cap's swirls).
const HEADER_TITLE_INSET: f32 = 60.0;
/// Real art sizes of the bar's end caps (texels).
const BAR_LEFT_PX: [usize; 2] = [132, 75];
const BAR_RIGHT_PX: [usize; 2] = [151, 75];

/// Geometry of a section header row.
#[derive(Debug, Clone, Copy)]
struct HeaderLayout {
    plate: egui::Rect,
    bar: egui::Rect,
    title_x: f32,
}

fn header_layout(rect: egui::Rect, ppp: f32) -> HeaderLayout {
    let plate = egui::Rect::from_min_size(
        egui::pos2(
            rect.min.x + HEADER_PLATE_INSET,
            rect.min.y + HEADER_PLATE_TOP,
        ),
        HEADER_PLATE_SIZE,
    );
    let bar = egui::Rect::from_min_max(
        egui::pos2(plate.max.x + HEADER_PLATE_INSET, rect.min.y),
        rect.max,
    );
    HeaderLayout {
        plate: slice::snap_rect(plate, ppp),
        bar: slice::snap_rect(bar, ppp),
        title_x: bar.min.x + HEADER_TITLE_INSET,
    }
}

/// One section header (36pt): the game's toggle plate, then the stone bar with the name, count
/// and the right-hand controls.
#[allow(clippy::too_many_arguments)]
fn draw_section_header(
    ui: &mut egui::Ui,
    skin: &Skin,
    sec: &SectionRow,
    collapsed: bool,
    matches: Option<usize>,
    list: &mut ListUi,
    reorder_enabled: bool,
    frame: &mut ListFrame,
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(width, SECTION_HEADER_HEIGHT), Sense::click());
    let accessible = section_accessible_name(&sec.name, sec.rules, collapsed);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &accessible));
    let was_editing = list.editing_section == Some(sec.ordinal) && !sec.economy;

    let ppp = ui.ctx().pixels_per_point();
    let lay = header_layout(rect, ppp);
    let painter = ui.painter().with_clip_rect(rect);
    let rects = slice::three_h_rects(lay.bar, BAR_LEFT_PX, BAR_RIGHT_PX, ppp);
    slice::paint_three(
        &painter,
        [&skin.bar.left, &skin.bar.middle, &skin.bar.right],
        rects,
        Color32::WHITE,
    );
    if ui.rect_contains_pointer(rect) {
        painter.rect_filled(lay.bar, 0.0, pal::ROW_HOVER);
    }
    if resp.has_focus() {
        painter.rect_stroke(
            lay.bar,
            0.0,
            egui::Stroke::new(1.0, pal::INPUT_BORDER_HOT),
            egui::StrokeKind::Inside,
        );
    }
    let inner = egui::Rect::from_min_max(
        egui::pos2(lay.bar.min.x, rect.min.y),
        egui::pos2(lay.bar.max.x - 12.0, rect.max.y),
    );
    let mid_y = rect.center().y;

    // Right cluster first, so the count and the name take what remains.
    let mut right = ui.new_child(
        UiBuilder::new()
            .id_salt("ff-section-right")
            .max_rect(inner)
            .layout(Layout::right_to_left(Align::Center)),
    );
    right.spacing_mut().item_spacing.x = 4.0;
    if sec.economy {
        flag_icon(
            &mut right,
            FlagIcon::Padlock,
            pal::MUTED,
            "locked",
            ECONOMY_TIP,
        );
    } else {
        if sec.can_remove && ff_x_button(&mut right, "Remove section (its rules stay)").clicked() {
            frame.actions.push(RowAction::RemoveSection(sec.ordinal));
            list.editing_section = None;
        }
        let can_down = reorder_enabled && sec.can_move_down;
        let can_up = reorder_enabled && sec.can_move_up;
        let down = ff_arrow(&mut right, skin, false, can_down);
        let up = ff_arrow(&mut right, skin, true, can_up);
        if !reorder_enabled {
            down.on_hover_text("Clear the search to reorder");
            up.on_hover_text("Clear the search to reorder");
        } else {
            if can_down {
                if down.clicked() {
                    frame
                        .actions
                        .push(RowAction::MoveSection(sec.ordinal, Dir::Down));
                    list.editing_section = None;
                }
                down.on_hover_text("Move section down");
            }
            if can_up {
                if up.clicked() {
                    frame
                        .actions
                        .push(RowAction::MoveSection(sec.ordinal, Dir::Up));
                    list.editing_section = None;
                }
                up.on_hover_text("Move section up");
            }
        }
        if icon_button(&mut right, FlagIcon::Rename, "rename", "Rename section").clicked() {
            start_rename(list, sec);
        }
    }
    let used_left = right.min_rect().left();

    // Left: the toggle plate (a second click target for the same toggle). Not focusable: the
    // whole-bar `resp` is the keyboard stop, so a header costs one Tab stop.
    let chev = ui.interact(lay.plate, ui.id().with("chevron"), Sense::CLICK);
    let toggle_name = if collapsed {
        "Expand section"
    } else {
        "Collapse section"
    };
    chev.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, toggle_name));
    let plates = if collapsed {
        &skin.toggle_expand
    } else {
        &skin.toggle_collapse
    };
    let state = if (resp.is_pointer_button_down_on() || chev.is_pointer_button_down_on())
        && ui.rect_contains_pointer(rect)
    {
        2
    } else if chev.hovered()
        || resp.hovered() && ui.rect_contains_pointer(lay.plate)
        || resp.has_focus()
        || chev.has_focus()
    {
        1
    } else {
        0
    };
    slice::paint_image(ui.painter(), &plates[state], lay.plate, Color32::WHITE);
    let chev_clicked = chev.clicked();
    chev.on_hover_text(if collapsed { "Expand" } else { "Collapse" });

    let name_x = lay.title_x;
    let count = section_count_text(sec.rules, matches);
    let count_w = text_width(ui, &count, regular(15.0));
    let count_right = used_left - 8.0;
    let count_left = count_right - count_w;
    let slot_w = (count_left - 8.0 - name_x).max(20.0);
    let painter = ui.painter().with_clip_rect(rect);
    painter.text(
        egui::pos2(count_right, mid_y),
        egui::Align2::RIGHT_CENTER,
        &count,
        regular(15.0),
        pal::MUTED,
    );

    // The name (or its edit box).
    let mut name_rect = egui::Rect::NOTHING;
    let mut name_cut = false;
    let editing = list.editing_section == Some(sec.ordinal) && !sec.economy;
    if editing {
        let w = slot_w.min(220.0);
        let field_rect = egui::Rect::from_min_size(egui::pos2(name_x, mid_y - 15.0), vec2(w, 30.0));
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt("ff-section-name")
                .max_rect(field_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let id = child.id().with("name-box");
        let r = ff_text_field_id(&mut child, id, &mut list.section_draft, "Section name", w);
        if std::mem::take(&mut list.section_focus) {
            r.request_focus();
            let n = list.section_draft.chars().count();
            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), r.id) {
                st.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(n),
                )));
                st.store(ui.ctx(), r.id);
            }
        }
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if escape && (r.has_focus() || r.lost_focus()) {
            list.editing_section = None;
        } else if r.lost_focus() {
            frame.actions.push(RowAction::RenameSection {
                ordinal: sec.ordinal,
                name: list.section_draft.clone(),
            });
            list.editing_section = None;
        }
    } else {
        let g = cut_galley(ui, &sec.name, small_caps(17.0), pal::HEADING, slot_w);
        name_cut = g.elided;
        name_rect =
            egui::Rect::from_min_size(egui::pos2(name_x, mid_y - g.size().y / 2.0), g.size());
        painter.galley(name_rect.min, g, Color32::PLACEHOLDER);
    }

    // Header interaction: a click toggles (not while its name is being typed, nor during a
    // search, where the display ignores the saved state); a double click on the name renames.
    let double_on_name = resp.double_clicked()
        && !sec.economy
        && resp
            .interact_pointer_pos()
            .is_some_and(|p| name_rect.expand(4.0).contains(p));
    if double_on_name {
        start_rename(list, sec);
    }
    if (resp.clicked() || chev_clicked) && !was_editing && matches.is_none() {
        frame
            .toggled
            .push((sec.name.clone(), sec.economy, !collapsed));
    }
    if name_cut {
        resp.on_hover_text(sec.name.clone());
    }
}

/// An economy row: the collapsed-row look without a check or arrows, in muted text. A click
/// expands it (one row at a time, shared with the user rules) into the same editor body, drawn
/// read-only by [`economy_body`]. Returns the body's discarded actions (always empty; the tests
/// hold that).
fn draw_economy_row(
    ui: &mut egui::Ui,
    working: &WorkingFilter,
    classes: Option<&ClassCatalog>,
    row: &RuleRow,
    vi: usize,
    list: &mut ListUi,
) -> Vec<RowAction> {
    ui.spacing_mut().item_spacing.y = 0.0;
    let is_expanded = list.expanded == Some(row.id);
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Economy rule (read-only)")
    });
    row_background(ui, vi, rect);
    let inner = rect.shrink2(vec2(ROW_PAD, 0.0));
    let mut right = ui.new_child(
        UiBuilder::new()
            .id_salt("ff-economy-right")
            .max_rect(inner)
            .layout(Layout::right_to_left(Align::Center)),
    );
    right.spacing_mut().item_spacing.x = FLAG_GAP;
    let mut flagged = false;
    let skin = crate::skin::of(ui.ctx());
    for f in flags_for(row) {
        match &f.kind {
            FlagKind::Icon(icon @ (FlagIcon::GrammarError | FlagIcon::GrammarWarning)) => {
                flag_icon(&mut right, *icon, f.color, &f.text, &f.tip);
                flagged = true;
            }
            // The style chip (minimap icon and/or style name), muted and non-interactive.
            FlagKind::Styled { name, icon } => {
                let w = chip_width(
                    icon.is_some(),
                    name.as_ref().map(|n| text_width(&right, n, regular(15.0))),
                );
                let (r, resp) = right.allocate_exact_size(vec2(w, FLAG_ICON_SIZE), Sense::hover());
                let mut x = r.min.x;
                if let Some((shape, colour)) = *icon {
                    crate::skin::editor::paint_minimap_icon(
                        right.painter(),
                        &skin,
                        egui::pos2(x + FLAG_ICON_SIZE / 2.0, r.center().y),
                        shape,
                        colour,
                        FLAG_ICON_SIZE,
                        Color32::WHITE,
                    );
                    x += FLAG_ICON_SIZE + CHIP_ICON_GAP;
                }
                if let Some(name) = name {
                    right.painter().text(
                        egui::pos2(x, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        name,
                        regular(15.0),
                        pal::MUTED,
                    );
                }
                resp.on_hover_text(f.tip.clone());
                flagged = true;
            }
            _ => {}
        }
    }
    let used_left = if flagged {
        right.min_rect().left()
    } else {
        inner.max.x
    };
    let painter = ui.painter().with_clip_rect(rect);
    let sx = inner.min.x + CHECK_W + 8.0 + INDEX_W + 8.0;
    let name_x = sx + SHOW_W + 8.0;
    let line1_y = rect.min.y + 18.0;
    let line2_y = rect.min.y + 38.0;
    painter.text(
        egui::pos2(sx, line1_y),
        egui::Align2::LEFT_CENTER,
        keyword_text(row.keyword),
        small_caps(17.0),
        pal::MUTED,
    );
    if let Some(name) = &row.name {
        let g = cut_galley(
            ui,
            name,
            small_caps(17.0),
            pal::MUTED,
            (used_left - 8.0 - name_x).max(LABEL_MIN_W.min(inner.max.x - name_x)),
        );
        let pos = egui::pos2(name_x, line1_y - g.size().y / 2.0);
        painter.galley(pos, g, Color32::PLACEHOLDER);
    }
    if !is_expanded && !row.summary.is_empty() {
        let g = cut_galley(
            ui,
            &row.summary,
            regular(15.0),
            pal::DISABLED,
            (used_left - 8.0 - sx).max(40.0),
        );
        let pos = egui::pos2(sx, line2_y - g.size().y / 2.0);
        painter.galley(pos, g, Color32::PLACEHOLDER);
    }
    if resp.clicked() {
        list.expanded = if is_expanded { None } else { Some(row.id) };
        list.confirm_delete = None;
    }
    resp.on_hover_text(ECONOMY_TIP);

    let mut discarded = Vec::new();
    if is_expanded {
        let body = egui::Frame::new()
            .fill(pal::ROW_EVEN)
            .inner_margin(egui::Margin::symmetric(0, 4))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                discarded = economy_body(ui, working, classes, row, list);
            });
        let h = ROW_H + body.response.rect.height();
        if (h - list.expanded_h).abs() > 0.5 {
            list.expanded_h = h;
            ui.ctx().request_repaint();
        }
        let whole = egui::Rect::from_min_max(rect.min, body.response.rect.max);
        ui.painter().rect_stroke(
            whole,
            0.0,
            egui::Stroke::new(1.0, pal::INPUT_BORDER_HOT),
            egui::StrokeKind::Inside,
        );
    }
    discarded
}

/// Opacity of the read-only economy body (egui's default disabled fade is 0.5, too faint to read
/// a long list of base types by).
const ECONOMY_BODY_ALPHA: f32 = 0.6;

/// The body of an expanded economy rule: the same NAME (only when it has one), CONDITIONS and
/// STYLE lines as a user rule, every control drawn in a disabled `Ui` (egui gives a disabled
/// widget no clicks, drags, focus or keys, so no picker or popup can open), and no Section above /
/// Duplicate / Delete row. The editor functions run against a throwaway copy of the list state
/// and a throwaway action list, both dropped, so nothing they could record reaches the filter or
/// the view (and the model refuses any action aimed at an economy block anyway, T-32-13).
/// Returns the discarded actions (always empty; the tests hold that).
fn economy_body(
    ui: &mut egui::Ui,
    working: &WorkingFilter,
    classes: Option<&ClassCatalog>,
    row: &RuleRow,
    list: &ListUi,
) -> Vec<RowAction> {
    let mut scratch = list.clone();
    let mut sink: Vec<RowAction> = Vec::new();
    ui.scope(|ui| {
        ui.visuals_mut().disabled_alpha = ECONOMY_BODY_ALPHA;
        ui.disable();
        if let Some(name) = &row.name {
            let mut text = name.clone();
            crate::skin::widgets::ff_field_row(ui, 0, "Name", None, |ui| {
                let w = ui.available_width();
                let id = ui.id().with(("economy-name", row.id));
                crate::skin::widgets::ff_text_field_id(ui, id, &mut text, "Name", w)
            });
        }
        let entries = working.body_view(row.id);
        let bctx = BodyCtx {
            row,
            entries: &entries,
            editable: working.body_editable(row.id),
            classes,
            working,
        };
        body::conditions_group(ui, &bctx, &mut scratch, &mut sink);
        ui.add_space(16.0);
        body::style_section(ui, &bctx, &mut scratch, &mut sink);
        ui.add_space(8.0);
    });
    sink
}

/// `text` laid out on one line in `font`, cut with "…" past `max_w`.
fn cut_galley(
    ui: &egui::Ui,
    text: &str,
    font: egui::FontId,
    color: Color32,
    max_w: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_string(), font, color);
    job.wrap = egui::text::TextWrapping {
        max_width: max_w.max(1.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.fonts_mut(|f| f.layout_job(job))
}

fn keyword_text(k: BlockKeyword) -> &'static str {
    match k {
        BlockKeyword::Show => "Show",
        BlockKeyword::Hide => "Hide",
        BlockKeyword::Minimal => "Minimal",
    }
}

fn text_width(ui: &egui::Ui, text: &str, font: egui::FontId) -> f32 {
    ui.fonts_mut(|fo| {
        fo.layout_no_wrap(text.to_string(), font, Color32::WHITE)
            .size()
            .x
    })
}

fn draw_row(
    ui: &mut egui::Ui,
    skin: &Skin,
    working: &WorkingFilter,
    classes: Option<&ClassCatalog>,
    row: &RuleRow,
    vi: usize,
    list: &mut ListUi,
    reorder_enabled: bool,
    actions: &mut Vec<RowAction>,
) {
    // The virtual list's offsets assume a row spans exactly `ROW_H` (collapsed) or
    // `list.expanded_h` (expanded). That only holds with no spacing between the stacked
    // allocations, so the row enforces it itself instead of relying on the caller.
    ui.spacing_mut().item_spacing.y = 0.0;
    let is_expanded = list.expanded == Some(row.id);
    let width = ui.available_width();
    // The row itself is the click sink; the check and arrows are registered after it, so they
    // sit on top.
    let (rect, row_resp) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::click());
    row_background(ui, vi, rect);
    let inner = rect.shrink2(vec2(ROW_PAD, 0.0));
    let mid_y = rect.center().y;

    // Right side first: arrows, then flags, so the label can take what remains.
    let mut right = ui.new_child(
        UiBuilder::new()
            .id_salt("ff-row-right")
            .max_rect(inner)
            .layout(Layout::right_to_left(Align::Center)),
    );
    right.spacing_mut().item_spacing.x = 4.0;
    let dragging_this = list.drag.as_ref().is_some_and(|d| d.src == row.id);
    let handle = ff_drag_handle(&mut right, skin, reorder_enabled, dragging_this);
    if handle.drag_started() && reorder_enabled && list.drag.is_none() {
        // Every expanded row collapses at drag start (not only the dragged one): the row heights
        // are then all ROW_H and no inner scroll area can take the scroll delta.
        list.drag = Some(DragState {
            src: row.id,
            handle: handle.id,
            had_focus: right.ctx().input(|i| i.viewport().focused.unwrap_or(false)),
            injected: None,
            last_slot: None,
        });
        list.expanded = None;
        list.confirm_delete = None;
        right.ctx().request_repaint();
    }
    // A tap on the handle is consumed by it and does nothing (it is not a row click).
    right.add_space(4.0);
    let mut arrows_w = right.min_rect().width();
    // WR-02: a rule that matches every item but is followed by more rules. It sits next to the
    // arrows, outside the flag fold, and takes its width out of the flag budget.
    if working.shadowing().contains(&row.id) {
        flag_icon(
            &mut right,
            FlagIcon::GrammarWarning,
            pal::WARN,
            "matches every item",
            MATCHES_EVERYTHING_TIP,
        );
        arrows_w += FLAG_ICON_SIZE + FLAG_GAP;
    }

    let flags = flags_for(row);
    let widths: Vec<f32> = flags
        .iter()
        .map(|f| match &f.kind {
            FlagKind::Icon(_) => FLAG_ICON_SIZE,
            FlagKind::ContinueWord => text_width(ui, &f.text, small_caps(17.0)) + 4.0,
            FlagKind::Styled { name, icon } => chip_width(
                icon.is_some(),
                name.as_ref().map(|n| text_width(ui, n, regular(15.0))),
            ),
        })
        .collect();
    let budget = flag_budget(width, arrows_w);
    let (shown, hidden) = fit_flags(&widths, budget, FLAG_GAP);
    let badge_tip = (hidden > 0).then(|| {
        flags[shown..]
            .iter()
            .map(|f| format!("{}: {}", f.text, f.tip))
            .collect::<Vec<_>>()
            .join("\n")
    });
    // The first flag is painted closest to the arrows (errors first); the badge goes last, at
    // the far left, so it only ever stands in for the least important flags.
    for (i, flag) in flags.into_iter().take(shown).enumerate() {
        match &flag.kind {
            FlagKind::Icon(icon) => {
                flag_icon(&mut right, *icon, flag.color, &flag.text, &flag.tip);
            }
            FlagKind::ContinueWord => {
                let (r, resp) =
                    right.allocate_exact_size(vec2(widths[i], FLAG_ICON_SIZE), Sense::hover());
                right.painter().text(
                    egui::pos2(r.max.x, r.center().y),
                    egui::Align2::RIGHT_CENTER,
                    &flag.text,
                    small_caps(17.0),
                    flag.color,
                );
                resp.on_hover_text(flag.tip.clone());
            }
            FlagKind::Styled { name, icon } => {
                let (r, resp) =
                    right.allocate_exact_size(vec2(widths[i], FLAG_ICON_SIZE), Sense::hover());
                let mut x = r.min.x;
                if let Some((shape, colour)) = *icon {
                    crate::skin::editor::paint_minimap_icon(
                        right.painter(),
                        skin,
                        egui::pos2(x + FLAG_ICON_SIZE / 2.0, r.center().y),
                        shape,
                        colour,
                        FLAG_ICON_SIZE,
                        Color32::WHITE,
                    );
                    x += FLAG_ICON_SIZE + CHIP_ICON_GAP;
                }
                if let Some(name) = name {
                    right.painter().text(
                        egui::pos2(x, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        name,
                        regular(15.0),
                        pal::MUTED,
                    );
                }
                resp.on_hover_text(flag.tip.clone());
            }
        }
    }
    if let Some(tip) = badge_tip {
        let text = format!("+{hidden}");
        let w = text_width(&right, &text, regular(15.0));
        let (r, resp) = right.allocate_exact_size(vec2(w, FLAG_ICON_SIZE), Sense::hover());
        right.painter().text(
            r.left_center(),
            egui::Align2::LEFT_CENTER,
            text,
            regular(15.0),
            pal::MUTED,
        );
        resp.on_hover_text(tip);
    }
    let used_left = right.min_rect().left();

    // Left: the round check, centred across both lines.
    let check_rect = egui::Rect::from_min_size(
        egui::pos2(inner.min.x, mid_y - CHECK_W / 2.0),
        vec2(CHECK_W, CHECK_W),
    );
    let mut left = ui.new_child(
        UiBuilder::new()
            .id_salt("ff-row-check")
            .max_rect(check_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let mut enabled = row.enabled;
    let toggle = ff_check(&mut left, skin, &mut enabled, "Enabled");
    if toggle.changed() {
        actions.push(RowAction::SetEnabled(row.id, enabled));
    }
    if !row.enabled && row.cannot_enable {
        toggle.on_hover_text("Can't enable: the commented lines are not valid filter grammar.");
    } else {
        toggle.on_hover_text(if row.enabled { "Enabled" } else { "Disabled" });
    }

    // Text: index, Show/Hide and name on line 1, the summary on line 2.
    let painter = ui.painter().with_clip_rect(rect);
    let index_right = inner.min.x + CHECK_W + 8.0 + INDEX_W;
    let kw_x = index_right + 8.0;
    let name_x = kw_x + SHOW_W + 8.0;
    let dim = !row.enabled || dragging_this;
    let line1_y = rect.min.y + 18.0;
    let line2_y = rect.min.y + 38.0;
    painter.text(
        egui::pos2(index_right, line1_y),
        egui::Align2::RIGHT_CENTER,
        row.index.to_string(),
        regular(15.0),
        if dim { pal::DISABLED } else { pal::MUTED },
    );
    painter.text(
        egui::pos2(kw_x, line1_y),
        egui::Align2::LEFT_CENTER,
        keyword_text(row.keyword),
        small_caps(17.0),
        if dim { pal::DISABLED } else { pal::LABEL },
    );
    let label_w = (used_left - 8.0 - name_x).max(LABEL_MIN_W.min(inner.max.x - name_x));
    let mut cut = false;
    if let Some(name) = &row.name {
        let g = cut_galley(
            ui,
            name,
            small_caps(17.0),
            if dim { pal::DISABLED } else { pal::HEADING },
            label_w,
        );
        cut |= g.elided;
        let pos = egui::pos2(name_x, line1_y - g.size().y / 2.0);
        painter.galley(pos, g, Color32::PLACEHOLDER);
    }
    if !is_expanded && !row.summary.is_empty() {
        // The flags and arrows are centred across both lines, so the summary stops where they
        // start, like the name (WR-06).
        let g = cut_galley(
            ui,
            &row.summary,
            regular(15.0),
            if dim { pal::DISABLED } else { pal::MUTED },
            (used_left - 8.0 - (index_right + 8.0)).max(40.0),
        );
        cut |= g.elided;
        let pos = egui::pos2(index_right + 8.0, line2_y - g.size().y / 2.0);
        painter.galley(pos, g, Color32::PLACEHOLDER);
    }
    if dragging_this {
        // The source row stays in place, greyed, while its shadow follows the pointer.
        ui.painter()
            .rect_filled(rect, 0.0, pal::GROUND.gamma_multiply(0.55));
    }
    if row_resp.clicked() {
        list.expanded = if is_expanded { None } else { Some(row.id) };
        list.confirm_delete = None;
    }
    if cut {
        let full = match &row.name {
            Some(n) if !row.summary.is_empty() => format!("{n}\n{}", row.summary),
            Some(n) => n.clone(),
            None => row.summary.clone(),
        };
        row_resp.on_hover_text(full);
    }

    if is_expanded {
        let body = egui::Frame::new()
            .fill(pal::ROW_EVEN)
            .inner_margin(egui::Margin::symmetric(0, 4))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                expanded_body(ui, working, classes, row, list, actions);
            });
        let h = ROW_H + body.response.rect.height();
        if (h - list.expanded_h).abs() > 0.5 {
            list.expanded_h = h;
            ui.ctx().request_repaint();
        }
        let whole = egui::Rect::from_min_max(rect.min, body.response.rect.max);
        ui.painter().rect_stroke(
            whole,
            0.0,
            egui::Stroke::new(1.0, pal::INPUT_BORDER_HOT),
            egui::StrokeKind::Inside,
        );
    }
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
    let skin = crate::skin::of(ui.ctx());
    // The draft outlives `row.name`: emptying the box clears the meta line, and the box must
    // keep its focus while the author retypes.
    let draft = list
        .name_drafts
        .entry(id)
        .or_insert_with(|| row.name.clone().unwrap_or_default());
    let name_id = ui.id().with(("name-box", id));
    let resp = crate::skin::widgets::ff_field_row(ui, 0, "Name", None, |ui| {
        let w = ui.available_width();
        crate::skin::widgets::ff_text_field_id(ui, name_id, draft, "Name", w)
    })
    .inner;
    if resp.changed() {
        actions.push(name_action(id, draft));
    }
    if list.focus_name == Some(id) {
        resp.request_focus();
        list.focus_name = None;
    }

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
        ui.add(egui::Label::new(
            RichText::new("Can't enable")
                .font(regular(15.0))
                .color(pal::WARN),
        ))
        .on_hover_text(CANNOT_ENABLE_LONG);
    }
    for d in &row.errors {
        ui.add(
            egui::Label::new(
                RichText::new(format!("Line {}: {}", d.line, d.message)).color(pal::WARN),
            )
            .wrap(),
        );
    }
    ui.add_space(16.0);

    // STYLE (Custom or preset, with its actions), last before Duplicate/Delete
    body::style_section(ui, &bctx, list, actions);
    ui.add_space(8.0);

    // Duplicate / Delete
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        // Explicit 8pt gaps between the three buttons.
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space(crate::skin::widgets::ROW_PAD - 8.0);
        if ff_button_ex(ui, &skin, "Section above", 128.0, true, false)
            .on_hover_text("Inserts a section divider directly above this rule.")
            .clicked()
        {
            actions.push(RowAction::AddSectionAbove(id));
        }
        if crate::skin::widgets::ff_button(ui, &skin, "Duplicate", 112.0).clicked() {
            actions.push(RowAction::Duplicate(id));
        }
        if crate::skin::widgets::ff_button_ex(ui, &skin, "Delete", 112.0, true, true).clicked() {
            list.confirm_delete = Some(id);
        }
    });
}

/// What the toolbar reported this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarClick {
    None,
    AddRule,
    AddSection,
}

/// The toolbar of the RULES tab: the recessed search field (hint names the rule count), a small
/// Clear while a query is typed, `+ Add rule` and `+ Add section`. Escape clears the query.
pub fn search_row(
    ui: &mut egui::Ui,
    skin: &Skin,
    query: &mut String,
    add_enabled: bool,
    total: usize,
) -> ToolbarClick {
    search_row_rects(ui, skin, query, add_enabled, total).0
}

/// [`search_row`], also returning the rects of the search field, `+ Add rule` and
/// `+ Add section`.
fn search_row_rects(
    ui: &mut egui::Ui,
    skin: &Skin,
    query: &mut String,
    add_enabled: bool,
    total: usize,
) -> (ToolbarClick, egui::Rect, egui::Rect, egui::Rect) {
    let mut out = (
        ToolbarClick::None,
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
    );
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), 34.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let hint = format!("Search {total} rules, classes, base types…");
            // The buttons are at least 124 / 140pt, wider if a caption needs it (see `ff_button_ex`).
            let add_w = (text_width(ui, "+ Add rule", small_caps(17.0)) + 44.0).max(124.0);
            let sec_w = (text_width(ui, "+ Add section", small_caps(17.0)) + 44.0).max(140.0);
            let width =
                (ui.available_width() - add_w - sec_w - 8.0 - crate::skin::widgets::FIELD_GAP)
                    .max(40.0);
            let start_x = ui.next_widget_position().x;
            let search_id = ui.id().with("rules-search");
            let resp = ff_search_field(ui, skin, search_id, query, &hint, width);
            // The field is 30pt high and `width` wide, centred on the edit.
            let field = egui::Rect::from_min_max(
                egui::pos2(start_x, resp.rect.center().y - 15.0),
                egui::pos2(start_x + width, resp.rect.center().y + 15.0),
            );
            if (resp.has_focus() || resp.lost_focus())
                && !query.is_empty()
                && ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
                query.clear();
            }
            // The buttons are placed from the row's right edge, so their position does not depend
            // on where the field leaves the cursor; the field is sized to leave `FIELD_GAP`.
            let row = ui.max_rect();
            let sec_btn =
                egui::Rect::from_min_max(egui::pos2(row.max.x - sec_w, row.min.y), row.max);
            let add_btn = egui::Rect::from_min_max(
                egui::pos2(sec_btn.min.x - 8.0 - add_w, row.min.y),
                egui::pos2(sec_btn.min.x - 8.0, row.max.y),
            );
            let add = ui
                .scope_builder(UiBuilder::new().max_rect(add_btn), |ui| {
                    ff_button_ex(ui, skin, "+ Add rule", add_w, add_enabled, false)
                })
                .inner
                .on_hover_text(ADD_RULE_TIP);
            let section = ui
                .scope_builder(UiBuilder::new().max_rect(sec_btn), |ui| {
                    ff_button_ex(ui, skin, "+ Add section", sec_w, add_enabled, false)
                })
                .inner
                .on_hover_text(ADD_SECTION_TIP);
            let click = if add_enabled && add.clicked() {
                ToolbarClick::AddRule
            } else if add_enabled && section.clicked() {
                ToolbarClick::AddSection
            } else {
                ToolbarClick::None
            };
            out = (click, field, add.rect, section.rect);
        },
    );
    out
}

/// Tooltip on `+ Add section`.
pub const ADD_SECTION_TIP: &str = "Adds a section divider at the top of the list.";

/// Tooltip on `+ Add rule` (it replaces the standing empty-state paragraph).
pub const ADD_RULE_TIP: &str = "Adds a new rule at the top of the file.";

/// Test support shared with the app tests: browse every economy rule as a user would (expand it,
/// click every widget of its body, press keys at it) and report what that produced.
#[cfg(test)]
pub(crate) mod economy_browse {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

    /// A filter with one user rule and a two-block economy section whose blocks use list,
    /// comparison, boolean, defence and every style line, so the read-only body draws nearly
    /// every editor control.
    pub(crate) fn rich_economy_text() -> String {
        let r = crate::filter::section::RULE_LINE;
        let b = crate::filter::section::BEGIN_LINE;
        let bases = [
            "Chaos Orb",
            "Divine Orb",
            "Exalted Orb",
            "Orb of Annulment",
            "Orb of Chance",
            "Regal Orb",
            "Vaal Orb",
            "Gemcutter's Prism",
            "Glassblower's Bauble",
            "Mirror of Kalandra",
        ]
        .map(|n| format!("\"{n}\""))
        .join(" ");
        let first = [
            "Show".to_string(),
            format!("  BaseType == {bases}"),
            "  Class \"Stackable Currency\"".into(),
            "  Rarity Normal".into(),
            "  ItemLevel >= 70".into(),
            "  AreaLevel >= 60".into(),
            "  SetTextColor 255 121 198 255".into(),
            "  SetBorderColor 255 121 198 255".into(),
            "  SetBackgroundColor 40 42 54 255".into(),
            "  SetFontSize 37".into(),
            "  MinimapIcon 0 Pink Circle".into(),
            "  PlayEffect Pink".into(),
            "  PlayAlertSound 2 200".into(),
            "  DisableDropSound".into(),
            "  Continue".into(),
        ]
        .join("\n");
        let second = [
            "Hide",
            "  Class \"Body Armours\"",
            "  BaseArmour > 0",
            "  BaseEvasion 0",
            "  Sockets >= 2",
            "  Corrupted False",
            "  SetFontSize 21",
            "  MinimapIcon -1 Red Star",
            "  PlayEffect None",
        ]
        .join("\n");
        format!("Show\n  Class \"Rings\"\n\n{r}\n{b}\n{r}\n\n{first}\n\n{second}\n")
    }

    /// What browsing produced.
    #[derive(Debug, Default)]
    pub(crate) struct Report {
        /// Every action any frame recorded, including the ones the body discards.
        pub actions: Vec<RowAction>,
        /// Enabled interactive widgets found inside an expanded body (must be none).
        pub enabled_in_body: Vec<Rect>,
        /// Disabled interactive widgets found inside the bodies (the drawn controls).
        pub disabled_in_body: usize,
        /// Frames that ended with a popup open or a widget focused.
        pub popups_or_focus: usize,
        /// Frames after which the list state changed (anything but the measured height).
        pub list_changed: usize,
        /// The tallest expanded height measured.
        pub max_expanded_h: f32,
        /// Economy rules browsed.
        pub rules: usize,
    }

    fn input(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(535.0, 2400.0))),
            events,
            ..Default::default()
        }
    }

    fn click(pos: Pos2) -> Vec<Vec<Event>> {
        let button = |pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::default(),
        };
        vec![
            vec![Event::PointerMoved(pos)],
            vec![button(true)],
            vec![button(false)],
        ]
    }

    fn key(k: egui::Key) -> Vec<Event> {
        [true, false]
            .into_iter()
            .map(|pressed| Event::Key {
                key: k,
                physical_key: Some(k),
                pressed,
                repeat: false,
                modifiers: Modifiers::default(),
            })
            .collect()
    }

    /// The parts of the list state a read-only body must never touch (`expanded` is left out:
    /// the row header, which may hold keyboard focus, expands and collapses the row).
    fn snapshot(list: &ListUi) -> String {
        format!(
            "{:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {}",
            list.confirm_delete,
            list.name_drafts,
            list.pending,
            list.focus_line,
            list.line_error,
            list.pending_defences,
            list.pending_simple_defences,
            list.simple_all,
            list.focus_card,
            list.focus_name,
            list.editing_section,
            list.confirm_economy_remove,
            list.section_draft,
        )
    }

    fn frame(
        ctx: &egui::Context,
        events: Vec<Event>,
        wf: &WorkingFilter,
        row: &RuleRow,
        list: &mut ListUi,
        report: &mut Report,
    ) {
        let before = snapshot(list);
        ctx.run_ui(input(events), |ui| {
            let area = Rect::from_min_size(Pos2::ZERO, vec2(535.0, 2400.0));
            let mut child = ui.new_child(UiBuilder::new().max_rect(area));
            // The actions the body records (and the list drops).
            report
                .actions
                .extend(draw_economy_row(&mut child, wf, None, row, 0, list));
        })
        .drop_without_applying_deltas();
        if snapshot(list) != before {
            report.list_changed += 1;
        }
        // The row header itself may take keyboard focus (it expands and collapses the row);
        // nothing inside the body may.
        let body_focus = ctx.memory(|m| m.focused()).is_some_and(|id| {
            ctx.viewport(|v| v.prev_pass.widgets.get(id).map(|w| w.rect.min.y))
                .is_none_or(|y| y >= ROW_H)
        });
        if egui::Popup::is_any_open(ctx) || body_focus {
            report.popups_or_focus += 1;
        }
        report.max_expanded_h = report.max_expanded_h.max(list.expanded_h);
    }

    /// [`browse`] through the whole list, as the RULES tab draws it: every economy rule expanded in
    /// turn inside [`show_list`], every disabled widget (the body's controls) clicked and keys
    /// pressed. Returns what the list frames reported, which the app applies to the filter.
    pub(crate) fn browse_list(wf: &mut WorkingFilter) -> (Vec<RowAction>, usize) {
        let entries = wf.entries("", |_| false);
        let wf: &WorkingFilter = wf;
        let mut actions = Vec::new();
        let mut clicked = 0;
        for row in wf.economy_rows() {
            let ctx = egui::Context::default();
            crate::skin::fonts::install(&ctx);
            let mut list = ListUi {
                expanded: Some(row.id),
                last_expanded: Some(row.id),
                ..Default::default()
            };
            let run = |events: Vec<Event>, list: &mut ListUi, actions: &mut Vec<RowAction>| {
                ctx.run_ui(input(events), |ui| {
                    let area = Rect::from_min_size(Pos2::ZERO, vec2(535.0, 2400.0));
                    let mut child = ui.new_child(UiBuilder::new().max_rect(area));
                    actions.extend(show_list(&mut child, wf, &entries, list, true, None).actions);
                })
                .drop_without_applying_deltas();
            };
            for _ in 0..3 {
                run(vec![], &mut list, &mut actions);
            }
            let body: Vec<Rect> = ctx.viewport(|v| {
                v.prev_pass
                    .widgets
                    .layers()
                    .flat_map(|(_, w)| w.iter())
                    .filter(|w| w.sense.interactive() && !w.enabled)
                    .map(|w| w.rect)
                    .collect()
            });
            for r in body {
                clicked += 1;
                for events in click(r.center()) {
                    run(events, &mut list, &mut actions);
                }
                list.expanded = Some(row.id);
            }
            for k in [
                egui::Key::Tab,
                egui::Key::Enter,
                egui::Key::Space,
                egui::Key::Delete,
            ] {
                run(key(k), &mut list, &mut actions);
            }
        }
        (actions, clicked)
    }

    /// Expand each economy rule of `wf`, click the centre of every interactive widget of its body
    /// and press keys at it (type, Enter, Space, Delete, arrows, Escape, Tab through it).
    pub(crate) fn browse(wf: &WorkingFilter) -> Report {
        let mut report = Report::default();
        for row in wf.economy_rows() {
            report.rules += 1;
            let ctx = egui::Context::default();
            crate::skin::fonts::install(&ctx);
            let mut list = ListUi {
                expanded: Some(row.id),
                last_expanded: Some(row.id),
                ..Default::default()
            };
            for _ in 0..3 {
                frame(&ctx, vec![], wf, row, &mut list, &mut report);
            }
            let widgets: Vec<egui::WidgetRect> = ctx.viewport(|v| {
                v.prev_pass
                    .widgets
                    .layers()
                    .flat_map(|(_, w)| w.iter().copied())
                    .filter(|w| w.sense.interactive() && w.rect.min.y >= ROW_H)
                    .collect()
            });
            for w in &widgets {
                if w.enabled {
                    report.enabled_in_body.push(w.rect);
                } else {
                    report.disabled_in_body += 1;
                }
            }
            for w in &widgets {
                for events in click(w.rect.center()) {
                    frame(&ctx, events, wf, row, &mut list, &mut report);
                }
            }
            let mut keys = vec![vec![Event::Text("x".into())]];
            for k in [
                egui::Key::Enter,
                egui::Key::Space,
                egui::Key::Delete,
                egui::Key::Backspace,
                egui::Key::ArrowDown,
                egui::Key::ArrowUp,
                egui::Key::Escape,
            ] {
                keys.push(key(k));
            }
            keys.extend(std::iter::repeat_with(|| key(egui::Key::Tab)).take(40));
            keys.push(key(egui::Key::Enter));
            keys.push(vec![Event::Text("1".into())]);
            for events in keys {
                frame(&ctx, events, wf, row, &mut list, &mut report);
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::super::virtual_list::build_offsets;
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

    /// Every user rule as a plain entry (no sections).
    fn rule_entries(wf: &WorkingFilter) -> Vec<ListEntry> {
        (0..wf.rows().len()).map(ListEntry::Rule).collect()
    }

    fn headless_ctx() -> egui::Context {
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        ctx
    }

    fn input(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(535.0, 600.0))),
            events,
            ..Default::default()
        }
    }

    /// [`input`] on a 1200pt-tall screen: an expanded Custom rule's style lines push the rule's
    /// buttons below the 600pt one.
    fn tall_input(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(535.0, 1200.0))),
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
        let visible = rule_entries(wf);
        let area_h = raw.screen_rect.map_or(600.0, |r| r.height());
        let mut out = (ListFrame::default(), 0.0);
        ctx.run_ui(raw, |ui| {
            let h = row_height();
            let area = Rect::from_min_size(Pos2::ZERO, vec2(535.0, area_h));
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
            "/tests/fixtures/seed-styles.json"
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
            assert_eq!(g.color, pal::WARN);
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
        assert_eq!(u.color, pal::WARN);
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
        assert_eq!(k.text, "styled: Exalted Sh…");
        assert_eq!(
            k.kind,
            FlagKind::Styled {
                name: Some("Exalted Sh…".into()),
                icon: row.chip_icon,
            }
        );
        assert!(k.tip.contains("Exalted Showcase"));
        row.style = StyleState::Unchecked("S".into());
        assert!(!flags_for(&row).iter().any(|f| f.text.contains("style")));
    }

    fn chip_flag(row: &RuleRow) -> Option<Flag> {
        flags_for(row)
            .into_iter()
            .find(|f| matches!(f.kind, FlagKind::Styled { .. }))
    }

    #[test]
    fn known_style_chip_draws_its_minimap_icon_then_the_name() {
        // B: Small Red Circle.
        let wf = styled_filter("B");
        let row = &wf.rows()[0];
        assert_eq!(row.chip_icon, Some((0, 0)));
        let f = chip_flag(row).expect("chip");
        assert_eq!(f.text, "styled: B");
        assert_eq!(
            f.kind,
            FlagKind::Styled {
                name: Some("B".into()),
                icon: Some((0, 0)),
            }
        );
        assert_eq!(f.tip, "Style \"B\" sets this rule's actions on Apply");
        // The untagged second rule has no MinimapIcon: no chip.
        assert!(chip_flag(&wf.rows()[1]).is_none());
    }

    #[test]
    fn known_style_without_minimap_chip_is_name_only() {
        let wf = styled_filter("E");
        let f = chip_flag(&wf.rows()[0]).expect("chip");
        assert_eq!(
            f.kind,
            FlagKind::Styled {
                name: Some("E".into()),
                icon: None,
            }
        );
    }

    #[test]
    fn untagged_rule_chip_is_its_minimap_icon_alone() {
        let wf = WorkingFilter::from_text(
            "Show\n  Class \"Rings\"\n  MinimapIcon -1\n  MinimapIcon 0 Red Circle\n  \
             MinimapIcon 1 Blue Star\n\nShow\n  MinimapIcon -1\n\n\
             Show\n  MinimapIcon 3 Blue Star\n\nShow\n  Class \"Belts\"\n\n\
             Show\n  MinimapIcon 1 Blue Star\n  MinimapIcon -1\n",
        );
        let f = chip_flag(&wf.rows()[0]).expect("chip");
        assert_eq!(f.text, "minimap icon");
        assert_eq!(f.tip, "Minimap icon: Medium Blue Star");
        // Star is shape 4, Blue colour 2; the last line wins, as it does in game.
        assert_eq!(
            f.kind,
            FlagKind::Styled {
                name: None,
                icon: Some((4, 2)),
            }
        );
        // MinimapIcon -1, an out-of-range size, no MinimapIcon, or a shown icon overridden by a
        // later `MinimapIcon -1`: no chip.
        for row in &wf.rows()[1..] {
            assert!(chip_flag(row).is_none(), "{}", row.label);
        }
    }

    #[test]
    fn unknown_or_unchecked_style_has_no_chip() {
        let src = "#ff:style Nope\nShow\n  MinimapIcon 1 Blue Star\n";
        let wf = WorkingFilter::from_text(src);
        // No styles loaded: Unchecked, no chip.
        assert!(matches!(wf.rows()[0].style, StyleState::Unchecked(_)));
        assert!(chip_flag(&wf.rows()[0]).is_none());
        let mut wf = wf;
        wf.set_styles(Ok(crate::loader::load_styles(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/seed-styles.json"
        ))
        .unwrap()));
        let f = flags_for(&wf.rows()[0]);
        assert!(f.iter().any(|f| f.text == "unknown style"));
        assert!(chip_flag(&wf.rows()[0]).is_none());
    }

    #[test]
    fn chip_icon_follows_the_loaded_styles() {
        // Rows rebuild on set_styles, so a style set arriving later lights the icon.
        let mut wf = WorkingFilter::from_text("#ff:style S\nShow\n");
        assert_eq!(wf.rows()[0].chip_icon, None);
        wf.set_styles(Ok(crate::loader::load_styles(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/seed-styles.json"
        ))
        .unwrap()));
        assert_eq!(wf.rows()[0].chip_icon, Some((0, 9)));
    }

    #[test]
    fn chip_width_is_icon_gap_and_name() {
        assert_eq!(chip_width(true, Some(30.0)), FLAG_ICON_SIZE + 4.0 + 30.0);
        assert_eq!(chip_width(true, None), FLAG_ICON_SIZE);
        assert_eq!(chip_width(false, Some(30.0)), 30.0);
    }

    #[test]
    fn fit_flags_collapses_overflow_into_a_badge() {
        assert_eq!(fit_flags(&[50.0, 50.0, 50.0], 120.0, 4.0), (1, 2));
        assert_eq!(fit_flags(&[40.0, 40.0], 120.0, 4.0), (2, 0));
        assert_eq!(fit_flags(&[18.0; 5], 120.0, 4.0), (5, 0));
        let (shown, hidden) = fit_flags(&[18.0; 8], 120.0, 4.0);
        assert!(shown < 8 && shown + hidden == 8 && hidden > 0);
    }

    #[test]
    fn flag_budget_keeps_label_minimum() {
        let hw = crate::skin::widgets::DRAG_HANDLE_SIZE + 4.0;
        assert!(flag_budget(517.0, hw) >= 120.0);
        assert_eq!(flag_budget(2000.0, hw), FLAG_BUDGET);
        assert!(flag_budget(300.0, hw) < 120.0);
        assert_eq!(flag_budget(100.0, hw), 0.0);
        // At any width the label keeps its minimum: width - fixed parts - budget >= LABEL_MIN_W
        // whenever the budget was cut.
        let w = 400.0;
        let fixed = 2.0 * ROW_PAD + CHECK_W + INDEX_W + SHOW_W + 24.0 + hw + 8.0;
        assert!(w - fixed - flag_budget(w, hw) >= LABEL_MIN_W - 0.01);
    }

    #[test]
    fn flags_carry_kinds_tips_and_paint_order() {
        let wf = styled_filter("S");
        let mut row = wf.rows()[0].clone();
        row.has_continue = true;
        row.group_broken = true;
        row.moved_across_continue = true;
        let diag = |severity| FilterDiagnostic {
            line: 1,
            severity,
            message: "bad".into(),
        };
        row.errors = vec![diag(crate::filter::diag::Severity::Error)];
        row.warnings = vec![diag(crate::filter::diag::Severity::Warning)];
        let f = flags_for(&row);
        let texts: Vec<&str> = f.iter().map(|f| f.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "grammar error",
                "grammar warning",
                "group broken",
                "moved across Continue",
                "Continue",
                "styled: S"
            ]
        );
        assert_eq!(f[0].kind, FlagKind::Icon(FlagIcon::GrammarError));
        assert_eq!(f[2].kind, FlagKind::Icon(FlagIcon::GroupBroken));
        assert!(
            f[2].tip
                .starts_with("Group broken: these blocks were one rule (# ff:group)")
        );
        assert_eq!(f[4].kind, FlagKind::ContinueWord);
        // S: Pink Circle (shape 0, colour 9).
        assert_eq!(
            f[5].kind,
            FlagKind::Styled {
                name: Some("S".into()),
                icon: Some((0, 9)),
            }
        );

        let mut dis = row.clone();
        dis.enabled = false;
        dis.cannot_enable = true;
        let f = flags_for(&dis);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, FlagKind::Icon(FlagIcon::Padlock));
        assert_eq!(
            f[0].tip,
            "Can't enable: these lines are not valid filter grammar. You can still move or delete it."
        );
    }

    fn text_rects(mut full: egui::FullOutput) -> Vec<(String, Rect)> {
        full.textures_delta.clear();
        full.shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::epaint::Shape::Text(t) => Some((
                    t.galley.text().to_string(),
                    t.galley.rect.translate(t.pos.to_vec2()),
                )),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn continue_word_is_8pt_from_styled_chip() {
        let wf = styled_filter("S");
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        let visible = rule_entries(&wf);
        let mut found = None;
        for _ in 0..3 {
            let full = ctx.run_ui(input(vec![]), |ui| {
                let area = Rect::from_min_size(Pos2::ZERO, vec2(535.0, 600.0));
                let mut child = ui.new_child(UiBuilder::new().max_rect(area));
                show_list(&mut child, &wf, &visible, &mut list, true, None);
            });
            let texts = text_rects(full);
            let cont = texts.iter().find(|(t, _)| t == "Continue").map(|(_, r)| *r);
            let name = texts.iter().find(|(t, _)| t == "S").map(|(_, r)| *r);
            found = cont.zip(name);
        }
        let (cont, name) = found.expect("Continue word and style name painted");
        assert!(
            cont.min.x - name.max.x >= 8.0 - 0.01,
            "gap {}",
            cont.min.x - name.max.x
        );
    }

    /// Bounds of every mesh drawn with texture `tex`.
    fn meshes_with(shape: &egui::Shape, tex: egui::TextureId, out: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Mesh(m) if m.texture_id == tex => out.push(m.calc_bounds()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| meshes_with(s, tex, out)),
            _ => {}
        }
    }

    /// The style chip paints the atlas icon (18pt) left of the style name; a style without a
    /// minimap paints no atlas icon.
    #[test]
    fn style_chip_paints_the_atlas_icon_left_of_the_name() {
        for (tag, want_icon) in [("S", true), ("E", false)] {
            let wf = styled_filter(tag);
            let ctx = headless_ctx();
            let mut list = ListUi::default();
            let visible = rule_entries(&wf);
            let mut last = None;
            for _ in 0..3 {
                let mut full = ctx.run_ui(input(vec![]), |ui| {
                    let area = Rect::from_min_size(Pos2::ZERO, vec2(535.0, 600.0));
                    let mut child = ui.new_child(UiBuilder::new().max_rect(area));
                    show_list(&mut child, &wf, &visible, &mut list, true, None);
                });
                let tex = crate::skin::of(&ctx).minimap_icons.id();
                let mut icons = Vec::new();
                for c in &full.shapes {
                    meshes_with(&c.shape, tex, &mut icons);
                }
                full.textures_delta.clear();
                let name = text_rects(full)
                    .into_iter()
                    .find(|(t, _)| t == tag)
                    .map(|(_, r)| r);
                last = Some((icons, name));
            }
            let (icons, name) = last.unwrap();
            let name = name.expect("style name painted");
            if want_icon {
                assert_eq!(icons.len(), 1, "{tag}");
                let icon = icons[0];
                assert!((icon.width() - FLAG_ICON_SIZE).abs() < 1.0, "{icon:?}");
                assert!(icon.max.x <= name.min.x + 0.01, "{icon:?} {name:?}");
            } else {
                assert!(icons.is_empty(), "{tag}");
            }
        }
    }

    /// WR-06: a long collapsed-row summary is cut before the flags and arrows, not painted under
    /// them.
    #[test]
    fn long_summary_stops_before_the_flags() {
        let src = "Show\n  Class \"Body Armours\" \"Helmets\"\n  AreaLevel >= 65\n  \
                   ItemLevel >= 80\n  Quality >= 10\n  Sockets >= 2\n  DropLevel >= 50\n  \
                   Rarity >= Rare\n  Corrupted False\n  Continue\n";
        let wf = WorkingFilter::from_text(src);
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        let visible = rule_entries(&wf);
        let mut found = None;
        for _ in 0..3 {
            let full = ctx.run_ui(input(vec![]), |ui| {
                let area = Rect::from_min_size(Pos2::ZERO, vec2(535.0, 600.0));
                let mut child = ui.new_child(UiBuilder::new().max_rect(area));
                show_list(&mut child, &wf, &visible, &mut list, true, None);
            });
            let texts = text_rects(full);
            let cont = texts.iter().find(|(t, _)| t == "Continue").map(|(_, r)| *r);
            let summary = texts
                .iter()
                .find(|(t, _)| t.starts_with("Class"))
                .map(|(_, r)| *r);
            found = cont.zip(summary);
        }
        let (cont, summary) = found.expect("Continue flag and summary painted");
        assert!(
            summary.max.x <= cont.min.x - 8.0 + 0.5,
            "summary {summary:?} runs into the flags at {cont:?}"
        );
    }

    #[test]
    fn search_field_and_add_button_do_not_touch() {
        let ctx = headless_ctx();
        let skin = crate::skin::install(&ctx);
        for q in ["", "rings"] {
            let mut query = q.to_string();
            let mut rects = (
                ToolbarClick::None,
                Rect::NOTHING,
                Rect::NOTHING,
                Rect::NOTHING,
            );
            let mut avail = 0.0;
            ctx.run_ui(input(vec![]), |ui| {
                avail = ui.available_width();
                rects = search_row_rects(ui, &skin, &mut query, true, 5);
            })
            .drop_without_applying_deltas();
            let (_, field, add, section) = rects;
            let gap = add.min.x - field.max.x;
            assert!(
                (gap - crate::skin::widgets::FIELD_GAP).abs() < 0.6,
                "query {q:?}: gap {gap}"
            );
            let gap2 = section.min.x - add.max.x;
            assert!((gap2 - 8.0).abs() < 0.6, "query {q:?}: gap {gap2}");
            assert!(add.width() >= 124.0 - 0.6, "add {}", add.width());
            // The caption needs more than the 140pt minimum (ff_button_ex adds 44pt of padding).
            assert!(
                section.width() >= 140.0 - 0.6,
                "section {}",
                section.width()
            );
            assert!(
                (field.width() - (avail - add.width() - section.width() - 18.0)).abs() < 0.6,
                "field {} of {avail}",
                field.width()
            );
            assert_eq!(query, q);
        }
    }

    #[test]
    fn pending_scroll_brings_a_far_row_into_view() {
        let wf = filter_of(511);
        let ctx = headless_ctx();
        let visible = rule_entries(&wf);
        let mut list = ListUi::default();
        let frame = |list: &mut ListUi| {
            ctx.run_ui(input(vec![]), |ui| {
                let area = Rect::from_min_size(Pos2::ZERO, vec2(535.0, 600.0));
                let mut child = ui.new_child(UiBuilder::new().max_rect(area));
                show_list(&mut child, &wf, &visible, list, true, None);
            })
        };
        let first = text_rects(frame(&mut list));
        assert!(first.iter().any(|(t, _)| t == "1"));
        list.pending_scroll = Some(wf.rows()[300].id);
        let mut texts = Vec::new();
        for _ in 0..3 {
            texts = text_rects(frame(&mut list));
        }
        assert!(list.pending_scroll.is_none());
        assert!(texts.iter().any(|(t, _)| t == "301"), "row 301 not visible");
        assert!(!texts.iter().any(|(t, _)| t == "1"));
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
    fn collapsed_row_is_52() {
        assert_eq!(row_height(), 52.0);
        assert_eq!(ROW_H, 52.0);
    }

    #[test]
    fn expanded_row_height_excludes_summary() {
        let wf = filter_of(3);
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        list.expanded = Some(wf.rows()[1].id);
        let header = ROW_H;
        for _ in 0..3 {
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
                let row_h = row_height();
                let skin = crate::skin::of(ui.ctx());
                let mut actions = Vec::new();
                let drawn = ui.push_id(("rule", i), |ui| {
                    draw_row(
                        ui,
                        &skin,
                        &wf,
                        None,
                        &wf.rows()[i],
                        i,
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
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        let (_, row_h) = run_frame(&ctx, input(vec![]), &wf, &mut list, true);
        let click = |list: &mut ListUi, pos: Pos2| {
            // Hover first so the press lands on last frame's hit-test, then press and release.
            for events in click_events(pos).into_iter().map(|e| vec![e]) {
                run_frame(&ctx, input(events), &wf, list, true);
            }
        };
        let y = row_h + 38.0;
        // Control: the blank area past the text (left of the handle) has always expanded the row.
        click(&mut list, Pos2::new(420.0, y));
        assert_eq!(list.expanded, Some(wf.rows()[1].id), "blank-area click");
        list.expanded = None;
        run_frame(&ctx, input(vec![]), &wf, &mut list, true);
        // Second row, a few pixels into the summary text (past the indent).
        click(
            &mut list,
            Pos2::new(10.0 + CHECK_W + 8.0 + INDEX_W + 8.0 + 12.0, y),
        );
        assert_eq!(list.expanded, Some(wf.rows()[1].id), "summary-text click");
    }

    #[test]
    fn focus_name_is_consumed_by_the_expanded_body() {
        let wf = filter_of(2);
        let id = wf.rows()[0].id;
        let ctx = headless_ctx();
        let mut list = ListUi {
            expanded: Some(id),
            focus_name: Some(id),
            ..ListUi::default()
        };
        for _ in 0..3 {
            run_frame(&ctx, input(vec![]), &wf, &mut list, true);
        }
        assert_eq!(list.focus_name, None);
        assert!(ctx.memory(|m| m.focused()).is_some());
    }

    /// Press at `pos`, then move `dy` further down over the next frames.
    fn press_events(pos: Pos2) -> Vec<Event> {
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::default(),
            },
        ]
    }

    /// Row 0's handle sweep: does a press and a 10pt drag at `x` start a drag?
    fn drag_starts_at(wf: &WorkingFilter, reorder: bool, x: f32) -> (bool, Vec<RowAction>) {
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        let y = row_height() / 2.0;
        run_frame(&ctx, input(vec![]), wf, &mut list, reorder);
        run_frame(
            &ctx,
            input(press_events(Pos2::new(x, y))),
            wf,
            &mut list,
            reorder,
        );
        let mut actions = vec![];
        for step in 1..=3 {
            let (f, _) = run_frame(
                &ctx,
                input(vec![Event::PointerMoved(Pos2::new(
                    x,
                    y + 5.0 * step as f32,
                ))]),
                wf,
                &mut list,
                reorder,
            );
            actions.extend(f.actions);
        }
        (list.drag.is_some(), actions)
    }

    #[test]
    fn handle_disabled_while_searching_and_starts_a_drag_when_enabled() {
        let wf = filter_of(5);
        let sweep = |reorder: bool| -> bool {
            let mut any = false;
            let mut x = 380.0;
            while x < 533.0 {
                let (started, actions) = drag_starts_at(&wf, reorder, x);
                assert!(actions.is_empty(), "a drag start emits no action");
                any |= started;
                x += 3.0;
            }
            any
        };
        assert!(!sweep(false), "disabled reorder must not start a drag");
        assert!(
            sweep(true),
            "enabled reorder must start a drag on the handle"
        );
    }

    #[test]
    fn drag_start_collapses_every_expanded_row() {
        let wf = filter_of(5);
        let mut found = false;
        let mut x = 380.0;
        while x < 533.0 && !found {
            let ctx = headless_ctx();
            let mut list = ListUi {
                expanded: Some(wf.rows()[3].id),
                ..Default::default()
            };
            let y = row_height() / 2.0;
            run_frame(&ctx, input(vec![]), &wf, &mut list, true);
            run_frame(
                &ctx,
                input(press_events(Pos2::new(x, y))),
                &wf,
                &mut list,
                true,
            );
            for step in 1..=3 {
                run_frame(
                    &ctx,
                    input(vec![Event::PointerMoved(Pos2::new(
                        x,
                        y + 5.0 * step as f32,
                    ))]),
                    &wf,
                    &mut list,
                    true,
                );
            }
            if let Some(d) = list.drag {
                found = true;
                assert_eq!(d.src, wf.rows()[0].id);
                assert!(list.expanded.is_none());
            }
            x += 3.0;
        }
        assert!(found);
    }

    #[test]
    fn a_tap_on_the_handle_does_nothing() {
        let wf = filter_of(5);
        let mut hx = None;
        let mut x = 380.0;
        while x < 533.0 {
            if drag_starts_at(&wf, true, x).0 {
                hx = Some(x);
                break;
            }
            x += 3.0;
        }
        let x = hx.expect("a handle x");
        let ctx = headless_ctx();
        let open = wf.rows()[2].id;
        let mut list = ListUi {
            expanded: Some(open),
            ..Default::default()
        };
        let y = row_height() / 2.0;
        run_frame(&ctx, input(vec![]), &wf, &mut list, true);
        let (f, _) = run_frame(
            &ctx,
            input(click_events(Pos2::new(x, y))),
            &wf,
            &mut list,
            true,
        );
        assert!(f.actions.is_empty());
        assert!(list.drag.is_none());
        assert_eq!(list.expanded, Some(open), "the tap is not a row click");
    }

    /// WR-07: collapsing or switching the expanded row orphans the old row's card state, so a
    /// stale hint or `Named` mode does not come back when it is reopened.
    #[test]
    fn collapsing_a_row_orphans_card_state() {
        let wf = filter_of(2);
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        let mut list = ListUi {
            expanded: Some(wf.rows()[0].id),
            ..Default::default()
        };
        let visible = rule_entries(&wf);
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
        list.drag = Some(DragState {
            src: id,
            handle: egui::Id::new("h"),
            had_focus: true,
            injected: None,
            last_slot: None,
        });
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
        assert!(list.drag.is_none());
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
    fn expanded_body_orders_name_conditions_style() {
        let src = include_str!("rule_list.rs");
        let start = src.find("fn expanded_body(").unwrap();
        // `\n}` (not `\n}\n`) so a CRLF checkout (core.autocrlf) still finds the function end.
        let end = start + src[start..].find("\n}").unwrap();
        let f = &src[start..end];
        let name = f.find("\"Name\", None").unwrap();
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
                "/tests/fixtures/seed-styles.json"
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
    fn duplicate_and_delete_are_skin_buttons() {
        let wf = WorkingFilter::from_text("Show\n    Class \"Rings\"\n");
        let id = wf.rows()[0].id;
        let (mut dup, mut del) = (false, false);
        for y in (20..1200).step_by(4) {
            for x in [30.0f32, 150.0, 200.0, 300.0] {
                let ctx = headless_ctx();
                let mut list = ListUi::default();
                list.expanded = Some(id);
                run_frame(&ctx, tall_input(vec![]), &wf, &mut list, true);
                run_frame(&ctx, tall_input(vec![]), &wf, &mut list, true);
                let (f, _) = run_frame(
                    &ctx,
                    tall_input(click_events(Pos2::new(x, y as f32))),
                    &wf,
                    &mut list,
                    true,
                );
                assert!(
                    !f.actions.iter().any(|a| matches!(a, RowAction::Delete(_))),
                    "Delete only asks, the popup deletes"
                );
                dup |= f.actions.contains(&RowAction::Duplicate(id));
                del |= list.confirm_delete == Some(id);
            }
        }
        assert!(dup, "Duplicate emits RowAction::Duplicate");
        assert!(del, "Delete arms confirm_delete");
        let src = include_str!("rule_list.rs");
        assert!(src.contains("\"Duplicate\", 112.0") && src.contains("\"Delete\", 112.0"));
    }

    // ---- sections (Phase 32) ----

    fn banner(name: &str) -> String {
        let r = crate::filter::section::RULE_LINE;
        format!("{r}\n# ff:section {name}\n{r}\n\n")
    }

    /// One headerless rule, then Leveling (1 rule) and Gear (1 rule).
    fn sectioned_text() -> String {
        format!(
            "Show # r1\n  Class == \"Rings\"\n\n{}Show # r2\n  Class == \"Belts\"\n\n{}Show # r3\n  Class == \"Amulets\"\n\n",
            banner("Leveling"),
            banner("Gear")
        )
    }

    fn with_economy_text() -> String {
        let r = crate::filter::section::RULE_LINE;
        let b = crate::filter::section::BEGIN_LINE;
        format!(
            "{}{r}\n{b}\n{r}\n\nShow # Currency S\n  Class == \"Stackable Currency\"\n\n",
            sectioned_text()
        )
    }

    fn section_frame(
        ctx: &egui::Context,
        raw: RawInput,
        wf: &mut WorkingFilter,
        list: &mut ListUi,
        reorder: bool,
        collapsed: &dyn Fn(&SectionRow) -> bool,
    ) -> ListFrame {
        let entries = wf.entries("", collapsed);
        let mut out = ListFrame::default();
        ctx.run_ui(raw, |ui| {
            let area = Rect::from_min_size(Pos2::ZERO, vec2(535.0, 600.0));
            let mut child = ui.new_child(UiBuilder::new().max_rect(area));
            out = show_list(&mut child, wf, &entries, list, reorder, None);
        })
        .drop_without_applying_deltas();
        out
    }

    /// Click pos on a fresh context (warm-up frames first) and return everything the click
    /// frames produced.
    fn click_at(
        wf: &mut WorkingFilter,
        list: &mut ListUi,
        pos: Pos2,
        reorder: bool,
        collapsed: &dyn Fn(&SectionRow) -> bool,
    ) -> (Vec<RowAction>, Vec<(String, bool, bool)>) {
        let ctx = headless_ctx();
        for _ in 0..2 {
            section_frame(&ctx, input(vec![]), wf, list, reorder, collapsed);
        }
        let mut actions = Vec::new();
        let mut toggled = Vec::new();
        for e in click_events(pos).into_iter().map(|e| vec![e]) {
            let f = section_frame(&ctx, input(e), wf, list, reorder, collapsed);
            actions.extend(f.actions);
            toggled.extend(f.toggled);
        }
        (actions, toggled)
    }

    /// Sweep a click across the right part of the header row at y.
    fn sweep_header(
        text: &str,
        y: f32,
        reorder: bool,
        collapsed: &dyn Fn(&SectionRow) -> bool,
    ) -> (Vec<RowAction>, bool, bool) {
        let mut all = Vec::new();
        let (mut renamed, mut confirm) = (false, false);
        let mut x = 330.0;
        while x < 530.0 {
            let mut wf = WorkingFilter::from_text(text);
            let mut list = ListUi::default();
            let (a, _) = click_at(&mut wf, &mut list, Pos2::new(x, y), reorder, collapsed);
            all.extend(a);
            renamed |= list.editing_section.is_some();
            confirm |= list.confirm_economy_remove;
            x += 2.0;
        }
        (all, renamed, confirm)
    }

    #[test]
    fn section_count_and_accessible_name() {
        assert_eq!(section_count_text(3, None), "3 rules");
        assert_eq!(section_count_text(1, None), "1 rule");
        assert_eq!(section_count_text(0, None), "0 rules");
        assert_eq!(section_count_text(12, Some(2)), "2 of 12 rules");
        assert_eq!(
            section_accessible_name("Leveling", 3, true),
            "Section Leveling, 3 rules, collapsed"
        );
        assert_eq!(
            section_accessible_name("Leveling", 1, false),
            "Section Leveling, 1 rule, expanded"
        );
        assert_eq!(SECTION_HEADER_HEIGHT, 36.0);
    }

    #[test]
    fn header_layout_matches_the_spec() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(600.0, 36.0));
        let l = header_layout(rect, 1.0);
        assert_eq!(
            l.plate,
            Rect::from_min_size(Pos2::new(4.0, 2.0), egui::vec2(27.0, 32.0))
        );
        assert_eq!(
            l.bar,
            Rect::from_min_max(Pos2::new(35.0, 0.0), Pos2::new(600.0, 36.0))
        );
        assert_eq!(l.title_x, 95.0);
    }

    #[test]
    fn header_layout_fills_the_row_and_snaps() {
        for ppp in [1.0_f32, 1.25, 1.5, 2.0] {
            for w in [120.0_f32, 500.0, 643.0] {
                let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(w, 36.0));
                let l = header_layout(rect, ppp);
                assert_eq!(l.bar.height(), SECTION_HEADER_HEIGHT);
                assert!((l.bar.max.x - rect.max.x).abs() < 1.0);
                for v in [
                    l.plate.min.x,
                    l.plate.min.y,
                    l.plate.max.x,
                    l.plate.max.y,
                    l.bar.min.x,
                    l.bar.max.x,
                ] {
                    let p = v * ppp;
                    assert!((p - p.round()).abs() < 1e-3, "{v} at {ppp}");
                }
            }
        }
    }

    #[test]
    fn header_bar_caps_keep_the_art_aspect() {
        let wide = Rect::from_min_max(Pos2::new(35.0, 0.0), Pos2::new(600.0, 36.0));
        let r = slice::three_h_rects(wide, BAR_LEFT_PX, BAR_RIGHT_PX, 1.0);
        assert!((r[0].width() - 63.36).abs() < 1.0);
        assert!((r[2].width() - 72.5).abs() < 1.0);
        let narrow = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 36.0));
        let r = slice::three_h_rects(narrow, BAR_LEFT_PX, BAR_RIGHT_PX, 1.0);
        assert!(r[0].max.x <= r[2].min.x);
    }

    #[test]
    fn rules_above_the_first_divider_have_no_header() {
        let mut wf = WorkingFilter::from_text(&sectioned_text());
        let entries = wf.entries("", |_| false);
        assert!(matches!(entries[0], ListEntry::Rule(0)));
        assert!(matches!(entries[1], ListEntry::Header { .. }));
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        let f = section_frame(&ctx, input(vec![]), &mut wf, &mut list, true, &|_| false);
        assert_eq!(f.drawn, entries.len());
    }

    #[test]
    fn clicking_a_header_toggles_without_touching_the_filter() {
        let mut wf = WorkingFilter::from_text(&sectioned_text());
        let mut list = ListUi::default();
        // Row 1 is 52pt, so the Leveling header spans y 52..88; x 200 is clear of the cluster.
        let (actions, toggled) =
            click_at(&mut wf, &mut list, Pos2::new(200.0, 70.0), true, &|_| false);
        assert_eq!(toggled, [("Leveling".to_string(), false, true)]);
        assert!(actions.is_empty(), "{actions:?}");
        assert!(!wf.is_dirty());
        // A collapsed header expands.
        let (_, toggled) = click_at(&mut wf, &mut list, Pos2::new(200.0, 70.0), true, &|s| {
            s.name == "Leveling"
        });
        assert_eq!(toggled, [("Leveling".to_string(), false, false)]);
    }

    #[test]
    fn collapsed_sections_hide_their_rules_and_headers_do_not_alternate() {
        let mut wf = WorkingFilter::from_text(&sectioned_text());
        let all = wf.entries("", |_| false).len();
        let folded = wf.entries("", |s| s.name == "Leveling").len();
        assert_eq!(all, 5);
        assert_eq!(folded, 4);
    }

    #[test]
    fn user_header_cluster_emits_remove_move_and_rename() {
        let text = sectioned_text();
        let wf = WorkingFilter::from_text(&text);
        let sec = wf.sections()[0].clone();
        assert_eq!(sec.name, "Leveling");
        let (actions, renamed, _) = sweep_header(&text, 70.0, true, &|_| false);
        assert!(actions.contains(&RowAction::RemoveSection(sec.ordinal)));
        assert_eq!(
            actions.contains(&RowAction::MoveSection(sec.ordinal, Dir::Up)),
            sec.can_move_up
        );
        assert_eq!(
            actions.contains(&RowAction::MoveSection(sec.ordinal, Dir::Down)),
            sec.can_move_down
        );
        assert!(sec.can_move_up && sec.can_move_down);
        assert!(renamed, "the pencil opens the name box");
        // With the search active the arrows do nothing.
        let (actions, _, _) = sweep_header(&text, 70.0, false, &|_| false);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, RowAction::MoveSection(..)))
        );
        assert!(actions.contains(&RowAction::RemoveSection(sec.ordinal)));
    }

    fn rename_frames(keys: Vec<Vec<Event>>) -> (Vec<RowAction>, ListUi) {
        let mut wf = WorkingFilter::from_text(&sectioned_text());
        let ord = wf.sections()[0].ordinal;
        let ctx = headless_ctx();
        let mut list = ListUi {
            editing_section: Some(ord),
            section_draft: "Leveling".into(),
            section_focus: true,
            ..Default::default()
        };
        let mut actions = Vec::new();
        for _ in 0..3 {
            let f = section_frame(&ctx, input(vec![]), &mut wf, &mut list, true, &|_| false);
            actions.extend(f.actions);
        }
        for ev in keys {
            let f = section_frame(&ctx, input(ev), &mut wf, &mut list, true, &|_| false);
            actions.extend(f.actions);
        }
        (actions, list)
    }

    fn key(k: egui::Key) -> Vec<Event> {
        vec![
            Event::Key {
                key: k,
                physical_key: Some(k),
                pressed: true,
                repeat: false,
                modifiers: Modifiers::default(),
            },
            Event::Key {
                key: k,
                physical_key: Some(k),
                pressed: false,
                repeat: false,
                modifiers: Modifiers::default(),
            },
        ]
    }

    #[test]
    fn rename_mode_commits_on_enter_and_cancels_on_escape() {
        let (actions, list) = rename_frames(vec![
            vec![Event::Text("Gear".into())],
            key(egui::Key::Enter),
            vec![],
        ]);
        let renames: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, RowAction::RenameSection { .. }))
            .collect();
        assert_eq!(renames.len(), 1, "{actions:?}");
        assert!(
            matches!(renames[0], RowAction::RenameSection { name, .. } if name == "Gear"),
            "typing replaces the selected text: {renames:?}"
        );
        assert!(list.editing_section.is_none());

        let (actions, list) = rename_frames(vec![
            vec![Event::Text("Nope".into())],
            key(egui::Key::Escape),
            vec![],
        ]);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, RowAction::RenameSection { .. })),
            "{actions:?}"
        );
        assert!(list.editing_section.is_none());
    }

    #[test]
    fn economy_header_is_locked_and_its_rows_only_expand() {
        let text = with_economy_text();
        let mut wf = WorkingFilter::from_text(&text);
        let econ = wf.sections().iter().position(|s| s.economy).unwrap();
        assert_eq!(wf.sections().len(), 3);
        let folded = |s: &SectionRow| s.economy;
        // Collapsed Economy: header only after the user sections' rules.
        let entries = wf.entries("", folded);
        assert!(matches!(
            entries.last(),
            Some(ListEntry::Header { section, collapsed: true, .. }) if *section == econ
        ));
        // Expanded: the economy row follows its header.
        let entries = wf.entries("", |_| false);
        assert!(matches!(entries.last(), Some(ListEntry::EconomyRule(0))));
        // y of the economy header: r1 52 + Leveling 36 + r2 52 + Gear 36 + r3 52.
        let y = 52.0 + 36.0 + 52.0 + 36.0 + 52.0 + 18.0;
        let (actions, renamed, confirm) = sweep_header(&text, y, true, &|_| false);
        assert!(actions.is_empty(), "{actions:?}");
        assert!(!renamed);
        assert!(!confirm, "the economy header is display-only: no x");
        // A click on an economy row emits nothing; it only expands the row (read-only).
        let before = wf.text();
        let mut list = ListUi {
            expanded_h: 0.0,
            ..Default::default()
        };
        let (actions, toggled) = click_at(
            &mut wf,
            &mut list,
            Pos2::new(200.0, y + 36.0 + 26.0),
            true,
            &|_| false,
        );
        assert!(actions.is_empty() && toggled.is_empty(), "{actions:?}");
        assert_eq!(list.expanded, Some(wf.economy_rows()[0].id));
        // The next frames draw the body under the row and measure it.
        let ctx = headless_ctx();
        for _ in 0..2 {
            let f = section_frame(&ctx, input(vec![]), &mut wf, &mut list, true, &|_| false);
            assert!(f.actions.is_empty(), "{:?}", f.actions);
        }
        assert!(
            list.expanded_h > ROW_H + 60.0,
            "the body was drawn: {}",
            list.expanded_h
        );
        assert_eq!(wf.text(), before);
        assert!(!wf.is_dirty());
    }

    #[test]
    fn browsing_economy_rules_changes_nothing() {
        let text = economy_browse::rich_economy_text();
        let wf = WorkingFilter::from_text(&text);
        let before = wf.text();
        let report = economy_browse::browse(&wf);
        assert_eq!(report.rules, 2);
        assert!(report.actions.is_empty(), "{:?}", report.actions);
        assert!(
            report.enabled_in_body.is_empty(),
            "enabled controls in a read-only body: {:?}",
            report.enabled_in_body
        );
        // Conditions, lists, operators, swatches, sliders, checks and pickers were all drawn.
        assert!(report.disabled_in_body > 40, "{report:?}");
        assert_eq!(
            report.popups_or_focus, 0,
            "no picker opens, nothing takes focus"
        );
        assert_eq!(report.list_changed, 0, "the list state is untouched");
        // Ten base types, uncapped (a capped list would be 7 rows), plus the style lines.
        assert!(report.max_expanded_h > ROW_H + 10.0 * 28.0, "{report:?}");
        assert_eq!(wf.text(), before);
        assert!(!wf.is_dirty());
    }

    #[test]
    fn economy_rule_header_click_collapses_and_search_still_finds_it() {
        let mut wf = WorkingFilter::from_text(&economy_browse::rich_economy_text());
        let id = wf.economy_rows()[0].id;
        let hits = wf.entries("divine orb", |_| true);
        assert!(hits.contains(&ListEntry::EconomyRule(0)), "{hits:?}");
        let entries = vec![ListEntry::EconomyRule(0)];
        let ctx = headless_ctx();
        let mut list = ListUi {
            expanded: Some(id),
            ..Default::default()
        };
        let mut actions = Vec::new();
        let mut frame = |events: Vec<Event>, list: &mut ListUi| {
            ctx.run_ui(tall_input(events), |ui| {
                let area = Rect::from_min_size(Pos2::ZERO, vec2(535.0, 1200.0));
                let mut child = ui.new_child(UiBuilder::new().max_rect(area));
                actions.extend(show_list(&mut child, &wf, &entries, list, true, None).actions);
            })
            .drop_without_applying_deltas();
        };
        for _ in 0..2 {
            frame(vec![], &mut list);
        }
        assert_eq!(list.expanded, Some(id));
        for e in click_events(Pos2::new(200.0, 20.0)) {
            frame(vec![e], &mut list);
        }
        assert!(list.expanded.is_none(), "a header click collapses it");
        assert!(actions.is_empty(), "{actions:?}");
    }

    #[test]
    fn economy_header_click_toggles_with_the_economy_flag() {
        let text = with_economy_text();
        let mut wf = WorkingFilter::from_text(&text);
        let mut list = ListUi::default();
        let y = 52.0 + 36.0 + 52.0 + 36.0 + 52.0 + 18.0;
        let (_, toggled) = click_at(&mut wf, &mut list, Pos2::new(200.0, y), true, &|s| {
            s.economy
        });
        assert_eq!(
            toggled,
            [(
                crate::overlay::rules::ECONOMY_DISPLAY_NAME.to_string(),
                true,
                false
            )]
        );
    }

    #[test]
    fn section_above_button_emits_add_section_above() {
        let wf = WorkingFilter::from_text("Show\n    Class \"Rings\"\n");
        let id = wf.rows()[0].id;
        let mut got = false;
        for y in (20..1200).step_by(4) {
            for x in [30.0f32, 100.0] {
                let ctx = headless_ctx();
                let mut list = ListUi {
                    expanded: Some(id),
                    ..Default::default()
                };
                run_frame(&ctx, tall_input(vec![]), &wf, &mut list, true);
                run_frame(&ctx, tall_input(vec![]), &wf, &mut list, true);
                let (f, _) = run_frame(
                    &ctx,
                    tall_input(click_events(Pos2::new(x, y as f32))),
                    &wf,
                    &mut list,
                    true,
                );
                got |= f.actions.contains(&RowAction::AddSectionAbove(id));
            }
        }
        assert!(got, "Section above emits AddSectionAbove");
        let src = include_str!("rule_list.rs");
        assert!(src.contains("\"Section above\", 128.0"));
    }

    #[test]
    fn toolbar_buttons_click_and_disable() {
        let ctx = headless_ctx();
        let skin = crate::skin::install(&ctx);
        let mut query = String::new();
        let mut rects = (
            ToolbarClick::None,
            Rect::NOTHING,
            Rect::NOTHING,
            Rect::NOTHING,
        );
        ctx.run_ui(input(vec![]), |ui| {
            rects = search_row_rects(ui, &skin, &mut query, true, 5);
        })
        .drop_without_applying_deltas();
        let run = |enabled: bool, pos: Pos2| {
            let ctx = headless_ctx();
            let skin = crate::skin::install(&ctx);
            let mut q = String::new();
            let mut hit = ToolbarClick::None;
            for e in std::iter::once(vec![]).chain(click_events(pos).into_iter().map(|e| vec![e])) {
                ctx.run_ui(input(e), |ui| {
                    let c = search_row(ui, &skin, &mut q, enabled, 5);
                    if c != ToolbarClick::None {
                        hit = c;
                    }
                })
                .drop_without_applying_deltas();
            }
            hit
        };
        assert_eq!(run(true, rects.2.center()), ToolbarClick::AddRule);
        assert_eq!(run(true, rects.3.center()), ToolbarClick::AddSection);
        assert_eq!(run(false, rects.2.center()), ToolbarClick::None);
        assert_eq!(run(false, rects.3.center()), ToolbarClick::None);
    }

    #[test]
    fn wr02_marker_shows_on_a_shadowing_rule_only() {
        let wf = WorkingFilter::from_text("Show\n\nShow\n  Class \"Rings\"\n\nShow\n");
        assert_eq!(wf.shadowing().len(), 1);
        assert!(wf.shadowing().contains(&wf.rows()[0].id));
        let ctx = headless_ctx();
        let mut list = ListUi::default();
        for _ in 0..3 {
            ctx.run_ui(input(vec![]), |ui| {
                let area = Rect::from_min_size(Pos2::ZERO, vec2(535.0, 600.0));
                let mut child = ui.new_child(UiBuilder::new().max_rect(area));
                let entries = rule_entries(&wf);
                show_list(&mut child, &wf, &entries, &mut list, true, None);
            })
            .drop_without_applying_deltas();
        }
        // flags_for never carries it: the marker is computed from shadowing().
        assert!(
            !flags_for(&wf.rows()[0])
                .iter()
                .any(|f| f.text == "matches every item")
        );
    }

    #[test]
    fn searching_shows_collapsed_matches_under_their_header() {
        let mut wf = WorkingFilter::from_text(&sectioned_text());
        let entries = wf.entries("belts", |s| s.name == "Leveling");
        assert_eq!(entries.len(), 2);
        assert!(matches!(
            entries[0],
            ListEntry::Header {
                matches: Some(1),
                collapsed: false,
                ..
            }
        ));
        assert!(matches!(entries[1], ListEntry::Rule(1)));
    }

    // ---- drag and drop ----

    const LIST_H: f32 = 600.0;

    /// A multi-frame rig for drags: a 535 x 700 screen, the list in its top 600pt, real time.
    struct DragRig {
        ctx: egui::Context,
        time: f64,
        wf: WorkingFilter,
        list: ListUi,
        entries: Vec<ListEntry>,
        actions: Vec<RowAction>,
        scroll_y: f32,
        escape_seen_after: bool,
        focused: bool,
        reorder: bool,
        /// The last frame's paint list, in paint order.
        shapes: Vec<egui::epaint::ClippedShape>,
    }

    /// Centre x of a rule row's handle.
    fn handle_x() -> f32 {
        535.0 - crate::skin::widgets::SCROLLBAR_WIDTH - 4.0 - ROW_PAD - 14.0
    }

    impl DragRig {
        fn new(wf: WorkingFilter) -> Self {
            let entries = rule_entries(&wf);
            Self::with_entries(wf, entries)
        }

        fn with_entries(wf: WorkingFilter, entries: Vec<ListEntry>) -> Self {
            let mut rig = DragRig {
                ctx: headless_ctx(),
                time: 0.0,
                wf,
                list: ListUi::default(),
                entries,
                actions: vec![],
                scroll_y: 0.0,
                escape_seen_after: false,
                focused: true,
                reorder: true,
                shapes: vec![],
            };
            rig.frame(vec![]);
            rig
        }

        fn frame(&mut self, events: Vec<Event>) {
            self.time += 1.0 / 60.0;
            let raw = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(535.0, 700.0))),
                time: Some(self.time),
                predicted_dt: 1.0 / 60.0,
                events,
                viewports: [(
                    egui::ViewportId::ROOT,
                    egui::ViewportInfo {
                        focused: Some(self.focused),
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..Default::default()
            };
            let mut out = ListFrame::default();
            let mut esc = false;
            let (wf, entries, list, reorder) =
                (&self.wf, &self.entries, &mut self.list, self.reorder);
            let mut full = self.ctx.run_ui(raw, |ui| {
                let area = Rect::from_min_size(Pos2::ZERO, vec2(535.0, LIST_H));
                let mut child = ui.new_child(UiBuilder::new().max_rect(area));
                out = show_list(&mut child, wf, entries, list, reorder, None);
                esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
            });
            self.shapes = std::mem::take(&mut full.shapes);
            full.drop_without_applying_deltas();
            self.escape_seen_after = esc;
            self.scroll_y = out.scroll_y;
            self.actions.extend(out.actions);
        }

        fn hover(&mut self, pos: Pos2) {
            self.frame(vec![Event::PointerMoved(pos)]);
        }

        fn release(&mut self, pos: Pos2) {
            self.frame(vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::default(),
            }]);
        }

        /// Press on rule `n`'s handle and move 10pt: the drag has started.
        fn start_drag(&mut self, n: usize) {
            let p = Pos2::new(handle_x(), n as f32 * ROW_H + ROW_H / 2.0);
            self.frame(press_events(p));
            for step in 1..=3 {
                self.hover(p + vec2(0.0, 5.0 * step as f32));
            }
            assert!(self.list.drag.is_some(), "drag did not start");
        }

        /// Move to `y` and let two frames pass so the slot settles.
        fn move_to(&mut self, y: f32) -> Pos2 {
            let p = Pos2::new(300.0, y);
            self.hover(p);
            self.hover(p);
            p
        }
    }

    #[test]
    fn wheel_scrolls_the_list_during_a_drag() {
        let mut rig = DragRig::new(filter_of(60));
        rig.start_drag(3);
        let p = rig.move_to(300.0);
        let before = rig.scroll_y;
        for _ in 0..10 {
            rig.frame(vec![
                Event::PointerMoved(p),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, -60.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: Modifiers::default(),
                },
            ]);
        }
        for _ in 0..5 {
            rig.hover(p);
        }
        assert!(
            rig.scroll_y > before + 50.0,
            "wheel did not scroll: {before} -> {}",
            rig.scroll_y
        );
        assert!(rig.list.drag.is_some());
    }

    #[test]
    fn edge_autoscroll_survives_the_source_leaving_the_range_and_drop_emits_one_move() {
        let mut rig = DragRig::new(filter_of(60));
        rig.start_drag(3);
        let p = Pos2::new(300.0, LIST_H - 5.0);
        for _ in 0..40 {
            rig.hover(p);
        }
        assert!(rig.scroll_y > 220.0, "scrolled {}", rig.scroll_y);
        assert!(rig.list.drag.is_some(), "drag survives virtualization");
        // Back to mid list: the marker slot settles, then release over it.
        let p = rig.move_to(300.0);
        let slot = rig.list.drag.unwrap().last_slot.expect("a valid slot");
        let se = slot_entries_of(&rig.wf, &rig.entries);
        let dest = move_dest(drag_reorder::slot_target(&se, slot).unwrap(), rig.wf.rows()).unwrap();
        rig.release(p);
        assert!(rig.list.drag.is_none());
        assert_eq!(
            rig.actions,
            vec![RowAction::MoveTo {
                id: rig.wf.rows()[3].id,
                dest
            }]
        );
    }

    #[test]
    fn release_outside_the_list_or_with_the_pointer_gone_cancels() {
        let mut rig = DragRig::new(filter_of(10));
        rig.start_drag(2);
        let p = Pos2::new(300.0, 650.0);
        rig.hover(p);
        rig.hover(p);
        rig.release(p);
        assert!(rig.list.drag.is_none());
        assert!(rig.actions.is_empty(), "{:?}", rig.actions);

        let mut rig = DragRig::new(filter_of(10));
        rig.start_drag(2);
        rig.move_to(300.0);
        rig.frame(vec![Event::PointerGone]);
        rig.release(Pos2::new(300.0, 300.0));
        assert!(rig.actions.is_empty(), "{:?}", rig.actions);
        assert!(rig.list.drag.is_none());
    }

    #[test]
    fn escape_cancels_a_drag_and_is_consumed() {
        let mut rig = DragRig::new(filter_of(10));
        rig.start_drag(2);
        rig.move_to(300.0);
        rig.frame(vec![Event::Key {
            key: egui::Key::Escape,
            physical_key: Some(egui::Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        }]);
        assert!(rig.list.drag.is_none());
        assert!(!rig.escape_seen_after, "Escape was consumed");
        rig.release(Pos2::new(300.0, 300.0));
        assert!(rig.actions.is_empty());
    }

    #[test]
    fn losing_focus_cancels_a_drag_that_started_focused() {
        let mut rig = DragRig::new(filter_of(10));
        rig.start_drag(2);
        assert!(rig.list.drag.unwrap().had_focus);
        rig.move_to(300.0);
        rig.focused = false;
        rig.frame(vec![]);
        assert!(rig.list.drag.is_none());
        rig.release(Pos2::new(300.0, 300.0));
        assert!(rig.actions.is_empty());
    }

    #[test]
    fn drop_marker_is_painted_above_the_shadow() {
        let mut rig = DragRig::new(filter_of(10));
        rig.start_drag(2);
        rig.move_to(6.0 * ROW_H + 10.0);
        let slot = rig.list.drag.unwrap().last_slot.expect("a slot");
        // No headers and no scroll: boundary `slot` is at y = slot * ROW_H. The shadow is
        // centred on the pointer, so it covers the line; the line must still be on top.
        let p = Pos2::new(200.0, slot as f32 * ROW_H);
        assert_eq!(
            drag_reorder::top_fill_at(&rig.shapes, p),
            Some(pal::GOLD),
            "gold line on top at boundary {slot}"
        );
    }

    #[test]
    fn reorder_switched_off_mid_drag_cancels_it() {
        let mut rig = DragRig::new(filter_of(10));
        rig.start_drag(2);
        rig.move_to(300.0);
        assert!(rig.list.drag.unwrap().last_slot.is_some());
        // A search typed into a still-focused box turns reordering off while the handle is held.
        rig.reorder = false;
        rig.frame(vec![]);
        assert!(rig.list.drag.is_none());
        rig.release(Pos2::new(300.0, 300.0));
        assert!(rig.actions.is_empty(), "{:?}", rig.actions);
    }

    #[test]
    fn dropping_next_to_the_source_is_a_no_op() {
        let mut rig = DragRig::new(filter_of(10));
        rig.start_drag(3);
        // Boundaries 3 and 4 (y 156 and 208) are the source's own: no slot, so no marker.
        let p = rig.move_to(3.0 * ROW_H + 26.0);
        assert_eq!(rig.list.drag.unwrap().last_slot, None);
        rig.release(p);
        assert!(rig.actions.is_empty(), "{:?}", rig.actions);
        assert!(rig.list.drag.is_none());
    }

    #[test]
    fn economy_boundaries_are_never_slots_and_end_sits_above_the_economy_header() {
        let mut wf = WorkingFilter::from_text(&with_economy_text());
        let entries = wf.entries("", |s| s.economy);
        let mut rig = DragRig::with_entries(wf, entries);
        rig.start_drag(0);
        // Far below everything: the nearest valid boundary is the one above the economy header.
        let p = rig.move_to(500.0);
        let slot = rig.list.drag.unwrap().last_slot;
        let se = slot_entries_of(&rig.wf, &rig.entries);
        let end = se
            .iter()
            .position(|e| *e == SlotEntry::EconomyHeader)
            .unwrap();
        assert_eq!(slot, Some(end), "boundary above the economy header");
        rig.release(p);
        assert_eq!(
            rig.actions,
            vec![RowAction::MoveTo {
                id: rig.wf.rows()[0].id,
                dest: MoveDest::End
            }]
        );
    }

    #[test]
    fn dropping_after_a_collapsed_header_makes_the_rule_first_and_keeps_it_collapsed() {
        let mut wf = WorkingFilter::from_text(&sectioned_text());
        let lev = wf
            .sections()
            .iter()
            .position(|s| s.name == "Leveling")
            .unwrap();
        let ordinal = wf.sections()[lev].ordinal;
        let folded = |s: &SectionRow| s.name == "Leveling";
        let entries = wf.entries("", folded);
        // r1, [Leveling] (r2 hidden), Gear, r3.
        assert!(matches!(
            entries[1],
            ListEntry::Header {
                collapsed: true,
                ..
            }
        ));
        let mut rig = DragRig::with_entries(wf, entries);
        rig.start_drag(0);
        // Boundary 2 sits right under the collapsed Leveling header: y = 52 + 36.
        let p = rig.move_to(ROW_H + SECTION_HEADER_HEIGHT + 2.0);
        rig.release(p);
        assert_eq!(
            rig.actions,
            vec![RowAction::MoveTo {
                id: rig.wf.rows()[0].id,
                dest: MoveDest::AfterBanner(ordinal)
            }]
        );
        let action = rig.actions[0].clone();
        assert!(rig.wf.apply_action(action).changed);
        let lev = rig
            .wf
            .sections()
            .iter()
            .position(|s| s.name == "Leveling")
            .unwrap();
        assert_eq!(rig.wf.sections()[lev].rules, 2);
        let after = rig.wf.entries("", folded);
        assert!(matches!(
            after[0],
            ListEntry::Header {
                collapsed: true,
                ..
            }
        ));
    }

    #[test]
    fn dropping_a_grouped_rule_moves_it_by_its_leader_id() {
        let text = "# ff:group abc 1/2
Show
  Class \"Body Armours\"
  BaseArmour > 0

# ff:group abc 2/2
Show
  Class \"Body Armours\"
  BaseEvasion > 0

Show # a
  Class \"Rings\"

Show # b
  Class \"Belts\"

";
        let wf = WorkingFilter::from_text(text);
        assert_eq!(wf.rows().len(), 3);
        assert_eq!(wf.rows()[0].group_size, 2);
        let mut rig = DragRig::new(wf);
        rig.start_drag(0);
        let p = rig.move_to(3.0 * ROW_H);
        rig.release(p);
        let lead = rig.wf.rows()[0].id;
        assert_eq!(rig.actions.len(), 1, "{:?}", rig.actions);
        assert!(matches!(
            rig.actions[0],
            RowAction::MoveTo { id, .. } if id == lead
        ));
    }

    #[test]
    fn the_shadow_aligns_to_the_list_edge_and_is_one_row_tall() {
        let mut rig = DragRig::new(filter_of(10));
        rig.start_drag(2);
        rig.move_to(300.0);
        let r = rig
            .ctx
            .memory(|m| m.area_rect(egui::Id::new(("ff-drag-shadow", "rules"))))
            .expect("shadow area");
        assert!(r.left().abs() < 0.5, "{r:?}");
        assert!((r.height() - ROW_H).abs() < 0.5, "{r:?}");
        // The source row stays in the list (greyed): still one entry per rule.
        assert_eq!(rig.list.drag.unwrap().src, rig.wf.rows()[2].id);
    }
}
