// Price building: join, convert, gem filter, anchor ordering.
// Implements build_prices, parse_gem_level, write_prices_atomic, run_sync_prices.

use crate::models::prices::{PriceItem, PricesFile};
use crate::sync::mapping::Mapping;
use crate::sync::models::{ExchangeOverview, ItemOverview};

pub const OVERVIEW_URL: &str =
    "https://poe.ninja/poe2/api/economy/exchange/current/overview";

/// Stash item overview endpoint — serves uniques (and other non-fungible items),
/// keyed by plural `type` values (e.g. `UniqueWeapons`). Different shape than the
/// currency `exchange` overview: each line carries `name`/`baseType`/`category`.
pub const ITEM_OVERVIEW_URL: &str =
    "https://poe.ninja/poe2/api/economy/stash/current/item/overview";

/// Convert a `primaryValue` (expressed in `core.primary`) into (chaos, div, exalt)
/// using the overview's `core.rates`. Shared by the currency and unique pipelines.
///
/// Guards against a zero divisor in the `chaos`-primary branch: a present-but-zero
/// rate would otherwise yield inf/NaN, which serde_json serializes as null and
/// silently drops the price.
fn convert_primary(
    primary: &str,
    primary_value: f64,
    rates: &std::collections::HashMap<String, f64>,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    let chaos_rate = rates.get("chaos").copied();
    let exalt_rate = rates.get("exalted").copied();
    match primary {
        "divine" => {
            let chaos = chaos_rate.map(|r| primary_value * r);
            let div = Some(primary_value);
            let exalt = exalt_rate.map(|r| primary_value * r);
            (chaos, div, exalt)
        }
        "chaos" => {
            let chaos = Some(primary_value);
            let divine_rate = rates.get("divine").copied();
            let div = divine_rate.and_then(|r| (r != 0.0).then_some(primary_value / r));
            let exalt = exalt_rate.and_then(|r| (r != 0.0).then_some(primary_value / r));
            (chaos, div, exalt)
        }
        _ => {
            // Unknown primary — try to use chaos rate if present.
            let chaos = chaos_rate.map(|r| primary_value * r);
            (chaos, None, None)
        }
    }
}

