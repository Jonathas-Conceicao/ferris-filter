use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de;
use indexmap::IndexMap;
use miette::Diagnostic;
use thiserror::Error;

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
// StyleSet — config-driven named styles (Phase 17, STYLE-01/02)
// ---------------------------------------------------------------------------

/// Reasons a parsed `styles.json` is rejected by [`StyleSet::validate`].
#[derive(Debug, Error, Diagnostic)]
pub enum StyleError {
    #[error(
        "styles file is missing required style(s): {} (S, A, B, C, D, E, F, unchanged and hidden are required)",
        .0.join(", ")
    )]
    #[diagnostic(code(ferrisfilter::styles::missing))]
    MissingRequired(Vec<String>),

    #[error(
        "style name \"{name}\" differs only by case from the required style \"{required}\"; \
         names that differ only by case from a required style are reserved"
    )]
    #[diagnostic(code(ferrisfilter::styles::shadows_required))]
    ShadowsRequired { name: String, required: String },

    #[error(
        "style \"{style}\" has an invalid sound.custom {value:?}: it must not contain a double quote or control character"
    )]
    #[diagnostic(code(ferrisfilter::styles::bad_custom_sound))]
    InvalidCustomSound { style: String, value: String },

    #[error(
        "rule {rule_index} (\"{rule_label}\") uses style \"{style}\", which {path} does not define; defined styles: {available}"
    )]
    #[diagnostic(code(ferrisfilter::styles::unknown_in_rule))]
    UnknownInRule {
        rule_index: usize,
        rule_label: String,
        style: String,
        path: String,
        available: String,
    },

    #[error(
        "{path} still has a `styles` section, but named styles now live in styles.json; move its entries into styles.json (or pass --styles <file>) and delete the `styles` section from {path}"
    )]
    #[diagnostic(code(ferrisfilter::styles::inline_section))]
    InlineStylesSection { path: String },
}

/// Named styles in file order. The top level of `styles.json` *is* this map.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(transparent)]
pub struct StyleSet(IndexMap<String, Style>);

impl<'de> Deserialize<'de> for StyleSet {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = StyleSet;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a map of style name to style")
            }
            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<StyleSet, A::Error> {
                let mut out = IndexMap::new();
                while let Some((name, style)) = map.next_entry::<String, Style>()? {
                    if out.contains_key(&name) {
                        return Err(de::Error::custom(format!("duplicate style name \"{name}\"")));
                    }
                    out.insert(name, style);
                }
                Ok(StyleSet(out))
            }
        }
        deserializer.deserialize_map(V)
    }
}

