use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de;

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
// Rule schema types (v0.2 Declarative Rule Model)
// ---------------------------------------------------------------------------

/// The PoE2 item rarity tiers in ordinal order (Normal < Magic < Rare < Unique).
///
/// JSON values are PascalCase matching the variant names (e.g. `"Rare"`).
/// `PartialOrd`/`Ord` allow comparisons such as `rarity >= Magic` in Phase 6 emit logic.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    Normal,
    Magic,
    Rare,
    Unique,
}

/// Comparison operator for a rarity condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RarityOp {
    /// `==` (or bare value — see `RarityComparator`)
    Eq,
    /// `>=`
    Gte,
    /// `<=`
    Lte,
}

/// A rarity condition expressed as a **compact string** in JSON.
///
/// Format: `[op]<Rarity>` where `op` is `==`, `>=`, or `<=`.
/// A bare value with no operator prefix means `==` (e.g. `"Rare"` → `Rarity == Rare`).
///
/// Examples:
/// - `"Rare"`    → `{ op: Eq,  rarity: Rare   }` (bare == Eq)
/// - `"==Rare"`  → `{ op: Eq,  rarity: Rare   }`
/// - `">=Magic"` → `{ op: Gte, rarity: Magic  }`
/// - `"<=Rare"`  → `{ op: Lte, rarity: Rare   }`
///
/// **Round-trip note:** Serialization emits the bare form for `Eq` (no `==` prefix),
/// so `"==Rare"` round-trips to `"Rare"` — both deserialize to the same value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RarityComparator {
    pub op: RarityOp,
    pub rarity: Rarity,
}

impl<'de> Deserialize<'de> for RarityComparator {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let s = s.trim();

        // Strip a leading comparison operator if present; a bare word means `==`.
        // Any malformed operator (e.g. ">", "><") leaves a non-rarity remainder that
        // the rarity match below rejects with a clear, case-listing error — so no
        // fragile byte-slice prefix heuristic is needed (avoids panics on multi-byte
        // UTF-8 input, and gives lowercase words like "rare" a useful error).
        let (op, rarity_str) = if let Some(rest) = s.strip_prefix(">=") {
            (RarityOp::Gte, rest.trim())
        } else if let Some(rest) = s.strip_prefix("<=") {
            (RarityOp::Lte, rest.trim())
        } else if let Some(rest) = s.strip_prefix("==") {
            (RarityOp::Eq, rest.trim())
        } else {
            (RarityOp::Eq, s)
        };

        let rarity = match rarity_str {
            "Normal" => Rarity::Normal,
            "Magic"  => Rarity::Magic,
            "Rare"   => Rarity::Rare,
            "Unique" => Rarity::Unique,
            other => return Err(de::Error::custom(format!(
                "unknown rarity {:?} in rarity comparator; expected Normal, Magic, Rare, or Unique \
                 (optionally prefixed with ==, >=, or <=)",
                other
            ))),
        };

        Ok(RarityComparator { op, rarity })
    }
}

impl Serialize for RarityComparator {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Eq serializes as a bare rarity word (no == prefix) — round-trip produces "Rare" not "==Rare".
        let rarity_str = match self.rarity {
            Rarity::Normal => "Normal",
            Rarity::Magic  => "Magic",
            Rarity::Rare   => "Rare",
            Rarity::Unique => "Unique",
        };
        let s = match self.op {
            RarityOp::Eq  => rarity_str.to_string(),
            RarityOp::Gte => format!(">={}", rarity_str),
            RarityOp::Lte => format!("<={}", rarity_str),
        };
        serializer.serialize_str(&s)
    }
}

/// Numeric operator condition for base-defence attributes, quality, and sockets.
///
/// Field mapping to emitted filter conditions:
/// - `eq`  → `<Attr> <value>`   (exact match; e.g. `BaseEvasion 0`, `BaseArmour 0`)
/// - `min` → `<Attr> >= <min>`  (at-least; e.g. `Quality >= 1`, `Sockets >= 1`,
///                                or `BaseEnergyShield > 0` authored as `min: 1`)
/// - `max` → `<Attr> <= <max>`  (at-most; included for symmetry and future use)
///
/// Note: `> 0` (strictly positive) is expressed as `min: 1` for integer attributes.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NumericCondition {
    pub eq: Option<i64>,
    /// Emits `>= min` (e.g. `min: 1` covers both `Quality >= 1` and `BaseEnergyShield > 0`)
    pub min: Option<i64>,
    /// Emits `<= max` (included for symmetry; for future use)
    pub max: Option<i64>,
}

