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
pub fn rule_rows(config: &Config) -> Vec<RuleRow> {
    let groups = super::edit::class_groups(config);
    config
        .rules
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .enumerate()
        .map(|(ordinal, rule)| rule_row(ordinal, rule, &groups))
        .collect()
}

fn rule_row(ordinal: usize, rule: &Rule, groups: &[(&str, &[String])]) -> RuleRow {
    RuleRow {
        ordinal,
        enabled: rule.enabled != Some(false),
        style: rule.style.clone(),
        name: rule.name.clone(),
        summary: summarize(rule.when.as_ref(), groups),
    }
}

/// Joins the present conditions of a `when` block with ` · ` in a fixed order: class, rarity,
/// area level, armour, evasion, energy shield, quality, sockets, corrupted, base type (labelled
/// `Bases:`, D-15), unidentified tier. Absent conditions contribute nothing to the string — never
/// a placeholder like `none` or `-`.
fn summarize(when: Option<&When>, groups: &[(&str, &[String])]) -> String {
    let Some(when) = when else {
        return "matches everything".to_string();
    };

    let mut parts: Vec<String> = Vec::new();

    if let Some(classes) = &when.class {
        parts.push(summarize_classes(classes, groups));
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
        // D-15: labelled explicitly. With `class` now optional (Phase 15), a base-type-only
        // rule's summary would otherwise be a bare name list, structurally identical to a class
        // list -- ambiguous about which condition it describes.
        parts.push(format!("Bases: {}", summarize_list(base_types)));
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

/// Renders a rule's class list, collapsing any fully-selected group to `Any Weapons`,
/// `Any Armour`, and so on before falling back to [`summarize_list`] for whatever is left over.
///
/// A rule covering every weapon class listed twelve names and truncated to
/// `Bows, Crossbows, Foci, One Hand Maces +8 more`, which says almost nothing (author,
/// 2026-09-08). `Any Weapons` says exactly what the rule does in two words.
///
/// **"Any", not "All".** A summary reads as the rule's own matching test — `Any Jewellery ·
/// Rarity = Normal` is the question asked of each item as it drops, and any one class in the
/// list satisfies it. "Any Jewellery" reads like the item must somehow be all three at once
/// (author, 2026-09-08, reversing the first cut).
///
/// Groups are checked against the config's own class lists, so a group only collapses when every
/// member the config defines is present. Adding a class to `config.json` correctly stops an
/// existing rule from reading as "Any <group>" until that class is selected too.
fn summarize_classes(classes: &[String], groups: &[(&str, &[String])]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut remaining: Vec<String> = classes.to_vec();

    for (group, members) in groups {
        if members.is_empty() {
            continue;
        }
        let fully_selected = members.iter().all(|m| classes.iter().any(|c| c == m));
        if fully_selected {
            parts.push(format!("Any {}", title_case(group)));
            remaining.retain(|c| !members.iter().any(|m| m == c));
        }
    }

    if !remaining.is_empty() {
        parts.push(summarize_list(&remaining));
    }
    parts.join(", ")
}

/// `"FLASKS & CHARMS"` -> `"Flasks & Charms"`. The picker's group labels are shouted to match the
/// game's own section headers; a summary sentence is not.
fn title_case(label: &str) -> String {
    label
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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

    fn summarize_for(classes: &[&str]) -> String {
        let when = When {
            class: Some(classes.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        };
        rule_rows(&with_rules(vec![rule("unchanged", Some(when))]))[0]
            .summary
            .clone()
    }

    #[test]
    fn a_fully_selected_group_collapses_to_any_of_it() {
        let config = base_config();
        let summary = summarize_for(
            &config
                .armour_classes
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
        );
        assert_eq!(summary, "Any Armour", "got: {summary}");
    }

    #[test]
    fn one_missing_class_stops_a_group_collapsing() {
        // The whole point of checking against the config lists: adding a class to config.json
        // must stop an existing rule reading as "Any <group>" until that class is selected too.
        let config = base_config();
        let mut classes: Vec<&str> = config.armour_classes.iter().map(|s| s.as_str()).collect();
        classes.pop();
        let summary = summarize_for(&classes);
        assert!(!summary.contains("Any Armour"), "got: {summary}");
    }

    #[test]
    fn several_groups_collapse_together_and_leftovers_still_list() {
        let config = base_config();
        let mut classes: Vec<&str> = config.armour_classes.iter().map(|s| s.as_str()).collect();
        classes.extend(config.jewellery_classes.iter().map(|s| s.as_str()));
        classes.push("Bows"); // one weapon, so WEAPONS must not collapse
        let summary = summarize_for(&classes);
        assert!(summary.contains("Any Armour"), "got: {summary}");
        assert!(summary.contains("Any Jewellery"), "got: {summary}");
        assert!(summary.contains("Bows"), "got: {summary}");
        assert!(!summary.contains("Any Weapons"), "got: {summary}");
    }

    #[test]
    fn group_labels_are_title_cased_not_shouted() {
        let config = base_config();
        let summary = summarize_for(
            &config
                .flask_charm_classes
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
        );
        assert_eq!(summary, "Any Flasks & Charms", "got: {summary}");
    }

    #[test]
    fn a_class_list_longer_than_four_truncates_with_a_more_suffix() {
        // Six weapon classes out of twelve: enough to truncate, not a whole group, so this
        // still exercises truncation now that a fully-selected group collapses instead.
        let classes = [
            "Bows",
            "Crossbows",
            "Foci",
            "One Hand Maces",
            "Quivers",
            "Quarterstaves",
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
            summary.starts_with("Bows, Crossbows, Foci, One Hand Maces +2 more"),
            "got: {summary}"
        );
        assert!(!summary.contains("Shields"), "a truncated list must not spell out the rest");
    }

    #[test]
    fn base_types_are_labelled_in_the_summary() {
        // D-15: with `class` now optional (Phase 15), a bare name list is ambiguous about which
        // condition it describes -- the explicit `Bases:` label disambiguates it.
        let when = When {
            base_type: Some(vec!["Heavy Belt".to_string(), "Utility Belt".to_string()]),
            ..Default::default()
        };
        let config = with_rules(vec![rule("unchanged", Some(when))]);
        let summary = rule_rows(&config)[0].summary.clone();
        assert!(
            summary.contains("Bases: Heavy Belt, Utility Belt"),
            "got: {summary}"
        );
    }

    #[test]
    fn a_long_base_type_list_is_still_truncated_after_the_label() {
        let names: Vec<String> = [
            "Heavy Belt",
            "Utility Belt",
            "Armoured Cap",
            "Sacred Focus",
            "Leather Belt",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let when = When {
            base_type: Some(names),
            ..Default::default()
        };
        let config = with_rules(vec![rule("unchanged", Some(when))]);
        let summary = rule_rows(&config)[0].summary.clone();
        assert!(
            summary.contains("Bases: Heavy Belt, Utility Belt, Armoured Cap, Sacred Focus +1 more"),
            "got: {summary}"
        );
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
