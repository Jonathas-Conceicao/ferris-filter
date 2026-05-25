use assert_cmd::Command;
use predicates::str::contains;
use std::fs;

/// Normalize CRLF line endings to LF.
///
/// Required for CRLF-safe test comparisons on Windows: the generator emits LF-only
/// (`writeln!` produces `\n`), but the working-tree `universal.filter` is checked out
/// as CRLF on Windows (`.gitattributes` has `*.filter text` + `core.autocrlf=true`).
/// Always normalize both sides before comparing or snapshotting.
fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

/// `ferrisfilter --help` lists both subcommands.
#[test]
fn help_lists_subcommands() {
    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("generate"))
        .stdout(contains("sync"));
}

/// `ferrisfilter generate --help` describes the `--output` flag.
#[test]
fn generate_help_shows_output_flag() {
    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .args(["generate", "--help"])
        .assert()
        .success()
        .stdout(contains("--output"));
}

/// Unknown subcommand exits code 2 with an `unrecognized subcommand` message.
///
/// Satisfies CLI-05 / ROADMAP success criterion 5.
#[test]
fn unknown_subcommand_exits_code_2() {
    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .arg("nonexistent")
        .assert()
        .failure()
        .code(2)
        .stderr(contains("unrecognized subcommand"));
}

/// `ferrisfilter generate` run from the repo root (where config.json and tiers.json
/// exist) must exit successfully.
///
/// Satisfies ROADMAP success criterion — generate + valid inputs → exit 0.
///
/// Uses `--output <tmpfile>` to avoid clobbering the committed `universal.filter`
/// during test runs (which would make artifact_guard compare the test's output
/// against itself, defeating its purpose).
#[test]
fn generate_in_repo_root_succeeds() {
    // The test binary's cwd is the Cargo manifest directory, which is the repo root.
    // config.json is created by plan 01-02; tiers.json is created by plan 01-03.
    // In the merged state both files exist at the repo root, so this test passes.
    //
    // In the plan 01-03 worktree, config.json does not exist yet (plan 01-02 owns it),
    // so we skip this test if it is absent.
    let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config.json");
    if !config_path.exists() {
        eprintln!("SKIP: config.json not present in this worktree (plan 01-02 owns it)");
        return;
    }

    let out_path = std::env::temp_dir().join("ferrisfilter_repo_root_test.filter");

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["generate", "--output", out_path.to_str().unwrap()])
        .assert()
        .success();
}

/// Snapshot test — locks the generator output against future regression (D-05).
///
/// Runs `ferrisfilter generate --output <tmp>` from the repo root (where config.json,
/// currency.json, and tiers.json live), reads the produced file, normalizes CRLF→LF,
/// and asserts it matches the committed snapshot oracle.
///
/// To update the snapshot after a deliberate generator change:
///   `INSTA_UPDATE=always cargo test --test cli_generate` then review the diff and commit
///   the updated `tests/snapshots/cli_generate__generate_snapshot.snap`.
#[test]
fn generate_snapshot() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out_path = std::env::temp_dir().join("ferrisfilter_snapshot_test.filter");

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(manifest)
        .args(["generate", "--output", out_path.to_str().unwrap()])
        .assert()
        .success();

    let generated = fs::read_to_string(&out_path).expect("read generated filter");
    let normalized = normalize(&generated);

    insta::assert_snapshot!("generate_snapshot", normalized);
}

/// Artifact-guard test — asserts the committed `universal.filter` matches generator
/// output (D-06). Prevents the shipped artifact from silently drifting from the generator.
///
/// Both sides are CRLF-normalized before comparison: the generator emits LF-only, but
/// the working-tree `universal.filter` is CRLF on Windows checkout.
///
/// If this test fails, run `ferrisfilter generate` and commit the updated `universal.filter`.
#[test]
fn artifact_guard() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out_path = std::env::temp_dir().join("ferrisfilter_artifact_guard_test.filter");

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(manifest)
        .args(["generate", "--output", out_path.to_str().unwrap()])
        .assert()
        .success();

    let generated = fs::read_to_string(&out_path).expect("read generated filter");
    let committed = fs::read_to_string(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/universal.filter"))
    )
    .expect("read committed universal.filter");

    let gen_lf = normalize(&generated);
    let com_lf = normalize(&committed);

    assert_eq!(
        gen_lf,
        com_lf,
        "committed universal.filter does not match generator output — \
         run `ferrisfilter generate` and commit the updated file"
    );
}

/// Determinism test — two consecutive `generate` runs with identical inputs must produce
/// byte-identical output (GEN-01 / D-07).
///
/// Both outputs are CRLF-normalized before comparison.
#[test]
fn generate_is_deterministic() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out1 = std::env::temp_dir().join("ferrisfilter_determinism_1.filter");
    let out2 = std::env::temp_dir().join("ferrisfilter_determinism_2.filter");

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(manifest)
        .args(["generate", "--output", out1.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(manifest)
        .args(["generate", "--output", out2.to_str().unwrap()])
        .assert()
        .success();

    let first = normalize(&fs::read_to_string(&out1).expect("read first output"));
    let second = normalize(&fs::read_to_string(&out2).expect("read second output"));

    assert_eq!(first, second, "generate() is not deterministic across two runs");
}
