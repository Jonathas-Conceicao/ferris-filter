//! The single keyword table for the PoE2 item-filter grammar.
//!
//! This table is the one source of truth for every block type, condition and action. It is
//! checked against <https://www.pathofexile.com/item-filter/about>. The FilterBlade sample is
//! test material, not a grammar source. Word lists are duplicated here on purpose; parsing must
//! not depend on `models::config` enums. PoE1-only mechanics (links, influences, Ward, ...) are
//! excluded; a filter line using one is unknown, kept opaque and warned about, never rewritten.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    BlockHeader,
    Condition,
    Action,
    /// `Continue`.
    Modifier,
    /// `Import`.
    Directive,
}

/// Operator policy. `Compare` permits any of the eight operators (or none); the parser records the
/// operator verbatim and never rejects one by policy on a condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ops {
    None,
    Compare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueShape {
    NoValue,
    Bool,
    Int,
    QuotedList,
    Enum(&'static [&'static str]),
    Rarity,
    Sockets,
    CountThenNames,
    BoolOrName,
    /// Zero values, or a single True/False (real filters write `DisableDropSound True`).
    OptionalBool,
    Rgba,
    SoundId,
    CustomSound,
    MinimapIcon,
    PlayEffect,
    FontSize,
    ImportPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keyword {
    pub name: &'static str,
    pub kind: Kind,
    pub ops: Ops,
    pub value: ValueShape,
}

pub static COLOURS: &[&str] = &[
    "Red", "Green", "Blue", "Brown", "White", "Yellow", "Cyan", "Grey", "Orange", "Pink", "Purple",
];
pub static SHAPES: &[&str] = &[
    "Circle",
    "Diamond",
    "Hexagon",
    "Square",
    "Star",
    "Triangle",
    "Cross",
    "Moon",
    "Raindrop",
    "Kite",
    "Pentagon",
    "UpsideDownHouse",
];
pub static RARITIES: &[&str] = &["Normal", "Magic", "Rare", "Unique"];

#[rustfmt::skip]
pub static KEYWORDS: &[Keyword] = &[
    Keyword { name: "Show", kind: Kind::BlockHeader, ops: Ops::None, value: ValueShape::NoValue },
    Keyword { name: "Hide", kind: Kind::BlockHeader, ops: Ops::None, value: ValueShape::NoValue },
    Keyword { name: "Minimal", kind: Kind::BlockHeader, ops: Ops::None, value: ValueShape::NoValue },
    Keyword { name: "Continue", kind: Kind::Modifier, ops: Ops::None, value: ValueShape::NoValue },
    Keyword { name: "Import", kind: Kind::Directive, ops: Ops::None, value: ValueShape::ImportPath },
    Keyword { name: "AlwaysShow", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "AnyEnchantment", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "Corrupted", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "Foulborn", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "FracturedItem", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "HasImplicitMod", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "HasVaalUniqueMod", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "Identified", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "Imbued", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "IsVaalUnique", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "MirageMap", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "Mirrored", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "TwiceCorrupted", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "Vestigial", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Bool },
    Keyword { name: "AreaLevel", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "BaseArmour", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "BaseEnergyShield", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "BaseEvasion", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "CorruptedMods", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "DropLevel", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "GemLevel", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "Height", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "ItemLevel", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "Quality", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "StackSize", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "UnidentifiedItemTier", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "WaystoneTier", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "Width", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Int },
    Keyword { name: "BaseType", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::QuotedList },
    Keyword { name: "Class", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::QuotedList },
    Keyword { name: "HasEnchantment", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::CountThenNames },
    Keyword { name: "HasExplicitMod", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::CountThenNames },
    Keyword { name: "Rarity", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Rarity },
    Keyword { name: "Sockets", kind: Kind::Condition, ops: Ops::Compare, value: ValueShape::Sockets },
    Keyword { name: "PlayAlertSound", kind: Kind::Action, ops: Ops::None, value: ValueShape::SoundId },
    Keyword { name: "PlayAlertSoundPositional", kind: Kind::Action, ops: Ops::None, value: ValueShape::SoundId },
    Keyword { name: "CustomAlertSound", kind: Kind::Action, ops: Ops::None, value: ValueShape::CustomSound },
    Keyword { name: "CustomAlertSoundOptional", kind: Kind::Action, ops: Ops::None, value: ValueShape::CustomSound },
    Keyword { name: "DisableDropSound", kind: Kind::Action, ops: Ops::None, value: ValueShape::OptionalBool },
    Keyword { name: "EnableDropSound", kind: Kind::Action, ops: Ops::None, value: ValueShape::OptionalBool },
    Keyword { name: "DisableDropSoundIfAlertSound", kind: Kind::Action, ops: Ops::None, value: ValueShape::OptionalBool },
    Keyword { name: "EnableDropSoundIfAlertSound", kind: Kind::Action, ops: Ops::None, value: ValueShape::OptionalBool },
    Keyword { name: "MinimapIcon", kind: Kind::Action, ops: Ops::None, value: ValueShape::MinimapIcon },
    Keyword { name: "PlayEffect", kind: Kind::Action, ops: Ops::None, value: ValueShape::PlayEffect },
    Keyword { name: "SetBackgroundColor", kind: Kind::Action, ops: Ops::None, value: ValueShape::Rgba },
    Keyword { name: "SetBorderColor", kind: Kind::Action, ops: Ops::None, value: ValueShape::Rgba },
    Keyword { name: "SetTextColor", kind: Kind::Action, ops: Ops::None, value: ValueShape::Rgba },
    Keyword { name: "SetFontSize", kind: Kind::Action, ops: Ops::None, value: ValueShape::FontSize },
];

