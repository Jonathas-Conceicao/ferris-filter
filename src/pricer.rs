use miette::Diagnostic;
use thiserror::Error;

use crate::models::prices::{PriceItem, PricesFile};
use crate::models::tiers::{Unit, ValidatedTiers};

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// Chaos-equivalent exchange rates derived from the first three `prices.json` anchors.
#[derive(Debug, Clone)]
pub struct CrossRates {
    /// Chaos per 1 Exalt.
    pub exalt_in_chaos: f64,
    /// Chaos per 1 Divine.
    pub div_in_chaos: f64,
}

/// A single priced item after normalization (ready for block emission).
#[derive(Debug, Clone)]
pub struct TieredItem {
    pub base_type: String,
    pub class: String,
}

/// A group of same-tier items within a category (one filter block).
#[derive(Debug, Clone)]
pub struct TierGroup {
    pub tier: String,
    /// The `Class ==` value for this block.
    ///
    /// For single-class blocks this is a bare class name (e.g. `"Stackable Currency"`).
    /// When `is_multi_class` is true, this is the pre-quoted multi-class string
    /// `UNCUT_GEM_MULTICLASS` — emitted verbatim by the emitter.
    pub class: String,
    /// True when `class` holds a pre-formatted multi-class string (e.g. the Uncut Gems
    /// merge block). The emitter uses this typed signal instead of the fragile
    /// `contains('"')` heuristic to decide whether to wrap the value in quotes.
    pub is_multi_class: bool,
    pub items: Vec<TieredItem>,
}

/// All tier groups for one price category (e.g. "Currency", "Runes").
#[derive(Debug, Clone)]
pub struct CategoryBlock {
    pub name: String,
    pub tiers: Vec<TierGroup>,
    /// Per-category fixed background `[R, G, B]` carried from mapping.json for Phase 9.
    /// Phase 8 does NOT emit this — the emitter leaves it unread this phase.
    /// None = no background override (Currency, Verisium, etc.).
    pub background_override: Option<[u8; 3]>,
}

/// Derive the per-category background override from the category name.
///
/// These values mirror the `backgroundOverride` fields in mapping.json gemFilters.
/// Phase 9 will consume this; Phase 8 only carries it.
fn category_background(name: &str) -> Option<[u8; 3]> {
    match name {
        UNCUT_GEMS_CATEGORY => Some([80, 70, 55]),
        "Lineage Support Gems" => Some([10, 150, 150]),
        _ => None,
    }
}

/// Category sentinel for the Uncut Gems merge case.
///
/// This string originates from `GemFilter.category` in `mapping.json`
/// (`"category": "Uncut Gems"`). Using a named const avoids brittle string literals
/// scattered across the pricer and `category_background` — if the category value
/// is ever renamed in `mapping.json`, only this const needs updating.
const UNCUT_GEMS_CATEGORY: &str = "Uncut Gems";

/// Canonical multi-class Class value for the Uncut Gems category.
///
/// The three uncut gem subtype classes are merged into a single `Class ==` line
/// by the emitter. The ordering (Skill, Spirit, Support) is fixed and deterministic.
/// Phase 8 builds this string at the pricer boundary; the emitter emits it verbatim
/// when `TierGroup.is_multi_class` is true (set by the pricer for this group).
const UNCUT_GEM_MULTICLASS: &str = r#""Uncut Skill Gems" "Uncut Spirit Gems" "Uncut Support Gems""#;

/// A price-tiered unique block (one or more base types at the same tier).
#[derive(Debug, Clone)]
pub struct UniqueBlock {
    pub tier: String,
    pub base_types: Vec<String>,
}

/// The complete output of `price_tiers` — consumed by the emitter.
#[derive(Debug, Clone)]
pub struct PricedData {
    pub category_blocks: Vec<CategoryBlock>,
    pub unique_blocks: Vec<UniqueBlock>,
}

