use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Currency models
// ---------------------------------------------------------------------------

/// One tier entry within a category. `tier` is a plain String ("S"/"A"/"B"/…)
/// for Phase 1 — will unify with a shared Tier enum in a later phase.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CurrencyEntry {
    pub tier: String,
    pub class: String,
    pub bases: Vec<String>,
}

/// One named currency category (e.g. "Currency", "Runes", "Omens").
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CurrencyCategory {
    pub name: String,
    pub entries: Vec<CurrencyEntry>,
}

/// Top-level deserialized representation of `currency.json`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CurrencyData {
    pub categories: Vec<CurrencyCategory>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
        // D-06: semantic round-trip — compare deserialized values for structural equality.
        assert_eq!(first, second, "currency round-trip produced semantically different result");
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
