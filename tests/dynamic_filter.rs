//! Integration coverage for filter-as-store `apply` (Phase 18).
//!
//! Every case runs the binary hermetically in a fresh temp dir seeded from committed files —
//! never against the repo root, whose `prices.json` is git-ignored and reflects whatever the last
//! `sync` fetched. The only spawn point is [`apply`], which sets `FERRISFILTER_HOME` and
//! `USERPROFILE` to the temp dir, so no test can reach the real PoE2 game folder even when
//! `--filter` is omitted.

use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use std::path::{Path, PathBuf};

const BEGIN: &str = "# ff:economy-begin";
const ECONOMY_LINE: &str = ferrisfilter::filter::section::BEGIN_LINE;
const RULE: &str = ferrisfilter::filter::section::RULE_LINE;
/// The retired end marker key: it must never be written.
const OLD_END: &str = "economy-end";

/// Normalize CRLF to LF. The generator emits LF-only; the working tree is CRLF on Windows.
fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A fresh temp dir (the app root) with the inputs an economy compile needs, plus `game/`.
fn workspace(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ferrisfilter_dyn_{tag}_{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(dir.join("game")).expect("create temp dir");
    for (src, dst) in [
        ("tiers-default.json", "tiers.json"),
        ("styles-default.json", "styles.json"),
    ] {
        fs::copy(manifest().join(src), dir.join(dst)).unwrap_or_else(|e| panic!("copy {src}: {e}"));
    }
    fs::copy(
        manifest().join("tests/fixtures/seed-prices.json"),
        dir.join("prices.json"),
    )
    .expect("copy seed prices");
    dir
}

/// Like [`workspace`], but the app root holds the committed `styles-default.json` /
/// `tiers-default.json` under their own names and no live `styles.json` / `tiers.json`.
fn workspace_defaults(tag: &str) -> PathBuf {
    let dir = workspace(tag);
    fs::remove_file(dir.join("styles.json")).expect("remove live styles");
    fs::remove_file(dir.join("tiers.json")).expect("remove live tiers");
    for f in ["styles-default.json", "tiers-default.json"] {
        fs::copy(manifest().join(f), dir.join(f)).unwrap_or_else(|e| panic!("copy {f}: {e}"));
    }
    dir
}

/// The single place `apply` is spawned.
fn apply(dir: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("ferrisfilter")
        .expect("binary")
        .current_dir(dir)
        .env("FERRISFILTER_HOME", dir)
        .env("USERPROFILE", dir)
        .arg("apply")
        .args(args)
        .assert()
}

fn gf(dir: &Path) -> PathBuf {
    dir.join("game").join("dynamic.filter")
}

fn gfs(dir: &Path) -> String {
    gf(dir).to_string_lossy().into_owned()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("read filter")
}

/// Everything under the economy banner (the three banner lines are not included).
fn section_body(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let b = lines
        .iter()
        .position(|l| l.starts_with(BEGIN))
        .expect("begin marker");
    assert_eq!(lines[b - 1], RULE, "rule line above the banner");
    assert_eq!(lines[b + 1], RULE, "rule line below the banner");
    lines[b + 2..].join("\n")
}

fn ffbak(dir: &Path) -> PathBuf {
    dir.join("game").join("dynamic.filter.ffbak")
}

#[test]
fn add_economy_creates_a_new_filter_with_only_the_section() {
    let dir = workspace("create");
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f, "--add-economy"]).success();
    let text = read(&gf(&dir));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], RULE, "{text}");
    assert_eq!(lines[1], ECONOMY_LINE, "{text}");
    assert_eq!(lines[2], RULE, "{text}");
    assert!(lines[3..].iter().any(|l| l.starts_with("Show")), "{text}");
    assert!(!text.contains(OLD_END));
    assert!(!ffbak(&dir).exists());
}

#[test]
fn plain_apply_on_a_missing_filter_writes_nothing() {
    let dir = workspace("missing");
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f])
        .failure()
        .stderr(contains("dynamic.filter"))
        .stderr(contains("--add-economy"));
    assert!(!gf(&dir).exists());
}

#[test]
fn economy_section_matches_generate() {
    let dir = workspace("parity");
    fs::copy(manifest().join("config.json"), dir.join("config.json")).expect("copy config");
    Command::cargo_bin("ferrisfilter")
        .expect("binary")
        .current_dir(&dir)
        .env("FERRISFILTER_HOME", &dir)
        .env("USERPROFILE", &dir)
        .args(["generate", "--output", "universal.filter"])
        .assert()
        .success();
    let generated = normalize(&read(&dir.join("universal.filter")));
    let expected = &generated[generated.find("# ============").expect("banner")..];

    let f = gfs(&dir);
    apply(&dir, &["--filter", &f, "--add-economy"]).success();
    let body = normalize(&section_body(&read(&gf(&dir))));
    // Only the economy section carries `# ff:name` / `# ff:style` lines; generate has none.
    let body: String = body
        .lines()
        .filter(|l| !l.starts_with("# ff:"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(body.trim_matches('\n'), expected.trim_matches('\n'));
}

#[test]
fn reapply_is_a_no_op() {
    let dir = workspace("noop");
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f, "--add-economy"]).success();
    let before = fs::read(gf(&dir)).unwrap();
    let mtime = fs::metadata(gf(&dir)).unwrap().modified().unwrap();
    apply(&dir, &["--filter", &f])
        .success()
        .stdout(contains("already up to date"));
    assert_eq!(before, fs::read(gf(&dir)).unwrap());
    assert_eq!(mtime, fs::metadata(gf(&dir)).unwrap().modified().unwrap());
}