/// Derive the per-subtype plural Class from an uncut gem base_type.
///
/// "Uncut Skill Gem (Level 20)"   → Some("Uncut Skill Gems")
/// "Uncut Spirit Gem (Level 18)"  → Some("Uncut Spirit Gems")
/// "Uncut Support Gem (Level 16)" → Some("Uncut Support Gems")
///
/// Returns None for base types that don't match the known uncut gem prefixes.
pub(crate) fn derive_uncut_class(base_type: &str) -> Option<String> {
    // Strip the " (Level N)" suffix to get the stem, then match.
    let stem = base_type
        .split(" (Level ")
        .next()
        .unwrap_or(base_type)
        .trim();
    match stem {
        "Uncut Skill Gem" => Some("Uncut Skill Gems".to_string()),
        "Uncut Spirit Gem" => Some("Uncut Spirit Gems".to_string()),
        "Uncut Support Gem" => Some("Uncut Support Gems".to_string()),
        _ => None,
    }
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
        // Determine if this is a gemFilters type by looking up in gem_filters Vec.
        let gem_filter = mapping.gem_filters.iter().find(|g| &g.r#type == type_value);
        let is_gem_type = gem_filter.is_some();

        // Determine category/class from mapping.
        // For gem types, category comes from the matched gem_filter; per-item class is
        // derived at the item level (derive_uncut_class or gem_filter.classes[0]).
        // For currency types, class comes from the overview type mapping.
        let (category, class, base_type_is_name) = if is_gem_type {
            let g = gem_filter.unwrap();
            (
                g.category.clone(),
                String::new(), // placeholder; real class derived per-item in the gem branch
                true,          // gems: baseType = name
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

        // Build id→name index. The comprehensive dictionary is the TOP-LEVEL `items`
        // array; `core.items` only holds the 3 rate-reference currencies. Merge both
        // (top-level wins, core.items fills any gaps) so every priced line resolves to
        // a real name/baseType instead of falling back to its kebab-case id slug.
        let mut item_index: std::collections::HashMap<&str, &str> = overview
            .items
            .iter()
            .map(|i| (i.id.as_str(), i.name.as_str()))
            .collect();
        for i in &overview.core.items {
            item_index.entry(i.id.as_str()).or_insert(i.name.as_str());
        }

        // Process each line
        for line in &overview.lines {
            let name = match item_index.get(line.id.as_str()) {
                Some(n) => n.to_string(),
                None => {
                    // poe.ninja occasionally returns a `lines` entry whose `id` is absent
                    // from `core.items`. The only thing we have for it is the `id` itself —
                    // a kebab-case slug (e.g. "vaal-cultivation-orb"). That slug is NOT a
                    // valid PoE2 `BaseType`, so emitting it would produce a filter rule that
                    // can never match. Skip it rather than poison the filter with a slug;
                    // real base types come from `core.items[].name` (baseType == name for
                    // currency-class items).
                    eprintln!(
                        "Warning: poe.ninja type '{}' line id '{}' has no entry in core.items \
                         — skipping (cannot resolve a valid BaseType)",
                        type_value, line.id
                    );
                    continue;
                }
            };

            // Gem branch: emit one PriceItem per level with the level KEPT in base_type.
            // No minLevel floor; no dedup — every level is a distinct PoE2 base type.
            if is_gem_type {
                let gf = gem_filter.unwrap();
                // Derive the per-item class from the base-type prefix.
                // UncutGems: prefix "Uncut Skill/Spirit/Support Gem" → plural class.
                // LineageSupportGems: derive_uncut_class returns None → fall back to
                // gem_filter.classes[0] (i.e. "Support Gems").
                let item_class = match derive_uncut_class(&name) {
                    Some(cls) => cls,
                    None => {
                        // Not an uncut gem prefix — use the first class from the filter config.
                        // This handles LineageSupportGems and any future gem categories.
                        match gf.classes.first() {
                            Some(cls) => cls.clone(),
                            None => {
                                eprintln!(
                                    "Warning: gem_filter '{}' has no classes configured; \
                                     skipping item '{}'",
                                    gf.r#type, name
                                );
                                continue;
                            }
                        }
                    }
                };

                let (chaos, div, exalt) = convert_primary(
                    &overview.core.primary,
                    line.primary_value,
                    &overview.core.rates,
                );

                body.push(PriceItem {
                    name: name.clone(),
                    base_type: name, // keep the full "(Level N)" name as the base type
                    class: item_class,
                    category: category.clone(),
                    exalt,
                    chaos,
                    div,
                });
                continue;
            }

            // For currency-style categories the poe.ninja display `name` IS the filter base
            // type (baseType == name). Uniques are handled separately by build_unique_items.
            let _ = base_type_is_name;
            let base_type = name.clone();

            let (chaos, div, exalt) =
                convert_primary(&overview.core.primary, line.primary_value, &overview.core.rates);

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

/// Build `PriceItem`s from the stash/item unique overviews.
///
/// Each line is self-contained (`name` + `baseType` + `primaryValue`), so there is no
/// id→core.items join. The produced items have `name != base_type` (the unique's display
/// name vs its filter base type), which is exactly what `pricer::price_tiers` keys on to
/// collect them into `Rarity == Unique` blocks (deduping shared base types to the highest
/// tier). Prices convert via the same `convert_primary` helper as currency.
///
/// Lines with no `baseType` are skipped (a `BaseType ==` rule needs a real base to match).
pub fn build_unique_items(
    overviews: &[(String, crate::sync::models::ItemOverview)],
    mapping: &Mapping,
) -> Vec<PriceItem> {
    let mut out: Vec<PriceItem> = Vec::new();

    for (type_value, overview) in overviews {
        // Category comes from the mapping's itemTypes entry; default to "Uniques".
        let category = mapping
            .item_types
            .iter()
            .find(|t| &t.r#type == type_value)
            .map(|t| t.category.clone())
            .unwrap_or_else(|| "Uniques".to_string());

        if overview.lines.is_empty() {
            continue;
        }
        if overview.core.rates.is_empty() {
            eprintln!("Warning: unique type '{type_value}' has empty rates, skipping (no conversion possible)");
            continue;
        }

        for line in &overview.lines {
            // A unique block matches `Rarity == Unique` + `BaseType ==`, so a real base
            // type is required. poe.ninja supplies it here (unlike the exchange endpoint).
            if line.base_type.trim().is_empty() {
                eprintln!(
                    "Warning: unique '{}' ({type_value}) has no baseType — skipping",
                    line.name
                );
                continue;
            }

            let (chaos, div, exalt) =
                convert_primary(&overview.core.primary, line.primary_value, &overview.core.rates);

            out.push(PriceItem {
                name: line.name.clone(),
                base_type: line.base_type.clone(),
                // Unique blocks match on Rarity == Unique + BaseType, not Class.
                class: String::new(),
                category: category.clone(),
                exalt,
                chaos,
                div,
            });
        }
    }

    out
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

    // Fetch each gem_filters entry (UncutGems, LineageSupportGems, etc.).
    // Each entry uses the same exchange overview endpoint as regular overviewTypes.
    if mapping.gem_filters.is_empty() {
        eprintln!("Warning: gem_filters is empty, skipping gem fetch");
    }
    for gf in &mapping.gem_filters {
        let query = [
            ("league", league_name.as_str()),
            ("type", gf.r#type.as_str()),
        ];
        match client.get_json(OVERVIEW_URL, &query).await {
            Ok(body) => match serde_json::from_str::<ExchangeOverview>(&body) {
                Ok(ov) => {
                    overviews.push((gf.r#type.clone(), ov));
                }
                Err(e) => {
                    eprintln!(
                        "Warning: failed to parse overview for gem type '{}': {e}",
                        gf.r#type
                    );
                }
            },
            Err(e) => {
                eprintln!(
                    "Warning: failed to fetch overview for gem type '{}': {e}",
                    gf.r#type
                );
            }
        }
    }

    // Step 3b: fetch unique item overviews from the stash/item endpoint.
    // This is a SEPARATE endpoint family from the currency exchange overview, with a
    // different response shape (each line carries name + baseType directly). The
    // unique `type` values are plural (e.g. UniqueWeapons) — see mapping.itemTypes.
    let mut item_overviews: Vec<(String, ItemOverview)> = Vec::new();
    for it in &mapping.item_types {
        let query = [
            ("league", league_name.as_str()),
            ("type", it.r#type.as_str()),
        ];
        match client.get_json(ITEM_OVERVIEW_URL, &query).await {
            Ok(body) => match serde_json::from_str::<ItemOverview>(&body) {
                Ok(ov) => item_overviews.push((it.r#type.clone(), ov)),
                Err(e) => {
                    eprintln!("Warning: failed to parse item overview for type '{}': {e}", it.r#type);
                }
            },
            Err(e) => {
                eprintln!("Warning: failed to fetch item overview for type '{}': {e}", it.r#type);
            }
        }
    }

    // Step 4: build prices (currency/gems first, then uniques appended).
    let mut items = build_prices(&overviews, &mapping)?;
    let unique_items = build_unique_items(&item_overviews, &mapping);
    let unique_count = unique_items.len();
    items.extend(unique_items);

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
        "Wrote {} items to prices.json (league: {}, {} uniques)",
        prices_file.items.len(),
        league_name,
        unique_count
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
            // Helper puts the dictionary in core.items; the top-level `items` array is
            // exercised separately by `top_level_items_resolves_names`.
            items: Vec::new(),
        }
    }

    fn minimal_mapping() -> Mapping {
        let src = r#"{
            "version": 2,
            "overviewTypes": [
                {
                    "type": "Currency",
                    "category": "Currency",
                    "class": "Stackable Currency",
                    "baseTypeIsName": true
                }
            ],
            "gemFilters": [
                {
                    "type": "UncutGems",
                    "category": "Uncut Gems",
                    "classes": ["Uncut Skill Gems", "Uncut Spirit Gems", "Uncut Support Gems"]
                }
            ]
        }"#;
        serde_json::from_str(src).expect("minimal mapping should parse")
    }

    fn gem_mapping() -> Mapping {
        let src = r#"{
            "version": 2,
            "overviewTypes": [],
            "gemFilters": [
                {
                    "type": "UncutGems",
                    "category": "Uncut Gems",
                    "classes": ["Uncut Skill Gems", "Uncut Spirit Gems", "Uncut Support Gems"]
                }
            ]
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
    // Test: gem per-level entries — all three levels survive, level kept in base_type,
    // class derived from prefix (new behavior: no minLevel floor, no collapse/dedup)
    // -----------------------------------------------------------------------
    #[test]
    fn gem_per_level_entries_keep_level_and_derive_class() {
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
            &[("UncutGems".to_string(), gem_overview)],
            &mapping,
        )
        .expect("build_prices with gem filter should succeed");

        // All three levels survive — no minLevel floor.
        assert_eq!(
            items.len(),
            3,
            "all three gem levels must produce distinct PriceItems; got {} items: {:?}",
            items.len(),
            items.iter().map(|i| &i.base_type).collect::<Vec<_>>()
        );

        // Each level keeps its "(Level N)" suffix in both name and base_type.
        let base_types: Vec<&str> = items.iter().map(|i| i.base_type.as_str()).collect();
        assert!(
            base_types.contains(&"Uncut Spirit Gem (Level 16)"),
            "level 16 must survive: {:?}",
            base_types
        );
        assert!(
            base_types.contains(&"Uncut Spirit Gem (Level 18)"),
            "level 18 must survive: {:?}",
            base_types
        );
        assert!(
            base_types.contains(&"Uncut Spirit Gem (Level 20)"),
            "level 20 must survive: {:?}",
            base_types
        );

        // All items derive class "Uncut Spirit Gems" from the prefix.
        for item in &items {
            assert_eq!(
                item.class, "Uncut Spirit Gems",
                "class must be derived from prefix for item '{}'",
                item.base_type
            );
            // name == base_type (category item, not a unique)
            assert_eq!(
                item.name, item.base_type,
                "gem name must equal base_type (not a unique): {:?}",
                item
            );
        }

        // No collapse: three distinct base_types.
        let l16 = items.iter().find(|i| i.base_type == "Uncut Spirit Gem (Level 16)").unwrap();
        let l20 = items.iter().find(|i| i.base_type == "Uncut Spirit Gem (Level 20)").unwrap();
        assert!(
            (l16.div.unwrap_or(0.0) - 0.1).abs() < 0.001,
            "level 16 keeps its own price (div ~0.1); got {:?}",
            l16.div
        );
        assert!(
            (l20.div.unwrap_or(0.0) - 5.0).abs() < 0.001,
            "level 20 keeps its own price (div 5.0); got {:?}",
            l20.div
        );
    }

    // -----------------------------------------------------------------------
    // Test: derive_uncut_class helper
    // -----------------------------------------------------------------------
    #[test]
    fn derive_uncut_class_from_base_type() {
        assert_eq!(
            super::derive_uncut_class("Uncut Skill Gem (Level 20)"),
            Some("Uncut Skill Gems".to_string())
        );
        assert_eq!(
            super::derive_uncut_class("Uncut Spirit Gem (Level 18)"),
            Some("Uncut Spirit Gems".to_string())
        );
        assert_eq!(
            super::derive_uncut_class("Uncut Support Gem (Level 16)"),
            Some("Uncut Support Gems".to_string())
        );
        assert_eq!(
            super::derive_uncut_class("Lifeforce Support"),
            None,
            "non-uncut gem names must return None"
        );
        assert_eq!(
            super::derive_uncut_class("Divine Orb"),
            None,
            "currency names must return None"
        );
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
    // Test: a `lines` entry whose id is missing from core.items is skipped, NOT
    // emitted with the kebab-case slug as its baseType. Regression for the bug
    // where "vaal-cultivation-orb" (a slug) leaked into the filter as a BaseType.
    // -----------------------------------------------------------------------
    #[test]
    fn unresolved_line_id_is_skipped_not_emitted_as_slug() {
        let overview = make_overview(
            "divine",
            &[("chaos", 32.24), ("exalted", 149.0)],
            // core.items has "Some Orb" but NOT "vaal-cultivation-orb"
            &[("item-a", "Some Orb")],
            &[("item-a", 0.5), ("vaal-cultivation-orb", 0.5)],
        );
        let mapping = minimal_mapping();
        let items = super::build_prices(&[("Currency".to_string(), overview)], &mapping)
            .expect("build_prices should succeed");

        // The resolvable item is present...
        assert!(items.iter().any(|i| i.name == "Some Orb"));
        // ...and the unresolved slug never appears as a name OR a baseType.
        assert!(
            !items.iter().any(|i| i.name.contains('-') || i.base_type.contains('-')),
            "no kebab-case slug should leak into name/baseType: {:?}",
            items.iter().map(|i| &i.base_type).collect::<Vec<_>>()
        );
        assert!(
            !items.iter().any(|i| i.base_type == "vaal-cultivation-orb"),
            "the unresolved slug must be skipped, not emitted as a BaseType"
        );
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

    // -----------------------------------------------------------------------
    // Test: names resolve from the TOP-LEVEL `items` dictionary, not just core.items.
    // The real poe.ninja exchange response keeps the full id→name map in the top-level
    // `items` array while core.items carries only the rate-reference currencies. A line
    // whose id is only in the top-level dictionary must still get its real baseType.
    // -----------------------------------------------------------------------
    #[test]
    fn top_level_items_resolves_names() {
        use crate::sync::models::{ExchangeCore, ExchangeItem, ExchangeLine, ExchangeOverview};
        use std::collections::HashMap;

        let mut rates = HashMap::new();
        rates.insert("chaos".to_string(), 220.0);
        rates.insert("exalted".to_string(), 48.89);

        let overview = ExchangeOverview {
            core: ExchangeCore {
                // Only the rate anchor lives in core.items (mirrors the real API).
                items: vec![ExchangeItem {
                    id: "divine".to_string(),
                    name: "Divine Orb".to_string(),
                    category: None,
                }],
                rates,
                primary: "divine".to_string(),
            },
            lines: vec![
                ExchangeLine { id: "divine".to_string(), primary_value: 1.0 },
                ExchangeLine { id: "vaal-cultivation-orb".to_string(), primary_value: 0.5 },
            ],
            // The comprehensive dictionary — only here, NOT in core.items.
            items: vec![ExchangeItem {
                id: "vaal-cultivation-orb".to_string(),
                name: "Vaal Cultivation Orb".to_string(),
                category: None,
            }],
        };

        let mapping = minimal_mapping();
        let items = super::build_prices(&[("Currency".to_string(), overview)], &mapping)
            .expect("build_prices should succeed");

        // The slug resolved to its real base type from the top-level dictionary —
        // NOT dropped and NOT emitted as "vaal-cultivation-orb".
        let resolved = items.iter().find(|i| i.base_type == "Vaal Cultivation Orb");
        assert!(
            resolved.is_some(),
            "top-level items dictionary must resolve the slug to its real base type"
        );
        assert!(
            !items.iter().any(|i| i.base_type.contains('-')),
            "no kebab slug should remain once the top-level dictionary is consulted"
        );
    }

    // -----------------------------------------------------------------------
    // Test: build_unique_items produces name != base_type items (so the pricer
    // collects them into Rarity == Unique blocks), converts prices via core.rates,
    // and skips lines with no baseType.
    // -----------------------------------------------------------------------
    #[test]
    fn build_unique_items_emits_uniques_with_base_type() {
        use crate::sync::models::ItemOverview;
        let overview_json = r#"{
            "core": { "items": [], "rates": { "chaos": 220.0, "exalted": 48.89 }, "primary": "divine" },
            "lines": [
                { "name": "Temporalis", "baseType": "Silk Robe", "category": "Body Armour", "primaryValue": 100.0 },
                { "name": "Death Articulated", "baseType": "Ornate Gauntlets", "category": "Gloves", "primaryValue": 0.5 },
                { "name": "Nameless", "baseType": "", "category": "Gloves", "primaryValue": 1.0 }
            ]
        }"#;
        let overview: ItemOverview =
            serde_json::from_str(overview_json).expect("parse item overview");
        // minimal_mapping has empty itemTypes → category defaults to "Uniques".
        let mapping = minimal_mapping();
        let items =
            super::build_unique_items(&[("UniqueArmours".to_string(), overview)], &mapping);

        // The empty-baseType line is skipped; the two real ones survive.
        assert_eq!(items.len(), 2, "the empty-baseType unique must be skipped");
        assert!(!items.iter().any(|i| i.name == "Nameless"));

        let temp = items
            .iter()
            .find(|i| i.name == "Temporalis")
            .expect("Temporalis present");
        // The filter base type — NOT the unique name (this is the whole point).
        assert_eq!(temp.base_type, "Silk Robe");
        assert_ne!(
            temp.name, temp.base_type,
            "uniques MUST have name != base_type — the pricer keys on this to build Unique blocks"
        );
        assert_eq!(temp.category, "Uniques");
        // primary=divine → div = primaryValue, chaos = primaryValue * chaos_rate.
        assert_eq!(temp.div, Some(100.0));
        assert!((temp.chaos.unwrap() - 100.0 * 220.0).abs() < 1.0, "chaos converted via rate");
    }
}
