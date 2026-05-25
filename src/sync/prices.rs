// Price building: join, convert, gem filter, anchor ordering.
// Implements build_prices, parse_gem_level, write_prices_atomic, run_sync_prices.

use crate::models::prices::{PriceItem, PricesFile};
use crate::sync::mapping::Mapping;
use crate::sync::models::ExchangeOverview;

pub const OVERVIEW_URL: &str =
    "https://poe.ninja/poe2/api/economy/exchange/current/overview";

/// Parse the gem level from an item name or id slug.
///
/// Handles two known formats:
/// - Parenthetical: "Uncut Spirit Gem (Level 18)" → Some(18)
/// - Slug suffix:   "uncut-spirit-gem-l18"        → Some(18)
///
/// Returns None for strings that don't contain a recognizable level pattern.
pub(crate) fn parse_gem_level(s: &str) -> Option<u32> {
    // Pattern 1: "(Level N)" in the display name
    if let Some(start) = s.find("(Level ") {
        let rest = &s[start + 7..];
        if let Some(end) = rest.find(')') {
            return rest[..end].trim().parse().ok();
        }
    }

    // Pattern 2: "-lN" suffix in an id slug (e.g. "uncut-spirit-gem-l18")
    if let Some(pos) = s.rfind("-l") {
        let level_str = &s[pos + 2..];
        // Consume digits only
        let digits: String = level_str.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits.parse().ok();
        }
    }

    None
}

