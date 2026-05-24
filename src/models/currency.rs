// populated in plan 02 — tests added in RED phase, implementation in GREEN phase

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENCY_JSON: &str =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/currency.json"));

    #[test]
    fn currency_parses_real_file() {
        let result: Result<CurrencyData, _> = serde_json::from_str(CURRENCY_JSON);
        assert!(result.is_ok(), "currency.json failed to parse: {:?}", result.err());
    }

    #[test]
    fn currency_round_trips() {
        let first: CurrencyData = serde_json::from_str(CURRENCY_JSON).unwrap();
        let serialized = serde_json::to_string(&first).unwrap();
        let second: CurrencyData = serde_json::from_str(&serialized).unwrap();
        let first_s = serde_json::to_string(&first).unwrap();
        let second_s = serde_json::to_string(&second).unwrap();
        assert_eq!(first_s, second_s, "currency round-trip produced different result");
    }

    #[test]
    fn currency_rejects_unknown_key() {
        let bad = r#"{"categories": [], "unknown_field": true}"#;
        let result: Result<CurrencyData, _> = serde_json::from_str(bad);
        assert!(result.is_err(), "expected error for unknown key, got Ok");
    }

    #[test]
    fn currency_has_15_categories() {
        let data: CurrencyData = serde_json::from_str(CURRENCY_JSON).unwrap();
        assert_eq!(data.categories.len(), 15, "expected 15 categories");
    }

    #[test]
    fn currency_has_mirror_of_kalandra() {
        let data: CurrencyData = serde_json::from_str(CURRENCY_JSON).unwrap();
        let currency_cat = data.categories.iter().find(|c| c.name == "Currency").unwrap();
        let s_entry = currency_cat.entries.iter().find(|e| e.tier == "S").unwrap();
        assert!(
            s_entry.bases.contains(&"Mirror of Kalandra".to_string()),
            "Mirror of Kalandra not found in Currency S"
        );
    }
}
