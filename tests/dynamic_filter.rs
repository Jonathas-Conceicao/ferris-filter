//! Integration coverage for the dynamic filter store (Phase 12).
//!
//! Every case runs the binary hermetically in a fresh temp dir seeded from committed files —
//! never against the repo root, whose `prices.json` is git-ignored and reflects whatever the last
//! `sync` fetched. That mistake cost time twice during v0.3.

use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use std::path::{Path, PathBuf};

/// Normalize CRLF to LF. The generator emits LF-only; the working tree is CRLF on Windows.
fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A fresh temp dir containing the inputs a dynamic compile needs.
fn workspace(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ferrisfilter_dyn_{tag}_{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(dir.join("game")).expect("create temp dir");
    for f in ["dynamic_base.json", "tiers.json", "config.json", "styles.json"] {
        fs::copy(manifest().join(f), dir.join(f)).unwrap_or_else(|e| panic!("copy {f}: {e}"));
    }
    fs::copy(
        manifest().join("tests/fixtures/seed-prices.json"),
        dir.join("prices.json"),
    )
    .expect("copy seed prices");
    dir
}

fn apply(dir: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("ferrisfilter")
        .expect("binary")
        .current_dir(dir)
        .args(["apply", "--dest", "game"])
        .assert()
}

fn filter_text(dir: &Path) -> String {
    normalize(&fs::read_to_string(dir.join("game").join("dynamic.filter")).expect("read filter"))
}

/// Read the store, mutate it as JSON, write it back.
fn edit_store(dir: &Path, f: impl FnOnce(&mut serde_json::Value)) {
    let path = dir.join("dynamic.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read store")).expect("parse store");
    f(&mut v);
    fs::write(&path, serde_json::to_string_pretty(&v).expect("serialize")).expect("write store");
}

/// Only the policy blocks — everything before the first economy section banner.
fn policy_section(filter: &str) -> String {
    match filter.find("#============") {
        Some(i) => filter[..i].to_string(),
        None => filter.to_string(),
    }
}

