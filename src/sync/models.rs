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
    pub name: String,    // e.g. "Fate of the Vaal" — USE THIS as league= query param
    pub url: String,     // slug e.g. "vaal" / "standard" / "hardcore"
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
}

#[derive(serde::Deserialize)]
pub struct ExchangeCore {
    #[serde(default)]
    pub items: Vec<ExchangeItem>,
    #[serde(default)]
    pub rates: HashMap<String, f64>, // keys: "chaos", "exalted", ...
    pub primary: String,             // e.g. "divine"
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

    #[test]
    fn test_empty_overview_deserializes_to_empty_vecs() {
        let overview: super::ExchangeOverview = serde_json::from_str(EMPTY_OVERVIEW_FIXTURE)
            .expect("Empty ExchangeOverview should deserialize without error");
        assert!(overview.core.items.is_empty(), "items should be empty");
        assert!(overview.lines.is_empty(), "lines should be empty");
        assert!(overview.core.rates.is_empty(), "rates should be empty");
    }
}
