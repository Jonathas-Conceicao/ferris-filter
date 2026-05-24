// populated in plan 02 — tests added in RED phase, implementation in GREEN phase

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
        // Semantic equality: re-serialize and compare strings (both came from same typed repr)
        let first_s = serde_json::to_string(&first).unwrap();
        let second_s = serde_json::to_string(&second).unwrap();
        assert_eq!(first_s, second_s, "config round-trip produced different result");
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
