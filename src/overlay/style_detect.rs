//! Auto Detect, the pure planner: which untagged rules already look exactly like a style, which
//! new `Custom Style N` styles the rest need, and the status line. No UI, no document access:
//! the caller hands in each candidate rule's body entries and applies the returned plan.
//!
//! Equality is on in-game values (a 3-value background is alpha 240, a missing volume is the
//! default one, see [`StyleView::to_style`]), over every parameter except the id. A rule with an
//! action line no style line shows exactly (see `body::style_leftovers`) or a `Minimal` header
//! stays Custom and is not planned.

use crate::filter::{BlockId, BlockKeyword};
use crate::models::config::{Style, StyleSet};

use super::body;
use super::rules::BodyEntry;

/// Status line for a run that changed nothing.
pub const NOTHING_NEW: &str = "Auto Detect: nothing new";

/// What one Auto Detect click would do.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DetectPlan {
    /// New styles to add, in order of first appearance (the id is assigned on insert).
    pub new_styles: Vec<(String, Style)>,
    /// Every rule to tag, with the name of the style it gets (existing or planned).
    pub tags: Vec<(BlockId, String)>,
}

impl DetectPlan {
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}

/// Style equality ignoring the id (`Style`'s derived `PartialEq` includes it).
pub(crate) fn same_style(a: &Style, b: &Style) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.id = None;
    b.id = None;
    a == b
}

/// The first `Custom Style N` (N from 1) no taken name equals, ignoring case.
pub(crate) fn next_name(taken: &[String]) -> String {
    (1..)
        .map(|n| format!("Custom Style {n}"))
        .find(|c| !taken.iter().any(|t| t.eq_ignore_ascii_case(c)))
        .expect("an unbounded range has a free name")
}

/// Plan the run over `(rule, body entries, header keyword)` candidates, in file order. Match
/// order: the built-ins (with their pending values), then the custom styles, then the styles
/// already planned in this run.
pub fn plan(inputs: &[(BlockId, Vec<BodyEntry>, BlockKeyword)], styles: &StyleSet) -> DetectPlan {
    let mut out = DetectPlan::default();
    let builtins = styles.iter().filter(|(n, _)| StyleSet::is_builtin(n));
    let customs = styles.iter().filter(|(n, _)| !StyleSet::is_builtin(n));
    let existing: Vec<(&str, &Style)> = builtins.chain(customs).collect();
    let mut taken: Vec<String> = styles.names().map(str::to_string).collect();

    for (id, entries, keyword) in inputs {
        let (view, binding) = body::rule_style_view(entries, *keyword);
        if !body::style_leftovers(entries, &binding).is_empty() {
            continue;
        }
        let Some(style) = view.to_style() else {
            continue;
        };
        let found = existing
            .iter()
            .find(|(_, s)| same_style(s, &style))
            .map(|(n, _)| n.to_string())
            .or_else(|| {
                out.new_styles
                    .iter()
                    .find(|(_, s)| same_style(s, &style))
                    .map(|(n, _)| n.clone())
            });
        let name = match found {
            Some(n) => n,
            None => {
                let n = next_name(&taken);
                taken.push(n.clone());
                out.new_styles.push((n.clone(), style));
                n
            }
        };
        out.tags.push((*id, name));
    }
    out
}

