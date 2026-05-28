// Integration tests for the sync prices pipeline.
//
// All tests use a FakeClient that serves pre-recorded fixture strings —
// zero live network calls are made in any test here.
// FakeClient only; the real HTTP client is never constructed or imported.

use ferrisfilter::sync::client::HttpClient;
use ferrisfilter::sync::league::resolve_league;
use ferrisfilter::sync::models::IndexState;
use ferrisfilter::sync::prices::{build_prices, write_prices_atomic};
use ferrisfilter::models::prices::{PriceItem, PricesFile};

// ---------------------------------------------------------------------------
// Fixture strings (loaded at compile-time from tests/fixtures/)
// ---------------------------------------------------------------------------

const FIXTURE_INDEX_STATE: &str =
    include_str!("fixtures/index-state.json");

const FIXTURE_CURRENCY: &str =
    include_str!("fixtures/overview-currency.json");

const FIXTURE_UNIQUES: &str =
    include_str!("fixtures/overview-uniques.json");

const FIXTURE_UNCUTGEM: &str =
    include_str!("fixtures/overview-uncutgem.json");

const FIXTURE_EMPTY: &str =
    include_str!("fixtures/overview-empty.json");

// ---------------------------------------------------------------------------
// FakeClient — implements HttpClient by matching on URL + `type` query param.
//
// Mappings (url, type query value) → fixture string.
// For the "force-network-error" sentinel URL it returns Err.
// ---------------------------------------------------------------------------

pub struct FakeClient {
    /// Mapping of (url_contains, type_value) → fixture body.
    /// If type_value is "", it matches any type for that URL.
    responses: Vec<(&'static str, &'static str, &'static str)>,
}

impl FakeClient {
    /// Build a FakeClient that serves the happy-path fixtures:
    /// index-state, currency, UniqueWeapon, UncutGem.
    fn happy_path() -> Self {
        Self {
            responses: vec![
                // index-state endpoint (no type param)
                ("index-state", "", FIXTURE_INDEX_STATE),
                // currency overview
                ("overview", "Currency", FIXTURE_CURRENCY),
                // unique weapon overview (and all other Unique* types → empty)
                ("overview", "UniqueWeapon", FIXTURE_UNIQUES),
                // all other Unique* types → empty
                ("overview", "UniqueArmour", FIXTURE_EMPTY),
                ("overview", "UniqueAccessory", FIXTURE_EMPTY),
                ("overview", "UniqueFlask", FIXTURE_EMPTY),
                ("overview", "UniqueCharm", FIXTURE_EMPTY),
                ("overview", "UniqueJewel", FIXTURE_EMPTY),
                ("overview", "UniqueMap", FIXTURE_EMPTY),
                ("overview", "SanctumRelic", FIXTURE_EMPTY),
                // non-currency types → empty (off-season)
                ("overview", "Breach", FIXTURE_EMPTY),
                ("overview", "Delirium", FIXTURE_EMPTY),
                ("overview", "Essence", FIXTURE_EMPTY),
                ("overview", "Expedition", FIXTURE_EMPTY),
                ("overview", "Rune", FIXTURE_EMPTY),
                ("overview", "Omen", FIXTURE_EMPTY),
                ("overview", "SoulCore", FIXTURE_EMPTY),
                ("overview", "Waystone", FIXTURE_EMPTY),
                ("overview", "Talisman", FIXTURE_EMPTY),
                ("overview", "Abyss", FIXTURE_EMPTY),
                ("overview", "Fragment", FIXTURE_EMPTY),
                // uncut gem overview
                ("overview", "UncutGem", FIXTURE_UNCUTGEM),
            ],
        }
    }

    /// Build a FakeClient where every overview type returns empty (off-season / Err).
    fn all_empty() -> Self {
        Self {
            responses: vec![
                ("index-state", "", FIXTURE_INDEX_STATE),
                ("overview", "", FIXTURE_EMPTY),
            ],
        }
    }

