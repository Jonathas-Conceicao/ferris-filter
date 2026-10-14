//! Pure logic behind the rule editor's condition cards (Phase 26: UAT3-05, UAT3-06).
//!
//! No egui types and no file IO. It answers four questions the body view would otherwise have to
//! decide inline:
//!
//! * how a rule's condition lines group into cards ([`card_plan`]): the three defence keywords
//!   share one Defences card, and exactly one lower plus one upper bound of the same numeric
//!   keyword share one Between card;
//! * which picker entries are already on the rule ([`present_entries`]);
//! * which operator choices a numeric condition offers, Between included
//!   ([`numeric_op_choices`]);
//! * what switching a single-line condition to Between writes ([`between_switch`]).
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
/// clamped so the pair is always satisfiable.
pub fn between_switch(kw: &Keyword, op: Option<Operator>, value: i64) -> BetweenSwitch {
    let (min, max, _) = grammar::slider_range(kw.name);
    if is_lower(op) {
        BetweenSwitch {
            new_op_for_existing: None,
            insert_after: between_line(kw.name, Operator::Le, max.max(value)),
            existing_is_lower: true,
        }
    } else if is_upper(op) {
        BetweenSwitch {
            new_op_for_existing: None,
            insert_after: between_line(kw.name, Operator::Ge, min.min(value)),
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

/// A Between pair is only valid when `min <= max`.
pub fn between_ok(min: i64, max: i64) -> bool {
    min <= max
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
    fn between_ok_rejects_min_above_max() {
        assert!(between_ok(1, 1));
        assert!(between_ok(1, 2));
        assert!(!between_ok(3, 2));
        assert_eq!(between_line("Quality", Operator::Ge, 5), "Quality >= 5");
    }
}
