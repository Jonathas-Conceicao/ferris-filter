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

/// A missing `config.json` (cwd has no config.json) must produce a non-zero exit
/// with a "Cannot read" / not-found message.
///
/// Satisfies CFG-02 / ROADMAP success criterion 3.
#[test]
fn missing_config_json_exits_nonzero_with_message() {
    // Temp dir has tiers.json but NO config.json.
    let tiers_src =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tiers.json"))
            .expect("read tiers.json");
    let dir = make_temp_dir("missing_cfg", &[("tiers.json", &tiers_src)]);

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&dir)
        .arg("generate")
        .assert()
        .failure()
        .stderr(contains("Cannot read"));
}

/// A `config.json` containing invalid JSON syntax must produce a non-zero exit.
/// The error output must reference `config.json` (from the `JsonLoadError` message).
///
/// Satisfies CFG-02 / ROADMAP success criterion 3.
#[test]
fn malformed_config_json_exits_nonzero_with_file_ref() {
    let tiers_src =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tiers.json"))
            .expect("read tiers.json");

    // JSON with a syntax error (trailing comma + bad token); serde will fail with path info.
    let bad_config = r#"{"styles": {"S": {"font": "not-a-number"}, bad_key}}"#;

    let dir = make_temp_dir("bad_cfg", &[
        ("config.json", bad_config),
        ("tiers.json", &tiers_src),
    ]);

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&dir)
        .arg("generate")
        .assert()
        .failure()
        .stderr(contains("config.json"));
}

/// A `tiers.json` missing a required tier must produce a non-zero exit with a
/// "missing required tier" validation message.
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

    // Minimal valid-JSON config so config loading passes; tiers loading should fail.
    let minimal_config = r#"{"styles": {}}"#;

    let dir = make_temp_dir("missing_tier", &[
        ("config.json", minimal_config),
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

    let minimal_config = r#"{"styles": {}}"#;

    let dir = make_temp_dir("bad_order", &[
        ("config.json", minimal_config),
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