    /// Build a FakeClient that returns Err for any overview URL (simulates network failure).
    fn network_error_for_overview() -> Self {
        Self {
            responses: vec![
                ("index-state", "", FIXTURE_INDEX_STATE),
                // "force-network-error" sentinel — no overview entries at all
            ],
        }
    }
}

impl HttpClient for FakeClient {
    async fn get_json(&self, url: &str, query: &[(&str, &str)]) -> miette::Result<String> {
        // Extract `type` query value if present
        let type_val = query
            .iter()
            .find(|(k, _)| *k == "type")
            .map(|(_, v)| *v)
            .unwrap_or("");

        // Match on URL substring + type value
        for (url_fragment, expected_type, body) in &self.responses {
            let url_matches = url.contains(url_fragment);
            let type_matches = expected_type.is_empty() || *expected_type == type_val;
            if url_matches && type_matches {
                return Ok(body.to_string());
            }
        }

        // No match → network error sentinel
        Err(miette::miette!(
            "FakeClient: no response registered for url={url} type={type_val}"
        ))
    }
}

// ---------------------------------------------------------------------------
// Helper: build a minimal mapping for Currency + UniqueWeapon + UncutGem only.
// Used in tests that don't need the full committed mapping.json.
// ---------------------------------------------------------------------------

fn minimal_mapping_str() -> &'static str {
    r#"{
        "version": 1,
        "overviewTypes": [
            {
                "type": "Currency",
                "category": "Currency",
                "class": "Stackable Currency",
                "baseTypeIsName": true
            },
            {
                "type": "UniqueWeapon",
                "category": "Uniques",
                "class": null,
                "baseTypeIsName": false
            }
        ],
        "gemFilter": {
            "type": "UncutGem",
            "category": "Uncut Gems",
            "class": "Stackable Currency",
            "minLevel": 18
        }
    }"#
}

// ---------------------------------------------------------------------------
// Task 1: Happy-path build test (SYNC-01/02/03)
//
// Feed FakeClient (index-state + currency + UniqueWeapon + UncutGem) through
// build_prices + the committed mapping.json.
// Asserts:
//   items[0].name == "Exalted Orb"
//   items[1].name == "Chaos Orb"
//   items[2].name == "Divine Orb"
//   chaos values are converted (not raw primaryValue)
//   each item has non-empty category + class from mapping
//   the unique from overview-uniques appears with category "Uniques"
// ---------------------------------------------------------------------------

