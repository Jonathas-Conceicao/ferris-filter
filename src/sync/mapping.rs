// Mapping artifact models and utilities.
// `mapping.json` is a committed repo artifact that maps poe.ninja overview types
// to filter category, class, and baseType rules. `sync metadata` refreshes it;
// `sync prices` reads it.

use serde::{Deserialize, Serialize};

/// Top-level mapping.json structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    pub version: u32,
    pub overview_types: Vec<OverviewType>,
    pub gem_filter: GemFilter,
}

/// One poe.ninja overview type → filter category/class entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewType {
    /// The poe.ninja `type=` query parameter value (e.g. "Currency").
    pub r#type: String,
    /// The PriceItem.category value to emit in prices.json.
    pub category: String,
    /// The filter `Class ==` value (e.g. "Stackable Currency"). Null for uniques
    /// (class determined per-item at prices time).
    pub class: Option<String>,
    /// If true, `baseType = name` (applies to all currency-style types).
    pub base_type_is_name: bool,
}

/// Special gem filter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GemFilter {
    /// poe.ninja type parameter for gems.
    pub r#type: String,
    /// PriceItem.category value.
    pub category: String,
    /// Filter Class== value.
    pub class: String,
    /// Minimum gem level to include (items below this level are excluded).
    pub min_level: u32,
}

impl Mapping {
    /// Look up the OverviewType entry for the given poe.ninja type string.
    pub fn lookup(&self, type_: &str) -> Option<&OverviewType> {
        self.overview_types.iter().find(|t| t.r#type == type_)
    }
}

/// Load mapping.json from the given path.
pub fn load_mapping(path: &str) -> miette::Result<Mapping> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| miette::miette!("Cannot read {path}: {e}"))?;
    serde_json::from_str(&content)
        .map_err(|e| miette::miette!("Cannot parse {path}: {e}"))
}

