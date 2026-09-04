//! Pure, framework-free mapping from a [`Config`]'s rules to read-only display rows (D-19).
//!
//! `rule_rows` is the only function in this module and the only thing the presentation layer
//! calls. It contains no `egui`/`eframe` types in its signature — see `PITFALLS.md` §8: if a bug
//! can only be found by clicking, it belongs in a different layer than this one. Every branch
//! below is driven by `#[test]`s in the `tests` module, not by running the GUI.
//!
//! Ordinal identity (`PITFALLS.md` §9): `ordinal` is always the rule's index in
//! `config.rules`, computed via a single `enumerate()` that never filters. A disabled rule is
//! not skipped — skipping it would renumber every rule after it, breaking the trace from a row
//! back to its position in `dynamic.json`. This mirrors `emitter::emit_rules`'s own labeling
//! discipline (`rule {idx}`), just for display instead of filter text.

use crate::models::config::{AreaLevelRange, Config, NumericCondition, RarityOp, Rule, When};

/// A maximum of this many class names are spelled out in a summary before the rest collapse
/// into a `+N more` suffix, so a long class list cannot blow out the stripe width (D-23).
const MAX_CLASSES_SHOWN: usize = 4;

/// One row of the read-only rule list.
///
/// `ordinal` is the rule's array index — the identity the overlay (and, in Phase 14, the editor)
/// must key off, never a recomputed position after any filtering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleRow {
    /// The rule's position in `config.rules`, unchanged by enabled/disabled state.
    pub ordinal: usize,
    /// `true` unless the rule is `enabled: Some(false)`.
    pub enabled: bool,
    /// The raw `style` field (e.g. `"S"`, `"unchanged"`, `"hidden"`) — not resolved against
    /// `Styles`; this is a display label, not a validated reference.
    pub style: String,
    /// The rule's `name`, if authored. `dynamic_base.json`'s rules have none — ordinal is the
    /// identity there, not the name.
    pub name: Option<String>,
    /// A human-readable summary of `when`'s present conditions, or `"matches everything"` for
    /// an absent/empty `when`.
    pub summary: String,
}

/// Maps every rule in `config.rules` (in order) to a [`RuleRow`]. An absent `rules` array (a
/// `Config` from before Phase 6, or a hand-stripped store) yields an empty vec, not an error —
/// an empty read-only list is a valid, renderable state.
pub fn rule_rows(_config: &Config) -> Vec<RuleRow> {
    // RED stub (13-04 Task 1 TDD cycle): intentionally ignores its input so the behavior
    // tests below fail before the real mapping is implemented.
    Vec::new()
}

fn rule_row(ordinal: usize, rule: &Rule) -> RuleRow {
    RuleRow {
        ordinal,
        enabled: rule.enabled != Some(false),
        style: rule.style.clone(),
        name: rule.name.clone(),
        summary: summarize(rule.when.as_ref()),
    }
}

/// Joins the present conditions of a `when` block with ` · ` in a fixed order: class, rarity,
/// area level, armour, evasion, energy shield, quality, sockets, corrupted, base type,
/// unidentified tier. Absent conditions contribute nothing to the string — never a placeholder
/// like `none` or `-`.
fn summarize(when: Option<&When>) -> String {
    let Some(when) = when else {
        return "matches everything".to_string();
    };

    let mut parts: Vec<String> = Vec::new();

    if let Some(classes) = &when.class {
        parts.push(summarize_list(classes));
    }
    if let Some(rarity) = &when.rarity {
        parts.push(format!(
            "Rarity {} {}",
            rarity_op_str(rarity.op),
            rarity_str(rarity.rarity)
        ));
    }
    if let Some(area) = &when.area_level {
        if let Some(s) = summarize_area_level(area) {
            parts.push(s);
        }
    }
    if let Some(nc) = &when.base_armour {
        parts.extend(summarize_numeric("Armour", nc));
    }
    if let Some(nc) = &when.base_evasion {
        parts.extend(summarize_numeric("Evasion", nc));
    }
    if let Some(nc) = &when.base_energy_shield {
        parts.extend(summarize_numeric("Energy Shield", nc));
    }
    if let Some(nc) = &when.quality {
        parts.extend(summarize_numeric("Quality", nc));
    }
    if let Some(nc) = &when.sockets {
        parts.extend(summarize_numeric("Sockets", nc));
    }
    if let Some(c) = when.corrupted {
        parts.push(format!("Corrupted {c}"));
    }
    if let Some(base_types) = &when.base_type {
        parts.push(summarize_list(base_types));
    }
    if let Some(ut) = &when.unidentified_tier {
        parts.push(format!("Unidentified Tier ≥ {}", ut.min));
    }

    if parts.is_empty() {
        "matches everything".to_string()
    } else {
        parts.join(" · ")
    }
}