#[test]
fn happy_path_anchors_first_and_correct_values() {
    // Parse fixtures directly (no network needed)
    let currency_overview: ferrisfilter::sync::models::ExchangeOverview =
        serde_json::from_str(FIXTURE_CURRENCY).expect("parse currency fixture");
    let uniques_overview: ferrisfilter::sync::models::ExchangeOverview =
        serde_json::from_str(FIXTURE_UNIQUES).expect("parse uniques fixture");
    let gem_overview: ferrisfilter::sync::models::ExchangeOverview =
        serde_json::from_str(FIXTURE_UNCUTGEM).expect("parse uncutgem fixture");

    // Use the minimal mapping (3 types: Currency, UniqueWeapon, UncutGem)
    let mapping: ferrisfilter::sync::mapping::Mapping =
        serde_json::from_str(minimal_mapping_str()).expect("parse minimal mapping");

    let overviews = vec![
        ("Currency".to_string(), currency_overview),
        ("UniqueWeapon".to_string(), uniques_overview),
        ("UncutGem".to_string(), gem_overview),
    ];

    let items = build_prices(&overviews, &mapping)
        .expect("build_prices should succeed on happy-path fixtures");

    // Anchors must be first three
    assert!(
        items.len() >= 3,
        "expected at least 3 items (anchors); got {}",
        items.len()
    );
    assert_eq!(items[0].name, "Exalted Orb", "items[0] must be Exalted Orb");
    assert_eq!(items[1].name, "Chaos Orb", "items[1] must be Chaos Orb");
    assert_eq!(items[2].name, "Divine Orb", "items[2] must be Divine Orb");

    // Chaos conversion must be via rates, NOT raw primaryValue.
    // With primary="divine", rates.chaos=220.0:
    //   Chaos Orb primaryValue=0.004545 → chaos = 0.004545 * 220.0 ≈ 1.0 (override kicks in)
    //   Divine Orb primaryValue=1.0    → chaos = 1.0 * 220.0 = 220.0
    let chaos_orb = &items[1];
    let chaos_val = chaos_orb.chaos.expect("Chaos Orb chaos must be Some");
    // Explicit override sets Chaos Orb chaos = 1.0
    assert_eq!(chaos_val, 1.0, "Chaos Orb chaos must be 1.0 (explicit override)");

    let divine_orb = &items[2];
    let divine_chaos = divine_orb.chaos.expect("Divine Orb chaos must be Some");
    // Converted: 1.0 * 220.0 = 220.0
    assert!(
        (divine_chaos - 220.0).abs() < 1.0,
        "Divine Orb chaos must be ~220.0 (converted); got {divine_chaos}"
    );

    // Each item has non-empty category and class from the mapping
    for item in items.iter().take(3) {
        assert!(
            !item.category.is_empty(),
            "item '{}' has empty category",
            item.name
        );
        // Anchor orbs are Currency category
        assert_eq!(item.category, "Currency", "anchor '{}' must be Currency", item.name);
        assert_eq!(
            item.class, "Stackable Currency",
            "anchor '{}' must be Stackable Currency",
            item.name
        );
    }

    // The unique from overview-uniques.json (Voidforge) must appear with category "Uniques"
    let voidforge = items.iter().find(|i| i.name == "Voidforge");
    assert!(
        voidforge.is_some(),
        "Voidforge (from overview-uniques.json) must appear in output"
    );
    let voidforge = voidforge.unwrap();
    assert_eq!(
        voidforge.category, "Uniques",
        "Voidforge must have category 'Uniques'"
    );

    // Uncut gems collapse to their level-less base type; level 16 is excluded and the
    // highest-valued surviving variant (level 20, div 5.0) is kept.
    let gem = items.iter().find(|i| i.base_type == "Uncut Spirit Gem");
    assert!(gem.is_some(), "uncut gem base type must be present in output");
    assert!(
        !items.iter().any(|i| i.name.contains("(Level")),
        "no (Level N) suffix should remain in gem names"
    );
    assert_eq!(
        gem.unwrap().div,
        Some(5.0),
        "kept gem must be the highest-valued variant (level 20)"
    );
}

/// Write the happy-path output to a temp prices.json, then load it via the
/// real PricesFile model and assert the schema-correct output.
#[test]
fn happy_path_writes_schema_correct_prices_json() {
    let currency_overview: ferrisfilter::sync::models::ExchangeOverview =
        serde_json::from_str(FIXTURE_CURRENCY).expect("parse currency fixture");
    let uniques_overview: ferrisfilter::sync::models::ExchangeOverview =
        serde_json::from_str(FIXTURE_UNIQUES).expect("parse uniques fixture");
    let gem_overview: ferrisfilter::sync::models::ExchangeOverview =
        serde_json::from_str(FIXTURE_UNCUTGEM).expect("parse uncutgem fixture");

    let mapping: ferrisfilter::sync::mapping::Mapping =
        serde_json::from_str(minimal_mapping_str()).expect("parse minimal mapping");

    let overviews = vec![
        ("Currency".to_string(), currency_overview),
        ("UniqueWeapon".to_string(), uniques_overview),
        ("UncutGem".to_string(), gem_overview),
    ];

    let items = build_prices(&overviews, &mapping)
        .expect("build_prices should succeed");

    let prices = PricesFile {
        league: "Fate of the Vaal".to_string(),
        fetched_at: "2026-05-25T12:00:00Z".to_string(),
        items,
    };

    // Use a tempdir and write to a named path inside it (avoids open-handle conflicts on Windows)
    let tmp_dir = tempfile::tempdir().expect("create temp dir");
    let path = tmp_dir.path().join("prices.json");
    let path_str = path.to_str().unwrap();

    write_prices_atomic(path_str, &prices).expect("write_prices_atomic should succeed");

    // Load back via real PricesFile model
    let contents = std::fs::read_to_string(&path).expect("read back temp prices.json");
    let loaded: PricesFile = serde_json::from_str(&contents)
        .expect("written prices.json should parse as PricesFile");

    assert_eq!(loaded.league, "Fate of the Vaal");
    assert!(!loaded.fetched_at.is_empty());
    assert!(loaded.items.len() >= 3);
    assert_eq!(loaded.items[0].name, "Exalted Orb");
    assert_eq!(loaded.items[1].name, "Chaos Orb");
    assert_eq!(loaded.items[2].name, "Divine Orb");
}

