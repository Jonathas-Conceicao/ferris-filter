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

/// The repo's real, valid `styles.json`.
fn valid_styles() -> String {
    fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/styles.json"))
        .expect("read repo styles.json")
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

/// A `styles.json` that is valid JSON but has a TYPE-MALFORMED field must exit non-zero
/// and print BOTH the dotted field path AND the expected type. This is the core of
/// ROADMAP success criterion 3: the typed model + serde_path_to_error surface
/// `S.font: invalid type: string ..., expected u8`.
///
/// Phase 17 changed this one test (and only this one): the named styles moved out of
/// `config.json` into `styles.json`, so the malformed field now lives in the styles file and
/// the dotted path starts at the style name (`S.font`, no `styles.` prefix).
#[test]
fn malformed_styles_json_reports_field_path_and_expected_type() {
    let tiers_src = valid_tiers();

    // Break exactly one numeric field's type: the first `"font": 37` (style "S") becomes a
    // string. Everything else stays valid, so deserialization fails specifically at S.font.
    let styles_src = valid_styles();
    let bad_styles = styles_src.replacen("\"font\": 37", "\"font\": \"BROKEN\"", 1);
    assert_ne!(bad_styles, styles_src, "fixture setup: expected to break a font field");

    let dir = make_temp_dir("bad_styles_type", &[
        ("config.json", &valid_config()),
        ("tiers.json", &tiers_src),
        ("styles.json", &bad_styles),
    ]);

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&dir)
        .arg("generate")
        .assert()
        .failure()
        // file reference
        .stderr(contains("styles.json"))
        // bad field path
        .stderr(contains("S.font"))
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
    let dir = make_temp_dir("missing_tier", &[
        ("config.json", &cfg),
        ("tiers.json", bad_tiers),
        ("styles.json", &valid_styles()),
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
    let dir = make_temp_dir("bad_order", &[
        ("config.json", &cfg),
        ("tiers.json", bad_tiers),
        ("styles.json", &valid_styles()),
    ]);

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&dir)
        .arg("generate")
        .assert()
        .failure()
        .stderr(contains("not strictly ascending"));
}

/// Temp dir with the early-game inputs plus the given config and styles text.
fn styles_case_dir(tag: &str, config: &str, styles: &str) -> PathBuf {
    let m = env!("CARGO_MANIFEST_DIR");
    let pf = fs::read_to_string(format!("{m}/prices-fallback.json")).unwrap();
    let tf = fs::read_to_string(format!("{m}/tiers-fallback.json")).unwrap();
    make_temp_dir(
        tag,
        &[
            ("config.json", config),
            ("styles.json", styles),
            ("prices-fallback.json", &pf),
            ("tiers-fallback.json", &tf),
        ],
    )
}

fn generate_in(dir: &PathBuf) -> assert_cmd::assert::Assert {
    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(dir)
        .args(["generate", "--early-game"])
        .assert()
}

const RULE_STYLE: &str = "\"style\": \"Recycle\",";

/// An enabled rule naming an undefined style fails at load.
#[test]
fn unknown_style_in_an_enabled_rule_fails_at_load() {
    let cfg = valid_config();
    let bad = cfg.replacen(RULE_STYLE, "\"style\": \"nope\",", 1);
    assert_ne!(bad, cfg, "mutation must change the config");
    let dir = styles_case_dir("unk_enabled", &bad, &valid_styles());
    generate_in(&dir)
        .failure()
        .stderr(contains("nope"))
        .stderr(contains("rule"))
        .stderr(contains("styles.json"));
}

/// A disabled rule is inert, so an unknown style on it does not block generate.
#[test]
fn unknown_style_in_a_disabled_rule_does_not_fail() {
    let cfg = valid_config();
    let bad = cfg.replacen(RULE_STYLE, "\"enabled\": false, \"style\": \"nope\",", 1);
    assert_ne!(bad, cfg, "mutation must change the config");
    let dir = styles_case_dir("unk_disabled", &bad, &valid_styles());
    generate_in(&dir).success();
}

/// styles.json without a required style is rejected.
#[test]
fn styles_json_missing_a_required_style_fails() {
    let styles = valid_styles();
    let kept: Vec<&str> = styles
        .lines()
        .filter(|l| !l.trim_start().starts_with("\"E\":"))
        .collect();
    let bad = kept.join("\n");
    assert_ne!(bad.len(), styles.len(), "removal must change the text");
    serde_json::from_str::<serde_json::Value>(&bad).expect("still valid JSON");
    let dir = styles_case_dir("miss_req", &valid_config(), &bad);
    generate_in(&dir)
        .failure()
        .stderr(contains("missing required style"))
        .stderr(contains("E"));
}

/// A repeated style name is rejected rather than silently last-wins.
#[test]
fn duplicate_style_name_fails() {
    let styles = valid_styles();
    let s_line = styles
        .lines()
        .find(|l| l.trim_start().starts_with("\"S\":"))
        .expect("S line")
        .trim_end_matches(',')
        .to_string();
    let end = styles.rfind('}').unwrap();
    let bad = format!("{},\n{}\n{}", styles[..end].trim_end(), s_line, &styles[end..]);
    assert_ne!(bad, styles);
    let dir = styles_case_dir("dup_style", &valid_config(), &bad);
    generate_in(&dir)
        .failure()
        .stderr(contains("duplicate style name"));
}

/// An unknown colour name in styles.json names the file and offending value.
#[test]
fn bad_colour_in_styles_json_names_the_field() {
    let styles = valid_styles();
    let bad = styles.replacen("\"Pink\"", "\"Mauve\"", 1);
    assert_ne!(bad, styles);
    let dir = styles_case_dir("bad_colour", &valid_config(), &bad);
    generate_in(&dir)
        .failure()
        .stderr(contains("styles.json"))
        .stderr(contains("Mauve"));
}

/// A config that still carries an inline `styles` section is rejected (OD-1).
#[test]
fn inline_styles_section_in_config_is_rejected() {
    let cfg = valid_config();
    let bad = cfg.replacen('{', "{\"styles\": {},", 1);
    assert_ne!(bad, cfg);
    let dir = styles_case_dir("inline_styles", &bad, &valid_styles());
    generate_in(&dir)
        .failure()
        .stderr(contains("config.json"))
        .stderr(contains("styles.json"));
}