/// Normalize a RePoE PascalCase `item_class` value to the filter's `Class==` spaced form.
///
/// Algorithm: insert a space before each interior uppercase letter, then apply the
/// override table for known filter spellings confirmed against CLAUDE.md class list.
///
/// # Examples
/// ```
/// use ferrisfilter::sync::mapping::normalize_item_class;
/// assert_eq!(normalize_item_class("StackableCurrency"), "Stackable Currency");
/// assert_eq!(normalize_item_class("BodyArmour"), "Body Armours");
/// assert_eq!(normalize_item_class("Amulet"), "Amulets");
/// ```
pub fn normalize_item_class(raw: &str) -> String {
    // Override table: RePoE PascalCase → exact filter Class== spelling from CLAUDE.md.
    // Covers cases where simple space-insertion doesn't produce the right result,
    // and pluralization differences between RePoE and the filter class list.
    let overrides: &[(&str, &str)] = &[
        // Armour slots — RePoE uses "Armour" singular, filter uses "Armours" plural
        ("BodyArmour", "Body Armours"),
        // Head/foot/hand slots
        ("Helmet", "Helmets"),
        ("Gloves", "Gloves"),
        ("Boots", "Boots"),
        // Weapon classes
        ("Bow", "Bows"),
        ("Crossbow", "Crossbows"),
        ("Quarterstaff", "Quarterstaves"),
        ("Spear", "Spears"),
        ("Wand", "Wands"),
        ("Sceptre", "Sceptres"),
        ("Staff", "Staves"),
        ("Focus", "Foci"),
        ("OneHandMace", "One Hand Maces"),
        ("TwoHandMace", "Two Hand Maces"),
        // Off-hands
        ("Quiver", "Quivers"),
        ("Shield", "Shields"),
        ("Buckler", "Bucklers"),
        // Jewellery
        ("Ring", "Rings"),
        ("Amulet", "Amulets"),
        ("Belt", "Belts"),
        // Currency/special
        ("StackableCurrency", "Stackable Currency"),
        ("Talisman", "Talismans"),
        ("Jewel", "Jewels"),
    ];

    // Check override table first (exact match on raw PascalCase).
    for (pascal, spaced) in overrides {
        if raw == *pascal {
            return spaced.to_string();
        }
    }

    // Fallback: insert space before each interior uppercase letter.
    let mut result = String::with_capacity(raw.len() + 4);
    let chars: Vec<char> = raw.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if i > 0 && c.is_uppercase() {
            result.push(' ');
        }
        result.push(c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- normalize_item_class tests ---

    #[test]
    fn normalize_stackable_currency() {
        assert_eq!(normalize_item_class("StackableCurrency"), "Stackable Currency");
    }

    #[test]
    fn normalize_body_armour() {
        // RePoE says "BodyArmour"; filter says "Body Armours"
        assert_eq!(normalize_item_class("BodyArmour"), "Body Armours");
    }

    #[test]
    fn normalize_single_word_unchanged() {
        // Single-word classes pass through (amulet → "Amulets" via override table)
        assert_eq!(normalize_item_class("Amulet"), "Amulets");
    }

    #[test]
    fn normalize_one_hand_mace() {
        assert_eq!(normalize_item_class("OneHandMace"), "One Hand Maces");
    }

    #[test]
    fn normalize_two_hand_mace() {
        assert_eq!(normalize_item_class("TwoHandMace"), "Two Hand Maces");
    }

    #[test]
    fn normalize_fallback_inserts_spaces() {
        // A class not in the override table gets simple space insertion.
        assert_eq!(normalize_item_class("SomeNewClass"), "Some New Class");
    }

    // --- mapping.json parse + lookup tests ---

    const MAPPING_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/mapping.json"));

    #[test]
    fn mapping_json_parses() {
        let m: Mapping = serde_json::from_str(MAPPING_JSON)
            .expect("mapping.json should parse into Mapping struct");
        assert!(!m.overview_types.is_empty(), "overviewTypes must be non-empty");
        assert_eq!(m.version, 1);
    }

    #[test]
    fn mapping_lookup_currency() {
        let m: Mapping = serde_json::from_str(MAPPING_JSON).unwrap();
        let entry = m.lookup("Currency").expect("Currency type must be present");
        assert_eq!(entry.category, "Currency");
        assert_eq!(entry.class.as_deref(), Some("Stackable Currency"));
        assert!(entry.base_type_is_name, "Currency should have baseTypeIsName=true");
    }

    #[test]
    fn mapping_contains_all_unique_types() {
        let m: Mapping = serde_json::from_str(MAPPING_JSON).unwrap();
        let unique_types = [
            "UniqueWeapon", "UniqueArmour", "UniqueAccessory", "UniqueFlask",
            "UniqueCharm", "UniqueJewel", "UniqueMap", "SanctumRelic",
        ];
        for t in unique_types {
            assert!(
                m.lookup(t).is_some(),
                "mapping.json must contain Unique type: {t}"
            );
        }
    }

    #[test]
    fn mapping_contains_uncut_gem() {
        let m: Mapping = serde_json::from_str(MAPPING_JSON).unwrap();
        assert_eq!(m.gem_filter.r#type, "UncutGem");
    }

    #[test]
    fn mapping_covers_currency_json_categories() {
        // Ensure every distinct category from currency.json is covered by some overviewType.
        // These are the categories in currency.json that have confirmed poe.ninja type mappings.
        let m: Mapping = serde_json::from_str(MAPPING_JSON).unwrap();
        let categories: Vec<&str> = m.overview_types.iter().map(|t| t.category.as_str()).collect();

        // Categories that must appear in mapping (from currency.json):
        let required_categories = [
            "Currency",
            "Breach",
            "Delirium",
            "Essences",
            "Expedition",
            "Omens",
            "Runes",
            "Soul Cores",
        ];
        for cat in required_categories {
            assert!(
                categories.contains(&cat),
                "mapping.json must cover currency.json category: {cat}"
            );
        }
    }
}