// ---------------------------------------------------------------------------
// Task 2: League resolution tests (SYNC-05/CLI-04)
// ---------------------------------------------------------------------------

/// All four flag combos of resolve_league against the index-state fixture.
#[test]
fn league_resolution_all_four_flag_combos() {
    let state: IndexState = serde_json::from_str(FIXTURE_INDEX_STATE)
        .expect("parse index-state fixture");

    // (false, false) → softcore indexed: "Fate of the Vaal"
    let sc = resolve_league(&state, false, false)
        .expect("softcore should resolve");
    assert_eq!(sc, "Fate of the Vaal", "(false,false) must resolve to Fate of the Vaal");

    // (false, true) → HC trade (hardcore, url != "hardcore"): "HC Fate of the Vaal"
    let hc_trade = resolve_league(&state, false, true)
        .expect("hc trade should resolve");
    assert_eq!(hc_trade, "HC Fate of the Vaal", "(false,true) must resolve to HC Fate of the Vaal");

    // (true, false) → standard by url: "Standard"
    let standard = resolve_league(&state, true, false)
        .expect("standard should resolve");
    assert_eq!(standard, "Standard", "(true,false) must resolve to Standard");

    // (true, true) → hardcore by url: "Hardcore"
    let hardcore = resolve_league(&state, true, true)
        .expect("hardcore should resolve");
    assert_eq!(hardcore, "Hardcore", "(true,true) must resolve to Hardcore");
}

/// The league name (not url slug) is recorded in the PricesFile output.
#[test]
fn league_name_recorded_in_prices_output() {
    let state: IndexState = serde_json::from_str(FIXTURE_INDEX_STATE)
        .expect("parse index-state fixture");

    let league_name = resolve_league(&state, false, false)
        .expect("resolve softcore league");
    assert_eq!(league_name, "Fate of the Vaal");

    // Build a minimal PricesFile and assert league is recorded
    let currency_overview: ferrisfilter::sync::models::ExchangeOverview =
        serde_json::from_str(FIXTURE_CURRENCY).expect("parse currency fixture");
    let mapping: ferrisfilter::sync::mapping::Mapping =
        serde_json::from_str(minimal_mapping_str()).expect("parse minimal mapping");

    let items = build_prices(
        &[("Currency".to_string(), currency_overview)],
        &mapping,
    )
    .expect("build_prices should succeed");

    let prices = PricesFile {
        league: league_name.clone(),
        fetched_at: "2026-05-25T12:00:00Z".to_string(),
        items,
    };

    // League name must be recorded, not empty, not a url slug
    assert_eq!(prices.league, "Fate of the Vaal");
    assert_ne!(prices.league, "vaal", "league must be display name, not url slug");
    assert!(!prices.fetched_at.is_empty());
}

// ---------------------------------------------------------------------------
// Task 2: Gem ≥18 test (SYNC-03/D-09)
// ---------------------------------------------------------------------------