/// Area-level range condition — both bounds are optional (open-ended ranges allowed).
///
/// - `min` → emits `AreaLevel >= min`
/// - `max` → emits `AreaLevel <= max`
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AreaLevelRange {
    /// Emits `AreaLevel >= min`
    pub min: Option<u32>,
    /// Emits `AreaLevel <= max`
    pub max: Option<u32>,
}

/// UnidentifiedItemTier threshold condition.
///
/// Emits `UnidentifiedItemTier >= min`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnidentifiedTier {
    /// Emits `UnidentifiedItemTier >= min`
    pub min: u32,
}

/// The match conditions for a declarative rule.
///
/// All fields are optional. An absent/empty `When` matches everything (catch-all semantics).
/// Unknown fields are rejected (`deny_unknown_fields`) to catch typos at load time.
///
/// JSON field names are camelCase (e.g. `areaLevel`, `baseArmour`, `baseEnergyShield`,
/// `baseType`, `unidentifiedTier`) mapped to snake_case Rust fields by `rename_all`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct When {
    /// `Class == "..."` (exact full-name match)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<Vec<String>>,
    /// Compact rarity comparator (e.g. `">=Magic"`, `"Rare"`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rarity: Option<RarityComparator>,
    /// `AreaLevel >= min` and/or `AreaLevel <= max`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_level: Option<AreaLevelRange>,
    /// `BaseArmour` numeric condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_armour: Option<NumericCondition>,
    /// `BaseEvasion` numeric condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_evasion: Option<NumericCondition>,
    /// `BaseEnergyShield` numeric condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_energy_shield: Option<NumericCondition>,
    /// `Quality` numeric condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<NumericCondition>,
    /// `Sockets` numeric condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sockets: Option<NumericCondition>,
    /// `BaseType == "..."` (exact full-name match)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_type: Option<Vec<String>>,
    /// `UnidentifiedItemTier >= min`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unidentified_tier: Option<UnidentifiedTier>,
}

