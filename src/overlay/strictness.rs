//! Strictness levels: a named preset of which built-in tier styles (S..F) are hidden.
//!
//! Layer rule: no GUI-framework types live here. The level is never stored; it is derived from
//! the Show/Hide actions of `builtin:S`..`builtin:F` in a style set, so any hand-made mix reads as
//! `Custom`.

use crate::models::config::{StyleAction, StyleSet};

/// The seven tier styles, best first.
pub const TIER_LETTERS: [&str; 7] = ["S", "A", "B", "C", "D", "E", "F"];

/// Shared last paragraph of every level tooltip.
const AFFECTS_NOTE: &str = "Affects every rule that uses these tier styles, yours included.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strictness {
    Custom,
    FlatBroke,
    Scrounging,
    GettingBy,
    Comfy,
    Loaded,
    DivFarmer,
}

impl Strictness {
    /// Button order, `Custom` first.
    pub const ALL: [Strictness; 7] = [
        Strictness::Custom,
        Strictness::FlatBroke,
        Strictness::Scrounging,
        Strictness::GettingBy,
        Strictness::Comfy,
        Strictness::Loaded,
        Strictness::DivFarmer,
    ];

    /// Tier letters this level hides. `Custom` is not a preset and hides nothing here.
    pub fn hidden(self) -> &'static [&'static str] {
        match self {
            Strictness::Custom | Strictness::FlatBroke => &[],
            Strictness::Scrounging => &["F"],
            Strictness::GettingBy => &["E", "F"],
            Strictness::Comfy => &["D", "E", "F"],
            Strictness::Loaded => &["C", "D", "E", "F"],
            Strictness::DivFarmer => &["B", "C", "D", "E", "F"],
        }
    }

    /// Caption lines of the button (two words are stacked).
    pub fn caption_lines(self) -> &'static [&'static str] {
        match self {
            Strictness::Custom => &["Custom"],
            Strictness::FlatBroke => &["Flat", "Broke"],
            Strictness::Scrounging => &["Scrounging"],
            Strictness::GettingBy => &["Getting", "By"],
            Strictness::Comfy => &["Comfy"],
            Strictness::Loaded => &["Loaded"],
            Strictness::DivFarmer => &["Div", "Farmer"],
        }
    }

    pub fn tooltip(self) -> String {
        let first = match self {
            Strictness::Custom => {
                return "Your own mix of shown and hidden tiers. Pick a level to replace it."
                    .to_string();
            }
            Strictness::FlatBroke => "Hides nothing. Every tier shows.",
            Strictness::Scrounging => "Hides tier F.",
            Strictness::GettingBy => "Hides tiers E and F.",
            Strictness::Comfy => "Hides tiers D to F.",
            Strictness::Loaded => "Hides tiers C to F.",
            Strictness::DivFarmer => "Hides tiers B to F.",
        };
        format!("{first}\n\n{AFFECTS_NOTE}")
    }
}

/// The level a style set currently reads as. The seven tier styles are found by their built-in
/// id; a missing one, or any pattern that is not a preset, is `Custom`.
pub fn strictness_of(set: &StyleSet) -> Strictness {
    let mut hidden: Vec<&str> = Vec::new();
    for letter in TIER_LETTERS {
        let Some(name) = set.name_of_id(&StyleSet::builtin_id(letter)) else {
            return Strictness::Custom;
        };
        let Some(style) = set.get(name) else {
            return Strictness::Custom;
        };
        if style.action == StyleAction::Hide {
            hidden.push(letter);
        }
    }
    Strictness::ALL
        .into_iter()
        .filter(|l| *l != Strictness::Custom)
        .find(|l| l.hidden() == hidden.as_slice())
        .unwrap_or(Strictness::Custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_hidden(letters: &[&str]) -> StyleSet {
        let mut set = StyleSet::shipped_defaults().clone();
        for l in letters {
            set.get_mut(l).unwrap().action = StyleAction::Hide;
        }
        set
    }

    #[test]
    fn shipped_defaults_read_flat_broke() {
        assert_eq!(
            strictness_of(StyleSet::shipped_defaults()),
            Strictness::FlatBroke
        );
    }

    #[test]
    fn every_preset_pattern_is_recognised() {
        for level in Strictness::ALL {
            if level == Strictness::Custom {
                continue;
            }
            assert_eq!(strictness_of(&with_hidden(level.hidden())), level);
        }
    }

    #[test]
    fn other_patterns_are_custom() {
        assert_eq!(strictness_of(&with_hidden(&["S"])), Strictness::Custom);
        assert_eq!(
            strictness_of(&with_hidden(&["A", "B", "C", "D", "E", "F"])),
            Strictness::Custom
        );
        assert_eq!(strictness_of(&with_hidden(&["E"])), Strictness::Custom);
        assert_eq!(
            strictness_of(&with_hidden(&["S", "A", "B", "C", "D", "E", "F"])),
            Strictness::Custom
        );
    }

    #[test]
    fn a_missing_tier_id_is_custom() {
        let mut set = StyleSet::shipped_defaults().clone();
        set.get_mut("F").unwrap().id = Some("zzzzzz".to_string());
        assert_eq!(strictness_of(&set), Strictness::Custom);
    }

    #[test]
    fn custom_hides_nothing_and_texts_match_the_spec() {
        assert!(Strictness::Custom.hidden().is_empty());
        assert_eq!(
            Strictness::Custom.tooltip(),
            "Your own mix of shown and hidden tiers. Pick a level to replace it."
        );
        assert_eq!(
            Strictness::GettingBy.tooltip(),
            "Hides tiers E and F.\n\nAffects every rule that uses these tier styles, yours included."
        );
        assert_eq!(Strictness::DivFarmer.caption_lines(), ["Div", "Farmer"]);
        assert_eq!(Strictness::ALL[0], Strictness::Custom);
    }
}