/// Renders a name list (class or base type), truncating past [`MAX_CLASSES_SHOWN`] entries to
/// `first, second, third, fourth +N more`.
fn summarize_list(names: &[String]) -> String {
    if names.len() > MAX_CLASSES_SHOWN {
        let shown = names[..MAX_CLASSES_SHOWN].join(", ");
        format!("{shown} +{} more", names.len() - MAX_CLASSES_SHOWN)
    } else {
        names.join(", ")
    }
}

fn summarize_area_level(area: &AreaLevelRange) -> Option<String> {
    match (area.min, area.max) {
        (Some(min), Some(max)) => Some(format!("AreaLevel {min}\u{2013}{max}")),
        (Some(min), None) => Some(format!("AreaLevel \u{2265} {min}")),
        (None, Some(max)) => Some(format!("AreaLevel \u{2264} {max}")),
        (None, None) => None,
    }
}

/// Mirrors `emitter::emit_numeric`'s own semantics exactly, rather than inventing a second
/// vocabulary: `eq: N` -> `N`, `min: 1` on a defence attribute -> `> 0`, `min: N` (otherwise) ->
/// `\u{2265} N`, `max: N` -> `\u{2264} N`.
fn summarize_numeric(label: &str, nc: &NumericCondition) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(v) = nc.eq {
        out.push(format!("{label} {v}"));
    }
    if let Some(n) = nc.min {
        let is_defence = matches!(label, "Armour" | "Evasion" | "Energy Shield");
        if is_defence && n == 1 {
            out.push(format!("{label} > 0"));
        } else {
            out.push(format!("{label} \u{2265} {n}"));
        }
    }
    if let Some(n) = nc.max {
        out.push(format!("{label} \u{2264} {n}"));
    }
    out
}

fn rarity_op_str(op: RarityOp) -> &'static str {
    match op {
        RarityOp::Eq => "=",
        RarityOp::Gte => "\u{2265}",
        RarityOp::Lte => "\u{2264}",
    }
}

