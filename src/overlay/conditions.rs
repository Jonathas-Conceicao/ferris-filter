//! Pure logic behind the rule editor's condition cards (Phase 26: UAT3-05, UAT3-06).
//!
//! No egui types and no file IO. It answers five questions the body view would otherwise have to
//! decide inline:
//!
//! * how a rule's condition lines group into cards ([`card_plan`]): the three defence keywords
//!   share one Defences card, and exactly one lower plus one upper bound of the same numeric
//!   keyword share one Between card;
//! * which picker entries are already on the rule ([`present_entries`]);
//! * which operator choices a numeric condition offers, Between included
//!   ([`numeric_op_choices`]);
//! * what switching a single-line condition to Between writes ([`between_switch`]);
//! * whether a rule's defence lines read as a Simple Defences card (Any | Has | No per type) or
//!   the Defences card ([`defences_kind`]), what one segment change writes ([`seg_write`]), and
//!   which of the seven combinations the settings allow ([`lit_combos`]) (Phase 27: UAT4-03).
//!
//! Grouping is display-only: no line is rewritten by it (PARSE-04).

use crate::filter::keywords::{Keyword, ValueShape};
use crate::filter::tokenize::Operator;
use crate::overlay::grammar::{self, ConditionEntry};
use crate::overlay::rules::{BodyEntry, EntryKind};

/// The three defence keywords with their toggle labels, in card order.
pub const DEFENCES: [(&str, &str); 3] = [
    ("BaseArmour", "Armour"),
    ("BaseEvasion", "Evasion"),
    ("BaseEnergyShield", "Energy Shield"),
];

/// True for the three defence keywords (case-insensitive).
pub fn is_defence(name: &str) -> bool {
    DEFENCES.iter().any(|(n, _)| n.eq_ignore_ascii_case(name))
}

/// One displayable unit: a single line, or a lower+upper pair of the same keyword. Indices point
/// into the slice given to [`card_plan`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Single(usize),
    Between { lower: usize, upper: usize },
}

