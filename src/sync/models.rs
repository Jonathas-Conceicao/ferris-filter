use std::collections::HashMap;

// --- poe.ninja index-state ---
// GET https://poe.ninja/poe2/api/data/index-state

#[derive(serde::Deserialize)]
pub struct IndexState {
    #[serde(rename = "economyLeagues")]
    pub economy_leagues: Vec<EconomyLeague>,
}

#[derive(serde::Deserialize)]
pub struct EconomyLeague {
    pub name: String, // e.g. "Fate of the Vaal" — USE THIS as league= query param
    pub url: String,  // slug e.g. "vaal" / "standard" / "hardcore"
    pub hardcore: bool,
    pub indexed: bool,
}

// --- poe.ninja exchange overview ---
// GET https://poe.ninja/poe2/api/economy/exchange/current/overview?league={name}&type={type}
// NOTE: lenient parsing only — poe.ninja is undocumented (Pitfall 5 in RESEARCH.md).
// Use #[serde(default)] on collection fields so off-season empty responses deserialize cleanly.

#[derive(serde::Deserialize)]
pub struct ExchangeOverview {
    pub core: ExchangeCore,
    #[serde(default)]
    pub lines: Vec<ExchangeLine>,
    /// TOP-LEVEL item dictionary (sibling of `lines`, NOT `core.items`). This is the
    /// comprehensive id→name map — `core.items` only carries the 3 rate-reference
    /// currencies (divine/exalted/chaos). The bulk of currency names live here, so
    /// the id→name join must consult this array (see build_prices). Defaults to empty
    /// for off-season / older fixtures.
    #[serde(default)]
    pub items: Vec<ExchangeItem>,
}

#[derive(serde::Deserialize)]
pub struct ExchangeCore {
    #[serde(default)]
    pub items: Vec<ExchangeItem>,
    #[serde(default)]
    pub rates: HashMap<String, f64>, // keys: "chaos", "exalted", ...
    pub primary: String, // e.g. "divine"
}

#[derive(serde::Deserialize)]
pub struct ExchangeItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct ExchangeLine {
    pub id: String,
    #[serde(rename = "primaryValue")]
    pub primary_value: f64,
}

// --- poe.ninja stash item overview (uniques) ---
// GET https://poe.ninja/poe2/api/economy/stash/current/item/overview?league={name}&type={UniquePluralType}
// NOTE: this is a DIFFERENT endpoint family than the currency `exchange` overview.
// Unlike exchange (where `lines[].id` must be joined against `core.items`), each line
// here is self-contained: it carries `name`, `baseType`, `category`, and `primaryValue`
// directly. The `core` block still provides `rates`/`primary` for price conversion.
// Lenient parsing only — unknown fields ignored.

#[derive(serde::Deserialize)]
pub struct ItemOverview {
    pub core: ExchangeCore,
    #[serde(default)]
    pub lines: Vec<ItemLine>,
}

#[derive(serde::Deserialize)]
pub struct ItemLine {
    #[serde(default)]
    pub name: String, // unique display name, e.g. "Temporalis"
    #[serde(rename = "baseType", default)]
    pub base_type: String, // filter BaseType, e.g. "Silk Robe"
    #[serde(default)]
    pub category: Option<String>, // e.g. "Body Armour" (may be bracketed like "[Jewel]")
    #[serde(rename = "primaryValue")]
    pub primary_value: f64,
}

#[cfg(test)]
mod tests {
    // Test fixtures from 04-RESEARCH.md §SPIKE response structures