fn rarity_str(rarity: crate::models::config::Rarity) -> &'static str {
    use crate::models::config::Rarity;
    match rarity {
        Rarity::Normal => "Normal",
        Rarity::Magic => "Magic",
        Rarity::Rare => "Rare",
        Rarity::Unique => "Unique",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::{Rarity, RarityComparator, UnidentifiedTier};

    /// A real `Config` (the committed dynamic template) with its `rules` swapped out — reuses a
    /// fully valid `styles`/`build`/`meta` section instead of hand-rolling a minimal one, and
    /// exercises the same deserialization path production code does.
    fn base_config() -> Config {
        let json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dynamic_base.json"));
        serde_json::from_str(json).expect("dynamic_base.json must parse as a Config")
    }

    fn with_rules(rules: Vec<Rule>) -> Config {
        let mut config = base_config();
        config.rules = Some(rules);
        config
    }

    fn rule(style: &str, when: Option<When>) -> Rule {
        Rule {
            enabled: None,
            name: None,
            style: style.to_string(),
            when,
        }
    }

    fn disabled_rule(style: &str) -> Rule {
        Rule {
            enabled: Some(false),
            name: None,
            style: style.to_string(),
            when: None,
        }
    }

    #[test]
    fn ordinals_match_array_position_including_a_disabled_rule() {
        let config = with_rules(vec![rule("S", None), disabled_rule("A"), rule("B", None)]);
        let rows = rule_rows(&config);

        assert_eq!(rows.len(), 3, "the disabled rule must still be listed, not omitted");
        assert_eq!(rows[0].ordinal, 0);
        assert_eq!(rows[1].ordinal, 1);
        assert_eq!(rows[2].ordinal, 2);
    }

    #[test]
    fn enabled_none_is_true_and_some_false_is_false_without_changing_ordinal() {
        let config = with_rules(vec![rule("S", None), disabled_rule("A")]);
        let rows = rule_rows(&config);

        assert!(rows[0].enabled, "enabled: None must display as enabled");
        assert!(!rows[1].enabled, "enabled: Some(false) must display as disabled");
        assert_eq!(rows[1].ordinal, 1, "a disabled rule keeps its ordinal position");
    }

    #[test]
    fn absent_when_summarizes_as_matches_everything() {
        let config = with_rules(vec![rule("S", None)]);
        assert_eq!(rule_rows(&config)[0].summary, "matches everything");
    }

    #[test]
    fn summary_joins_class_sockets_and_corrupted_in_fixed_order() {
        let when = When {
            class: Some(vec!["Body Armours".to_string(), "Helmets".to_string()]),
            sockets: Some(NumericCondition {
                eq: None,
                min: Some(2),
                max: None,
            }),
            corrupted: Some(false),
            ..Default::default()
        };
        let config = with_rules(vec![rule("unchanged", Some(when))]);
        let summary = rule_rows(&config)[0].summary.clone();

        assert!(summary.contains("Body Armours, Helmets"), "got: {summary}");
        assert!(summary.contains("Sockets \u{2265} 2"), "got: {summary}");
        assert!(summary.contains("Corrupted false"), "got: {summary}");

        let class_pos = summary.find("Body Armours").unwrap();
        let sockets_pos = summary.find("Sockets").unwrap();
        let corrupted_pos = summary.find("Corrupted").unwrap();
        assert!(
            class_pos < sockets_pos && sockets_pos < corrupted_pos,
            "conditions must join in the fixed order (class, ..., sockets, corrupted): {summary}"
        );
    }

    #[test]
    fn rarity_gte_renders_with_the_gte_operator() {
        let when = When {
            rarity: Some(RarityComparator {
                op: RarityOp::Gte,
                rarity: Rarity::Magic,
            }),
            ..Default::default()
        };
        let config = with_rules(vec![rule("C", Some(when))]);
        assert_eq!(rule_rows(&config)[0].summary, "Rarity \u{2265} Magic");
    }

    #[test]
    fn rarity_bare_eq_renders_with_the_equals_sign() {
        let when = When {
            rarity: Some(RarityComparator {
                op: RarityOp::Eq,
                rarity: Rarity::Rare,
            }),
            ..Default::default()
        };
        let config = with_rules(vec![rule("C", Some(when))]);
        assert_eq!(rule_rows(&config)[0].summary, "Rarity = Rare");
    }

    #[test]
    fn energy_shield_min_one_renders_as_strictly_positive() {
        let when = When {
            base_energy_shield: Some(NumericCondition {
                eq: None,
                min: Some(1),
                max: None,
            }),
            ..Default::default()
        };
        let config = with_rules(vec![rule("D", Some(when))]);
        assert_eq!(rule_rows(&config)[0].summary, "Energy Shield > 0");
    }

    #[test]
    fn evasion_eq_zero_renders_as_the_exact_value() {
        let when = When {
            base_evasion: Some(NumericCondition {
                eq: Some(0),
                min: None,
                max: None,
            }),
            ..Default::default()
        };
        let config = with_rules(vec![rule("D", Some(when))]);
        assert_eq!(rule_rows(&config)[0].summary, "Evasion 0");
    }

    #[test]
    fn a_class_list_longer_than_four_truncates_with_a_more_suffix() {
        let classes = [
            "Body Armours",
            "Helmets",
            "Gloves",
            "Boots",
            "Shields",
            "Bucklers",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let when = When {
            class: Some(classes),
            ..Default::default()
        };
        let config = with_rules(vec![rule("unchanged", Some(when))]);
        let summary = rule_rows(&config)[0].summary.clone();

        assert!(
            summary.starts_with("Body Armours, Helmets, Gloves, Boots +2 more"),
            "got: {summary}"
        );
        assert!(!summary.contains("Shields"), "a truncated list must not spell out the rest");
    }

    #[test]
    fn unidentified_tier_and_area_level_render_with_their_own_operators() {
        let when = When {
            area_level: Some(AreaLevelRange {
                min: Some(10),
                max: None,
            }),
            unidentified_tier: Some(UnidentifiedTier { min: 4 }),
            ..Default::default()
        };
        let config = with_rules(vec![rule("unidentified", Some(when))]);
        let summary = rule_rows(&config)[0].summary.clone();

        assert!(summary.contains("AreaLevel \u{2265} 10"), "got: {summary}");
        assert!(summary.contains("Unidentified Tier \u{2265} 4"), "got: {summary}");
    }

    #[test]
    fn rule_rows_on_the_real_dynamic_base_json_has_dense_ordinals_and_nonempty_styles() {
        let config = base_config();
        let rows = rule_rows(&config);

        assert!(!rows.is_empty(), "dynamic_base.json must have at least one rule");
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.ordinal, i, "ordinals must be exactly 0..len-1");
            assert!(!row.style.is_empty(), "every row must have a non-empty style");
        }
    }
}