/// One card of the rule body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardPlan {
    Part(Part),
    /// Defence keyword name with its parts, in `DEFENCES` order; only keywords present.
    Defences(Vec<(&'static str, Vec<Part>)>),
}

fn is_lower(op: Option<Operator>) -> bool {
    matches!(op, Some(Operator::Ge | Operator::Gt))
}

fn is_upper(op: Option<Operator>) -> bool {
    matches!(op, Some(Operator::Le | Operator::Lt))
}

fn single_int(e: &BodyEntry) -> bool {
    e.values.len() == 1 && e.values[0].parse::<i64>().is_ok()
}

fn part_pos(p: &Part) -> usize {
    match p {
        Part::Single(i) => *i,
        Part::Between { lower, upper } => (*lower).min(*upper),
    }
}

/// Parts of one keyword's lines (`idxs` ascending). Between only for exactly two lines, one lower
/// and one upper bound, of an `Int` keyword with a single integer value each.
fn parts_of(entries: &[&BodyEntry], idxs: &[usize]) -> Vec<Part> {
    if let [a, b] = *idxs {
        let (ea, eb) = (entries[a], entries[b]);
        let int = ea.keyword.is_some_and(|k| k.value == ValueShape::Int);
        if int && single_int(ea) && single_int(eb) {
            if is_lower(ea.op) && is_upper(eb.op) {
                return vec![Part::Between { lower: a, upper: b }];
            }
            if is_upper(ea.op) && is_lower(eb.op) {
                return vec![Part::Between { lower: b, upper: a }];
            }
        }
    }
    idxs.iter().map(|i| Part::Single(*i)).collect()
}

/// Group a rule's condition entries into cards, ordered by the position of each card's first
/// line. Entries that are not conditions with a known keyword (opaque raw rows) are always their
/// own `Single` card.
pub fn card_plan(entries: &[&BodyEntry]) -> Vec<CardPlan> {
    let mut cards: Vec<(usize, CardPlan)> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut defence_idx: Vec<usize> = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        match (e.kind, e.keyword) {
            (EntryKind::Condition, Some(k)) if is_defence(k.name) => defence_idx.push(i),
            (EntryKind::Condition, Some(k)) => {
                let key = k.name.to_ascii_lowercase();
                if seen.contains(&key) {
                    continue;
                }
                seen.push(key);
                let idxs: Vec<usize> = (0..entries.len())
                    .filter(|j| {
                        entries[*j].kind == EntryKind::Condition
                            && entries[*j]
                                .keyword
                                .is_some_and(|x| x.name.eq_ignore_ascii_case(k.name))
                    })
                    .collect();
                for p in parts_of(entries, &idxs) {
                    cards.push((part_pos(&p), CardPlan::Part(p)));
                }
            }
            _ => cards.push((i, CardPlan::Part(Part::Single(i)))),
        }
    }
    if let Some(first) = defence_idx.first().copied() {
        let mut subs: Vec<(&'static str, Vec<Part>)> = Vec::new();
        for (name, _) in DEFENCES {
            let idxs: Vec<usize> = defence_idx
                .iter()
                .copied()
                .filter(|j| {
                    entries[*j]
                        .keyword
                        .is_some_and(|k| k.name.eq_ignore_ascii_case(name))
                })
                .collect();
            if !idxs.is_empty() {
                subs.push((name, parts_of(entries, &idxs)));
            }
        }
        cards.push((first, CardPlan::Defences(subs)));
    }
    cards.sort_by_key(|(pos, _)| *pos);
    cards.into_iter().map(|(_, c)| c).collect()
}

/// Picker entries already on the rule: lines of the rule, pending keyword rows, and a pending
/// Defences card. Any defence line counts as `Defences`. No duplicates, first-seen order.
pub fn present_entries(
    entries: &[&BodyEntry],
    pending: &[&'static Keyword],
    pending_defences: bool,
) -> Vec<ConditionEntry> {
    let mut out: Vec<ConditionEntry> = Vec::new();
    let mut push = |e: ConditionEntry| {
        if !out.contains(&e) {
            out.push(e);
        }
    };
    for e in entries {
        if e.kind != EntryKind::Condition {
            continue;
        }
        if let Some(k) = e.keyword {
            push(entry_for(k));
        }
    }
    for k in pending {
        push(entry_for(k));
    }
    if pending_defences {
        push(ConditionEntry::Defences);
    }
    out
}

fn entry_for(k: &'static Keyword) -> ConditionEntry {
    if is_defence(k.name) {
        ConditionEntry::Defences
    } else {
        ConditionEntry::Keyword(k)
    }
}

/// The operator dropdown value of a numeric condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericOp {
    Single(Option<Operator>),
    Between,
}

/// True when `entries[i]` is the only condition line of its keyword in the rule. Between is only
/// offered then: with other lines of the keyword present, the inserted bound would not group into
/// a Between card (see [`card_plan`]) and could not be switched back in one step.
pub fn sole_line(entries: &[&BodyEntry], i: usize) -> bool {
    let Some(k) = entries.get(i).and_then(|e| e.keyword) else {
        return false;
    };
    entries
        .iter()
        .filter(|e| {
            e.kind == EntryKind::Condition
                && e.keyword
                    .is_some_and(|x| x.name.eq_ignore_ascii_case(k.name))
        })
        .count()
        == 1
}

/// [`numeric_op_choices`] without Between unless `can_between` (see [`sole_line`]).
pub fn numeric_op_choices_for(
    kw: &Keyword,
    current: NumericOp,
    can_between: bool,
) -> Vec<(NumericOp, &'static str)> {
    let mut out = numeric_op_choices(kw, current);
    if !can_between {
        out.retain(|(c, _)| *c != NumericOp::Between);
    }
    out
}

/// Operator dropdown entries. `Int` keywords get a trailing "Between"; every other shape gets the
/// existing choices only.
pub fn numeric_op_choices(kw: &Keyword, current: NumericOp) -> Vec<(NumericOp, &'static str)> {
    let cur = match current {
        NumericOp::Single(op) => op,
        NumericOp::Between => Some(Operator::Ge),
    };
    let mut out: Vec<(NumericOp, &'static str)> = grammar::operator_choices(kw, cur, false)
        .into_iter()
        .map(|c| (NumericOp::Single(c.op), c.label))
        .collect();
    if kw.value == ValueShape::Int {
        out.push((NumericOp::Between, "Between"));
    }
    out
}

/// What switching a single-line numeric condition to Between does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetweenSwitch {
    /// New operator for the existing line; `None` keeps its bytes.
    pub new_op_for_existing: Option<Operator>,
    /// Full text (no indent) of the other bound, to insert right after the existing line.
    pub insert_after: String,
    /// True when the existing line is the lower bound.
    pub existing_is_lower: bool,
}

/// Line text `<kw> <op> <value>`.
pub fn between_line(kw_name: &str, op: Operator, value: i64) -> String {
    format!("{kw_name} {} {value}", op.as_str())
}

/// Minimal switch of `kw op value` to a Between pair. The inserted bound is the slider limit,
/// clamped so the pair is always satisfiable (a strict `>` / `<` bound excludes its own value).
pub fn between_switch(kw: &Keyword, op: Option<Operator>, value: i64) -> BetweenSwitch {
    let (min, max, _) = grammar::slider_range(kw.name);
    if is_lower(op) {
        BetweenSwitch {
            new_op_for_existing: None,
            insert_after: between_line(kw.name, Operator::Le, max.max(inclusive_lo(op, value))),
            existing_is_lower: true,
        }
    } else if is_upper(op) {
        BetweenSwitch {
            new_op_for_existing: None,
            insert_after: between_line(kw.name, Operator::Ge, min.min(inclusive_hi(op, value))),
            existing_is_lower: false,
        }
    } else {
        BetweenSwitch {
            new_op_for_existing: Some(Operator::Ge),
            insert_after: between_line(kw.name, Operator::Le, value),
            existing_is_lower: true,
        }
    }
}

/// The smallest whole number a lower bound `op value` admits (`> v` admits `v + 1`).
pub fn inclusive_lo(op: Option<Operator>, value: i64) -> i64 {
    if op == Some(Operator::Gt) {
        value.saturating_add(1)
    } else {
        value
    }
}

/// The largest whole number an upper bound `op value` admits (`< v` admits `v - 1`).
pub fn inclusive_hi(op: Option<Operator>, value: i64) -> i64 {
    if op == Some(Operator::Lt) {
        value.saturating_sub(1)
    } else {
        value
    }
}

/// A Between pair matches something only when some whole number satisfies both bounds, with
/// strict `>` / `<` bounds excluding their own value.
pub fn between_ok(lo_op: Option<Operator>, lo: i64, hi_op: Option<Operator>, hi: i64) -> bool {
    inclusive_lo(lo_op, lo) <= inclusive_hi(hi_op, hi)
}

/// The ranges each bound's value may take so the pair stays satisfiable: `(min range, max
/// range)`, honouring strict operators on either side.
pub fn bound_clamps(
    lo_op: Option<Operator>,
    lo: i64,
    hi_op: Option<Operator>,
    hi: i64,
) -> ((i64, i64), (i64, i64)) {
    let lo_max = inclusive_hi(hi_op, hi).saturating_sub(inclusive_lo(lo_op, 0));
    let hi_min = inclusive_lo(lo_op, lo).saturating_sub(inclusive_hi(hi_op, 0));
    ((i64::MIN, lo_max), (hi_min, i64::MAX))
}

/// One row of the Simple Defences card: no line, a "has this defence" line, or a "has none of
/// it" line (UAT4-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefenceSeg {
    Any,
    Has,
    No,
}