#[test]
fn refresh_changes_nothing_outside_the_banner() {
    let dir = workspace("outside");
    let prefix = format!(
        "Show # one\r\n  Class \"Rings\"\r\n\r\nShow # two\r\n  Rarity Rare\r\n\r\n{RULE}\r\n{BEGIN} old\r\n{RULE}\r\n"
    );
    let original = format!("{prefix}\r\nShow # GARBAGE\r\n  Class \"Rings\"\r\n\r\n");
    fs::write(gf(&dir), &original).unwrap();
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f]).success();
    let after = fs::read_to_string(gf(&dir)).unwrap();
    assert!(after.starts_with(&prefix), "prefix changed");
    assert!(!after.contains("GARBAGE"));
    assert!(
        !after.replace("\r\n", "").contains('\n'),
        "bare LF in a CRLF file"
    );
}

#[test]
fn plain_apply_on_a_foreign_filter_without_a_section_refuses() {
    let dir = workspace("foreign");
    let original = "Show # mine\n  Class \"Rings\"\n";
    fs::write(gf(&dir), original).unwrap();
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f])
        .failure()
        .stderr(contains("no economy section"));
    assert_eq!(fs::read_to_string(gf(&dir)).unwrap(), original);
    assert!(!ffbak(&dir).exists());
}

#[test]
fn remove_economy_removes_the_section_and_keeps_the_rest() {
    let dir = workspace("remove");
    fs::write(gf(&dir), "Show # mine\n  Class \"Rings\"\n").unwrap();
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f, "--add-economy"]).success();
    apply(&dir, &["--filter", &f, "--remove-economy"]).success();
    let text = read(&gf(&dir));
    assert!(!text.contains(BEGIN) && !text.contains(RULE));
    assert_eq!(text, "Show # mine\n  Class \"Rings\"\n");
}

#[test]
fn remove_keeps_a_following_section() {
    let dir = workspace("removekeep");
    let tail = format!("{RULE}\n# ff:section Later\n{RULE}\n\nShow # later\n  Class \"Belts\"\n");
    let original = format!(
        "Show # mine\n  Class \"Rings\"\n\n{RULE}\n{ECONOMY_LINE}\n{RULE}\n\nHide # old\n\n{tail}"
    );
    fs::write(gf(&dir), &original).unwrap();
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f, "--remove-economy"]).success();
    assert_eq!(
        read(&gf(&dir)),
        format!("Show # mine\n  Class \"Rings\"\n{tail}")
    );
}

#[test]
fn refresh_keeps_a_following_section() {
    let dir = workspace("refreshkeep");
    let tail = format!("{RULE}\n# ff:section Later\n{RULE}\n\nShow # later\n  Class \"Belts\"\n");
    let original = format!("{RULE}\n{ECONOMY_LINE}\n{RULE}\n\nHide # old\n\n{tail}");
    fs::write(gf(&dir), &original).unwrap();
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f]).success();
    let after = read(&gf(&dir));
    assert!(after.ends_with(&format!("\n\n{tail}")), "{after}");
    assert!(!after.contains("Hide # old"));
}

#[test]
fn old_one_line_markers_are_damaged() {
    let dir = workspace("oldmarkers");
    let original = "Show # a\n\n# ff:economy-begin old\n\nShow # old\n\n# ff:economy-end old\n";
    fs::write(gf(&dir), original).unwrap();
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f])
        .failure()
        .stderr(contains("line 3"));
    apply(&dir, &["--filter", &f, "--add-economy"]).failure();
    assert_eq!(read(&gf(&dir)), original);
}

#[test]
fn add_and_remove_conflict() {
    let dir = workspace("conflict");
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f, "--add-economy", "--remove-economy"]).failure();
    assert!(!gf(&dir).exists());
}

#[test]
fn damaged_markers_are_a_hard_stop() {
    let dir = workspace("damaged");
    let original = format!("Show # a\n\n{BEGIN} x\nShow # b\n");
    fs::write(gf(&dir), &original).unwrap();
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f])
        .failure()
        .stderr(contains("line 3"));
    assert_eq!(read(&gf(&dir)), original);
}