impl StyleSet {
    /// Styles every config needs: the economy emitter reads S..F, rules lean on
    /// `unchanged`/`hidden`. `kite`, `Recycle` and `exceptional` are deliberately optional.
    pub const REQUIRED: [&'static str; 9] =
        ["S", "A", "B", "C", "D", "E", "F", "unchanged", "hidden"];

    pub fn get(&self, name: &str) -> Option<&Style> {
        self.0.get(name)
    }

    /// Style names in file order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn from_pairs<I, N>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (N, Style)>,
        N: Into<String>,
    {
        StyleSet(pairs.into_iter().map(|(n, s)| (n.into(), s)).collect())
    }

    pub fn validate(&self) -> Result<(), StyleError> {
        let missing: Vec<String> = Self::REQUIRED
            .iter()
            .filter(|r| !self.0.contains_key(**r))
            .map(|r| r.to_string())
            .collect();
        if !missing.is_empty() {
            return Err(StyleError::MissingRequired(missing));
        }
        for name in self.0.keys() {
            if Self::REQUIRED.contains(&name.as_str()) {
                continue;
            }
            if let Some(req) = Self::REQUIRED.iter().find(|r| r.eq_ignore_ascii_case(name)) {
                return Err(StyleError::ShadowsRequired {
                    name: name.clone(),
                    required: req.to_string(),
                });
            }
        }
        for (name, style) in &self.0 {
            if let Some(value) = style.sound.as_ref().and_then(|s| s.custom.as_ref()) {
                if value.chars().any(|c| c == '"' || c.is_control()) {
                    return Err(StyleError::InvalidCustomSound {
                        style: name.clone(),
                        value: value.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Check that every ENABLED rule names a defined style (D-07). A disabled rule is inert
    /// and mirrors the emitter, which skips it entirely.
    pub fn check_rules(&self, rules: &[Rule], styles_path: &str) -> Result<(), StyleError> {
        for (idx, rule) in rules.iter().enumerate() {
            if rule.enabled == Some(false) {
                continue;
            }
            if !self.0.contains_key(&rule.style) {
                return Err(StyleError::UnknownInRule {
                    rule_index: idx,
                    rule_label: rule.name.clone().unwrap_or_else(|| format!("rule {idx}")),
                    style: rule.style.clone(),
                    path: styles_path.to_string(),
                    available: self.names().collect::<Vec<_>>().join(", "),
                });
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn insert(&mut self, name: &str, style: Style) {
        self.0.insert(name.to_string(), style);
    }

    #[cfg(test)]
    pub(crate) fn remove(&mut self, name: &str) -> Option<Style> {
        self.0.shift_remove(name)
    }

    #[cfg(test)]
    pub(crate) fn committed_for_tests() -> StyleSet {
        let set: StyleSet = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/styles.json"
        )))
        .unwrap_or_else(|e| panic!("committed styles.json failed to parse: {e}"));
        set.validate()
            .unwrap_or_else(|e| panic!("committed styles.json failed validation: {e}"));
        set
    }
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
    /// `Corrupted True` / `Corrupted False` (boolean — NO operator). None emits nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corrupted: Option<bool>,
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
/// The `style` field is a plain `String` resolved against the loaded `StyleSet` (`styles.json`)
/// at generate-time: it must be one of the set's names. Any style defined in the set may be used.
/// An unknown style name is a **hard error** at generate-time (Phase 6 validation — not enforced by serde here).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Rule {
    /// Whether this rule contributes a block to the generated filter.
    ///
    /// Absent (the default) or `Some(true)` → the rule is active and emits normally.
    /// `Some(false)` → the rule stays in the file but emits **nothing at all** — not even a
    /// commented-out block. This is how the overlay switches a rule off without losing its
    /// configuration; deleting a rule is a separate operation (removal from the array).
    ///
    /// Absence is the default so every existing config parses and round-trips unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Optional label emitted as the block comment header (`Show # <name>`).
    /// When absent the emitter derives a label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Style name — must be a key of the loaded `StyleSet` (styles.json).
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
    /// Named styles, in file order. Look up by name with `StyleSet::get`.
    /// Populated by the loaders from styles.json; never read from or written to
    /// config/store JSON (D-03).
    #[serde(skip)]
    pub styles: StyleSet,
    /// Raw pre-Phase-17 inline `styles` section (OD-1). Present only so an old file still
    /// parses far enough to be rejected with a clear message by `loader::load_config`.
    /// Never serialized.
    #[serde(default, rename = "styles", skip_serializing)]
    pub legacy_styles: Option<serde_json::Value>,
    pub build: Build,
    pub weapon_classes: Vec<String>,
    pub armour_classes: Vec<String>,
    pub flask_charm_classes: Vec<String>,
    /// Rings / Amulets / Belts. Added 2026-09-07 so the overlay's class picker can offer
    /// jewellery — `config.json`'s own rules already use these classes, but there was no list
    /// to source them from, leaving them unreachable in the editor (EDIT-04 requires options
    /// come from config, never a hardcoded list — `PITFALLS.md` §10).
    ///
    /// `#[serde(default)]` rather than required, so a `dynamic.json` seeded before this field
    /// existed still loads. An absent list yields an empty group, which the picker omits.
    #[serde(default)]
    pub jewellery_classes: Vec<String>,
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
    fn config_styles_lookup_by_name() {
        // Styles are a name -> Style map; tiers are looked up by name.
        let styles = StyleSet::committed_for_tests();
        for name in ["S", "A", "B", "C", "D", "E", "F", "Recycle", "unchanged"] {
            assert_eq!(styles.get(name).unwrap().action, StyleAction::Show, "{name}");
        }
        assert_eq!(styles.get("hidden").unwrap().action, StyleAction::Hide);
    }

    #[test]
    fn config_round_trips() {
        let first: Config = serde_json::from_str(CONFIG_JSON).unwrap();
        let serialized = serde_json::to_string(&first).unwrap();
        let second: Config = serde_json::from_str(&serialized).unwrap();
        // D-06: semantic round-trip — the deserialized values must be equal.
        // StyleSet is insertion-ordered, so PartialEq is deterministic.
        assert_eq!(first, second, "config round-trip produced semantically different result");
    }

    #[test]
    fn config_rejects_unknown_key() {
        // The error here comes from the unknown "minimip" key.
        let bad = r#"{
            "meta": {"name":"test","palette":"x","background":[0,0,0]},
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
        // inject an invalid color name "Mauve" into styles.json
        let styles_json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/styles.json"));
        let bad = styles_json.replace("\"Pink\"", "\"Mauve\"");
        let result: Result<StyleSet, _> = serde_json::from_str(&bad);
        assert!(result.is_err(), "expected error for unknown FilterColor 'Mauve', got Ok");
    }

    #[test]
    fn config_with_rules_and_class_groups_parses() {
        // Prove that rules + classGroups parse correctly from the real config.json.
        // Phase 6 migrated config.json to include a rules array and classGroups map.
        let config: Config = serde_json::from_str(CONFIG_JSON)
            .expect("config with rules+classGroups should parse");
        assert!(config.rules.is_some(), "config.json rules should be Some after Phase 6 migration");
        assert!(
            config.rules.as_ref().unwrap().len() >= 13,
            "config.json should have at least 13 rules (the 13 policy blocks)"
        );
        assert!(config.class_groups.is_some(), "classGroups should be Some after Phase 6 migration");
        assert!(config.class_groups.as_ref().unwrap().contains_key("buildWeapons"));
        // Also confirm a minimal config (no rules/classGroups) parses with both as None.
        let minimal = r#"{
            "meta": {"name":"x","palette":"x","background":[0,0,0]},
            "build": {"weapons":[],"armour":[],"armourTypes":{}},
            "weaponClasses": [],
            "armourClasses": [],
            "flaskCharmClasses": [],
            "alwaysShow": {"currency":{"style":"x"},"recycle":{"match":"x","style":"Recycle"}},
            "fallthrough": "x"
        }"#;
        let baseline: Config = serde_json::from_str(minimal).unwrap();
        assert!(baseline.rules.is_none(), "minimal config should have rules = None");
        assert!(baseline.class_groups.is_none(), "minimal config should have class_groups = None");
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
        // Verify that config.json (which now includes rules + classGroups) round-trips
        // losslessly: deserialize → serialize → re-deserialize produces equal values.
        // Phase 6 migrated config.json to include the full 13-rule policy array.
        let first: Config = serde_json::from_str(CONFIG_JSON)
            .expect("config with rules should parse");
        let serialized = serde_json::to_string(&first).expect("serialize should succeed");
        let second: Config = serde_json::from_str(&serialized)
            .expect("re-deserialize after round-trip should succeed");
        assert_eq!(first, second, "config with rules did not round-trip losslessly");

        // Spot-check: rules and classGroups are present
        let rules = first.rules.as_ref().unwrap();
        assert!(rules.len() >= 13, "should have at least 13 policy rules");
        // First rule should be the first exceptional rule (recycle is now rules[3])
        assert_eq!(rules[0].style, "exceptional", "first rule should be exceptional style (Phase 10 placed 3 exceptional rules before recycle)");

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

    // -----------------------------------------------------------------------
    // Exceptional style tests (EXC-04)
    // -----------------------------------------------------------------------

    #[test]
    fn config_has_exceptional_style() {
        let styles = StyleSet::committed_for_tests();
        let exceptional = styles.get("exceptional")
            .expect("styles.json must define the exceptional style");
        assert_eq!(
            exceptional.action,
            StyleAction::Show,
            "exceptional style must be Show"
        );
        // Verify the pink color (255 121 198)
        assert_eq!(
            exceptional.text,
            Some([255, 121, 198]),
            "exceptional text color must be pink [255, 121, 198]"
        );
        assert_eq!(
            exceptional.border,
            Some([255, 121, 198]),
            "exceptional border color must be pink [255, 121, 198]"
        );
        // No sound, no font
        assert!(exceptional.sound.is_none(), "exceptional must have no sound");
        assert!(exceptional.font.is_none(), "exceptional must have no font");
        // Pink Cross minimap
        let mm = exceptional.minimap.as_ref().expect("exceptional must have minimap");
        assert_eq!(mm.color, FilterColor::Pink, "exceptional minimap color must be Pink");
        assert_eq!(mm.shape, MinimapShape::Cross, "exceptional minimap shape must be Cross");
    }

    /// Verify that a styles object that omits `"exceptional"` still parses (WR-01 fix).
    /// This models SSF custom configs like `ssf_icewolf_config.json` which predate Phase 10.
    #[test]
    fn styles_without_exceptional_parses_as_none() {
        // A minimal styles JSON with all required fields but no "exceptional" key.
        let styles_json = r#"{
            "S":{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false},
            "A":{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false},
            "B":{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false},
            "C":{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false},
            "D":{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false},
            "E":{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false},
            "F":{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false},
            "Recycle":{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false},
            "unchanged":{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false},
            "hidden":{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Hide","disableDropSound":false},
            "kite":{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false}
        }"#;
        let styles: StyleSet = serde_json::from_str(styles_json)
            .expect("styles without 'exceptional' must parse (WR-01 regression check)");
        styles.validate().expect("the 11-style set is valid");
        assert!(
            styles.get("exceptional").is_none(),
            "exceptional must be absent when not in the file"
        );
    }

    // -----------------------------------------------------------------------
    // Rule.enabled round-trip (DYN-01)
    // -----------------------------------------------------------------------

    #[test]
    fn rule_enabled_false_parses() {
        let rule: Rule = serde_json::from_str(r#"{"enabled": false, "style": "E"}"#)
            .expect("enabled:false must parse");
        assert_eq!(rule.enabled, Some(false));
    }

    #[test]
    fn rule_enabled_true_parses() {
        let rule: Rule = serde_json::from_str(r#"{"enabled": true, "style": "E"}"#)
            .expect("enabled:true must parse");
        assert_eq!(rule.enabled, Some(true));
    }

    #[test]
    fn rule_enabled_absent_is_none() {
        let rule: Rule =
            serde_json::from_str(r#"{"style": "E"}"#).expect("absent enabled must parse");
        assert_eq!(
            rule.enabled, None,
            "absence must mean None (treated as enabled), so existing configs are unaffected"
        );
    }

    /// The guarantee that keeps `config.json` and the committed fixtures byte-identical:
    /// a rule that never set `enabled` must not grow the key on serialize.
    #[test]
    fn rule_without_enabled_serializes_without_the_key() {
        let rule = Rule {
            enabled: None,
            name: None,
            style: "E".to_string(),
            when: None,
        };
        let json = serde_json::to_string(&rule).expect("serialize");
        assert!(
            !json.contains("enabled"),
            "enabled:None must be skipped on serialize, got: {json}"
        );
    }

    #[test]
    fn rule_enabled_round_trips() {
        for value in [Some(true), Some(false), None] {
            let rule = Rule {
                enabled: value,
                name: None,
                style: "E".to_string(),
                when: None,
            };
            let json = serde_json::to_string(&rule).expect("serialize");
            let back: Rule = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.enabled, value, "round-trip failed for {value:?}");
        }
    }

    // -----------------------------------------------------------------------
    // Corrupted condition tests (EXC-01)
    // -----------------------------------------------------------------------

    #[test]
    fn when_corrupted_false_parses() {
        let rule: Rule = serde_json::from_str(r#"{"style":"E","when":{"corrupted":false}}"#).unwrap();
        assert_eq!(rule.when.as_ref().unwrap().corrupted, Some(false));
    }

    #[test]
    fn when_corrupted_true_parses() {
        let rule: Rule = serde_json::from_str(r#"{"style":"E","when":{"corrupted":true}}"#).unwrap();
        assert_eq!(rule.when.as_ref().unwrap().corrupted, Some(true));
    }

    #[test]
    fn when_corrupted_absent_is_none() {
        let rule: Rule = serde_json::from_str(r#"{"style":"E","when":{"sockets":{"min":1}}}"#).unwrap();
        assert!(rule.when.as_ref().unwrap().corrupted.is_none());
    }

    #[test]
    fn when_corrupted_round_trips() {
        let rule = Rule {
            enabled: None,
            name: Some("test-corrupted".to_string()),
            style: "E".to_string(),
            when: Some(When {
                corrupted: Some(false),
                ..Default::default()
            }),
        };
        let serialized = serde_json::to_string(&rule).unwrap();
        let restored: Rule = serde_json::from_str(&serialized).unwrap();
        assert_eq!(rule, restored);
        // None-valued fields must not appear in the serialized form
        assert!(serialized.contains("\"corrupted\":false"));
    }
    // -----------------------------------------------------------------------
    // StyleSet (Phase 17)
    // -----------------------------------------------------------------------

    const STYLES_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/styles.json"));
    const DYNAMIC_BASE_JSON: &str =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dynamic_base.json"));

    const COMMITTED_ORDER: [&str; 12] = [
        "S", "A", "B", "C", "D", "E", "F", "Recycle", "unchanged", "hidden", "kite", "exceptional",
    ];

    #[test]
    fn committed_configs_carry_no_inline_styles() {
        for src in [CONFIG_JSON, DYNAMIC_BASE_JSON] {
            let cfg: Config = serde_json::from_str(src).unwrap();
            assert!(cfg.legacy_styles.is_none());
            assert!(cfg.styles.is_empty());
        }
    }

    #[test]
    fn serialized_config_has_no_styles_key() {
        let mut cfg: Config = serde_json::from_str(CONFIG_JSON).unwrap();
        cfg.styles = StyleSet::committed_for_tests();
        let v: serde_json::Value = serde_json::to_value(&cfg).unwrap();
        assert!(v.get("styles").is_none());
    }

    #[test]
    fn inline_styles_section_parses_into_legacy_field() {
        let mut v: serde_json::Value = serde_json::from_str(CONFIG_JSON).unwrap();
        v["styles"] = serde_json::json!({});
        let cfg: Config = serde_json::from_value(v).unwrap();
        assert!(cfg.legacy_styles.is_some());
    }

    #[test]
    fn check_rules_flags_unknown_enabled_rule_and_skips_disabled() {
        let styles = StyleSet::committed_for_tests();
        let mut cfg: Config = serde_json::from_str(CONFIG_JSON).unwrap();
        let rules = cfg.rules.as_mut().unwrap();
        rules[2].style = "nope".to_string();
        rules[2].name = None;
        rules[2].enabled = None;
        let err = styles.check_rules(rules, "styles.json").unwrap_err().to_string();
        assert!(err.contains("nope") && err.contains("rule 2") && err.contains("exceptional"), "{err}");
        rules[2].enabled = Some(false);
        styles.check_rules(rules, "styles.json").unwrap();
    }

    #[test]
    fn styles_json_has_the_twelve_committed_styles() {
        let set = StyleSet::committed_for_tests();
        assert_eq!(set.names().collect::<Vec<_>>(), COMMITTED_ORDER);
        let s = set.get("S").unwrap();
        assert_eq!(s.font, Some(37));
        assert_eq!(s.sound.as_ref().unwrap().custom.as_deref(), Some("uwu.mp3"));
        let h = set.get("hidden").unwrap();
        assert_eq!(h.action, StyleAction::Hide);
        assert_eq!(h.font, Some(21));
        assert!(h.disable_drop_sound);
    }

    fn sparse(action: &str) -> String {
        format!(
            r#"{{"text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"{action}","disableDropSound":false}}"#
        )
    }

    fn set_json(names: &[&str]) -> String {
        let body: Vec<String> = names
            .iter()
            .map(|n| format!("\"{n}\":{}", sparse(if *n == "hidden" { "Hide" } else { "Show" })))
            .collect();
        format!("{{{}}}", body.join(","))
    }

    const REQ: [&str; 9] = ["S", "A", "B", "C", "D", "E", "F", "unchanged", "hidden"];

    #[test]
    fn styleset_missing_required_names_every_missing_style() {
        let names: Vec<&str> = REQ.iter().copied().filter(|n| *n != "E").collect();
        let set: StyleSet = serde_json::from_str(&set_json(&names)).unwrap();
        match set.validate() {
            Err(StyleError::MissingRequired(m)) => assert_eq!(m, vec!["E"]),
            other => panic!("unexpected {other:?}"),
        }
        let names: Vec<&str> =
            REQ.iter().copied().filter(|n| *n != "unchanged" && *n != "hidden").collect();
        let set: StyleSet = serde_json::from_str(&set_json(&names)).unwrap();
        match set.validate() {
            Err(StyleError::MissingRequired(m)) => assert_eq!(m, vec!["unchanged", "hidden"]),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn styleset_rejects_duplicate_style_name() {
        let dup = format!(r#"{{"S":{0},"S":{0}}}"#, sparse("Show"));
        let err = serde_json::from_str::<StyleSet>(&dup).unwrap_err().to_string();
        assert!(err.contains("duplicate style name \"S\""), "{err}");
    }

    #[test]
    fn styleset_rejects_names_shadowing_required_by_case() {
        for bad in ["s", "Hidden"] {
            let mut all: Vec<&str> = REQ.to_vec();
            all.push(bad);
            let set: StyleSet = serde_json::from_str(&set_json(&all)).unwrap();
            match set.validate() {
                Err(StyleError::ShadowsRequired { name, .. }) => assert_eq!(name, bad),
                other => panic!("unexpected {other:?}"),
            }
        }
        let mut all: Vec<&str> = REQ.to_vec();
        all.extend(["hidden2", "craftable"]);
        let set: StyleSet = serde_json::from_str(&set_json(&all)).unwrap();
        set.validate().unwrap();
    }

    #[test]
    fn styleset_rejects_quote_or_control_char_in_custom_sound() {
        for bad in ["a\\\"b", "a\\nb"] {
            let mut set = StyleSet::committed_for_tests();
            let mut st = set.get("S").unwrap().clone();
            let json = format!(r#"{{"id":null,"custom":"{bad}","vol":100}}"#);
            st.sound = Some(serde_json::from_str(&json).unwrap());
            set.insert("S", st);
            match set.validate() {
                Err(StyleError::InvalidCustomSound { style, .. }) => assert_eq!(style, "S"),
                other => panic!("unexpected {other:?}"),
            }
        }
        StyleSet::committed_for_tests().validate().unwrap();
    }

    #[test]
    fn styleset_keeps_file_order_with_extra_user_style() {
        let mut set = StyleSet::committed_for_tests();
        let st = set.get("unchanged").unwrap().clone();
        set.insert("craftable", st);
        set.validate().unwrap();
        assert_eq!(set.names().last(), Some("craftable"));
        assert_eq!(set.len(), 13);
        assert!(set.remove("exceptional").is_some());
        assert_eq!(set.len(), 12);
    }

    #[test]
    fn styleset_rejects_bad_colour_and_unknown_field() {
        let bad = STYLES_JSON.replacen("\"Pink\"", "\"Mauve\"", 1);
        assert!(serde_json::from_str::<StyleSet>(&bad).is_err());
        let bad = STYLES_JSON.replacen("\"font\"", "\"fnot\"", 1);
        assert!(serde_json::from_str::<StyleSet>(&bad).is_err());
    }
}
