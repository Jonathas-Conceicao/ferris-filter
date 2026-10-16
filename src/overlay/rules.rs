//! Pure working model of the rule list: no GUI types, no file IO.
//!
//! [`WorkingFilter`] owns the edited [`Document`], the last-applied text (the Revert target) and a
//! cached `Vec<RuleRow>`. Rows are rebuilt once per mutation, never per frame, so a 500-block
//! filter stays responsive. Every edit goes through the `filter::Document` structural ops; this
//! module only translates [`RowAction`]s into them and projects the result for the view.
//!
//! Naming rule (CONTEXT, Rule naming): a row's name comes only from the `# ff:name` setting. The
//! foreign header comment text (`$type->currency %D4 !slug`) is never a name, never part of a
//! label and never searched.

use std::collections::{HashMap, HashSet};

use super::apply::{STYLE_REMOVED_STATUS, styles_unavailable_message};
use crate::dynamic::{BlockingError, blocking_errors};
use crate::filter::group::{self, GroupIndex};
use crate::filter::keywords::{Keyword, Kind};
use crate::filter::section::{self, SectionSpan};
use crate::filter::tokenize::Operator;
use crate::filter::{
    Block, BlockId, BlockKeyword, DisabledState, Document, FilterDiagnostic, LinePatch, Severity,
    parse as parse_filter,
};
use crate::models::config::{Style, StyleSet};
use crate::styled::{self, StyleReport};

/// Refusal copy for a line edit whose target line changed since the frame started.
pub const STALE_LINE_STATUS: &str =
    "That line changed before the edit landed; nothing was changed.";
/// Refusal of an edit that would change a grouped rule's defence lines outside the chips.
pub const GROUP_DEFENCES_STATUS: &str =
    "This rule spans several blocks; change its defences with the Simple Defences chips.";
/// Refusal copy for line edits on a block whose lines the model cannot structure.
pub const NOT_EDITABLE_STATUS: &str = "This rule's lines can't be edited here.";

/// Which section of the body a new line belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineKind {
    Condition,
    Action,
}

/// Typed role of one body line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Condition,
    Action,
    Modifier,
    Opaque,
}

/// One body line of a block, typed for the editor body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyEntry {
    /// Document line index.
    pub line: usize,
    pub kind: EntryKind,
    pub keyword: Option<&'static Keyword>,
    pub op: Option<Operator>,
    /// Raw value tokens as written, quotes kept.
    pub values: Vec<String>,
    /// Full raw line text.
    pub text: String,
    /// 0-based occurrence of this keyword (case-insensitive) within the block; opaque entries
    /// count among themselves.
    pub ordinal: usize,
}

/// Start of the refusal shown when a style-owned action is edited.
pub const STYLE_LOCKED_STATUS_PREFIX: &str = "Set by style";

fn style_locked_status(name: &str) -> String {
    format!("{STYLE_LOCKED_STATUS_PREFIX} \"{name}\". Remove the style to edit it.")
}

/// Refusal when a line added to the conditions is really an action.
pub const ACTION_IN_CONDITIONS_STATUS: &str = "That line is an action; add it under ACTIONS.";
/// Refusal when a line added to the actions is really a condition.
pub const CONDITION_IN_ACTIONS_STATUS: &str = "That line is a condition; add it under CONDITIONS.";

/// Kind of the known keyword `text` starts with; `None` for unknown keywords or untokenizable
/// text (those are left to the document's own validation).
fn text_kind(text: &str) -> Option<Kind> {
    let t = crate::filter::tokenize::tokenize(text).ok()?;
    let kw = t.keyword?;
    crate::filter::keywords::lookup(&text[kw]).map(|k| k.kind)
}

/// What a rule's `# ff:style` tag means against the loaded styles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleState {
    /// No tag.
    None,
    /// Tag names a style in styles.json.
    Known(String),
    /// Tag names a style that does not exist.
    Unknown(String),
    /// Tag present but styles.json could not be read, so it cannot be judged.
    Unchecked(String),
}

const CANNOT_ENABLE_STATUS: &str =
    "Can't enable: the commented lines are not valid filter grammar.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
}

/// Deferred-mutation vocabulary rows emit during rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowAction {
    SetEnabled(BlockId, bool),
    Move(BlockId, Dir),
    Delete(BlockId),
    Duplicate(BlockId),
    SetKeyword(BlockId, BlockKeyword),
    SetContinue(BlockId, bool),
    SetName(BlockId, String),
    ClearName(BlockId),
    ApplyStyle(BlockId, String),
    RemoveStyle(BlockId),
    AddRule,
    EditLine {
        id: BlockId,
        line: usize,
        expect: String,
        patch: LinePatch,
    },
    AddLine {
        id: BlockId,
        kind: LineKind,
        text: String,
    },
    RemoveLine {
        id: BlockId,
        line: usize,
        expect: String,
    },
    /// Insert a condition line directly after line `after` (whose text must still be `expect`),
    /// e.g. the second bound of a Between.
    InsertLineAfter {
        id: BlockId,
        after: usize,
        expect: String,
        text: String,
    },
    /// Set the Simple Defences selection (a bit set over [`group::COMBOS`]; 0 = no defence
    /// lines). The rule is rewritten as however many blocks that selection needs.
    SetDefenceSelection {
        id: BlockId,
        mask: u8,
    },
}

/// The rule an action is aimed at (`None` for `AddRule`).
fn action_target(action: &RowAction) -> Option<BlockId> {
    match action {
        RowAction::SetEnabled(id, _)
        | RowAction::Move(id, _)
        | RowAction::Delete(id)
        | RowAction::Duplicate(id)
        | RowAction::SetKeyword(id, _)
        | RowAction::SetContinue(id, _)
        | RowAction::SetName(id, _)
        | RowAction::ClearName(id)
        | RowAction::ApplyStyle(id, _)
        | RowAction::RemoveStyle(id)
        | RowAction::EditLine { id, .. }
        | RowAction::AddLine { id, .. }
        | RowAction::RemoveLine { id, .. }
        | RowAction::InsertLineAfter { id, .. }
        | RowAction::SetDefenceSelection { id, .. } => Some(*id),
        RowAction::AddRule => None,
    }
}

/// Keep only the last `EditLine` per `(id, line, patch variant)`, at the position of the last;
/// every other action is untouched and in order.
pub fn coalesce_edits(actions: Vec<RowAction>) -> Vec<RowAction> {
    let key = |a: &RowAction| match a {
        RowAction::EditLine {
            id, line, patch, ..
        } => Some((*id, *line, std::mem::discriminant(patch))),
        _ => None,
    };
    let mut keep = vec![true; actions.len()];
    for i in 0..actions.len() {
        if let Some(k) = key(&actions[i])
            && actions[i + 1..].iter().any(|b| key(b) == Some(k))
        {
            keep[i] = false;
        }
    }
    actions
        .into_iter()
        .zip(keep)
        .filter_map(|(a, k)| k.then_some(a))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionOutcome {
    pub changed: bool,
    pub status: Option<String>,
    pub focus: Option<BlockId>,
    /// Doc line index of a line `AddLine` created.
    pub line: Option<usize>,
}

/// Economy section state, projected: includes the staged add/remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomyView {
    Absent,
    Present { blocks: usize },
    Damaged(String),
}

/// What Apply should do with the economy section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EconomyPlan {
    Keep,
    Refresh,
    Add,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleRow {
    pub id: BlockId,
    /// 1-based file order among rules.
    pub index: usize,
    pub enabled: bool,
    pub keyword: BlockKeyword,
    pub has_continue: bool,
    pub name: Option<String>,
    /// The `# ff:style` tag judged against the loaded styles.
    pub style: StyleState,
    pub label: String,
    /// Lowercase search text: name, label, Class and BaseType values.
    pub haystack: String,
    /// Error diagnostics, enabled blocks only.
    pub errors: Vec<FilterDiagnostic>,
    /// Warning diagnostics, enabled blocks only.
    pub warnings: Vec<FilterDiagnostic>,
    pub cannot_enable: bool,
    pub can_move_up: bool,
    pub can_move_down: bool,
    pub moved_across_continue: bool,
    pub conditions: Vec<String>,
    /// Number of blocks the rule is written as (1 unless it is a `# ff:group` group).
    pub group_size: usize,
    /// The block carries a `# ff:group` line but its group is not valid, so it is an ordinary
    /// rule of its own.
    pub group_broken: bool,
    /// The Simple Defences selection (bit set over `group::COMBOS`): the union over the group,
    /// or what a single block's zero-based defence lines match. `None` without defence lines
    /// (single block) or when a defence line is not zero based.
    pub simple_mask: Option<u8>,
    /// Collapsed-row summary: the condition strings joined with `; `, capped at
    /// `SUMMARY_MAX_CHARS`; built at row rebuild, never per frame.
    pub summary: String,
}

/// Longest collapsed-row summary, in characters (the rest is cut with an ellipsis).
pub const SUMMARY_MAX_CHARS: usize = 160;

/// Joins condition strings into the one-line collapsed-row summary.
fn row_summary(conditions: &[String]) -> String {
    let joined = conditions.join("; ");
    match joined.char_indices().nth(SUMMARY_MAX_CHARS) {
        Some((cut, _)) => format!("{}\u{2026}", &joined[..cut]),
        None => joined,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pending {
    Add { blocks: usize },
    Remove,
}

pub struct WorkingFilter {
    doc: Document,
    applied_text: String,
    rows: Vec<RuleRow>,
    base_economy: EconomyView,
    pending: Option<Pending>,
    projected: EconomyView,
    blocking: Vec<BlockingError>,
    /// Grouped rules (several blocks written as one rule), recomputed on every rebuild.
    groups: GroupIndex,
    moved: HashSet<BlockId>,
    generation: u64,
    dirty: bool,
    search_cache: Option<(String, u64, Vec<usize>)>,
    styles: Option<StyleSet>,
    /// The applied (on-disk) style set that legacy name-only tags were written against, so a
    /// freshly parsed document (RELOAD, filter switch) still follows a pending rename.
    legacy_base: Option<StyleSet>,
    styles_error: Option<String>,
    styles_rev: u64,
    pending_cache: Option<(u64, u64, bool)>,
}

fn refused(msg: String) -> ActionOutcome {
    ActionOutcome {
        status: Some(msg),
        ..ActionOutcome::default()
    }
}

/// A successful change (with the block to focus), or the refusal of a failed one.
fn done_or(r: Result<(), crate::filter::EditError>, focus: Option<BlockId>) -> ActionOutcome {
    match r {
        Ok(()) => ActionOutcome {
            changed: true,
            focus,
            ..ActionOutcome::default()
        },
        Err(e) => refused(e.to_string()),
    }
}

/// Number of blocks in a filter text (0 when it does not parse).
pub fn count_blocks(text: &str) -> usize {
    parse_filter(text.as_bytes())
        .map(|d| d.blocks().count())
        .unwrap_or(0)
}

fn parse_text(text: &str) -> Document {
    // `&str` is always valid UTF-8, so parsing cannot fail; the default is a safe empty fallback.
    parse_filter(text.as_bytes()).unwrap_or_default()
}

fn keyword_word(k: BlockKeyword) -> &'static str {
    match k {
        BlockKeyword::Show => "Show",
        BlockKeyword::Hide => "Hide",
        BlockKeyword::Minimal => "Minimal",
    }
}

/// Class and BaseType value lists plus the display strings of every condition line.
struct Extracted {
    class: Vec<String>,
    base_type: Vec<String>,
    /// First keyword names of non-Class/BaseType conditions, in order.
    other_names: Vec<String>,
    conditions: Vec<String>,
}

fn unquote(s: &str) -> &str {
    s.trim_matches('"')
}

/// Collapsed-row text of a Simple Defences selection: `Defences: Full AR, AR/ES`.
fn selection_summary(mask: u8) -> String {
    let labels: Vec<&str> = group::COMBOS
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, (label, _, _))| *label)
        .collect();
    format!("Defences: {}", labels.join(", "))
}

/// [`extract`] of a group's leader: its own defence lines are only one part of the rule, so they
/// are replaced by the whole selection.
fn extract_group(doc: &Document, b: &Block, mask: Option<u8>) -> Extracted {
    let mut x = extract_skipping(doc, b, &group::defence_lines(b));
    if let Some(m) = mask {
        x.conditions.push(selection_summary(m));
    }
    x
}

fn extract(doc: &Document, b: &Block) -> Extracted {
    extract_skipping(doc, b, &[])
}