/// The segment a zero-based defence line stands for, or `None` when the line is anything else.
///
/// Zero based: exactly one integer value, and `0` with no operator, `=` or `==` (No), or `> 0`,
/// or `>= 1` (Has). Non-integer or multi-value lines are `None`, never a panic.
pub fn zero_based(e: &BodyEntry) -> Option<DefenceSeg> {
    let [v] = e.values.as_slice() else {
        return None;
    };
    let v: i64 = v.parse().ok()?;
    match (e.op, v) {
        (None | Some(Operator::Eq | Operator::EqEq), 0) => Some(DefenceSeg::No),
        (Some(Operator::Gt), 0) | (Some(Operator::Ge), 1) => Some(DefenceSeg::Has),
        _ => None,
    }
}

/// A rule's defence lines read as Simple Defences rows, in `DEFENCES` order. Each row holds the
/// type's segment and the index (into the slice given to [`simple_defences`]) of its line, `None`
/// for Any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimpleSet {
    pub rows: [(DefenceSeg, Option<usize>); 3],
}

impl SimpleSet {
    /// The three segments, in `DEFENCES` order.
    pub fn settings(&self) -> [DefenceSeg; 3] {
        self.rows.map(|(s, _)| s)
    }
}

/// Read the rule's defence lines as a Simple Defences set. Only condition entries with a defence
/// keyword count (case-insensitive); other lines are ignored. `None` when any defence line is not
/// zero based or a type has more than one line (a Between pair included). A rule without defence
/// lines gives all Any; callers decide whether a card shows.
pub fn simple_defences(entries: &[&BodyEntry]) -> Option<SimpleSet> {
    let mut rows = [(DefenceSeg::Any, None); 3];
    for (i, e) in entries.iter().enumerate() {
        if e.kind != EntryKind::Condition {
            continue;
        }
        let Some(k) = e.keyword else { continue };
        let Some(t) = DEFENCES
            .iter()
            .position(|(n, _)| n.eq_ignore_ascii_case(k.name))
        else {
            continue;
        };
        if rows[t].1.is_some() {
            return None;
        }
        rows[t] = (zero_based(e)?, Some(i));
    }
    Some(SimpleSet { rows })
}