#[test]
fn v04_filter_is_converted_on_first_apply() {
    let dir = workspace("v04");
    let fixture = fs::read(manifest().join("tests/fixtures/v04-dynamic.filter")).unwrap();
    fs::write(gf(&dir), &fixture).unwrap();
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f])
        .success()
        .stdout(contains("converting"));
    let text = read(&gf(&dir));
    assert!(text.contains(ECONOMY_LINE) && !text.contains(OLD_END));
    assert!(text.contains("# ff:name Chance bases"));
    assert!(!text.contains("ff:name rule 4"));
    assert_eq!(fs::read(ffbak(&dir)).unwrap(), fixture);
    apply(&dir, &["--filter", &f])
        .success()
        .stdout(contains("already up to date"));
}

#[test]
fn no_price_data_refuses_and_keeps_the_section() {
    let dir = workspace("noprices");
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f, "--add-economy"]).success();
    let before = fs::read(gf(&dir)).unwrap();
    fs::remove_file(dir.join("prices.json")).unwrap();
    apply(&dir, &["--filter", &f])
        .failure()
        .stderr(contains("No price data"));
    assert_eq!(before, fs::read(gf(&dir)).unwrap());
}

#[test]
fn app_root_from_ferrisfilter_home_wins_over_cwd() {
    let dir = workspace("root");
    let other = workspace("rootcwd");
    // The cwd copy differs in a value that reaches the output.
    let styles = fs::read_to_string(other.join("styles.json")).unwrap();
    assert!(styles.contains("\"font\": 37"));
    fs::write(
        other.join("styles.json"),
        styles.replace("\"font\": 37", "\"font\": 44"),
    )
    .unwrap();

    let f = gfs(&dir);
    Command::cargo_bin("ferrisfilter")
        .expect("binary")
        .current_dir(&other)
        .env("FERRISFILTER_HOME", &dir)
        .env("USERPROFILE", &dir)
        .args(["apply", "--add-economy", "--filter", &f])
        .assert()
        .success();
    let text = read(&gf(&dir));
    assert!(text.contains("SetFontSize 37"), "workspace styles not used");
    assert!(!text.contains("SetFontSize 44"), "cwd styles leaked in");
}

#[test]
fn apply_styles_flag_reads_alternate_path() {
    let dir = workspace("stylesflag");
    fs::rename(dir.join("styles.json"), dir.join("alt-styles.json")).expect("rename styles");
    let f = gfs(&dir);
    apply(
        &dir,
        &[
            "--filter",
            &f,
            "--add-economy",
            "--styles",
            "alt-styles.json",
        ],
    )
    .success();
    assert!(gf(&dir).exists());
}

/// A relative `--styles`/`--tiers` resolves against the app root, not the working directory: run
/// from the `game` subfolder, where neither file exists, the overrides are still found.
#[test]
fn apply_relative_overrides_resolve_against_the_app_root_not_the_cwd() {
    let dir = workspace("overrideroot");
    fs::rename(dir.join("styles.json"), dir.join("alt-styles.json")).expect("rename styles");
    fs::copy(
        manifest().join("tiers-fallback.json"),
        dir.join("alt-tiers.json"),
    )
    .expect("copy tiers");
    let f = gfs(&dir);
    Command::cargo_bin("ferrisfilter")
        .expect("binary")
        .current_dir(dir.join("game"))
        .env("FERRISFILTER_HOME", &dir)
        .env("USERPROFILE", &dir)
        .args([
            "apply",
            "--filter",
            &f,
            "--add-economy",
            "--styles",
            "alt-styles.json",
            "--tiers",
            "alt-tiers.json",
        ])
        .assert()
        .success();
    assert!(gf(&dir).exists());
}

#[test]
fn default_filter_path_uses_userprofile() {
    let dir = workspace("default");
    apply(&dir, &[])
        .failure()
        .stderr(contains("Path of Exile 2"))
        .stderr(contains("dynamic.filter"));
    assert!(!dir.join("OneDrive").exists());
}

/// DEF-03: with no live files in the app root, apply reads the defaults, writes the same economy
/// section as from live copies, and creates neither live file.
#[test]
fn apply_falls_back_to_default_files_and_creates_no_live_file() {
    let dir = workspace_defaults("defaults");
    let f = gfs(&dir);
    apply(&dir, &["--filter", &f, "--add-economy"]).success();
    let from_defaults = read(&gf(&dir));
    assert!(from_defaults.contains(ECONOMY_LINE), "{from_defaults}");
    assert!(!dir.join("styles.json").exists(), "styles.json was created");
    assert!(!dir.join("tiers.json").exists(), "tiers.json was created");

    let live = workspace("defaults_live");
    let lf = gfs(&live);
    apply(&live, &["--filter", &lf, "--add-economy"]).success();
    assert_eq!(normalize(&from_defaults), normalize(&read(&gf(&live))));
}
