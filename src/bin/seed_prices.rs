//! One-time migration tool: reads `currency.json` and writes `prices.json`.
//!
//! Run once with `cargo run --bin seed_prices` from the repo root. The
//! resulting `prices.json` is committed as the seeded value source for Phase 3.
//! After this run, `currency.json` is reference-only — not consumed by `generate`.
//!
//! ## Models
//! The currency and prices models are imported from the library
//! (`ferrisfilter::models::currency` / `::prices`) — this binary defines none of its own.

use std::collections::HashSet;

use ferrisfilter::models::currency::CurrencyData;
use ferrisfilter::models::prices::{PriceItem, PricesFile};

// ---------------------------------------------------------------------------
// Seed logic
// ---------------------------------------------------------------------------

/// D-09: Non-reference bases get their tier's lower-bound chaos value.
/// S=1100.0, A=220.0, B=30.0, C=10.0, D=2.0, E=0.5, F=0.1
fn tier_lower_bound(tier: &str) -> f64 {
    match tier {
        "S" => 1100.0,
        "A" => 220.0,
        "B" => 30.0,
        "C" => 10.0,
        "D" => 2.0,
        "E" => 0.5,
        "F" => 0.1,
        other => panic!("unknown tier '{}' in currency.json", other),
    }
}

/// D-09: Reference orb realistic chaos values that deliberately re-tier them.
/// Chaos Orb  -> 1.0c  (was C, becomes E)
/// Exalted Orb -> 4.5c (was C, becomes D)
/// Divine Orb  -> 220.0c (was S, becomes A — exactly 1 div)
fn reference_chaos(base: &str) -> Option<f64> {
    match base {
        "Chaos Orb" => Some(1.0),
        "Exalted Orb" => Some(4.5),
        "Divine Orb" => Some(220.0),
        _ => None,
    }
}

fn make_item(base: &str, class: &str, category: &str, tier: &str) -> PriceItem {
    // Override reference orbs with realistic values; all others get tier lower-bound.
    let chaos = reference_chaos(base).unwrap_or_else(|| tier_lower_bound(tier));
    PriceItem {
        name: base.to_string(),
        base_type: base.to_string(),
        class: class.to_string(),
        category: category.to_string(),
        exalt: None,
        chaos: Some(chaos),
        div: None,
    }
}

fn main() {
    // -----------------------------------------------------------------------
    // Read currency.json from the repo root (cwd when invoked by cargo run)
    // -----------------------------------------------------------------------
    let currency_src = std::fs::read_to_string("currency.json")
        .expect("cannot read currency.json — run from repo root");
    let currency: CurrencyData =
        serde_json::from_str(&currency_src).expect("currency.json parse failed");

    // -----------------------------------------------------------------------
    // The three reference orbs are de-duped: they appear as prepended anchors
    // (items[0..2]: Exalted, Chaos, Divine) and are SKIPPED when encountered
    // while walking the Currency category in file order (D-03 / SYNC-02).
    //
    // This ensures:
    //   1. items[0] = Exalted Orb (chaos 4.5)  — cross-rate anchor
    //   2. items[1] = Chaos Orb   (chaos 1.0)  — reference unit anchor
    //   3. items[2] = Divine Orb  (chaos 220.0) — div-rate anchor
    //   4. Each base appears exactly once in the output.
    // -----------------------------------------------------------------------
    const REF_CLASS: &str = "Stackable Currency";
    const REF_CAT: &str = "Currency";

    let anchors: Vec<PriceItem> = vec![
        make_item("Exalted Orb", REF_CLASS, REF_CAT, "C"), // chaos = 4.5 (override)
        make_item("Chaos Orb", REF_CLASS, REF_CAT, "C"),   // chaos = 1.0 (override)
        make_item("Divine Orb", REF_CLASS, REF_CAT, "S"),  // chaos = 220.0 (override)
    ];

    // Track de-duped bases (the three reference orbs).
    let reference_orbs: HashSet<&str> = ["Exalted Orb", "Chaos Orb", "Divine Orb"]
        .iter()
        .copied()
        .collect();

    // -----------------------------------------------------------------------
    // Walk currency.json in file order (category order, S→F within each, bases
    // in file order). This preserves the block emission order that Phase 2
    // established (D-04: first-appearance category order).
    // -----------------------------------------------------------------------
    let mut body: Vec<PriceItem> = Vec::new();

    for cat in &currency.categories {
        for entry in &cat.entries {
            for base in &entry.bases {
                // Skip the three reference orbs — they are already in anchors.
                if reference_orbs.contains(base.as_str()) {
                    continue;
                }
                body.push(make_item(base, &entry.class, &cat.name, &entry.tier));
            }
        }
    }

    // Combine: anchors first (items[0..2]), then all other bases in file order.
    let total = anchors.len() + body.len();
    let mut items: Vec<PriceItem> = anchors;
    items.extend(body);

    assert_eq!(
        items.len(),
        total,
        "item count mismatch after assembly"
    );

    let item_count = items.len();
    let prices = PricesFile {
        league: "Standard".to_string(),
        fetched_at: "2026-05-23T00:00:00Z".to_string(),
        items,
    };

    let json = serde_json::to_string_pretty(&prices).expect("serialization failed");

    std::fs::write("prices.json", &json).expect("cannot write prices.json");

    println!(
        "wrote {} items ({} bytes) to prices.json",
        item_count,
        json.len()
    );
}