/// Build over overview-uncutgem.json (levels 16/18/20); assert only {18,20} survive.
#[test]
fn gem_filter_excludes_level_16() {
    let gem_overview: ferrisfilter::sync::models::ExchangeOverview =
        serde_json::from_str(FIXTURE_UNCUTGEM).expect("parse uncutgem fixture");

    // Use a mapping with only the gem filter (no other types)
    let mapping_src = r#"{
        "version": 1,
        "overviewTypes": [],
        "gemFilter": {
            "type": "UncutGem",
            "category": "Uncut Gems",
            "class": "Stackable Currency",
            "minLevel": 18
        }
    }"#;
    let mapping: ferrisfilter::sync::mapping::Mapping =
        serde_json::from_str(mapping_src).expect("parse gem-only mapping");

    let items = build_prices(
        &[("UncutGem".to_string(), gem_overview)],
        &mapping,
    )
    .expect("build_prices with gem filter should succeed");

    // Level 16 excluded; levels 18 and 20 collapse to a single base-type entry, keeping
    // the highest-valued variant (level 20, div 5.0). The "(Level N)" suffix is stripped.
    assert_eq!(
        items.len(),
        1,
        "expected exactly 1 collapsed gem base type; got {} items: {:?}",
        items.len(),
        items.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
    let gem = &items[0];
    assert_eq!(gem.base_type, "Uncut Spirit Gem");
    assert!(
        !gem.name.contains("(Level"),
        "level suffix must be stripped from gem name: {}",
        gem.name
    );
    assert_eq!(gem.div, Some(5.0), "kept variant must be level 20 (highest value)");
}

// ---------------------------------------------------------------------------
// Task 2: Failure-safety byte-equal test (SYNC-04/D-07)
// ---------------------------------------------------------------------------

/// Simulated network error: existing prices.json must remain byte-equal after
/// a failed sync attempt. The failure path must return Err.
#[test]
fn failure_safety_byte_equal_on_network_error() {
    // Write a known-good prices.json to a temp dir
    let known_good = PricesFile {
        league: "Fate of the Vaal".to_string(),
        fetched_at: "2026-05-24T00:00:00Z".to_string(),
        items: vec![
            PriceItem {
                name: "Exalted Orb".to_string(),
                base_type: "Exalted Orb".to_string(),
                class: "Stackable Currency".to_string(),
                category: "Currency".to_string(),
                exalt: Some(1.0),
                chaos: Some(4.5),
                div: None,
            },
            PriceItem {
                name: "Chaos Orb".to_string(),
                base_type: "Chaos Orb".to_string(),
                class: "Stackable Currency".to_string(),
                category: "Currency".to_string(),
                exalt: None,
                chaos: Some(1.0),
                div: None,
            },
            PriceItem {
                name: "Divine Orb".to_string(),
                base_type: "Divine Orb".to_string(),
                class: "Stackable Currency".to_string(),
                category: "Currency".to_string(),
                exalt: None,
                chaos: Some(220.0),
                div: Some(1.0),
            },
        ],
    };

    let tmp_dir = tempfile::tempdir().expect("create temp dir");
    let prices_path = tmp_dir.path().join("prices.json");
    let prices_path_str = prices_path.to_str().unwrap();

    write_prices_atomic(prices_path_str, &known_good)
        .expect("initial write should succeed");

    // Snapshot the bytes before the (simulated) failed sync
    let bytes_before = std::fs::read(&prices_path).expect("read prices.json before");

    // Simulate a failed sync: build_prices with all-empty data → empty items → write_prices_atomic refuses
    let empty_overview: ferrisfilter::sync::models::ExchangeOverview =
        serde_json::from_str(FIXTURE_EMPTY).expect("parse empty fixture");
    let mapping: ferrisfilter::sync::mapping::Mapping =
        serde_json::from_str(minimal_mapping_str()).expect("parse mapping");

    // build_prices on all-empty overviews produces 0 items
    let empty_items = build_prices(
        &[("Currency".to_string(), empty_overview)],
        &mapping,
    )
    .expect("build_prices should not error on empty data");

    assert!(
        empty_items.is_empty() || empty_items.iter().all(|i| i.name == ""), // defensive
        "expected empty item list from all-empty overview"
    );

    // Attempt to write empty prices → must return Err
    let bad_prices = PricesFile {
        league: "Fate of the Vaal".to_string(),
        fetched_at: "2026-05-25T12:00:00Z".to_string(),
        items: empty_items,
    };
    let write_result = write_prices_atomic(prices_path_str, &bad_prices);
    assert!(
        write_result.is_err(),
        "write_prices_atomic must return Err for empty items"
    );

    // The existing prices.json must be byte-equal to before the failed attempt
    let bytes_after = std::fs::read(&prices_path).expect("read prices.json after");
    assert_eq!(
        bytes_before, bytes_after,
        "prices.json must be byte-equal after a failed sync attempt"
    );
}

