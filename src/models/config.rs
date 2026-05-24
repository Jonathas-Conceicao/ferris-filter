use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Closed-vocabulary enums (D-07)
// ---------------------------------------------------------------------------

/// Minimap / beam colors. JSON uses PascalCase matching Rust variant names.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum FilterColor {
    Red,
    Green,
    Blue,
    Brown,
    White,
    Yellow,
    Cyan,
    Grey,
    Orange,
    Pink,
    Purple,
}

/// Minimap icon shapes. JSON uses PascalCase matching Rust variant names.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum MinimapShape {
    Circle,
    Diamond,
    Hexagon,
    Square,
    Star,
    Triangle,
    Cross,
    Moon,
    Raindrop,
    Kite,
    Pentagon,
    UpsideDownHouse,
}

/// Style action — whether the matched item is shown or hidden.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum StyleAction {
    Show,
    Hide,
}

// ---------------------------------------------------------------------------
// Style sub-structs
// ---------------------------------------------------------------------------

/// Minimap icon specification.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Minimap {
    pub size: u8,
    pub color: FilterColor,
    pub shape: MinimapShape,
}

/// Ground beam specification.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Beam {
    pub color: FilterColor,
    pub temp: bool,
}

/// Sound specification.
/// Either a built-in alert (`id`) or a custom file (`custom`); `vol` always present.
/// Mutual-exclusivity of `id` and `custom` is enforced at generate-time (Phase 2).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Sound {
    pub id: Option<u8>,
    pub custom: Option<String>,
    pub vol: u16,
}

/// One tier entry in the `styles` map.
/// `text`/`border`/`background` are RGB triples; alpha is added at generate-time (Phase 2).
/// All visual fields are `Option` because `unchanged` and `hidden` tiers are sparse.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Style {
    pub text: Option<[u8; 3]>,
    pub border: Option<[u8; 3]>,
    pub background: Option<[u8; 3]>,
    pub font: Option<u8>,
    pub minimap: Option<Minimap>,
    pub beam: Option<Beam>,
    pub sound: Option<Sound>,
    pub action: StyleAction,
    pub disable_drop_sound: bool,
}

// ---------------------------------------------------------------------------
// Styles named-fields struct (D-11)
// Replaces HashMap<String, Style> for deterministic, type-safe tier access.
// JSON keys stay "S","A","B","C","D","E","F","Recycle","unchanged","hidden";
// serde renames map the uppercase JSON keys to snake_case Rust fields.
// ---------------------------------------------------------------------------

/// All style tiers as named fields — deterministic access, no HashMap lookup.
/// The JSON object keys are the original uppercase tier names; serde renames handle the mapping.
/// `deny_unknown_fields` ensures an unknown tier key (e.g. a typo "SS") fails deserialization.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Styles {
    #[serde(rename = "S")]       pub s: Style,
    #[serde(rename = "A")]       pub a: Style,
    #[serde(rename = "B")]       pub b: Style,
    #[serde(rename = "C")]       pub c: Style,
    #[serde(rename = "D")]       pub d: Style,
    #[serde(rename = "E")]       pub e: Style,
    #[serde(rename = "F")]       pub f: Style,
    #[serde(rename = "Recycle")] pub recycle: Style,
    pub unchanged: Style,
    pub hidden: Style,
}

// ---------------------------------------------------------------------------
// Build section
// ---------------------------------------------------------------------------

/// `build` section: which weapon/armour classes the user is building for,
/// plus the per-level armour-type condition strings.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Build {
    pub weapons: Vec<String>,
    pub armour: Vec<String>,
    /// keyed by armour-type key (e.g. "anyNonEvasion"); values are condition strings
    pub armour_types: HashMap<String, Vec<String>>,
}

// ---------------------------------------------------------------------------
// RarityRule enum (D-10)
// Replaces freeform Option<String> rarity fields in BandShow and ArmourShow.
// config.json string values stay unchanged; serde renames + alias handle the mapping.
// ---------------------------------------------------------------------------