impl PricedData {
    /// Returns an empty PricedData — used by GEN-05 (no prices.json present).
    pub fn empty() -> Self {
        PricedData {
            category_blocks: vec![],
            unique_blocks: vec![],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Error type (TIER-04)
// ─────────────────────────────────────────────────────────────────────────────

/// Error returned when the reference orbs (Exalt/Chaos/Divine) can't yield cross-rates.
#[derive(Debug, Error, Diagnostic)]
pub enum PriceRateError {
    #[error(
        "prices.json: reference currency '{0}' has a null or zero price — \
         cannot derive cross-rates (run `ferrisfilter sync` to refresh prices.json)"
    )]
    #[diagnostic(code(ferrisfilter::prices::bad_rate))]
    NullOrZero(&'static str),

    /// The three anchors are quoted in different units, so their ratio is meaningless.
    /// Rates are derived as `value(orb) / value(Chaos Orb)`, which only holds when all
    /// three are quoted in the same unit (all chaos for live data, all exalt for the
    /// committed fallback).
    #[error(
        "prices.json: the Exalt/Chaos/Divine anchors must all be quoted in the same unit \
         (all chaos, or all exalt) to derive cross-rates"
    )]
    #[diagnostic(code(ferrisfilter::prices::mixed_anchor_units))]
    MixedAnchorUnits,
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixed tier ordering (used for deterministic emission)
// ─────────────────────────────────────────────────────────────────────────────

const TIER_ORDER: &[&str] = &["S", "A", "B", "C", "D", "E", "F"];

// ─────────────────────────────────────────────────────────────────────────────
// Core math functions
// ─────────────────────────────────────────────────────────────────────────────

/// The unit an anchor orb is quoted in (chaos > div > exalt preference).
#[derive(Debug, PartialEq, Clone, Copy)]
enum AnchorUnit {
    Chaos,
    Div,
    Exalt,
}

/// Read an anchor orb's quoted value and the unit it is quoted in.
///
/// Returns TIER-04 `NullOrZero` (naming the orb) when the item is missing or has no
/// non-zero price in any of chaos/div/exalt.
fn anchor_quote(
    item: Option<&PriceItem>,
    name: &'static str,
) -> Result<(f64, AnchorUnit), PriceRateError> {
    let it = item.ok_or(PriceRateError::NullOrZero(name))?;
    let (val, unit) = if let Some(c) = it.chaos {
        (c, AnchorUnit::Chaos)
    } else if let Some(d) = it.div {
        (d, AnchorUnit::Div)
    } else if let Some(e) = it.exalt {
        (e, AnchorUnit::Exalt)
    } else {
        return Err(PriceRateError::NullOrZero(name));
    };
    if val == 0.0 {
        return Err(PriceRateError::NullOrZero(name));
    }
    Ok((val, unit))
}

/// Derive cross-rates from the first three items in the prices file.
///
/// Convention (SYNC-02/D-03): items[0]=Exalt, items[1]=Chaos, items[2]=Divine. The rates
/// are the orbs' own values expressed *relative to the Chaos Orb* — so they're derived from
/// the prices file rather than stored in it:
///
/// ```text
/// exalt_in_chaos = value(Exalt)  / value(Chaos)
/// div_in_chaos   = value(Divine) / value(Chaos)
/// ```
///
/// This is unit-agnostic: live data quotes the orbs in chaos (Chaos = 1c, so the divisor is
/// a no-op and the numbers match the historical behaviour), while `prices-fallback.json`
/// quotes them in exalt (Chaos = 2.5ex → exalt_in_chaos = 1/2.5 = 0.4, etc.). The three
/// anchors must share one unit, or the ratio mixes units — that's rejected. A null/zero
/// anchor triggers TIER-04 abort, naming the offending currency.
pub fn derive_rates(items: &[PriceItem]) -> Result<CrossRates, PriceRateError> {
    let (exalt_q, ue) = anchor_quote(items.get(0), "Exalt")?;
    let (chaos_q, uc) = anchor_quote(items.get(1), "Chaos")?;
    let (div_q, ud) = anchor_quote(items.get(2), "Divine")?;

    if ue != uc || uc != ud {
        return Err(PriceRateError::MixedAnchorUnits);
    }

    Ok(CrossRates {
        // chaos_q is guaranteed non-zero by anchor_quote.
        exalt_in_chaos: exalt_q / chaos_q,
        div_in_chaos: div_q / chaos_q,
    })
}

/// Normalize an item's price to chaos.
///
/// Preference order: chaos > div×div_rate > exalt×exalt_rate (most direct source first).
/// Returns `None` if no usable price field is populated.
pub fn normalize_to_chaos(item: &PriceItem, rates: &CrossRates) -> Option<f64> {
    if let Some(c) = item.chaos {
        return Some(c);
    }
    if let Some(d) = item.div {
        return Some(d * rates.div_in_chaos);
    }
    if let Some(e) = item.exalt {
        return Some(e * rates.exalt_in_chaos);
    }
    None
}

/// Convert `tiers.json` cutoffs to chaos values and sort descending (highest min first).
fn normalize_cutoffs(tiers: &ValidatedTiers, rates: &CrossRates) -> Vec<(String, f64)> {
    let mut normalized: Vec<(String, f64)> = tiers
        .0
        .iter()
        .map(|e| {
            let min_chaos = match e.unit {
                Unit::Chaos => e.min,
                Unit::Div => e.min * rates.div_in_chaos,
                Unit::Exalt => e.min * rates.exalt_in_chaos,
            };
            (e.tier.clone(), min_chaos)
        })
        .collect();
    // Sort descending so we check the highest tier first in assign_tier.
    normalized.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    normalized
}

/// Assign the highest tier whose chaos floor the item meets (inclusive ≥).
///
/// Priced items below every cutoff are floored to the LOWEST tier rather than dropped, so
/// anything present in prices.json gets a rule (e.g. Scroll of Wisdom at 0.0018c → F).
/// Returns `None` only when there are no cutoffs at all.
pub fn assign_tier(chaos_value: f64, cutoffs: &[(String, f64)]) -> Option<String> {
    for (tier, min_chaos) in cutoffs {
        if chaos_value >= *min_chaos {
            return Some(tier.clone());
        }
    }
    // Below all cutoffs → floor to the lowest tier. `cutoffs` is sorted descending by
    // min_chaos, so the last entry has the smallest floor (tier F).
    cutoffs.last().map(|(tier, _)| tier.clone())
}

// ─────────────────────────────────────────────────────────────────────────────
// Grouping — produces PricedData
// ─────────────────────────────────────────────────────────────────────────────

/// Build a `PricedData` from a loaded `PricesFile` and the pre-derived `CrossRates`.
///
/// - Currency items (name == base_type) are grouped by category + tier + class.
/// - Unique candidates (name != base_type) are collected, base-collision resolved to
///   the highest tier, and placed in `unique_blocks`.
/// - Category blocks preserve first-appearance order; tiers appear in S→F order.
pub fn price_tiers(
    prices: &PricesFile,
    tiers: &ValidatedTiers,
    rates: &CrossRates,
) -> Result<PricedData, PriceRateError> {
    let cutoffs = normalize_cutoffs(tiers, rates);

    // ── Accumulation structures ────────────────────────────────────────────
    // category_order: first-appearance sequence of category names
    let mut category_order: Vec<String> = Vec::new();
    // For each (category, tier, class) key -> items vec
    // We use a HashMap for O(1) accumulation but NEVER iterate it to emit —
    // emission uses category_order × TIER_ORDER × class_order instead.
    let mut category_tier_class_items: std::collections::HashMap<
        (String, String, String),
        Vec<TieredItem>,
    > = std::collections::HashMap::new();
    // class first-appearance per (category, tier)
    let mut class_order: std::collections::HashMap<(String, String), Vec<String>> =
        std::collections::HashMap::new();

    // Unique candidates: base_type -> (best_tier_index, tier_str)
    let mut unique_best: std::collections::HashMap<String, (usize, String)> =
        std::collections::HashMap::new();
    // Unique base_type first-appearance order (for deterministic output)
    let mut unique_base_order: Vec<String> = Vec::new();

    for item in &prices.items {
        let chaos_val = match normalize_to_chaos(item, rates) {
            Some(v) => v,
            None => continue,
        };
        let tier = match assign_tier(chaos_val, &cutoffs) {
            Some(t) => t,
            None => continue,
        };

        let is_unique_candidate = item.name != item.base_type;

        if is_unique_candidate {
            // Unique collision: keep highest tier (lowest index in TIER_ORDER).
            let new_idx = TIER_ORDER
                .iter()
                .position(|&t| t == tier.as_str())
                .unwrap_or(TIER_ORDER.len());
            let entry = unique_best
                .entry(item.base_type.clone())
                .or_insert_with(|| {
                    unique_base_order.push(item.base_type.clone());
                    (new_idx, tier.clone())
                });
            if new_idx < entry.0 {
                *entry = (new_idx, tier.clone());
            }
        } else {
            // Currency / category item
            let cat = &item.category;

            // Track category first-appearance
            if !category_order.contains(cat) {
                category_order.push(cat.clone());
            }

            let key = (cat.clone(), tier.clone(), item.class.clone());

            // Track class first-appearance within (category, tier)
            let class_key = (cat.clone(), tier.clone());
            let classes = class_order.entry(class_key).or_insert_with(Vec::new);
            if !classes.contains(&item.class) {
                classes.push(item.class.clone());
            }

            category_tier_class_items
                .entry(key)
                .or_insert_with(Vec::new)
                .push(TieredItem {
                    base_type: item.base_type.clone(),
                    class: item.class.clone(),
                });
        }
    }

    // ── Build Vec<CategoryBlock> using deterministic iteration ─────────────
    // Iterate category_order (first-appearance) × TIER_ORDER (fixed S→F) × class_order.
    //
    // Special case — "Uncut Gems": the three subtype classes ("Uncut Skill Gems",
    // "Uncut Spirit Gems", "Uncut Support Gems") are merged into ONE TierGroup per
    // tier. The `class` field on that group is the pre-quoted multi-class string
    // (UNCUT_GEM_MULTICLASS) which the emitter emits verbatim via its `contains('"')`
    // check. Items from all three subtypes are unioned into that single group.
    let mut category_blocks: Vec<CategoryBlock> = Vec::new();
    for cat in &category_order {
        let is_uncut_gems = cat == UNCUT_GEMS_CATEGORY;
        let mut tier_groups: Vec<TierGroup> = Vec::new();

        for &tier in TIER_ORDER {
            if is_uncut_gems {
                // Collect all items in this tier across all three subtype classes.
                let all_items: Vec<TieredItem> = [
                    "Uncut Skill Gems",
                    "Uncut Spirit Gems",
                    "Uncut Support Gems",
                ]
                .iter()
                .flat_map(|cls| {
                    let key = (cat.clone(), tier.to_string(), cls.to_string());
                    category_tier_class_items
                        .get(&key)
                        .cloned()
                        .unwrap_or_default()
                })
                .collect();

                if !all_items.is_empty() {
                    tier_groups.push(TierGroup {
                        tier: tier.to_string(),
                        // Pre-quoted multi-class value — emitter uses is_multi_class to
                        // emit this verbatim rather than wrapping it in extra quotes.
                        class: UNCUT_GEM_MULTICLASS.to_string(),
                        is_multi_class: true,
                        items: all_items,
                    });
                }
            } else {
                // Regular categories: one TierGroup per (category, tier, class).
                let classes = class_order
                    .get(&(cat.clone(), tier.to_string()))
                    .cloned()
                    .unwrap_or_default();
                for cls in &classes {
                    let key = (cat.clone(), tier.to_string(), cls.clone());
                    if let Some(items) = category_tier_class_items.get(&key) {
                        if !items.is_empty() {
                            tier_groups.push(TierGroup {
                                tier: tier.to_string(),
                                class: cls.clone(),
                                is_multi_class: false,
                                items: items.clone(),
                            });
                        }
                    }
                }
            }
        }

        if !tier_groups.is_empty() {
            category_blocks.push(CategoryBlock {
                name: cat.clone(),
                tiers: tier_groups,
                background_override: category_background(cat),
            });
        }
    }

    // ── Build Vec<UniqueBlock> — group base_types by tier, in TIER_ORDER ──
    // Collect base_types per tier, preserving unique_base_order
    let mut unique_per_tier: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for base_type in &unique_base_order {
        if let Some((_, tier)) = unique_best.get(base_type) {
            unique_per_tier
                .entry(tier.clone())
                .or_insert_with(Vec::new)
                .push(base_type.clone());
        }
    }
    // Emit in TIER_ORDER, only populated tiers
    let mut unique_blocks: Vec<UniqueBlock> = Vec::new();
    for &tier in TIER_ORDER {
        if let Some(base_types) = unique_per_tier.remove(tier) {
            unique_blocks.push(UniqueBlock {
                tier: tier.to_string(),
                base_types,
            });
        }
    }

    Ok(PricedData {
        category_blocks,
        unique_blocks,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::prices::PricesFile;
    use crate::models::tiers::{TierEntry, ValidatedTiers};

    // ── Test helpers ────────────────────────────────────────────────────────

    /// Load the real tiers.json from disk and validate it.
    fn load_real_tiers() -> ValidatedTiers {
        let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tiers.json"));
        let entries: Vec<TierEntry> = serde_json::from_str(src).expect("parse tiers.json");
        ValidatedTiers::try_from(entries).expect("validate tiers.json")
    }

    /// Build CrossRates with the seeded div_rate=220.
    fn seeded_rates() -> CrossRates {
        CrossRates {
            exalt_in_chaos: 4.5,
            div_in_chaos: 220.0,
        }
    }

    /// Build normalized cutoffs from real tiers.json + seeded rates.
    fn seeded_cutoffs() -> (ValidatedTiers, Vec<(String, f64)>) {
        let tiers = load_real_tiers();
        let rates = seeded_rates();
        let cutoffs = normalize_cutoffs(&tiers, &rates);
        (tiers, cutoffs)
    }

    // ── TIER-04: derive_rates abort on null / zero ───────────────────────────

    fn make_anchor(name: &str, chaos: Option<f64>) -> PriceItem {
        PriceItem {
            name: name.to_string(),
            base_type: name.to_string(),
            class: "Stackable Currency".to_string(),
            category: "Currency".to_string(),
            exalt: None,
            chaos,
            div: None,
        }
    }

    #[test]
    fn tier04_exalt_null_aborts_naming_exalt() {
        let items = vec![
            make_anchor("Exalted Orb", None),
            make_anchor("Chaos Orb", Some(1.0)),
            make_anchor("Divine Orb", Some(220.0)),
        ];
        let err = derive_rates(&items).unwrap_err();
        assert!(
            matches!(err, PriceRateError::NullOrZero("Exalt")),
            "expected NullOrZero(Exalt), got {err:?}"
        );
    }

    #[test]
    fn tier04_chaos_null_aborts_naming_chaos() {
        let items = vec![
            make_anchor("Exalted Orb", Some(4.5)),
            make_anchor("Chaos Orb", None),
            make_anchor("Divine Orb", Some(220.0)),
        ];
        let err = derive_rates(&items).unwrap_err();
        assert!(
            matches!(err, PriceRateError::NullOrZero("Chaos")),
            "expected NullOrZero(Chaos), got {err:?}"
        );
    }

    #[test]
    fn tier04_divine_null_aborts_naming_divine() {
        let items = vec![
            make_anchor("Exalted Orb", Some(4.5)),
            make_anchor("Chaos Orb", Some(1.0)),
            make_anchor("Divine Orb", None),
        ];
        let err = derive_rates(&items).unwrap_err();
        assert!(
            matches!(err, PriceRateError::NullOrZero("Divine")),
            "expected NullOrZero(Divine), got {err:?}"
        );
    }

    #[test]
    fn tier04_exalt_zero_aborts_naming_exalt() {
        let items = vec![
            make_anchor("Exalted Orb", Some(0.0)),
            make_anchor("Chaos Orb", Some(1.0)),
            make_anchor("Divine Orb", Some(220.0)),
        ];
        let err = derive_rates(&items).unwrap_err();
        assert!(
            matches!(err, PriceRateError::NullOrZero("Exalt")),
            "expected NullOrZero(Exalt), got {err:?}"
        );
    }

    #[test]
    fn tier04_chaos_zero_aborts_naming_chaos() {
        let items = vec![
            make_anchor("Exalted Orb", Some(4.5)),
            make_anchor("Chaos Orb", Some(0.0)),
            make_anchor("Divine Orb", Some(220.0)),
        ];
        let err = derive_rates(&items).unwrap_err();
        assert!(
            matches!(err, PriceRateError::NullOrZero("Chaos")),
            "expected NullOrZero(Chaos), got {err:?}"
        );
    }

    #[test]
    fn tier04_divine_zero_aborts_naming_divine() {
        let items = vec![
            make_anchor("Exalted Orb", Some(4.5)),
            make_anchor("Chaos Orb", Some(1.0)),
            make_anchor("Divine Orb", Some(0.0)),
        ];
        let err = derive_rates(&items).unwrap_err();
        assert!(
            matches!(err, PriceRateError::NullOrZero("Divine")),
            "expected NullOrZero(Divine), got {err:?}"
        );
    }

    // ── TIER-01: derive_rates happy path ─────────────────────────────────────

    #[test]
    fn derive_rates_returns_correct_cross_rates() {
        let items = vec![
            make_anchor("Exalted Orb", Some(4.5)),
            make_anchor("Chaos Orb", Some(1.0)),
            make_anchor("Divine Orb", Some(220.0)),
        ];
        let rates = derive_rates(&items).unwrap();
        assert_eq!(rates.exalt_in_chaos, 4.5);
        assert_eq!(rates.div_in_chaos, 220.0);
    }

    fn exalt_anchor(name: &str, exalt: f64) -> PriceItem {
        PriceItem {
            name: name.to_string(),
            base_type: name.to_string(),
            class: "Stackable Currency".to_string(),
            category: "Currency".to_string(),
            exalt: Some(exalt),
            chaos: None,
            div: None,
        }
    }

    #[test]
    fn derive_rates_from_exalt_quoted_anchors() {
        // Fallback-style: orbs quoted in exalt → rates come out relative to the Chaos Orb.
        let items = vec![
            exalt_anchor("Exalted Orb", 1.0),
            exalt_anchor("Chaos Orb", 2.5),
            exalt_anchor("Divine Orb", 20.0),
        ];
        let rates = derive_rates(&items).unwrap();
        assert_eq!(rates.exalt_in_chaos, 1.0 / 2.5); // 0.4 chaos per exalt
        assert_eq!(rates.div_in_chaos, 20.0 / 2.5); // 8 chaos per div
    }

    #[test]
    fn derive_rates_mixed_anchor_units_errors() {
        // Exalt quoted in exalt but Chaos quoted in chaos → ratio would mix units → reject.
        let items = vec![
            exalt_anchor("Exalted Orb", 1.0),
            make_anchor("Chaos Orb", Some(1.0)), // chaos-quoted
            exalt_anchor("Divine Orb", 20.0),
        ];
        let err = derive_rates(&items).unwrap_err();
        assert!(
            matches!(err, PriceRateError::MixedAnchorUnits),
            "expected MixedAnchorUnits, got {err:?}"
        );
    }

    // ── normalize_to_chaos preference order ─────────────────────────────────

    #[test]
    fn normalize_prefers_chaos_over_div_and_exalt() {
        let rates = seeded_rates();
        let item = PriceItem {
            name: "Test".to_string(),
            base_type: "Test".to_string(),
            class: "Stackable Currency".to_string(),
            category: "Currency".to_string(),
            exalt: Some(10.0),
            chaos: Some(5.0),
            div: Some(0.1),
        };
        assert_eq!(normalize_to_chaos(&item, &rates), Some(5.0));
    }

    #[test]
    fn normalize_falls_back_to_div_when_no_chaos() {
        let rates = seeded_rates();
        let item = PriceItem {
            name: "Test".to_string(),
            base_type: "Test".to_string(),
            class: "Stackable Currency".to_string(),
            category: "Currency".to_string(),
            exalt: Some(100.0),
            chaos: None,
            div: Some(1.0),
        };
        // 1.0 * 220.0 = 220.0
        assert_eq!(normalize_to_chaos(&item, &rates), Some(220.0));
    }

    #[test]
    fn normalize_falls_back_to_exalt_when_no_chaos_or_div() {
        let rates = seeded_rates();
        let item = PriceItem {
            name: "Test".to_string(),
            base_type: "Test".to_string(),
            class: "Stackable Currency".to_string(),
            category: "Currency".to_string(),
            exalt: Some(2.0),
            chaos: None,
            div: None,
        };
        // 2.0 * 4.5 = 9.0
        assert_eq!(normalize_to_chaos(&item, &rates), Some(9.0));
    }

    #[test]
    fn normalize_returns_none_when_all_null() {
        let rates = seeded_rates();
        let item = PriceItem {
            name: "Test".to_string(),
            base_type: "Test".to_string(),
            class: "Stackable Currency".to_string(),
            category: "Currency".to_string(),
            exalt: None,
            chaos: None,
            div: None,
        };
        assert_eq!(normalize_to_chaos(&item, &rates), None);
    }

    // ── TIER-02: assign_tier with seeded cutoffs ─────────────────────────────

    #[test]
    fn assign_tier_anchor_rertiering() {
        let (_, cutoffs) = seeded_cutoffs();
        // With tiers.json: S=30c, A=20c, B=12c, C=8c, D=2c, E=0.5c, F=0.1c.
        // Divine (220c): 220 >= 30 → S
        assert_eq!(assign_tier(220.0, &cutoffs).as_deref(), Some("S"));
        // Chaos (1c): 1 >= 0.5 (E floor) → E
        assert_eq!(assign_tier(1.0, &cutoffs).as_deref(), Some("E"));
        // Exalt (4.5c): 4.5 >= 2 (D floor) → D (4.5 < 8 = C floor)
        assert_eq!(assign_tier(4.5, &cutoffs).as_deref(), Some("D"));
    }

    #[test]
    fn assign_tier_s_at_exactly_30c() {
        let (_, cutoffs) = seeded_cutoffs();
        // S floor = 30c (tiers.json) — exactly at boundary -> S
        assert_eq!(assign_tier(30.0, &cutoffs).as_deref(), Some("S"));
        // Also test well above: 1100c is solidly in S
        assert_eq!(assign_tier(1100.0, &cutoffs).as_deref(), Some("S"));
    }

    #[test]
    fn assign_tier_boundary_inclusive_e_at_0_5c() {
        let (_, cutoffs) = seeded_cutoffs();
        // E floor = 0.5c, item at exactly 0.5c -> E
        assert_eq!(assign_tier(0.5, &cutoffs).as_deref(), Some("E"));
    }

    #[test]
    fn assign_tier_sub_floor_floors_to_lowest_tier() {
        let (_, cutoffs) = seeded_cutoffs();
        // F floor = 0.1c; 0.05c is below all cutoffs and now floors to the lowest tier (F),
        // rather than being dropped — so a priced item like Scroll of Wisdom still gets a rule.
        assert_eq!(assign_tier(0.05, &cutoffs).as_deref(), Some("F"));
    }

    #[test]
    fn assign_tier_f_at_exactly_0_1c() {
        let (_, cutoffs) = seeded_cutoffs();
        // F floor = 0.1c, item at exactly 0.1c -> F
        assert_eq!(assign_tier(0.1, &cutoffs).as_deref(), Some("F"));
    }

    // ── TIER-03: unique base-collision ──────────────────────────────────────

    #[test]
    fn unique_base_collision_uses_highest_tier() {
        // Inline fixture: two uniques sharing "Heavy Belt", one high-tier, one D-tier.
        // With tiers.json: S=30c, A=20c, B=12c, C=8c, D=2c.
        // Headhunter: 50c >= 30c = tier S; Darkness Enthroned: 5c >= 2c = tier D.
        // Collision on "Heavy Belt": highest (S) wins over D.
        let prices_json = r#"{
            "league": "Standard",
            "fetchedAt": "2026-01-01T00:00:00Z",
            "items": [
                {"name":"Exalted Orb","baseType":"Exalted Orb","class":"Stackable Currency","category":"Currency","exalt":null,"chaos":4.5,"div":null},
                {"name":"Chaos Orb","baseType":"Chaos Orb","class":"Stackable Currency","category":"Currency","exalt":null,"chaos":1.0,"div":null},
                {"name":"Divine Orb","baseType":"Divine Orb","class":"Stackable Currency","category":"Currency","exalt":null,"chaos":220.0,"div":null},
                {"name":"Headhunter","baseType":"Heavy Belt","class":"Belts","category":"Unique Belts","exalt":null,"chaos":50.0,"div":null},
                {"name":"Darkness Enthroned","baseType":"Heavy Belt","class":"Belts","category":"Unique Belts","exalt":null,"chaos":5.0,"div":null}
            ]
        }"#;
        let prices: PricesFile = serde_json::from_str(prices_json).unwrap();
        let tiers = load_real_tiers();
        let rates = derive_rates(&prices.items).unwrap();
        let priced = price_tiers(&prices, &tiers, &rates).unwrap();

        // Exactly one unique block at tier S (Headhunter wins collision) with BaseType == "Heavy Belt"
        assert_eq!(priced.unique_blocks.len(), 1);
        assert_eq!(
            priced.unique_blocks[0].tier, "S",
            "Headhunter (50c) must land S (>= 30c floor)"
        );
        assert_eq!(
            priced.unique_blocks[0].base_types,
            vec!["Heavy Belt".to_string()]
        );
    }

    // ── Committed prices.json — zero unique blocks (D-08) ──────────────────

    #[test]
    fn seeded_prices_yields_zero_unique_blocks() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/seed-prices.json"
        ));
        let prices: PricesFile = serde_json::from_str(src).expect("parse prices.json");
        let tiers = load_real_tiers();
        let rates = derive_rates(&prices.items).unwrap();
        let priced = price_tiers(&prices, &tiers, &rates).unwrap();
        assert!(
            priced.unique_blocks.is_empty(),
            "expected zero unique blocks for committed seed, got {:?}",
            priced.unique_blocks
        );
    }

    // ── Committed prices.json — anchor re-tier verification (D-09) ─────────

    #[test]
    fn seeded_currency_category_anchor_retiers() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/seed-prices.json"
        ));
        let prices: PricesFile = serde_json::from_str(src).expect("parse prices.json");
        let tiers = load_real_tiers();
        let rates = derive_rates(&prices.items).unwrap();
        let priced = price_tiers(&prices, &tiers, &rates).unwrap();

        // Find the Currency category block
        let currency_block = priced
            .category_blocks
            .iter()
            .find(|b| b.name == "Currency")
            .expect("Currency category block not found");

        // Helper: collect all base_types in a given tier for this category
        let items_in_tier = |tier: &str| -> Vec<String> {
            currency_block
                .tiers
                .iter()
                .filter(|g| g.tier == tier)
                .flat_map(|g| g.items.iter().map(|i| i.base_type.clone()))
                .collect::<Vec<_>>()
        };

        // With tiers.json: S=30c, A=20c, B=12c, C=8c, D=2c, E=0.5c, F=0.1c.
        // seed-prices.json anchors: Exalt=4.5c, Chaos=1.0c, Divine=220c.
        // Divine (220c): >= 30c → S.  Chaos (1c): >= 0.5c → E.  Exalt (4.5c): >= 2c → D.
        let s_items = items_in_tier("S");
        let e_items = items_in_tier("E");
        let d_items = items_in_tier("D");

        assert!(
            s_items.contains(&"Divine Orb".to_string()),
            "Divine Orb should be in S tier (220c >= 30c floor), got S items: {s_items:?}"
        );
        assert!(
            e_items.contains(&"Chaos Orb".to_string()),
            "Chaos Orb should be in E tier (1c >= 0.5c floor), got E items: {e_items:?}"
        );
        assert!(
            d_items.contains(&"Exalted Orb".to_string()),
            "Exalted Orb should be in D tier (4.5c >= 2c floor), got D items: {d_items:?}"
        );

        // Verify they are NOT in their old positions
        assert!(
            !items_in_tier("A").contains(&"Divine Orb".to_string()),
            "Divine Orb must NOT be in A tier (moved to S with new cutoffs)"
        );
        assert!(
            !items_in_tier("C").contains(&"Chaos Orb".to_string()),
            "Chaos Orb must NOT be in C tier"
        );
        assert!(
            !items_in_tier("C").contains(&"Exalted Orb".to_string()),
            "Exalted Orb must NOT be in C tier"
        );
    }

    // ── Early-league fallback: exalt-denominated tiers pin orbs to S/A/B ──────
    //
    // With tiers-fallback.json in EXALT units and the orbs carrying chaos anchors
    // (Exalt 4.5 / Chaos 11.25 / Divine 220), the divine→A coupling is gone:
    // Divine lands S, Chaos A, Exalt B — matching the poe2filter.com export.
    #[test]
    fn fallback_pair_pins_orbs_to_s_a_b() {
        let prices: PricesFile = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prices-fallback.json"
        )))
        .expect("parse prices-fallback.json");
        let entries: Vec<TierEntry> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tiers-fallback.json"
        )))
        .expect("parse tiers-fallback.json");
        let tiers = ValidatedTiers::try_from(entries).expect("validate tiers-fallback.json");
        let rates = derive_rates(&prices.items).unwrap();
        let priced = price_tiers(&prices, &tiers, &rates).unwrap();

        let currency = priced
            .category_blocks
            .iter()
            .find(|b| b.name == "Currency")
            .expect("Currency block");
        let tier_of = |base: &str| -> Option<String> {
            currency
                .tiers
                .iter()
                .find(|g| g.items.iter().any(|i| i.base_type == base))
                .map(|g| g.tier.clone())
        };

        assert_eq!(tier_of("Divine Orb").as_deref(), Some("S"), "Divine → S");
        assert_eq!(tier_of("Chaos Orb").as_deref(), Some("A"), "Chaos → A");
        assert_eq!(tier_of("Exalted Orb").as_deref(), Some("B"), "Exalt → B");
        // A representative S-tier base (priced in exalt at the S floor) also lands S.
        assert_eq!(
            tier_of("Mirror of Kalandra").as_deref(),
            Some("S"),
            "Mirror → S"
        );
    }
}