    const INDEX_STATE_FIXTURE: &str = r#"{
  "economyLeagues": [
    {"name": "Fate of the Vaal", "url": "vaal", "displayName": "Fate of the Vaal", "hardcore": false, "indexed": true},
    {"name": "HC Fate of the Vaal", "url": "vaalhc", "displayName": "HC Fate of the Vaal", "hardcore": true, "indexed": false},
    {"name": "Standard", "url": "standard", "displayName": "Standard", "hardcore": false, "indexed": false},
    {"name": "Hardcore", "url": "hardcore", "displayName": "Hardcore", "hardcore": true, "indexed": false}
  ],
  "oldEconomyLeagues": [],
  "snapshotVersions": []
}"#;

    const CURRENCY_OVERVIEW_FIXTURE: &str = r#"{
  "core": {
    "items": [
      {"id": "divine", "name": "Divine Orb", "image": "...", "category": "Currency", "detailsId": "divine-orb"},
      {"id": "exalted", "name": "Exalted Orb", "image": "...", "category": "Currency", "detailsId": "exalted-orb"},
      {"id": "chaos", "name": "Chaos Orb", "image": "...", "category": "Currency", "detailsId": "chaos-orb"}
    ],
    "rates": {
      "exalted": 149.0,
      "chaos": 32.24
    },
    "primary": "divine",
    "secondary": "exalted"
  },
  "lines": [
    {
      "id": "divine",
      "primaryValue": 1.0,
      "volumePrimaryValue": 1320.0
    },
    {
      "id": "chaos",
      "primaryValue": 0.03102,
      "volumePrimaryValue": 500.0
    }
  ],
  "items": []
}"#;

    const EMPTY_OVERVIEW_FIXTURE: &str = r#"{
  "core": {
    "items": [],
    "rates": {},
    "primary": "divine",
    "secondary": "exalted"
  },
  "lines": [],
  "items": []
}"#;

    #[test]
    fn test_index_state_deserializes() {
        let state: super::IndexState = serde_json::from_str(INDEX_STATE_FIXTURE)
            .expect("IndexState should deserialize from fixture");
        assert_eq!(state.economy_leagues.len(), 4);
        let first = &state.economy_leagues[0];
        assert_eq!(first.name, "Fate of the Vaal");
        assert_eq!(first.url, "vaal");
        assert!(!first.hardcore);
        assert!(first.indexed);
    }

    #[test]
    fn test_currency_overview_deserializes() {
        let overview: super::ExchangeOverview = serde_json::from_str(CURRENCY_OVERVIEW_FIXTURE)
            .expect("ExchangeOverview should deserialize from Currency fixture");
        assert_eq!(overview.core.items.len(), 3);
        assert_eq!(overview.lines.len(), 2);
        assert_eq!(overview.core.primary, "divine");
        let chaos_rate = overview.core.rates.get("chaos").copied().unwrap_or(0.0);
        assert!((chaos_rate - 32.24).abs() < 0.001);
    }

    // A real-shaped stash/item unique overview line: self-contained name + baseType.
    const ITEM_OVERVIEW_FIXTURE: &str = r#"{
  "core": {
    "items": [],
    "rates": { "exalted": 160.0, "chaos": 31.63 },
    "primary": "divine",
    "secondary": "exalted"
  },
  "lines": [
    {
      "id": 232,
      "itemId": "Temporalis Silk Robe",
      "detailsId": "temporalis-silk-robe",
      "name": "Temporalis",
      "baseType": "Silk Robe",
      "levelRequired": 64,
      "category": "Body Armour",
      "primaryValue": 19364.0,
      "listingCount": 65,
      "corrupted": false
    }
  ]
}"#;

    #[test]
    fn test_item_overview_deserializes() {
        let ov: super::ItemOverview = serde_json::from_str(ITEM_OVERVIEW_FIXTURE)
            .expect("ItemOverview should deserialize from stash/item fixture");
        assert_eq!(ov.lines.len(), 1);
        assert_eq!(ov.core.primary, "divine");
        let line = &ov.lines[0];
        assert_eq!(line.name, "Temporalis", "name is the unique display name");
        assert_eq!(
            line.base_type, "Silk Robe",
            "baseType is the filter base type"
        );
        assert_eq!(line.category.as_deref(), Some("Body Armour"));
        assert!((line.primary_value - 19364.0).abs() < 0.001);
    }

    #[test]
    fn test_empty_overview_deserializes_to_empty_vecs() {
        let overview: super::ExchangeOverview = serde_json::from_str(EMPTY_OVERVIEW_FIXTURE)
            .expect("Empty ExchangeOverview should deserialize without error");
        assert!(overview.core.items.is_empty(), "items should be empty");
        assert!(overview.lines.is_empty(), "lines should be empty");
        assert!(overview.core.rates.is_empty(), "rates should be empty");
    }
}