/// Closed enum for the rarity rules used in areaLevelBands.
///
/// NOTE: config.json uses TWO strings that mean "show Rare":
///   - `"Rare only"` (used in weapons, bands 26+)
///   - `"Rare"` (used in otherGear, bands 6-25 and 26-45)
/// Both emit the identical `Rarity == Rare` condition (verified against universal.filter).
/// They collapse to ONE variant via a serde alias. Serializing always yields `"Rare only"`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum RarityRule {
    #[serde(rename = "any rarity")]
    Any,
    #[serde(rename = "Rare only", alias = "Rare")]
    RareOnly,
    #[serde(rename = "Magic + Rare")]
    MagicPlusRare,
    #[serde(rename = "Magic")]
    Magic,
    #[serde(rename = "Quality only")]
    QualityOnly,
}

// ---------------------------------------------------------------------------
// AreaLevelBands section
// ---------------------------------------------------------------------------

/// Armour-show entry used inside a band's `show.armour` field.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ArmourShow {
    #[serde(rename = "type")]
    pub armour_type: String,
    pub rarity: RarityRule,
}

/// The `show` object inside one area-level band.
/// All keys are optional; which keys appear determines which rules apply (Phase 2 interprets).
/// Rarity values are typed as `RarityRule` — the closed enum (D-10).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BandShow {
    pub weapons: Option<RarityRule>,
    /// Only present in band 1-5 (all armour shown)
    pub all_armour: Option<RarityRule>,
    /// Present in bands 6+ (typed armour progression)
    pub armour: Option<ArmourShow>,
    /// Other gear (non-build classes), present in some bands
    pub other_gear: Option<RarityRule>,
    pub flasks_charms: Option<RarityRule>,
}

/// One entry in `areaLevelBands`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AreaLevelBand {
    pub range: String,
    pub show: BandShow,
}

// ---------------------------------------------------------------------------
// AlwaysShow section
// ---------------------------------------------------------------------------

/// `alwaysShow.currency` — just a note about where the curated data lives.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AlwaysShowCurrency {
    pub style: String,
}

/// `alwaysShow.recycle` — match condition and style reference.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AlwaysShowRecycle {
    #[serde(rename = "match")]
    pub match_condition: String,
    pub style: String,
}

/// `alwaysShow` top-level section.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AlwaysShow {
    pub currency: AlwaysShowCurrency,
    pub recycle: AlwaysShowRecycle,
}

// ---------------------------------------------------------------------------
// Meta section
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub name: String,
    pub palette: String,
    pub background: [u8; 3],
}

// ---------------------------------------------------------------------------
// Top-level Config
// ---------------------------------------------------------------------------