/// Join `core.items` + `lines` on `id`, convert `primaryValue` to chaos/div/exalt
/// via `core.rates`, filter uncut gems to level ≥ minLevel, and promote
/// Exalted/Chaos/Divine Orbs to positions [0], [1], [2].
///
/// # Guarantees
/// - Empty `lines` → contributes zero items, no error (Pitfall 1).
/// - Empty `rates` → type skipped, no error (Pitfall 2).
/// - Each reference orb appears exactly once.
/// - items[0]="Exalted Orb", items[1]="Chaos Orb", items[2]="Divine Orb" (Pitfall 6).
pub fn build_prices(
    overviews: &[(String, ExchangeOverview)],
    mapping: &Mapping,
) -> miette::Result<Vec<PriceItem>> {
    let mut body: Vec<PriceItem> = Vec::new();

    // Track which overview types we've already collected reference orbs from.
    // We use a map from orb name → PriceItem for the three anchor slots.
    let anchor_names = ["Exalted Orb", "Chaos Orb", "Divine Orb"];
    let mut anchors: std::collections::HashMap<String, PriceItem> =
        std::collections::HashMap::new();

    for (type_value, overview) in overviews {
        // Determine if this is the gemFilter type or a regular overviewType
        let is_gem_type = type_value == &mapping.gem_filter.r#type;

        // Determine category/class from mapping
        let (category, class, base_type_is_name) = if is_gem_type {
            (
                mapping.gem_filter.category.clone(),
                mapping.gem_filter.class.clone(),
                true, // gems: baseType = name
            )
        } else {
            match mapping.lookup(type_value) {
                Some(ot) => (
                    ot.category.clone(),
                    ot.class.clone().unwrap_or_default(),
                    ot.base_type_is_name,
                ),
                None => {
                    eprintln!("Warning: unmapped poe.ninja type '{}', skipping", type_value);
                    continue;
                }
            }
        };

        // Pitfall 1: skip empty lines (off-season / not yet priced)
        if overview.lines.is_empty() {
            eprintln!(
                "Warning: poe.ninja type '{}' returned empty lines, skipping",
                type_value
            );
            continue;
        }

        // Pitfall 2: skip if rates are empty (no conversion possible)
        if overview.core.rates.is_empty() {
            eprintln!(
                "Warning: poe.ninja type '{}' has empty rates, skipping (no conversion possible)",
                type_value
            );
            continue;
        }

        // Build id→name index from core.items
        let item_index: std::collections::HashMap<&str, &str> = overview
            .core
            .items
            .iter()
            .map(|i| (i.id.as_str(), i.name.as_str()))
            .collect();

        // Price conversion factors from core.rates
        let chaos_rate = overview.core.rates.get("chaos").copied();
        let exalt_rate = overview.core.rates.get("exalted").copied();

        let compute_prices = |primary_value: f64| -> (Option<f64>, Option<f64>, Option<f64>) {
            // core.primary is the currency that primaryValue is expressed in
            let (chaos, div, exalt) = match overview.core.primary.as_str() {
                "divine" => {
                    // primaryValue is in divine
                    let chaos = chaos_rate.map(|r| primary_value * r);
                    let div = Some(primary_value);
                    let exalt = exalt_rate.map(|r| primary_value * r);
                    (chaos, div, exalt)
                }
                "chaos" => {
                    // primaryValue is in chaos
                    let chaos = Some(primary_value);
                    let divine_rate = overview.core.rates.get("divine").copied();
                    // Guard the divisor: a zero rate (thin/off-season market with the
                    // key present but zero) would yield inf/NaN, which serde_json
                    // serializes as null and silently drops the price. Skip those.
                    let div = divine_rate.and_then(|r| (r != 0.0).then_some(primary_value / r));
                    let exalt = exalt_rate.and_then(|r| (r != 0.0).then_some(primary_value / r));
                    (chaos, div, exalt)
                }
                _ => {
                    // Unknown primary — try to use chaos rate if present
                    let chaos = chaos_rate.map(|r| primary_value * r);
                    (chaos, None, None)
                }
            };
            (chaos, div, exalt)
        };

        // Process each line
        for line in &overview.lines {
            let name = match item_index.get(line.id.as_str()) {
                Some(n) => n.to_string(),
                None => {
                    // Fallback: use id as display name when core.items is missing this id
                    line.id.clone()
                }
            };

            // Gem level filter
            if is_gem_type {
                let level = parse_gem_level(&name)
                    .or_else(|| parse_gem_level(&line.id))
                    .unwrap_or(0);
                if level < mapping.gem_filter.min_level {
                    continue;
                }
            }

            let base_type = if base_type_is_name {
                name.clone()
            } else {
                // For uniques: use name as baseType (best available from poe.ninja)
                name.clone()
            };

            let (chaos, div, exalt) = compute_prices(line.primary_value);

            let item = PriceItem {
                name: name.clone(),
                base_type,
                class: class.clone(),
                category: category.clone(),
                exalt,
                chaos,
                div,
            };

            // Separate reference orbs from body items
            if anchor_names.contains(&name.as_str()) {
                anchors.insert(name, item);
            } else {
                body.push(item);
            }
        }
    }

    // Apply explicit overrides to the three reference orbs (by definition):
    // Exalted Orb: exalt = 1.0
    // Chaos Orb: chaos = 1.0
    // (Divine Orb: div = 1.0 when primary=="divine" — already set by compute_prices)
    if let Some(orb) = anchors.get_mut("Exalted Orb") {
        orb.exalt = Some(1.0);
    }
    if let Some(orb) = anchors.get_mut("Chaos Orb") {
        orb.chaos = Some(1.0);
    }

    // Assemble: Exalted Orb [0], Chaos Orb [1], Divine Orb [2], then body
    let mut items: Vec<PriceItem> = Vec::new();
    for anchor_name in &anchor_names {
        if let Some(item) = anchors.remove(*anchor_name) {
            items.push(item);
        }
    }
    items.extend(body);

    Ok(items)
}

/// Atomically write `prices.json` via `NamedTempFile::persist` (same-dir).
///
/// Refuses to write if `prices.items.is_empty()` (T-04-11, SYNC-04).
/// The existing file is never in a partial state (T-04-12).
pub fn write_prices_atomic(path: &str, prices: &PricesFile) -> miette::Result<()> {
    if prices.items.is_empty() {
        return Err(miette::miette!(
            "Refusing to write empty prices.json — no items to write"
        ));
    }

    let json = serde_json::to_string_pretty(prices)
        .map_err(|e| miette::miette!("Cannot serialize prices: {e}"))?;

    // Create temp file in SAME directory as target (Pitfall 4: cross-device)
    let target = std::path::Path::new(path);
    let dir = target.parent().unwrap_or(std::path::Path::new("."));

    let mut temp = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| miette::miette!("Cannot create temp file in {}: {e}", dir.display()))?;

    std::io::Write::write_all(&mut temp, json.as_bytes())
        .map_err(|e| miette::miette!("Cannot write temp file: {e}"))?;

    temp.persist(target)
        .map_err(|e| miette::miette!("Cannot persist prices.json: {e}"))?;

    Ok(())
}