/// [`extract`] leaving out the body lines in `skip`.
fn extract_skipping(doc: &Document, b: &Block, skip: &[usize]) -> Extracted {
    let mut out = Extracted {
        class: Vec::new(),
        base_type: Vec::new(),
        other_names: Vec::new(),
        conditions: Vec::new(),
    };
    for bl in b.body() {
        let (Some(kw), Some(tok)) = (bl.keyword, bl.tokens.as_ref()) else {
            continue;
        };
        if kw.kind != Kind::Condition || skip.contains(&bl.line) {
            continue;
        }
        let Some(text) = doc.lines().get(bl.line).map(|l| l.text()) else {
            continue;
        };
        // Disabled-block token ranges are relative to the text past the `#` (and one space).
        let base = if b.is_enabled() {
            text
        } else {
            crate::filter::commented_inner(text)
        };
        let values: Vec<String> = tok
            .values
            .iter()
            .filter_map(|r| base.get(r.clone()))
            .map(|v| unquote(v).to_string())
            .collect();
        if kw.name.eq_ignore_ascii_case("Class") || kw.name.eq_ignore_ascii_case("BaseType") {
            let is_class = kw.name.eq_ignore_ascii_case("Class");
            let summary = if values.len() > 4 {
                format!("{}: {} items", kw.name, values.len())
            } else {
                format!("{}: {}", kw.name, values.join(", "))
            };
            out.conditions.push(summary);
            if is_class {
                out.class.extend(values);
            } else {
                out.base_type.extend(values);
            }
        } else {
            out.other_names.push(kw.name.to_string());
            let start = tok.keyword.as_ref().map(|r| r.start);
            let end = tok
                .values
                .last()
                .map(|r| r.end)
                .or_else(|| tok.operator.as_ref().map(|(r, _)| r.end))
                .or_else(|| tok.keyword.as_ref().map(|r| r.end));
            if let (Some(s), Some(e)) = (start, end)
                && let Some(raw) = base.get(s..e)
            {
                out.conditions.push(raw.to_string());
            }
        }
    }
    out
}

/// Rewrite every legacy (name-only) tag in `doc` whose style, as the first of `olds` that knows
/// the tag resolves it, still exists in `new` under another name, to that style's canonical
/// `<id> <name>` in `new`. A tag `new` resolves by id, or by name to that same style, is left
/// alone; so is one whose style is gone. A same-named newcomer in `new` therefore cannot take
/// over a renamed style's tags. True when a tag line changed.
fn migrate_legacy_tags(doc: &mut Document, olds: &[&StyleSet], new: &StyleSet) -> bool {
    let moves: Vec<(BlockId, String)> = doc
        .blocks()
        .filter_map(|b| {
            let tag = doc.meta_get(b.id(), "style")?;
            let first = tag.split_whitespace().next()?;
            if new.name_of_id(first).is_some() {
                return None;
            }
            let id = olds
                .iter()
                .find_map(|old| old.resolve_tag(tag))?
                .style
                .id
                .as_deref()?;
            let now = new.resolve_tag(tag).and_then(|m| m.style.id.as_deref());
            if now == Some(id) {
                return None;
            }
            let name = new.name_of_id(id)?;
            Some((b.id(), new.tag_for(name)?))
        })
        .collect();
    let mut changed = false;
    for (id, tag) in moves {
        changed |= doc.set_meta(id, "style", &tag).is_ok();
    }
    changed
}

impl WorkingFilter {
    pub fn from_text(text: &str) -> Self {
        let mut wf = WorkingFilter {
            doc: parse_text(text),
            applied_text: text.to_string(),
            rows: Vec::new(),
            base_economy: EconomyView::Absent,
            pending: None,
            projected: EconomyView::Absent,
            blocking: Vec::new(),
            groups: GroupIndex::default(),
            moved: HashSet::new(),
            generation: 0,
            dirty: false,
            search_cache: None,
            styles: None,
            legacy_base: None,
            styles_error: None,
            styles_rev: 0,
            pending_cache: None,
        };
        wf.rebuild();
        wf
    }

    /// Recompute every cached projection from the document. Called once per mutation.
    fn rebuild(&mut self) {
        let sec: Result<Option<SectionSpan>, _> = section::find(&self.doc);
        let (span, damaged) = match &sec {
            Ok(s) => (*s, None),
            Err(e) => (None, Some(e.to_string())),
        };
        let in_economy = |h: usize| span.is_some_and(|s| h > s.begin && h < s.end);

        // (block, is_economy, side) in file order; side 0 = before the section, 1 = after.
        let mut order: Vec<(&Block, bool, u8)> = Vec::new();
        for b in self.doc.blocks() {
            let econ = in_economy(b.header_line());
            let side = u8::from(span.is_some_and(|s| b.header_line() >= s.end));
            order.push((b, econ, side));
        }
        let econ_blocks = order.iter().filter(|(_, e, _)| *e).count();
        let groups = group::analyze(&self.doc);

        let mut rows = Vec::new();
        let mut index = 0usize;
        for (pos, (b, econ, side)) in order.iter().enumerate() {
            if *econ || groups.is_follower(b.id()) {
                continue;
            }
            index += 1;
            let id = b.id();
            let members = groups.members(id);
            let last_pos = pos + members.len() - 1;
            let has_defences = !group::defence_lines(b).is_empty();
            let simple_mask = if members.len() > 1 || has_defences {
                group::selection(&self.doc, &members)
            } else {
                None
            };
            let x = if members.len() > 1 {
                extract_group(&self.doc, b, simple_mask)
            } else {
                extract(&self.doc, b)
            };
            let name = self.doc.meta_get(id, "name").map(str::to_string);
            let style = match self.doc.meta_get(id, "style") {
                None => StyleState::None,
                Some(n) => match &self.styles {
                    Some(set) => match set.resolve_tag(n) {
                        Some(m) => StyleState::Known(m.name.to_string()),
                        None => StyleState::Unknown(n.to_string()),
                    },
                    None => StyleState::Unchecked(n.to_string()),
                },
            };
            let kw = keyword_word(b.keyword());
            let label = match &name {
                Some(n) => format!("{index}  {kw} - {n}"),
                None => format!("{index}  {kw}"),
            };
            let mut hay = String::new();
            if let Some(n) = &name {
                hay.push_str(n);
                hay.push('\n');
            }
            hay.push_str(&label);
            for v in x
                .class
                .iter()
                .chain(x.base_type.iter())
                .chain(x.other_names.iter())
            {
                hay.push('\n');
                hay.push_str(v);
            }
            let enabled = b.is_enabled();
            let (mut errors, mut warnings) = (Vec::new(), Vec::new());
            if enabled {
                let owned = b.owned();
                for d in self.doc.diagnostics() {
                    if d.line > owned.start && d.line <= owned.end {
                        match d.severity {
                            Severity::Error => errors.push(d.clone()),
                            Severity::Warning => warnings.push(d.clone()),
                            Severity::Info => {}
                        }
                    }
                }
            }
            // One pass: a neighbour is swappable iff it is a rule on the same side of the
            // economy section. Mirrors `Document::move_neighbour` without rescanning per row.
            let same_side_rule = |p: Option<usize>| {
                p.and_then(|p| order.get(p))
                    .is_some_and(|(_, e, s)| !*e && *s == *side)
            };
            let movable = damaged.is_none();
            rows.push(RuleRow {
                id,
                index,
                enabled,
                keyword: b.keyword(),
                has_continue: b.has_continue(),
                name,
                style,
                label,
                haystack: hay.to_lowercase(),
                errors,
                warnings,
                cannot_enable: !enabled && !b.can_enable(),
                can_move_up: movable && same_side_rule(pos.checked_sub(1)),
                can_move_down: movable && same_side_rule(Some(last_pos + 1)),
                moved_across_continue: self.moved.contains(&id),
                summary: row_summary(&x.conditions),
                conditions: x.conditions,
                group_size: members.len(),
                group_broken: groups.is_broken(id),
                simple_mask,
            });
        }

        self.base_economy = match (&sec, damaged) {
            (_, Some(msg)) => EconomyView::Damaged(msg),
            (Ok(Some(_)), None) => EconomyView::Present {
                blocks: econ_blocks,
            },
            _ => EconomyView::Absent,
        };
        self.rows = rows;
        self.groups = groups;
        self.blocking = blocking_errors(&self.doc);
        self.generation += 1;
        self.search_cache = None;
        self.refresh_projection();
    }

    /// Projected economy view and dirty flag from the base state and the staged op.
    fn refresh_projection(&mut self) {
        self.projected = match self.pending {
            Some(Pending::Add { blocks }) => EconomyView::Present { blocks },
            Some(Pending::Remove) => EconomyView::Absent,
            None => self.base_economy.clone(),
        };
        self.dirty = self.pending.is_some() || self.doc.serialize() != self.applied_text;
    }

    /// Store the freshly loaded styles (or the load error) and re-judge every tag.
    pub fn set_styles(&mut self, styles: Result<StyleSet, String>) {
        match styles {
            Ok(s) => {
                self.retag_renamed_legacy(&s);
                self.styles = Some(s);
                self.styles_error = None;
            }
            Err(e) => {
                self.styles = None;
                self.styles_error = Some(e);
            }
        }
        self.styles_rev += 1;
        self.rebuild();
    }

    /// A legacy name-only tag (`# ff:style kite`) links by name, so renaming that style would
    /// orphan it (and Apply would then drop the tag). Before `new` replaces the current set, move
    /// every such tag whose style keeps its id in `new` but no longer resolves there to the
    /// canonical `<id> <new name>` form. Only those tag lines change.
    ///
    /// The applied baseline gets the same migration: it is not a user edit, so it never makes the
    /// rules dirty, and a RULES Revert (which re-parses the baseline) keeps it (CR-01, WR-04).
    fn retag_renamed_legacy(&mut self, new: &StyleSet) {
        // The applied set first: legacy tags only ever come from disk.
        let olds: Vec<&StyleSet> = self.legacy_base.iter().chain(self.styles.iter()).collect();
        if olds.is_empty() {
            return;
        }
        migrate_legacy_tags(&mut self.doc, &olds, new);
        let mut base = parse_text(&self.applied_text);
        if migrate_legacy_tags(&mut base, &olds, new) {
            self.applied_text = base.serialize();
        }
    }

    /// [`Self::set_styles`] with the CONFIG tab's pending set, remembering `base` (the applied
    /// set) as the set legacy name-only tags were written against.
    pub fn set_styles_over(&mut self, styles: StyleSet, base: &StyleSet) {
        if self.legacy_base.as_ref() != Some(base) {
            self.legacy_base = Some(base.clone());
        }
        self.set_styles(Ok(styles));
    }

    /// Style names in styles.json order; empty when unavailable.
    pub fn style_names(&self) -> Vec<String> {
        self.styles
            .as_ref()
            .map(|s| s.names().map(str::to_string).collect())
            .unwrap_or_default()
    }

    /// The style set tags are currently judged against, if any loaded.
    pub fn style_set(&self) -> Option<&StyleSet> {
        self.styles.as_ref()
    }

    pub fn style(&self, name: &str) -> Option<&Style> {
        self.styles.as_ref().and_then(|s| s.get(name))
    }

    pub fn styles_error(&self) -> Option<&str> {
        self.styles_error.as_deref()
    }

    /// The text Apply should write: the document with every enabled styled rule rewritten from
    /// the current styles, produced on a clone so the user's edits are never touched.
    pub fn text_with_styles(&self) -> Result<(String, StyleReport), String> {
        self.text_with_styles_retag(true)
    }

    /// [`Self::text_with_styles`] that never migrates tags. For an Apply that does not write
    /// styles.json: ids assigned at load exist only in memory then, and a tag carrying one would
    /// stop resolving as soon as styles.json changes by hand (WR-06). Legacy name tags still
    /// resolve by name, so keeping them loses nothing.
    pub fn text_with_styles_keeping_tags(&self) -> Result<(String, StyleReport), String> {
        self.text_with_styles_retag(false)
    }

    fn text_with_styles_retag(&self, retag: bool) -> Result<(String, StyleReport), String> {
        let Some(set) = &self.styles else {
            if self
                .rows
                .iter()
                .any(|r| r.enabled && r.style != StyleState::None)
            {
                let err = self
                    .styles_error
                    .as_deref()
                    .unwrap_or("styles were not loaded");
                return Err(styles_unavailable_message(err));
            }
            return Ok((self.text(), StyleReport::default()));
        };
        Ok(self.text_with_style_set_retag(set, retag))
    }

    /// The text Apply should write when the styles are `set` (the pending CONFIG styles) rather
    /// than the loaded ones: same rewrite, on a clone.
    pub fn text_with_style_set(&self, set: &StyleSet) -> (String, StyleReport) {
        self.text_with_style_set_retag(set, true)
    }

    fn text_with_style_set_retag(&self, set: &StyleSet, retag: bool) -> (String, StyleReport) {
        let mut clone = self.doc.clone();
        let mut report = styled::rewrite_styled_with(&mut clone, set, retag);
        report.dropped = styled::drop_unknown_style_tags(&mut clone, set);
        (clone.serialize(), report)
    }