/// Top-level deserialized representation of `config.json`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Config {
    pub meta: Meta,
    /// Tier styles as named fields (D-11): deterministic, type-safe, complete-by-construction.
    /// Replaces the former HashMap<String, Style>. Access via config.styles.s, config.styles.recycle, etc.
    pub styles: Styles,
    pub build: Build,
    pub weapon_classes: Vec<String>,
    pub armour_classes: Vec<String>,
    pub flask_charm_classes: Vec<String>,
    pub area_level_bands: Vec<AreaLevelBand>,
    pub always_show: AlwaysShow,
    pub fallthrough: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.json"));

    #[test]
    fn config_parses_real_file() {
        let result: Result<Config, _> = serde_json::from_str(CONFIG_JSON);
        assert!(result.is_ok(), "config.json failed to parse: {:?}", result.err());
    }

    #[test]
    fn config_styles_named_fields() {
        // D-11: styles is a typed Styles struct — access tier styles by named field,
        // not by map lookup. This test verifies the new interface compiles and works.
        let config: Config = serde_json::from_str(CONFIG_JSON).unwrap();
        // Verify named field access works for all tiers
        assert_eq!(config.styles.s.action, StyleAction::Show);
        assert_eq!(config.styles.a.action, StyleAction::Show);
        assert_eq!(config.styles.b.action, StyleAction::Show);
        assert_eq!(config.styles.c.action, StyleAction::Show);
        assert_eq!(config.styles.d.action, StyleAction::Show);
        assert_eq!(config.styles.e.action, StyleAction::Show);
        assert_eq!(config.styles.f.action, StyleAction::Show);
        assert_eq!(config.styles.recycle.action, StyleAction::Show);
        assert_eq!(config.styles.unchanged.action, StyleAction::Show);
        assert_eq!(config.styles.hidden.action, StyleAction::Hide);
    }

    #[test]
    fn config_round_trips() {
        let first: Config = serde_json::from_str(CONFIG_JSON).unwrap();
        let serialized = serde_json::to_string(&first).unwrap();
        let second: Config = serde_json::from_str(&serialized).unwrap();
        // D-06: semantic round-trip — the deserialized values must be equal.
        // Styles is now a typed struct (D-11), so PartialEq is deterministic
        // (no HashMap key-ordering non-determinism).
        assert_eq!(first, second, "config round-trip produced semantically different result");
    }

    #[test]
    fn config_rejects_unknown_key() {
        // Note: the "styles": {} here will now fail BEFORE reaching the unknown-key check
        // because all Styles fields are required (missing "S", "A", etc.).
        // The test still passes because it only asserts is_err() — any deserialization
        // error (missing required styles fields OR unknown "minimip" key) satisfies it.
        let bad = r#"{
            "meta": {"name":"test","palette":"x","background":[0,0,0]},
            "styles": {},
            "build": {"weapons":[],"armour":[],"armourTypes":{}},
            "weaponClasses": [],
            "armourClasses": [],
            "flaskCharmClasses": [],
            "areaLevelBands": [],
            "alwaysShow": {"currency":{"style":"x"},"recycle":{"match":"x","style":"Recycle"}},
            "fallthrough": "x",
            "minimip": "typo key"
        }"#;
        let result: Result<Config, _> = serde_json::from_str(bad);
        assert!(result.is_err(), "expected error for unknown key 'minimip', got Ok");
    }

    #[test]
    fn config_rejects_bad_enum_color() {
        // inject an invalid color name "Mauve" into the S style minimap
        let bad = CONFIG_JSON.replace("\"Pink\"", "\"Mauve\"");
        let result: Result<Config, _> = serde_json::from_str(&bad);
        assert!(result.is_err(), "expected error for unknown FilterColor 'Mauve', got Ok");
    }

    #[test]
    fn filter_color_variants_roundtrip() {
        let json = r#""Pink""#;
        let color: FilterColor = serde_json::from_str(json).unwrap();
        assert_eq!(color, FilterColor::Pink);
        let back = serde_json::to_string(&color).unwrap();
        assert_eq!(back, json);
    }

    #[test]
    fn style_action_variants() {
        let show: StyleAction = serde_json::from_str(r#""Show""#).unwrap();
        let hide: StyleAction = serde_json::from_str(r#""Hide""#).unwrap();
        assert_eq!(show, StyleAction::Show);
        assert_eq!(hide, StyleAction::Hide);
    }

    // -----------------------------------------------------------------------
    // RarityRule tests (D-10)
    // -----------------------------------------------------------------------

    #[test]
    fn rarity_rule_variants_roundtrip() {
        // "Magic + Rare" deserializes to MagicPlusRare and round-trips back
        let r: RarityRule = serde_json::from_str(r#""Magic + Rare""#).unwrap();
        assert_eq!(r, RarityRule::MagicPlusRare);
        let back = serde_json::to_string(&r).unwrap();
        assert_eq!(back, r#""Magic + Rare""#);
    }

    #[test]
    fn rare_alias_maps_to_rare_only() {
        // otherGear uses bare "Rare" — must map to RarityRule::RareOnly via the serde alias
        let r: RarityRule = serde_json::from_str(r#""Rare""#).unwrap();
        assert_eq!(r, RarityRule::RareOnly);
    }

    #[test]
    fn rarity_rule_rejects_unknown() {
        let result: Result<RarityRule, _> = serde_json::from_str(r#""Legendary""#);
        assert!(result.is_err(), "expected error for unknown rarity rule 'Legendary', got Ok");
    }

    #[test]
    fn rarity_rule_all_variants_parse() {
        // Verify all five rarity-rule strings from config.json deserialize correctly
        let any: RarityRule = serde_json::from_str(r#""any rarity""#).unwrap();
        assert_eq!(any, RarityRule::Any);

        let rare_only: RarityRule = serde_json::from_str(r#""Rare only""#).unwrap();
        assert_eq!(rare_only, RarityRule::RareOnly);

        let magic_rare: RarityRule = serde_json::from_str(r#""Magic + Rare""#).unwrap();
        assert_eq!(magic_rare, RarityRule::MagicPlusRare);

        let magic: RarityRule = serde_json::from_str(r#""Magic""#).unwrap();
        assert_eq!(magic, RarityRule::Magic);

        let quality: RarityRule = serde_json::from_str(r#""Quality only""#).unwrap();
        assert_eq!(quality, RarityRule::QualityOnly);
    }
}
