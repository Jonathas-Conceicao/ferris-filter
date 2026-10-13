//! Pure working model of the rule list: no egui, no file IO.
//!
//! [`WorkingFilter`] owns the edited [`Document`], the last-applied text (the Revert target) and a
//! cached `Vec<RuleRow>`. Rows are rebuilt once per mutation, never per frame, so a 500-block
//! filter stays responsive. Every edit goes through the `filter::Document` structural ops; this
//! module only translates [`RowAction`]s into them and projects the result for the view.
//!
//! Naming rule (CONTEXT, Rule naming): a row's name comes only from the `#ff:name` setting. The
//! foreign header comment text (`$type->currency %D4 !slug`) is never a name, never part of a
//! label and never searched.

use std::collections::HashSet;

use crate::dynamic::{BlockingError, blocking_errors};
use crate::filter::keywords::Kind;
use crate::filter::section::{self, SectionSpan};
use crate::filter::{
    Block, BlockId, BlockKeyword, Document, FilterDiagnostic, Severity, parse as parse_filter,
};

/// Refusal copy for enabling a commented-out block that is not valid grammar (UI-SPEC).
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
    AddRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionOutcome {
    pub changed: bool,
    pub status: Option<String>,
    pub focus: Option<BlockId>,
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
    /// The `#ff:style` tag (display only this phase).
    pub style_tag: Option<String>,
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
    moved: HashSet<BlockId>,
    generation: u64,
    dirty: bool,
    search_cache: Option<(String, u64, Vec<usize>)>,
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

fn extract(doc: &Document, b: &Block) -> Extracted {
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
        if kw.kind != Kind::Condition {
            continue;
        }
        let Some(text) = doc.lines().get(bl.line).map(|l| l.text()) else {
            continue;
        };
        // Disabled-block token ranges are relative to the text after the first `#`.
        let base = if b.is_enabled() {
            text
        } else {
            match text.find('#') {
                Some(i) => &text[i + 1..],
                None => text,
            }
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

fn derived_pieces(x: &Extracted) -> String {
    let mut pieces: Vec<String> = Vec::new();
    if let Some(first) = x.class.first() {
        pieces.push(if x.class.len() > 1 {
            format!("{first} +{} more", x.class.len() - 1)
        } else {
            first.clone()
        });
    }
    if !x.base_type.is_empty() {
        pieces.push(format!("BaseType \u{00D7}{}", x.base_type.len()));
    }
    if pieces.is_empty() {
        if let Some(first) = x.other_names.first() {
            pieces.push(if x.other_names.len() > 1 {
                format!("{first} +{} more", x.other_names.len() - 1)
            } else {
                first.clone()
            });
        }
    }
    if pieces.is_empty() {
        "matches everything".to_string()
    } else {
        pieces.join(" \u{00B7} ")
    }
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
            moved: HashSet::new(),
            generation: 0,
            dirty: false,
            search_cache: None,
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
            let side = u8::from(span.is_some_and(|s| b.header_line() > s.end));
            order.push((b, econ, side));
        }
        let econ_blocks = order.iter().filter(|(_, e, _)| *e).count();

        let mut rows = Vec::new();
        let mut index = 0usize;
        for (pos, (b, econ, side)) in order.iter().enumerate() {
            if *econ {
                continue;
            }
            index += 1;
            let id = b.id();
            let x = extract(&self.doc, b);
            let name = self.doc.meta_get(id, "name").map(str::to_string);
            let style_tag = self.doc.meta_get(id, "style").map(str::to_string);
            let kw = keyword_word(b.keyword());
            let label = match &name {
                Some(n) => format!("{index}  {kw} - {n}"),
                None => format!("{index}  {kw} \u{00B7} {}", derived_pieces(&x)),
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
                style_tag,
                label,
                haystack: hay.to_lowercase(),
                errors,
                warnings,
                cannot_enable: !enabled && !b.can_enable(),
                can_move_up: movable && same_side_rule(pos.checked_sub(1)),
                can_move_down: movable && same_side_rule(Some(pos + 1)),
                moved_across_continue: self.moved.contains(&id),
                conditions: x.conditions,
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

    pub fn rows(&self) -> &[RuleRow] {
        &self.rows
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

    pub fn apply_action(&mut self, action: RowAction) -> ActionOutcome {
        let mut focus = None;
        let mut status = None;
        let result: Result<(), String> = match action {
            RowAction::SetEnabled(id, on) => {
                if on && self.row(id).is_some_and(|r| r.cannot_enable) {
                    return ActionOutcome {
                        changed: false,
                        status: Some(CANNOT_ENABLE_STATUS.to_string()),
                        focus: None,
                    };
                }
                self.doc.set_enabled(id, on).map_err(|e| e.to_string())
            }
            RowAction::AddRule => match self.doc.append_rule(vec!["Show".to_string()]) {
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
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            RowAction::Move(id, dir) => {
                let up = dir == Dir::Up;
                match self.doc.move_neighbour(id, up) {
                    Err(e) => Err(e.to_string()),
                    Ok(None) => Err("There is no rule to swap with in that direction.".to_string()),
                    Ok(Some(nb)) => {
                        let crossing = self.doc.block(id).is_some_and(Block::has_continue)
                            || self.doc.block(nb).is_some_and(Block::has_continue);
                        let r = self.doc.move_block(id, up).map_err(|e| e.to_string());
                        if r.is_ok() {
                            focus = Some(id);
                            if crossing {
                                self.moved.insert(id);
                            } else {
                                self.moved.remove(&id);
                            }
                        }
                        r
                    }
                }
            }
            RowAction::SetKeyword(id, kw) => self
                .doc
                .set_block_keyword(id, kw)
                .map_err(|e| e.to_string()),
            RowAction::SetContinue(id, on) => {
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
        };
        match result {
            Ok(()) => {
                self.rebuild();
                ActionOutcome {
                    changed: true,
                    status,
                    focus,
                }
            }
            Err(msg) => ActionOutcome {
                changed: false,
                status: Some(msg),
                focus: None,
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
    use crate::filter::section::{BEGIN_LINE, END_LINE};

    const THREE: &str =
        "Show\n  Class \"Rings\"\n\nHide\n  Rarity Normal\n\nShow\n  ItemLevel >= 82\n";

    fn with_economy() -> String {
        format!(
            "Show\n  Class \"Rings\"\n\n{BEGIN_LINE}\nShow\n  Class \"Gems\"\n\nShow\n  Class \"Currency\"\n\n{END_LINE}\n\nHide\n  Rarity Normal\n\nShow\n  ItemLevel >= 82\n"
        )
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
    fn derived_labels() {
        let bases: Vec<String> = (0..14).map(|i| format!("\"Base {i}\"")).collect();
        let t = format!(
            "Show\n  Class \"Stackable Currency\"\n  BaseType == {}\n\nShow\n  Class \"Rings\" \"Amulets\" \"Belts\"\n\nHide\n\nShow\n  AreaLevel <= 65\n  Rarity Magic\n",
            bases.join(" ")
        );
        let wf = WorkingFilter::from_text(&t);
        let l: Vec<&str> = wf.rows().iter().map(|r| r.label.as_str()).collect();
        assert_eq!(
            l[0],
            "1  Show \u{00B7} Stackable Currency \u{00B7} BaseType \u{00D7}14"
        );
        assert_eq!(l[1], "2  Show \u{00B7} Rings +2 more");
        assert_eq!(l[2], "3  Hide \u{00B7} matches everything");
        assert_eq!(l[3], "4  Show \u{00B7} AreaLevel +1 more");
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
    fn style_tag_is_exposed() {
        let wf = WorkingFilter::from_text("#ff:style S\nShow\n  Rarity Rare\n");
        assert_eq!(wf.rows()[0].style_tag.as_deref(), Some("S"));
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

    // ---- actions ----

    #[test]
    fn add_rule_appends_before_economy() {
        let mut wf = WorkingFilter::from_text(&with_economy());
        let o = wf.apply_action(RowAction::AddRule);
        assert!(o.changed);
        let id = o.focus.expect("focus");
        let row = wf.row(id).expect("row");
        assert_eq!(row.label, "2  Show \u{00B7} matches everything");
        assert_eq!(wf.rows().len(), 4);
        assert!(wf.is_dirty());
        assert_eq!(*wf.economy(), EconomyView::Present { blocks: 2 });
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
        assert!(wf.text().contains("#ff:name My rule\n"));
        assert_eq!(wf.rows()[0].label, "1  Show - My rule");
        wf.apply_action(RowAction::SetName(id, "  ".into()));
        assert!(wf.rows()[0].name.is_none());
        wf.apply_action(RowAction::SetName(id, "x".into()));
        wf.apply_action(RowAction::ClearName(id));
        assert!(!wf.text().contains("#ff:name"));
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
}
