// Price building: join, convert, gem filter, anchor ordering.
// This module will implement build_prices, parse_gem_level, write_prices_atomic,
// and run_sync_prices. Tests drive the RED→GREEN→REFACTOR TDD cycle.

// Stub imports that tests will exercise
#[allow(unused_imports)]
use crate::models::prices::{PriceItem, PricesFile};
#[allow(unused_imports)]
use crate::sync::mapping::Mapping;
#[allow(unused_imports)]
use crate::sync::models::ExchangeOverview;

#[cfg(test)]
mod tests {
    use crate::models::prices::PriceItem;
    use crate::sync::mapping::Mapping;
    use crate::sync::models::{ExchangeCore, ExchangeItem, ExchangeLine, ExchangeOverview};
    use std::collections::HashMap;

    // -----------------------------------------------------------------------
    // Fixture helpers
    // -----------------------------------------------------------------------

    fn make_overview(
        primary: &str,
        rates: &[(&str, f64)],
        items: &[(&str, &str)],   // (id, name)
        lines: &[(&str, f64)],    // (id, primaryValue)
    ) -> ExchangeOverview {
        let rates_map: HashMap<String, f64> =
            rates.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let items_vec: Vec<ExchangeItem> = items
            .iter()
            .map(|(id, name)| ExchangeItem {
                id: id.to_string(),
                name: name.to_string(),
                category: None,
            })
            .collect();
        let lines_vec: Vec<ExchangeLine> = lines
            .iter()
            .map(|(id, pv)| ExchangeLine {
                id: id.to_string(),
                primary_value: *pv,
            })
            .collect();
        ExchangeOverview {
            core: ExchangeCore {
                items: items_vec,
                rates: rates_map,
                primary: primary.to_string(),
            },
            lines: lines_vec,
        }
    }

