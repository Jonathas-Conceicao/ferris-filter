// Integration tests for the `ferrisfilter install` subcommand.
//
// NOTE: this file is deliberately NOT named `cli_install.rs`. On Windows, UAC
// "installer detection" forces an elevation prompt for any executable whose name
// contains `install` (also `setup`/`update`/`patch`). A test binary named
// `cli_install-<hash>.exe` therefore fails to launch under `cargo test` with
// `os error 740` (ERROR_ELEVATION_REQUIRED). Naming the binary `cli_filter_copy`
// sidesteps the heuristic; the subcommand itself is still `install`.

use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use std::path::PathBuf;

/// Helper: create a uniquely-named temporary directory under the system temp dir,
/// populate it with the provided files, and return the path. Not auto-cleaned — fine
/// for short-lived integration tests.
fn make_temp_dir(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ferrisfilter_filtercopy_{tag}_{:x}",
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

/// Happy path: a present filter + an existing --dest folder copies the file through,
/// keeping the source file name, and reports the destination path.
#[test]
fn install_copies_filter_to_dest() {
    let work = make_temp_dir("ok", &[("universal.filter", "Show # test\n")]);
    let dest = work.join("game-config");
    fs::create_dir_all(&dest).expect("create dest dir");

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&work)
        .arg("install")
        .arg("--dest")
        .arg(&dest)
        .assert()
        .success()
        .stdout(contains("installed"));

    let installed = dest.join("universal.filter");
    assert!(installed.is_file(), "filter should be copied into --dest");
    assert_eq!(
        fs::read_to_string(&installed).expect("read installed filter"),
        "Show # test\n",
        "installed content must match the source byte-for-byte"
    );
}

/// A missing source filter must exit non-zero and point the user at `generate`.
#[test]
fn install_missing_source_exits_nonzero() {
    // Empty work dir: no universal.filter present. Provide a valid --dest so the failure
    // is unambiguously the missing source, not the destination.
    let work = make_temp_dir("no_src", &[]);
    let dest = work.join("game-config");
    fs::create_dir_all(&dest).expect("create dest dir");

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&work)
        .arg("install")
        .arg("--dest")
        .arg(&dest)
        .assert()
        .failure()
        .stderr(contains("Source filter not found"));
}

/// A non-existent destination folder must exit non-zero with guidance, never create it.
#[test]
fn install_missing_dest_exits_nonzero() {
    let work = make_temp_dir("no_dest", &[("universal.filter", "Show # test\n")]);
    let missing_dest = work.join("does-not-exist");

    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&work)
        .arg("install")
        .arg("--dest")
        .arg(&missing_dest)
        .assert()
        .failure()
        .stderr(contains("Destination folder not found"));

    assert!(
        !missing_dest.exists(),
        "install must not create the destination folder"
    );
}

/// `install` appears in the top-level help alongside generate and sync.
#[test]
fn help_lists_install_subcommand() {
    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("install"));
}