/// `Auto Detect: 3 styles, 41 rules tagged`.
pub fn status_text(new_styles: usize, rules_tagged: usize) -> String {
    let s = if new_styles == 1 { "style" } else { "styles" };
    let r = if rules_tagged == 1 { "rule" } else { "rules" };
    format!("Auto Detect: {new_styles} {s}, {rules_tagged} {r} tagged")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::rules::WorkingFilter;

    fn inputs(text: &str) -> Vec<(BlockId, Vec<BodyEntry>, BlockKeyword)> {
        let w = WorkingFilter::from_text(text);
        w.rows()
            .iter()
            .map(|r| (r.id, w.body_view(r.id), r.keyword))
            .collect()
    }

    fn run_with(text: &str, set: &StyleSet) -> DetectPlan {
        plan(&inputs(text), set)
    }

    fn run(text: &str) -> DetectPlan {
        run_with(text, StyleSet::shipped_defaults())
    }

    /// One rule from a header and its body lines.
    fn rule(header: &str, lines: &str) -> String {
        let mut s = format!("{header}\n");
        for l in lines.split(';').map(str::trim).filter(|l| !l.is_empty()) {
            s.push_str(&format!("\t{l}\n"));
        }
        s
    }

    #[derive(Debug)]
    enum Want {
        Tag(&'static str),
        New,
        Skip,
    }

    #[test]
    fn equality_table() {
        use Want::*;
        let rows: &[(&str, &str, Want)] = &[
            ("Show", "", Tag("unchanged")),
            ("Hide", "", New),
            ("Hide", "SetFontSize 21; DisableDropSound", Tag("hidden")),
            ("Show", "SetFontSize 20 # note", New),
            ("Show", "DisableDropSound False", Skip),
            ("Show", "SetFontSize 20; SetFontSize 21", Skip),
            ("Show", "CustomAlertSoundOptional \"x.mp3\"", Skip),
            ("Show", "EnableDropSoundIfAlertSound", Skip),
            ("Show", "SetFontSize 60", Skip),
            ("Show", "PlayAlertSound 6 400", Skip),
            ("Show", "MinimapIcon 3 Red Circle", Skip),
            ("Minimal", "", Skip),
        ];
        for (header, lines, want) in rows {
            let p = run(&rule(header, lines));
            match want {
                Tag(n) => {
                    assert_eq!(p.tags.len(), 1, "{header} {lines}");
                    assert_eq!(p.tags[0].1, *n, "{header} {lines}");
                    assert!(p.new_styles.is_empty(), "{header} {lines}");
                }
                New => {
                    assert_eq!(p.tags.len(), 1, "{header} {lines}");
                    assert_eq!(p.new_styles.len(), 1, "{header} {lines}");
                    assert_eq!(p.tags[0].1, p.new_styles[0].0);
                }
                Skip => assert!(p.is_empty(), "{header} {lines}: {p:?}"),
            }
        }
        assert!(run(&rule("Show", "SetFontSize 60")).is_empty());
    }

    /// Pairs of bodies that must (or must not) end up as one planned style.
    #[test]
    fn same_and_different_pairs() {
        let same = [
            ("SetBackgroundColor 1 2 3", "SetBackgroundColor 1 2 3 240"),
            ("SetTextColor 1 2 3", "SetTextColor 1 2 3 255"),
            ("PlayAlertSound 6", "PlayAlertSound 6 50"),
            (
                "CustomAlertSound \"a.mp3\"",
                "CustomAlertSound \"a.mp3\" 100",
            ),
            ("DisableDropSound", "DisableDropSound True"),
            ("SetFontSize 20", "SetFontSize 20 # note"),
        ];
        for (a, b) in same {
            let text = format!("{}\n{}", rule("Show", a), rule("Show", b));
            let p = run(&text);
            assert_eq!(p.tags.len(), 2, "{a} / {b}");
            assert_eq!(p.new_styles.len(), 1, "{a} / {b}");
        }
        let different = [
            ("SetBackgroundColor 1 2 3", "SetBackgroundColor 1 2 3 255"),
            ("PlayAlertSound 6", "PlayAlertSound 6 300"),
            ("PlayAlertSound 6", "PlayAlertSoundPositional 6"),
            ("PlayAlertSound None", "SetFontSize 20"),
            ("MinimapIcon -1", "SetFontSize 20"),
            ("SetFontSize 20", "SetFontSize 23"),
            ("DisableDropSound", "EnableDropSound"),
        ];
        for (a, b) in different {
            let text = format!("{}\n{}", rule("Show", a), rule("Show", b));
            let p = run(&text);
            assert_eq!(p.tags.len(), 2, "{a} / {b}");
            assert_eq!(p.new_styles.len(), 2, "{a} / {b}");
        }
        // A cancel value is not the same as no line.
        let p = run(&format!(
            "{}\n{}",
            rule("Show", "PlayAlertSound None"),
            rule("Show", "")
        ));
        assert_eq!(p.new_styles.len(), 1);
        assert_eq!(p.tags[1].1, "unchanged");
    }

    #[test]
    fn an_edited_builtin_matches_its_pending_values() {
        let mut set = StyleSet::shipped_defaults().clone();
        set.get_mut("E").unwrap().font = Some(30);
        let colours = "SetTextColor 241 250 140 255; SetBorderColor 241 250 140 255; \
                       SetBackgroundColor 40 42 54 255";
        let edited = rule("Show", &format!("{colours}; SetFontSize 30"));
        let original = rule("Show", &format!("{colours}; SetFontSize 23"));
        let p = run_with(&format!("{edited}\n{original}"), &set);
        assert_eq!(p.tags[0].1, "E");
        assert_eq!(p.new_styles.len(), 1);
        assert_eq!(p.new_styles[0].1.font, Some(23));
        assert_eq!(p.tags[1].1, p.new_styles[0].0);
    }

    #[test]
    fn builtins_win_over_customs_and_earlier_customs_win() {
        let mut set = StyleSet::shipped_defaults().clone();
        let mut clone = set.get("F").unwrap().clone();
        clone.id = None;
        set.insert_new("Mine", clone.clone()).unwrap();
        set.insert_new("Mine too", clone).unwrap();
        let body = "SetTextColor 248 248 242 255; SetBorderColor 248 248 242 255; \
                    SetBackgroundColor 40 42 54 255; SetFontSize 20";
        let p = run_with(&rule("Show", body), &set);
        assert_eq!(p.tags[0].1, "F");

        let mut set = StyleSet::shipped_defaults().clone();
        let mut custom = Style::blank_show();
        custom.font = Some(40);
        set.insert_new("First", custom.clone()).unwrap();
        set.insert_new("Second", custom).unwrap();
        let p = run_with(&rule("Show", "SetFontSize 40"), &set);
        assert_eq!(p.tags[0].1, "First");
        assert!(p.new_styles.is_empty());
    }

    #[test]
    fn names_take_the_smallest_free_number_in_order_of_appearance() {
        let mut set = StyleSet::shipped_defaults().clone();
        set.insert_new("custom style 1", Style::blank_show())
            .unwrap();
        let mut c3 = Style::blank_show();
        c3.font = Some(44);
        set.insert_new("Custom Style 3", c3).unwrap();
        let text = format!(
            "{}\n{}",
            rule("Show", "SetFontSize 20"),
            rule("Show", "SetFontSize 23")
        );
        let names: Vec<String> = run_with(&text, &set)
            .new_styles
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        // 1 is taken case-insensitively and 3 exactly: the new styles get 2, then 4.
        assert_eq!(names, vec!["Custom Style 2", "Custom Style 4"]);
    }

    #[test]
    fn next_name_is_case_insensitive() {
        let taken = vec!["custom style 1".to_string(), "Custom Style 3".to_string()];
        assert_eq!(next_name(&taken), "Custom Style 2");
        let taken = vec![
            "CUSTOM STYLE 1".to_string(),
            "Custom Style 2".to_string(),
            "Custom Style 3".to_string(),
        ];
        assert_eq!(next_name(&taken), "Custom Style 4");
        assert_eq!(next_name(&[]), "Custom Style 1");
    }

    #[test]
    fn same_style_ignores_the_id() {
        let mut a = Style::blank_show();
        let mut b = Style::blank_show();
        a.id = Some("abc".into());
        b.id = Some("xyz".into());
        assert!(same_style(&a, &b));
        b.font = Some(1);
        assert!(!same_style(&a, &b));
    }

    #[test]
    fn status_texts() {
        assert_eq!(status_text(3, 41), "Auto Detect: 3 styles, 41 rules tagged");
        assert_eq!(status_text(1, 1), "Auto Detect: 1 style, 1 rule tagged");
        assert_eq!(status_text(0, 12), "Auto Detect: 0 styles, 12 rules tagged");
        assert_eq!(NOTHING_NEW, "Auto Detect: nothing new");
    }

    #[test]
    fn nothing_plannable_is_empty() {
        let p = run(&format!(
            "{}\n{}",
            rule("Minimal", ""),
            rule("Show", "SetFontSize 60")
        ));
        assert!(p.is_empty());
        assert!(p.new_styles.is_empty());
    }
}