/// Only the economy sections — from the first banner to the end.
fn economy_section(filter: &str) -> String {
    match filter.find("#============") {
        Some(i) => filter[i..].to_string(),
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------
// DYN-01 — the store parses, and malformation is reported with a field path
// ---------------------------------------------------------------------------

#[test]
fn malformed_store_reports_the_field_path() {
    let dir = workspace("malformed");
    apply(&dir).success();

    // `style` must be a string; make it a number.
    edit_store(&dir, |v| {
        v["rules"][0]["style"] = serde_json::json!(42);
    });

    Command::cargo_bin("ferrisfilter")
        .expect("binary")
        .current_dir(&dir)
        .args(["apply", "--dest", "game"])
        .assert()
        .failure()
        .stderr(contains("dynamic.json"))
        .stderr(contains("rules[0].style"));
}

// ---------------------------------------------------------------------------
// DYN-04 — every condition is independent; an absent one emits nothing
// ---------------------------------------------------------------------------

#[test]
fn a_sockets_only_rule_emits_sockets_and_nothing_else() {
    let dir = workspace("socketsonly");
    apply(&dir).success();

    edit_store(&dir, |v| {
        v["rules"] = serde_json::json!([{
            "style": "E",
            "when": { "sockets": { "min": 3 } }
        }]);
    });
    apply(&dir).success();

    let policy = policy_section(&filter_text(&dir));
    assert!(
        policy.contains("Sockets >= 3"),
        "expected the sockets condition, got:\n{policy}"
    );
    for absent in [
        "Class",
        "Rarity",
        "BaseArmour",
        "BaseEvasion",
        "BaseEnergyShield",
        "Quality",
        "Corrupted",
    ] {
        assert!(
            !policy.contains(absent),
            "a rule that sets only `sockets` must not emit a `{absent}` line, got:\n{policy}"
        );
    }
}

// ---------------------------------------------------------------------------
// DYN-02 — array order is the precedence
// ---------------------------------------------------------------------------

#[test]
fn reordering_rules_reorders_their_blocks() {
    let dir = workspace("reorder");
    apply(&dir).success();

    edit_store(&dir, |v| {
        v["rules"] = serde_json::json!([
            { "style": "E", "when": { "sockets": { "min": 3 } } },
            { "style": "F", "when": { "quality": { "min": 7 } } }
        ]);
    });
    apply(&dir).success();
    let first = policy_section(&filter_text(&dir));
    let sockets_first = first.find("Sockets >= 3").expect("sockets block");
    let quality_first = first.find("Quality >= 7").expect("quality block");
    assert!(sockets_first < quality_first, "authored order should hold");

    edit_store(&dir, |v| {
        let rules = v["rules"].as_array_mut().unwrap();
        rules.swap(0, 1);
    });
    apply(&dir).success();
    let second = policy_section(&filter_text(&dir));
    let sockets_second = second.find("Sockets >= 3").expect("sockets block");
    let quality_second = second.find("Quality >= 7").expect("quality block");
    assert!(
        quality_second < sockets_second,
        "swapping the rules must swap their blocks — array order is the precedence"
    );
}

// ---------------------------------------------------------------------------
// DYN-06 — dynamic.filter is a complete, playable filter
// ---------------------------------------------------------------------------

#[test]
fn dynamic_filter_carries_the_same_economy_sections_as_universal() {
    let dir = workspace("economy");
    apply(&dir).success();
    let dynamic_economy = economy_section(&filter_text(&dir));

    // Same prices.json, same tiers.json, compiled through `generate` instead.
    Command::cargo_bin("ferrisfilter")
        .expect("binary")
        .current_dir(&dir)
        .args(["generate", "--output", "universal.filter"])
        .assert()
        .success();
    let universal =
        normalize(&fs::read_to_string(dir.join("universal.filter")).expect("read universal"));
    let universal_economy = economy_section(&universal);

    assert!(
        !dynamic_economy.is_empty(),
        "dynamic.filter must have economy sections — without them, selecting it in-game \
         silently loses every currency, gem and unique highlight"
    );
    assert_eq!(
        dynamic_economy, universal_economy,
        "the economy half of dynamic.filter must match universal.filter's, priced from the same \
         prices.json"
    );
}

// ---------------------------------------------------------------------------
// DYN-05 — dynamic rules carry no area-level conditions
// ---------------------------------------------------------------------------

#[test]
fn dynamic_emits_no_area_level_while_universal_still_does() {
    let dir = workspace("arealevel");
    apply(&dir).success();
    let dynamic = filter_text(&dir);

    Command::cargo_bin("ferrisfilter")
        .expect("binary")
        .current_dir(&dir)
        .args(["generate", "--output", "universal.filter"])
        .assert()
        .success();
    let universal =
        normalize(&fs::read_to_string(dir.join("universal.filter")).expect("read universal"));

    assert!(
        !dynamic.contains("AreaLevel"),
        "the dynamic filter must emit no AreaLevel lines"
    );
    // Asserting only the dynamic half would pass even if AreaLevel emission were broken
    // everywhere, so pin the other half too.
    assert!(
        universal.contains("AreaLevel"),
        "config.json's area-level rules must still emit — otherwise this test proves nothing"
    );
}

// ---------------------------------------------------------------------------
// DYN-03 — the store persists across runs
// ---------------------------------------------------------------------------

#[test]
fn edits_to_the_store_survive_across_runs() {
    let dir = workspace("persist");
    apply(&dir).success().stdout(contains("created dynamic.json"));

    // Disable the first rule, then run again.
    edit_store(&dir, |v| {
        v["rules"][0]["enabled"] = serde_json::json!(false);
    });
    apply(&dir)
        .success()
        .stdout(contains("using existing dynamic.json"));

    let policy = policy_section(&filter_text(&dir));
    assert!(
        !policy.contains("# rule 0"),
        "the second run must reflect the edit — the store was re-seeded instead of read back"
    );
    // And the edit is still on disk afterwards.
    let store = fs::read_to_string(dir.join("dynamic.json")).expect("read store");
    assert!(store.contains("\"enabled\""), "the edit must persist on disk");
}

#[test]
fn reset_restores_the_store_from_the_base() {
    let dir = workspace("reset");
    apply(&dir).success();
    edit_store(&dir, |v| {
        v["rules"] = serde_json::json!([]);
    });

    Command::cargo_bin("ferrisfilter")
        .expect("binary")
        .current_dir(&dir)
        .args(["apply", "--dest", "game", "--reset"])
        .assert()
        .success()
        .stdout(contains("reset dynamic.json"));

    let store: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("dynamic.json")).expect("read"))
            .expect("parse");
    assert_eq!(
        store["rules"].as_array().expect("rules").len(),
        10,
        "--reset must restore the base's full rule set"
    );
}

// ---------------------------------------------------------------------------
// Disabled rules (12-01 integration)
// ---------------------------------------------------------------------------

#[test]
fn a_disabled_rule_emits_nothing_and_leaves_later_labels_alone() {
    let dir = workspace("disabled");
    apply(&dir).success();
    let before = policy_section(&filter_text(&dir));
    assert!(before.contains("# rule 1"), "baseline: rule 1 emits");
    assert!(before.contains("# rule 2"), "baseline: rule 2 emits");

    edit_store(&dir, |v| {
        v["rules"][1]["enabled"] = serde_json::json!(false);
    });
    apply(&dir).success();
    let after = policy_section(&filter_text(&dir));

    assert!(
        !after.contains("# rule 1"),
        "a disabled rule must emit no block at all"
    );
    assert!(
        after.contains("# rule 2"),
        "the rule after a disabled one must keep its own index label, not shift down"
    );
    // The rule is still in the store — disabled, not deleted.
    let store: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("dynamic.json")).expect("read"))
            .expect("parse");
    assert_eq!(store["rules"].as_array().expect("rules").len(), 10);
}

/// OD-2: `apply --styles <path>` reads that file with the default styles.json absent.
#[test]
fn apply_styles_flag_reads_alternate_path() {
    let dir = workspace("stylesflag");
    fs::rename(dir.join("styles.json"), dir.join("alt-styles.json")).expect("rename styles");

    Command::cargo_bin("ferrisfilter")
        .expect("binary")
        .current_dir(&dir)
        .args(["apply", "--dest", "game", "--styles", "alt-styles.json"])
        .assert()
        .success();

    assert!(dir.join("game").join("dynamic.filter").exists());
}