/// Case-insensitive lookup. `None` means the keyword is unknown; callers keep the line opaque.
pub fn lookup(name: &str) -> Option<&'static Keyword> {
    KEYWORDS.iter().find(|k| k.name.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_unique_case_insensitive() {
        for (i, a) in KEYWORDS.iter().enumerate() {
            for b in &KEYWORDS[i + 1..] {
                assert!(!a.name.eq_ignore_ascii_case(b.name), "duplicate {}", a.name);
            }
        }
    }

    #[test]
    fn lookup_is_case_insensitive() {
        for n in ["rarity", "RARITY", "Rarity"] {
            assert_eq!(lookup(n).unwrap().name, "Rarity");
        }
    }

    #[test]
    fn block_types_modifier_and_directive_present() {
        assert_eq!(lookup("Continue").unwrap().kind, Kind::Modifier);
        assert_eq!(lookup("Import").unwrap().kind, Kind::Directive);
        for n in ["Show", "Hide", "Minimal"] {
            assert_eq!(lookup(n).unwrap().kind, Kind::BlockHeader);
        }
    }

    #[test]
    fn documented_vocabulary_present() {
        let conditions = "AlwaysShow AnyEnchantment Corrupted Foulborn FracturedItem \
            HasImplicitMod HasVaalUniqueMod Identified Imbued IsVaalUnique MirageMap Mirrored \
            TwiceCorrupted Vestigial AreaLevel BaseArmour BaseEnergyShield \
            BaseEvasion CorruptedMods DropLevel GemLevel Height ItemLevel Quality StackSize \
            UnidentifiedItemTier WaystoneTier Width BaseType Class HasEnchantment HasExplicitMod \
            Rarity Sockets";
        for n in conditions.split_whitespace() {
            assert_eq!(
                lookup(n).unwrap_or_else(|| panic!("missing {n}")).kind,
                Kind::Condition,
                "{n}"
            );
        }
        let actions = "PlayAlertSound PlayAlertSoundPositional CustomAlertSound CustomAlertSoundOptional \
            DisableDropSound EnableDropSound DisableDropSoundIfAlertSound EnableDropSoundIfAlertSound \
            MinimapIcon PlayEffect SetBackgroundColor SetBorderColor SetTextColor SetFontSize";
        for n in actions.split_whitespace() {
            assert_eq!(
                lookup(n).unwrap_or_else(|| panic!("missing {n}")).kind,
                Kind::Action,
                "{n}"
            );
        }
        assert_eq!(KEYWORDS.len(), 53);
    }

    #[test]
    fn poe1_only_conditions_are_unknown() {
        for n in [
            "LinkedSockets",
            "SocketGroup",
            "ShaperItem",
            "ElderItem",
            "SynthesisedItem",
            "HasInfluence",
            "ElderMap",
            "ShapedMap",
            "BlightedMap",
            "UberBlightedMap",
            "MapTier",
            "Scourged",
            "Replica",
            "HasSearingExarchImplicit",
            "HasEaterOfWorldsImplicit",
            "EnchantmentPassiveNode",
            "EnchantmentPassiveNum",
            "MemoryStrands",
            "AlternateQuality",
            "GemQualityType",
            "TransfiguredGem",
            "ArchnemesisMod",
            "HasCruciblePassiveTree",
            "ZanaMemory",
            "BaseWard",
            "BaseDefencePercentile",
            "Exceptional",
        ] {
            assert!(lookup(n).is_none(), "{n} should be unknown");
        }
        for n in ["MirageMap", "Minimal"] {
            assert!(lookup(n).is_some(), "{n} should stay");
        }
    }

    #[test]
    fn unknown_keyword_is_none() {
        assert!(lookup("FutureKeyword").is_none());
    }

    #[test]
    fn filterblade_keywords_subset_of_table() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/parser/filterblade.filter"
        ));
        let mut misses = std::collections::BTreeSet::new();
        for line in src.split('\n') {
            let t = line.trim_start();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            let word: String = t.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
            if word.is_empty() {
                continue;
            }
            if lookup(&word).is_none() {
                misses.insert(word);
            }
        }
        assert!(misses.is_empty(), "keywords missing from table: {misses:?}");
    }
}