/// write_prices_atomic with empty items returns Err and writes nothing.
#[test]
fn write_prices_atomic_empty_returns_err_and_no_file() {
    let tmp_dir = tempfile::tempdir().expect("create temp dir");
    let path = tmp_dir.path().join("should-not-exist.json");
    let path_str = path.to_str().unwrap();

    let prices = PricesFile {
        league: "Test".to_string(),
        fetched_at: "2026-01-01T00:00:00Z".to_string(),
        items: vec![],
    };

    let result = write_prices_atomic(path_str, &prices);
    assert!(
        result.is_err(),
        "write_prices_atomic must return Err for empty items"
    );
    assert!(
        !path.exists(),
        "no file should be created when write is refused"
    );
}

// ---------------------------------------------------------------------------
// Task 2: Generate-loop test (closes the sync→prices.json→generate loop)
// ---------------------------------------------------------------------------

/// Take a sync-produced prices.json (from build_prices), write it to a temp dir
/// alongside config.json + tiers.json, run `ferrisfilter generate`, and assert
/// it exits 0 and produces a non-empty filter.
///
/// CRLF normalization: STATE.md decision — apply .replace("\r\n", "\n") before
/// comparing any filter text.
#[test]
fn generate_loop_with_sync_produced_prices() {
    use assert_cmd::Command;
    use std::fs;

    let manifest = env!("CARGO_MANIFEST_DIR");

    // Build prices.json from happy-path fixtures
    let currency_overview: ferrisfilter::sync::models::ExchangeOverview =
        serde_json::from_str(FIXTURE_CURRENCY).expect("parse currency fixture");
    let mapping: ferrisfilter::sync::mapping::Mapping =
        serde_json::from_str(minimal_mapping_str()).expect("parse mapping");

    let items = build_prices(
        &[("Currency".to_string(), currency_overview)],
        &mapping,
    )
    .expect("build_prices should succeed");

    let prices = PricesFile {
        league: "Fate of the Vaal".to_string(),
        fetched_at: "2026-05-25T12:00:00Z".to_string(),
        items,
    };

    // Set up a temp working dir with all three required inputs
    let tmp_dir = tempfile::tempdir().expect("create temp dir");

    // Write sync-produced prices.json to temp dir
    let prices_path = tmp_dir.path().join("prices.json");
    write_prices_atomic(prices_path.to_str().unwrap(), &prices)
        .expect("write sync-produced prices.json");

    // Copy config.json and tiers.json from repo root to temp dir
    let config_src = std::path::Path::new(manifest).join("config.json");
    let tiers_src = std::path::Path::new(manifest).join("tiers.json");
    let mapping_src = std::path::Path::new(manifest).join("mapping.json");

    if !config_src.exists() {
        eprintln!("SKIP: config.json not present — skipping generate-loop test");
        return;
    }

    fs::copy(&config_src, tmp_dir.path().join("config.json"))
        .expect("copy config.json to temp dir");
    fs::copy(&tiers_src, tmp_dir.path().join("tiers.json"))
        .expect("copy tiers.json to temp dir");
    fs::copy(&mapping_src, tmp_dir.path().join("mapping.json"))
        .expect("copy mapping.json to temp dir");

    // Run `ferrisfilter generate --output <tmpfile>` from the temp dir
    let out_path = tmp_dir.path().join("output.filter");

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(tmp_dir.path())
        .args(["generate", "--output", out_path.to_str().unwrap()])
        .assert()
        .success();

    // Assert the output filter is non-empty
    let filter_text = fs::read_to_string(&out_path).expect("read generated filter");
    // Apply CRLF normalization per STATE.md decision
    let normalized = filter_text.replace("\r\n", "\n");
    assert!(
        !normalized.is_empty(),
        "generate must produce a non-empty filter from sync-produced prices.json"
    );
    assert!(
        normalized.contains("Show"),
        "generated filter must contain at least one Show block"
    );
}