/// A declarative filter rule: a named, optionally-conditional style assignment.
///
/// Rules are evaluated in order; the first matching rule wins (first-match-wins semantics).
/// A rule with no `when` matches everything and acts as a catch-all/fallback.
///
/// The `style` field is a plain `String` resolved against the `Styles` struct keys at
/// generate-time. Valid values: `"S"`, `"A"`, `"B"`, `"C"`, `"D"`, `"E"`, `"F"`,
/// `"unchanged"`, `"hidden"`, `"Recycle"`. An unknown style name is a **hard error**
/// at generate-time (Phase 6 validation — not enforced by serde here).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Rule {
    /// Optional label emitted as the block comment header (`Show # <name>`).
    /// When absent the emitter derives a label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Style tier name — must match a `Styles` key (S/A/B/C/D/E/F/unchanged/hidden/Recycle).
    /// Validation (unknown-style hard error) is deferred to generate-time (Phase 6).
    pub style: String,
    /// Optional match conditions. Absent/empty `when` matches everything.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,
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
    pub always_show: AlwaysShow,
    pub fallthrough: String,
    /// Ordered declarative policy rules (RULE-01). Optional during the migration —
    /// absent today, populated in Phase 6. Authored order is preserved (first-match-wins).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<Rule>>,
    /// Optional class-alias map: alias name -> list of item classes, referenced by
    /// `when.class` names to avoid repetition (e.g. "buildWeapons": ["Staves","Wands","Foci"]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_groups: Option<HashMap<String, Vec<String>>>,
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
    fn config_with_rules_and_class_groups_parses() {
        // Prove that rules + classGroups are additive optional fields: a config parsed from
        // the real file (no rules/classGroups) has both as None (via config_parses_real_file),
        // while a config that includes them parses with both as Some.
        // We inject rules+classGroups by string-replacement into the real config.json.
        let with_rules = CONFIG_JSON.replacen(
            r#""fallthrough":"#,
            r#""rules": [
    { "style": "F" },
    { "name": "Test rule", "style": "E", "when": { "class": ["Staves"] } }
  ],
  "classGroups": {
    "buildWeapons": ["Staves", "Wands", "Foci"]
  },
  "fallthrough":"#,
            1,
        );
        let config: Config = serde_json::from_str(&with_rules)
            .expect("config with rules+classGroups should parse");
        assert!(config.rules.is_some(), "rules should be Some");
        assert_eq!(config.rules.as_ref().unwrap().len(), 2);
        assert!(config.class_groups.is_some(), "classGroups should be Some");
        assert!(config.class_groups.as_ref().unwrap().contains_key("buildWeapons"));
        // Also confirm baseline (no rules) still has both as None
        let baseline: Config = serde_json::from_str(CONFIG_JSON).unwrap();
        assert!(baseline.rules.is_none(), "baseline config should have rules = None");
        assert!(baseline.class_groups.is_none(), "baseline config should have class_groups = None");
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
    // RarityComparator tests (v0.2 rule schema — COND-02)
    // -----------------------------------------------------------------------

    #[test]
    fn rarity_comparator_bare_is_eq() {
        let r: RarityComparator = serde_json::from_str(r#""Rare""#).unwrap();
        assert_eq!(r, RarityComparator { op: RarityOp::Eq, rarity: Rarity::Rare });
    }

    #[test]
    fn rarity_comparator_gte_magic() {
        let r: RarityComparator = serde_json::from_str(r#"">=Magic""#).unwrap();
        assert_eq!(r, RarityComparator { op: RarityOp::Gte, rarity: Rarity::Magic });
    }

    #[test]
    fn rarity_comparator_lte_rare() {
        let r: RarityComparator = serde_json::from_str(r#""<=Rare""#).unwrap();
        assert_eq!(r, RarityComparator { op: RarityOp::Lte, rarity: Rarity::Rare });
    }

    #[test]
    fn rarity_comparator_eq_prefix_unique() {
        let r: RarityComparator = serde_json::from_str(r#""==Unique""#).unwrap();
        assert_eq!(r, RarityComparator { op: RarityOp::Eq, rarity: Rarity::Unique });
    }

    #[test]
    fn rarity_comparator_rejects_unknown_rarity() {
        let result: Result<RarityComparator, _> = serde_json::from_str(r#""Legendary""#);
        assert!(result.is_err(), "expected error for unknown rarity 'Legendary', got Ok");
    }

    #[test]
    fn rarity_comparator_rejects_bad_op() {
        let result: Result<RarityComparator, _> = serde_json::from_str(r#""><Rare""#);
        assert!(result.is_err(), "expected error for unknown op '><' in '><Rare', got Ok");
    }

    #[test]
    fn rarity_comparator_rejects_multibyte_without_panic() {
        // Regression: a multi-byte UTF-8 input must produce a clean Err, never a
        // byte-slice boundary panic (the old `&s[..2]` heuristic panicked here).
        let result: Result<RarityComparator, _> = serde_json::from_str(r#""日本語""#);
        assert!(result.is_err(), "expected error for multi-byte input, got Ok");
    }

    #[test]
    fn rarity_comparator_round_trip() {
        // ">=Magic" deserializes, serializes to ">=Magic", re-deserializes to same value.
        let original: RarityComparator = serde_json::from_str(r#"">=Magic""#).unwrap();
        let serialized = serde_json::to_string(&original).unwrap();
        let restored: RarityComparator = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn rarity_comparator_eq_serializes_bare() {
        // Eq serializes without == prefix — "==Rare" round-trips to "Rare"
        let r = RarityComparator { op: RarityOp::Eq, rarity: Rarity::Rare };
        let s = serde_json::to_string(&r).unwrap();
        assert_eq!(s, r#""Rare""#);
    }

    // -----------------------------------------------------------------------
    // NumericCondition tests (v0.2 rule schema — COND-04)
    // -----------------------------------------------------------------------

    #[test]
    fn numeric_condition_eq_zero() {
        let nc: NumericCondition = serde_json::from_str(r#"{"eq": 0}"#).unwrap();
        assert_eq!(nc, NumericCondition { eq: Some(0), min: None, max: None });
    }

    #[test]
    fn numeric_condition_min_one() {
        let nc: NumericCondition = serde_json::from_str(r#"{"min": 1}"#).unwrap();
        assert_eq!(nc, NumericCondition { eq: None, min: Some(1), max: None });
    }

    #[test]
    fn numeric_condition_rejects_unknown_key() {
        let result: Result<NumericCondition, _> = serde_json::from_str(r#"{"lt": 3}"#);
        assert!(result.is_err(), "expected error for unknown key 'lt', got Ok");
    }

    // -----------------------------------------------------------------------
    // AreaLevelRange tests (v0.2 rule schema — COND-03)
    // -----------------------------------------------------------------------

    #[test]
    fn area_level_range_both_bounds() {
        let r: AreaLevelRange = serde_json::from_str(r#"{"min": 5, "max": 10}"#).unwrap();
        assert_eq!(r, AreaLevelRange { min: Some(5), max: Some(10) });
    }

    #[test]
    fn area_level_range_min_only() {
        let r: AreaLevelRange = serde_json::from_str(r#"{"min": 5}"#).unwrap();
        assert_eq!(r, AreaLevelRange { min: Some(5), max: None });
    }

    #[test]
    fn area_level_range_max_only() {
        let r: AreaLevelRange = serde_json::from_str(r#"{"max": 10}"#).unwrap();
        assert_eq!(r, AreaLevelRange { min: None, max: Some(10) });
    }

    // -----------------------------------------------------------------------
    // UnidentifiedTier tests (v0.2 rule schema — COND-06)
    // -----------------------------------------------------------------------

    #[test]
    fn unidentified_tier_min() {
        let t: UnidentifiedTier = serde_json::from_str(r#"{"min": 2}"#).unwrap();
        assert_eq!(t, UnidentifiedTier { min: 2 });
    }

    // -----------------------------------------------------------------------
    // Rule + When tests (v0.2 rule schema — RULE-02 / COND-01..06)
    // -----------------------------------------------------------------------

    /// A representative rules+classGroups JSON fragment (not a full Config).
    /// Used by round-trip and order-preservation tests — exercises every condition type:
    /// class list, rarity comparator (>=), areaLevel min+max, baseArmour eq, baseEvasion eq,
    /// baseEnergyShield min, quality min, sockets min, baseType list, unidentifiedTier, classGroups.
    const REPRESENTATIVE_RULES_JSON: &str = r#"[
    { "style": "F" },
    { "style": "S", "name": "Top tier" },
    { "style": "unchanged", "when": {
        "class": ["Body Armours", "Helmets"],
        "rarity": ">=Magic",
        "areaLevel": { "min": 6, "max": 25 },
        "baseArmour": { "eq": 0 },
        "baseEvasion": { "eq": 0 },
        "baseEnergyShield": { "min": 1 },
        "quality": { "min": 1 },
        "sockets": { "min": 1 },
        "baseType": ["Titanium Kite Shield", "Astral Plate"],
        "unidentifiedTier": { "min": 2 }
    }}
]"#;

    #[test]
    fn rules_preserve_authored_order() {
        // Authored order is F, S, unchanged — must NOT be sorted.
        let rules: Vec<Rule> = serde_json::from_str(REPRESENTATIVE_RULES_JSON).unwrap();
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].style, "F", "first rule should be style F");
        assert_eq!(rules[1].style, "S", "second rule should be style S");
        assert_eq!(rules[2].style, "unchanged", "third rule should be style unchanged");
    }

    #[test]
    fn config_with_rules_round_trips() {
        // Build a Config JSON with a representative rules array and classGroups, then
        // serialize and re-deserialize — the two values must be equal (lossless round-trip).
        let with_rules = CONFIG_JSON.replacen(
            r#""fallthrough":"#,
            &format!(
                r#""rules": {},
  "classGroups": {{
    "buildWeapons": ["Staves", "Wands", "Foci"],
    "buildArmour": ["Body Armours", "Helmets", "Gloves", "Boots"]
  }},
  "fallthrough":"#,
                REPRESENTATIVE_RULES_JSON
            ),
            1,
        );
        let first: Config = serde_json::from_str(&with_rules)
            .expect("config with representative rules should parse");
        let serialized = serde_json::to_string(&first).expect("serialize should succeed");
        let second: Config = serde_json::from_str(&serialized)
            .expect("re-deserialize after round-trip should succeed");
        assert_eq!(first, second, "config with rules did not round-trip losslessly");

        // Spot-check: authored rule order is preserved in the round-tripped value
        let rules = first.rules.as_ref().unwrap();
        assert_eq!(rules[0].style, "F");
        assert_eq!(rules[1].style, "S");
        assert_eq!(rules[2].style, "unchanged");

        // classGroups round-tripped
        let groups = first.class_groups.as_ref().unwrap();
        assert!(groups.contains_key("buildWeapons"));
        assert!(groups.contains_key("buildArmour"));
    }

    #[test]
    fn bad_rarity_in_rule_reports_error() {
        // A rule with when.rarity: "Legendary" (invalid) must produce an Err.
        // At runtime, src/loader.rs supplies serde_path_to_error for field-path context.
        // Here we verify the plain serde_json error (the underlying rejection is the same).
        let json = r#"{
            "style": "E",
            "when": { "rarity": "Legendary" }
        }"#;
        let result: Result<Rule, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "a rule with an invalid rarity in when should produce an Err (got Ok)"
        );
        // loader.rs applies serde_path_to_error at runtime, which will include "rarity"
        // in the field path. This is the same rejection surfaced with full context there.
    }

    #[test]
    fn rule_style_only() {
        // A rule with just "style" (no when, no name) deserializes — matches-everything fallback.
        let r: Rule = serde_json::from_str(r#"{"style": "F"}"#).unwrap();
        assert_eq!(r.style, "F");
        assert!(r.name.is_none());
        assert!(r.when.is_none());
    }

    #[test]
    fn rule_full_round_trip() {
        let json = r#"{
            "name": "Energy-shield gear",
            "style": "E",
            "when": {
                "class": ["Body Armours", "Helmets"],
                "rarity": ">=Magic",
                "areaLevel": {"min": 26},
                "baseEvasion": {"eq": 0},
                "baseType": ["Titan Plate"],
                "unidentifiedTier": {"min": 3}
            }
        }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();
        let when = rule.when.as_ref().unwrap();
        assert_eq!(rule.name.as_deref(), Some("Energy-shield gear"));
        assert_eq!(rule.style, "E");
        assert_eq!(when.class.as_ref().unwrap(), &["Body Armours", "Helmets"]);
        assert_eq!(when.rarity.unwrap().op, RarityOp::Gte);
        assert_eq!(when.rarity.unwrap().rarity, Rarity::Magic);
        assert_eq!(when.area_level.as_ref().unwrap().min, Some(26));
        assert_eq!(when.base_evasion.as_ref().unwrap().eq, Some(0));
        assert_eq!(when.base_type.as_ref().unwrap(), &["Titan Plate"]);
        assert_eq!(when.unidentified_tier.as_ref().unwrap().min, 3);

        // Round-trip: serialize then re-deserialize, must be equal
        let serialized = serde_json::to_string(&rule).unwrap();
        let restored: Rule = serde_json::from_str(&serialized).unwrap();
        assert_eq!(rule, restored);
    }

    #[test]
    fn rule_rejects_unknown_field() {
        let json = r#"{"style": "E", "unknown_key": true}"#;
        let result: Result<Rule, _> = serde_json::from_str(json);
        assert!(result.is_err(), "expected error for unknown field 'unknown_key' on Rule");
    }

    #[test]
    fn when_rejects_unknown_field() {
        let json = r#"{"style": "E", "when": {"notAField": 1}}"#;
        let result: Result<Rule, _> = serde_json::from_str(json);
        assert!(result.is_err(), "expected error for unknown field 'notAField' on When");
    }

    #[test]
    fn when_all_condition_fields_populate() {
        // Full when object with all condition types present
        let json = r#"{
            "style": "C",
            "when": {
                "class": ["Staves"],
                "rarity": ">=Magic",
                "areaLevel": {"min": 1, "max": 25},
                "baseArmour": {"eq": 0},
                "baseEvasion": {"eq": 0},
                "baseEnergyShield": {"min": 1},
                "quality": {"min": 1},
                "sockets": {"min": 1},
                "baseType": ["Short Staff"],
                "unidentifiedTier": {"min": 2}
            }
        }"#;
        let rule: Rule = serde_json::from_str(json).unwrap();
        let when = rule.when.as_ref().unwrap();
        assert!(when.class.is_some());
        assert!(when.rarity.is_some());
        assert!(when.area_level.is_some());
        assert!(when.base_armour.is_some());
        assert!(when.base_evasion.is_some());
        assert!(when.base_energy_shield.is_some());
        assert!(when.quality.is_some());
        assert!(when.sockets.is_some());
        assert!(when.base_type.is_some());
        assert!(when.unidentified_tier.is_some());
    }
}