/// Full sync prices pipeline:
/// 1. Fetch index-state → resolve league name.
/// 2. Load committed mapping.json.
/// 3. Fetch each overviewType + gemFilter type from poe.ninja.
/// 4. build_prices → validate anchors → write prices.json atomically.
pub async fn run_sync_prices<C: crate::sync::client::HttpClient>(
    client: &C,
    standard: bool,
    hc: bool,
) -> miette::Result<()> {
    use crate::sync::league::{fetch_index_state, resolve_league};
    use crate::sync::mapping::load_mapping;

    // Step 1: resolve league
    let state = fetch_index_state(client).await?;
    let league_name = resolve_league(&state, standard, hc)?;
    println!("Resolved league: {league_name}");

    // Step 2: load committed mapping
    let mapping = load_mapping("mapping.json")?;

    // Step 3: fetch each overview type
    let mut overviews: Vec<(String, ExchangeOverview)> = Vec::new();

    // Regular overviewTypes
    for ot in &mapping.overview_types {
        let query = [
            ("league", league_name.as_str()),
            ("type", ot.r#type.as_str()),
        ];
        match client.get_json(OVERVIEW_URL, &query).await {
            Ok(body) => match serde_json::from_str::<ExchangeOverview>(&body) {
                Ok(ov) => {
                    overviews.push((ot.r#type.clone(), ov));
                }
                Err(e) => {
                    eprintln!(
                        "Warning: failed to parse overview for type '{}': {e}",
                        ot.r#type
                    );
                }
            },
            Err(e) => {
                eprintln!(
                    "Warning: failed to fetch overview for type '{}': {e}",
                    ot.r#type
                );
            }
        }
    }

    // gemFilter type
    {
        let gem_type = &mapping.gem_filter.r#type;
        let query = [
            ("league", league_name.as_str()),
            ("type", gem_type.as_str()),
        ];
        match client.get_json(OVERVIEW_URL, &query).await {
            Ok(body) => match serde_json::from_str::<ExchangeOverview>(&body) {
                Ok(ov) => {
                    overviews.push((gem_type.clone(), ov));
                }
                Err(e) => {
                    eprintln!("Warning: failed to parse UncutGem overview: {e}");
                }
            },
            Err(e) => {
                eprintln!("Warning: failed to fetch UncutGem overview: {e}");
            }
        }
    }

    // Step 4: build prices
    let items = build_prices(&overviews, &mapping)?;

    // Validate anchors present and non-zero (SYNC-04 + T-04-11)
    let validate = || -> miette::Result<()> {
        if items.is_empty() {
            return Err(miette::miette!(
                "build_prices produced no items — aborting write"
            ));
        }
        let exalt = items.get(0).filter(|i| i.name == "Exalted Orb");
        let chaos = items.get(1).filter(|i| i.name == "Chaos Orb");
        let divine = items.get(2).filter(|i| i.name == "Divine Orb");

        match (exalt, chaos, divine) {
            (Some(e), Some(c), Some(d)) => {
                let chaos_val = c.chaos.unwrap_or(0.0);
                let div_val = d.div.unwrap_or(0.0);
                let exalt_val = e.exalt.unwrap_or(0.0);
                if chaos_val == 0.0 && c.chaos.is_some() {
                    return Err(miette::miette!("Chaos Orb chaos value is zero"));
                }
                // Divine Orb is the cross-rate anchor. A missing/zero div means the
                // rate table was malformed; aborting preserves the existing good file.
                if div_val == 0.0 {
                    return Err(miette::miette!(
                        "Divine Orb div anchor is zero/missing — aborting write to preserve existing prices.json"
                    ));
                }
                let _ = exalt_val; // non-fatal if null
                Ok(())
            }
            _ => Err(miette::miette!(
                "Validation failed: first three items are not Exalted/Chaos/Divine Orb. \
                 Got: [{}] — aborting write to preserve existing prices.json",
                items
                    .iter()
                    .take(3)
                    .map(|i| i.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    };
    validate()?;

    // Build PricesFile with resolved league + timestamp
    let fetched_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            // Format as ISO-8601 UTC string for readability
            let secs = d.as_secs();
            format_utc_timestamp(secs)
        })
        .unwrap_or_else(|_| "0".to_string());

    let prices_file = PricesFile {
        league: league_name.clone(),
        fetched_at,
        items,
    };

    write_prices_atomic("prices.json", &prices_file)?;

    println!(
        "Wrote {} items to prices.json (league: {})",
        prices_file.items.len(),
        league_name
    );

    Ok(())
}

/// Format a Unix timestamp (seconds since epoch) as an ISO-8601 UTC string.
/// Uses only std (no chrono dependency).
fn format_utc_timestamp(secs: u64) -> String {
    // Simple implementation: calculate YYYY-MM-DDTHH:MM:SSZ from seconds
    const SECS_PER_MIN: u64 = 60;
    const SECS_PER_HOUR: u64 = 3600;
    const SECS_PER_DAY: u64 = 86400;

    let mut remaining = secs;
    let time_of_day = remaining % SECS_PER_DAY;
    let days_since_epoch = remaining / SECS_PER_DAY;

    let hours = time_of_day / SECS_PER_HOUR;
    remaining = time_of_day % SECS_PER_HOUR;
    let minutes = remaining / SECS_PER_MIN;
    let seconds = remaining % SECS_PER_MIN;

    // Calculate year/month/day from days_since_epoch (1970-01-01 = day 0)
    let (year, month, day) = days_to_ymd(days_since_epoch);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap_year(year);
    let month_days: &[u64] = if leap {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &md in month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

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
        let items = super::build_prices(&[("Currency".to_string(), overview)], &mapping)
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
        let items = super::build_prices(&[("Currency".to_string(), overview)], &mapping)
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
        let items = super::build_prices(&[("Currency".to_string(), overview)], &mapping)
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
        let items = super::build_prices(&[("Currency".to_string(), overview)], &mapping)
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
        let result = super::build_prices(
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
        let result = super::build_prices(&[("Currency".to_string(), overview)], &mapping);
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
        let items = super::build_prices(
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
            super::parse_gem_level("Uncut Spirit Gem (Level 18)"),
            Some(18)
        );
        assert_eq!(
            super::parse_gem_level("Uncut Spirit Gem (Level 20)"),
            Some(20)
        );
        assert_eq!(
            super::parse_gem_level("Uncut Spirit Gem (Level 16)"),
            Some(16)
        );
    }

    #[test]
    fn parse_gem_level_from_id_slug() {
        assert_eq!(super::parse_gem_level("uncut-spirit-gem-l18"), Some(18));
        assert_eq!(super::parse_gem_level("uncut-spirit-gem-l20"), Some(20));
        assert_eq!(super::parse_gem_level("uncut-spirit-gem-l16"), Some(16));
    }

    #[test]
    fn parse_gem_level_returns_none_for_non_gem() {
        assert_eq!(super::parse_gem_level("Divine Orb"), None);
        assert_eq!(super::parse_gem_level("chaos"), None);
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
        let items = super::build_prices(&[("Currency".to_string(), overview)], &mapping)
            .expect("should succeed");

        let exalt_count = items.iter().filter(|i| i.name == "Exalted Orb").count();
        let chaos_count = items.iter().filter(|i| i.name == "Chaos Orb").count();
        let divine_count = items.iter().filter(|i| i.name == "Divine Orb").count();
        assert_eq!(exalt_count, 1, "Exalted Orb should appear exactly once");
        assert_eq!(chaos_count, 1, "Chaos Orb should appear exactly once");
        assert_eq!(divine_count, 1, "Divine Orb should appear exactly once");
    }

    // -----------------------------------------------------------------------
    // Test: write_prices_atomic with empty-items PricesFile returns Err
    // -----------------------------------------------------------------------
    #[test]
    fn write_prices_atomic_refuses_empty_items() {
        use crate::models::prices::PricesFile;
        let prices = PricesFile {
            league: "Test".to_string(),
            fetched_at: "2026-01-01T00:00:00Z".to_string(),
            items: vec![],
        };
        let result = super::write_prices_atomic("prices-test-empty.json", &prices);
        assert!(
            result.is_err(),
            "write_prices_atomic should return Err for empty items"
        );
        // Ensure no file was created
        assert!(
            !std::path::Path::new("prices-test-empty.json").exists(),
            "no file should be created for empty items"
        );
    }
}
