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
    Block, BlockId, BlockKeyword, DisabledState, Document, FilterDiagnostic, LinePatch, RuleStep,
    Severity, parse as parse_filter, parse_with_id_base,
};
use crate::models::config::{Cancelable, Style, StyleAction, StyleSet};
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
    /// Insert a section divider at the top of the list (where `AddRule` inserts).
    AddSection,
    /// Insert a section divider directly above a rule (or its whole group).
    AddSectionAbove(BlockId),
    /// Rename the section whose banner has this ordinal.
    RenameSection {
        ordinal: usize,
        name: String,
    },
    /// Remove a section divider; its rules stay.
    RemoveSection(usize),
    /// Move a section divider past one rule (or group).
    MoveSection(usize, Dir),
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
        RowAction::AddRule
        | RowAction::AddSection
        | RowAction::RenameSection { .. }
        | RowAction::RemoveSection(_)
        | RowAction::MoveSection(..) => None,
        RowAction::AddSectionAbove(id) => Some(*id),
    }
}

/// Keep only the last `EditLine` per `(id, line, patch variant)`, at the position of the last;
/// every other action is untouched and in order, except that section renames run first.
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
    let (renames, rest): (Vec<_>, Vec<_>) = actions
        .into_iter()
        .zip(keep)
        .filter_map(|(a, k)| k.then_some(a))
        .partition(|a| matches!(a, RowAction::RenameSection { .. }));
    // A rename addresses its section by ordinal and commits on the focus loss that any other
    // click in the same frame also causes: run it before a structural action shifts the ordinals.
    renames.into_iter().chain(rest).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionOutcome {
    pub changed: bool,
    pub status: Option<String>,
    pub focus: Option<BlockId>,
    /// Doc line index of a line `AddLine` created.
    pub line: Option<usize>,
    /// Ordinal of a section an action created.
    pub section: Option<usize>,
}

/// Refusal of any edit aimed at the generated economy section.
pub const ECONOMY_LOCKED_STATUS: &str =
    "The FF Economy Section is generated by FerrisFilter and cannot be changed here.";
/// The economy section's name in the RULES list (display only: the banner written in the file
/// keeps [`crate::filter::section::BEGIN_LINE`], and its collapsed flag keeps the economy key).
pub const ECONOMY_DISPLAY_NAME: &str = "FF Economy Section";
const SECTION_ADDED_TOP_STATUS: &str = "Section added at the top. Press Apply to save it.";
const SECTION_REMOVED_STATUS: &str = "Section removed. Its rules joined the section above.";
const SECTION_CANT_MOVE_STATUS: &str = "That section can't move further in that direction.";
pub const DEFAULT_SECTION_NAME: &str = "Section";

/// One section divider as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionRow {
    /// Index of the banner in the document (the economy banner included).
    pub ordinal: usize,
    pub name: String,
    pub economy: bool,
    /// Rules in the section (a `# ff:group` run counts once); economy: its block count.
    pub rules: usize,
    pub can_move_up: bool,
    pub can_move_down: bool,
    /// False for the economy header and for the divider right below the economy section.
    pub can_remove: bool,
}

/// One line of the displayed list, in file order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListEntry {
    /// A section header. `section` indexes [`WorkingFilter::sections`]; `collapsed` is the
    /// display state (always false during a search); `matches` is `Some(m)` only during one.
    Header {
        section: usize,
        collapsed: bool,
        matches: Option<usize>,
    },
    /// Index into [`WorkingFilter::rows`].
    Rule(usize),
    /// Index into [`WorkingFilter::economy_rows`].
    EconomyRule(usize),
}

/// File-order skeleton used to answer move and list questions without rescanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    /// A user rule unit (index into `rows`).
    Unit(usize),
    /// A user banner (ordinal).
    Banner(usize),
    EconomyBanner,
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
    /// Ordinal of the containing banner; `None` above the first banner and for rules written
    /// after the economy section (they belong to no divider).
    pub section: Option<usize>,
    /// The collapsed-row style chip's minimap icon as (shape, colour) indices into
    /// `skin::minimap::SHAPES` / `COLOURS`: the known style's minimap, or for an untagged rule
    /// its first representable `MinimapIcon` line. Built at row rebuild, never per frame.
    pub chip_icon: Option<(usize, usize)>,
    /// The chip icon in words (`Medium Blue Star`), for an untagged rule's chip tooltip.
    pub chip_icon_desc: Option<String>,
}

/// The chip icon (shape, colour indices) and its description for a minimap spec. Off and the
/// cancel value (`MinimapIcon -1`) have no icon.
fn chip_icon_of(m: &crate::models::config::Minimap) -> Option<((usize, usize), String)> {
    use crate::skin::minimap;
    let shape = minimap::index_of(&minimap::SHAPES, m.shape.as_str())?;
    let colour = minimap::index_of(&minimap::COLOURS, m.color.as_str())?;
    let size = minimap::SIZES.get(usize::from(m.size))?;
    Some((
        (shape, colour),
        format!("{size} {} {}", m.color.as_str(), m.shape.as_str()),
    ))
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
    economy_rows: Vec<RuleRow>,
    /// Sections as in the file (economy included when present).
    base_sections: Vec<SectionRow>,
    /// `base_sections` with the staged economy op applied.
    sections: Vec<SectionRow>,
    /// Rules and banners in file order (the economy blocks sit after `EconomyBanner`).
    layout: Vec<Item>,
    shadowing: HashSet<BlockId>,
    base_economy: EconomyView,
    pending: Option<Pending>,
    /// While an economy add is staged: the economy section as Apply will write it, parsed on its
    /// own with block ids disjoint from `doc`, so its rows can be browsed read-only.
    preview: Option<Box<WorkingFilter>>,
    projected: EconomyView,
    blocking: Vec<BlockingError>,
    /// Grouped rules (several blocks written as one rule), recomputed on every rebuild.
    groups: GroupIndex,
    moved: HashSet<BlockId>,
    generation: u64,
    dirty: bool,
    /// The rule text differs from the applied text (a staged economy op does not count).
    rules_edited: bool,
    search_cache: Option<(String, u64, Vec<usize>)>,
    styles: Option<StyleSet>,
    /// The applied (on-disk) style set that legacy name-only tags were written against, so a
    /// freshly parsed document (RELOAD, filter switch) still follows a pending rename.
    legacy_base: Option<StyleSet>,
    styles_error: Option<String>,
    styles_rev: u64,
    pending_cache: Option<(u64, u64, bool)>,
}

/// An economy body of `n` named blocks, for tests that stage an add.
#[cfg(test)]
pub fn tests_econ_body(n: usize) -> String {
    (0..n)
        .map(|k| format!("# ff:name Block {k}\nShow\n  Class \"Rings\"\n\n"))
        .collect::<String>()
        .trim_end()
        .to_string()
        + "\n"
}

/// First block id of the staged-economy preview document: far above any id the working document
/// hands out, so the two never collide.
const PREVIEW_ID_BASE: u64 = 1 << 48;