    fn minimal_mapping() -> Mapping {
        let src = r#"{
            "version": 1,
            "overviewTypes": [
                {
                    "type": "Currency",
                    "category": "Currency",
                    "class": "Stackable Currency",
                    "baseTypeIsName": true
                }
            ],
            "gemFilter": {
                "type": "UncutGem",
                "category": "Uncut Gems",
                "class": "Stackable Currency",
                "minLevel": 18
            }
        }"#;
        serde_json::from_str(src).expect("minimal mapping should parse")
    }

    fn gem_mapping() -> Mapping {
        let src = r#"{
            "version": 1,
            "overviewTypes": [],
            "gemFilter": {
                "type": "UncutGem",
                "category": "Uncut Gems",
                "class": "Stackable Currency",
                "minLevel": 18
            }
        }"#;
        serde_json::from_str(src).expect("gem mapping should parse")
    }

    // -----------------------------------------------------------------------
    // Test: conversion math
    //
    // When core.primary="divine" and rates = {chaos: 32.24, exalted: 149.0},
    // an item with primaryValue=0.5 should yield:
    //   chaos  = 0.5 * 32.24  = 16.12
    //   div    = 0.5           = 0.5
    //   exalt  = 0.5 * 149.0  = 74.5
    // -----------------------------------------------------------------------
    #[test]
    fn conversion_math_divine_primary() {
        let overview = make_overview(
            "divine",
            &[("chaos", 32.24), ("exalted", 149.0)],
            &[("item-a", "Some Orb")],
            &[("item-a", 0.5)],
        );
        let mapping = minimal_mapping();
        let items = super::super::build_prices(&[("Currency".to_string(), overview)], &mapping)
            .expect("build_prices should succeed");

        // Find "Some Orb" (not the anchor orbs)
        let item = items.iter().find(|i| i.name == "Some Orb").expect("Some Orb must be present");
        let chaos = item.chaos.expect("chaos must be Some");
        let div = item.div.expect("div must be Some");
        let exalt = item.exalt.expect("exalt must be Some");

        assert!((chaos - 16.12).abs() < 0.01, "chaos={chaos}");
        assert!((div - 0.5).abs() < 0.001, "div={div}");
        assert!((exalt - 74.5).abs() < 0.1, "exalt={exalt}");
    }

    // -----------------------------------------------------------------------
    // Test: primaryValue is NOT used raw as chaos (Pitfall 2 guard)
    // With divine as primary and primaryValue=1.0 for Divine Orb,
    // chaos should be 32.24, NOT 1.0.
    // -----------------------------------------------------------------------
    #[test]
    fn primary_value_not_used_raw_as_chaos() {
        let overview = make_overview(
            "divine",
            &[("chaos", 32.24), ("exalted", 149.0)],
            &[("divine", "Divine Orb"), ("chaos", "Chaos Orb"), ("exalted", "Exalted Orb")],
            &[("divine", 1.0), ("chaos", 0.031), ("exalted", 0.0067)],
        );
        let mapping = minimal_mapping();
        let items = super::super::build_prices(&[("Currency".to_string(), overview)], &mapping)
            .expect("build_prices should succeed");

        // Divine Orb: primaryValue=1.0, divine primary → chaos = 1.0 * 32.24 = 32.24
        let divine = items.iter().find(|i| i.name == "Divine Orb").expect("Divine Orb must be present");
        let chaos_val = divine.chaos.expect("Divine Orb chaos must be Some");
        assert!(
            (chaos_val - 32.24).abs() < 0.1,
            "Divine Orb chaos should be ~32.24 (primaryValue * rate), not 1.0 (raw primaryValue). Got {chaos_val}"
        );
    }

    // -----------------------------------------------------------------------
    // Test: anchor ordering — Exalted[0], Chaos[1], Divine[2]
    // -----------------------------------------------------------------------
    #[test]
    fn anchors_first_in_correct_order() {
        // Include all three reference orbs + extra items in random order
        let overview = make_overview(
            "divine",
            &[("chaos", 32.24), ("exalted", 149.0)],
            &[
                ("divine", "Divine Orb"),
                ("chaos", "Chaos Orb"),
                ("exalted", "Exalted Orb"),
                ("other", "Other Orb"),
            ],
            &[
                ("other", 0.01),
                ("divine", 1.0),
                ("chaos", 0.031),
                ("exalted", 0.0067),
            ],
        );
        let mapping = minimal_mapping();
        let items = super::super::build_prices(&[("Currency".to_string(), overview)], &mapping)
            .expect("build_prices should succeed");

        assert!(items.len() >= 4, "expected at least 4 items");
        assert_eq!(items[0].name, "Exalted Orb", "items[0] must be Exalted Orb");
        assert_eq!(items[1].name, "Chaos Orb", "items[1] must be Chaos Orb");
        assert_eq!(items[2].name, "Divine Orb", "items[2] must be Divine Orb");
    }

    // -----------------------------------------------------------------------
    // Test: Exalted Orb exalt=1.0 and Chaos Orb chaos=1.0 (explicit override)
    // -----------------------------------------------------------------------
    #[test]
    fn reference_orb_overrides() {
        let overview = make_overview(
            "divine",
            &[("chaos", 32.24), ("exalted", 149.0)],
            &[
                ("divine", "Divine Orb"),
                ("chaos", "Chaos Orb"),
                ("exalted", "Exalted Orb"),
            ],
            &[("divine", 1.0), ("chaos", 0.031), ("exalted", 0.0067)],
        );
        let mapping = minimal_mapping();
        let items = super::super::build_prices(&[("Currency".to_string(), overview)], &mapping)
            .expect("build_prices should succeed");

        let exalt = items.iter().find(|i| i.name == "Exalted Orb").expect("Exalted Orb must exist");
        assert_eq!(exalt.exalt, Some(1.0), "Exalted Orb exalt must be 1.0 (by definition)");

        let chaos = items.iter().find(|i| i.name == "Chaos Orb").expect("Chaos Orb must exist");
        assert_eq!(chaos.chaos, Some(1.0), "Chaos Orb chaos must be 1.0 (by definition)");
    }

    // -----------------------------------------------------------------------
    // Test: empty lines overview contributes zero items and does NOT error (Pitfall 1)
    // -----------------------------------------------------------------------
    #[test]
    fn empty_lines_contributes_zero_items_no_error() {
        let empty_overview = make_overview("divine", &[("chaos", 32.24)], &[], &[]);
        let mapping = minimal_mapping();
        let result = super::super::build_prices(
            &[("Currency".to_string(), empty_overview)],
            &mapping,
        );
        assert!(result.is_ok(), "empty lines should not error");
        // Items list may be empty (no anchors either since no data)
        let items = result.unwrap();
        assert!(
            !items.iter().any(|i| i.name == "Some Orb"),
            "no items from empty overview"
        );
    }

    // -----------------------------------------------------------------------
    // Test: empty rates skips type (Pitfall 2 guard; no garbage conversion)
    // -----------------------------------------------------------------------
    #[test]
    fn empty_rates_skips_type() {
        let overview = make_overview(
            "divine",
            &[], // empty rates
            &[("item-a", "Some Orb")],
            &[("item-a", 0.5)],
        );
        let mapping = minimal_mapping();
        let result = super::super::build_prices(&[("Currency".to_string(), overview)], &mapping);
        assert!(result.is_ok(), "empty rates should not error");
        // Should have no items from this type (skipped due to empty rates)
        let items = result.unwrap();
        assert!(!items.iter().any(|i| i.name == "Some Orb"), "item should be skipped when rates empty");
    }

    // -----------------------------------------------------------------------
    // Test: gem level filter — levels {16,18,20} → only {18,20} survive
    // -----------------------------------------------------------------------
    #[test]
    fn gem_level_filter_keeps_only_min_level_and_above() {
        let gem_overview = make_overview(
            "divine",
            &[("chaos", 32.24), ("exalted", 149.0)],
            &[
                ("uncut-spirit-gem-l16", "Uncut Spirit Gem (Level 16)"),
                ("uncut-spirit-gem-l18", "Uncut Spirit Gem (Level 18)"),
                ("uncut-spirit-gem-l20", "Uncut Spirit Gem (Level 20)"),
            ],
            &[
                ("uncut-spirit-gem-l16", 0.1),
                ("uncut-spirit-gem-l18", 1.0),
                ("uncut-spirit-gem-l20", 5.0),
            ],
        );
        let mapping = gem_mapping();
        let items = super::super::build_prices(
            &[("UncutGem".to_string(), gem_overview)],
            &mapping,
        )
        .expect("build_prices with gem filter should succeed");

        assert!(
            !items.iter().any(|i| i.name.contains("Level 16")),
            "level 16 gem must be excluded"
        );
        assert!(
            items.iter().any(|i| i.name.contains("Level 18")),
            "level 18 gem must be included"
        );
        assert!(
            items.iter().any(|i| i.name.contains("Level 20")),
            "level 20 gem must be included"
        );
    }

    // -----------------------------------------------------------------------
    // Test: parse_gem_level helper
    // -----------------------------------------------------------------------
    #[test]
    fn parse_gem_level_from_name_parenthetical() {
        assert_eq!(
            super::super::parse_gem_level("Uncut Spirit Gem (Level 18)"),
            Some(18)
        );
        assert_eq!(
            super::super::parse_gem_level("Uncut Spirit Gem (Level 20)"),
            Some(20)
        );
        assert_eq!(
            super::super::parse_gem_level("Uncut Spirit Gem (Level 16)"),
            Some(16)
        );
    }

    #[test]
    fn parse_gem_level_from_id_slug() {
        assert_eq!(super::super::parse_gem_level("uncut-spirit-gem-l18"), Some(18));
        assert_eq!(super::super::parse_gem_level("uncut-spirit-gem-l20"), Some(20));
        assert_eq!(super::super::parse_gem_level("uncut-spirit-gem-l16"), Some(16));
    }

    #[test]
    fn parse_gem_level_returns_none_for_non_gem() {
        assert_eq!(super::super::parse_gem_level("Divine Orb"), None);
        assert_eq!(super::super::parse_gem_level("chaos"), None);
    }

    // -----------------------------------------------------------------------
    // Test: de-dup — each reference orb appears exactly once
    // -----------------------------------------------------------------------
    #[test]
    fn reference_orbs_deduplicated() {
        let overview = make_overview(
            "divine",
            &[("chaos", 32.24), ("exalted", 149.0)],
            &[
                ("divine", "Divine Orb"),
                ("chaos", "Chaos Orb"),
                ("exalted", "Exalted Orb"),
            ],
            &[("divine", 1.0), ("chaos", 0.031), ("exalted", 0.0067)],
        );
        let mapping = minimal_mapping();
        let items = super::super::build_prices(&[("Currency".to_string(), overview)], &mapping)
            .expect("should succeed");

        let exalt_count = items.iter().filter(|i| i.name == "Exalted Orb").count();
        let chaos_count = items.iter().filter(|i| i.name == "Chaos Orb").count();
        let divine_count = items.iter().filter(|i| i.name == "Divine Orb").count();
        assert_eq!(exalt_count, 1, "Exalted Orb should appear exactly once");
        assert_eq!(chaos_count, 1, "Chaos Orb should appear exactly once");
        assert_eq!(divine_count, 1, "Divine Orb should appear exactly once");
    }
}
