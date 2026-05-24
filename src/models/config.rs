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
// AreaLevelBands section
// ---------------------------------------------------------------------------

/// Armour-show entry used inside a band's `show.armour` field.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ArmourShow {
    #[serde(rename = "type")]
    pub armour_type: String,
    pub rarity: String,
}

/// The `show` object inside one area-level band.
/// All keys are optional; which keys appear determines which rules apply (Phase 2 interprets).
/// Rarity values ("any rarity", "Magic + Rare", "Rare only", "Quality only") stay as strings (D-07).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BandShow {
    pub weapons: Option<String>,
    /// Only present in band 1-5 (all armour shown)
    pub all_armour: Option<String>,
    /// Present in bands 6+ (typed armour progression)
    pub armour: Option<ArmourShow>,
    /// Other gear (non-build classes), present in some bands
    pub other_gear: Option<String>,
    pub flasks_charms: Option<String>,
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
    /// Tier styles keyed by tier name ("S", "A", "B", "C", "D", "E", "F",
    /// "Recycle", "unchanged", "hidden").
    pub styles: HashMap<String, Style>,
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
    fn config_round_trips() {
        let first: Config = serde_json::from_str(CONFIG_JSON).unwrap();
        let serialized = serde_json::to_string(&first).unwrap();
        let second: Config = serde_json::from_str(&serialized).unwrap();
        // D-06: semantic round-trip — the deserialized values must be equal.
        // Comparing the typed structs (not re-serialized strings) avoids HashMap
        // key-ordering non-determinism while still validating semantic equivalence.
        assert_eq!(first, second, "config round-trip produced semantically different result");
    }

    #[test]
    fn config_rejects_unknown_key() {
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
}