/// True for a block id handed out by the staged-economy preview document. Those ids restart on
/// every re-price, so an expanded row holding one must be collapsed when that happens.
pub fn is_preview_id(id: crate::filter::BlockId) -> bool {
    id.0 >= PREVIEW_ID_BASE
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

/// Refusal text of a failed section edit.
fn section_refusal(e: &crate::filter::EditError) -> String {
    match e {
        crate::filter::EditError::SectionLocked => ECONOMY_LOCKED_STATUS.to_string(),
        other => other.to_string(),
    }
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
    /// The style state and chip of a block: its `# ff:style` tag resolved against the loaded
    /// styles, or (untagged) the effective `MinimapIcon` line. Shared by user and economy rows.
    fn style_of(
        &self,
        id: BlockId,
        b: &Block,
    ) -> (StyleState, Option<(usize, usize)>, Option<String>) {
        let mut chip = None;
        let style = match self.doc.meta_get(id, "style") {
            None => {
                // Untagged: the effective (last) MinimapIcon line the Custom style lines
                // bind, when it is a shown icon (`MinimapIcon -1` shows none).
                let has_icon = b
                    .body()
                    .iter()
                    .any(|bl| bl.keyword.is_some_and(|k| k.name == "MinimapIcon"));
                if has_icon {
                    let entries = self.body_view(id);
                    chip = super::body::rule_style_view(&entries, b.keyword())
                        .0
                        .minimap
                        .as_ref()
                        .and_then(Cancelable::value)
                        .and_then(chip_icon_of);
                }
                StyleState::None
            }
            Some(n) => match &self.styles {
                Some(set) => match set.resolve_tag(n) {
                    Some(m) => {
                        chip = m
                            .style
                            .minimap
                            .as_ref()
                            .and_then(Cancelable::value)
                            .and_then(chip_icon_of);
                        StyleState::Known(m.name.to_string())
                    }
                    None => StyleState::Unknown(n.to_string()),
                },
                None => StyleState::Unchecked(n.to_string()),
            },
        };
        let (chip_icon, chip_icon_desc) = chip.map_or((None, None), |(i, d)| (Some(i), Some(d)));
        (style, chip_icon, chip_icon_desc)
    }

    /// A read-only row for one generated economy block: searchable, never editable.
    /// The keyword a user rule's row shows: an enabled rule styled with a known preset takes the
    /// preset's Show/Hide from the loaded (pending) styles, as the next Apply will write it;
    /// anything else shows the keyword in the file.
    fn shown_keyword(&self, b: &Block, style: &StyleState) -> BlockKeyword {
        let preset = match style {
            StyleState::Known(name) if b.is_enabled() => {
                self.styles.as_ref().and_then(|s| s.get(name))
            }
            _ => None,
        };
        match preset.map(|s| s.action) {
            Some(StyleAction::Show) => BlockKeyword::Show,
            Some(StyleAction::Hide) => BlockKeyword::Hide,
            None => b.keyword(),
        }
    }

    fn economy_row(&self, b: &Block) -> RuleRow {
        let doc = &self.doc;
        let x = extract(doc, b);
        let name = doc.meta_get(b.id(), "name").map(str::to_string);
        let (style, chip_icon, chip_icon_desc) = self.style_of(b.id(), b);
        let kw = keyword_word(b.keyword());
        let label = match &name {
            Some(n) => format!("{kw} - {n}"),
            None => kw.to_string(),
        };
        let mut hay = String::new();
        if let Some(n) = &name {
            hay.push_str(n);
            hay.push('\n');
        }
        hay.push_str(&label);
        for v in x.class.iter().chain(x.base_type.iter()) {
            hay.push('\n');
            hay.push_str(v);
        }
        RuleRow {
            id: b.id(),
            index: 0,
            enabled: b.is_enabled(),
            keyword: b.keyword(),
            has_continue: b.has_continue(),
            name,
            style,
            label,
            haystack: hay.to_lowercase(),
            errors: Vec::new(),
            warnings: Vec::new(),
            cannot_enable: false,
            can_move_up: false,
            can_move_down: false,
            moved_across_continue: false,
            summary: row_summary(&x.conditions),
            conditions: x.conditions,
            group_size: 1,
            group_broken: false,
            simple_mask: None,
            section: None,
            chip_icon,
            chip_icon_desc,
        }
    }
}

impl WorkingFilter {
    pub fn from_text(text: &str) -> Self {
        Self::from_text_based(text, 0)
    }

    fn from_text_based(text: &str, id_base: u64) -> Self {
        let mut wf = WorkingFilter {
            doc: parse_with_id_base(text.as_bytes(), id_base).unwrap_or_default(),
            applied_text: text.to_string(),
            rows: Vec::new(),
            economy_rows: Vec::new(),
            base_sections: Vec::new(),
            sections: Vec::new(),
            layout: Vec::new(),
            shadowing: HashSet::new(),
            base_economy: EconomyView::Absent,
            pending: None,
            preview: None,
            projected: EconomyView::Absent,
            blocking: Vec::new(),
            groups: GroupIndex::default(),
            moved: HashSet::new(),
            generation: 0,
            dirty: false,
            rules_edited: false,
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

        let mut rows: Vec<RuleRow> = Vec::new();
        let mut economy_rows: Vec<RuleRow> = Vec::new();
        // Side of the economy section and header line of each row, for the layout pass below.
        let mut row_meta: Vec<(u8, usize)> = Vec::new();
        let mut index = 0usize;
        for (b, econ, side) in &order {
            if *econ {
                economy_rows.push(self.economy_row(b));
                continue;
            }
            if groups.is_follower(b.id()) {
                continue;
            }
            index += 1;
            let id = b.id();
            let members = groups.members(id);
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
            let (style, chip_icon, chip_icon_desc) = self.style_of(id, b);
            let keyword = self.shown_keyword(b, &style);
            let kw = keyword_word(keyword);
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
            row_meta.push((*side, b.header_line()));
            rows.push(RuleRow {
                id,
                index,
                enabled,
                keyword,
                has_continue: b.has_continue(),
                name,
                style,
                label,
                haystack: hay.to_lowercase(),
                errors,
                warnings,
                cannot_enable: !enabled && !b.can_enable(),
                // Filled in by the layout pass below.
                can_move_up: false,
                can_move_down: false,
                moved_across_continue: self.moved.contains(&id),
                summary: row_summary(&x.conditions),
                conditions: x.conditions,
                group_size: members.len(),
                group_broken: groups.is_broken(id),
                simple_mask,
                section: None,
                chip_icon,
                chip_icon_desc,
            });
        }

        // One pass over rules and banners in file order. A rule can step to a neighbouring rule
        // on the same side of the economy section, or hop a user divider; never the economy
        // banner. Mirrors `Document::rule_step` without calling it per row.
        let banners = section::banners(&self.doc);
        let mut layout: Vec<Item> = Vec::new();
        let mut bi = 0usize;
        let mut containing: Option<usize> = None;
        for (r, row) in rows.iter_mut().enumerate() {
            while bi < banners.len() && banners[bi].start < row_meta[r].1 {
                layout.push(match banners[bi].kind {
                    section::BannerKind::Section => Item::Banner(bi),
                    section::BannerKind::Economy => Item::EconomyBanner,
                });
                containing = (banners[bi].kind == section::BannerKind::Section).then_some(bi);
                bi += 1;
            }
            row.section = containing;
            layout.push(Item::Unit(r));
        }
        for (k, b) in banners.iter().enumerate().skip(bi) {
            layout.push(match b.kind {
                section::BannerKind::Section => Item::Banner(k),
                section::BannerKind::Economy => Item::EconomyBanner,
            });
        }
        if damaged.is_none() {
            let steps: Vec<(usize, bool, bool)> = layout
                .iter()
                .enumerate()
                .filter_map(|(i, it)| {
                    let Item::Unit(r) = it else { return None };
                    let ok = |n: Option<&Item>| match n {
                        Some(Item::Unit(o)) => row_meta[*o].0 == row_meta[*r].0,
                        Some(Item::Banner(_)) => true,
                        Some(Item::EconomyBanner) | None => false,
                    };
                    // Hopping up over the divider right after the economy section would land
                    // the rule inside it.
                    let up = match i.checked_sub(1) {
                        Some(j) if matches!(layout[j], Item::Banner(_)) => {
                            j == 0 || layout[j - 1] != Item::EconomyBanner
                        }
                        j => ok(j.and_then(|j| layout.get(j))),
                    };
                    Some((*r, up, ok(layout.get(i + 1))))
                })
                .collect();
            for (r, up, down) in steps {
                rows[r].can_move_up = up;
                rows[r].can_move_down = down;
            }
        }

        let mut tally = vec![0usize; banners.len()];
        for row in &rows {
            if let Some(o) = row.section {
                tally[o] += 1;
            }
        }
        self.base_sections = banners
            .iter()
            .enumerate()
            .map(|(k, b)| {
                let economy = b.kind == section::BannerKind::Economy;
                SectionRow {
                    ordinal: k,
                    name: if economy {
                        ECONOMY_DISPLAY_NAME.to_string()
                    } else {
                        b.name.clone()
                    },
                    economy,
                    rules: if economy { econ_blocks } else { tally[k] },
                    can_move_up: !economy && self.doc.section_can_move(k, true),
                    can_move_down: !economy && self.doc.section_can_move(k, false),
                    can_remove: self.doc.section_can_remove(k),
                }
            })
            .collect();
        self.layout = layout;

        // WR-02 (presentation only): an enabled catch-all that swallows the enabled rules after it.
        let mut shadowing = HashSet::new();
        let mut later_enabled = false;
        for row in rows.iter().rev() {
            if row.enabled && row.conditions.is_empty() && !row.has_continue && later_enabled {
                shadowing.insert(row.id);
            }
            later_enabled |= row.enabled;
        }
        self.shadowing = shadowing;
        self.economy_rows = economy_rows;

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
        self.rules_edited = self.doc.serialize() != self.applied_text;
        self.dirty = self.pending.is_some() || self.rules_edited;
        let mut sections = self.base_sections.clone();
        match self.pending {
            Some(Pending::Remove) => sections.retain(|s| !s.economy),
            Some(Pending::Add { blocks }) => sections.push(SectionRow {
                ordinal: self.base_sections.len(),
                name: ECONOMY_DISPLAY_NAME.to_string(),
                economy: true,
                rules: blocks,
                can_move_up: false,
                can_move_down: false,
                can_remove: false,
            }),
            // A pending refresh: the header counts the re-priced blocks.
            None => {
                if let Some(p) = &self.preview {
                    for s in sections.iter_mut().filter(|s| s.economy) {
                        s.rules = p.economy_rows.len();
                    }
                }
            }
        }
        self.sections = sections;
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
        // The staged-economy preview judges its tags against the same set.
        if let (Some(p), Some(set)) = (&mut self.preview, &self.styles) {
            p.set_styles(Ok(set.clone()));
        }
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

    /// Section dividers in file order, with the staged economy op applied.
    pub fn sections(&self) -> &[SectionRow] {
        &self.sections
    }

    /// The generated economy blocks as read-only rows (empty while a remove is staged): the
    /// preview's while a staged add or a pending refresh has one, else the file's.
    pub fn economy_rows(&self) -> &[RuleRow] {
        match (&self.pending, &self.preview) {
            (Some(Pending::Remove), _) => &[],
            (_, Some(p)) => &p.economy_rows,
            (Some(Pending::Add { .. }), None) => &[],
            (None, None) => &self.economy_rows,
        }
    }

    /// Enabled, condition-less, non-Continue rules that have an enabled user rule after them.
    pub fn shadowing(&self) -> &HashSet<BlockId> {
        &self.shadowing
    }

    /// The displayed list in file order. `collapsed` says which sections are folded; during a
    /// search nothing is folded: matching rules show inside their section, whose header (with
    /// `m of n`) is listed only when it has a match. Section names are not searched.
    pub fn entries(
        &mut self,
        query: &str,
        collapsed: impl Fn(&SectionRow) -> bool,
    ) -> Vec<ListEntry> {
        let q = query.trim().to_lowercase();
        let searching = !q.is_empty();
        let hits: Vec<usize> = if searching {
            self.matches(&q)
        } else {
            Vec::new()
        };
        let hit = |i: usize| !searching || hits.binary_search(&i).is_ok();
        let econ_shown = self.pending != Some(Pending::Remove);
        let econ_pos = self.sections.iter().position(|s| s.economy);
        let pos_of = |ordinal: usize| self.sections.iter().position(|s| s.ordinal == ordinal);
        let folded: Vec<bool> = self.sections.iter().map(&collapsed).collect();
        let econ_hit = |r: &RuleRow| !searching || r.haystack.contains(&q);
        // Matches per section (rules for a divider, blocks for the economy section).
        let mut counts = vec![0usize; self.sections.len()];
        if searching {
            for (i, row) in self.rows.iter().enumerate() {
                if hit(i)
                    && let Some(p) = row.section.and_then(pos_of)
                {
                    counts[p] += 1;
                }
            }
            if econ_shown && let Some(p) = econ_pos {
                counts[p] = self.economy_rows().iter().filter(|r| econ_hit(r)).count();
            }
        }
        let header = |p: usize| ListEntry::Header {
            section: p,
            collapsed: !searching && folded[p],
            matches: searching.then_some(counts[p]),
        };
        let mut out = Vec::new();
        for item in &self.layout {
            match *item {
                Item::Unit(i) => {
                    let current = self.rows[i].section.and_then(pos_of);
                    let hidden = current.is_some_and(|p| !searching && folded[p]);
                    if !hidden && hit(i) {
                        out.push(ListEntry::Rule(i));
                    }
                }
                Item::Banner(ord) => {
                    if let Some(p) = pos_of(ord)
                        && (!searching || counts[p] > 0)
                    {
                        out.push(header(p));
                    }
                }
                Item::EconomyBanner => {
                    let (true, Some(p)) = (econ_shown, econ_pos) else {
                        continue;
                    };
                    if !searching || counts[p] > 0 {
                        out.push(header(p));
                    }
                    if searching || !folded[p] {
                        for (k, r) in self.economy_rows().iter().enumerate() {
                            if econ_hit(r) {
                                out.push(ListEntry::EconomyRule(k));
                            }
                        }
                    }
                }
            }
        }
        // A staged economy add has no banner in the file yet: its header closes the list.
        if matches!(self.pending, Some(Pending::Add { .. }))
            && let Some(p) = econ_pos
        {
            if !searching || counts[p] > 0 {
                out.push(header(p));
            }
            if searching || !folded[p] {
                for (k, r) in self.economy_rows().iter().enumerate() {
                    if econ_hit(r) {
                        out.push(ListEntry::EconomyRule(k));
                    }
                }
            }
        }
        out
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
    /// Returns `false` when `body` cannot be previewed (nothing is staged then), so the caller can
    /// say so instead of showing an empty section (WR-03).
    pub fn stage_add_economy(&mut self, body: &str) -> bool {
        match (&self.pending, &self.base_economy) {
            (Some(Pending::Remove), _) => self.clear_pending(),
            (None, EconomyView::Absent) => {
                let Some(preview) = self.build_preview(body) else {
                    return false;
                };
                let blocks = preview.economy_rows.len();
                self.preview = Some(preview);
                self.pending = Some(Pending::Add { blocks });
            }
            _ => return true,
        }
        self.refresh_projection();
        true
    }

    /// Replace the staged add's preview with a re-priced `body` (pending styles or tiers
    /// changed). A no-op unless an economy add is staged.
    /// Returns `false` when `body` cannot be previewed; the previous preview then stays (WR-03).
    pub fn set_staged_economy_body(&mut self, body: &str) -> bool {
        if !matches!(self.pending, Some(Pending::Add { .. })) {
            return true;
        }
        let Some(preview) = self.build_preview(body) else {
            return false;
        };
        let blocks = preview.economy_rows.len();
        self.preview = Some(preview);
        self.pending = Some(Pending::Add { blocks });
        self.refresh_projection();
        true
    }

    /// With the economy section in the file and nothing staged: `Some(body)` shows the section
    /// re-priced with the pending styles and tiers (what Apply will write) instead of the file's
    /// blocks; `None` goes back to the file's blocks. A no-op otherwise.
    /// Returns `false` when `body` cannot be previewed; the file's blocks then show.
    pub fn set_economy_refresh(&mut self, body: Option<&str>) -> bool {
        if self.pending.is_some() || !matches!(self.base_economy, EconomyView::Present { .. }) {
            return true;
        }
        let preview = match body {
            Some(b) => match self.build_preview(b) {
                Some(p) => Some(p),
                None => {
                    self.preview = None;
                    self.refresh_projection();
                    return false;
                }
            },
            None => None,
        };
        self.preview = preview;
        self.refresh_projection();
        true
    }

    /// Whether the economy rows come from a preview (a staged add or a pending refresh).
    pub fn has_economy_preview(&self) -> bool {
        self.preview.is_some()
    }

    fn build_preview(&self, body: &str) -> Option<Box<WorkingFilter>> {
        let text = section::add("", body).ok()?;
        let mut p = Self::from_text_based(&text, PREVIEW_ID_BASE);
        if let Some(s) = &self.styles {
            p.set_styles(Ok(s.clone()));
        }
        Some(Box::new(p))
    }

    fn clear_pending(&mut self) {
        self.pending = None;
        self.preview = None;
    }

    /// The filter the economy rows' bodies are read from: the preview while a staged add or a
    /// pending refresh has one, else this filter.
    pub fn economy_source(&self) -> &WorkingFilter {
        match (&self.pending, &self.preview) {
            (Some(Pending::Remove), _) | (_, None) => self,
            (_, Some(p)) => p,
        }
    }

    /// Stage removing the economy section (only from Present); the opposite staged op cancels.
    pub fn stage_remove_economy(&mut self) {
        match (self.pending, &self.base_economy) {
            (Some(Pending::Add { .. }), _) => self.clear_pending(),
            (None, EconomyView::Present { .. }) => self.pending = Some(Pending::Remove),
            _ => return,
        }
        self.refresh_projection();
    }

    /// Whether an economy add/remove is staged.
    pub fn economy_staged(&self) -> bool {
        self.pending.is_some()
    }

    /// Drop a staged economy add/remove without touching other edits (ECONOMY tab Revert).
    pub fn unstage_economy(&mut self) {
        if self.pending.is_some() {
            self.clear_pending();
            self.refresh_projection();
        }
    }

    /// Auto Detect candidates: untagged rules outside the economy section (disabled ones
    /// included, a `# ff:group` once), skipping `can't enable` and grammar-error rules.
    pub fn detect_inputs(&self) -> Vec<(BlockId, Vec<BodyEntry>, BlockKeyword)> {
        self.rows()
            .iter()
            .filter(|r| self.detect_eligible(r))
            .map(|r| (r.id, self.body_view(r.id), r.keyword))
            .collect()
    }

    fn detect_eligible(&self, r: &RuleRow) -> bool {
        r.style == StyleState::None
            && !r.cannot_enable
            && r.errors.is_empty()
            && self.body_editable(r.id)
    }

    /// Auto Detect: tag a batch of rules with `# ff:style <id> <name>` in memory. Every member
    /// of a group gets the tag so the run stays one rule. Nothing is rewritten here (Apply
    /// rewrites enabled tagged rules from their style), one rebuild at the end. Entries with an
    /// unknown style or an id that is no longer an eligible untagged rule are skipped. Returns
    /// the number of rules (not blocks) tagged.
    pub fn tag_styles(&mut self, tags: &[(BlockId, String)]) -> usize {
        if tags.is_empty() {
            return 0;
        }
        let mut done = 0;
        for (id, name) in tags {
            let Some(tag) = self.styles.as_ref().and_then(|s| s.tag_for(name)) else {
                continue;
            };
            let eligible = self.row(*id).is_some_and(|r| self.detect_eligible(r));
            if !eligible {
                continue;
            }
            let snapshot = self.doc.clone();
            let ok = self
                .groups
                .members(*id)
                .into_iter()
                .all(|m| self.doc.set_meta(m, "style", &tag).is_ok());
            if ok {
                done += 1;
            } else {
                self.doc = snapshot;
            }
        }
        if done > 0 {
            self.rebuild();
        }
        done
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
        // Economy blocks are not rows: no rule action may reach them (T-32-13).
        if let Some(id) = action_target(&action)
            && self.row(id).is_none()
            && self.doc.block(id).is_some()
        {
            return refused(ECONOMY_LOCKED_STATUS.to_string());
        }
        match action {
            RowAction::AddSection
            | RowAction::AddSectionAbove(_)
            | RowAction::RenameSection { .. }
            | RowAction::RemoveSection(_)
            | RowAction::MoveSection(..) => return self.section_action(action),
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

    /// A section edit. Anything aimed at the economy section is refused.
    fn section_action(&mut self, action: RowAction) -> ActionOutcome {
        let created = |ordinal: usize, status: String| ActionOutcome {
            changed: true,
            status: Some(status),
            section: Some(ordinal),
            ..ActionOutcome::default()
        };
        let plain = |r: Result<(), crate::filter::EditError>, status: Option<&str>| match r {
            Ok(()) => ActionOutcome {
                changed: true,
                status: status.map(str::to_string),
                ..ActionOutcome::default()
            },
            Err(e) => refused(section_refusal(&e)),
        };
        // The synthetic staged economy row has an ordinal past the document's banners.
        let locked = |wf: &Self, ordinal: usize| {
            wf.sections
                .iter()
                .any(|s| s.ordinal == ordinal && s.economy)
        };
        match action {
            RowAction::AddSection => match self.doc.insert_section_at_top(DEFAULT_SECTION_NAME) {
                Ok(o) => created(o, SECTION_ADDED_TOP_STATUS.to_string()),
                Err(e) => refused(section_refusal(&e)),
            },
            RowAction::AddSectionAbove(id) => {
                let index = self.row(id).map_or(0, |r| r.index);
                match self.doc.insert_section_above(id, DEFAULT_SECTION_NAME) {
                    Ok(o) => created(
                        o,
                        format!("Section added above rule {index}. Press Apply to save it."),
                    ),
                    Err(e) => refused(section_refusal(&e)),
                }
            }
            RowAction::RenameSection { ordinal, .. } | RowAction::RemoveSection(ordinal)
                if locked(self, ordinal) =>
            {
                refused(ECONOMY_LOCKED_STATUS.to_string())
            }
            RowAction::MoveSection(ordinal, _) if locked(self, ordinal) => {
                refused(ECONOMY_LOCKED_STATUS.to_string())
            }
            // Committing a name box unchanged (the new section's default name, or a click away)
            // is not an edit: it must not clear the event message the add just posted.
            RowAction::RenameSection { ordinal, name }
                if self.sections.iter().any(|s| {
                    s.ordinal == ordinal && s.name == crate::filter::section::sanitize_name(&name)
                }) =>
            {
                ActionOutcome::default()
            }
            RowAction::RenameSection { ordinal, name } => {
                plain(self.doc.rename_section(ordinal, &name), None)
            }
            RowAction::RemoveSection(ordinal) => plain(
                self.doc.remove_section(ordinal),
                Some(SECTION_REMOVED_STATUS),
            ),
            RowAction::MoveSection(ordinal, dir) => {
                let up = dir == Dir::Up;
                if !self.doc.section_can_move(ordinal, up) {
                    return refused(SECTION_CANT_MOVE_STATUS.to_string());
                }
                plain(self.doc.move_section(ordinal, up), None)
            }
            _ => refused(NOT_EDITABLE_STATUS.to_string()),
        }
    }

    /// Move a rule one step: swap with its neighbour rule (either may be a group, which moves as
    /// one unit) or hop the divider in that direction. Never crosses the economy section.
    fn move_rule(&mut self, id: BlockId, dir: Dir) -> ActionOutcome {
        let up = dir == Dir::Up;
        let a = self.groups.members(id);
        let edge = if up { a[0] } else { a[a.len() - 1] };
        let nb = match self.doc.rule_step(edge, up) {
            Err(e) => return refused(e.to_string()),
            Ok(RuleStep::Blocked) => {
                return refused("There is no rule to swap with in that direction.".into());
            }
            // The divider hops the rule; the rule order is unchanged, so no Continue flag.
            Ok(RuleStep::CrossBanner(k)) => {
                return done_or(self.doc.move_section(k, !up), Some(id));
            }
            Ok(RuleStep::Swap(nb)) => self.groups.rule_of(nb),
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
                        section: None,
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
            RowAction::Move(..)
            | RowAction::SetDefenceSelection { .. }
            | RowAction::AddSection
            | RowAction::AddSectionAbove(_)
            | RowAction::RenameSection { .. }
            | RowAction::RemoveSection(_)
            | RowAction::MoveSection(..) => Err(NOT_EDITABLE_STATUS.to_string()),
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
                section: None,
            },
            Err(msg) => ActionOutcome {
                changed: false,
                status: Some(msg),
                focus: None,
                line: None,
                section: None,
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

    /// Rule edits only: like [`Self::is_dirty`] but a staged economy add/remove (owned by the
    /// ECONOMY tab) does not count.
    pub fn rules_edited(&self) -> bool {
        self.rules_edited
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
        self.clear_pending();
        self.revert_rules();
    }

    /// The RULES Revert: discard the rule edits and keep a staged economy add/remove, which the
    /// ECONOMY tab owns. The economy section is locked to rule edits, so the staged op stays valid
    /// against the re-parsed text.
    pub fn revert_rules(&mut self) {
        self.doc = parse_text(&self.applied_text);
        self.moved.clear();
        self.rebuild();
    }

    /// `applied_text` is now what is on disk: make it the baseline.
    pub fn on_applied(&mut self, applied_text: &str) {
        self.applied_text = applied_text.to_string();
        self.doc = parse_text(applied_text);
        self.clear_pending();
        self.moved.clear();
        self.rebuild();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::section::{BEGIN_LINE, RULE_LINE};

    fn econ_body(n: usize) -> String {
        tests_econ_body(n)
    }

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
    fn tagged_economy_rows_show_name_and_style() {
        let text = format!(
            "Show
  Class \"Rings\"

{RULE_LINE}
{BEGIN_LINE}
{RULE_LINE}
# ff:name S tier Currency
# ff:style builtin:S S
Show # Currency S
  Class == \"Stackable Currency\"
  BaseType == \"Divine Orb\"
  MinimapIcon 0 Pink Circle

# ff:name A tier Uniques
Show # Unique A
  Rarity == Unique
  BaseType == \"Infernal Sword\"
  SetBackgroundColor 175 96 37 240
"
        );
        let mut wf = WorkingFilter::from_text(&text);
        wf.set_styles(Ok(StyleSet::committed_for_tests()));
        let rows = wf.economy_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name.as_deref(), Some("S tier Currency"));
        assert!(rows[0].label.contains("S tier Currency"));
        assert_eq!(rows[0].style, StyleState::Known("S".into()));
        assert!(rows[0].chip_icon.is_some());
        assert_eq!(rows[1].name.as_deref(), Some("A tier Uniques"));
        assert_eq!(rows[1].style, StyleState::None);
        // Search finds an economy row by its name; style users ignore economy blocks.
        let hits = wf.entries("tier currency", |_| false);
        assert!(hits.iter().any(|e| matches!(e, ListEntry::EconomyRule(_))));
        assert!(!wf.style_users().contains_key("S"));
    }

    /// An enabled rule styled with a preset shows the pending style's Show/Hide before Apply; a
    /// disabled one (never rewritten) and a Custom one keep the file's keyword.
    #[test]
    fn styled_rows_show_the_pending_style_visibility() {
        let text = "# ff:name On\n# ff:style builtin:S S\nShow\n  Class \"Rings\"\n\n\
                    # ff:name Off\n# ff:style builtin:S S\n# Show\n#   Class \"Rings\"\n\n\
                    # ff:name Custom\nShow\n  Class \"Belts\"\n";
        let mut wf = WorkingFilter::from_text(text);
        let mut styles = StyleSet::committed_for_tests();
        styles.get_mut("S").unwrap().action = StyleAction::Hide;
        wf.set_styles(Ok(styles));
        let rows = wf.rows();
        assert_eq!(rows[0].keyword, BlockKeyword::Hide);
        assert_eq!(rows[0].label, "1  Hide - On");
        assert_eq!(rows[1].keyword, BlockKeyword::Show);
        assert_eq!(rows[2].keyword, BlockKeyword::Show);
        // Display only: the file is untouched.
        assert!(!wf.is_dirty());
        assert!(
            wf.text()
                .starts_with("# ff:name On\n# ff:style builtin:S S\nShow\n")
        );
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

    fn test_styles() -> StyleSet {
        crate::loader::load_styles(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/seed-styles.json"
        ))
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
    fn a_body_that_cannot_be_previewed_stages_nothing() {
        let mut wf = WorkingFilter::from_text(THREE);
        // A body holding a section banner marker is refused by the section writer.
        assert!(!wf.stage_add_economy("# ff:section Oops\nShow\n"));
        assert!(!wf.economy_staged());
        assert_eq!(*wf.economy(), EconomyView::Absent);
        // A failed re-price keeps the previous preview.
        assert!(wf.stage_add_economy(&econ_body(2)));
        assert!(!wf.set_staged_economy_body("# ff:section Oops\nShow\n"));
        assert_eq!(*wf.economy(), EconomyView::Present { blocks: 2 });
    }

    #[test]
    fn unstage_economy_keeps_other_edits() {
        let mut wf = WorkingFilter::from_text(THREE);
        assert!(!wf.economy_staged());
        wf.stage_add_economy(&econ_body(5));
        assert!(wf.economy_staged());
        assert_eq!(*wf.economy(), EconomyView::Present { blocks: 5 });
        wf.unstage_economy();
        assert!(!wf.economy_staged());
        assert_eq!(*wf.economy(), EconomyView::Absent);
        assert!(!wf.is_dirty());

        wf.stage_add_economy(&econ_body(5));
        let id = wf.rows()[0].id;
        wf.apply_action(RowAction::Delete(id));
        wf.unstage_economy();
        assert!(wf.is_dirty(), "the rule edit survives");
        assert!(!wf.economy_staged());
    }

    #[test]
    fn a_staged_economy_op_is_not_a_rules_edit() {
        let mut wf = WorkingFilter::from_text(THREE);
        wf.stage_add_economy(&econ_body(5));
        assert!(wf.is_dirty());
        assert!(!wf.rules_edited());
        let id = wf.rows()[0].id;
        wf.apply_action(RowAction::Delete(id));
        assert!(wf.rules_edited());
        wf.unstage_economy();
        assert!(wf.rules_edited(), "the rule edit survives");
    }

    #[test]
    fn revert_rules_keeps_a_staged_economy_op() {
        let mut wf = WorkingFilter::from_text(&with_economy());
        wf.stage_remove_economy();
        let id = wf.rows()[0].id;
        wf.apply_action(RowAction::Delete(id));
        assert!(wf.rules_edited());
        wf.revert_rules();
        assert_eq!(wf.text(), with_economy());
        assert!(!wf.rules_edited());
        assert!(wf.economy_staged(), "ECONOMY owns the staged op");
        assert_eq!(wf.economy_plan(), EconomyPlan::Remove);
        assert_eq!(*wf.economy(), EconomyView::Absent);
        assert!(wf.is_dirty());

        let mut wf = WorkingFilter::from_text(THREE);
        wf.stage_add_economy(&econ_body(5));
        wf.apply_action(RowAction::AddRule);
        wf.revert_rules();
        assert_eq!(wf.text(), THREE);
        assert_eq!(*wf.economy(), EconomyView::Present { blocks: 5 });
    }

    #[test]
    fn economy_staging_revert_and_applied() {
        let mut wf = WorkingFilter::from_text(THREE);
        let before = wf.text();
        wf.stage_add_economy(&econ_body(5));
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
        wf.stage_add_economy(&econ_body(1));
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
    fn rename_runs_before_a_structural_action_in_the_same_batch() {
        let r = crate::filter::section::RULE_LINE;
        let mut wf = WorkingFilter::from_text(&format!(
            "{r}
# ff:section Old
{r}

Show
"
        ));
        let batch = vec![
            RowAction::AddSection,
            RowAction::RenameSection {
                ordinal: 0,
                name: "New".into(),
            },
        ];
        for a in coalesce_edits(batch) {
            wf.apply_action(a);
        }
        let t = wf.text();
        assert!(
            t.contains(
                "# ff:section New
"
            ),
            "{t}"
        );
        assert!(
            t.contains(
                "# ff:section Section
"
            ),
            "{t}"
        );
        assert!(!t.contains("# ff:section Old"), "{t}");
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

    // -- sections, economy rows, entries (Phase 32) ----------------------------------------------

    fn banner(name: &str) -> String {
        format!("{RULE_LINE}\n# ff:section {name}\n{RULE_LINE}\n")
    }

    /// rule 1 | Leveling: rule 2, group (rules 3) | Endgame: rule 4 | economy (3 blocks)
    fn sectioned() -> String {
        format!(
            "Show\n  Rarity Unique\n\n{}\nShow\n  Rarity Normal\n\n{PART1}\n{PART2}\n{}\nShow\n  Rarity Magic\n\n{RULE_LINE}\n{BEGIN_LINE}\n{RULE_LINE}\nShow\n  Class \"Gems\"\n\nShow\n  Class \"Currency\"\n\nHide\n  Class \"Flasks\"\n",
            banner("Leveling"),
            banner("Endgame"),
        )
    }

    fn header(section: usize) -> ListEntry {
        ListEntry::Header {
            section,
            collapsed: false,
            matches: None,
        }
    }

    #[test]
    fn sections_rows_and_economy_rows() {
        let wf = WorkingFilter::from_text(&sectioned());
        let s = wf.sections();
        assert_eq!(s.len(), 3);
        assert_eq!(
            (s[0].name.as_str(), s[0].rules, s[0].economy),
            ("Leveling", 2, false)
        );
        assert_eq!(
            (s[1].name.as_str(), s[1].rules, s[1].economy),
            ("Endgame", 1, false)
        );
        assert_eq!((s[2].rules, s[2].economy), (3, true));
        // Display name only: the banner in the file keeps its own text.
        assert_eq!(s[2].name, "FF Economy Section");
        assert!(wf.text().contains(crate::filter::section::BEGIN_LINE));
        assert!(!wf.text().contains("FF Economy Section"));
        assert!(!s[2].can_move_up && !s[2].can_move_down);
        let secs: Vec<Option<usize>> = wf.rows().iter().map(|r| r.section).collect();
        assert_eq!(secs, [None, Some(0), Some(0), Some(1)]);
        let idx: Vec<usize> = wf.rows().iter().map(|r| r.index).collect();
        assert_eq!(idx, [1, 2, 3, 4]);
        assert_eq!(wf.economy_rows().len(), 3);
        assert!(
            wf.economy_rows()
                .iter()
                .all(|r| r.index == 0 && !r.can_move_up)
        );
    }

    #[test]
    fn entries_interleave_headers_and_rules() {
        let mut wf = WorkingFilter::from_text(&sectioned());
        use ListEntry::*;
        assert_eq!(
            wf.entries("", |_| false),
            [
                Rule(0),
                header(0),
                Rule(1),
                Rule(2),
                header(1),
                Rule(3),
                header(2),
                EconomyRule(0),
                EconomyRule(1),
                EconomyRule(2)
            ]
        );
        let folded = |s: &SectionRow| s.name == "Leveling" || s.economy;
        let collapsed = |section| Header {
            section,
            collapsed: true,
            matches: None,
        };
        assert_eq!(
            wf.entries("", folded),
            [Rule(0), collapsed(0), header(1), Rule(3), collapsed(2)]
        );
    }

    #[test]
    fn search_opens_collapsed_sections_and_counts_matches() {
        let mut wf = WorkingFilter::from_text(&sectioned());
        use ListEntry::*;
        let folded = |s: &SectionRow| s.name == "Leveling" || s.economy;
        // Only rule 2 (index 1) has Normal... "Rarity" is not searched; use the name instead.
        let t = sectioned().replace(
            "Show\n  Rarity Normal",
            "# ff:name Needle\nShow\n  Rarity Normal",
        );
        let mut wf2 = WorkingFilter::from_text(&t);
        assert_eq!(
            wf2.entries("needle", folded),
            [
                Header {
                    section: 0,
                    collapsed: false,
                    matches: Some(1)
                },
                Rule(1)
            ]
        );
        // Section names are not searched.
        assert!(wf.entries("leveling", folded).is_empty());
        // An economy row is found, under its header.
        assert_eq!(
            wf.entries("currency", folded),
            [
                Header {
                    section: 2,
                    collapsed: false,
                    matches: Some(1)
                },
                EconomyRule(1)
            ]
        );
    }

    #[test]
    fn staged_economy_ops_project_into_sections() {
        let mut wf = WorkingFilter::from_text(THREE);
        assert!(wf.sections().is_empty());
        wf.stage_add_economy(&econ_body(7));
        assert_eq!(wf.sections().len(), 1);
        assert!(wf.sections()[0].economy);
        assert_eq!(wf.sections()[0].rules, 7);
        assert_eq!(wf.economy_rows().len(), 7);
        assert_eq!(wf.economy_rows()[2].name.as_deref(), Some("Block 2"));
        // Collapsed: just the header; expanded: the header, then the preview rows.
        let folded = wf.entries("", |_| true);
        assert!(matches!(
            folded.last(),
            Some(ListEntry::Header {
                collapsed: true,
                ..
            })
        ));
        assert_eq!(folded.len(), 3 + 1);
        let open = wf.entries("", |_| false);
        assert_eq!(open.len(), 3 + 1 + 7);
        assert_eq!(open[open.len() - 7], ListEntry::EconomyRule(0));
        // Search reaches the preview rows.
        let hit = wf.entries("block 3", |_| true);
        assert!(hit.contains(&ListEntry::EconomyRule(3)));
        // Preview ids never collide with the document's.
        let doc_ids: Vec<_> = wf.rows().iter().map(|r| r.id).collect();
        assert!(wf.economy_rows().iter().all(|r| !doc_ids.contains(&r.id)));
        assert!(
            wf.economy_source()
                .doc
                .block(wf.economy_rows()[0].id)
                .is_some()
        );
        assert!(wf.doc.block(wf.economy_rows()[0].id).is_none());
        // The body view reads from the preview (ids are not in the working document, so the model
        // refuses any action aimed at them).
        assert!(
            !wf.economy_source()
                .body_view(wf.economy_rows()[0].id)
                .is_empty()
        );
        // Re-pricing replaces the preview; Remove cancels the Add.
        wf.set_staged_economy_body(&econ_body(2));
        assert_eq!(wf.sections()[0].rules, 2);
        assert_eq!(wf.economy_rows().len(), 2);
        wf.stage_remove_economy();
        assert!(wf.economy_rows().is_empty());
        assert!(!wf.economy_staged());
        wf.stage_add_economy(&econ_body(3));
        wf.unstage_economy();
        assert!(wf.economy_rows().is_empty());
        wf.set_staged_economy_body(&econ_body(4));
        assert!(wf.economy_rows().is_empty());
        let mut wf = WorkingFilter::from_text(&sectioned());
        wf.stage_remove_economy();
        assert!(wf.sections().iter().all(|s| !s.economy));
        assert!(wf.economy_rows().is_empty());
        assert!(
            !wf.entries("", |_| false)
                .iter()
                .any(|e| matches!(e, ListEntry::EconomyRule(_)))
        );
    }

    #[test]
    fn section_move_flags_follow_the_document() {
        let wf = WorkingFilter::from_text(&sectioned());
        for s in wf.sections().iter().filter(|s| !s.economy) {
            assert_eq!(s.can_move_up, wf.doc.section_can_move(s.ordinal, true));
            assert_eq!(s.can_move_down, wf.doc.section_can_move(s.ordinal, false));
        }
    }

    /// Row flags equal what `Document::rule_step` says, for every rule of `text`.
    fn assert_moves_match_rule_step(text: &str) {
        let wf = WorkingFilter::from_text(text);
        for r in wf.rows() {
            let m = wf.groups.members(r.id);
            let up = wf.doc.rule_step(m[0], true).unwrap() != crate::filter::RuleStep::Blocked;
            let down = wf.doc.rule_step(m[m.len() - 1], false).unwrap()
                != crate::filter::RuleStep::Blocked;
            assert_eq!((r.can_move_up, r.can_move_down), (up, down), "{}", r.label);
        }
    }

    #[test]
    fn rule_move_flags_equal_rule_step() {
        assert_moves_match_rule_step(&sectioned());
        assert_moves_match_rule_step(THREE);
        assert_moves_match_rule_step(&with_economy());
        let adjacent = format!(
            "{}{}Show\n  Rarity Normal\n\n{PART1}\n{PART2}\n{}Show\n  Rarity Magic\n",
            banner("A"),
            banner("B"),
            banner("C")
        );
        assert_moves_match_rule_step(&adjacent);
        let edge = format!("Show\n  Rarity Normal\n\n{}{PART1}\n{PART2}", banner("A"));
        assert_moves_match_rule_step(&edge);
    }

    #[test]
    fn shadowing_marks_catch_alls_with_enabled_rules_after() {
        let t = "Show\n\nShow\n  Class \"Rings\"\n\nShow\n";
        let wf = WorkingFilter::from_text(t);
        let ids: Vec<BlockId> = wf.rows().iter().map(|r| r.id).collect();
        assert!(wf.shadowing().contains(&ids[0]));
        assert!(!wf.shadowing().contains(&ids[1]));
        assert!(!wf.shadowing().contains(&ids[2]), "last user rule");
        let t = "Show\n  Continue\n\nShow\n  Class \"Rings\"\n";
        let wf = WorkingFilter::from_text(t);
        assert!(wf.shadowing().is_empty());
        let t = "# Show\n\nShow\n  Class \"Rings\"\n";
        let wf = WorkingFilter::from_text(t);
        assert!(wf.shadowing().is_empty());
        let t = format!("Show\n\n{RULE_LINE}\n{BEGIN_LINE}\n{RULE_LINE}\nShow\n");
        let wf = WorkingFilter::from_text(&t);
        assert!(wf.shadowing().is_empty(), "economy rows never shadow");
    }

    // -- section actions -------------------------------------------------------------------------

    #[test]
    fn add_section_at_the_top_is_a_revertible_edit() {
        let mut wf = WorkingFilter::from_text(THREE);
        let out = wf.apply_action(RowAction::AddSection);
        assert!(out.changed);
        assert_eq!(out.section, Some(0));
        assert_eq!(
            out.status.as_deref(),
            Some("Section added at the top. Press Apply to save it.")
        );
        assert!(wf.is_dirty());
        assert_eq!(wf.sections()[0].name, "Section");
        wf.revert();
        assert_eq!(wf.text(), THREE);
        assert!(wf.sections().is_empty());
    }

    #[test]
    fn add_section_above_a_rule_keeps_its_id() {
        let mut wf = WorkingFilter::from_text(THREE);
        let id = wf.rows()[2].id;
        let out = wf.apply_action(RowAction::AddSectionAbove(id));
        assert_eq!(
            out.status.as_deref(),
            Some("Section added above rule 3. Press Apply to save it.")
        );
        assert_eq!(out.section, Some(0));
        assert_eq!(wf.rows()[2].id, id);
        assert_eq!(wf.rows()[2].section, Some(0));
        assert_eq!(wf.rows()[1].section, None);
    }

    #[test]
    fn rename_sanitises_and_remove_keeps_the_rules() {
        let mut wf = WorkingFilter::from_text(&sectioned());
        let ids: Vec<BlockId> = wf.rows().iter().map(|r| r.id).collect();
        let out = wf.apply_action(RowAction::RenameSection {
            ordinal: 0,
            name: " New \n name ".into(),
        });
        assert!(out.changed);
        assert_eq!(wf.sections()[0].name, "New name");
        assert!(wf.is_dirty());
        let out = wf.apply_action(RowAction::RemoveSection(0));
        assert_eq!(
            out.status.as_deref(),
            Some("Section removed. Its rules joined the section above.")
        );
        assert_eq!(wf.rows().iter().map(|r| r.id).collect::<Vec<_>>(), ids);
        assert_eq!(wf.sections().len(), 2);
        assert_eq!(wf.rows()[1].section, None);
    }

    #[test]
    fn move_section_changes_only_the_banner_position() {
        let mut wf = WorkingFilter::from_text(&sectioned());
        let ids: Vec<BlockId> = wf.rows().iter().map(|r| r.id).collect();
        assert!(wf.sections()[0].can_move_up);
        let out = wf.apply_action(RowAction::MoveSection(0, Dir::Up));
        assert!(out.changed);
        assert_eq!(wf.rows().iter().map(|r| r.id).collect::<Vec<_>>(), ids);
        assert_eq!(
            wf.rows()[0].section,
            Some(0),
            "the first rule joined Leveling"
        );
        let before = wf.text();
        let out = wf.apply_action(RowAction::MoveSection(0, Dir::Up));
        assert!(!out.changed && out.status.is_some());
        assert_eq!(wf.text(), before);
    }

    #[test]
    fn economy_targets_are_refused_and_nothing_changes() {
        let mut wf = WorkingFilter::from_text(&sectioned());
        let before = wf.text();
        let econ = wf.sections().iter().position(|s| s.economy).unwrap();
        let ord = wf.sections()[econ].ordinal;
        let eid = wf.economy_rows()[0].id;
        let actions = [
            RowAction::RenameSection {
                ordinal: ord,
                name: "x".into(),
            },
            RowAction::RemoveSection(ord),
            RowAction::MoveSection(ord, Dir::Up),
            RowAction::MoveSection(ord, Dir::Down),
            RowAction::AddSectionAbove(eid),
            RowAction::SetEnabled(eid, false),
            RowAction::Delete(eid),
            RowAction::Duplicate(eid),
            RowAction::Move(eid, Dir::Up),
            RowAction::Move(eid, Dir::Down),
            RowAction::SetContinue(eid, true),
            RowAction::EditLine {
                id: eid,
                line: 0,
                expect: String::new(),
                patch: LinePatch::Whole("Show".into()),
            },
        ];
        for a in actions {
            let out = wf.apply_action(a.clone());
            assert!(!out.changed, "{a:?}");
            assert_eq!(out.status.as_deref(), Some(ECONOMY_LOCKED_STATUS), "{a:?}");
            assert_eq!(wf.text(), before, "{a:?}");
        }
    }

    #[test]
    fn rules_hop_dividers_without_reordering() {
        let mut wf = WorkingFilter::from_text(&sectioned());
        let ids: Vec<BlockId> = wf.rows().iter().map(|r| r.id).collect();
        // Rule 4 is the only rule of Endgame; down is the economy banner (blocked).
        let last = ids[3];
        let out = wf.apply_action(RowAction::Move(last, Dir::Down));
        assert!(!out.changed);
        // The group (rule 3) is the last of Leveling; moving down hops the Endgame divider.
        let grp = ids[2];
        assert_eq!(wf.rows()[2].section, Some(0));
        let out = wf.apply_action(RowAction::Move(grp, Dir::Down));
        assert!(out.changed, "{:?}", out.status);
        assert_eq!(wf.rows().iter().map(|r| r.id).collect::<Vec<_>>(), ids);
        assert_eq!(wf.rows()[2].section, Some(1));
        assert_eq!(wf.rows()[2].group_size, 2, "the run moved as one");
        assert!(!wf.rows()[2].moved_across_continue);
        // And back up.
        wf.apply_action(RowAction::Move(grp, Dir::Up));
        assert_eq!(wf.rows()[2].section, Some(0));
        assert_eq!(wf.text(), sectioned());
        // The rule right below the economy-following divider cannot hop up into the economy.
        let mut wf = WorkingFilter::from_text(&with_economy());
        let after = wf.rows()[1].id;
        assert!(!wf.apply_action(RowAction::Move(after, Dir::Up)).changed);
    }

    // -- Auto Detect batch model (Phase 44) --------------------------------------------------

    const DET_GROUP: &str = "# ff:group abc 1/3\nShow\n  Rarity Rare\n  BaseArmour > 0\n  BaseEvasion 0\n  BaseEnergyShield 0\n  SetFontSize 30\n\n# ff:group abc 2/3\nShow\n  Rarity Rare\n  BaseArmour 0\n  BaseEvasion > 0\n  BaseEnergyShield 0\n  SetFontSize 30\n\n# ff:group abc 3/3\nShow\n  Rarity Rare\n  BaseArmour 0\n  BaseEvasion 0\n  BaseEnergyShield > 0\n  SetFontSize 30\n";

    fn detect_fixture() -> String {
        use crate::filter::section::{BEGIN_LINE, RULE_LINE};
        format!(
            "Show\n  Class \"Rings\"\n  SetFontSize 20\n\n#Hide\n#  Class \"Belts\"\n#  SetFontSize 20\n\n# ff:style builtin:E E\nShow\n  Class \"Amulets\"\n\n#Show\n#  this is prose\n\n{DET_GROUP}\n{RULE_LINE}\n{BEGIN_LINE}\n{RULE_LINE}\nShow\n  Class \"Gems\"\n"
        )
    }

    #[test]
    fn detect_inputs_lists_untagged_rules_group_once() {
        let wf = styled_wf(&detect_fixture());
        let ids: Vec<BlockId> = wf.detect_inputs().iter().map(|t| t.0).collect();
        let rows = wf.rows();
        let want: Vec<BlockId> = rows
            .iter()
            .filter(|r| r.style == StyleState::None && !r.cannot_enable && r.errors.is_empty())
            .map(|r| r.id)
            .collect();
        assert_eq!(ids, want);
        // rings, disabled belts, the group leader; not the tagged, can't-enable or economy ones
        assert_eq!(ids.len(), 3, "{rows:?}");
        assert!(rows.iter().any(|r| r.cannot_enable));
    }

    #[test]
    fn detect_inputs_skips_grammar_errors() {
        let wf = styled_wf("Show\n  Rarity Bogus\n\nShow\n  Class \"Rings\"\n");
        let n = wf.rows().iter().filter(|r| !r.errors.is_empty()).count();
        assert_eq!(n, 1);
        assert_eq!(wf.detect_inputs().len(), 1);
    }

    #[test]
    fn tag_styles_tags_every_group_member_and_keeps_one_rule() {
        let mut wf = styled_wf(&detect_fixture());
        let id = wf
            .rows()
            .iter()
            .find(|r| r.group_size == 3)
            .map(|r| r.id)
            .unwrap();
        assert_eq!(wf.tag_styles(&[(id, "S".into())]), 1);
        let t = wf.text();
        assert_eq!(t.matches("# ff:style builtin:S S").count(), 3, "{t}");
        let r = wf.rows().iter().find(|r| r.group_size == 3).unwrap();
        assert!(!r.group_broken);
        assert_eq!(r.style, StyleState::Known("S".into()));
        assert!(wf.rules_edited());
        wf.revert_rules();
        assert!(!wf.text().contains("builtin:S"));
    }

    #[test]
    fn tag_styles_disabled_rule_keeps_action_bytes() {
        let mut wf =
            styled_wf("#Hide\n#  Class \"Rings\"\n#  SetFontSize 20\n\nShow\n  SetFontSize 20\n");
        let id = wf.rows()[0].id;
        let en = wf.rows()[1].id;
        assert_eq!(wf.tag_styles(&[(id, "S".into()), (en, "S".into())]), 2);
        let t = wf.text();
        assert!(
            t.contains("#  SetFontSize 20") && t.contains("\n  SetFontSize 20\n"),
            "{t}"
        );
        assert!(!t.contains("SetFontSize 37"), "{t}");
        assert!(!wf.rows()[0].enabled);
    }

    #[test]
    fn tag_styles_skips_unknown_and_already_tagged() {
        let mut wf = styled_wf(
            "# ff:style builtin:E E\nShow\n\nShow\n  Class \"Rings\"\n\nShow\n  Class \"Belts\"\n",
        );
        let (tagged, a, b) = (wf.rows()[0].id, wf.rows()[1].id, wf.rows()[2].id);
        let n = wf.tag_styles(&[(tagged, "S".into()), (a, "Nope".into()), (b, "S".into())]);
        assert_eq!(n, 1);
        let t = wf.text();
        assert_eq!(t.matches("ff:style").count(), 2, "{t}");
        assert!(
            t.contains("builtin:E E") && !t.contains("builtin:S S\nShow\n\n"),
            "{t}"
        );
        assert_eq!(wf.tag_styles(&[]), 0);
    }
}