/// Which defence card a rule shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefencesKind {
    Simple(SimpleSet),
    /// The existing Defences card (per-type operator, slider, Between).
    Advanced,
}

/// Which defence card the rule's lines load as; `None` when the rule has no defence line.
///
/// `advanced` is the per-rule marker the rule list keeps once a rule shows a Defences card
/// (loaded or added from the picker): with it set the rule always stays Advanced, so an edit
/// inside the Defences card that happens to write zero-based lines (`>= 1` from a toggle, `> 0`
/// from a slider) never flips it to Simple mid-session. After a reload the marker is cleared and
/// classification comes from the file again.
pub fn defences_kind(entries: &[&BodyEntry], advanced: bool) -> Option<DefencesKind> {
    let any = entries
        .iter()
        .any(|e| e.kind == EntryKind::Condition && e.keyword.is_some_and(|k| is_defence(k.name)));
    if !any {
        return None;
    }
    if advanced {
        return Some(DefencesKind::Advanced);
    }
    Some(simple_defences(entries).map_or(DefencesKind::Advanced, DefencesKind::Simple))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::keywords::lookup;
    use crate::overlay::rules::WorkingFilter;

    fn e(line: usize, kw: &str, op: Option<Operator>, val: &str) -> BodyEntry {
        BodyEntry {
            line,
            kind: EntryKind::Condition,
            keyword: lookup(kw),
            op,
            values: vec![val.to_string()],
            text: format!("\t{kw} {val}"),
            ordinal: 0,
        }
    }

    fn plan(es: &[BodyEntry]) -> Vec<CardPlan> {
        let refs: Vec<&BodyEntry> = es.iter().collect();
        card_plan(&refs)
    }

    const GE: Option<Operator> = Some(Operator::Ge);
    const LE: Option<Operator> = Some(Operator::Le);
    const GT: Option<Operator> = Some(Operator::Gt);
    const LT: Option<Operator> = Some(Operator::Lt);

    #[test]
    fn lower_and_upper_make_one_between() {
        let p = plan(&[e(1, "ItemLevel", GE, "60"), e(2, "ItemLevel", LE, "80")]);
        assert_eq!(p, [CardPlan::Part(Part::Between { lower: 0, upper: 1 })]);
        let p = plan(&[e(1, "ItemLevel", GT, "60"), e(2, "ItemLevel", LT, "80")]);
        assert_eq!(p, [CardPlan::Part(Part::Between { lower: 0, upper: 1 })]);
        let p = plan(&[e(1, "ItemLevel", LE, "80"), e(2, "ItemLevel", GE, "60")]);
        assert_eq!(p, [CardPlan::Part(Part::Between { lower: 1, upper: 0 })]);
    }

    #[test]
    fn other_duplicates_stay_separate() {
        let two_ge = plan(&[e(1, "ItemLevel", GE, "60"), e(2, "ItemLevel", GE, "70")]);
        assert_eq!(two_ge.len(), 2);
        let eq_ge = plan(&[
            e(1, "ItemLevel", Some(Operator::EqEq), "60"),
            e(2, "ItemLevel", GE, "70"),
        ]);
        assert_eq!(eq_ge.len(), 2);
        let three = plan(&[
            e(1, "ItemLevel", GE, "60"),
            e(2, "ItemLevel", LE, "70"),
            e(3, "ItemLevel", LE, "90"),
        ]);
        assert_eq!(three.len(), 3);
        assert!(
            three
                .iter()
                .all(|c| matches!(c, CardPlan::Part(Part::Single(_))))
        );
    }

    #[test]
    fn defences_collapse_into_one_card_at_first_position() {
        let p = plan(&[
            e(1, "BaseArmour", GE, "100"),
            e(2, "Rarity", None, "Rare"),
            e(3, "BaseEvasion", GE, "5"),
        ]);
        assert_eq!(
            p,
            [
                CardPlan::Defences(vec![
                    ("BaseArmour", vec![Part::Single(0)]),
                    ("BaseEvasion", vec![Part::Single(2)]),
                ]),
                CardPlan::Part(Part::Single(1)),
            ]
        );
    }

    #[test]
    fn between_inside_defences() {
        let p = plan(&[
            e(1, "BaseEnergyShield", GE, "1"),
            e(2, "BaseEnergyShield", LE, "50"),
        ]);
        assert_eq!(
            p,
            [CardPlan::Defences(vec![(
                "BaseEnergyShield",
                vec![Part::Between { lower: 0, upper: 1 }]
            )])]
        );
    }

    #[test]
    fn raw_rows_and_non_int_shapes_never_group() {
        let mut raw = e(1, "Class", None, "x");
        raw.kind = EntryKind::Opaque;
        raw.keyword = None;
        let p = plan(&[raw.clone(), raw]);
        assert_eq!(p.len(), 2);
        // Sockets is not an Int shape, so a pair stays two cards.
        let p = plan(&[e(1, "Sockets", GE, "1"), e(2, "Sockets", LE, "4")]);
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn removed_keyword_is_an_opaque_raw_row() {
        let wf = WorkingFilter::from_text("Show\n\tBaseDefencePercentile 50\n");
        let id = wf.rows()[0].id;
        let view = wf.body_view(id);
        assert_eq!(view.len(), 1);
        assert_eq!(view[0].kind, EntryKind::Opaque);
        assert!(view[0].keyword.is_none());
        let refs: Vec<&BodyEntry> = view.iter().collect();
        assert_eq!(card_plan(&refs), [CardPlan::Part(Part::Single(0))]);
    }

    #[test]
    fn presence_counts_lines_pending_and_defences() {
        let es = [
            e(1, "Class", None, "\"Rings\""),
            e(2, "BaseEvasion", GE, "5"),
        ];
        let refs: Vec<&BodyEntry> = es.iter().collect();
        let present = present_entries(&refs, &[], false);
        assert!(present.contains(&ConditionEntry::Keyword(lookup("Class").unwrap())));
        assert!(present.contains(&ConditionEntry::Defences));
        assert!(!present.contains(&ConditionEntry::Keyword(lookup("BaseEvasion").unwrap())));
        let none = present_entries(&[], &[lookup("Rarity").unwrap()], true);
        assert_eq!(
            none,
            [
                ConditionEntry::Keyword(lookup("Rarity").unwrap()),
                ConditionEntry::Defences
            ]
        );
        assert!(present_entries(&[], &[], false).is_empty());
    }

    #[test]
    fn between_offered_only_for_int_keywords() {
        let il = lookup("ItemLevel").unwrap();
        let ch = numeric_op_choices(il, NumericOp::Single(GE));
        assert!(ch.iter().any(|(o, _)| *o == NumericOp::Between));
        let cl = lookup("Class").unwrap();
        let ch = numeric_op_choices(cl, NumericOp::Single(None));
        assert!(ch.iter().all(|(o, _)| *o != NumericOp::Between));
        let so = lookup("Sockets").unwrap();
        let ch = numeric_op_choices(so, NumericOp::Single(GE));
        assert!(ch.iter().all(|(o, _)| *o != NumericOp::Between));
        let ch = numeric_op_choices(il, NumericOp::Between);
        assert!(ch.iter().any(|(o, _)| *o == NumericOp::Between));
    }

    #[test]
    fn between_needs_the_keywords_only_line() {
        let es = [
            e(1, "ItemLevel", GE, "60"),
            e(2, "ItemLevel", Some(Operator::Ne), "70"),
            e(3, "Quality", GE, "5"),
        ];
        let refs: Vec<&BodyEntry> = es.iter().collect();
        assert!(!sole_line(&refs, 0));
        assert!(!sole_line(&refs, 1));
        assert!(sole_line(&refs, 2));
        let il = lookup("ItemLevel").unwrap();
        let cur = NumericOp::Single(GE);
        let no = numeric_op_choices_for(il, cur, false);
        assert!(no.iter().all(|(c, _)| *c != NumericOp::Between));
        assert_eq!(no.len() + 1, numeric_op_choices(il, cur).len());
        let yes = numeric_op_choices_for(il, cur, true);
        assert!(yes.iter().any(|(c, _)| *c == NumericOp::Between));
    }

    #[test]
    fn switch_from_lower_keeps_bytes_and_adds_upper() {
        let il = lookup("ItemLevel").unwrap();
        let s = between_switch(il, GE, 60);
        assert_eq!(s.new_op_for_existing, None);
        assert_eq!(s.insert_after, "ItemLevel <= 100");
        assert!(s.existing_is_lower);
    }

    #[test]
    fn switch_from_upper_adds_lower() {
        let il = lookup("ItemLevel").unwrap();
        let s = between_switch(il, LE, 80);
        assert_eq!(s.new_op_for_existing, None);
        assert_eq!(s.insert_after, "ItemLevel >= 1");
        assert!(!s.existing_is_lower);
    }

    #[test]
    fn switch_from_exact_or_bare_edits_only_the_operator() {
        let il = lookup("ItemLevel").unwrap();
        for op in [Some(Operator::EqEq), Some(Operator::Eq), None] {
            let s = between_switch(il, op, 70);
            assert_eq!(s.new_op_for_existing, GE);
            assert_eq!(s.insert_after, "ItemLevel <= 70");
            assert!(s.existing_is_lower);
        }
    }

    #[test]
    fn switch_never_proposes_an_impossible_pair() {
        let il = lookup("ItemLevel").unwrap();
        // Existing lower bound above the slider max: the inserted upper is clamped up to it.
        let s = between_switch(il, GE, 250);
        assert_eq!(s.insert_after, "ItemLevel <= 250");
        let s = between_switch(il, LE, -5);
        assert_eq!(s.insert_after, "ItemLevel >= -5");
    }

    #[test]
    fn switch_from_a_strict_bound_never_proposes_an_empty_pair() {
        let q = lookup("Quality").unwrap();
        let (_, qmax, _) = grammar::slider_range("Quality");
        let s = between_switch(q, GT, qmax);
        assert_eq!(s.insert_after, format!("Quality <= {}", qmax + 1));
        let il = lookup("ItemLevel").unwrap();
        let (ilmin, _, _) = grammar::slider_range("ItemLevel");
        let s = between_switch(il, LT, ilmin);
        assert_eq!(s.insert_after, format!("ItemLevel >= {}", ilmin - 1));
        // Strictness inside the range changes nothing.
        assert_eq!(between_switch(il, GT, 60).insert_after, "ItemLevel <= 100");
    }

    #[test]
    fn strict_bounds_are_exclusive_in_the_check_and_the_clamps() {
        assert!(!between_ok(GT, 50, LE, 50));
        assert!(!between_ok(GE, 50, LT, 50));
        assert!(!between_ok(GT, 50, LT, 51));
        assert!(between_ok(GT, 50, LT, 52));
        assert!(between_ok(GT, 50, LE, 51));
        // `> 50` / `<= 60`: max may go down to 51, min up to 59.
        assert_eq!(
            bound_clamps(GT, 50, LE, 60),
            ((i64::MIN, 59), (51, i64::MAX))
        );
        // `>= 50` / `< 60`: max down to 51, min up to 59.
        assert_eq!(
            bound_clamps(GE, 50, LT, 60),
            ((i64::MIN, 59), (51, i64::MAX))
        );
        assert_eq!(
            bound_clamps(GE, 50, LE, 60),
            ((i64::MIN, 60), (50, i64::MAX))
        );
    }

    #[test]
    fn between_ok_rejects_min_above_max() {
        assert!(between_ok(GE, 1, LE, 1));
        assert!(between_ok(GE, 1, LE, 2));
        assert!(!between_ok(GE, 3, LE, 2));
        assert_eq!(between_line("Quality", Operator::Ge, 5), "Quality >= 5");
    }

    // --- Simple Defences classification (UAT4-03) ---

    fn view_of(text: &str) -> Vec<BodyEntry> {
        let wf = WorkingFilter::from_text(text);
        let id = wf.rows()[0].id;
        wf.body_view(id)
    }

    fn zb(line: &str) -> Option<DefenceSeg> {
        let v = view_of(&format!("Show\n\t{line}\n"));
        zero_based(&v[0])
    }

    #[test]
    fn zero_based_recognises_no_and_has_forms() {
        assert_eq!(zb("BaseArmour 0"), Some(DefenceSeg::No));
        assert_eq!(zb("BaseArmour = 0"), Some(DefenceSeg::No));
        assert_eq!(zb("BaseArmour == 0"), Some(DefenceSeg::No));
        assert_eq!(zb("BaseArmour > 0"), Some(DefenceSeg::Has));
        assert_eq!(zb("BaseArmour >= 1"), Some(DefenceSeg::Has));
    }

    #[test]
    fn zero_based_rejects_everything_else() {
        for line in [
            "BaseArmour >= 0",
            "BaseArmour != 0",
            "BaseArmour < 5",
            "BaseArmour 5",
            "BaseArmour > 1",
            "BaseArmour >= 2",
            "BaseArmour 0 1",
            "BaseArmour > abc",
        ] {
            assert_eq!(zb(line), None, "{line}");
        }
    }

    fn simple_of(text: &str) -> Option<SimpleSet> {
        let v = view_of(text);
        let refs: Vec<&BodyEntry> = v.iter().collect();
        simple_defences(&refs)
    }

    #[test]
    fn simple_set_reads_each_type() {
        let s = simple_of("Show\n\tRarity Rare\n\tBaseArmour > 0\n\tBaseEvasion 0\n").unwrap();
        assert_eq!(s.rows[0], (DefenceSeg::Has, Some(1)));
        assert_eq!(s.rows[1], (DefenceSeg::No, Some(2)));
        assert_eq!(s.rows[2], (DefenceSeg::Any, None));
        assert_eq!(
            s.settings(),
            [DefenceSeg::Has, DefenceSeg::No, DefenceSeg::Any]
        );
    }

    #[test]
    fn simple_set_keyword_match_is_case_insensitive() {
        let s = simple_of("Show\n\tbaseenergyshield >= 1\n").unwrap();
        assert_eq!(s.rows[2], (DefenceSeg::Has, Some(0)));
    }

    #[test]
    fn simple_set_without_defence_lines_is_all_any() {
        let s = simple_of("Show\n\tRarity Rare\n").unwrap();
        assert_eq!(s.settings(), [DefenceSeg::Any; 3]);
        assert!(s.rows.iter().all(|(_, i)| i.is_none()));
    }

    #[test]
    fn mixed_duplicate_and_between_sets_are_not_simple() {
        assert!(simple_of("Show\n\tBaseArmour > 0\n\tBaseEnergyShield >= 100\n").is_none());
        assert!(simple_of("Show\n\tBaseArmour > 0\n\tBaseArmour >= 1\n").is_none());
        assert!(simple_of("Show\n\tBaseArmour >= 100\n\tBaseArmour <= 500\n").is_none());
    }

    fn kind_of(text: &str, advanced: bool) -> Option<DefencesKind> {
        let v = view_of(text);
        let refs: Vec<&BodyEntry> = v.iter().collect();
        defences_kind(&refs, advanced)
    }

    #[test]
    fn defences_kind_classifies_and_honours_the_advanced_marker() {
        let simple = "Show\n\tBaseArmour > 0\n\tBaseEvasion 0\n";
        assert!(matches!(
            kind_of(simple, false),
            Some(DefencesKind::Simple(_))
        ));
        assert_eq!(kind_of(simple, true), Some(DefencesKind::Advanced));
        assert_eq!(
            kind_of("Show\n\tBaseArmour >= 100\n", false),
            Some(DefencesKind::Advanced)
        );
        assert_eq!(kind_of("Show\n\tRarity Rare\n", false), None);
        assert_eq!(kind_of("Show\n\tRarity Rare\n", true), None);
    }

    // --- Simple Defences writes and the combination strip (UAT4-03) ---

    use DefenceSeg::{Any, Has, No};

    #[test]
    fn seg_write_from_no_line() {
        assert_eq!(seg_write("BaseArmour", None, Any), SegWrite::Nothing);
        assert_eq!(
            seg_write("BaseArmour", None, Has),
            SegWrite::Add("BaseArmour > 0".into())
        );
        assert_eq!(
            seg_write("BaseArmour", None, No),
            SegWrite::Add("BaseArmour 0".into())
        );
    }

    #[test]
    fn seg_write_from_an_existing_line() {
        assert_eq!(seg_write("BaseEvasion", Some(Has), Has), SegWrite::Nothing);
        assert_eq!(seg_write("BaseEvasion", Some(No), No), SegWrite::Nothing);
        assert_eq!(
            seg_write("BaseEvasion", Some(Has), No),
            SegWrite::Replace("BaseEvasion 0".into())
        );
        assert_eq!(
            seg_write("BaseEnergyShield", Some(No), Has),
            SegWrite::Replace("BaseEnergyShield > 0".into())
        );
        assert_eq!(seg_write("BaseEvasion", Some(Has), Any), SegWrite::Remove);
        assert_eq!(seg_write("BaseEvasion", Some(No), Any), SegWrite::Remove);
        // `Some(Any)` means no line too.
        assert_eq!(seg_write("BaseEvasion", Some(Any), Any), SegWrite::Nothing);
    }

    #[test]
    fn written_lines_classify_back_to_their_segment() {
        for (name, _) in DEFENCES {
            for want in [Has, No] {
                let SegWrite::Add(line) = seg_write(name, None, want) else {
                    panic!("expected Add");
                };
                assert_eq!(zb(&line), Some(want), "{line}");
            }
        }
    }

    #[test]
    fn combo_labels_and_short_names() {
        let labels: Vec<&str> = DEFENCE_COMBOS.iter().map(|(l, _)| *l).collect();
        assert_eq!(
            labels,
            ["AR", "EV", "ES", "AR+ES", "AR+EV", "EV+ES", "AR+EV+ES"]
        );
        assert_eq!(DEFENCE_SHORT, ["AR", "EV", "ES"]);
        // Each label spells out its membership.
        for (label, members) in DEFENCE_COMBOS {
            let parts: Vec<&str> = label.split('+').collect();
            for (t, short) in DEFENCE_SHORT.iter().enumerate() {
                assert_eq!(members[t], parts.contains(short), "{label}");
            }
        }
    }

    fn lit_labels(settings: [DefenceSeg; 3]) -> Vec<&'static str> {
        let lit = lit_combos(settings);
        DEFENCE_COMBOS
            .iter()
            .zip(lit)
            .filter(|(_, on)| *on)
            .map(|((l, _), _)| *l)
            .collect()
    }

    #[test]
    fn lit_combos_examples() {
        assert_eq!(lit_combos([Any, Any, Any]), [true; 7]);
        assert_eq!(lit_labels([Any, No, Any]), ["AR", "ES", "AR+ES"]);
        assert_eq!(lit_combos([No, No, No]), [false; 7]);
        assert_eq!(
            lit_labels([Has, Any, Any]),
            ["AR", "AR+ES", "AR+EV", "AR+EV+ES"]
        );
    }

    #[test]
    fn lit_combos_all_27_settings() {
        let segs = [Any, Has, No];
        let mut count = 0;
        for a in segs {
            for b in segs {
                for c in segs {
                    let s = [a, b, c];
                    let lit = lit_combos(s);
                    for (k, (label, members)) in DEFENCE_COMBOS.iter().enumerate() {
                        // Independent formulation: every Has type is in the combo, no No type is.
                        let mut expect = true;
                        for t in 0..3 {
                            if s[t] == Has && !members[t] {
                                expect = false;
                            }
                            if s[t] == No && members[t] {
                                expect = false;
                            }
                        }
                        assert_eq!(lit[k], expect, "{s:?} {label}");
                        assert_eq!(combo_matches(s, *members), expect);
                    }
                    count += 1;
                }
            }
        }
        assert_eq!(count, 27);
    }
}
