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
    /// Unique categories fetched from the `stash/current/item/overview` endpoint
    /// (distinct from the currency `exchange` overview). Defaults to empty for
    /// backward compatibility with older mapping.json files.
    #[serde(default)]
    pub item_types: Vec<ItemType>,
    /// Gem fetch/emit configurations. Replaces the singular `gemFilter` (schema v2).
    pub gem_filters: Vec<GemFilter>,
    /// Per-base-type class corrections, keyed by base type. Defaults to empty so every existing
    /// mapping.json still parses. See [`ClassOverride`].
    #[serde(default)]
    pub class_overrides: std::collections::HashMap<String, ClassOverride>,
}

/// One poe.ninja stash-item overview type (uniques) → filter category.
///
/// These are fetched from `.../economy/stash/current/item/overview?type=<plural>`
/// (e.g. `UniqueWeapons`). Each response line already carries `baseType`, so unlike
/// currency types there is no `baseTypeIsName`/class rule — the base type comes from
/// the data and the filter block matches `Rarity == Unique` + `BaseType ==`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemType {
    /// The poe.ninja `type=` value on the stash/item endpoint (e.g. "UniqueWeapons").
    pub r#type: String,
    /// The PriceItem.category value to emit in prices.json (e.g. "Uniques").
    pub category: String,
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

/// Per-category gem fetch/emit configuration (schema v2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GemFilter {
    /// poe.ninja `type=` value (e.g. "UncutGems", "LineageSupportGems").
    pub r#type: String,
    /// PriceItem.category value (e.g. "Uncut Gems", "Lineage Support Gems").
    pub category: String,
    /// Filter `Class ==` value(s). One entry for LineageSupportGems ("Support Gems");
    /// for UncutGems, all three subtype plural classes (Uncut Skill Gems, Uncut Spirit
    /// Gems, Uncut Support Gems) so the emitted block matches the multi-class line.
    /// The per-subtype Class is derived from each base-type prefix at build time (Plan 02).
    pub classes: Vec<String>,
    /// Per-category fixed background `[R, G, B]`. STORED for Phase 9 — Phase 8 does NOT
    /// emit it. `None` means no background override.
    #[serde(default)]
    pub background_override: Option<[u8; 3]>,
}

