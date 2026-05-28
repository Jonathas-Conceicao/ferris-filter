use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use std::path::PathBuf;

/// Helper: create a uniquely-named temporary directory under the system temp dir,
/// populate it with the provided files, and return the path.
///
/// The caller is responsible for the directory's lifetime; it is **not** cleaned up
/// automatically. This is acceptable for short-lived integration tests.
fn make_temp_dir(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ferrisfilter_test_{tag}_{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
    ));
    fs::create_dir_all(&dir).expect("create test temp dir");
    for (name, content) in files {
        fs::write(dir.join(name), content).expect("write fixture file");
    }
    dir
}

/// The repo's real, valid `config.json` — a complete `Config`. Used as the passing
/// config in tests that intend to exercise *tiers* validation, not config validation.
fn valid_config() -> String {
    fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/config.json"))
        .expect("read repo config.json")
}

/// The repo's real, valid `tiers.json`. Used as the passing tiers fixture in tests that
/// intend to exercise *config* validation.
fn valid_tiers() -> String {
    fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tiers.json"))
        .expect("read repo tiers.json")
}

/// The repo's real, valid `currency.json`. As of Phase 2, `generate` loads config.json +
/// currency.json before tiers.json, so any test that intends to fail at *tiers* validation
/// must supply a valid currency.json (otherwise generate errors on the missing currency file).
fn valid_currency() -> String {
    fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/currency.json"))
        .expect("read repo currency.json")
}

/// A missing `config.json` (cwd has no config.json) must produce a non-zero exit
/// with a "Cannot read" / not-found message.
///
/// Satisfies CFG-02 / ROADMAP success criterion 3 (missing config).
#[test]
fn missing_config_json_exits_nonzero_with_message() {
    // Temp dir has tiers.json but NO config.json.
    let tiers_src = valid_tiers();
    let dir = make_temp_dir("missing_cfg", &[("tiers.json", &tiers_src)]);

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&dir)
        .arg("generate")
        .assert()
        .failure()
        .stderr(contains("Cannot read"));
}

/// A `config.json` that is valid JSON but has a TYPE-MALFORMED field must exit non-zero
/// and print BOTH the dotted field path AND the expected type. This is the core of
/// ROADMAP success criterion 3: the typed `Config` model + serde_path_to_error surface
/// `styles.S.font: invalid type: string ..., expected u8`.
#[test]
fn malformed_config_json_reports_field_path_and_expected_type() {
    let tiers_src = valid_tiers();

    // Take the real config and break exactly one numeric field's type: the first
    // `"font": 37` (style "S") becomes a string. Everything else stays valid, so
    // deserialization fails specifically at styles.S.font with an expected-type message.
    let bad_config = valid_config().replacen("\"font\": 37", "\"font\": \"BROKEN\"", 1);
    assert!(
        bad_config.contains("\"font\": \"BROKEN\""),
        "fixture setup: expected to break a font field"
    );

    let dir = make_temp_dir("bad_cfg_type", &[
        ("config.json", &bad_config),
        ("tiers.json", &tiers_src),
    ]);

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&dir)
        .arg("generate")
        .assert()
        .failure()
        // file reference
        .stderr(contains("config.json"))
        // bad field path
        .stderr(contains("styles.S.font"))
        // expected type
        .stderr(contains("expected u8"));
}

/// A `tiers.json` missing a required tier must produce a non-zero exit with a
/// "missing required tier" validation message. Config is the real valid config so the
/// failure is unambiguously the tiers validation, not config loading.
///
/// Satisfies CFG-03 / ROADMAP success criterion 4.
#[test]
fn missing_tier_in_tiers_json_exits_nonzero() {
    // Valid-syntax tiers.json but missing tier "B".
    let bad_tiers = r#"[
        { "tier": "A", "min": 1.0,  "unit": "div"   },
        { "tier": "S", "min": 5.0,  "unit": "div"   },
        { "tier": "F", "min": 0.1,  "unit": "chaos" },
        { "tier": "E", "min": 0.5,  "unit": "chaos" },
        { "tier": "D", "min": 2.0,  "unit": "chaos" },
        { "tier": "C", "min": 10.0, "unit": "chaos" }
    ]"#;

    let cfg = valid_config();
    let cur = valid_currency();
    let dir = make_temp_dir("missing_tier", &[
        ("config.json", &cfg),
        ("currency.json", &cur),
        ("tiers.json", bad_tiers),
    ]);

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&dir)
        .arg("generate")
        .assert()
        .failure()
        .stderr(contains("missing required tier"));
}

/// A `tiers.json` with non-ascending bounds within a unit group must exit non-zero
/// with a "not strictly ascending" validation message.
///
/// Satisfies CFG-03 / ROADMAP success criterion 4.
#[test]
fn non_ascending_tiers_json_exits_nonzero() {
    // E min=0.5 then D min=0.05 — not ascending within the chaos group.
    let bad_tiers = r#"[
        { "tier": "A", "min": 1.0,  "unit": "div"   },
        { "tier": "S", "min": 5.0,  "unit": "div"   },
        { "tier": "F", "min": 0.1,  "unit": "chaos" },
        { "tier": "E", "min": 0.5,  "unit": "chaos" },
        { "tier": "D", "min": 0.05, "unit": "chaos" },
        { "tier": "C", "min": 10.0, "unit": "chaos" },
        { "tier": "B", "min": 30.0, "unit": "chaos" }
    ]"#;

    let cfg = valid_config();
    let cur = valid_currency();
    let dir = make_temp_dir("bad_order", &[
        ("config.json", &cfg),
        ("currency.json", &cur),
        ("tiers.json", bad_tiers),
    ]);

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&dir)
        .arg("generate")
        .assert()
        .failure()
        .stderr(contains("not strictly ascending"));
}
