use serde::{Deserialize, Serialize};

/// Top-level wrapper for `prices.json`.
///
/// The first three items in `items` are always Exalt, Chaos, Divine (by convention).
/// `fetched_at` maps to the JSON key `fetchedAt` via `camelCase` rename.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PricesFile {
    pub league: String,
    pub fetched_at: String,
    pub items: Vec<PriceItem>,
}

/// A single price entry in `prices.json`.
///
/// `name` = display name (poe.ninja); `base_type` = the filter `BaseType ==` value
/// (same as `name` for currency bases). `class` = the filter `Class ==` value.
/// `category` = grouping category (e.g. "Currency", "Runes", "Breach").
///
/// Any of `exalt`, `chaos`, `div` may be `null` — not all items have prices in
/// every unit. The first three items in the file are always Exalt/Chaos/Divine
/// and serve as cross-rate anchors (D-03/SYNC-02).
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PriceItem {
    pub name: String,
    pub base_type: String,
    pub class: String,
    pub category: String,
    pub exalt: Option<f64>,
    pub chaos: Option<f64>,
    pub div: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_real_prices_json_parses() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/seed-prices.json"
        ));
        let prices: PricesFile = serde_json::from_str(src).expect("parse prices.json");
        assert!(!prices.league.is_empty());
        assert!(!prices.fetched_at.is_empty());
        assert!(!prices.items.is_empty());
    }

    #[test]
    fn round_trip_prices_file() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/seed-prices.json"
        ));
        let prices: PricesFile = serde_json::from_str(src).unwrap();
        let serialized = serde_json::to_string(&prices).unwrap();
        let prices2: PricesFile = serde_json::from_str(&serialized).expect("round-trip failed");
        assert_eq!(prices.league, prices2.league);
        assert_eq!(prices.items.len(), prices2.items.len());
    }

    #[test]
    fn item_with_null_exalt_parses() {
        let json = r#"{
            "name": "Chaos Orb",
            "baseType": "Chaos Orb",
            "class": "Stackable Currency",
            "category": "Currency",
            "exalt": null,
            "chaos": 1.0,
            "div": null
        }"#;
        let item: PriceItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.name, "Chaos Orb");
        assert_eq!(item.base_type, "Chaos Orb");
        assert_eq!(item.class, "Stackable Currency");
        assert_eq!(item.category, "Currency");
        assert!(item.exalt.is_none());
        assert_eq!(item.chaos, Some(1.0));
        assert!(item.div.is_none());
    }

    #[test]
    fn item_with_base_type_parses() {
        let json = r#"{
            "name": "Mirror of Kalandra",
            "baseType": "Mirror of Kalandra",
            "class": "Stackable Currency",
            "category": "Currency",
            "exalt": null,
            "chaos": null,
            "div": 8000.0
        }"#;
        let item: PriceItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.base_type, "Mirror of Kalandra");
        assert_eq!(item.class, "Stackable Currency");
        assert_eq!(item.category, "Currency");
        assert_eq!(item.div, Some(8000.0));
    }

    /// All three reference orb anchors must be present as first three items,
    /// with the correct chaos values and required class/category fields.
    /// This test passes only once the seeded prices.json has been committed (Task 3).
    #[test]
    fn seeded_prices_anchors_first() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/seed-prices.json"
        ));
        let prices: PricesFile = serde_json::from_str(src).expect("parse prices.json");
        assert!(
            prices.items.len() > 300,
            "expected ~358+ items, got {}",
            prices.items.len()
        );

        let exalt = &prices.items[0];
        assert_eq!(exalt.name, "Exalted Orb", "items[0] must be Exalted Orb");
        assert_eq!(exalt.chaos, Some(4.5), "Exalted Orb chaos must be 4.5");

        let chaos = &prices.items[1];
        assert_eq!(chaos.name, "Chaos Orb", "items[1] must be Chaos Orb");
        assert_eq!(chaos.chaos, Some(1.0), "Chaos Orb chaos must be 1.0");

        let divine = &prices.items[2];
        assert_eq!(divine.name, "Divine Orb", "items[2] must be Divine Orb");
        assert_eq!(divine.chaos, Some(220.0), "Divine Orb chaos must be 220.0");
    }

    /// No item in the committed seed has name != base_type (no uniques in seed — D-08).
    #[test]
    fn seeded_prices_no_uniques() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/seed-prices.json"
        ));
        let prices: PricesFile = serde_json::from_str(src).expect("parse prices.json");
        for item in &prices.items {
            assert_eq!(
                item.name, item.base_type,
                "item '{}' has name != baseType (unexpected unique in seed)",
                item.name
            );
        }
    }
}