/// A per-base-type correction to the class (and optionally category) an overview type would
/// otherwise assign.
///
/// poe.ninja's overview types are merchandising buckets, not item classes: the `Currency` feed
/// carries anything a player trades as currency, whatever its actual `Class`. `mapping.json`
/// gives each overview type a single `class`, which is right for the overwhelming majority and
/// wrong for the handful of items that merely *sit* in that bucket. A rule emitted with the wrong
/// `Class ==` never matches, and nothing warns — the item simply drops unstyled.
///
/// This is the data-driven form of the same correction [`super::prices::derive_expedition_class`]
/// already hard-codes for the Expedition feed, which mixes three classes under one type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassOverride {
    /// The filter `Class ==` value this base type actually needs.
    pub class: String,
    /// Optionally also move it to a different emitted category/section. When absent the item
    /// keeps the category its overview type assigned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Why this override exists — kept in the data so a future reader can re-verify it against
    /// the source rather than trusting it blindly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Mapping {
    /// Look up the OverviewType entry for the given poe.ninja type string.
    pub fn lookup(&self, type_: &str) -> Option<&OverviewType> {
        self.overview_types.iter().find(|t| t.r#type == type_)
    }

    /// The class correction for a base type, if one is recorded.
    pub fn class_override(&self, base_type: &str) -> Option<&ClassOverride> {
        self.class_overrides.get(base_type)
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
        assert_eq!(m.version, 2);
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
    fn mapping_contains_all_unique_item_types() {
        let m: Mapping = serde_json::from_str(MAPPING_JSON).unwrap();
        // Uniques are fetched from the stash/item endpoint with PLURAL type values.
        // NOTE: "UniqueMaps" is intentionally absent — it is a PoE1-only concept (PoE2 uses
        // Waystones, not maps) and the poe.ninja PoE2 endpoint returns HTTP 404 for it.
        let unique_types = [
            "UniqueWeapons", "UniqueArmours", "UniqueAccessories", "UniqueFlasks",
            "UniqueCharms", "UniqueJewels", "UniqueSanctumRelics",
        ];
        for t in unique_types {
            assert!(
                m.item_types.iter().any(|it| it.r#type == t),
                "mapping.json itemTypes must contain Unique type: {t}"
            );
            assert!(
                m.item_types.iter().find(|it| it.r#type == t).unwrap().category == "Uniques",
                "{t} must map to category 'Uniques'"
            );
        }
        assert_eq!(m.item_types.len(), 7, "expected exactly 7 unique item types");
        assert!(
            !m.item_types.iter().any(|it| it.r#type == "UniqueMaps"),
            "UniqueMaps is PoE1-only (PoE2 has no unique maps) — must not be queried"
        );
    }

    #[test]
    fn unique_types_not_in_overview_types() {
        // The unique types must NOT be in overviewTypes (currency exchange endpoint) —
        // they were always empty there. They belong in itemTypes (stash endpoint).
        let m: Mapping = serde_json::from_str(MAPPING_JSON).unwrap();
        for ot in &m.overview_types {
            assert!(
                !ot.r#type.starts_with("Unique") && ot.r#type != "SanctumRelic",
                "overviewTypes must not contain unique type '{}' (moved to itemTypes)",
                ot.r#type
            );
        }
    }

    #[test]
    fn overview_types_use_correct_plural_spellings() {
        // poe.ninja's exchange `type=` values for these are PLURAL — the singular forms
        // return empty (verified live against "Fate of the Vaal"). This guards against
        // regressing to the singular spellings that silently fetch nothing.
        let m: Mapping = serde_json::from_str(MAPPING_JSON).unwrap();
        let types: Vec<&str> = m.overview_types.iter().map(|t| t.r#type.as_str()).collect();
        for t in ["Essences", "Runes", "SoulCores", "Fragments", "Idols", "Verisium"] {
            assert!(types.contains(&t), "overviewTypes must use plural type '{t}'");
        }
        for bad in ["Essence", "Rune", "SoulCore", "Fragment"] {
            assert!(
                !types.contains(&bad),
                "'{bad}' is the wrong singular spelling (returns empty) — use the plural form"
            );
        }
        // Guard: gemFilters must contain both gem categories with correct plural spellings.
        // Silent-empty-fetch pitfall: wrong type= value returns empty from poe.ninja (D-CTX-5).
        assert!(
            m.gem_filters.iter().any(|g| g.r#type == "LineageSupportGems"),
            "gemFilters must contain LineageSupportGems (silent-empty-fetch guard)"
        );
        assert!(
            m.gem_filters.iter().any(|g| g.r#type == "UncutGems"),
            "gemFilters must contain UncutGems"
        );
        for bad in ["LineageSupportGem"] {
            assert!(
                !m.gem_filters.iter().any(|g| g.r#type == bad),
                "'{bad}' is the wrong singular spelling for the Lineage gem type"
            );
        }
    }

    #[test]
    fn mapping_contains_uncut_gem() {
        let m: Mapping = serde_json::from_str(MAPPING_JSON).unwrap();
        assert!(
            m.gem_filters.iter().any(|g| g.r#type == "UncutGems"),
            "gemFilters must contain an UncutGems entry"
        );
    }

    #[test]
    fn mapping_covers_currency_json_categories() {
        // Ensure every category we emit blocks for is covered by some overviewType. The list
        // below was lifted from the v0.1 `currency.json` tier table (retired 2026-09-16 with the
        // `seed_prices` migration binary that was its only reader); it is kept as a literal
        // because it is an assertion about mapping.json, not about that file.
        let m: Mapping = serde_json::from_str(MAPPING_JSON).unwrap();
        let categories: Vec<&str> = m.overview_types.iter().map(|t| t.category.as_str()).collect();

        // Categories that must appear in mapping:
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
                "mapping.json must cover category: {cat}"
            );
        }
    }
}