    /// Whether Apply would change the text because of styles alone. Cached per
    /// (generation, styles revision).
    pub fn style_rewrite_pending(&mut self) -> bool {
        if let Some((g, r, v)) = self.pending_cache
            && g == self.generation
            && r == self.styles_rev
        {
            return v;
        }
        // retag = false: a pure tag migration must not count as an unapplied change.
        let v = match &self.styles {
            Some(set) => self.text_with_style_set_retag(set, false).0 != self.text(),
            None => false,
        };
        self.pending_cache = Some((self.generation, self.styles_rev, v));
        v
    }

    /// Enabled rules naming an unknown style, in file order, as (rule index, block id).
    pub fn unknown_styles(&self) -> Vec<(usize, BlockId)> {
        self.rows
            .iter()
            .filter(|r| r.enabled && matches!(r.style, StyleState::Unknown(_)))
            .map(|r| (r.index, r.id))
            .collect()
    }

    /// Refusal text when the rule's actions belong to a known style.
    fn style_lock(&self, id: BlockId) -> Option<String> {
        match self.row(id).map(|r| &r.style) {
            Some(StyleState::Known(n)) => Some(style_locked_status(n)),
            _ => None,
        }
    }

    pub fn rows(&self) -> &[RuleRow] {
        &self.rows
    }

    /// How many rules carry each `# ff:style` name (known, unknown or unchecked alike).
    pub fn style_users(&self) -> HashMap<String, usize> {
        let mut users = HashMap::new();
        for row in &self.rows {
            if let StyleState::Known(n) | StyleState::Unknown(n) | StyleState::Unchecked(n) =
                &row.style
            {
                *users.entry(n.clone()).or_insert(0) += 1;
            }
        }
        users
    }

    pub fn row(&self, id: BlockId) -> Option<&RuleRow> {
        self.rows.iter().find(|r| r.id == id)
    }

    pub fn economy(&self) -> &EconomyView {
        &self.projected
    }

    pub fn economy_plan(&self) -> EconomyPlan {
        match self.pending {
            Some(Pending::Add { .. }) => EconomyPlan::Add,
            Some(Pending::Remove) => EconomyPlan::Remove,
            None => match self.base_economy {
                EconomyView::Present { .. } => EconomyPlan::Refresh,
                _ => EconomyPlan::Keep,
            },
        }
    }

    /// Stage adding the economy section (only from Absent); the opposite staged op cancels.
    pub fn stage_add_economy(&mut self, blocks: usize) {
        match (self.pending, &self.base_economy) {
            (Some(Pending::Remove), _) => self.pending = None,
            (None, EconomyView::Absent) => self.pending = Some(Pending::Add { blocks }),
            _ => return,
        }
        self.refresh_projection();
    }

    /// Stage removing the economy section (only from Present); the opposite staged op cancels.
    pub fn stage_remove_economy(&mut self) {
        match (self.pending, &self.base_economy) {
            (Some(Pending::Add { .. }), _) => self.pending = None,
            (None, EconomyView::Present { .. }) => self.pending = Some(Pending::Remove),
            _ => return,
        }
        self.refresh_projection();
    }

    /// Whether the block's body lines accept line edits: false for an unknown id and for an
    /// invalid disabled block (its lines are commented prose the parser cannot structure).
    pub fn body_editable(&self, id: BlockId) -> bool {
        self.doc
            .block(id)
            .is_some_and(|b| !matches!(b.disabled_state(), Some(DisabledState::Invalid(_))))
    }

    /// Every body line of a block, in file order, typed for the editor body. Computed on demand
    /// for the single expanded row; empty for an unknown id.
    pub fn body_view(&self, id: BlockId) -> Vec<BodyEntry> {
        let Some(b) = self.doc.block(id) else {
            return Vec::new();
        };
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut out = Vec::new();
        for bl in b.body() {
            let Some(text) = self.doc.lines().get(bl.line).map(|l| l.text()) else {
                continue;
            };
            let kind = match (bl.keyword, bl.tokens.as_ref()) {
                (Some(k), Some(_)) => match k.kind {
                    Kind::Condition => EntryKind::Condition,
                    Kind::Action => EntryKind::Action,
                    Kind::Modifier => EntryKind::Modifier,
                    _ => EntryKind::Opaque,
                },
                _ => EntryKind::Opaque,
            };
            let keyword = if kind == EntryKind::Opaque {
                None
            } else {
                bl.keyword
            };
            // Disabled-block token ranges are relative to the text past the `#` (and one space).
            let base = if b.is_enabled() {
                text
            } else {
                crate::filter::commented_inner(text)
            };
            let (op, values) = match (&keyword, bl.tokens.as_ref()) {
                (Some(_), Some(t)) => (
                    t.operator.as_ref().map(|(_, o)| *o),
                    t.values
                        .iter()
                        .filter_map(|r| base.get(r.clone()))
                        .map(str::to_string)
                        .collect(),
                ),
                _ => (None, Vec::new()),
            };
            let key = keyword.map_or_else(String::new, |k| k.name.to_ascii_lowercase());
            let n = seen.entry(key).or_insert(0);
            let ordinal = *n;
            *n += 1;
            out.push(BodyEntry {
                line: bl.line,
                kind,
                keyword,
                op,
                values,
                text: text.to_string(),
                ordinal,
            });
        }
        out
    }

    /// Guard shared by the line actions: the id must be an editable row, and (for edits and
    /// removals) the addressed line must still read as the widget saw it.
    fn line_refusal(&self, id: BlockId, target: Option<(usize, &str)>) -> Option<String> {
        if self.row(id).is_none() || !self.body_editable(id) {
            return Some(NOT_EDITABLE_STATUS.to_string());
        }
        match target {
            Some((at, expect)) if !self.line_matches(at, expect) => {
                Some(STALE_LINE_STATUS.to_string())
            }
            _ => None,
        }
    }

    /// Style lock for an edit or removal aimed at an action line of a known-styled rule.
    fn action_line_lock(&self, id: BlockId, line: usize) -> Option<String> {
        let msg = self.style_lock(id)?;
        self.body_view(id)
            .iter()
            .any(|e| e.line == line && e.kind == EntryKind::Action)
            .then_some(msg)
    }

    /// Style lock for an edit whose new text would turn the line into an action line of a
    /// known-styled rule (a raw `Whole` edit of an opaque or unknown-keyword line). Such a line
    /// would be silently dropped by the style rewrite on Apply (WR-03).
    fn becomes_action_lock(&self, id: BlockId, patch: &LinePatch) -> Option<String> {
        let LinePatch::Whole(text) = patch else {
            return None;
        };
        let msg = self.style_lock(id)?;
        (text_kind(text) == Some(Kind::Action)).then_some(msg)
    }

    /// Current text of a doc line when it equals `expect` (bounds-checked).
    fn line_matches(&self, line: usize, expect: &str) -> bool {
        self.doc
            .lines()
            .get(line)
            .is_some_and(|l| l.text() == expect)
    }

    /// Apply one row action. A grouped rule (several blocks, `# ff:group`) is one rule: every
    /// action acts on all of its blocks and keeps them identical apart from their defence lines.
    /// All or nothing: a refusal leaves the document as it was.
    pub fn apply_action(&mut self, action: RowAction) -> ActionOutcome {
        let out = self.dispatch(action);
        if out.changed {
            self.rebuild();
        }
        out
    }

    fn dispatch(&mut self, action: RowAction) -> ActionOutcome {
        // A later block of a group is not a rule of its own: acting on it alone would break the
        // group (review WR-02).
        if action_target(&action).is_some_and(|id| self.groups.is_follower(id)) {
            return refused(NOT_EDITABLE_STATUS.to_string());
        }
        match action {
            RowAction::Move(id, dir) => return self.move_rule(id, dir),
            RowAction::SetDefenceSelection { id, mask } => return self.set_selection(id, mask),
            _ => {}
        }
        let members = action_target(&action)
            .map(|id| self.groups.members(id))
            .unwrap_or_default();
        if members.len() < 2 {
            return self.block_action(action);
        }
        let snapshot = self.doc.clone();
        let out = match action {
            RowAction::SetEnabled(id, on) => {
                if on && self.row(id).is_some_and(|r| r.cannot_enable) {
                    return refused(CANNOT_ENABLE_STATUS.to_string());
                }
                let r = members
                    .iter()
                    .try_for_each(|m| self.doc.set_enabled(*m, on));
                done_or(r, None)
            }
            RowAction::Delete(id) => {
                let r = members.iter().try_for_each(|m| self.doc.remove_block(*m));
                if r.is_ok() {
                    self.moved.remove(&id);
                }
                done_or(r, None)
            }
            RowAction::Duplicate(id) => {
                let name = self.doc.meta_get(id, "name").map(|n| format!("{n} (copy)"));
                match group::duplicate(&mut self.doc, &members, name.as_deref()) {
                    Ok(copy) => done_or(Ok(()), copy.first().copied()),
                    Err(e) => refused(e.to_string()),
                }
            }
            other => {
                let out = self.block_action(other);
                if !out.changed {
                    out
                } else if let Err(e) = group::resync(&mut self.doc, &members) {
                    refused(e.to_string())
                } else if group::analyze(&self.doc).members(members[0]).len() != members.len() {
                    // The edit changed the leader's defence lines in a way the copies cannot
                    // follow (e.g. a raw defence line): refuse rather than break the group
                    // (review WR-01).
                    refused(GROUP_DEFENCES_STATUS.to_string())
                } else {
                    out
                }
            }
        };
        if !out.changed {
            self.doc = snapshot;
        }
        out
    }

    /// Move a rule past its neighbour rule. Either may be a group: a group moves as one unit.
    fn move_rule(&mut self, id: BlockId, dir: Dir) -> ActionOutcome {
        let up = dir == Dir::Up;
        let a = self.groups.members(id);
        let edge = if up { a[0] } else { a[a.len() - 1] };
        let nb = match self.doc.move_neighbour(edge, up) {
            Err(e) => return refused(e.to_string()),
            Ok(None) => return refused("There is no rule to swap with in that direction.".into()),
            Ok(Some(nb)) => self.groups.rule_of(nb),
        };
        let b = self.groups.members(nb);
        let crossing = self.doc.block(id).is_some_and(Block::has_continue)
            || self.doc.block(nb).is_some_and(Block::has_continue);
        let r = match (a.len(), b.len(), up) {
            (1, 1, _) => self.doc.move_block(id, up),
            (_, _, true) => self.doc.swap_runs(&b, &a),
            (_, _, false) => self.doc.swap_runs(&a, &b),
        };
        if r.is_ok() {
            if crossing {
                self.moved.insert(id);
            } else {
                self.moved.remove(&id);
            }
        }
        done_or(r, Some(id))
    }

    /// Set the Simple Defences selection of a rule (a bit set over [`group::COMBOS`]): the rule is
    /// rewritten as the minimum number of blocks matching exactly that selection.
    fn set_selection(&mut self, id: BlockId, mask: u8) -> ActionOutcome {
        if self.row(id).is_none() || !self.body_editable(id) {
            return refused(NOT_EDITABLE_STATUS.to_string());
        }
        let members = self.groups.members(id);
        let snapshot = self.doc.clone();
        match group::set_selection(&mut self.doc, &members, mask) {
            Ok(_) => done_or(Ok(()), None),
            Err(e) => {
                self.doc = snapshot;
                refused(e.to_string())
            }
        }
    }

