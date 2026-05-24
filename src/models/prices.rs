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

/// A single price entry from poe.ninja.
///
/// Any of `exalt`, `chaos`, or `div` may be `null` (not all currencies have
/// prices expressed in every unit). `base_type` is only present for items where
/// the base matters (e.g. unique items).
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PriceItem {
    pub name: String,
    #[serde(default)]
    pub base_type: Option<String>,
    pub exalt: Option<f64>,
    pub chaos: Option<f64>,
    pub div: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_real_prices_json_parses() {
        let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/prices.json"));
        let prices: PricesFile = serde_json::from_str(src).expect("parse prices.json");
        assert!(!prices.league.is_empty());
        assert!(!prices.fetched_at.is_empty());
        assert!(!prices.items.is_empty());
    }

    #[test]
    fn round_trip_prices_file() {
        let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/prices.json"));
        let prices: PricesFile = serde_json::from_str(src).unwrap();
        let serialized = serde_json::to_string(&prices).unwrap();
        let prices2: PricesFile =
            serde_json::from_str(&serialized).expect("round-trip failed");
        assert_eq!(prices.league, prices2.league);
        assert_eq!(prices.items.len(), prices2.items.len());
    }

    #[test]
    fn item_with_null_exalt_parses() {
        let json = r#"{
            "name": "Chaos Orb",
            "exalt": null,
            "chaos": 1.0,
            "div": null
        }"#;
        let item: PriceItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.name, "Chaos Orb");
        assert!(item.exalt.is_none());
        assert_eq!(item.chaos, Some(1.0));
        assert!(item.div.is_none());
        assert!(item.base_type.is_none());
    }

    #[test]
    fn item_with_base_type_parses() {
        let json = r#"{
            "name": "Mirror of Kalandra",
            "baseType": "Mirror of Kalandra",
            "exalt": null,
            "chaos": null,
            "div": 8000.0
        }"#;
        let item: PriceItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.base_type, Some("Mirror of Kalandra".to_string()));
        assert_eq!(item.div, Some(8000.0));
    }
}