    /// One action on one block, without the row rebuild (the caller rebuilds once).
    fn block_action(&mut self, action: RowAction) -> ActionOutcome {
        let mut focus = None;
        let mut line = None;
        let mut status = None;
        let result: Result<(), String> = match action {
            RowAction::SetEnabled(id, on) => {
                if on && self.row(id).is_some_and(|r| r.cannot_enable) {
                    return ActionOutcome {
                        changed: false,
                        status: Some(CANNOT_ENABLE_STATUS.to_string()),
                        focus: None,
                        line: None,
                    };
                }
                self.doc.set_enabled(id, on).map_err(|e| e.to_string())
            }
            RowAction::AddRule => match self.doc.insert_rule_at_top(vec!["Show".to_string()]) {
                Ok(id) => {
                    focus = Some(id);
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            },
            RowAction::Delete(id) => {
                let r = self.doc.remove_block(id).map_err(|e| e.to_string());
                if r.is_ok() {
                    self.moved.remove(&id);
                }
                r
            }
            RowAction::Duplicate(id) => {
                let name = self.doc.meta_get(id, "name").map(str::to_string);
                match self.doc.duplicate_block(id) {
                    Ok(copy) => {
                        focus = Some(copy);
                        if let Some(n) = name
                            && let Err(e) = self.doc.set_meta(copy, "name", &format!("{n} (copy)"))
                        {
                            status = Some(e.to_string());
                        }
                        // A copy of a block whose group is broken must not join that group.
                        if let Err(e) = self.doc.remove_meta(copy, group::GROUP_KEY) {
                            status = Some(e.to_string());
                        }
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            // Handled before a single block is chosen (see `dispatch`).
            RowAction::Move(..) | RowAction::SetDefenceSelection { .. } => {
                Err(NOT_EDITABLE_STATUS.to_string())
            }
            RowAction::SetKeyword(id, kw) => {
                if let Some(msg) = self.style_lock(id) {
                    return refused(msg);
                }
                self.doc
                    .set_block_keyword(id, kw)
                    .map_err(|e| e.to_string())
            }
            RowAction::ApplyStyle(id, name) => {
                if self.row(id).is_none() {
                    return refused(NOT_EDITABLE_STATUS.to_string());
                }
                let Some(set) = &self.styles else {
                    let err = self
                        .styles_error
                        .as_deref()
                        .unwrap_or("styles were not loaded");
                    return refused(styles_unavailable_message(err));
                };
                // An id-less style (only possible in a hand-built set) falls back to its name.
                let (style, tag) = match set.get(&name) {
                    Some(st) if styled::style_name_ok(&name) => {
                        (st, set.tag_for(&name).unwrap_or_else(|| name.clone()))
                    }
                    _ => return refused(format!("There is no style named \"{name}\".")),
                };
                let mut clone = self.doc.clone();
                let enabled = clone.block(id).is_some_and(Block::is_enabled);
                let r = clone
                    .set_meta(id, "style", &tag)
                    .and_then(|()| {
                        if enabled {
                            styled::rewrite_block(&mut clone, id, style).map(|_| ())
                        } else {
                            Ok(())
                        }
                    })
                    .map_err(|e| e.to_string());
                if r.is_ok() {
                    self.doc = clone;
                }
                r
            }
            RowAction::RemoveStyle(id) => {
                let r = self.doc.remove_meta(id, "style").map_err(|e| e.to_string());
                if r.is_ok() {
                    status = Some(STYLE_REMOVED_STATUS.to_string());
                }
                r
            }
            RowAction::SetContinue(id, on) => {
                // The "moved across Continue" warning may no longer be true once a Continue
                // line is added or removed, so drop all of them rather than leave a stale one.
                self.moved.clear();
                self.doc.set_continue(id, on).map_err(|e| e.to_string())
            }
            RowAction::SetName(id, s) => {
                let t = s.trim();
                if t.is_empty() {
                    self.doc.remove_meta(id, "name").map_err(|e| e.to_string())
                } else {
                    self.doc.set_meta(id, "name", t).map_err(|e| e.to_string())
                }
            }
            RowAction::ClearName(id) => self.doc.remove_meta(id, "name").map_err(|e| e.to_string()),
            RowAction::EditLine {
                id,
                line: at,
                expect,
                patch,
            } => {
                if let Some(msg) = self.line_refusal(id, Some((at, &expect))) {
                    return refused(msg);
                }
                if let Some(msg) = self.action_line_lock(id, at) {
                    return refused(msg);
                }
                if let Some(msg) = self.becomes_action_lock(id, &patch) {
                    return refused(msg);
                }
                self.doc
                    .patch_line(id, at, patch)
                    .map_err(|e| e.to_string())
            }
            RowAction::RemoveLine {
                id,
                line: at,
                expect,
            } => {
                if let Some(msg) = self.line_refusal(id, Some((at, &expect))) {
                    return refused(msg);
                }
                if let Some(msg) = self.action_line_lock(id, at) {
                    return refused(msg);
                }
                self.doc.remove_body_line(id, at).map_err(|e| e.to_string())
            }
            RowAction::InsertLineAfter {
                id,
                after,
                expect,
                text,
            } => {
                if let Some(msg) = self.line_refusal(id, Some((after, &expect))) {
                    return refused(msg);
                }
                if text_kind(&text) == Some(Kind::Action) {
                    return refused(
                        self.style_lock(id)
                            .unwrap_or_else(|| ACTION_IN_CONDITIONS_STATUS.to_string()),
                    );
                }
                match self.doc.insert_body_line(id, after, &text) {
                    Ok(new) => {
                        focus = Some(id);
                        line = Some(new);
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            RowAction::AddLine { id, kind, text } => {
                if let Some(msg) = self.line_refusal(id, None) {
                    return refused(msg);
                }
                // The declared section must agree with what the text actually is, so a
                // condition-section add cannot smuggle an action past the style lock (WR-03).
                match (kind, text_kind(&text)) {
                    (LineKind::Condition, Some(Kind::Action)) => {
                        return refused(
                            self.style_lock(id)
                                .unwrap_or_else(|| ACTION_IN_CONDITIONS_STATUS.to_string()),
                        );
                    }
                    (LineKind::Action, Some(Kind::Condition)) => {
                        return refused(CONDITION_IN_ACTIONS_STATUS.to_string());
                    }
                    _ => {}
                }
                if kind == LineKind::Action
                    && let Some(msg) = self.style_lock(id)
                {
                    return refused(msg);
                }
                let want = match kind {
                    LineKind::Condition => EntryKind::Condition,
                    LineKind::Action => EntryKind::Action,
                };
                let view = self.body_view(id);
                let after = view
                    .iter()
                    .rfind(|e| e.kind == want)
                    .map(|e| e.line)
                    .or_else(|| match kind {
                        LineKind::Condition => None,
                        LineKind::Action => view.last().map(|e| e.line),
                    })
                    .or_else(|| self.doc.block(id).map(Block::header_line));
                match after {
                    None => Err(NOT_EDITABLE_STATUS.to_string()),
                    Some(after) => match self.doc.insert_body_line(id, after, &text) {
                        Ok(new) => {
                            focus = Some(id);
                            line = Some(new);
                            Ok(())
                        }
                        Err(e) => Err(e.to_string()),
                    },
                }
            }
        };
        match result {
            Ok(()) => ActionOutcome {
                changed: true,
                status,
                focus,
                line,
            },
            Err(msg) => ActionOutcome {
                changed: false,
                status: Some(msg),
                focus: None,
                line: None,
            },
        }
    }

    /// Indices into [`rows`](Self::rows) matching the query (case-insensitive over name, label,
    /// Class and BaseType values). Empty query matches everything. Cached per generation.
    pub fn matches(&mut self, query: &str) -> Vec<usize> {
        let q = query.trim().to_lowercase();
        if let Some((cq, g, v)) = &self.search_cache
            && *cq == q
            && *g == self.generation
        {
            return v.clone();
        }
        let out: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| q.is_empty() || r.haystack.contains(&q))
            .map(|(i, _)| i)
            .collect();
        self.search_cache = Some((q, self.generation, out.clone()));
        out
    }

    pub fn row_matches(&self, id: BlockId, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        self.row(id)
            .is_some_and(|r| q.is_empty() || r.haystack.contains(&q))
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Enabled-block grammar errors that gate Apply.
    pub fn blocking(&self) -> &[BlockingError] {
        &self.blocking
    }

    pub fn text(&self) -> String {
        self.doc.serialize()
    }

    /// Discard every in-memory edit and staged economy op.
    pub fn revert(&mut self) {
        self.doc = parse_text(&self.applied_text);
        self.pending = None;
        self.moved.clear();
        self.rebuild();
    }

    /// `applied_text` is now what is on disk: make it the baseline.
    pub fn on_applied(&mut self, applied_text: &str) {
        self.applied_text = applied_text.to_string();
        self.doc = parse_text(applied_text);
        self.pending = None;
        self.moved.clear();
        self.rebuild();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::section::{BEGIN_LINE, RULE_LINE};

    const THREE: &str =
        "Show\n  Class \"Rings\"\n\nHide\n  Rarity Normal\n\nShow\n  ItemLevel >= 82\n";

    fn with_economy() -> String {
        format!(
            "Show\n  Class \"Rings\"\n\n{RULE_LINE}\n{BEGIN_LINE}\n{RULE_LINE}\nShow\n  Class \"Gems\"\n\nShow\n  Class \"Currency\"\n\n{RULE_LINE}\n# ff:section After\n{RULE_LINE}\n\nHide\n  Rarity Normal\n\nShow\n  ItemLevel >= 82\n"
        )
    }

    #[test]
    fn style_users_counts_every_tagged_rule() {
        let wf = WorkingFilter::from_text(
            "#ff:style kite
Show

#ff:style kite
Hide

#ff:style S
Show
",
        );
        let users = wf.style_users();
        assert_eq!(users.get("kite"), Some(&2));
        assert_eq!(users.get("S"), Some(&1));
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn economy_blocks_are_not_rows() {
        let wf = WorkingFilter::from_text(&with_economy());
        assert_eq!(wf.rows().len(), 3);
        assert_eq!(*wf.economy(), EconomyView::Present { blocks: 2 });
        assert_eq!(wf.economy_plan(), EconomyPlan::Refresh);
        let plain = WorkingFilter::from_text(THREE);
        assert_eq!(plain.rows().len(), 3);
        assert_eq!(*plain.economy(), EconomyView::Absent);
        assert_eq!(plain.economy_plan(), EconomyPlan::Keep);
    }

    #[test]
    fn lone_begin_marker_is_damaged_and_every_block_is_a_row() {
        let t = format!("Show\n  Class \"Rings\"\n\n{BEGIN_LINE}\nHide\n  Rarity Normal\n");
        let wf = WorkingFilter::from_text(&t);
        assert!(matches!(wf.economy(), EconomyView::Damaged(m) if !m.is_empty()));
        assert_eq!(wf.rows().len(), 2);
        assert!(wf.rows().iter().all(|r| !r.can_move_up && !r.can_move_down));
    }

    #[test]
    fn name_comes_only_from_ff_name() {
        let t = "#ff:name Rare rings\nShow # %D4 $type->currency $tier->s !slug\n  Rarity Rare\n\nShow # %D4 $type->currency $tier->s !slug\n  Rarity Magic\n";
        let wf = WorkingFilter::from_text(t);
        assert_eq!(wf.rows()[0].label, "1  Show - Rare rings");
        assert_eq!(wf.rows()[0].name.as_deref(), Some("Rare rings"));
        let l = &wf.rows()[1].label;
        assert!(wf.rows()[1].name.is_none());
        for jargon in ["%D4", "$type", "!slug", "$tier"] {
            assert!(!l.contains(jargon), "{l}");
            assert!(!wf.rows()[1].haystack.contains(&jargon.to_lowercase()));
        }
    }

    #[test]
    fn unnamed_label_is_index_and_keyword() {
        let t = "Show\n  Class \"Rings\"\n\nHide\n";
        let wf = WorkingFilter::from_text(t);
        assert_eq!(wf.rows()[0].label, "1  Show");
        assert_eq!(wf.rows()[1].label, "2  Hide");
        let wf = WorkingFilter::from_text("#ff:name My rule\nShow\n  Class \"Rings\"\n");
        assert_eq!(wf.rows()[0].label, "1  Show - My rule");
        let wf =
            WorkingFilter::from_text("Show\n  Class \"Rings\"\n  BaseType == \"Heavy Belt\"\n");
        let id = wf.rows()[0].id;
        assert_eq!(wf.rows()[0].label, "1  Show");
        assert!(wf.row_matches(id, "rings"));
        assert!(wf.row_matches(id, "heavy belt"));
    }

    #[test]
    fn disabled_blocks_valid_and_invalid() {
        let t = "# Show\n#   Class \"Rings\"\n\n# Show\n#   Rarity Bogus\n";
        let wf = WorkingFilter::from_text(t);
        assert_eq!(wf.rows().len(), 2);
        assert!(!wf.rows()[0].enabled && !wf.rows()[0].cannot_enable);
        assert!(!wf.rows()[1].enabled && wf.rows()[1].cannot_enable);
        assert!(wf.rows()[0].haystack.contains("rings"));
    }

    #[test]
    fn errors_and_warnings_split_by_severity() {
        let t = "Show\n  Rarity Bogus\n\nShow\n  TotallyUnknown 5\n";
        let wf = WorkingFilter::from_text(t);
        let all_sev: Vec<_> = wf
            .rows()
            .iter()
            .map(|r| (r.errors.len(), r.warnings.len()))
            .collect();
        assert!(all_sev[0].0 > 0, "{all_sev:?}");
        assert!(all_sev[1].0 == 0 && all_sev[1].1 > 0, "{all_sev:?}");
        assert!(!wf.blocking().is_empty());
    }

    #[test]
    fn style_state_is_exposed() {
        let mut wf = WorkingFilter::from_text(
            "#ff:style S\nShow\n  Rarity Rare\n\n#ff:style Nope\nShow\n\nShow\n",
        );
        let st = |wf: &WorkingFilter| {
            wf.rows()
                .iter()
                .map(|r| r.style.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(st(&wf)[0], StyleState::Unchecked("S".into()));
        wf.set_styles(Ok(test_styles()));
        assert_eq!(
            st(&wf),
            vec![
                StyleState::Known("S".into()),
                StyleState::Unknown("Nope".into()),
                StyleState::None
            ]
        );
        wf.set_styles(Err("bad".into()));
        assert_eq!(st(&wf)[0], StyleState::Unchecked("S".into()));
        assert_eq!(wf.styles_error(), Some("bad"));
        assert!(wf.style_names().is_empty());
    }

    #[test]
    fn row_summary_joins_conditions() {
        let wf = WorkingFilter::from_text(
            "Show\n    Class \"Rings\" \"Amulets\"\n    ItemLevel >= 70\n    Rarity Rare\n",
        );
        assert_eq!(
            wf.rows()[0].summary,
            "Class: Rings, Amulets; ItemLevel >= 70; Rarity Rare"
        );
        let none = WorkingFilter::from_text("Show\n    SetFontSize 30\n");
        assert_eq!(none.rows()[0].summary, "");
        let long = row_summary(&["\u{e9}".repeat(SUMMARY_MAX_CHARS + 20)]);
        assert_eq!(long.chars().count(), SUMMARY_MAX_CHARS + 1);
        assert!(long.ends_with('\u{2026}'));
    }

    #[test]
    fn condition_strings() {
        let many: Vec<String> = (0..14).map(|i| format!("\"B{i}\"")).collect();
        let t = format!(
            "Show\n  Class == \"Rings\" \"Amulets\"\n  ItemLevel >= 82\n  BaseType {}\n",
            many.join(" ")
        );
        let wf = WorkingFilter::from_text(&t);
        assert_eq!(
            wf.rows()[0].conditions,
            vec![
                "Class: Rings, Amulets",
                "ItemLevel >= 82",
                "BaseType: 14 items"
            ]
        );
    }

    #[test]
    fn search_covers_name_label_class_and_basetype() {
        let t = "#ff:name Fancy\nShow # header only\n  Class \"Rings\"\n\nShow\n  BaseType \"Sapphire Ring\"\n\nShow\n  Rarity Rare\n";
        let mut wf = WorkingFilter::from_text(t);
        assert_eq!(wf.matches("RING"), vec![0, 1]);
        assert_eq!(wf.matches("fancy"), vec![0]);
        assert!(wf.matches("header").is_empty());
        assert_eq!(wf.matches("").len(), 3);
        assert_eq!(wf.matches("  ").len(), 3);
        let id = wf.rows()[1].id;
        assert!(wf.row_matches(id, "sapphire"));
        assert!(!wf.row_matches(id, "emerald"));
    }

    #[test]
    fn move_flags_respect_the_economy_section() {
        let wf = WorkingFilter::from_text(&with_economy());
        let r = wf.rows();
        assert!(!r[0].can_move_up && !r[0].can_move_down, "economy follows");
        assert!(
            !r[1].can_move_up,
            "OD-3: rule after economy-end has no rule above"
        );
        assert!(r[1].can_move_down);
        assert!(r[2].can_move_up && !r[2].can_move_down);
    }

    #[test]
    fn move_flags_agree_with_move_neighbour() {
        for text in [THREE.to_string(), with_economy()] {
            let wf = WorkingFilter::from_text(&text);
            for r in wf.rows() {
                let up = wf.doc.move_neighbour(r.id, true).ok().flatten().is_some();
                let down = wf.doc.move_neighbour(r.id, false).ok().flatten().is_some();
                assert_eq!((r.can_move_up, r.can_move_down), (up, down), "{}", r.label);
            }
        }
    }

    fn test_styles() -> StyleSet {
        crate::loader::load_styles(concat!(env!("CARGO_MANIFEST_DIR"), "/styles-default.json"))
            .expect("styles.json")
    }

    fn styled_wf(text: &str) -> WorkingFilter {
        let mut wf = WorkingFilter::from_text(text);
        wf.set_styles(Ok(test_styles()));
        wf
    }

    const S_FONT: &str = "  SetFontSize 37\n";

    #[test]
    fn apply_style_sets_tag_keyword_and_actions() {
        let mut wf = styled_wf("Hide\n  Class \"Rings\"\n  SetFontSize 20\n");
        let id = wf.rows()[0].id;
        let o = wf.apply_action(RowAction::ApplyStyle(id, "S".into()));
        assert!(o.changed, "{o:?}");
        let t = wf.text();
        assert!(t.starts_with("# ff:style builtin:S S\nShow\n"), "{t}");
        assert!(t.contains(S_FONT) && !t.contains("SetFontSize 20"), "{t}");
        assert_eq!(wf.rows()[0].style, StyleState::Known("S".into()));
        // switching style rewrites the single tag
        let o = wf.apply_action(RowAction::ApplyStyle(id, "hidden".into()));
        assert!(o.changed);
        let t = wf.text();
        assert_eq!(t.matches("ff:style").count(), 1, "{t}");
        assert!(
            t.starts_with("# ff:style builtin:hidden hidden\nHide\n"),
            "{t}"
        );
    }

    #[test]
    fn apply_style_refusals_change_nothing() {
        let mut wf = styled_wf("Hide\n  SetFontSize 20\n");
        let id = wf.rows()[0].id;
        let before = wf.text();
        for bad in ["Nope", " S", ""] {
            let o = wf.apply_action(RowAction::ApplyStyle(id, bad.into()));
            assert!(!o.changed && o.status.is_some(), "{bad:?}");
            assert_eq!(wf.text(), before);
        }
        let mut none = WorkingFilter::from_text("Hide\n");
        let id = none.rows()[0].id;
        let o = none.apply_action(RowAction::ApplyStyle(id, "S".into()));
        assert!(!o.changed && o.status.is_some());
    }

    #[test]
    fn apply_style_on_disabled_rule_writes_tag_only() {
        let mut wf = styled_wf("#Hide\n#  Class \"Rings\"\n#  SetFontSize 20\n");
        let id = wf.rows()[0].id;
        let o = wf.apply_action(RowAction::ApplyStyle(id, "S".into()));
        assert!(o.changed, "{o:?}");
        let t = wf.text();
        assert!(
            t.contains("# ff:style builtin:S S") && t.contains("#  SetFontSize 20"),
            "{t}"
        );
        assert!(!t.contains("SetFontSize 37"), "{t}");
    }

    #[test]
    fn remove_style_keeps_actions_and_unlocks() {
        let mut wf = styled_wf("#ff:style S\nShow\n  Class \"Rings\"\n  SetFontSize 37\n");
        let id = wf.rows()[0].id;
        let o = wf.apply_action(RowAction::RemoveStyle(id));
        assert!(o.changed);
        assert_eq!(o.status.as_deref(), Some(STYLE_REMOVED_STATUS));
        assert_eq!(wf.text(), "Show\n  Class \"Rings\"\n  SetFontSize 37\n");
        assert_eq!(wf.rows()[0].style, StyleState::None);
    }

    #[test]
    fn known_style_locks_actions_but_not_conditions() {
        let t = "#ff:style S\nShow\n  Class \"Rings\"\n  SetFontSize 37\n";
        let mut wf = styled_wf(t);
        let id = wf.rows()[0].id;
        let want = "Set by style \"S\". Remove the style to edit it.";
        let a = wf.apply_action(RowAction::AddLine {
            id,
            kind: LineKind::Action,
            text: "PlayEffect Red".into(),
        });
        assert_eq!(a.status.as_deref(), Some(want));
        let act = edit(&wf, id, 3, LinePatch::Values("40".into()));
        assert_eq!(wf.apply_action(act).status.as_deref(), Some(want));
        let o = wf.apply_action(RowAction::RemoveLine {
            id,
            line: 3,
            expect: "  SetFontSize 37".into(),
        });
        assert_eq!(o.status.as_deref(), Some(want));
        let o = wf.apply_action(RowAction::SetKeyword(id, BlockKeyword::Hide));
        assert_eq!(o.status.as_deref(), Some(want));
        assert_eq!(wf.text(), t);
        let cond = edit(&wf, id, 2, LinePatch::Values("\"Belts\"".into()));
        assert!(wf.apply_action(cond).changed);
    }

    /// WR-03: the style lock cannot be bypassed by declaring an action a condition, or by a raw
    /// whole-line edit that turns an opaque line into an action.
    #[test]
    fn known_style_lock_cannot_be_bypassed() {
        let t = "#ff:style S\nShow\n  Class \"Rings\"\n  SetFontSiz 40\n  SetFontSize 37\n";
        let mut wf = styled_wf(t);
        let id = wf.rows()[0].id;
        let want = "Set by style \"S\". Remove the style to edit it.";
        let o = wf.apply_action(RowAction::AddLine {
            id,
            kind: LineKind::Condition,
            text: "SetFontSize 40".into(),
        });
        assert_eq!(o.status.as_deref(), Some(want));
        let typo = edit(&wf, id, 3, LinePatch::Whole("SetFontSize 40".into()));
        assert_eq!(wf.apply_action(typo).status.as_deref(), Some(want));
        assert_eq!(wf.text(), t);
        // Still fine: a raw edit that stays a non-action line.
        let ok = edit(&wf, id, 3, LinePatch::Whole("Quality >= 1".into()));
        assert!(wf.apply_action(ok).changed);

        let mut plain = WorkingFilter::from_text("Show\n  Class \"Rings\"\n");
        let id = plain.rows()[0].id;
        let o = plain.apply_action(RowAction::AddLine {
            id,
            kind: LineKind::Condition,
            text: "SetFontSize 40".into(),
        });
        assert_eq!(o.status.as_deref(), Some(ACTION_IN_CONDITIONS_STATUS));
        let o = plain.apply_action(RowAction::AddLine {
            id,
            kind: LineKind::Action,
            text: "Quality >= 1".into(),
        });
        assert_eq!(o.status.as_deref(), Some(CONDITION_IN_ACTIONS_STATUS));
        assert_eq!(plain.text(), "Show\n  Class \"Rings\"\n");
    }

    #[test]
    fn unknown_style_leaves_actions_editable() {
        let mut wf = styled_wf("#ff:style Nope\nShow\n  SetFontSize 20\n");
        let id = wf.rows()[0].id;
        let act = edit(&wf, id, 2, LinePatch::Values("21".into()));
        assert!(wf.apply_action(act).changed);
        assert!(
            wf.apply_action(RowAction::SetKeyword(id, BlockKeyword::Hide))
                .changed
        );
        assert!(
            wf.apply_action(RowAction::AddLine {
                id,
                kind: LineKind::Action,
                text: "PlayEffect Red".into()
            })
            .changed
        );
    }

    #[test]
    fn duplicate_of_styled_rule_keeps_style_tag() {
        let mut wf = styled_wf("#ff:style S\nShow\n  SetFontSize 37\n");
        let id = wf.rows()[0].id;
        let o = wf.apply_action(RowAction::Duplicate(id));
        assert!(o.changed);
        assert_eq!(wf.text().matches("#ff:style S").count(), 2, "{}", wf.text());
        assert_eq!(wf.rows()[1].style, StyleState::Known("S".into()));
    }

    #[test]
    fn text_with_styles_drops_unknown_tag_on_the_clone_only() {
        let mut wf = styled_wf("#ff:style Gone\nShow\n  SetFontSize 40\n");
        assert!(wf.style_rewrite_pending());
        let (t, rep) = wf.text_with_styles().expect("ok");
        assert_eq!(rep.dropped, vec![(1, "Gone".to_string())]);
        assert!(!t.contains("ff:style"));
        assert!(t.contains("SetFontSize 40"));
        assert!(wf.text().contains("ff:style Gone"));
    }

    #[test]
    fn text_with_styles_rewrites_on_a_clone() {
        let mut wf = styled_wf("#ff:style S\nShow\n  Class \"Rings\"\n");
        assert!(wf.style_rewrite_pending(), "actions missing");
        let (t, rep) = wf.text_with_styles().expect("ok");
        assert_eq!(rep.rewritten, 1);
        assert!(t.contains(S_FONT));
        assert!(!wf.text().contains("SetFontSize"), "working doc untouched");
        // an already-matching filter is not pending
        let mut done = styled_wf(&t);
        assert!(!done.style_rewrite_pending());
        let (same, rep) = done.text_with_styles().expect("ok");
        assert_eq!((same, rep.rewritten), (t.clone(), 0));
        // a changed style makes it pending
        let base = test_styles();
        let mut s40 = base.get("S").expect("S").clone();
        s40.font = Some(40);
        let set = StyleSet::from_pairs(
            base.names()
                .map(|n| {
                    let st = if n == "S" {
                        s40.clone()
                    } else {
                        base.get(n).expect("n").clone()
                    };
                    (n.to_string(), st)
                })
                .collect::<Vec<_>>(),
        );
        done.set_styles(Ok(set));
        assert!(done.style_rewrite_pending());
        assert!(
            done.text_with_styles()
                .expect("ok")
                .0
                .contains("SetFontSize 40")
        );
        assert!(done.text().contains(S_FONT));
        // cache: no generation change from querying
        let g = done.generation();
        let _ = done.style_rewrite_pending();
        assert_eq!(done.generation(), g);
    }

    #[test]
    fn text_with_style_set_uses_the_given_set() {
        let wf = styled_wf("#ff:style S\nShow\n  Class \"Rings\"\n");
        let base = test_styles();
        let mut s40 = base.get("S").expect("S").clone();
        s40.font = Some(40);
        let set = StyleSet::from_pairs(
            base.names()
                .map(|n| {
                    let st = if n == "S" {
                        s40.clone()
                    } else {
                        base.get(n).expect("n").clone()
                    };
                    (n.to_string(), st)
                })
                .collect::<Vec<_>>(),
        );
        let (given, rep) = wf.text_with_style_set(&set);
        assert_eq!(rep.rewritten, 1);
        assert!(given.contains("SetFontSize 40") && !given.contains(S_FONT));
        // The loaded set is untouched: text_with_styles still renders it.
        let (loaded, _) = wf.text_with_styles().expect("ok");
        assert!(loaded.contains(S_FONT) && !loaded.contains("SetFontSize 40"));
    }

    #[test]
    fn unknown_styles_listed_enabled_only() {
        let wf =
            styled_wf("Show\n\n#ff:style Nope\nShow\n\n#ff:style Gone\n#Show\n#  Rarity Rare\n");
        let list = wf.unknown_styles();
        assert_eq!(list, vec![(2, wf.rows()[1].id)]);
    }

    #[test]
    fn styles_unavailable_only_blocks_when_styled_rules_exist() {
        let mut wf = WorkingFilter::from_text("#ff:style S\nShow\n");
        wf.set_styles(Err("bad json".into()));
        assert_eq!(
            wf.text_with_styles().unwrap_err(),
            styles_unavailable_message("bad json")
        );
        assert!(!wf.style_rewrite_pending());
        let mut plain = WorkingFilter::from_text("Show\n");
        plain.set_styles(Err("bad json".into()));
        let (t, rep) = plain.text_with_styles().expect("ok");
        assert_eq!((t, rep), ("Show\n".to_string(), StyleReport::default()));
    }

    #[test]
    fn tag_status_resolves_through_the_style_id() {
        let wf = styled_wf(
            "#ff:style builtin:S S\nShow\n\n#ff:style kite\nShow\n\n#ff:style zzzzzz kite\nShow\n",
        );
        let st: Vec<_> = wf.rows().iter().map(|r| r.style.clone()).collect();
        assert_eq!(
            st,
            vec![
                StyleState::Known("S".into()),
                StyleState::Known("kite".into()),
                StyleState::Unknown("zzzzzz kite".into()),
            ]
        );
        let mut un = WorkingFilter::from_text("#ff:style builtin:S S\nShow\n");
        assert_eq!(
            un.rows()[0].style,
            StyleState::Unchecked("builtin:S S".into())
        );
        un.set_styles(Err("x".into()));
        assert!(un.style_set().is_none());
    }

    #[test]
    fn renaming_a_style_retags_its_legacy_name_tags_instead_of_orphaning_them() {
        let mut wf = styled_wf("#ff:style kite\nShow\n\n#ff:style S\nShow\n");
        let mut set = test_styles();
        set.rename_custom("kite", "locator").expect("rename");
        wf.set_styles(Ok(set));
        assert_eq!(wf.rows()[0].style, StyleState::Known("locator".into()));
        let text = wf.doc.serialize();
        assert!(text.contains("# ff:style t5hw2k locator\n"), "{text}");
        assert!(
            text.contains("#ff:style S\n"),
            "untouched legacy tag kept: {text}"
        );
        // A migration, not a user edit (WR-04).
        assert!(!wf.is_dirty());
    }

    #[test]
    fn apply_without_a_config_part_keeps_legacy_tags() {
        let wf = styled_wf("#ff:style S\nShow\n  Class \"Rings\"\n");
        let (t, rep) = wf.text_with_styles_keeping_tags().expect("ok");
        assert!(t.starts_with("#ff:style S\nShow\n"), "{t}");
        assert!(t.contains(S_FONT), "{t}");
        assert_eq!((rep.retagged, rep.rewritten), (0, 1));
    }

    #[test]
    fn a_rules_revert_after_a_rename_keeps_the_migrated_tag() {
        let mut wf = styled_wf("#ff:style kite\nShow\n");
        let mut set = test_styles();
        set.rename_custom("kite", "locator").expect("rename");
        wf.set_styles(Ok(set.clone()));
        wf.revert();
        assert_eq!(wf.rows()[0].style, StyleState::Known("locator".into()));
        let (t, rep) = wf.text_with_style_set(&set);
        assert!(t.starts_with("# ff:style t5hw2k locator\n"), "{t}");
        assert!(rep.dropped.is_empty(), "{rep:?}");
    }

    #[test]
    fn a_fresh_document_follows_a_pending_rename_through_the_applied_set() {
        let disk = test_styles();
        let mut pending = disk.clone();
        pending.rename_custom("kite", "locator").expect("rename");
        // A newcomer named like the old style must not take over its tags.
        pending
            .insert_new("kite", disk.get("S").unwrap().clone())
            .expect("add");
        let mut wf = WorkingFilter::from_text("#ff:style kite\nShow\n");
        wf.set_styles_over(pending, &disk);
        assert_eq!(wf.rows()[0].style, StyleState::Known("locator".into()));
        assert!(!wf.is_dirty());
        wf.revert();
        assert_eq!(wf.rows()[0].style, StyleState::Known("locator".into()));
    }

    #[test]
    fn renamed_style_keeps_rules_linked_and_counted() {
        let mut wf = styled_wf("#ff:style t5hw2k kite\nShow\n");
        let mut set = test_styles();
        let id = set.id_of("kite").expect("id").to_string();
        assert_eq!(id, "t5hw2k");
        set.rename_custom("kite", "locator").expect("rename");
        wf.set_styles(Ok(set));
        assert_eq!(wf.rows()[0].style, StyleState::Known("locator".into()));
        assert_eq!(wf.style_users().get("locator"), Some(&1));
        assert!(wf.style_set().is_some());
        let (t, rep) = wf.text_with_styles().expect("ok");
        assert!(t.starts_with("# ff:style t5hw2k locator\n"), "{t}");
        assert_eq!(rep.retagged, 1);
    }

    #[test]
    fn apply_text_migrates_legacy_tags_but_pending_ignores_a_pure_migration() {
        let mut wf = styled_wf("#ff:style S\nShow\n  Class \"Rings\"\n");
        let (t, _) = wf.text_with_styles().expect("ok");
        // actions now match, only the tag is legacy
        let mut migr = styled_wf(&t.replace("# ff:style builtin:S S", "#ff:style S"));
        assert!(t.starts_with("# ff:style builtin:S S\n"), "{t}");
        assert!(!migr.style_rewrite_pending());
        let (t2, rep) = migr.text_with_styles().expect("ok");
        assert_eq!(rep.retagged, 1);
        assert!(t2.starts_with("# ff:style builtin:S S\n"), "{t2}");
        assert!(wf.style_rewrite_pending());
    }

    // ---- actions ----

    #[test]
    fn add_rule_inserts_at_top_of_file() {
        let mut wf = WorkingFilter::from_text(&with_economy());
        let o = wf.apply_action(RowAction::AddRule);
        assert!(o.changed);
        let id = o.focus.expect("focus");
        let row = wf.row(id).expect("row");
        assert_eq!(row.label, "1  Show");
        assert_eq!(wf.rows()[1].index, 2);
        assert_eq!(wf.rows().len(), 4);
        assert!(wf.is_dirty());
        assert_eq!(*wf.economy(), EconomyView::Present { blocks: 2 });
    }

    #[test]
    fn add_rule_above_a_group_keeps_the_group_one_rule() {
        let text = "# ff:group abc 1/2
Show
  Class \"Body Armours\"
  BaseArmour > 0

# ff:group abc 2/2
Show
  Class \"Body Armours\"
  BaseEvasion > 0
";
        let mut wf = WorkingFilter::from_text(text);
        assert_eq!(wf.rows().len(), 1);
        let o = wf.apply_action(RowAction::AddRule);
        assert!(o.changed);
        assert_eq!(wf.rows().len(), 2);
        assert_eq!(wf.rows()[0].label, "1  Show");
        assert_eq!(wf.rows()[1].index, 2);
    }

    #[test]
    fn duplicate_names_the_copy() {
        let mut wf = WorkingFilter::from_text("#ff:name Alpha\nShow\n  Rarity Rare\n\nHide\n");
        let src = wf.rows()[0].id;
        let o = wf.apply_action(RowAction::Duplicate(src));
        let copy = o.focus.expect("focus");
        assert_ne!(copy, src);
        assert_eq!(wf.rows()[1].id, copy);
        assert_eq!(wf.rows()[1].name.as_deref(), Some("Alpha (copy)"));
        assert_eq!(wf.rows()[0].name.as_deref(), Some("Alpha"));
        let un = wf.rows()[2].id;
        let o = wf.apply_action(RowAction::Duplicate(un));
        let c = wf.row(o.focus.expect("focus")).expect("row");
        assert!(c.name.is_none());
    }

    #[test]
    fn delete_and_move() {
        let mut wf = WorkingFilter::from_text(THREE);
        let ids: Vec<_> = wf.rows().iter().map(|r| r.id).collect();
        assert!(wf.apply_action(RowAction::Move(ids[1], Dir::Up)).changed);
        assert_eq!(wf.rows()[0].id, ids[1]);
        assert!(wf.apply_action(RowAction::Delete(ids[0])).changed);
        assert_eq!(wf.rows().len(), 2);
        let bad = wf.apply_action(RowAction::Move(ids[1], Dir::Up));
        assert!(!bad.changed && bad.status.is_some());
    }

    #[test]
    fn moving_across_continue_is_flagged_then_cleared() {
        let t =
            "Show\n  Rarity Rare\n  Continue\n\nHide\n  Rarity Normal\n\nShow\n  Class \"Rings\"\n";
        let mut wf = WorkingFilter::from_text(t);
        let ids: Vec<_> = wf.rows().iter().map(|r| r.id).collect();
        wf.apply_action(RowAction::Move(ids[1], Dir::Up));
        assert!(wf.row(ids[1]).expect("row").moved_across_continue);
        // Now order is [1, 0, 2]; moving 1 down crosses 0 (Continue) again.
        wf.apply_action(RowAction::Move(ids[1], Dir::Down));
        wf.apply_action(RowAction::Move(ids[2], Dir::Up));
        // ids[2] swapped with ids[1]; neither has Continue -> not flagged.
        assert!(!wf.row(ids[2]).expect("row").moved_across_continue);
        // Cross then re-move without crossing clears the flag.
        wf.apply_action(RowAction::Move(ids[1], Dir::Up));
        assert!(wf.row(ids[1]).is_some());
    }

    #[test]
    fn enable_round_trips_and_invalid_is_refused() {
        let mut wf =
            WorkingFilter::from_text("Show\n  Class \"Rings\"\n\n# Show\n#   Rarity Bogus\n");
        let id = wf.rows()[0].id;
        assert!(wf.apply_action(RowAction::SetEnabled(id, false)).changed);
        assert!(!wf.rows()[0].enabled);
        assert!(wf.apply_action(RowAction::SetEnabled(id, true)).changed);
        assert_eq!(wf.text(), wf.applied_text);
        let bad = wf.rows()[1].id;
        let o = wf.apply_action(RowAction::SetEnabled(bad, true));
        assert!(!o.changed);
        assert_eq!(
            o.status.as_deref(),
            Some("Can't enable: the commented lines are not valid filter grammar.")
        );
    }

    #[test]
    fn keyword_and_continue_edits() {
        let mut wf = WorkingFilter::from_text("Show\n  Class \"Rings\"\n");
        let id = wf.rows()[0].id;
        assert!(
            wf.apply_action(RowAction::SetKeyword(id, BlockKeyword::Hide))
                .changed
        );
        assert_eq!(wf.rows()[0].keyword, BlockKeyword::Hide);
        assert!(wf.apply_action(RowAction::SetContinue(id, true)).changed);
        assert!(wf.rows()[0].has_continue);
        let stale = BlockId(9999);
        let before = wf.text();
        let o = wf.apply_action(RowAction::SetKeyword(stale, BlockKeyword::Show));
        assert!(!o.changed && o.status.is_some());
        assert_eq!(wf.text(), before);
    }

    #[test]
    fn names_set_trim_and_clear() {
        let mut wf = WorkingFilter::from_text("Show\n  Class \"Rings\"\n");
        let id = wf.rows()[0].id;
        wf.apply_action(RowAction::SetName(id, "  My rule  ".into()));
        assert!(wf.text().contains("# ff:name My rule\n"));
        assert_eq!(wf.rows()[0].label, "1  Show - My rule");
        wf.apply_action(RowAction::SetName(id, "  ".into()));
        assert!(wf.rows()[0].name.is_none());
        wf.apply_action(RowAction::SetName(id, "x".into()));
        wf.apply_action(RowAction::ClearName(id));
        assert!(!wf.text().contains("ff:name"));
    }

    #[test]
    fn economy_staging_revert_and_applied() {
        let mut wf = WorkingFilter::from_text(THREE);
        let before = wf.text();
        wf.stage_add_economy(5);
        assert_eq!(*wf.economy(), EconomyView::Present { blocks: 5 });
        assert_eq!(wf.economy_plan(), EconomyPlan::Add);
        assert_eq!(wf.text(), before);
        assert!(wf.is_dirty());
        wf.stage_remove_economy();
        assert_eq!(wf.economy_plan(), EconomyPlan::Keep);
        assert!(!wf.is_dirty());

        let mut wf = WorkingFilter::from_text(&with_economy());
        wf.stage_remove_economy();
        assert_eq!(*wf.economy(), EconomyView::Absent);
        assert_eq!(wf.economy_plan(), EconomyPlan::Remove);
        wf.stage_add_economy(1);
        assert_eq!(wf.economy_plan(), EconomyPlan::Refresh);

        wf.stage_remove_economy();
        let id = wf.rows()[0].id;
        wf.apply_action(RowAction::Delete(id));
        assert!(wf.is_dirty());
        wf.revert();
        assert_eq!(wf.text(), with_economy());
        assert!(!wf.is_dirty());
        assert_eq!(wf.economy_plan(), EconomyPlan::Refresh);

        wf.apply_action(RowAction::AddRule);
        let new_text = wf.text();
        wf.on_applied(&new_text);
        assert!(!wf.is_dirty());
        wf.revert();
        assert_eq!(wf.text(), new_text);
    }

    #[test]
    fn count_blocks_counts() {
        assert_eq!(count_blocks(THREE), 3);
        assert_eq!(count_blocks(""), 0);
    }

    // ---- body view and line actions ----

    use crate::filter::tokenize::Operator as Op;

    fn edit(wf: &WorkingFilter, id: BlockId, i: usize, patch: LinePatch) -> RowAction {
        let e = wf
            .body_view(id)
            .into_iter()
            .find(|e| e.line == i)
            .expect("entry");
        RowAction::EditLine {
            id,
            line: i,
            expect: e.text,
            patch,
        }
    }

    #[test]
    fn body_view_types_every_line_in_order() {
        let t = "Show\n  Class \"Rings\" \"Amulets\"\n  ItemLevel >= 70\n  ItemLevel <= 80\n  SetFontSize 37\n  Continue\n  FutureKw 3\n";
        let wf = WorkingFilter::from_text(t);
        let id = wf.rows()[0].id;
        let v = wf.body_view(id);
        let kinds: Vec<_> = v.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![
                EntryKind::Condition,
                EntryKind::Condition,
                EntryKind::Condition,
                EntryKind::Action,
                EntryKind::Modifier,
                EntryKind::Opaque
            ]
        );
        assert_eq!(v[0].op, None);
        assert_eq!(v[0].values, vec!["\"Rings\"", "\"Amulets\""]);
        assert_eq!((v[1].ordinal, v[2].ordinal), (0, 1));
        assert_eq!((v[1].op, v[2].op), (Some(Op::Ge), Some(Op::Le)));
        for e in &v {
            assert_eq!(e.text, wf.doc.lines()[e.line].text());
        }
        assert!(wf.body_editable(id));
        assert!(wf.body_view(BlockId(9999)).is_empty());
        assert!(!wf.body_editable(BlockId(9999)));
    }

    #[test]
    fn body_view_disabled_blocks() {
        let wf = WorkingFilter::from_text("#Show\n#  Rarity>=Magic\n");
        let id = wf.rows()[0].id;
        let v = wf.body_view(id);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].op, Some(Op::Ge));
        assert_eq!(v[0].values, vec!["Magic"]);
        assert!(wf.body_editable(id));
        let bad = WorkingFilter::from_text("# Show\n#   Rarity Bogus\n");
        let id = bad.rows()[0].id;
        assert!(!bad.body_view(id).is_empty());
        assert!(!bad.body_editable(id));
    }

    #[test]
    fn insert_line_after_goes_directly_after_the_anchor() {
        let mut wf = WorkingFilter::from_text("Show\n  ItemLevel >= 60\n  Rarity Rare\n");
        let id = wf.rows()[0].id;
        let o = wf.apply_action(RowAction::InsertLineAfter {
            id,
            after: 1,
            expect: "  ItemLevel >= 60".into(),
            text: "ItemLevel <= 80".into(),
        });
        assert!(o.changed);
        assert_eq!(o.line, Some(2));
        assert_eq!(
            wf.text(),
            "Show\n  ItemLevel >= 60\n  ItemLevel <= 80\n  Rarity Rare\n"
        );
    }

    #[test]
    fn insert_line_after_refuses_stale_and_action_text() {
        let mut wf = WorkingFilter::from_text("Show\n  ItemLevel >= 60\n");
        let id = wf.rows()[0].id;
        let before = wf.text();
        let o = wf.apply_action(RowAction::InsertLineAfter {
            id,
            after: 1,
            expect: "  ItemLevel >= 99".into(),
            text: "ItemLevel <= 80".into(),
        });
        assert!(!o.changed);
        assert_eq!(o.status.as_deref(), Some(STALE_LINE_STATUS));
        let o = wf.apply_action(RowAction::InsertLineAfter {
            id,
            after: 1,
            expect: "  ItemLevel >= 60".into(),
            text: "SetFontSize 30".into(),
        });
        assert!(!o.changed);
        assert_eq!(o.status.as_deref(), Some(ACTION_IN_CONDITIONS_STATUS));
        assert_eq!(wf.text(), before);
    }

    #[test]
    fn coalesce_edits_leaves_insert_line_after_alone() {
        let ins = |t: &str| RowAction::InsertLineAfter {
            id: BlockId(1),
            after: 1,
            expect: "x".into(),
            text: t.into(),
        };
        let v = vec![ins("A >= 1"), ins("A <= 2")];
        assert_eq!(coalesce_edits(v.clone()), v);
    }

    #[test]
    fn edit_line_values_operator_and_revert() {
        let mut wf = WorkingFilter::from_text("Show\n  ItemLevel >= 70\n");
        let id = wf.rows()[0].id;
        let a = edit(&wf, id, 1, LinePatch::Values("75".into()));
        assert!(wf.apply_action(a).changed);
        assert!(wf.text().contains("ItemLevel >= 75"));
        assert!(wf.is_dirty());
        let a = edit(&wf, id, 1, LinePatch::Operator(Some(Op::Le)));
        assert!(wf.apply_action(a).changed);
        assert!(wf.text().contains("ItemLevel <= 75"));
        wf.revert();
        assert_eq!(wf.text(), "Show\n  ItemLevel >= 70\n");
        assert!(!wf.is_dirty());
    }

    #[test]
    fn stale_and_unsupported_line_edits_are_refused() {
        let mut wf = WorkingFilter::from_text("Show\n  ItemLevel >= 70\n");
        let id = wf.rows()[0].id;
        let before = wf.text();
        let o = wf.apply_action(RowAction::EditLine {
            id,
            line: 1,
            expect: "  ItemLevel >= 99".into(),
            patch: LinePatch::Values("75".into()),
        });
        assert!(!o.changed);
        assert_eq!(o.status.as_deref(), Some(STALE_LINE_STATUS));
        let o = wf.apply_action(RowAction::RemoveLine {
            id,
            line: 40,
            expect: "x".into(),
        });
        assert_eq!(o.status.as_deref(), Some(STALE_LINE_STATUS));
        let a = edit(&wf, id, 1, LinePatch::Values("Bogus Token \"".into()));
        let o = wf.apply_action(a);
        assert!(!o.changed && o.status.is_some());
        assert_eq!(wf.text(), before);
        let o = wf.apply_action(RowAction::AddLine {
            id: BlockId(9999),
            kind: LineKind::Condition,
            text: "Quality >= 1".into(),
        });
        assert_eq!(o.status.as_deref(), Some(NOT_EDITABLE_STATUS));
        let mut bad = WorkingFilter::from_text("# Show\n#   Rarity Bogus\n");
        let id = bad.rows()[0].id;
        let o = bad.apply_action(RowAction::RemoveLine {
            id,
            line: 1,
            expect: "#   Rarity Bogus".into(),
        });
        assert_eq!(o.status.as_deref(), Some(NOT_EDITABLE_STATUS));
    }

    #[test]
    fn add_line_placement() {
        let mut wf = WorkingFilter::from_text("Show\n  Class \"Rings\"\n  SetFontSize 37\n");
        let id = wf.rows()[0].id;
        let o = wf.apply_action(RowAction::AddLine {
            id,
            kind: LineKind::Condition,
            text: "Quality >= 1".into(),
        });
        assert!(o.changed);
        assert_eq!((o.focus, o.line), (Some(id), Some(2)));
        assert_eq!(
            wf.text(),
            "Show\n  Class \"Rings\"\n  Quality >= 1\n  SetFontSize 37\n"
        );
        wf.apply_action(RowAction::AddLine {
            id,
            kind: LineKind::Action,
            text: "SetFontSize 40".into(),
        });
        assert!(wf.text().ends_with("  SetFontSize 37\n  SetFontSize 40\n"));

        let mut bare = WorkingFilter::from_text("Show\n  SetFontSize 37\n");
        let id = bare.rows()[0].id;
        bare.apply_action(RowAction::AddLine {
            id,
            kind: LineKind::Condition,
            text: "Quality >= 1".into(),
        });
        assert_eq!(bare.text(), "Show\n  Quality >= 1\n  SetFontSize 37\n");
        let mut cond_only = WorkingFilter::from_text("Show\n  Quality >= 1\n");
        let id = cond_only.rows()[0].id;
        cond_only.apply_action(RowAction::AddLine {
            id,
            kind: LineKind::Action,
            text: "SetFontSize 40".into(),
        });
        assert_eq!(cond_only.text(), "Show\n  Quality >= 1\n  SetFontSize 40\n");
    }

    #[test]
    fn remove_line_and_opaque_edit() {
        let mut wf = WorkingFilter::from_text("Show\n  ItemLevel >= 70\n  FutureKw 3\n");
        let id = wf.rows()[0].id;
        let a = edit(&wf, id, 2, LinePatch::Whole("FutureKw 4".into()));
        assert!(wf.apply_action(a).changed);
        assert!(wf.text().contains("  FutureKw 4\n"));
        let a = edit(&wf, id, 2, LinePatch::Whole("ItemLevel >= 70".into()));
        assert!(wf.apply_action(a).changed);
        let kinds: Vec<_> = wf.body_view(id).iter().map(|e| e.kind).collect();
        assert_eq!(kinds, vec![EntryKind::Condition, EntryKind::Condition]);
        let before = wf.text();
        let a = edit(&wf, id, 2, LinePatch::Whole("Class".into()));
        let o = wf.apply_action(a);
        assert!(!o.changed && o.status.is_some());
        assert_eq!(wf.text(), before);
        let e = wf.body_view(id)[0].clone();
        let o = wf.apply_action(RowAction::RemoveLine {
            id,
            line: e.line,
            expect: e.text,
        });
        assert!(o.changed);
        assert_eq!(wf.text(), "Show\n  ItemLevel >= 70\n");
    }

    #[test]
    fn coalesce_keeps_last_edit_per_line() {
        let (a, b) = (BlockId(1), BlockId(2));
        let ed = |l: usize, v: &str| RowAction::EditLine {
            id: a,
            line: l,
            expect: String::new(),
            patch: LinePatch::Values(v.into()),
        };
        let out = coalesce_edits(vec![
            ed(1, "71"),
            RowAction::SetEnabled(b, false),
            ed(1, "72"),
        ]);
        assert_eq!(out, vec![RowAction::SetEnabled(b, false), ed(1, "72")]);
        let out = coalesce_edits(vec![ed(1, "71"), ed(2, "72")]);
        assert_eq!(out, vec![ed(1, "71"), ed(2, "72")]);
    }

    // -- grouped rules (`# ff:group`) ------------------------------------------------------------

    const PART1: &str = "# ff:name Pure\n# ff:group abc 1/2\nShow\n  Rarity Rare\n  BaseArmour > 0\n  BaseEvasion 0\n  BaseEnergyShield 0\n  SetFontSize 30\n";
    const PART2: &str = "# ff:name Pure\n# ff:group abc 2/2\nShow\n  Rarity Rare\n  BaseArmour 0\n  BaseEvasion > 0\n  BaseEnergyShield 0\n  SetFontSize 30\n";

    /// Rule 1 (single), rule 2 (a two-block group), rule 3 (single).
    fn grouped() -> String {
        format!("Show\n  Rarity Normal\n\n{PART1}\n{PART2}\nShow\n  Rarity Magic\n")
    }

    /// Every row's group is valid and its size matches the document.
    fn assert_groups_intact(wf: &WorkingFilter) {
        for r in wf.rows() {
            assert!(!r.group_broken, "{}: {}", r.label, wf.text());
            assert_eq!(wf.groups.members(r.id).len(), r.group_size);
        }
    }

    fn group_row(wf: &WorkingFilter) -> &RuleRow {
        wf.rows()
            .iter()
            .find(|r| r.group_size > 1)
            .expect("a group row")
    }

    #[test]
    fn a_group_is_one_row_and_stays_byte_identical() {
        let src = grouped();
        let mut wf = styled_wf(&src);
        assert_eq!(wf.rows().len(), 3);
        let r = &wf.rows()[1];
        assert_eq!((r.index, r.group_size), (2, 2));
        assert_eq!(r.label, "2  Show - Pure");
        assert_eq!(r.simple_mask, Some(0b011));
        assert_eq!(r.summary, "Rarity Rare; Defences: Full AR, Full EV");
        assert_eq!(wf.rows()[2].index, 3);
        assert_eq!(wf.matches("pure"), vec![1]);
        assert!(wf.rows()[0].can_move_down && wf.rows()[2].can_move_up);
        assert_eq!(wf.text(), src);
        assert_eq!(wf.text_with_styles().unwrap().0, src);
        assert!(!wf.style_rewrite_pending());
        assert!(!wf.is_dirty());
    }

    #[test]
    fn a_broken_group_is_separate_flagged_rules_and_is_not_rewritten() {
        let src = grouped().replacen("SetFontSize 30", "SetFontSize 31", 1);
        let wf = styled_wf(&src);
        assert_eq!(wf.rows().len(), 4);
        assert!(wf.rows()[1].group_broken && wf.rows()[2].group_broken);
        assert_eq!(wf.rows()[1].group_size, 1);
        assert!(wf.blocking().is_empty());
        assert_eq!(wf.text_with_styles().unwrap().0, src);
    }

    #[test]
    fn single_block_selection_reads_its_zero_based_lines() {
        let wf = WorkingFilter::from_text("Show\n  BaseArmour > 0\n  BaseEvasion 0\n");
        // Has AR, no EV: Full AR, AR/ES.
        assert_eq!(wf.rows()[0].simple_mask, Some(0b001_0001));
        let wf = WorkingFilter::from_text("Show\n  BaseArmour > 50\n");
        assert_eq!(wf.rows()[0].simple_mask, None);
        let wf = WorkingFilter::from_text("Show\n  Rarity Rare\n");
        assert_eq!(wf.rows()[0].simple_mask, None);
    }

    #[test]
    fn edits_on_a_group_keep_its_blocks_identical() {
        let mut wf = styled_wf(&grouped());
        let id = group_row(&wf).id;
        let rarity = wf
            .body_view(id)
            .into_iter()
            .find(|e| e.text.contains("Rarity"))
            .unwrap();
        let actions = vec![
            RowAction::SetName(id, "Renamed".into()),
            RowAction::SetKeyword(id, BlockKeyword::Hide),
            RowAction::SetContinue(id, true),
            edit(&wf, id, rarity.line, LinePatch::Values("Unique".into())),
            RowAction::AddLine {
                id,
                kind: LineKind::Condition,
                text: "ItemLevel >= 80".into(),
            },
            RowAction::AddLine {
                id,
                kind: LineKind::Action,
                text: "SetFontSize 40".into(),
            },
            RowAction::ApplyStyle(id, "S".into()),
            RowAction::RemoveStyle(id),
        ];
        for a in actions {
            let out = wf.apply_action(a.clone());
            assert!(out.changed, "{a:?}: {:?}", out.status);
            assert_eq!(wf.rows().len(), 3, "{a:?}\n{}", wf.text());
            assert_groups_intact(&wf);
            assert_eq!(group_row(&wf).simple_mask, Some(0b011), "{a:?}");
        }
        let t = wf.text();
        assert_eq!(t.matches("# ff:name Renamed").count(), 2, "{t}");
        assert_eq!(t.matches("ItemLevel >= 80").count(), 2, "{t}");
        assert_eq!(t.matches("Rarity Unique").count(), 2, "{t}");
        // The second block kept its own defence lines.
        assert!(t.contains("  BaseArmour 0\n  BaseEvasion > 0\n"), "{t}");
    }

    #[test]
    fn enable_disable_delete_and_duplicate_act_on_the_whole_group() {
        let mut wf = WorkingFilter::from_text(&grouped());
        let id = group_row(&wf).id;
        assert!(wf.apply_action(RowAction::SetEnabled(id, false)).changed);
        assert_eq!(wf.rows().len(), 3);
        assert!(!group_row(&wf).enabled);
        assert_groups_intact(&wf);
        assert!(wf.apply_action(RowAction::SetEnabled(id, true)).changed);
        assert_eq!(wf.text(), grouped());

        let out = wf.apply_action(RowAction::Duplicate(id));
        assert!(out.changed);
        assert_eq!(wf.rows().len(), 4);
        assert_groups_intact(&wf);
        let copy = out.focus.unwrap();
        let r = wf.row(copy).unwrap();
        assert_eq!((r.index, r.group_size), (3, 2));
        assert_eq!(r.name.as_deref(), Some("Pure (copy)"));
        assert!(
            wf.text()
                .starts_with(&format!("Show\n  Rarity Normal\n\n{PART1}\n{PART2}\n"))
        );

        assert!(wf.apply_action(RowAction::Delete(id)).changed);
        assert_eq!(wf.rows().len(), 3);
        assert!(!wf.text().contains("abc"));
        assert_groups_intact(&wf);
    }

    #[test]
    fn a_group_moves_as_one_unit() {
        let src = grouped();
        let mut wf = WorkingFilter::from_text(&src);
        let id = group_row(&wf).id;
        assert!(wf.apply_action(RowAction::Move(id, Dir::Up)).changed);
        assert_eq!(group_row(&wf).index, 1);
        assert_groups_intact(&wf);
        assert!(!wf.rows()[0].can_move_up && wf.rows()[0].can_move_down);
        assert!(wf.apply_action(RowAction::Move(id, Dir::Down)).changed);
        assert_eq!(wf.text(), src);
        // A single rule passes the whole group too, in either direction.
        let last = wf.rows()[2].id;
        assert!(wf.apply_action(RowAction::Move(last, Dir::Up)).changed);
        assert_eq!(wf.rows()[1].id, last);
        assert_eq!(wf.rows()[2].group_size, 2);
        assert_groups_intact(&wf);
        assert!(wf.apply_action(RowAction::Move(last, Dir::Down)).changed);
        assert_eq!(wf.text(), src);
    }

    #[test]
    fn moving_a_group_across_continue_is_flagged() {
        let src = grouped().replace(
            "Show\n  Rarity Normal\n",
            "Show\n  Rarity Normal\n  Continue\n",
        );
        let mut wf = WorkingFilter::from_text(&src);
        let id = group_row(&wf).id;
        assert!(wf.apply_action(RowAction::Move(id, Dir::Up)).changed);
        assert!(wf.row(id).unwrap().moved_across_continue);
    }

    #[test]
    fn selection_changes_grow_and_collapse_the_rule() {
        let mut wf = WorkingFilter::from_text("Show\n  Rarity Rare\n\nShow\n  Rarity Magic\n");
        let id = wf.rows()[0].id;
        let set = |wf: &mut WorkingFilter, mask: u8| {
            let out = wf.apply_action(RowAction::SetDefenceSelection { id, mask });
            assert!(out.changed, "{out:?}");
        };
        // The three pure bases: three blocks, one row.
        set(&mut wf, 0b111);
        assert_eq!(wf.rows().len(), 2);
        assert_eq!(wf.rows()[0].group_size, 3);
        assert_eq!(wf.rows()[0].simple_mask, Some(0b111));
        assert_eq!(wf.rows()[1].index, 2);
        assert_groups_intact(&wf);
        // Plus the triple hybrid: four.
        set(&mut wf, 0b100_0111);
        assert_eq!(wf.rows()[0].group_size, 4);
        assert_groups_intact(&wf);
        // "No EV": one block, marker gone.
        set(&mut wf, 0b001_0101);
        assert_eq!(wf.rows()[0].group_size, 1);
        assert_eq!(
            wf.text(),
            "Show\n  Rarity Rare\n  BaseEvasion 0\n\nShow\n  Rarity Magic\n"
        );
        // Empty selection (the card's X): no defence lines at all.
        set(&mut wf, 0);
        assert_eq!(wf.text(), "Show\n  Rarity Rare\n\nShow\n  Rarity Magic\n");
    }

    #[test]
    fn a_group_on_a_disabled_rule_can_be_re_covered() {
        let mut wf = WorkingFilter::from_text(&grouped());
        let id = group_row(&wf).id;
        assert!(wf.apply_action(RowAction::SetEnabled(id, false)).changed);
        let out = wf.apply_action(RowAction::SetDefenceSelection { id, mask: 0b111 });
        assert!(out.changed, "{out:?}");
        assert_eq!(group_row(&wf).group_size, 3);
        assert!(!group_row(&wf).enabled);
        assert_groups_intact(&wf);
    }

    #[test]
    fn styles_rewrite_every_block_and_count_the_rule_once() {
        let src = grouped().replace("# ff:name Pure\n", "# ff:name Pure\n# ff:style S\n");
        let wf = styled_wf(&src);
        assert_eq!(group_row(&wf).style, StyleState::Known("S".into()));
        assert_eq!(wf.style_users().get("S"), Some(&1));
        let (text, report) = wf.text_with_styles_keeping_tags().unwrap();
        assert_eq!(report.rewritten, 1);
        assert_eq!(text.matches("SetFontSize 37").count(), 2, "{text}");
        let applied = WorkingFilter::from_text(&text);
        assert_eq!(applied.rows().len(), 3);
        assert_eq!(group_row(&applied).group_size, 2);

        let unknown =
            styled_wf(&grouped().replace("# ff:name Pure\n", "# ff:name Pure\n# ff:style Nope\n"));
        assert_eq!(unknown.unknown_styles().len(), 1);
        let (text, report) = unknown.text_with_styles().unwrap();
        assert_eq!(report.dropped, vec![(2, "Nope".to_string())]);
        assert!(!text.contains("Nope"));
        assert_eq!(group_row(&WorkingFilter::from_text(&text)).group_size, 2);
    }

    #[test]
    fn grammar_errors_in_a_group_block_apply_once_by_rule_number() {
        let src = grouped().replace("SetFontSize 30", "SetFontSize 30\n  Rarity Bogus");
        let wf = WorkingFilter::from_text(&src);
        assert_eq!(wf.rows().len(), 3);
        assert_eq!(wf.blocking().len(), 1);
        assert_eq!(wf.blocking()[0].rule_index, 2);
    }

    #[test]
    fn follower_ids_are_not_rows_and_refuse_edits() {
        let mut wf = WorkingFilter::from_text(&grouped());
        let id = group_row(&wf).id;
        let follower = wf.groups.members(id)[1];
        assert!(wf.row(follower).is_none());
        let out = wf.apply_action(RowAction::AddLine {
            id: follower,
            kind: LineKind::Condition,
            text: "ItemLevel >= 1".into(),
        });
        assert!(!out.changed);
        assert_eq!(wf.text(), grouped());
    }

    #[test]
    fn edits_that_would_break_a_group_are_refused() {
        let mut wf = WorkingFilter::from_text(&grouped());
        let id = group_row(&wf).id;
        let follower = wf.groups.members(id)[1];
        // A raw defence line on the leader cannot be mirrored: refused, nothing changes.
        let out = wf.apply_action(RowAction::AddLine {
            id,
            kind: LineKind::Condition,
            text: "BaseArmour >= 100".into(),
        });
        assert!(!out.changed);
        assert_eq!(out.status.as_deref(), Some(GROUP_DEFENCES_STATUS));
        assert_eq!(wf.text(), grouped());
        // A follower id is not a rule: no action acts on it alone.
        for a in [
            RowAction::Delete(follower),
            RowAction::SetEnabled(follower, false),
            RowAction::Duplicate(follower),
            RowAction::Move(follower, Dir::Down),
            RowAction::SetDefenceSelection {
                id: follower,
                mask: 1,
            },
        ] {
            assert!(!wf.apply_action(a.clone()).changed, "{a:?}");
            assert_eq!(wf.text(), grouped(), "{a:?}");
        }
    }

    #[test]
    fn legacy_ff_spelling_does_not_break_a_group() {
        let src = grouped().replacen("# ff:name Pure", "#ff:name Pure", 1);
        let wf = WorkingFilter::from_text(&src);
        assert_eq!(wf.rows().len(), 3);
        assert_eq!(group_row(&wf).group_size, 2);
    }
}
